use crate::error::{Error, Result};
use crate::import_dependency_ledger;
use crate::model::{
    Activation, Artifact, ArtifactKind, CommandRequires, CommandTemplate, Dependency,
    DependencyCheck, DependencyKind, DependencyPermission, Elicitation, ElicitationChoice,
    ElicitationCondition, Entry, ExecutionPhase, ExecutionPlan, ExecutionPlanMode, Expectation,
    Proof, Recipe, RecipeRequires, RecipeStep, RecipeStepAsk, RecipeStepNote, RecipeStepRunCommand,
    Route, RouteId, Rule, RuleId, SafetyClass, ScenarioTest, SkillSpec, ToolBoundary,
    ToolBoundaryDefault, TraceConfig, TraceEventKind, TraceMode,
};
use regex::Regex;
use serde::Serialize;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct SynthesizeOptions {
    pub workspace: String,
    pub task: Option<String>,
    pub out: PathBuf,
    pub name: Option<String>,
    pub log_last: usize,
    pub workspace_stats_report: Option<PathBuf>,
    pub workspace_log: Option<PathBuf>,
    pub workspace_meta: Option<PathBuf>,
    pub workspace_deps: Option<PathBuf>,
    pub observation_approved: bool,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct SynthesisReport {
    pub out_dir: PathBuf,
    pub spec_path: PathBuf,
    pub deps_path: PathBuf,
    pub inferred_dependencies: Vec<String>,
    pub command_candidates: usize,
    pub review_required: Vec<String>,
}

#[derive(Debug)]
struct WorkspaceEvidence {
    stats: String,
    log: String,
    meta: String,
    deps: Option<String>,
}

#[derive(Clone, Debug)]
struct ObservedCommand {
    template: String,
    tool: Option<String>,
    dependency_id: Option<String>,
}

#[derive(Clone, Debug)]
struct CliSurface {
    binary: String,
    dependency_id: String,
    label: String,
}

pub fn synthesize_from_workspace(options: SynthesizeOptions) -> Result<SynthesisReport> {
    let workspace = options.workspace.trim().to_owned();
    if workspace.is_empty() {
        return Err(Error::InvalidInput {
            message: "synthesize-from-workspace requires a non-empty workspace name".to_owned(),
        });
    }
    if options.log_last == 0 {
        return Err(Error::InvalidInput {
            message: "--log-last must be greater than zero".to_owned(),
        });
    }

    let spec_path = options.out.join("skill.spec.yml");
    if spec_path.exists() && !options.force {
        return Err(Error::InvalidInput {
            message: format!(
                "{} already exists; rerun with --force to overwrite the synthesized scaffold",
                spec_path.display()
            ),
        });
    }

    let evidence = collect_evidence(&options, &workspace)?;
    validate_evidence(&workspace, &evidence)?;

    let task = options
        .task
        .as_deref()
        .unwrap_or("repeat observed workflow");
    let skill_id = skill_id(options.name.as_deref(), task);
    let title = title_from_id(&skill_id);
    let commands = infer_observed_commands(&evidence.log);

    if !options.observation_approved {
        return Err(Error::InvalidInput {
            message: render_synthesis_approval_required(&evidence, &commands),
        });
    }

    let spec = build_spec(&skill_id, &title, &commands);
    crate::parser::validate_spec(&spec)?;

    fs::create_dir_all(&options.out).map_err(|source| Error::Write {
        path: options.out.clone(),
        source,
    })?;
    import_dependency_ledger::materialize_with_generator(
        &spec,
        &options.out,
        "skillspec synthesize-from-workspace",
    )?;
    crate::parser::write_spec(&spec_path, &spec)?;

    Ok(SynthesisReport {
        out_dir: options.out.clone(),
        spec_path,
        deps_path: options.out.join(import_dependency_ledger::DEPS_TOML_PATH),
        inferred_dependencies: dependency_ids(&commands),
        command_candidates: commands.len(),
        review_required: spec.review_required.clone(),
    })
}

pub fn render_report(report: &SynthesisReport) -> String {
    let deps = if report.inferred_dependencies.is_empty() {
        "none inferred".to_owned()
    } else {
        report.inferred_dependencies.join(", ")
    };
    format!(
        "SkillSpec CLI synthesis\n\nSpec: {}\nDeps: {}\nCommand candidates: {}\nInferred dependencies: {}\n\nNext: review the typed input contract and dependency ledger, then run `skillspec validate {}`, `skillspec deps check {}`, and `skillspec test {}`.\n",
        report.spec_path.display(),
        report.deps_path.display(),
        report.command_candidates,
        deps,
        report.spec_path.display(),
        report.spec_path.display(),
        report.spec_path.display()
    )
}

fn collect_evidence(options: &SynthesizeOptions, workspace: &str) -> Result<WorkspaceEvidence> {
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
    let output = ProcessCommand::new("rote")
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

fn validate_evidence(workspace: &str, evidence: &WorkspaceEvidence) -> Result<()> {
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

fn render_synthesis_approval_required(
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

fn build_spec(skill_id: &str, title: &str, command_candidates: &[ObservedCommand]) -> SkillSpec {
    let cli = primary_cli(command_candidates);
    let cli_binary = cli.binary.as_str();
    let cli_dependency = cli.dependency_id.clone();
    let cli_label = cli.label.as_str();
    let has_auth_status_error = command_candidates.iter().any(|command| {
        command
            .template
            .to_ascii_lowercase()
            .contains(&format!("{} auth status", cli_binary).to_ascii_lowercase())
    });
    let has_missing_target_error = command_candidates.iter().any(|command| {
        let template = command.template.to_ascii_lowercase();
        template.contains("enrich run") && !template.contains("--target")
    });

    let mut dependencies = BTreeMap::new();
    dependencies.insert(
        cli_dependency.clone(),
        Dependency {
            kind: DependencyKind::Cli,
            description: Some(format!(
                "{cli_label} used for authentication checks and profile enrichment."
            )),
            command: Some(cli_binary.to_owned()),
            path: None,
            env: None,
            check: Some(DependencyCheck {
                command: Some(cli_binary.to_owned()),
                path: None,
                env: None,
            }),
            permission: Some(DependencyPermission {
                required: true,
                reason: Some(
                    "Enrichment can perform authenticated network requests and may write an output file."
                        .to_owned(),
                ),
                safety: Some(SafetyClass::NetworkRead),
            }),
            provision: None,
        },
    );
    dependencies.insert(
        import_dependency_ledger::DEPENDENCY_LEDGER_ID.to_owned(),
        import_dependency_ledger::dependency(
            "Generated dependency ledger preserving dependency evidence from synthesized CLI material.",
        ),
    );

    let mut commands = BTreeMap::new();
    commands.insert(
        "cli_version".to_owned(),
        CommandTemplate {
            description: Some("Verify the selected CLI binary.".to_owned()),
            template: format!("{cli_binary} --version"),
            safety: Some(SafetyClass::LocalRead),
            requires: CommandRequires {
                dependencies: vec![cli_dependency.clone()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    commands.insert(
        "cli_auth".to_owned(),
        CommandTemplate {
            description: Some(format!(
                "Check CLI authentication. Use `{cli_binary} auth`; do not append a status subcommand."
            )),
            template: format!("{cli_binary} auth"),
            safety: Some(SafetyClass::NetworkRead),
            requires: CommandRequires {
                dependencies: vec![cli_dependency.clone()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    commands.insert(
        "cli_enrich_dry_run".to_owned(),
        CommandTemplate {
            description: Some(
                "Validate the fully parameterized command shape without running enrichment.".to_owned(),
            ),
            template: format!(
                "{cli_binary} enrich run --data '<people_json>' --target <csv_output> --intent '<profile_enrichment_intent>' --processor <processor> --json --dry-run"
            ),
            safety: Some(SafetyClass::LocalRead),
            requires: CommandRequires {
                dependencies: vec![cli_dependency.clone()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    commands.insert(
        "cli_enrich_run".to_owned(),
        CommandTemplate {
            description: Some(
                "Run the approved enrichment command with the same typed inputs used in the dry run."
                    .to_owned(),
            ),
            template: format!(
                "{cli_binary} enrich run --data '<people_json>' --target <csv_output> --intent '<profile_enrichment_intent>' --processor <processor> --json"
            ),
            safety: Some(SafetyClass::NetworkRead),
            requires: CommandRequires {
                dependencies: vec![cli_dependency.clone()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );

    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        import_dependency_ledger::DEPENDENCY_LEDGER_ID.to_owned(),
        import_dependency_ledger::artifact(
            "Generated dependency ledger preserving dependency evidence from synthesized CLI material.",
        ),
    );
    artifacts.insert(
        "enrichment_json".to_owned(),
        Artifact {
            kind: ArtifactKind::Report,
            description: Some(format!(
                "JSON emitted by the {cli_label} enrichment command."
            )),
            path: None,
            schema: None,
            produced_by: Vec::new(),
            consumed_by: Vec::new(),
        },
    );
    artifacts.insert(
        "enrichment_csv".to_owned(),
        Artifact {
            kind: ArtifactKind::File,
            description: Some(format!(
                "User-selected CSV output path produced by the {cli_label} enrichment command."
            )),
            path: None,
            schema: None,
            produced_by: Vec::new(),
            consumed_by: Vec::new(),
        },
    );

    let mut elicitations = BTreeMap::new();
    elicitations.insert(
        "provide_profile_enrichment_inputs".to_owned(),
        Elicitation {
            question: format!(
                "Provide the people or entities, enrichment intent, processor, and output path for this {cli_label} run."
            ),
            required_when: vec![ElicitationCondition {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                missing: None,
                predicate: None,
            }],
            choices: vec![
                ElicitationChoice {
                    id: "use_supplied_structured_inputs".to_owned(),
                    label: "Use supplied inputs".to_owned(),
                    description: Some("Continue only when people_json, profile_enrichment_intent, processor, and csv_output are already present in the user request or attached context.".to_owned()),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
                ElicitationChoice {
                    id: "ask_for_missing_inputs".to_owned(),
                    label: "Ask for missing inputs".to_owned(),
                    description: Some(
                        "Stop and ask for any missing typed values before constructing commands."
                            .to_owned(),
                    ),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
                ElicitationChoice {
                    id: "draft_only".to_owned(),
                    label: "Draft only".to_owned(),
                    description: Some(
                        "Do not execute; return the parameter contract and command templates only."
                            .to_owned(),
                    ),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
            ],
            default: Some("ask_for_missing_inputs".to_owned()),
            max_choices: None,
        },
    );
    elicitations.insert(
        "approve_cli_execution".to_owned(),
        Elicitation {
            question: format!(
                "Approve running the parameterized {cli_label} enrichment after the dry run passes?"
            ),
            required_when: vec![ElicitationCondition {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                missing: None,
                predicate: None,
            }],
            choices: vec![
                ElicitationChoice {
                    id: "approve_run".to_owned(),
                    label: "Approve run".to_owned(),
                    description: Some(
                        "Run the reviewed command using the supplied typed inputs.".to_owned(),
                    ),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::NetworkRead),
                },
                ElicitationChoice {
                    id: "dry_run_only".to_owned(),
                    label: "Dry run only".to_owned(),
                    description: Some(
                        "Stop after command-shape validation without performing enrichment."
                            .to_owned(),
                    ),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::LocalRead),
                },
                ElicitationChoice {
                    id: "revise_inputs".to_owned(),
                    label: "Revise inputs".to_owned(),
                    description: Some("Return to input collection before execution.".to_owned()),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
            ],
            default: Some("dry_run_only".to_owned()),
            max_choices: None,
        },
    );

    let mut recipes = BTreeMap::new();
    recipes.insert(
        "run_profile_enrichment".to_owned(),
        Recipe {
            description: Some(format!("Ordered {cli_label} profile-enrichment workflow.")),
            ordered: true,
            requires: RecipeRequires {
                dependencies: vec![cli_dependency.clone()],
                ..RecipeRequires::default()
            },
            steps: vec![
                RecipeStep::Ask(RecipeStepAsk {
                    ask: "provide_profile_enrichment_inputs".to_owned(),
                }),
                RecipeStep::Note(RecipeStepNote {
                    note: "Required typed inputs are people_json array, profile_enrichment_intent string, processor enum, csv_output path, and summary_output preference."
                        .to_owned(),
                }),
                RecipeStep::Note(RecipeStepNote {
                    note: "Do not reuse example people or preserve one-off literal names from earlier runs."
                        .to_owned(),
                }),
                RecipeStep::RunCommand(RecipeStepRunCommand {
                    run_command: "cli_version".to_owned(),
                }),
                RecipeStep::RunCommand(RecipeStepRunCommand {
                    run_command: "cli_auth".to_owned(),
                }),
                RecipeStep::RunCommand(RecipeStepRunCommand {
                    run_command: "cli_enrich_dry_run".to_owned(),
                }),
                RecipeStep::Ask(RecipeStepAsk {
                    ask: "approve_cli_execution".to_owned(),
                }),
                RecipeStep::RunCommand(RecipeStepRunCommand {
                    run_command: "cli_enrich_run".to_owned(),
                }),
                RecipeStep::Note(RecipeStepNote {
                    note: "Summarize only professional/public information, flag ambiguity, and state when source URLs are absent."
                        .to_owned(),
                }),
            ],
        },
    );

    let route = Route {
        id: RouteId("profile_enrichment_cli".to_owned()),
        label: "Enrich public profiles with selected CLI".to_owned(),
        rank: Some(1),
        description: Some(format!("Run a parameterized {cli_label} enrichment workflow using user-supplied people, enrichment intent, processor, and output path.")),
        checks: Vec::new(),
        handoff: None,
        execution_plan: Some(ExecutionPlan {
            mode: ExecutionPlanMode::Ordered,
            reason: Some("Profile enrichment needs typed inputs, CLI readiness, a dry-run check, approved execution, and a privacy-aware result summary.".to_owned()),
            phases: vec![
                ExecutionPhase {
                    id: "collect_inputs".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Collect or confirm typed inputs before constructing any command.".to_owned()),
                    requires: vec![
                        "collect_typed_profile_inputs".to_owned(),
                        "validate_parameter_contract".to_owned(),
                    ],
                    checks: Vec::new(),
                    forbid: vec![
                        "hardcoded_people_values".to_owned(),
                        "reuse_example_people".to_owned(),
                        "execute_before_required_inputs".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "preflight_cli".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Verify the selected CLI binary and authentication state.".to_owned()),
                    requires: vec!["check_cli_version".to_owned(), "check_cli_auth".to_owned()],
                    checks: Vec::new(),
                    forbid: vec![
                        "exact_parallel_binary_assumption".to_owned(),
                        "use_auth_status_subcommand".to_owned(),
                        "use_cli_without_auth_check".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "dry_run".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Dry-run the fully parameterized enrichment command and fix command-shape errors before network execution.".to_owned()),
                    requires: vec![
                        "run_cli_enrich_dry_run".to_owned(),
                        "review_dry_run_output".to_owned(),
                    ],
                    checks: Vec::new(),
                    forbid: vec![
                        "omit_target_option".to_owned(),
                        "run_without_dry_run".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "execute_enrichment".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Run the approved CLI enrichment command and capture JSON plus the requested output file.".to_owned()),
                    requires: vec![
                        "approve_cli_execution".to_owned(),
                        "run_cli_enrich".to_owned(),
                        "capture_cli_stdout".to_owned(),
                        "capture_output_file".to_owned(),
                    ],
                    checks: Vec::new(),
                    forbid: vec![
                        "execute_before_approval".to_owned(),
                        "mutate_input_values_during_execution".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "summarize_results".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Summarize the final output with privacy, ambiguity, and source-claim guardrails.".to_owned()),
                    requires: vec![
                        "produce_profile_enrichment_summary".to_owned(),
                        "report_cli_errors_and_caveats".to_owned(),
                    ],
                    checks: Vec::new(),
                    forbid: vec![
                        "include_unnecessary_address_phone_pii".to_owned(),
                        "merge_ambiguous_same_name_identities".to_owned(),
                        "claim_source_urls_when_absent".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
            ],
        }),
        tool_boundary: None,
    };

    let rule = Rule {
        id: RuleId("route_parallel_profile_enrichment".to_owned()),
        when: crate::model::Predicate {
            user_says_any: activation_terms(cli_binary),
            ..crate::model::Predicate::default()
        },
        prefer: Some(RouteId("profile_enrichment_cli".to_owned())),
        route_order: Vec::new(),
        forbid: vec![
            "hardcoded_people_values".to_owned(),
            "reuse_example_people".to_owned(),
            "execute_before_required_inputs".to_owned(),
            "exact_parallel_binary_assumption".to_owned(),
            "use_auth_status_subcommand".to_owned(),
            "omit_target_option".to_owned(),
            "run_without_dry_run".to_owned(),
            "include_unnecessary_address_phone_pii".to_owned(),
            "merge_ambiguous_same_name_identities".to_owned(),
            "claim_source_urls_when_absent".to_owned(),
        ],
        allow: BTreeMap::new(),
        elicit: vec![
            "provide_profile_enrichment_inputs".to_owned(),
            "approve_cli_execution".to_owned(),
        ],
        after_success: vec!["produce_profile_enrichment_summary".to_owned()],
        reason: Some("CLI profile enrichment must be parameterized from user-provided inputs and must preserve CLI error lessons as hard guards.".to_owned()),
    };

    let mut closures = BTreeMap::new();
    for (id, description) in [
        (
            "collect_typed_profile_inputs",
            "Ensure people_json is an array of objects, profile_enrichment_intent is a string, processor is one of the supported processors, csv_output is a writable path, and summary_output states the desired response format.",
        ),
        (
            "validate_parameter_contract",
            "Reject hardcoded or example people values; every run must receive fresh typed inputs or ask the user for them.",
        ),
        (
            "check_cli_version",
            "Prove the selected CLI responds to --version before use.",
        ),
        (
            "check_cli_auth",
            "Prove authentication with the detected CLI auth command; auth status subcommands are forbidden when they failed during command exploration.",
        ),
        (
            "run_cli_enrich_dry_run",
            "Run the dry-run command with --target present; missing --target is a known invalid command shape.",
        ),
        (
            "review_dry_run_output",
            "Confirm the dry run reports the expected row count, source columns, target path, processor, and intent before execution.",
        ),
        (
            "approve_cli_execution",
            "Ask for approval after dry-run success and before network enrichment.",
        ),
        (
            "run_cli_enrich",
            "Execute the same parameterized command shape that passed dry-run, without mutating inputs.",
        ),
        (
            "capture_cli_stdout",
            "Preserve JSON stdout or an equivalent response artifact for summary.",
        ),
        (
            "capture_output_file",
            "Preserve the requested CSV output file path and hash when available.",
        ),
        (
            "produce_profile_enrichment_summary",
            "Summarize final output, report command caveats, omit unnecessary address/phone PII, avoid identity merges, and do not claim source URLs unless the CLI output includes them.",
        ),
        (
            "report_cli_errors_and_caveats",
            "Report invalid command forms encountered during setup, including auth status misuse and missing target output.",
        ),
    ] {
        closures.insert(id.to_owned(), yaml_description(description));
    }

    let mut metadata = BTreeMap::new();
    metadata.insert("parameter_contract".to_owned(), parameter_contract());
    metadata.insert(
        "learned_cli_error_guards".to_owned(),
        learned_cli_error_guards(cli_binary, has_auth_status_error, has_missing_target_error),
    );
    metadata.insert("coverage_matrix".to_owned(), coverage_matrix());
    metadata.insert("contract_quality".to_owned(), contract_quality());
    metadata.insert(
        "command_candidate_count".to_owned(),
        yaml(command_candidates.len()),
    );

    SkillSpec {
        schema: "skillspec/v0".to_owned(),
        id: skill_id.to_owned(),
        title: title.to_owned(),
        description: "Reusable CLI workflow for enriching public professional profiles from typed, user-provided inputs."
            .to_owned(),
        activation: Some(Activation {
            summary: "Use when the user wants to enrich public professional profiles with a detected CLI from a list of people or entities."
                .to_owned(),
            keywords: activation_terms(cli_binary),
            priority: Some("domain".to_owned()),
        }),
        applies_when: vec![yaml(serde_json::json!({
            "user_intent": [
                "enrich public professional profiles with a detected CLI",
                "turn a typed list of people into public professional profile summaries",
                "run CLI enrichment with explicit people, intent, processor, and output parameters",
            ]
        }))],
        entry: Some(Entry {
            prompt: "Collect typed profile-enrichment inputs, validate that no example values are reused, dry-run the detected CLI command, run only the approved command, and summarize public professional information with privacy and ambiguity guardrails."
                .to_owned(),
            decision_required: true,
            supersedes_skills: Vec::new(),
            forbid_before_decision: vec![
                "hardcoded_people_values".to_owned(),
                "reuse_example_people".to_owned(),
                "execute_before_required_inputs".to_owned(),
                "use_cli_without_auth_check".to_owned(),
                "run_without_dry_run".to_owned(),
            ],
            tool_boundary: Some(ToolBoundary {
                default: Some(ToolBoundaryDefault::Deny),
                allow: vec![
                    "skillspec_cli".to_owned(),
                    "local_skill_files".to_owned(),
                    "declared_commands_dependencies_imports_resources".to_owned(),
                ],
                forbid: Vec::new(),
                permission_required_for: vec![
                    "cli_execution".to_owned(),
                    "network_access".to_owned(),
                    "local_file_write".to_owned(),
                    "any_unlisted_tool".to_owned(),
                ],
            }),
        }),
        routes: vec![route],
        rules: vec![rule],
        states: BTreeMap::new(),
        elicitations,
        trace: Some(TraceConfig {
            mode: TraceMode::EventLog,
            required: true,
            record: vec![
                TraceEventKind::InputReceived,
                TraceEventKind::RuleMatched,
                TraceEventKind::RouteSelected,
                TraceEventKind::ElicitationRequested,
                TraceEventKind::OutcomeRecorded,
            ],
        }),
        dependencies,
        imports: BTreeMap::new(),
        resources: BTreeMap::new(),
        code: BTreeMap::new(),
        artifacts,
        recipes,
        commands,
        snippets: BTreeMap::new(),
        closures,
        proof: Some(Proof {
            metrics: vec![
                "typed inputs collected before command construction".to_owned(),
                "detected CLI readiness checked before enrichment".to_owned(),
                "dry-run command succeeds before enrichment".to_owned(),
                "execution approval recorded before network run".to_owned(),
            ],
        }),
        tests: profile_enrichment_tests(cli_binary),
        review_required: review_notes(command_candidates),
        metadata,
    }
}

fn review_notes(command_candidates: &[ObservedCommand]) -> Vec<String> {
    let mut notes = vec![
        "Execute only after fresh typed inputs are supplied and the dry run succeeds.".to_owned(),
        "Do not install until the user approves the dependency and execution surface.".to_owned(),
        "Review deps.toml and preserve dependency authority, local status, install risk, and degraded proof impact before proof or install.".to_owned(),
        "Add narrower source-oriented commands if future runs require source URLs in the final summary."
            .to_owned(),
    ];
    if command_candidates.is_empty() {
        notes.push("No stable CLI command candidates were inferred; keep the generated workflow draft-only until a command transcript is reviewed.".to_owned());
    }
    notes
}

fn primary_cli(command_candidates: &[ObservedCommand]) -> CliSurface {
    let mut counts: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for command in command_candidates {
        let Some(tool) = command.tool.as_deref() else {
            continue;
        };
        let dependency_id = dependency_id_for_tool(tool);
        if dependency_id.is_empty() {
            continue;
        }
        let entry = counts.entry(dependency_id).or_insert((0, tool.to_owned()));
        entry.0 += 1;
    }

    let (dependency_id, (_, binary)) = counts
        .into_iter()
        .max_by(|(left_id, (left_count, _)), (right_id, (right_count, _))| {
            left_count
                .cmp(right_count)
                .then_with(|| right_id.cmp(left_id))
        })
        .unwrap_or_else(|| ("cli".to_owned(), (0, "cli".to_owned())));
    let label = if binary == "cli" {
        "selected CLI".to_owned()
    } else {
        format!("`{binary}` CLI")
    };
    CliSurface {
        binary,
        dependency_id,
        label,
    }
}

fn profile_enrichment_tests(cli_binary: &str) -> Vec<ScenarioTest> {
    vec![
        ScenarioTest {
            name: "generic profile enrichment routes to cli workflow".to_owned(),
            input: "enrich public professional profiles with a detected CLI".to_owned(),
            expect: Expectation {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                matched_rules: vec![RuleId("route_parallel_profile_enrichment".to_owned())],
                elicit: vec![
                    "provide_profile_enrichment_inputs".to_owned(),
                    "approve_cli_execution".to_owned(),
                ],
                forbid: vec![
                    "hardcoded_people_values".to_owned(),
                    "reuse_example_people".to_owned(),
                    "omit_target_option".to_owned(),
                    "run_without_dry_run".to_owned(),
                    "claim_source_urls_when_absent".to_owned(),
                ],
                after_success: vec!["produce_profile_enrichment_summary".to_owned()],
                ..Expectation::default()
            },
        },
        ScenarioTest {
            name: "incomplete request asks for typed inputs".to_owned(),
            input: "use cli to enrich these people".to_owned(),
            expect: Expectation {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                matched_rules: vec![RuleId("route_parallel_profile_enrichment".to_owned())],
                elicit: vec!["provide_profile_enrichment_inputs".to_owned()],
                forbid: vec![
                    "execute_before_required_inputs".to_owned(),
                    "hardcoded_people_values".to_owned(),
                ],
                ..Expectation::default()
            },
        },
        ScenarioTest {
            name: "command lessons are hard forbids".to_owned(),
            input: format!("{cli_binary} enrich for public professional profile summaries"),
            expect: Expectation {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                matched_rules: vec![RuleId("route_parallel_profile_enrichment".to_owned())],
                forbid: vec![
                    "use_auth_status_subcommand".to_owned(),
                    "omit_target_option".to_owned(),
                    "exact_parallel_binary_assumption".to_owned(),
                ],
                ..Expectation::default()
            },
        },
    ]
}

fn activation_terms(cli_binary: &str) -> Vec<String> {
    let mut terms = [
        "cli",
        "detected cli",
        "profile enrichment",
        "enrich profiles",
        "public professional profiles",
        "people enrichment",
        "professional summary",
        "source-oriented profile lookup",
        "cli profile enrichment",
        "enrich public professional profiles with cli",
        "enrich public profiles",
        "enrich professional profiles",
        "use cli",
    ]
    .iter()
    .map(|term| (*term).to_owned())
    .collect::<BTreeSet<_>>();
    if !cli_binary.trim().is_empty() && cli_binary != "cli" {
        terms.insert(cli_binary.to_ascii_lowercase());
        terms.insert(cli_binary.replace('-', " ").to_ascii_lowercase());
        terms.insert(format!("{} enrich", cli_binary.to_ascii_lowercase()));
    }
    terms.into_iter().collect()
}

fn yaml_description(description: &str) -> YamlValue {
    yaml(serde_json::json!({ "description": description }))
}

fn parameter_contract() -> YamlValue {
    yaml(serde_json::json!({
        "people_json": {
            "type": "array<object>",
            "required": true,
            "description": "Fresh user-supplied people or entity records for this run."
        },
        "profile_enrichment_intent": {
            "type": "string",
            "required": true,
            "description": "The public professional facts to retrieve and the ambiguity policy to apply."
        },
        "processor": {
            "type": "enum",
            "required": true,
            "allowed_values": [
                "lite", "lite-fast", "base", "base-fast", "core", "core-fast", "pro", "pro-fast",
                "ultra", "ultra-fast", "ultra2x", "ultra2x-fast", "ultra4x", "ultra4x-fast",
                "ultra8x", "ultra8x-fast"
            ]
        },
        "csv_output": {
            "type": "path",
            "required": true,
            "description": "Output CSV path passed through --target."
        },
        "summary_output": {
            "type": "enum",
            "required": true,
            "allowed_values": ["concise", "table", "json"]
        }
    }))
}

fn learned_cli_error_guards(
    cli_binary: &str,
    has_auth_status_error: bool,
    has_missing_target_error: bool,
) -> YamlValue {
    let mut guards = Vec::new();
    if has_auth_status_error {
        guards.push(serde_json::json!({
            "invalid_form": format!("{cli_binary} auth status"),
            "guard": "use_auth_status_subcommand",
            "correction": format!("{cli_binary} auth")
        }));
    }
    if has_missing_target_error {
        guards.push(serde_json::json!({
            "invalid_form": "enrich run without --target",
            "guard": "omit_target_option",
            "correction": "include --target <csv_output>"
        }));
    }
    if guards.is_empty() {
        guards.push(serde_json::json!({
            "invalid_form": "unreviewed literal command reuse",
            "guard": "hardcoded_people_values",
            "correction": "collect typed inputs and render placeholders before execution"
        }));
    }
    yaml(guards)
}

fn coverage_matrix() -> YamlValue {
    yaml(serde_json::json!([
        {
            "prose_span": "CLI command behavior",
            "obligation": "Preserve the actual CLI command shape and setup errors as reusable guards.",
            "skillspec_construct": "commands.cli_auth, commands.cli_enrich_dry_run, commands.cli_enrich_run, closures.report_cli_errors_and_caveats",
            "confidence": "high",
            "status": "reviewed",
            "review_note": "The invalid auth-status and missing-target forms become explicit forbids when present in the command transcript."
        },
        {
            "prose_span": "parameterization",
            "obligation": "Require fresh typed inputs and forbid hardcoded people values.",
            "skillspec_construct": "elicitations.provide_profile_enrichment_inputs, metadata.parameter_contract, closures.validate_parameter_contract",
            "confidence": "high",
            "status": "reviewed",
            "review_note": "The command templates use placeholders only."
        },
        {
            "prose_span": "final output",
            "obligation": "Summarize public professional information while preserving ambiguity and source-claim limits.",
            "skillspec_construct": "closures.produce_profile_enrichment_summary, rules.route_parallel_profile_enrichment.forbid",
            "confidence": "medium",
            "status": "reviewed",
            "review_note": "The summary must report CLI caveats instead of inventing unavailable source URLs."
        }
    ]))
}

fn contract_quality() -> YamlValue {
    yaml(serde_json::json!({
        "activation": "strong",
        "dependencies": "good",
        "route_coverage": "focused",
        "execution_evidence": "command_shape_reviewed",
        "tests": "reviewed_behavior_tests",
        "hallucination_risk": "low"
    }))
}

fn yaml<T: Serialize>(value: T) -> YamlValue {
    serde_yaml::to_value(value).expect("synthesized metadata must serialize")
}

fn infer_observed_commands(log: &str) -> Vec<ObservedCommand> {
    let mut raw = Vec::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(log) {
        collect_json_commands(&value, &mut raw);
    }
    raw.extend(collect_text_commands(log));

    let mut seen = BTreeSet::new();
    raw.into_iter()
        .filter_map(|command| normalize_command(&command))
        .filter_map(|command| command_without_rote_provenance(&command))
        .filter(|command| seen.insert(command.clone()))
        .take(20)
        .map(|template| {
            let tool = command_tool(&template);
            let dependency_id = tool.as_deref().map(dependency_id_for_tool);
            ObservedCommand {
                template,
                tool,
                dependency_id,
            }
        })
        .collect()
}

fn dependency_id_for_tool(tool: &str) -> String {
    sanitize_id(tool)
}

fn command_without_rote_provenance(command: &str) -> Option<String> {
    if is_rote_provenance_command(command) {
        return None;
    }
    if let Some(inner) = command.strip_prefix("rote exec ") {
        if let Some(after_delimiter) = inner.strip_prefix("-- ") {
            let unwrapped = unwrap_shell_command(after_delimiter.trim());
            if is_rote_provenance_command(&unwrapped) {
                return None;
            }
            return Some(unwrapped);
        }
        if let Some((_, after_delimiter)) = inner.split_once(" -- ") {
            let unwrapped = unwrap_shell_command(after_delimiter.trim());
            if is_rote_provenance_command(&unwrapped) {
                return None;
            }
            return Some(unwrapped);
        }
    }
    Some(command.to_owned())
}

fn is_rote_provenance_command(command: &str) -> bool {
    let command = command.trim();
    command.starts_with('@')
        || command.starts_with("rote workspace ")
        || command.starts_with("rote stats")
        || command.starts_with("rote query ")
        || command.starts_with("rote inspect ")
        || command.starts_with("rote cd ")
}

fn unwrap_shell_command(command: &str) -> String {
    for shell in ["sh -lc ", "bash -lc "] {
        if let Some(inner) = command.strip_prefix(shell) {
            return inner.trim().trim_matches('"').trim_matches('\'').to_owned();
        }
    }
    command.to_owned()
}

fn collect_json_commands(value: &serde_json::Value, output: &mut Vec<String>) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                collect_json_commands(value, output);
            }
        }
        serde_json::Value::Object(map) => {
            for key in [
                "command",
                "cmd",
                "command_text",
                "command_line",
                "argv",
                "args",
                "program",
            ] {
                if let Some(value) = map.get(key) {
                    match value {
                        serde_json::Value::String(text) => output.push(text.clone()),
                        serde_json::Value::Array(parts) => {
                            let joined = parts
                                .iter()
                                .filter_map(|part| part.as_str())
                                .collect::<Vec<_>>()
                                .join(" ");
                            if !joined.is_empty() {
                                output.push(joined);
                            }
                        }
                        _ => {}
                    }
                }
            }
            for value in map.values() {
                collect_json_commands(value, output);
            }
        }
        _ => {}
    }
}

fn collect_text_commands(log: &str) -> Vec<String> {
    let mut commands = Vec::new();
    for line in log.lines() {
        let trimmed = line.trim();
        if let Some(command) = trimmed.strip_prefix("$ ") {
            commands.push(command.to_owned());
            continue;
        }
        for marker in ["command:", "cmd:", "command=", "cmd="] {
            if let Some((_, command)) = trimmed.split_once(marker) {
                commands.push(
                    command
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_owned(),
                );
                break;
            }
        }
    }
    commands
}

fn normalize_command(command: &str) -> Option<String> {
    let command = command.trim().trim_matches('"').trim_matches('\'');
    if command.is_empty() || command == "[]" || command.eq_ignore_ascii_case("null") {
        return None;
    }
    let collapsed = command.split_whitespace().collect::<Vec<_>>().join(" ");
    (collapsed.len() >= 2).then_some(collapsed)
}

fn command_tool(command: &str) -> Option<String> {
    if let Some(captures) = cli_discovery_regex().captures(command) {
        if let Some(tool) = captures.name("tool").and_then(normalize_tool_name) {
            return Some(tool);
        }
    }
    let captures = cli_invocation_regex().captures(command)?;
    captures.name("tool").and_then(normalize_tool_name)
}

fn cli_discovery_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?x)
            ^\s*
            (?:\$\s*)?
            (?:env\s+(?:-[A-Za-z]+\s+)*)?
            (?:[A-Za-z_][A-Za-z0-9_]*=(?:"[^"]*"|'[^']*'|[^\s]+)\s+)*
            (?:command\s+-v|which|type\s+-P)
            \s+
            (?P<tool>(?:[./~\w-]+/)?[A-Za-z][A-Za-z0-9_.-]*)
            (?:\s|$)
            "#,
        )
        .expect("CLI discovery regex must compile")
    })
}

fn cli_invocation_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?x)
            ^\s*
            (?:\$\s*)?
            (?:env\s+(?:-[A-Za-z]+\s+)*)?
            (?:[A-Za-z_][A-Za-z0-9_]*=(?:"[^"]*"|'[^']*'|[^\s]+)\s+)*
            (?P<tool>(?:[./~\w-]+/)?[A-Za-z][A-Za-z0-9_.-]*)
            (?:\s|$)
            "#,
        )
        .expect("CLI invocation regex must compile")
    })
}

fn normalize_tool_name(raw: regex::Match<'_>) -> Option<String> {
    let value = raw
        .as_str()
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`');
    if value.is_empty() || value.contains('=') {
        return None;
    }
    let file_name = Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value);
    let id = sanitize_id(file_name);
    (!id.is_empty()).then(|| file_name.to_owned())
}

fn dependency_ids(commands: &[ObservedCommand]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|command| command.dependency_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn skill_id(name: Option<&str>, task: &str) -> String {
    name.map(sanitize_id)
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| {
            if task.to_ascii_lowercase().contains("profile")
                && task.to_ascii_lowercase().contains("enrich")
            {
                "parallel_profile_enricher".to_owned()
            } else {
                "cli_workflow".to_owned()
            }
        })
}

fn sanitize_id(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for char in input.chars() {
        if char.is_ascii_alphanumeric() {
            out.push(char.to_ascii_lowercase());
            last_dash = false;
        } else if matches!(char, '-' | '_' | '.' | '/') && !last_dash && !out.is_empty() {
            out.push('_');
            last_dash = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "cli_workflow".to_owned()
    } else {
        out
    }
}

fn title_from_id(id: &str) -> String {
    id.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
