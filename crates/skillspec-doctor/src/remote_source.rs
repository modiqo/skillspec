//! Shape-aware remote staging reports for `skillspec doctor`.
//!
//! The checkout mechanics live in [`skillspec_source::remote`]. This module
//! adds the part that depends on doctor's own shape classification: choosing
//! candidate skill folders inside a staged checkout and rendering the staging
//! report.

use crate::DoctorShapeReport;
use serde::Serialize;
use skillspec_core::error::{Error, Result};
use skillspec_source::remote::{
    clone_remote_persistent, parse_target, set_sparse_path, RemoteSkillSource,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct RemoteStageReport {
    pub target: String,
    pub repo_url: String,
    pub branch: Option<String>,
    pub requested_path: Option<String>,
    pub checkout_dir: String,
    pub staged_source_path: String,
    pub source_shape: RemoteStageShapeReport,
    pub selected_source_path: Option<String>,
    pub candidates: Vec<RemoteStageCandidate>,
    pub next: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RemoteStageShapeReport {
    pub kind: String,
    pub skill_file_count: usize,
    pub plugin_roots: Vec<RemoteStagePluginRootReport>,
    pub recommended_command: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RemoteStagePluginRootReport {
    pub namespace: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RemoteStageCandidate {
    pub skill_path: String,
    pub source_path: String,
}

pub fn stage_remote_source(
    target: &str,
    out: Option<&Path>,
    detect_candidates: bool,
) -> Result<RemoteStageReport> {
    let remote = parse_target(target)?.ok_or_else(|| Error::InvalidInput {
        message: format!(
            "source stage target {target:?} is not a supported public GitHub repo or skill-folder URI"
        ),
    })?;
    let checkout = clone_remote_persistent(&remote, out)?;
    let candidates = materialize_candidates(&remote, &checkout.checkout_dir, detect_candidates)?;
    let selected_source_path = (candidates.len() == 1).then(|| candidates[0].source_path.clone());
    let scoped_source_path = staged_scope_path(&remote, &checkout.checkout_dir);
    let source_shape =
        RemoteStageShapeReport::from_shape(crate::classify_source_shape(&scoped_source_path)?);
    let mut next = Vec::new();
    match &selected_source_path {
        Some(path) => {
            next.push(format!("skillspec doctor {path}"));
            next.push(format!(
                "skillspec source map {path} --out <draft-dir>/.skillspec/source-map"
            ));
            next.push(format!(
                "skillspec import-skill {path} --out <draft-dir>/skill.spec.yml --source-map <draft-dir>/.skillspec/source-map/source-map.json"
            ));
        }
        None if candidates.is_empty() => {
            next.push("No SKILL.md candidates found; verify the URI points at a skill folder or skills repository.".to_owned());
        }
        None => {
            let scope = path_to_string(&scoped_source_path);
            next.push(format!("skillspec doctor {scope} --json"));
            next.push(format!(
                "skillspec workspace map {scope} --out <build>/skillspec.workspace.yml --summary"
            ));
            next.push(
                "If the request was for one atomic skill instead of the staged workspace, choose a specific candidates[].source_path and rerun the single-skill flow."
                    .to_owned(),
            );
        }
    }

    Ok(RemoteStageReport {
        target: target.to_owned(),
        repo_url: remote.repo_url,
        branch: remote.branch,
        requested_path: remote.path,
        checkout_dir: path_to_string(&checkout.checkout_dir),
        staged_source_path: path_to_string(&scoped_source_path),
        source_shape,
        selected_source_path,
        candidates,
        next,
    })
}

pub fn render_stage_report(report: &RemoteStageReport) -> String {
    let mut output = String::new();
    output.push_str("Remote source stage\n\n");
    output.push_str(&format!("- target: {}\n", report.target));
    output.push_str(&format!("- repo: {}\n", report.repo_url));
    if let Some(branch) = &report.branch {
        output.push_str(&format!("- branch: {branch}\n"));
    }
    if let Some(path) = &report.requested_path {
        output.push_str(&format!("- requested_path: {path}\n"));
    }
    output.push_str(&format!("- checkout: {}\n", report.checkout_dir));
    output.push_str(&format!(
        "- staged_source_path: {}\n",
        report.staged_source_path
    ));
    output.push_str(&format!(
        "- source_shape: {} ({} SKILL.md, {} plugin roots)\n",
        report.source_shape.kind,
        report.source_shape.skill_file_count,
        report.source_shape.plugin_roots.len()
    ));
    match &report.selected_source_path {
        Some(path) => output.push_str(&format!("- selected_source_path: {path}\n")),
        None => output.push_str("- selected_source_path: choose from candidates\n"),
    }
    output.push_str(&format!("- candidates: {}\n", report.candidates.len()));
    if !report.candidates.is_empty() {
        output.push_str("\nCandidates:\n");
        for candidate in &report.candidates {
            output.push_str(&format!(
                "- {} -> {}\n",
                candidate.skill_path, candidate.source_path
            ));
        }
    }
    if !report.next.is_empty() {
        output.push_str("\nNext:\n");
        for next in &report.next {
            output.push_str(&format!("- {next}\n"));
        }
    }
    output
}

impl RemoteStageShapeReport {
    fn from_shape(shape: DoctorShapeReport) -> Self {
        Self {
            kind: shape.kind,
            skill_file_count: shape.skill_files.len(),
            plugin_roots: shape
                .plugin_roots
                .into_iter()
                .map(|plugin| RemoteStagePluginRootReport {
                    namespace: plugin.namespace,
                    path: plugin.path,
                })
                .collect(),
            recommended_command: shape.recommended_command,
        }
    }
}

fn materialize_candidates(
    remote: &RemoteSkillSource,
    checkout_dir: &Path,
    detect_candidates: bool,
) -> Result<Vec<RemoteStageCandidate>> {
    if let Some(path) = &remote.path {
        set_sparse_path(checkout_dir, path)?;
        let scope_path = checkout_dir.join(path);
        if !scope_path.exists() {
            return Err(Error::InvalidInput {
                message: format!(
                    "remote path {path} did not materialize from {}",
                    remote.repo_url
                ),
            });
        }
        return find_materialized_candidates(checkout_dir, &scope_path);
    }

    if !detect_candidates {
        return Ok(Vec::new());
    }

    find_materialized_candidates(checkout_dir, checkout_dir)
}

fn staged_scope_path(remote: &RemoteSkillSource, checkout_dir: &Path) -> PathBuf {
    remote
        .path
        .as_deref()
        .map(|path| checkout_dir.join(path))
        .unwrap_or_else(|| checkout_dir.to_path_buf())
}

fn find_materialized_candidates(
    checkout_dir: &Path,
    scope_path: &Path,
) -> Result<Vec<RemoteStageCandidate>> {
    let mut skill_paths = Vec::new();
    collect_skill_files(scope_path, &mut skill_paths)?;
    skill_paths.sort();
    Ok(skill_paths
        .into_iter()
        .filter_map(|path| path.strip_prefix(checkout_dir).ok().map(Path::to_path_buf))
        .map(|skill_path| candidate_from_skill_path(checkout_dir, &skill_path))
        .collect())
}

fn collect_skill_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        collect_skill_files(&entry.path(), files)?;
    }
    Ok(())
}

fn candidate_from_skill_path(checkout_dir: &Path, skill_path: &Path) -> RemoteStageCandidate {
    let source_path = skill_path
        .parent()
        .map(|parent| checkout_dir.join(parent))
        .unwrap_or_else(|| checkout_dir.to_path_buf());
    RemoteStageCandidate {
        skill_path: path_to_slash(skill_path),
        source_path: path_to_string(&source_path),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => part.to_str().map(str::to_owned),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::materialize_candidates;
    use skillspec_source::remote::{clone_remote_into, unique_nanos, RemoteSkillSource};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    #[test]
    fn repo_root_stage_materializes_plugin_metadata() {
        let sandbox = test_sandbox("repo-root-stage");
        let origin = sandbox.join("origin");
        write_file(
            &origin.join(".agent-plugin").join("marketplace.json"),
            r#"{"name":"neutral-marketplace"}"#,
        );
        write_file(&origin.join("README.md"), "# Skills\n");
        write_file(
            &origin.join("skills").join("one").join("SKILL.md"),
            "---\nname: one\ndescription: One.\n---\n# One\n",
        );
        write_file(
            &origin.join("skills").join("two").join("SKILL.md"),
            "---\nname: two\ndescription: Two.\n---\n# Two\n",
        );
        commit_repo(&origin);

        let remote = RemoteSkillSource {
            repo_url: origin.to_string_lossy().into_owned(),
            branch: None,
            path: None,
        };
        let checkout = clone_remote_into(&remote, &sandbox.join("stage")).unwrap();
        let candidates = materialize_candidates(&remote, &checkout.checkout_dir, true).unwrap();

        assert!(checkout
            .checkout_dir
            .join(".agent-plugin")
            .join("marketplace.json")
            .is_file());
        assert!(checkout.checkout_dir.join("README.md").is_file());
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].skill_path, "skills/one/SKILL.md");
        assert_eq!(candidates[1].skill_path, "skills/two/SKILL.md");
        let _ = fs::remove_dir_all(sandbox);
    }

    #[test]
    fn subfolder_stage_materializes_only_requested_scope() {
        let sandbox = test_sandbox("subfolder-stage");
        let origin = sandbox.join("origin");
        write_file(
            &origin.join(".agent-plugin").join("marketplace.json"),
            r#"{"name":"repo-root"}"#,
        );
        write_file(
            &origin
                .join("packages")
                .join("plugin")
                .join(".agent-plugin")
                .join("marketplace.json"),
            r#"{"name":"scoped-plugin"}"#,
        );
        write_file(
            &origin
                .join("packages")
                .join("plugin")
                .join("skills")
                .join("one")
                .join("SKILL.md"),
            "---\nname: one\ndescription: One.\n---\n# One\n",
        );
        write_file(
            &origin.join("outside").join("SKILL.md"),
            "---\nname: outside\ndescription: Outside.\n---\n# Outside\n",
        );
        commit_repo(&origin);

        let remote = RemoteSkillSource {
            repo_url: origin.to_string_lossy().into_owned(),
            branch: None,
            path: Some("packages/plugin".to_owned()),
        };
        let checkout = clone_remote_into(&remote, &sandbox.join("stage")).unwrap();
        let candidates = materialize_candidates(&remote, &checkout.checkout_dir, true).unwrap();

        assert!(!checkout
            .checkout_dir
            .join(".agent-plugin")
            .join("marketplace.json")
            .exists());
        assert!(checkout
            .checkout_dir
            .join("packages")
            .join("plugin")
            .join(".agent-plugin")
            .join("marketplace.json")
            .is_file());
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].skill_path,
            "packages/plugin/skills/one/SKILL.md"
        );
        assert!(!checkout
            .checkout_dir
            .join("outside")
            .join("SKILL.md")
            .exists());
        let _ = fs::remove_dir_all(sandbox);
    }

    fn test_sandbox(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "skillspec-remote-source-{name}-{}-{}",
            std::process::id(),
            unique_nanos()
        ))
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn commit_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();
        run_git(path, &["init"]);
        run_git(path, &["config", "user.email", "skillspec@example.invalid"]);
        run_git(path, &["config", "user.name", "SkillSpec Test"]);
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "initial"]);
    }

    fn run_git(path: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
