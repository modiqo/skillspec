use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitContext {
    pub root: PathBuf,
    pub branch: Option<String>,
    pub remote_url: Option<String>,
}

pub fn discover(start: &Path) -> Option<GitContext> {
    let start = normalize_path(start);
    let start_dir = if start.is_dir() {
        start
    } else {
        start.parent()?.to_path_buf()
    };

    for directory in start_dir.ancestors() {
        let marker = directory.join(".git");
        if !marker.exists() {
            continue;
        }
        let git_dir = resolve_git_dir(directory, &marker)?;
        return Some(GitContext {
            root: directory.to_path_buf(),
            branch: read_branch(&git_dir),
            remote_url: read_origin_remote(&git_dir),
        });
    }

    None
}

pub fn port_pull_request_next_steps(
    source: &Path,
    generated_root: &Path,
    artifacts: &[&Path],
) -> Vec<String> {
    let Some(context) = discover(source) else {
        return Vec::new();
    };

    let source_label = relative_to(&context.root, source)
        .map(path_label)
        .unwrap_or_else(|| path_label(source));
    let generated_label = relative_to(&context.root, generated_root)
        .map(path_label)
        .unwrap_or_else(|| path_label(generated_root));
    let in_repo_artifacts = artifacts
        .iter()
        .filter_map(|path| relative_to(&context.root, path))
        .map(path_label)
        .collect::<Vec<_>>();

    let mut steps = Vec::new();
    steps.push(format!(
        "detected source git repo {}; after semantic review, QA, compile, install, harness restart, and a real agent interaction with the SkillSpec-backed skill, open a PR so the contract is not left only in the local draft",
        repo_label(&context)
    ));

    if in_repo_artifacts.is_empty() {
        steps.push(format!(
            "generated artifacts are outside the source checkout; copy the reviewed `skill.spec.yml`, compiled `SKILL.md` loader, `deps.toml`, preserved source, and proof reports from {generated_label} into {source_label} before committing"
        ));
    } else {
        steps.push(format!(
            "suggested PR contents from this run: {}",
            in_repo_artifacts.join(", ")
        ));
    }

    steps.push(format!(
        "recommended PR flow: create branch `skillspec/{}`, commit the reviewed contract artifacts plus Doctor/alignment reports after the restarted harness proves the skill experience, push to the detected remote, and open a pull request",
        branch_slug(source)
    ));
    steps
}

pub fn workspace_pull_request_next_steps(source_root: &Path, build_root: &Path) -> Vec<String> {
    let Some(context) = discover(source_root) else {
        return Vec::new();
    };
    let source_label = relative_to(&context.root, source_root)
        .map(path_label)
        .unwrap_or_else(|| path_label(source_root));
    let build_label = relative_to(&context.root, build_root)
        .map(path_label)
        .unwrap_or_else(|| path_label(build_root));

    vec![
        format!(
            "detected source git repo {}; after workspace QA, install, harness restart, and real agent interactions with the installed SkillSpec-backed skills, open a PR with the generated package contracts instead of leaving them only in {build_label}",
            repo_label(&context)
        ),
        format!(
            "copy reviewed package `skill.spec.yml` files, compiled `SKILL.md` loaders, dependency ledgers, and workspace Doctor/alignment reports from {build_label} into the source workspace at {source_label} before committing"
        ),
        format!(
            "recommended PR flow: create branch `skillspec/{}`, commit package contracts and proof artifacts after the restarted harness proves the skill experience, push to the detected remote, and open a pull request",
            branch_slug(source_root)
        ),
    ]
}

fn resolve_git_dir(repo_root: &Path, marker: &Path) -> Option<PathBuf> {
    if marker.is_dir() {
        return Some(marker.to_path_buf());
    }
    let content = fs::read_to_string(marker).ok()?;
    let git_dir = content.trim().strip_prefix("gitdir:")?.trim();
    let path = PathBuf::from(git_dir);
    Some(if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    })
}

fn read_branch(git_dir: &Path) -> Option<String> {
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    head.strip_prefix("ref: refs/heads/")
        .filter(|branch| !branch.trim().is_empty())
        .map(str::to_owned)
}

fn read_origin_remote(git_dir: &Path) -> Option<String> {
    let config = fs::read_to_string(git_dir.join("config")).ok()?;
    let mut in_origin = false;
    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_origin = trimmed == r#"[remote "origin"]"#;
            continue;
        }
        if !in_origin {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() == "url" {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn repo_label(context: &GitContext) -> String {
    let mut label = format!("root={}", context.root.display());
    if let Some(branch) = &context.branch {
        label.push_str(&format!(" branch={branch}"));
    }
    if let Some(remote) = &context.remote_url {
        label.push_str(&format!(" remote={}", sanitize_remote_url(remote)));
    }
    label
}

fn sanitize_remote_url(remote: &str) -> String {
    let Some(scheme_end) = remote.find("://") else {
        return remote.to_owned();
    };
    let authority_start = scheme_end + "://".len();
    let Some(at_offset) = remote[authority_start..].find('@') else {
        return remote.to_owned();
    };
    let host_start = authority_start + at_offset + 1;
    format!("{}{}", &remote[..authority_start], &remote[host_start..])
}

fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(path) = path.canonicalize() {
        return path;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn relative_to(root: &Path, path: &Path) -> Option<PathBuf> {
    let root = normalize_path(root);
    let path = normalize_path(path);
    path.strip_prefix(root).ok().map(Path::to_path_buf)
}

fn path_label(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    let text = path.display().to_string();
    if text.is_empty() {
        ".".to_owned()
    } else {
        text
    }
}

fn branch_slug(path: &Path) -> String {
    let normalized = normalize_path(path);
    let name = normalized
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill");
    let slug = name
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "skill".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_basic_git_context_and_pr_guidance() {
        let root = temp_root("skillspec-git-context");
        let skill = root.join("skills/pdf");
        fs::create_dir_all(skill.join(".skillspec/port")).expect("create skill");
        fs::create_dir_all(root.join(".git")).expect("create git dir");
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").expect("write head");
        fs::write(
            root.join(".git/config"),
            "[remote \"origin\"]\n\turl = https://github.com/example/skills.git\n",
        )
        .expect("write config");
        fs::write(skill.join("skill.spec.yml"), "schema: skillspec/v0\n").expect("write spec");

        let context = discover(&skill).expect("git context");
        assert_eq!(context.root, root.canonicalize().expect("canonical root"));
        assert_eq!(context.branch.as_deref(), Some("main"));
        assert_eq!(
            context.remote_url.as_deref(),
            Some("https://github.com/example/skills.git")
        );

        let spec = skill.join("skill.spec.yml");
        let steps = port_pull_request_next_steps(&skill, &skill, &[&spec]);
        assert!(steps.iter().any(|step| step.contains("open a PR")));
        assert!(steps
            .iter()
            .any(|step| step.contains("skills/pdf/skill.spec.yml")));
    }

    #[test]
    fn repo_label_does_not_echo_remote_credentials() {
        let context = GitContext {
            root: PathBuf::from("/repo"),
            branch: Some("main".to_owned()),
            remote_url: Some("https://user:secret@example.com/org/repo.git".to_owned()),
        };

        let label = repo_label(&context);

        assert!(label.contains("https://example.com/org/repo.git"));
        assert!(!label.contains("user:secret"));
    }

    fn temp_root(prefix: &str) -> PathBuf {
        let mut root = env::temp_dir();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        root.push(format!("{prefix}-{now}"));
        root
    }
}
