use super::{command_inference::ObservedCommand, SynthesizeOptions};
use crate::error::{Error, Result};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

#[derive(Debug)]
pub(super) struct WorkspaceEvidence {
    pub(super) stats: String,
    pub(super) log: String,
    pub(super) meta: String,
    pub(super) deps: Option<String>,
}

pub(super) fn collect_evidence(
    options: &SynthesizeOptions,
    workspace: &str,
) -> Result<WorkspaceEvidence> {
    let context = EvidenceCollectionContext::new(options, workspace);
    let uses_live_required_evidence = options.workspace_stats_report.is_none()
        || options.workspace_log.is_none()
        || options.workspace_meta.is_none();
    let stats = match options.workspace_stats_report.as_deref() {
        Some(path) => read_file(path, "workspace stats")?,
        None => collect_stats(workspace, &context)?,
    };
    let log_last = options.log_last.to_string();
    let log = read_or_collect(
        options.workspace_log.as_deref(),
        "workspace command log",
        &["workspace", "inspect", "log", "--last", &log_last],
        &context,
    )?;
    let meta = read_or_collect(
        options.workspace_meta.as_deref(),
        "workspace metadata",
        &["workspace", "inspect", "meta"],
        &context,
    )?;
    let deps = match options.workspace_deps.as_deref() {
        Some(path) => Some(read_file(path, "workspace dependency graph")?),
        None if uses_live_required_evidence => {
            collect_optional(&["workspace", "inspect", "deps"], &context)
                .ok()
                .filter(|content| !content.trim().is_empty())
        }
        None => None,
    };

    Ok(WorkspaceEvidence {
        stats,
        log,
        meta,
        deps,
    })
}

#[derive(Debug)]
struct EvidenceCollectionContext {
    workspace: String,
    invocation_cwd: String,
    overrides: String,
    candidate_cwds: Vec<PathBuf>,
}

impl EvidenceCollectionContext {
    fn new(options: &SynthesizeOptions, workspace: &str) -> Self {
        let current_dir = env::current_dir().ok();
        let invocation_cwd = current_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<unknown>".to_owned());
        let mut candidate_cwds = Vec::new();
        if let Some(path) = current_dir {
            candidate_cwds.push(path);
        }
        candidate_cwds.extend(discover_workspace_dirs(workspace));
        dedupe_paths(&mut candidate_cwds);
        Self {
            workspace: workspace.to_owned(),
            invocation_cwd,
            overrides: evidence_override_summary(options),
            candidate_cwds,
        }
    }
}

#[derive(Debug)]
struct RoteAttempt {
    cwd: PathBuf,
    exit: Option<i32>,
    stderr: String,
    stdout_empty: bool,
}

fn read_or_collect(
    path: Option<&Path>,
    label: &str,
    rote_args: &[&str],
    context: &EvidenceCollectionContext,
) -> Result<String> {
    match path {
        Some(path) => read_file(path, label),
        None => collect_required(rote_args, label, context),
    }
}

fn collect_stats(workspace: &str, context: &EvidenceCollectionContext) -> Result<String> {
    let primary = ["workspace", "stats", workspace];
    match collect_required(&primary, "workspace stats", context) {
        Ok(stats) => Ok(stats),
        Err(primary_error) => {
            match collect_required(&["workspace", "stats"], "workspace stats", context) {
                Ok(stats) => Ok(stats),
                Err(fallback_error) => Err(Error::InvalidInput {
                    message: format!(
                        "{}\n\nFallback without workspace name also failed:\n{}",
                        primary_error, fallback_error
                    ),
                }),
            }
        }
    }
}

fn read_file(path: &Path, label: &str) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })
        .and_then(|content| {
            if content.trim().is_empty() {
                Err(Error::InvalidInput {
                    message: format!("{label} evidence at {} is empty", path.display()),
                })
            } else {
                Ok(content)
            }
        })
}

fn collect_required(
    args: &[&str],
    label: &str,
    context: &EvidenceCollectionContext,
) -> Result<String> {
    let mut attempts = Vec::new();
    for cwd in &context.candidate_cwds {
        match run_rote(args, cwd) {
            Ok(stdout) => {
                if stdout.trim().is_empty() {
                    attempts.push(RoteAttempt {
                        cwd: cwd.clone(),
                        exit: Some(0),
                        stderr: String::new(),
                        stdout_empty: true,
                    });
                } else {
                    return Ok(stdout);
                }
            }
            Err(attempt) => attempts.push(attempt),
        }
    }
    Err(Error::InvalidInput {
        message: render_collect_failure(args, label, context, &attempts),
    })
}

fn run_rote(args: &[&str], cwd: &Path) -> std::result::Result<String, RoteAttempt> {
    let rote = crate::command_path::find_on_path("rote").unwrap_or_else(|| PathBuf::from("rote"));
    let output = ProcessCommand::new(rote)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|source| RoteAttempt {
            cwd: cwd.to_path_buf(),
            exit: None,
            stderr: source.to_string(),
            stdout_empty: false,
        })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(RoteAttempt {
            cwd: cwd.to_path_buf(),
            exit: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            stdout_empty: false,
        })
    }
}

fn collect_optional(args: &[&str], context: &EvidenceCollectionContext) -> Result<String> {
    collect_required(args, "optional workspace dependency graph", context)
}

fn render_collect_failure(
    args: &[&str],
    label: &str,
    context: &EvidenceCollectionContext,
    attempts: &[RoteAttempt],
) -> String {
    let mut message = format!(
        "`rote {}` failed while collecting {label}.\nsource id: {}\ninvocation cwd: {}\nevidence overrides: {}",
        args.join(" "),
        context.workspace,
        context.invocation_cwd,
        context.overrides
    );
    if attempts.is_empty() {
        message.push_str("\nattempts: none; no candidate source cwd could be resolved");
    } else {
        message.push_str("\nattempts:");
        for attempt in attempts {
            let exit = attempt
                .exit
                .map(|code| code.to_string())
                .unwrap_or_else(|| "not-started".to_owned());
            let stderr = if attempt.stderr.is_empty() {
                "<empty>".to_owned()
            } else {
                attempt.stderr.clone()
            };
            let empty = if attempt.stdout_empty {
                "; stdout was empty"
            } else {
                ""
            };
            message.push_str(&format!(
                "\n- cwd: {}; exit: {exit}{empty}; stderr: {stderr}",
                attempt.cwd.display()
            ));
        }
    }
    message.push_str(
        "\nhint: run from the source directory, or pass --workspace-stats-report, --workspace-log, and --workspace-meta files captured from the completed CLI interaction.",
    );
    message
}

fn evidence_override_summary(options: &SynthesizeOptions) -> String {
    format!(
        "stats={}, log={}, meta={}, deps={}",
        evidence_source(options.workspace_stats_report.as_deref()),
        evidence_source(options.workspace_log.as_deref()),
        evidence_source(options.workspace_meta.as_deref()),
        evidence_source(options.workspace_deps.as_deref())
    )
}

fn evidence_source(path: Option<&Path>) -> String {
    path.map(|path| format!("file:{}", path.display()))
        .unwrap_or_else(|| "live".to_owned())
}

fn discover_workspace_dirs(workspace: &str) -> Vec<PathBuf> {
    let Some(home) = env::var_os("HOME") else {
        return Vec::new();
    };
    let home = PathBuf::from(home);
    ["cursor", "http", "claude", "custom", "rote"]
        .iter()
        .map(|vendor| {
            home.join(".rote")
                .join(vendor)
                .join("workspaces")
                .join(workspace)
        })
        .filter(|path| path.is_dir())
        .collect()
}

fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = BTreeSet::new();
    paths.retain(|path| seen.insert(path.clone()));
}

pub(super) fn validate_evidence(workspace: &str, evidence: &WorkspaceEvidence) -> Result<()> {
    if !mentions_workspace(&evidence.stats, workspace) {
        return Err(Error::InvalidInput {
            message: format!(
                "source metrics evidence does not mention source id {workspace:?}; pass the matching metrics report"
            ),
        });
    }
    if !has_log_entries(&evidence.log) {
        return Err(Error::InvalidInput {
            message: "CLI interaction transcript has no command entries; synthesis needs at least one completed command to learn from".to_owned(),
        });
    }
    if !has_metadata(&evidence.meta) {
        return Err(Error::InvalidInput {
            message: "source metadata evidence is empty or placeholder-only".to_owned(),
        });
    }
    Ok(())
}

pub(super) fn render_synthesis_approval_required(
    evidence: &WorkspaceEvidence,
    commands: &[ObservedCommand],
) -> String {
    format!(
        "CLI interaction approval is required before synthesizing a reusable SkillSpec.\n\nSummary:\nMetrics evidence: present\nCommand transcript: present\nMetadata: present\nDependency graph: {}\nCommand candidates: {}\n\nApprove synthesis only if the command behavior and final output are satisfactory enough to become a reusable, typed workflow.\n\nIf satisfactory, rerun with --observation-approved.",
        if evidence.deps.is_some() {
            "present"
        } else {
            "not supplied"
        },
        commands.len()
    )
}

fn mentions_workspace(text: &str, workspace: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&workspace.to_ascii_lowercase())
}

fn has_metadata(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && trimmed != "[]" && !trimmed.eq_ignore_ascii_case("no rows")
}

fn has_log_entries(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "[]" || trimmed.eq_ignore_ascii_case("no rows") {
        return false;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return json_has_entries(&value);
    }
    trimmed
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.is_empty()
                && !line
                    .chars()
                    .all(|char| matches!(char, '-' | '+' | '|' | ' '))
                && !line.eq_ignore_ascii_case("no rows")
        })
        .count()
        > 1
}

fn json_has_entries(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Array(values) => !values.is_empty(),
        serde_json::Value::Object(map) => {
            if let Some(rows) = map.get("rows").or_else(|| map.get("data")) {
                return json_has_entries(rows);
            }
            !map.is_empty()
        }
        serde_json::Value::Null => false,
        _ => true,
    }
}
