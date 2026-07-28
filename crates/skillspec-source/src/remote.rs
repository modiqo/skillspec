//! Remote skill-source staging primitives.
//!
//! Target parsing and git checkout mechanics for public skill targets on any
//! git host: GitHub, GitLab, Bitbucket, self-hosted, or a direct .git URL.
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

    // SSH shorthand for any host: git@<host>:<owner>/<repo>[.git][/<path>].
    if let Some(rest) = trimmed.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            let parts = split_parts(path.trim_end_matches(".git"));
            if parts.len() < 2 {
                return Err(Error::InvalidInput {
                    message: "SSH shorthand requires git@<host>:<owner>/<repo>.git".to_owned(),
                });
            }
            return Ok(Some(RemoteSkillSource {
                repo_url: format!("https://{host}/{}/{}.git", parts[0], parts[1]),
                branch: None,
                path: (parts.len() > 2).then(|| parts[2..].join("/")),
            }));
        }
    }

    // A direct `.git` clone URL, any host, with no subpath.
    if (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
        && trimmed.trim_end_matches('/').ends_with(".git")
    {
        return Ok(Some(RemoteSkillSource {
            repo_url: trimmed.trim_end_matches('/').to_owned(),
            branch: None,
            path: None,
        }));
    }

    // A full http(s) URL to a repo or a subfolder on any git host.
    if let Some(rest) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        return parse_http_target(rest).map(Some);
    }

    // Host-prefixed shorthands and bare owner/repo remain GitHub conventions.
    if let Some(path) = trimmed.strip_prefix("github.com/") {
        return parse_http_target(&format!("github.com/{path}")).map(Some);
    }
    if let Some(path) = trimmed.strip_prefix("github:") {
        return github_shorthand(path).map(Some);
    }
    if looks_like_github_shorthand(trimmed) {
        return github_shorthand(trimmed).map(Some);
    }
    Ok(None)
}

/// Parse a scheme-stripped `host/owner/repo[/<tree-marker>/<branch>/<path>]`.
///
/// The repository boundary is found by a host's tree/blob marker: GitHub and
/// GitLab short form use `/tree/` and `/blob/`, GitLab also uses `/-/tree/`,
/// Bitbucket uses `/src/`. Without a marker the whole path is taken as the
/// repository and cloned in full, which is correct for a URL that names a repo
/// directly, including a GitLab subgroup path.
fn parse_http_target(rest: &str) -> Result<RemoteSkillSource> {
    let rest = rest.trim_end_matches('/');
    let Some((host, path)) = rest.split_once('/') else {
        return Err(Error::InvalidInput {
            message: "remote source requires a <host>/<owner>/<repo> target".to_owned(),
        });
    };
    let segments = split_parts(path);
    if segments.len() < 2 {
        return Err(Error::InvalidInput {
            message: "remote source requires an owner and repository, e.g. https://host/owner/repo"
                .to_owned(),
        });
    }

    let (repo_segments, branch, path_parts) = split_on_tree_marker(&segments);
    let repo_path = repo_segments.join("/");
    let repo_path = repo_path.trim_end_matches(".git");
    let path = path_parts.join("/");
    if is_skill_file_path(&path) {
        return Err(Error::InvalidInput {
            message: "remote source expects a skill folder or repo URL, not a SKILL.md blob"
                .to_owned(),
        });
    }
    Ok(RemoteSkillSource {
        repo_url: format!("https://{host}/{repo_path}.git"),
        branch,
        path: (!path.is_empty()).then_some(path),
    })
}

/// Split path segments at the first tree/blob marker into
/// (repo segments, branch, subpath segments).
fn split_on_tree_marker<'a>(
    segments: &'a [&'a str],
) -> (Vec<&'a str>, Option<String>, Vec<&'a str>) {
    for (index, segment) in segments.iter().enumerate() {
        // GitLab's `/-/tree/<branch>` and `/-/blob/<branch>`.
        if *segment == "-"
            && index + 2 < segments.len()
            && matches!(segments[index + 1], "tree" | "blob")
        {
            return (
                segments[..index].to_vec(),
                Some(segments[index + 2].to_owned()),
                segments[index + 3..].to_vec(),
            );
        }
        // GitHub/GitLab `/tree|blob/<branch>` and Bitbucket `/src/<branch>`.
        if matches!(*segment, "tree" | "blob" | "src") && index + 1 < segments.len() {
            return (
                segments[..index].to_vec(),
                Some(segments[index + 1].to_owned()),
                segments[index + 2..].to_vec(),
            );
        }
    }
    // No marker: the first two segments are owner/repo, the rest is a subpath.
    (
        segments[..2].to_vec(),
        None,
        segments.get(2..).unwrap_or(&[]).to_vec(),
    )
}

fn split_parts(path: &str) -> Vec<&str> {
    path.split('/').filter(|part| !part.is_empty()).collect()
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

/// A skill package materialized at a git revision in a detached worktree.
///
/// The worktree is registered with the repository and removed when this value
/// drops, so a drift comparison can read a prior revision of a local skill
/// without disturbing the working tree or index.
pub struct WorktreeAtRef {
    repo_root: PathBuf,
    worktree: PathBuf,
    package: PathBuf,
}

impl WorktreeAtRef {
    /// The package directory inside the worktree.
    pub fn package_dir(&self) -> &Path {
        &self.package
    }
}

impl Drop for WorktreeAtRef {
    fn drop(&mut self) {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.repo_root)
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(&self.worktree);
        let _ = command.output();
        let _ = fs::remove_dir_all(&self.worktree);
    }
}

/// Materialize the skill package that contains `local_path` as it was at `git_ref`.
///
/// `local_path` must be inside a git working tree. The package's location
/// relative to the repository root is preserved, so a skill in a subdirectory is
/// found at the same subpath in the checked-out revision.
pub fn worktree_at_ref(local_path: &Path, git_ref: &str) -> Result<WorktreeAtRef> {
    let repo_root = git_repo_root(local_path)?;
    let relative = local_path
        .canonicalize()
        .map_err(|source| Error::Read {
            path: local_path.to_path_buf(),
            source,
        })?
        .strip_prefix(&repo_root)
        .map(Path::to_path_buf)
        .unwrap_or_default();

    let worktree = std::env::temp_dir().join(format!(
        "skillspec-boundary-diff-{}-{}",
        std::process::id(),
        unique_nanos()
    ));
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(&repo_root)
        .arg("worktree")
        .arg("add")
        .arg("--detach")
        .arg("--quiet")
        .arg(&worktree)
        .arg(git_ref);
    run_git(command, "create a worktree for the comparison revision")?;

    let package = worktree.join(&relative);
    if !package.exists() {
        let guard = WorktreeAtRef {
            repo_root,
            worktree,
            package,
        };
        return Err(Error::InvalidInput {
            message: format!(
                "the package path did not exist at {git_ref}: {}",
                guard.package.display()
            ),
        });
    }
    Ok(WorktreeAtRef {
        repo_root,
        worktree,
        package,
    })
}

fn git_repo_root(path: &Path) -> Result<PathBuf> {
    let start = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    };
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(&start)
        .arg("rev-parse")
        .arg("--show-toplevel");
    let output = command.output().map_err(|source| Error::InvalidInput {
        message: format!(
            "failed to locate the git repository for {}: {source}",
            path.display()
        ),
    })?;
    if !output.status.success() {
        return Err(Error::InvalidInput {
            message: format!(
                "{} is not inside a git repository, so it has no revision to compare against",
                path.display()
            ),
        });
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(PathBuf::from(root))
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

/// A web link a person can open to read the evidence for a finding, built for
/// the host's own blob-URL convention.
///
/// `rel_path` is the file's path relative to the repository root; `line` is an
/// optional 1-based line to anchor. The ref is the source's branch when known,
/// or `HEAD`, which the major hosts resolve to the default branch.
pub fn web_url(remote: &RemoteSkillSource, rel_path: &str, line: Option<usize>) -> String {
    let root = remote
        .repo_url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_owned();
    let git_ref = remote.branch.as_deref().unwrap_or("HEAD");
    let path = rel_path.trim_start_matches('/');
    let host = root
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(&root);
    if host.contains("gitlab") {
        anchor(format!("{root}/-/blob/{git_ref}/{path}"), line, "#L")
    } else if host.contains("bitbucket") {
        anchor(format!("{root}/src/{git_ref}/{path}"), line, "#lines-")
    } else {
        // GitHub and the common self-hosted convention. A `#L` anchor only lands
        // on the source view; Markdown renders rich and ignores it, so request
        // plain source with `?plain=1` when anchoring a line in a Markdown file.
        let query = if line.is_some() && is_markdown_path(path) {
            "?plain=1"
        } else {
            ""
        };
        anchor(format!("{root}/blob/{git_ref}/{path}{query}"), line, "#L")
    }
}

fn is_markdown_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown")
}

fn anchor(url: String, line: Option<usize>, sep: &str) -> String {
    match line {
        Some(line) => format!("{url}{sep}{line}"),
        None => url,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_target;

    #[test]
    fn web_url_builds_host_appropriate_blob_links() {
        let github = parse_target("https://github.com/o/r").unwrap().unwrap();
        // A Markdown file with a line anchor gets ?plain=1 so the anchor lands.
        assert_eq!(
            super::web_url(&github, "skills/x/SKILL.md", Some(690)),
            "https://github.com/o/r/blob/HEAD/skills/x/SKILL.md?plain=1#L690"
        );

        // A non-Markdown source file needs no query.
        assert_eq!(
            super::web_url(&github, "scripts/run.sh", Some(4)),
            "https://github.com/o/r/blob/HEAD/scripts/run.sh#L4"
        );

        let branched = parse_target("https://github.com/o/r/tree/main/skills/x")
            .unwrap()
            .unwrap();
        assert_eq!(
            super::web_url(&branched, "skills/x/SKILL.md", Some(12)),
            "https://github.com/o/r/blob/main/skills/x/SKILL.md?plain=1#L12"
        );

        let gitlab = parse_target("https://gitlab.com/g/r").unwrap().unwrap();
        assert_eq!(
            super::web_url(&gitlab, "a.md", Some(3)),
            "https://gitlab.com/g/r/-/blob/HEAD/a.md#L3"
        );

        let bitbucket = parse_target("https://bitbucket.org/t/r").unwrap().unwrap();
        assert_eq!(
            super::web_url(&bitbucket, "a.md", Some(3)),
            "https://bitbucket.org/t/r/src/HEAD/a.md#lines-3"
        );

        // No line: no anchor and no query, even for Markdown.
        assert_eq!(
            super::web_url(&github, "a.md", None),
            "https://github.com/o/r/blob/HEAD/a.md"
        );
    }

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

    #[test]
    fn parses_a_gitlab_tree_url() {
        // GitLab uses `/-/tree/<branch>/<path>`.
        let remote = parse_target("https://gitlab.com/group/repo/-/tree/main/skills/pdf")
            .unwrap()
            .unwrap();
        assert_eq!(remote.repo_url, "https://gitlab.com/group/repo.git");
        assert_eq!(remote.branch.as_deref(), Some("main"));
        assert_eq!(remote.path.as_deref(), Some("skills/pdf"));
    }

    #[test]
    fn parses_a_bitbucket_src_url() {
        // Bitbucket uses `/src/<branch>/<path>`.
        let remote = parse_target("https://bitbucket.org/team/repo/src/develop/skills/pdf")
            .unwrap()
            .unwrap();
        assert_eq!(remote.repo_url, "https://bitbucket.org/team/repo.git");
        assert_eq!(remote.branch.as_deref(), Some("develop"));
        assert_eq!(remote.path.as_deref(), Some("skills/pdf"));
    }

    #[test]
    fn parses_a_self_hosted_repo_root_url() {
        let remote = parse_target("https://git.example.com/team/skill-repo")
            .unwrap()
            .unwrap();
        assert_eq!(
            remote.repo_url,
            "https://git.example.com/team/skill-repo.git"
        );
        assert_eq!(remote.path, None);
    }

    #[test]
    fn parses_a_direct_dot_git_clone_url() {
        let remote = parse_target("https://git.example.com/team/skill-repo.git")
            .unwrap()
            .unwrap();
        assert_eq!(
            remote.repo_url,
            "https://git.example.com/team/skill-repo.git"
        );
        assert_eq!(remote.path, None);
    }

    #[test]
    fn parses_ssh_shorthand_for_any_host() {
        let remote = parse_target("git@gitlab.com:group/repo.git")
            .unwrap()
            .unwrap();
        assert_eq!(remote.repo_url, "https://gitlab.com/group/repo.git");
    }

    #[test]
    fn a_gitlab_subgroup_repo_clones_the_full_path() {
        // Without a tree marker, a nested group path is the repository itself.
        let remote = parse_target("https://gitlab.com/group/subgroup/repo/-/tree/main/pdf")
            .unwrap()
            .unwrap();
        assert_eq!(
            remote.repo_url,
            "https://gitlab.com/group/subgroup/repo.git"
        );
        assert_eq!(remote.branch.as_deref(), Some("main"));
        assert_eq!(remote.path.as_deref(), Some("pdf"));
    }
}
