use crate::error::{Error, Result};
use crate::model::{
    Activation, Artifact, ArtifactKind, CommandRequires, CommandTemplate, Dependency,
    DependencyCheck, DependencyKind, DependencyPermission, Elicitation, ElicitationChoice,
    ElicitationCondition, ExecutableRefKind, ExecutionPhase, ExecutionPlan, ExecutionPlanMode,
    Expectation, ProducerRef, Proof, Recipe, RecipeRequires, RecipeStep, RecipeStepLoadResource,
    RecipeStepNote, Resource, ResourceRole, ResourceUse, ResourceUseKind, Route, RouteId, Rule,
    RuleId, SafetyClass, ScenarioTest, SkillSpec, Snippet, ToolBoundary, ToolBoundaryDefault,
    TraceConfig, TraceEventKind, TraceMode,
};
use serde::Serialize;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

const REPORT_PATH: &str = "resources/observed-workspace/report.md";
const STATS_PATH: &str = "resources/observed-workspace/stats.txt";
const LOG_PATH: &str = "resources/observed-workspace/log.txt";
const META_PATH: &str = "resources/observed-workspace/meta.txt";
const DEPS_PATH: &str = "resources/observed-workspace/deps.txt";
const COVERAGE_PATH: &str = "resources/observed-workspace/coverage-matrix.md";

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
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct SynthesisReport {
    pub workspace: String,
    pub out_dir: PathBuf,
    pub spec_path: PathBuf,
    pub report_path: PathBuf,
    pub stats_path: PathBuf,
    pub log_path: PathBuf,
    pub meta_path: PathBuf,
    pub deps_path: Option<PathBuf>,
    pub coverage_path: PathBuf,
    pub inferred_dependencies: Vec<String>,
    pub observed_command_candidates: usize,
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
    id: String,
    template: String,
    dependency_id: Option<String>,
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
    let skill_id = skill_id(options.name.as_deref(), task, &workspace);
    let title = title_from_id(&skill_id);
    let commands = infer_observed_commands(&evidence.log);
    let spec = build_spec(&skill_id, &title, &workspace, task, &evidence, &commands);
    crate::parser::validate_spec(&spec)?;

    fs::create_dir_all(options.out.join("resources/observed-workspace")).map_err(|source| {
        Error::Write {
            path: options.out.join("resources/observed-workspace"),
            source,
        }
    })?;
    write_file(&options.out.join(STATS_PATH), &evidence.stats)?;
    write_file(&options.out.join(LOG_PATH), &evidence.log)?;
    write_file(&options.out.join(META_PATH), &evidence.meta)?;
    if let Some(deps) = &evidence.deps {
        write_file(&options.out.join(DEPS_PATH), deps)?;
    }
    let coverage = render_coverage_matrix(&workspace, task, &commands, &evidence);
    write_file(&options.out.join(COVERAGE_PATH), &coverage)?;
    let report = render_workspace_report(&workspace, task, &commands, &evidence);
    write_file(&options.out.join(REPORT_PATH), &report)?;
    crate::parser::write_spec(&spec_path, &spec)?;

    Ok(SynthesisReport {
        workspace,
        out_dir: options.out.clone(),
        spec_path,
        report_path: options.out.join(REPORT_PATH),
        stats_path: options.out.join(STATS_PATH),
        log_path: options.out.join(LOG_PATH),
        meta_path: options.out.join(META_PATH),
        deps_path: evidence.deps.as_ref().map(|_| options.out.join(DEPS_PATH)),
        coverage_path: options.out.join(COVERAGE_PATH),
        inferred_dependencies: dependency_ids(&commands),
        observed_command_candidates: commands.len(),
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
        "SkillSpec workspace synthesis\n\nWorkspace: {}\nSpec: {}\nReport: {}\nObserved command candidates: {}\nInferred dependencies: {}\n\nNext: review {}, then run `skillspec validate {}` and `skillspec test {}`.\n",
        report.workspace,
        report.spec_path.display(),
        report.report_path.display(),
        report.observed_command_candidates,
        deps,
        report.spec_path.display(),
        report.spec_path.display(),
        report.spec_path.display()
    )
}

fn collect_evidence(options: &SynthesizeOptions, workspace: &str) -> Result<WorkspaceEvidence> {
    let uses_live_required_evidence = options.workspace_stats_report.is_none()
        || options.workspace_log.is_none()
        || options.workspace_meta.is_none();
    let stats = read_or_collect(
        options.workspace_stats_report.as_deref(),
        "workspace stats",
        &["workspace", "stats", workspace],
    )?;
    let log_last = options.log_last.to_string();
    let log = read_or_collect(
        options.workspace_log.as_deref(),
        "workspace command log",
        &["workspace", "inspect", "log", "--last", &log_last],
    )?;
    let meta = read_or_collect(
        options.workspace_meta.as_deref(),
        "workspace metadata",
        &["workspace", "inspect", "meta"],
    )?;
    let deps = match options.workspace_deps.as_deref() {
        Some(path) => Some(read_file(path, "workspace dependency graph")?),
        None if uses_live_required_evidence => collect_optional(&["workspace", "inspect", "deps"])
            .ok()
            .filter(|content| !content.trim().is_empty()),
        None => None,
    };

    Ok(WorkspaceEvidence {
        stats,
        log,
        meta,
        deps,
    })
}

fn read_or_collect(path: Option<&Path>, label: &str, rote_args: &[&str]) -> Result<String> {
    match path {
        Some(path) => read_file(path, label),
        None => collect_required(rote_args, label),
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

fn collect_required(args: &[&str], label: &str) -> Result<String> {
    let output = ProcessCommand::new("rote")
        .args(args)
        .output()
        .map_err(|source| Error::InvalidInput {
            message: format!(
                "failed to run `rote {}` for {label}: {source}",
                args.join(" ")
            ),
        })?;
    if !output.status.success() {
        return Err(Error::InvalidInput {
            message: format!(
                "`rote {}` failed while collecting {label}: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if stdout.trim().is_empty() {
        return Err(Error::InvalidInput {
            message: format!("`rote {}` produced empty {label} evidence", args.join(" ")),
        });
    }
    Ok(stdout)
}

fn collect_optional(args: &[&str]) -> Result<String> {
    let output = ProcessCommand::new("rote")
        .args(args)
        .output()
        .map_err(|source| Error::InvalidInput {
            message: format!("failed to run `rote {}`: {source}", args.join(" ")),
        })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(Error::InvalidInput {
            message: format!("`rote {}` failed", args.join(" ")),
        })
    }
}

fn validate_evidence(workspace: &str, evidence: &WorkspaceEvidence) -> Result<()> {
    if !mentions_workspace(&evidence.stats, workspace) {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace stats evidence does not mention workspace {workspace:?}; pass stats from `rote workspace stats {workspace}`"
            ),
        });
    }
    if !has_log_entries(&evidence.log) {
        return Err(Error::InvalidInput {
            message: "workspace command log evidence has no command entries; durable execution must create a workspace with command log evidence before synthesis".to_owned(),
        });
    }
    if !has_metadata(&evidence.meta) {
        return Err(Error::InvalidInput {
            message: "workspace metadata evidence is empty or placeholder-only".to_owned(),
        });
    }
    Ok(())
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

fn build_spec(
    skill_id: &str,
    title: &str,
    workspace: &str,
    task: &str,
    evidence: &WorkspaceEvidence,
    observed_commands: &[ObservedCommand],
) -> SkillSpec {
    let mut dependencies = BTreeMap::new();
    dependencies.insert(
        "rote_cli".to_owned(),
        Dependency {
            kind: DependencyKind::Cli,
            description: Some("Required to inspect durable workspace stats, logs, metadata, and dependency evidence.".to_owned()),
            command: Some("rote".to_owned()),
            path: None,
            env: None,
            check: Some(DependencyCheck {
                command: Some("rote".to_owned()),
                path: None,
                env: None,
            }),
            permission: Some(DependencyPermission {
                required: true,
                reason: Some("Workspace inspection may reveal local workflow evidence and sensitive task context.".to_owned()),
                safety: Some(SafetyClass::LocalRead),
            }),
            provision: None,
        },
    );
    for command in observed_commands {
        if let Some(id) = &command.dependency_id {
            dependencies.entry(id.clone()).or_insert_with(|| Dependency {
                kind: DependencyKind::Cli,
                description: Some(format!(
                    "Inferred from durable workspace command candidate `{}`; review before replay.",
                    command.template
                )),
                command: Some(id.trim_end_matches("_cli").to_owned()),
                path: None,
                env: None,
                check: Some(DependencyCheck {
                    command: Some(id.trim_end_matches("_cli").to_owned()),
                    path: None,
                    env: None,
                }),
                permission: Some(DependencyPermission {
                    required: true,
                    reason: Some("Observed command candidates are evidence, not pre-approved replay permission.".to_owned()),
                    safety: Some(SafetyClass::NetworkRead),
                }),
                provision: None,
            });
        }
    }

    let mut commands = BTreeMap::new();
    commands.insert(
        "inspect_workspace_stats".to_owned(),
        CommandTemplate {
            description: Some("Collect durable workspace stats before proving or revising this synthesized skill.".to_owned()),
            template: format!("rote workspace stats {workspace}"),
            safety: Some(SafetyClass::LocalRead),
            requires: CommandRequires {
                dependencies: vec!["rote_cli".to_owned()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    commands.insert(
        "inspect_workspace_log".to_owned(),
        CommandTemplate {
            description: Some(
                "Inspect the durable command log that this skill was synthesized from.".to_owned(),
            ),
            template: "rote workspace inspect log --last <count>".to_owned(),
            safety: Some(SafetyClass::LocalRead),
            requires: CommandRequires {
                dependencies: vec!["rote_cli".to_owned()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    commands.insert(
        "inspect_workspace_meta".to_owned(),
        CommandTemplate {
            description: Some(
                "Inspect durable workspace metadata for context and replay boundaries.".to_owned(),
            ),
            template: "rote workspace inspect meta".to_owned(),
            safety: Some(SafetyClass::LocalRead),
            requires: CommandRequires {
                dependencies: vec!["rote_cli".to_owned()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    for command in observed_commands {
        commands.insert(
            command.id.clone(),
            CommandTemplate {
                description: Some("Observed command candidate from durable workspace log; review and parameterize before replay.".to_owned()),
                template: command.template.clone(),
                safety: Some(SafetyClass::LocalWrite),
                requires: CommandRequires {
                    dependencies: command.dependency_id.iter().cloned().collect(),
                    resources: vec!["observed_workspace_report".to_owned()],
                    ..CommandRequires::default()
                },
                parse: BTreeMap::new(),
                success_when: BTreeMap::new(),
            },
        );
    }

    let mut resources = BTreeMap::new();
    resources.insert(
        "observed_workspace_report".to_owned(),
        Resource {
            path: REPORT_PATH.to_owned(),
            role: ResourceRole::SourceMaterial,
            description: Some("Human-readable report produced from durable workspace stats, log, metadata, dependency evidence, and synthesis notes.".to_owned()),
            used_by: vec![
                ResourceUse {
                    kind: ResourceUseKind::Route,
                    id: "observed_workflow".to_owned(),
                },
                ResourceUse {
                    kind: ResourceUseKind::Recipe,
                    id: "review_observed_workspace".to_owned(),
                },
            ],
            load_when: vec!["Load before promoting observed command candidates into approved workflow steps.".to_owned()],
        },
    );
    for (id, path, description) in [
        (
            "observed_workspace_stats",
            STATS_PATH,
            "Raw durable workspace stats evidence.",
        ),
        (
            "observed_workspace_log",
            LOG_PATH,
            "Raw durable workspace command log evidence.",
        ),
        (
            "observed_workspace_meta",
            META_PATH,
            "Raw durable workspace metadata evidence.",
        ),
        (
            "coverage_matrix",
            COVERAGE_PATH,
            "Coverage matrix for observed facts, inferred behavior, and review status.",
        ),
    ] {
        resources.insert(
            id.to_owned(),
            Resource {
                path: path.to_owned(),
                role: ResourceRole::SourceMaterial,
                description: Some(description.to_owned()),
                used_by: vec![ResourceUse {
                    kind: ResourceUseKind::Recipe,
                    id: "review_observed_workspace".to_owned(),
                }],
                load_when: Vec::new(),
            },
        );
    }
    if evidence.deps.is_some() {
        resources.insert(
            "observed_workspace_deps".to_owned(),
            Resource {
                path: DEPS_PATH.to_owned(),
                role: ResourceRole::Reference,
                description: Some(
                    "Raw durable workspace dependency graph evidence when available.".to_owned(),
                ),
                used_by: vec![ResourceUse {
                    kind: ResourceUseKind::Recipe,
                    id: "review_observed_workspace".to_owned(),
                }],
                load_when: Vec::new(),
            },
        );
    }

    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        "observed_workspace_report".to_owned(),
        Artifact {
            kind: ArtifactKind::Report,
            description: Some(
                "Synthesis report generated from durable workspace evidence.".to_owned(),
            ),
            path: Some(REPORT_PATH.to_owned()),
            schema: None,
            produced_by: Vec::new(),
            consumed_by: Vec::new(),
        },
    );
    artifacts.insert(
        "replay_result".to_owned(),
        Artifact {
            kind: ArtifactKind::Report,
            description: Some(
                "Future proof report from replaying or validating the reviewed observed workflow."
                    .to_owned(),
            ),
            path: None,
            schema: None,
            produced_by: observed_commands
                .iter()
                .map(|command| ProducerRef {
                    kind: ExecutableRefKind::Command,
                    id: command.id.clone(),
                })
                .collect(),
            consumed_by: Vec::new(),
        },
    );

    let mut elicitations = BTreeMap::new();
    elicitations.insert(
        "approve_observed_dependency_surface".to_owned(),
        Elicitation {
            question: "Do you approve promoting the observed dependency and command surface from this durable workspace?".to_owned(),
            required_when: vec![ElicitationCondition {
                route: Some(RouteId("observed_workflow".to_owned())),
                missing: None,
                predicate: None,
            }],
            choices: vec![
                ElicitationChoice {
                    id: "approve_reviewed_surface".to_owned(),
                    label: "Approve reviewed surface".to_owned(),
                    description: Some("Continue only after command candidates, dependencies, resources, and privacy boundaries have been reviewed.".to_owned()),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::LocalWrite),
                },
                ElicitationChoice {
                    id: "keep_draft_only".to_owned(),
                    label: "Keep draft only".to_owned(),
                    description: Some("Do not replay commands or install the generated skill yet.".to_owned()),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
            ],
            default: Some("keep_draft_only".to_owned()),
            max_choices: None,
        },
    );

    let mut recipes = BTreeMap::new();
    recipes.insert(
        "review_observed_workspace".to_owned(),
        Recipe {
            description: Some("Review durable workspace evidence before promoting observed commands into a reusable skill workflow.".to_owned()),
            ordered: true,
            requires: RecipeRequires {
                resources: vec![
                    "observed_workspace_report".to_owned(),
                    "observed_workspace_stats".to_owned(),
                    "observed_workspace_log".to_owned(),
                    "observed_workspace_meta".to_owned(),
                    "coverage_matrix".to_owned(),
                ],
                dependencies: vec!["rote_cli".to_owned()],
                ..RecipeRequires::default()
            },
            steps: vec![
                RecipeStep::LoadResource(RecipeStepLoadResource {
                    load_resource: "observed_workspace_report".to_owned(),
                }),
                RecipeStep::LoadResource(RecipeStepLoadResource {
                    load_resource: "coverage_matrix".to_owned(),
                }),
                RecipeStep::Note(RecipeStepNote {
                    note: "Promote only stable observed behavior. Keep inferred or unsafe behavior in review_required until validated.".to_owned(),
                }),
            ],
        },
    );

    let route = Route {
        id: RouteId("observed_workflow".to_owned()),
        label: "Run reviewed observed workflow".to_owned(),
        rank: Some(10),
        description: Some(format!(
            "Use after reviewing durable workspace {workspace:?} evidence for task: {task}"
        )),
        checks: Vec::new(),
        handoff: None,
        execution_plan: Some(ExecutionPlan {
            mode: ExecutionPlanMode::Ordered,
            reason: Some("Observed workflows require evidence review and dependency approval before replay.".to_owned()),
            phases: vec![
                ExecutionPhase {
                    id: "review_workspace_evidence".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Review workspace report, command log, stats, metadata, deps, coverage matrix, and proof gaps.".to_owned()),
                    requires: vec!["review_observed_workspace".to_owned()],
                    checks: Vec::new(),
                    forbid: vec!["skip_workspace_evidence_review".to_owned()],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: Some(ToolBoundary {
                        default: Some(ToolBoundaryDefault::Deny),
                        allow: vec![
                            "skillspec_cli".to_owned(),
                            "local_skill_files".to_owned(),
                            "local_workspace_evidence".to_owned(),
                        ],
                        forbid: vec!["replay_mutating_actions_without_review".to_owned()],
                        permission_required_for: vec!["any_execution_substrate".to_owned()],
                    }),
                },
                ExecutionPhase {
                    id: "approve_dependency_surface".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Approve or keep draft-only the inferred observed command and dependency surface.".to_owned()),
                    requires: vec!["approve_observed_dependency_surface".to_owned()],
                    checks: Vec::new(),
                    forbid: vec!["assume_observed_commands_are_approved".to_owned()],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "prove_reviewed_workflow".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Run only reviewed and parameterized command templates, then prove result and token/workspace evidence.".to_owned()),
                    requires: vec!["prove_observed_workflow".to_owned()],
                    checks: Vec::new(),
                    forbid: vec!["claim_workspace_tokens_without_stats".to_owned()],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
            ],
        }),
        tool_boundary: None,
    };

    let rule = Rule {
        id: RuleId("route_observed_task_to_reviewed_workflow".to_owned()),
        when: crate::model::Predicate {
            user_says_any: activation_terms(task, workspace),
            ..crate::model::Predicate::default()
        },
        prefer: Some(RouteId("observed_workflow".to_owned())),
        route_order: Vec::new(),
        forbid: vec![
            "replay_mutating_actions_without_review".to_owned(),
            "use_unobserved_substrate_without_approval".to_owned(),
            "claim_workspace_tokens_without_stats".to_owned(),
        ],
        allow: BTreeMap::new(),
        elicit: vec!["approve_observed_dependency_surface".to_owned()],
        after_success: vec!["prove_observed_workflow".to_owned()],
        reason: Some("The generated skill is grounded in durable workspace evidence and must stay draft-only until reviewed.".to_owned()),
    };

    let mut snippets = BTreeMap::new();
    snippets.insert(
        "observed_workspace_summary".to_owned(),
        Snippet {
            text: format!(
                "Synthesized from durable workspace `{workspace}` for task `{task}`. Stats/log/meta evidence is preserved under resources/observed-workspace/."
            ),
        },
    );

    let mut closures = BTreeMap::new();
    closures.insert(
        "prove_observed_workflow".to_owned(),
        YamlValue::String("Validate the reviewed workflow with structural checks, workspace/token stats, and explicit proof of final user value.".to_owned()),
    );

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "synthesized_from_workspace".to_owned(),
        YamlValue::String(workspace.to_owned()),
    );
    metadata.insert(
        "observed_task".to_owned(),
        YamlValue::String(task.to_owned()),
    );
    metadata.insert(
        "observed_command_candidate_count".to_owned(),
        YamlValue::Number(observed_commands.len().into()),
    );

    SkillSpec {
        schema: "skillspec/v0".to_owned(),
        id: skill_id.to_owned(),
        title: title.to_owned(),
        description: format!(
            "Draft SkillSpec scaffold synthesized from durable workspace `{workspace}`. Review before replay, install, or release."
        ),
        activation: Some(Activation {
            summary: format!("Use for the reviewed workflow observed in durable workspace {workspace}."),
            keywords: activation_terms(task, workspace),
            priority: Some("reviewed_observed_workflow".to_owned()),
        }),
        applies_when: Vec::new(),
        entry: None,
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
        resources,
        code: BTreeMap::new(),
        artifacts,
        recipes,
        commands,
        snippets,
        closures,
        proof: Some(Proof {
            metrics: vec![
                "workspace stats collected before synthesis".to_owned(),
                "workspace command log collected before synthesis".to_owned(),
                "workspace metadata collected before synthesis".to_owned(),
                "observed command candidates require review before replay".to_owned(),
            ],
        }),
        tests: vec![ScenarioTest {
            name: "observed task routes to reviewed workflow".to_owned(),
            input: task.to_owned(),
            expect: Expectation {
                route: Some(RouteId("observed_workflow".to_owned())),
                forbid: vec![
                    "replay_mutating_actions_without_review".to_owned(),
                    "use_unobserved_substrate_without_approval".to_owned(),
                    "claim_workspace_tokens_without_stats".to_owned(),
                ],
                elicit: vec!["approve_observed_dependency_surface".to_owned()],
                after_success: vec!["prove_observed_workflow".to_owned()],
                ..Expectation::default()
            },
        }],
        review_required: review_notes(observed_commands, evidence),
        metadata,
    }
}

fn review_notes(
    observed_commands: &[ObservedCommand],
    evidence: &WorkspaceEvidence,
) -> Vec<String> {
    let mut notes = vec![
        "Review the workspace report and coverage matrix before treating this scaffold as an installable skill.".to_owned(),
        "Promote observed command candidates only after parameterizing inputs, outputs, safety, and approvals.".to_owned(),
        "Review inferred dependencies and permission classes; the synthesizer marks them conservatively.".to_owned(),
        "Add domain-specific scenario tests before install or release.".to_owned(),
    ];
    if observed_commands.is_empty() {
        notes.push("No repeatable command candidates were inferred from the command log; add commands manually from reviewed evidence.".to_owned());
    }
    if evidence.deps.is_none() {
        notes.push("No workspace dependency graph evidence was available; inspect `rote workspace inspect deps` when possible.".to_owned());
    }
    notes
}

fn render_workspace_report(
    workspace: &str,
    task: &str,
    commands: &[ObservedCommand],
    evidence: &WorkspaceEvidence,
) -> String {
    let mut report = String::new();
    report.push_str("# Observed Durable Workspace Report\n\n");
    report.push_str(&format!("- workspace: `{workspace}`\n"));
    report.push_str(&format!("- observed task: `{task}`\n"));
    report.push_str("- source: durable execution workspace evidence collected by `skillspec synthesize-from-workspace`\n\n");
    report.push_str("## Validation\n\n");
    report.push_str("- workspace stats: present and names the requested workspace\n");
    report.push_str("- command log: present with at least one entry\n");
    report.push_str("- metadata: present\n");
    report.push_str(&format!(
        "- dependency graph: {}\n\n",
        if evidence.deps.is_some() {
            "present"
        } else {
            "not available"
        }
    ));
    report.push_str("## Observed Command Candidates\n\n");
    if commands.is_empty() {
        report.push_str("No stable command templates were inferred. Review the raw command log before adding commands.\n\n");
    } else {
        for command in commands {
            report.push_str(&format!(
                "- `{}`: `{}`{}\n",
                command.id,
                command.template,
                command
                    .dependency_id
                    .as_ref()
                    .map(|id| format!(" (dependency `{id}`)"))
                    .unwrap_or_default()
            ));
        }
        report.push('\n');
    }
    report.push_str("## Raw Evidence Files\n\n");
    report.push_str("- stats: `stats.txt`\n");
    report.push_str("- command log: `log.txt`\n");
    report.push_str("- metadata: `meta.txt`\n");
    if evidence.deps.is_some() {
        report.push_str("- dependency graph: `deps.txt`\n");
    }
    report.push_str("\n## Synthesis Policy\n\n");
    report.push_str("Observed evidence is not automatic replay permission. Keep inferred behavior in review until dependencies, resources, commands, safety, tests, and proof are explicitly reviewed.\n");
    report
}

fn render_coverage_matrix(
    workspace: &str,
    task: &str,
    commands: &[ObservedCommand],
    evidence: &WorkspaceEvidence,
) -> String {
    let deps_status = if evidence.deps.is_some() {
        "present"
    } else {
        "review_required"
    };
    let commands_status = if commands.is_empty() {
        "review_required"
    } else {
        "draft"
    };
    format!(
        "prose_span | obligation | skillspec_construct | confidence | status | review_note\n--- | --- | --- | --- | --- | ---\nworkspace `{workspace}` | Preserve durable workspace provenance | metadata.synthesized_from_workspace, resources.observed_workspace_* | high | present | Stats, log, and meta are required before synthesis.\nobserved task `{task}` | Route matching tasks to reviewed workflow | activation, rules.route_observed_task_to_reviewed_workflow, tests | medium | draft | Review trigger precision before install.\ncommand log | Promote stable commands only after review | commands.observed_command_* | medium | {commands_status} | Raw command log may include one-off or unsafe actions.\ndependency graph | Preserve dependency evidence when available | resources.observed_workspace_deps | medium | {deps_status} | Missing deps evidence should be collected with `rote workspace inspect deps` when possible.\n"
    )
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
        .filter(|command| !command.starts_with("rote workspace "))
        .filter(|command| seen.insert(command.clone()))
        .take(20)
        .enumerate()
        .map(|(index, template)| {
            let dependency_id =
                command_tool(&template).map(|tool| format!("{}_cli", sanitize_id(&tool)));
            ObservedCommand {
                id: format!("observed_command_{}", index + 1),
                template,
                dependency_id,
            }
        })
        .collect()
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
    let mut parts = command.split_whitespace();
    let mut first = parts.next()?.trim_start_matches('$');
    while first.contains('=') && !first.contains('/') {
        first = parts.next()?;
    }
    let file_name = Path::new(first)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(first);
    let sanitized = sanitize_id(file_name);
    (!sanitized.is_empty()).then_some(sanitized)
}

fn dependency_ids(commands: &[ObservedCommand]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|command| command.dependency_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn skill_id(name: Option<&str>, task: &str, workspace: &str) -> String {
    name.map(sanitize_id)
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| {
            let source = if task.trim().is_empty() {
                workspace
            } else {
                task
            };
            let words = source
                .split_whitespace()
                .filter(|word| word.chars().any(|char| char.is_ascii_alphanumeric()))
                .take(5)
                .collect::<Vec<_>>()
                .join("-");
            sanitize_id(&words)
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
        "observed_workflow".to_owned()
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

fn activation_terms(task: &str, workspace: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    terms.insert(workspace.to_owned());
    terms.insert(task.to_owned());
    for word in task.split_whitespace() {
        let word = word
            .trim_matches(|char: char| !char.is_ascii_alphanumeric())
            .to_ascii_lowercase();
        if word.len() >= 4 {
            terms.insert(word);
        }
    }
    terms.into_iter().collect()
}
