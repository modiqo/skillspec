use crate::error::{Error, Result};
use crate::model::{
    Artifact, ArtifactKind, CommandTemplate, ConsumerRef, Dependency, DependencyCheck,
    DependencyKind, DependencyPermission, DependencyProvision, DependencyProvisionOption,
    Elicitation, ElicitationChoice, ElicitationCondition, ExecutableRefKind, ExecutionPhase,
    ExecutionPlan, ExecutionPlanMode, Expectation, Predicate, ProducerRef, Route, RouteId, Rule,
    RuleId, SafetyClass, ScenarioTest, SkillSpec, State, ToolBoundary, ToolBoundaryDefault,
    TraceConfig, TraceEventKind, TraceMode,
};
use crate::{
    compiler, decision, deps, doctor, grammar, importer, imports, metrics, parser, progress,
    source_map, workspace,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct PortOneShotOptions {
    pub source: PathBuf,
    pub out: PathBuf,
    pub target: compiler::Target,
    pub prove: bool,
    pub force: bool,
    pub run_dir: Option<PathBuf>,
    pub phase: Option<String>,
    pub requirements: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortOneShotReport {
    pub ok: bool,
    pub status: String,
    pub source: String,
    pub out: String,
    pub spec_path: String,
    pub target: String,
    pub prove: bool,
    pub semantic_status: String,
    pub source_map_path: String,
    pub source_map_markdown_path: String,
    pub doctor_report_path: String,
    pub grammar_porting_path: String,
    pub grammar_checklist_path: String,
    pub schema_path: String,
    pub shape_crib_path: String,
    pub compiled_path: Option<String>,
    pub report_path: String,
    pub source_summary: PortSourceSummary,
    pub qa: Vec<PortGateReport>,
    pub stats: Option<PortStatsReport>,
    pub next: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortSourceSummary {
    pub files: usize,
    pub nodes: usize,
    pub code_blocks: usize,
    pub dependency_mentions: usize,
    pub review_required: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortGateReport {
    pub id: String,
    pub status: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortStatsReport {
    pub run_dir: String,
    pub phase: Option<String>,
    pub requirements: Vec<String>,
    pub agent_visible_tokens: u64,
    pub artifact_tokens_preserved: u64,
    pub avoided_tokens: u64,
    pub metrics_source: String,
}

pub fn run(options: PortOneShotOptions) -> Result<PortOneShotReport> {
    if options.run_dir.is_none() && (options.phase.is_some() || !options.requirements.is_empty()) {
        return Err(Error::InvalidInput {
            message: "--phase and --requirement require --run-dir for stats recording".to_owned(),
        });
    }
    if !options.requirements.is_empty() && options.phase.is_none() {
        return Err(Error::InvalidInput {
            message: "--requirement requires --phase".to_owned(),
        });
    }
    workspace::guard_single_skill_source(&options.source, "skillspec port-one-shot")?;
    let spec_path = options.out.join("skill.spec.yml");
    if spec_path.exists() && !options.force {
        return Err(Error::InvalidInput {
            message: format!(
                "{} already exists; pass --force to overwrite this draft",
                spec_path.display()
            ),
        });
    }

    fs::create_dir_all(&options.out).map_err(|source| Error::Write {
        path: options.out.clone(),
        source,
    })?;
    let port_dir = options.out.join(".skillspec/port");
    fs::create_dir_all(&port_dir).map_err(|source| Error::Write {
        path: port_dir.clone(),
        source,
    })?;

    let grammar_porting_path = port_dir.join("grammar-porting.md");
    let grammar_checklist_path = port_dir.join("grammar-checklist.md");
    let schema_path = port_dir.join("schema.json");
    let shape_crib_path = port_dir.join("shape-crib.yml");
    let doctor_report_path = port_dir.join("doctor.json");
    let compiled_path = port_dir.join(format!("compiled.{}.md", target_name(options.target)));
    let report_path = port_dir.join("port-one-shot.report.md");

    write_text(
        &grammar_porting_path,
        &grammar::render_sensemake(&grammar::sensemake(grammar::GrammarView::Porting)),
    )?;
    write_text(
        &grammar_checklist_path,
        &grammar::render_checklist(&grammar::checklist(grammar::ChecklistSubject::ImportSkill)),
    )?;
    write_json(&schema_path, &grammar::schema_json()?)?;
    write_text(&shape_crib_path, &shape_crib_yaml(&shape_crib_path)?)?;

    let source_map_dir = options.out.join(".skillspec/source-map");
    let source_map_report = source_map::create_source_map(&options.source, &source_map_dir)?;
    let source_map_path = PathBuf::from(&source_map_report.source_map);
    let source_root = source_map::source_root_for(&options.source);
    let stale = source_map::stale(&source_map_path, Some(&source_root))?;
    if !stale.ok {
        return Err(Error::InvalidInput {
            message: format!(
                "freshly generated source map {} is stale for {}",
                source_map_path.display(),
                source_root.display()
            ),
        });
    }
    let source_map = source_map::load(&source_map_path)?;

    let doctor_report = doctor::inspect(&options.source)?;
    write_json(&doctor_report_path, &doctor_report)?;

    let imported = importer::import_skill_for_output(&options.source, &spec_path)?;
    parser::write_spec(&spec_path, &imported)?;

    let mut qa = Vec::new();
    let mut fatal_failure = false;
    let mut compiled = None;
    let mut semantic_review_required = !imported.review_required.is_empty();

    let loaded = match parser::load_spec(&spec_path) {
        Ok(spec) => {
            qa.push(gate(
                "validate",
                "ok",
                "skill.spec.yml parsed and validated against the typed grammar",
                Some(&spec_path),
            ));
            Some(spec)
        }
        Err(error) => {
            fatal_failure = true;
            qa.push(gate(
                "validate",
                "failed",
                format!("validation failed: {error}"),
                Some(&spec_path),
            ));
            None
        }
    };

    if let Some(spec) = loaded.as_ref() {
        let import_report = imports::check(spec, &spec_path);
        if import_report.ok {
            qa.push(gate(
                "imports",
                "ok",
                format!("{} imports resolved", import_report.imports.len()),
                Some(&spec_path),
            ));
        } else {
            fatal_failure = true;
            qa.push(gate(
                "imports",
                "failed",
                "one or more imports are missing, invalid, or missing requested sections",
                Some(&spec_path),
            ));
        }

        let spec_dir = spec_path.parent().unwrap_or_else(|| Path::new("."));
        match deps::check(spec, spec_dir, None) {
            Ok(dep_report) if dep_report.ok => qa.push(gate(
                "deps",
                "ok",
                format!("{} dependencies checked", dep_report.dependencies.len()),
                Some(&spec_path),
            )),
            Ok(dep_report) => {
                semantic_review_required = true;
                qa.push(gate(
                    "deps",
                    "review_required",
                    format!(
                        "{} dependencies checked; missing/deferred dependency gaps preserved for review",
                        dep_report.dependencies.len()
                    ),
                    Some(&spec_path),
                ));
            }
            Err(error) => {
                fatal_failure = true;
                qa.push(gate(
                    "deps",
                    "failed",
                    format!("dependency check failed: {error}"),
                    Some(&spec_path),
                ));
            }
        }

        let test_run = decision::run_tests(spec);
        let test_count = test_run.passed.len() + test_run.failed.len();
        if !test_run.failed.is_empty() {
            fatal_failure = true;
            qa.push(gate(
                "test",
                "failed",
                format!(
                    "{}/{} scenario tests passed",
                    test_run.passed.len(),
                    test_count
                ),
                Some(&spec_path),
            ));
        } else if test_count == 0 {
            semantic_review_required = true;
            qa.push(gate(
                "test",
                "review_required",
                "no scenario tests declared; scaffold is valid but behavior is not proven",
                Some(&spec_path),
            ));
        } else {
            qa.push(gate(
                "test",
                "ok",
                format!(
                    "{}/{} scenario tests passed",
                    test_run.passed.len(),
                    test_count
                ),
                Some(&spec_path),
            ));
        }

        let compiled_markdown = compiler::compile(spec, options.target);
        write_text(&compiled_path, &compiled_markdown)?;
        compiled = Some(path_to_string(&compiled_path));
        qa.push(gate(
            "compile",
            "ok",
            format!("compiled {}", target_name(options.target)),
            Some(&compiled_path),
        ));
    } else {
        for id in ["imports", "deps", "test", "compile"] {
            qa.push(gate(
                id,
                "skipped",
                "skipped because validation failed",
                Some(&spec_path),
            ));
        }
    }

    let ok = !fatal_failure;
    let status = if fatal_failure {
        "failed"
    } else if semantic_review_required {
        "passed_with_review_required"
    } else {
        "passed"
    };

    Ok(PortOneShotReport {
        ok,
        status: status.to_owned(),
        source: path_to_string(&options.source),
        out: path_to_string(&options.out),
        spec_path: path_to_string(&spec_path),
        target: target_name(options.target).to_owned(),
        prove: options.prove,
        semantic_status: if semantic_review_required {
            "review_required".to_owned()
        } else {
            "ready_for_release_review".to_owned()
        },
        source_map_path: source_map_report.source_map,
        source_map_markdown_path: source_map_report.markdown_view,
        doctor_report_path: path_to_string(&doctor_report_path),
        grammar_porting_path: path_to_string(&grammar_porting_path),
        grammar_checklist_path: path_to_string(&grammar_checklist_path),
        schema_path: path_to_string(&schema_path),
        shape_crib_path: path_to_string(&shape_crib_path),
        compiled_path: compiled,
        report_path: path_to_string(&report_path),
        source_summary: PortSourceSummary {
            files: source_map.files.len(),
            nodes: source_map.nodes.len(),
            code_blocks: source_map
                .classifications
                .iter()
                .filter(|classification| {
                    classification.kind == source_map::SourceClassificationKind::CodeBlock
                })
                .count(),
            dependency_mentions: source_map
                .classifications
                .iter()
                .filter(|classification| {
                    classification.kind == source_map::SourceClassificationKind::DependencyMention
                })
                .count(),
            review_required: source_map.coverage.review_required,
        },
        qa,
        stats: options.run_dir.map(|run_dir| PortStatsReport {
            run_dir: path_to_string(&run_dir),
            phase: options.phase,
            requirements: options.requirements,
            agent_visible_tokens: 0,
            artifact_tokens_preserved: 0,
            avoided_tokens: 0,
            metrics_source: "estimated".to_owned(),
        }),
        next: vec![
            "review source-map coverage and dependency ledger before semantic promotion".to_owned(),
            format!(
                "edit {}, then rerun validate/imports/deps/test/compile",
                spec_path.display()
            ),
        ],
    })
}

pub fn record_estimated_stats(
    report: &mut PortOneShotReport,
    elapsed: Duration,
    agent_visible_bytes: u64,
) -> Result<()> {
    let metrics = metric_summary(report, elapsed, agent_visible_bytes);
    let Some(stats) = report.stats.as_mut() else {
        return Ok(());
    };
    stats.agent_visible_tokens = metrics.agent_visible_tokens();
    stats.artifact_tokens_preserved = metrics.artifact_tokens_preserved();
    stats.avoided_tokens = metrics.avoided_tokens();
    progress::record_stats(progress::StatsRecordOptions {
        run_dir: PathBuf::from(&stats.run_dir),
        workspace: None,
        phase: stats.phase.clone(),
        requirements: stats.requirements.clone(),
        workspace_stats_json: None,
        workspace_stats_report: None,
        total_tokens: None,
        context_tokens: None,
        query_result_tokens: None,
        response_tokens_cached: None,
        saved_tokens: None,
        reduction_percent: None,
        agent_visible_tokens: Some(stats.agent_visible_tokens),
        artifact_tokens_preserved: Some(stats.artifact_tokens_preserved),
        avoided_tokens: Some(stats.avoided_tokens),
        metrics_source: Some("estimated".to_owned()),
        message: Some("skillspec port-one-shot compact-output estimate".to_owned()),
    })?;
    Ok(())
}

pub fn render_summary(report: &PortOneShotReport, elapsed: Duration) -> String {
    let metrics = metric_summary(report, elapsed, 0);
    metrics::render_with_metrics(metrics, |metrics| {
        let mut output = String::new();
        output.push_str("SkillSpec port-one-shot summary\n\n");
        output.push_str(&format!("- status: {}\n", report.status));
        output.push_str(&format!("- semantic_status: {}\n", report.semantic_status));
        output.push_str(&format!("- source: {}\n", report.source));
        output.push_str(&format!("- out: {}\n", report.out));
        output.push_str(&format!("- spec: {}\n", report.spec_path));
        output.push_str(&format!("- target: {}\n", report.target));
        output.push_str(&format!(
            "- source_map: {} files, {} nodes, {} dependency mentions, {} code blocks\n",
            report.source_summary.files,
            report.source_summary.nodes,
            report.source_summary.dependency_mentions,
            report.source_summary.code_blocks
        ));
        output.push_str(&format!("- report: {}\n", report.report_path));
        output.push('\n');
        output.push_str("proof artifacts:\n");
        output.push_str(&format!(
            "- grammar_porting: {}\n",
            report.grammar_porting_path
        ));
        output.push_str(&format!(
            "- grammar_checklist: {}\n",
            report.grammar_checklist_path
        ));
        output.push_str(&format!("- schema: {}\n", report.schema_path));
        output.push_str(&format!("- shape_crib: {}\n", report.shape_crib_path));
        output.push_str(&format!("- source_map_json: {}\n", report.source_map_path));
        output.push_str(&format!(
            "- source_map_md: {}\n",
            report.source_map_markdown_path
        ));
        output.push_str(&format!("- doctor: {}\n", report.doctor_report_path));
        if let Some(compiled) = &report.compiled_path {
            output.push_str(&format!("- compiled: {}\n", compiled));
        }
        output.push('\n');
        output.push_str("QA gates:\n");
        for gate in &report.qa {
            output.push_str(&format!(
                "- {}: {} ({})\n",
                gate.id, gate.status, gate.message
            ));
        }
        if let Some(stats) = &report.stats {
            output.push('\n');
            output.push_str("stats:\n");
            output.push_str(&format!("- run_dir: {}\n", stats.run_dir));
            output.push_str(&format!(
                "- agent_visible_tokens: ~{}\n",
                stats.agent_visible_tokens
            ));
            output.push_str(&format!(
                "- artifact_tokens_preserved: ~{}\n",
                stats.artifact_tokens_preserved
            ));
            output.push_str(&format!("- avoided_tokens: ~{}\n", stats.avoided_tokens));
            output.push_str("- metrics_source: estimated\n");
        }
        output.push('\n');
        metrics::push_metric_block(&mut output, metrics);
        output.push_str("\nnext:\n");
        for next in &report.next {
            output.push_str(&format!("- {next}\n"));
        }
        output
    })
}

pub fn write_report(report: &PortOneShotReport, rendered: &str) -> Result<()> {
    write_text(Path::new(&report.report_path), rendered)
}

fn metric_summary(
    report: &PortOneShotReport,
    elapsed: Duration,
    agent_visible_bytes: u64,
) -> metrics::MetricSummary {
    let artifact_bytes = metrics::existing_paths_bytes(artifact_paths(report));
    let mut summary = metrics::MetricSummary::new(elapsed, artifact_bytes);
    summary.agent_visible_bytes = agent_visible_bytes;
    summary
}

fn artifact_paths(report: &PortOneShotReport) -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from(&report.spec_path),
        PathBuf::from(&report.source_map_path),
        PathBuf::from(&report.source_map_markdown_path),
        PathBuf::from(&report.doctor_report_path),
        PathBuf::from(&report.grammar_porting_path),
        PathBuf::from(&report.grammar_checklist_path),
        PathBuf::from(&report.schema_path),
        PathBuf::from(&report.shape_crib_path),
    ];
    if let Some(compiled) = &report.compiled_path {
        paths.push(PathBuf::from(compiled));
    }
    paths
}

fn gate(
    id: impl Into<String>,
    status: impl Into<String>,
    message: impl Into<String>,
    path: Option<&Path>,
) -> PortGateReport {
    PortGateReport {
        id: id.into(),
        status: status.into(),
        message: message.into(),
        path: path.map(path_to_string),
    }
}

fn shape_crib_yaml(path: &Path) -> Result<String> {
    let route_id = RouteId("primary_route".to_owned());
    let rule_id = RuleId("prefer_primary_route".to_owned());
    let spec = SkillSpec {
        schema: "skillspec/v0".to_owned(),
        id: "shape.crib".to_owned(),
        title: "Shape Crib".to_owned(),
        description: "Known-valid YAML shapes generated from the current Rust model.".to_owned(),
        activation: None,
        applies_when: Vec::new(),
        entry: None,
        routes: vec![Route {
            id: route_id.clone(),
            label: "Primary route".to_owned(),
            rank: Some(0),
            description: Some("Route with an ordered execution plan.".to_owned()),
            checks: vec!["validate_spec".to_owned()],
            handoff: None,
            execution_plan: Some(ExecutionPlan {
                mode: ExecutionPlanMode::Ordered,
                phases: vec![ExecutionPhase {
                    id: "draft".to_owned(),
                    owner_skill: "shape.crib".to_owned(),
                    route: Some(route_id.clone()),
                    description: Some("Draft from typed structures.".to_owned()),
                    requires: vec!["schema_loaded".to_owned()],
                    checks: vec!["validate_spec".to_owned()],
                    forbid: vec!["freehand_yaml".to_owned()],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: Some(ToolBoundary {
                        default: Some(ToolBoundaryDefault::Deny),
                        allow: vec!["shell".to_owned()],
                        forbid: vec!["network".to_owned()],
                        permission_required_for: vec!["write".to_owned()],
                    }),
                }],
                reason: Some("Execution plans use phases, not freeform steps.".to_owned()),
            }),
            tool_boundary: None,
        }],
        rules: vec![Rule {
            id: rule_id.clone(),
            when: Predicate {
                user_says_any: vec!["port".to_owned()],
                ..Predicate::default()
            },
            prefer: Some(route_id.clone()),
            route_order: vec![route_id.clone()],
            forbid: vec!["skip_validation".to_owned()],
            allow: BTreeMap::new(),
            elicit: vec!["confirm_target".to_owned()],
            after_success: vec!["compile".to_owned()],
            reason: Some("prefer is scalar; elicit is a sequence.".to_owned()),
        }],
        states: BTreeMap::from([
            (
                "drafting".to_owned(),
                State {
                    r#do: vec!["write_typed_yaml".to_owned()],
                    say: Some("drafting_message".to_owned()),
                    ask: None,
                    next: Some("qa".to_owned()),
                    yes: None,
                    no: None,
                },
            ),
            (
                "qa".to_owned(),
                State {
                    r#do: vec!["compile".to_owned()],
                    say: Some("qa_message".to_owned()),
                    ask: None,
                    next: None,
                    yes: None,
                    no: None,
                },
            ),
        ]),
        elicitations: BTreeMap::from([(
            "confirm_target".to_owned(),
            Elicitation {
                question: "Which compile target should be used?".to_owned(),
                required_when: vec![ElicitationCondition {
                    route: Some(route_id.clone()),
                    missing: Some("target".to_owned()),
                    predicate: None,
                }],
                choices: vec![ElicitationChoice {
                    id: "codex".to_owned(),
                    label: "Codex".to_owned(),
                    description: Some("Compile a Codex skill loader.".to_owned()),
                    sets: BTreeMap::new(),
                    route: Some(route_id.clone()),
                    next: None,
                    safety: Some(SafetyClass::LocalWrite),
                }],
                default: Some("codex".to_owned()),
                max_choices: Some(1),
            },
        )]),
        trace: Some(TraceConfig {
            mode: TraceMode::EventLog,
            required: true,
            record: vec![
                TraceEventKind::InputReceived,
                TraceEventKind::SpecLoaded,
                TraceEventKind::RuleEvaluated,
                TraceEventKind::RouteSelected,
                TraceEventKind::OutcomeRecorded,
            ],
        }),
        dependencies: BTreeMap::from([(
            "git".to_owned(),
            Dependency {
                kind: DependencyKind::Cli,
                description: Some("Example CLI dependency with structured permission.".to_owned()),
                command: Some("git".to_owned()),
                path: None,
                env: None,
                check: Some(DependencyCheck {
                    command: Some("git --version".to_owned()),
                    path: None,
                    env: None,
                }),
                permission: Some(DependencyPermission {
                    required: false,
                    reason: Some("Read-only version check.".to_owned()),
                    safety: Some(SafetyClass::LocalRead),
                }),
                provision: Some(DependencyProvision {
                    elicit: Some("confirm_target".to_owned()),
                    options: vec![DependencyProvisionOption {
                        id: "install_git".to_owned(),
                        label: "Install git".to_owned(),
                        description: Some("Install git outside the spec runtime.".to_owned()),
                        command: Some("brew install git".to_owned()),
                        safety: Some(SafetyClass::LocalWrite),
                    }],
                }),
            },
        )]),
        imports: BTreeMap::new(),
        resources: BTreeMap::new(),
        code: BTreeMap::new(),
        artifacts: BTreeMap::from([(
            "qa_report".to_owned(),
            Artifact {
                kind: ArtifactKind::Report,
                description: Some(
                    "Artifacts are consumed only by executable refs: command, code, or recipe."
                        .to_owned(),
                ),
                path: Some(".skillspec/reports/qa.md".to_owned()),
                schema: None,
                produced_by: vec![ProducerRef {
                    kind: ExecutableRefKind::Command,
                    id: "write_report".to_owned(),
                }],
                consumed_by: vec![ConsumerRef {
                    kind: ExecutableRefKind::Command,
                    id: "write_report".to_owned(),
                }],
            },
        )]),
        recipes: BTreeMap::new(),
        commands: BTreeMap::from([(
            "write_report".to_owned(),
            CommandTemplate {
                description: Some(
                    "Command used by the artifact example; routes and rules are not artifact consumers."
                        .to_owned(),
                ),
                template: "skillspec validate skill.spec.yml".to_owned(),
                safety: Some(SafetyClass::LocalRead),
                requires: Default::default(),
                parse: BTreeMap::new(),
                success_when: BTreeMap::new(),
            },
        )]),
        snippets: BTreeMap::from([
            (
                "drafting_message".to_owned(),
                crate::model::Snippet {
                    text: "Drafting from a typed template.".to_owned(),
                },
            ),
            (
                "qa_message".to_owned(),
                crate::model::Snippet {
                    text: "Running the QA gate.".to_owned(),
                },
            ),
        ]),
        closures: BTreeMap::from([
            (
                "write_typed_yaml".to_owned(),
                serde_yaml::Value::String("Patch only from typed shapes.".to_owned()),
            ),
            (
                "compile".to_owned(),
                serde_yaml::Value::String("Compile the reviewed spec.".to_owned()),
            ),
        ]),
        proof: None,
        tests: vec![ScenarioTest {
            name: "routes to primary".to_owned(),
            input: "port this skill".to_owned(),
            expect: Expectation {
                route: Some(route_id),
                matched_rules: vec![rule_id],
                plan_phases: vec!["draft".to_owned()],
                ..Expectation::default()
            },
        }],
        review_required: Vec::new(),
        metadata: BTreeMap::new(),
    };
    parser::validate_spec(&spec)?;
    serde_yaml::to_string(&spec).map_err(|source| Error::RenderYaml {
        path: path.to_path_buf(),
        source,
    })
}

fn target_name(target: compiler::Target) -> &'static str {
    match target {
        compiler::Target::CodexSkill => "codex-skill",
        compiler::Target::ClaudeSkill => "claude-skill",
        compiler::Target::Markdown => "markdown",
    }
}

fn write_text(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let content = serde_json::to_string_pretty(value)?;
    write_text(path, &format!("{content}\n"))
}

fn path_to_string(path: &Path) -> String {
    path.display().to_string()
}
