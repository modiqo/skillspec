//! Remote skill-source staging primitives.
//!
//! Target parsing and git checkout mechanics for public GitHub skill targets.
//! This module deliberately knows nothing about skill shape or analysis: it
//! resolves a target to a repository plus an optional path, and materializes
//! that path into a temporary or persistent checkout.
//!
//! Callers that need a shape-aware staging report build it on top of these
//! primitives.

use serde::Serialize;
use skillspec_core::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize)]
pub struct RemoteSkillSource {
    pub repo_url: String,
    pub branch: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug)]
pub struct RemoteCheckout {
    pub root: PathBuf,
    pub checkout_dir: PathBuf,
}

pub struct TemporaryRemoteCheckout {
    checkout: RemoteCheckout,
}

impl TemporaryRemoteCheckout {
    pub fn checkout_dir(&self) -> &Path {
        &self.checkout.checkout_dir
    }
}

impl Drop for TemporaryRemoteCheckout {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.checkout.root);
    }
}

pub fn parse_target(target: &str) -> Result<Option<RemoteSkillSource>> {
    let trimmed = target.trim();
    if let Some(path) = trimmed.strip_prefix("git@github.com:") {
        let path = path.trim_end_matches(".git");
        let parts = path
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() < 2 {
            return Err(Error::InvalidInput {
                message: "remote source SSH shorthand requires git@github.com:<owner>/<repo>.git"
                    .to_owned(),
            });
        }
        return Ok(Some(RemoteSkillSource {
            repo_url: format!("https://github.com/{}/{}.git", parts[0], parts[1]),
            branch: None,
            path: (parts.len() > 2).then(|| parts[2..].join("/")),
        }));
    }

    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let github_path = without_scheme
        .strip_prefix("github.com/")
        .or_else(|| trimmed.strip_prefix("github:"));
    let Some(github_path) = github_path else {
        if looks_like_github_shorthand(trimmed) {
            return github_shorthand(trimmed).map(Some);
        }
        return Ok(None);
    };

    let parts = github_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(Error::InvalidInput {
            message: "remote source requires a GitHub owner/repo target".to_owned(),
        });
    }

    let owner = parts[0];
    let repo = parts[1].trim_end_matches(".git");
    let (branch, path_parts) = if matches!(parts.get(2), Some(&"tree" | &"blob")) {
        if parts.len() < 4 {
            return Err(Error::InvalidInput {
                message: "GitHub tree/blob URL must include a branch".to_owned(),
            });
        }
        (Some(parts[3].to_owned()), &parts[4..])
    } else {
        (None, parts.get(2..).unwrap_or(&[]))
    };
    let path = path_parts.join("/");
    if is_skill_file_path(&path) {
        return Err(Error::InvalidInput {
            message: "remote source expects a skill folder or repo URL, not a SKILL.md blob"
                .to_owned(),
        });
    }
    Ok(Some(RemoteSkillSource {
        repo_url: format!("https://github.com/{owner}/{repo}.git"),
        branch,
        path: (!path.is_empty()).then_some(path),
    }))
}

pub fn clone_remote_temp(
    remote: &RemoteSkillSource,
    prefix: &str,
) -> Result<TemporaryRemoteCheckout> {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        unique_nanos()
    ));
    let checkout = clone_remote_into(remote, &root)?;
    Ok(TemporaryRemoteCheckout { checkout })
}

pub fn clone_remote_persistent(
    remote: &RemoteSkillSource,
    out: Option<&Path>,
) -> Result<RemoteCheckout> {
    let root = match out {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir()
            .map_err(|source| Error::InvalidInput {
                message: format!(
                    "failed to determine current directory for remote staging: {source}"
                ),
            })?
            .join(".skillspec")
            .join("staged")
            .join(format!("{}-{}", repo_slug(remote), unique_nanos())),
    };
    clone_remote_into(remote, &root)
}

pub fn clone_remote_into(remote: &RemoteSkillSource, root: &Path) -> Result<RemoteCheckout> {
    if root.exists() {
        let is_empty = fs::read_dir(root)
            .map_err(|source| Error::Read {
                path: root.to_path_buf(),
                source,
            })?
            .next()
            .is_none();
        if !is_empty {
            return Err(Error::InvalidInput {
                message: format!(
                    "remote staging output already exists and is not empty: {}",
                    root.display()
                ),
            });
        }
    }
    fs::create_dir_all(root).map_err(|source| Error::Write {
        path: root.to_path_buf(),
        source,
    })?;
    let checkout_dir = root.join("repo");
    let mut clone = Command::new("git");
    clone
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--filter=blob:none");
    if remote.path.is_some() {
        clone.arg("--sparse");
    }
    if let Some(branch) = &remote.branch {
        clone.arg("--branch").arg(branch);
    }
    clone.arg(&remote.repo_url).arg(&checkout_dir);
    run_git(clone, "clone remote skill repository")?;
    Ok(RemoteCheckout {
        root: root.to_path_buf(),
        checkout_dir,
    })
}

pub fn set_sparse_path(checkout_dir: &Path, path: &str) -> Result<()> {
    set_sparse_paths(checkout_dir, &[path.to_owned()])
}

fn set_sparse_paths(checkout_dir: &Path, paths: &[String]) -> Result<()> {
    let mut sparse = Command::new("git");
    sparse
        .arg("-C")
        .arg(checkout_dir)
        .arg("sparse-checkout")
        .arg("set");
    for path in paths {
        sparse.arg(path);
    }
    run_git(sparse, "sparse-checkout remote source target")
}

pub fn git_tree_files(checkout_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(checkout_dir)
        .arg("ls-tree")
        .arg("-r")
        .arg("--name-only")
        .arg("HEAD");
    let output = command.output().map_err(|source| Error::InvalidInput {
        message: format!("failed to list remote repository tree: {source}"),
    })?;
    if !output.status.success() {
        let stderr = compact_stderr(&output.stderr);
        return Err(Error::InvalidInput {
            message: format!("git failed to list remote repository tree: {stderr}"),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

pub fn git_show_text(checkout_dir: &Path, path: &str) -> Result<String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(checkout_dir)
        .arg("show")
        .arg(format!("HEAD:{path}"));
    let output = command.output().map_err(|source| Error::InvalidInput {
        message: format!("failed to read remote repository file {path}: {source}"),
    })?;
    if !output.status.success() {
        let stderr = compact_stderr(&output.stderr);
        return Err(Error::InvalidInput {
            message: format!("git failed to read remote repository file {path}: {stderr}"),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_git(mut command: Command, action: &str) -> Result<()> {
    let output = command.output().map_err(|source| Error::InvalidInput {
        message: format!("failed to run git for {action}: {source}"),
    })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = compact_stderr(&output.stderr);
    Err(Error::InvalidInput {
        message: format!("git failed during {action}: {stderr}"),
    })
}

fn compact_stderr(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .take(8)
        .collect::<Vec<_>>()
        .join("\n")
}

fn looks_like_github_shorthand(target: &str) -> bool {
    let parts = target
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    parts.len() >= 2
        && !target.starts_with('/')
        && !target.starts_with('.')
        && !target.contains("://")
        && parts[0]
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        && parts[1]
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn github_shorthand(target: &str) -> Result<RemoteSkillSource> {
    let parts = target
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let path = parts.get(2..).unwrap_or(&[]).join("/");
    if is_skill_file_path(&path) {
        return Err(Error::InvalidInput {
            message: "remote source shorthand expects owner/repo or owner/repo/<skill-folder>"
                .to_owned(),
        });
    }
    Ok(RemoteSkillSource {
        repo_url: format!("https://github.com/{}/{}.git", parts[0], parts[1]),
        branch: None,
        path: (!path.is_empty()).then_some(path),
    })
}

fn is_skill_file_path(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
}

fn repo_slug(remote: &RemoteSkillSource) -> String {
    remote
        .repo_url
        .trim_end_matches(".git")
        .rsplit('/')
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("--")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

pub fn unique_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::parse_target;

    #[test]
    fn parses_github_tree_skill_folder_url() {
        let remote = parse_target("https://github.com/anthropics/skills/tree/main/skills/pdf")
            .unwrap()
            .unwrap();
        assert_eq!(remote.repo_url, "https://github.com/anthropics/skills.git");
        assert_eq!(remote.branch.as_deref(), Some("main"));
        assert_eq!(remote.path.as_deref(), Some("skills/pdf"));
    }

    #[test]
    fn parses_github_blob_skill_folder_url() {
        let remote = parse_target(
            "https://github.com/anthropics/claude-for-legal/blob/main/employment-legal/skills/international-expansion",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            remote.repo_url,
            "https://github.com/anthropics/claude-for-legal.git"
        );
        assert_eq!(remote.branch.as_deref(), Some("main"));
        assert_eq!(
            remote.path.as_deref(),
            Some("employment-legal/skills/international-expansion")
        );
    }

    #[test]
    fn parses_github_owner_repo_path_shorthand() {
        let remote = parse_target("anthropics/skills/skills/pdf")
            .unwrap()
            .unwrap();
        assert_eq!(remote.repo_url, "https://github.com/anthropics/skills.git");
        assert_eq!(remote.branch, None);
        assert_eq!(remote.path.as_deref(), Some("skills/pdf"));
    }

    #[test]
    fn parses_github_repo_root_url() {
        let remote = parse_target("https://github.com/anthropics/skills")
            .unwrap()
            .unwrap();
        assert_eq!(remote.repo_url, "https://github.com/anthropics/skills.git");
        assert_eq!(remote.branch, None);
        assert_eq!(remote.path, None);
    }

    #[test]
    fn parses_github_tree_repo_root_url() {
        let remote = parse_target("https://github.com/anthropics/skills/tree/main")
            .unwrap()
            .unwrap();
        assert_eq!(remote.repo_url, "https://github.com/anthropics/skills.git");
        assert_eq!(remote.branch.as_deref(), Some("main"));
        assert_eq!(remote.path, None);
    }

    #[test]
    fn rejects_github_blob_skill_md_file_urls() {
        let error =
            parse_target("https://github.com/anthropics/skills/blob/main/skills/pdf/SKILL.md")
                .unwrap_err()
                .to_string();
        assert!(error.contains("not a SKILL.md blob"));
    }

    #[test]
    fn rejects_remote_skill_md_file_shorthand() {
        let error = parse_target("anthropics/skills/skills/pdf/SKILL.md")
            .unwrap_err()
            .to_string();
        assert!(error.contains("owner/repo or owner/repo/<skill-folder>"));
    }
}
