use crate::error::{Error, Result};
use crate::model::{CommandRequires, Expectation, Predicate, RecipeStep, RouteId, SkillSpec};
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt::Write;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    Index,
    Summary,
    Full,
}

#[derive(Clone, Debug, Serialize)]
pub struct SensemakeReport {
    pub spec_id: String,
    pub title: String,
    pub spec_path: String,
    pub view: View,
    pub sections: Vec<SectionMap>,
    pub navigation: Vec<NavigationHint>,
    pub escalation: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SectionMap {
    pub name: &'static str,
    pub role: &'static str,
    pub count: usize,
    pub ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NavigationHint {
    pub intent: &'static str,
    pub command: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryReport {
    pub spec_id: String,
    pub spec_path: String,
    pub handle: String,
    pub view: View,
    pub target: QueryTarget,
    pub value: Value,
    pub query_hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryTarget {
    pub kind: String,
    pub id: Option<String>,
    pub field_path: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RefsReport {
    pub spec_id: String,
    pub spec_path: String,
    pub handle: String,
    pub view: View,
    pub target: QueryTarget,
    pub outgoing: Vec<ReferenceEdge>,
    pub query_hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReferenceEdge {
    pub field: String,
    pub kind: String,
    pub ids: Vec<String>,
}

pub fn sensemake(spec: &SkillSpec, path: &Path, view: View) -> SensemakeReport {
    let spec_path = path.display().to_string();
    SensemakeReport {
        spec_id: spec.id.clone(),
        title: spec.title.clone(),
        spec_path: spec_path.clone(),
        view,
        sections: sections(spec, view),
        navigation: navigation(spec, &spec_path),
        escalation: escalation(spec),
    }
}

fn escalation(spec: &SkillSpec) -> Vec<String> {
    let mut items = vec![
        "start with sensemake --view index only when unfamiliar".to_owned(),
        "for active task execution, prefer run-loop --guide agent so the CLI prints start/current/end anchors and persists resume state".to_owned(),
        "when several routine proof rows are ready, stage them in <run-dir>/evidence-batch.jsonl and run progress batch --file ... --checkpoint \"checkpointing evidence\" --summary instead of printing one progress record command per row".to_owned(),
        "use decide for task routing".to_owned(),
        "use query/refs for matched ids instead of reading the whole YAML".to_owned(),
        "escalate index -> summary -> full only when needed".to_owned(),
    ];
    if has_capability_bootstrap(spec) {
        items.push(
            "for capability_bootstrap, query ranked local seeds before executing provider tools"
                .to_owned(),
        );
    }
    if has_rote_workspace_synthesis(spec) {
        items.push(
            "synthesize_from_workspace is rote-specific: show the observed result and evidence summary first, then run it with --observation-approved and durable rote workspace stats, command log, and metadata evidence; pass explicit evidence files when live workspace lookup is unreliable"
                .to_owned(),
        );
    }
    if has_doctor(spec) {
        items.push(
            "for source diagnostics, run doctor before import as a cheap current-skill baseline; default doctor output explains agent follow-through risk in plain language, --markdown is for GitHub summaries or issue comments, --html is for shareable review pages, and --json is for machine extraction; for URI imports, stage first and run doctor on the returned local source path; simple skills get full reliability scoring, while multi-skill, entry-with-subskills, plugin, and non-skill repo targets return shape-aware next steps; after doctor, import the skill, read the alignment summary, optionally publish the proof artifacts, restart, and try the SkillSpec-backed skill normally"
                .to_owned(),
        );
    }
    if has_source_import(spec) {
        items.push(
            "for URI imports, first run source stage and use the returned selected_source_path/candidates; for one atomic local prose import, prefer port-one-shot; for manual imports, run source map/query/coverage/stale before import-skill and pass the fresh source-map.json with --source-map"
                .to_owned(),
        );
    }
    if has_workspace_authoring(spec) {
        items.push(
            "for multi-skill or plugin-shaped source roots, run workspace map/validate before fanout import; use workspace converge before compile and workspace install dry-run before writing harness roots"
                .to_owned(),
        );
    }
    if has_router_lifecycle(spec) {
        items.push(
            "treat direct `skillspec index` as router-specific catalog construction only; for installed router maintenance use `skillspec router index refresh`, and for authoring recon use source/workspace map"
                .to_owned(),
        );
    }
    if has_retire_existing_install(spec) {
        items.push(
            "for installs that replace an existing active prose skill, ask for retirement approval and use --retire-existing so the old skill is backed up outside harness discovery roots"
                .to_owned(),
        );
    }
    items
}

pub fn query(spec: &SkillSpec, path: &Path, handle: &str, view: View) -> Result<QueryReport> {
    let parsed = ParsedHandle::parse(handle)?;
    let selected = select_value(spec, &parsed, view)?;
    let query_hints = query_hints(path, &parsed);
    Ok(QueryReport {
        spec_id: spec.id.clone(),
        spec_path: path.display().to_string(),
        handle: handle.to_owned(),
        view,
        target: QueryTarget {
            kind: parsed.kind.clone(),
            id: parsed.id.clone(),
            field_path: parsed.field_path.clone(),
        },
        value: selected,
        query_hints,
    })
}

pub fn refs(spec: &SkillSpec, path: &Path, handle: &str, view: View) -> Result<RefsReport> {
    let parsed = ParsedHandle::parse(handle)?;
    if parsed.id.is_none() {
        return Err(Error::InvalidInput {
            message: format!("refs requires an item handle such as rule:<id>, got {handle:?}"),
        });
    }
    let outgoing = outgoing_refs(spec, &parsed)?;
    let query_hints = refs_hints(path, &parsed, &outgoing);
    Ok(RefsReport {
        spec_id: spec.id.clone(),
        spec_path: path.display().to_string(),
        handle: handle.to_owned(),
        view,
        target: QueryTarget {
            kind: parsed.kind.clone(),
            id: parsed.id.clone(),
            field_path: parsed.field_path.clone(),
        },
        outgoing,
        query_hints,
    })
}

pub fn render_sensemake(report: &SensemakeReport) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "SkillSpec map: {} ({})",
        report.title, report.spec_id
    )
    .unwrap();
    writeln!(output, "spec: {}", report.spec_path).unwrap();
    writeln!(output).unwrap();
    writeln!(output, "Grammar shape:").unwrap();
    for section in &report.sections {
        writeln!(
            output,
            "- {}: {} ({})",
            section.name, section.role, section.count
        )
        .unwrap();
    }
    writeln!(output).unwrap();
    writeln!(output, "Query handles:").unwrap();
    if report.view == View::Index {
        for section in &report.sections {
            output.push_str(&format!(
                "- {}: {} handle(s) hidden in index view\n",
                section.name, section.count
            ));
        }
        output.push_str(&format!(
            "- full handles: `skillspec sensemake {} --view full`",
            report.spec_path
        ));
        output.push('\n');
    } else {
        for section in &report.sections {
            let ids = if section.ids.is_empty() {
                "<none>".to_owned()
            } else {
                section.ids.join(", ")
            };
            output.push_str(&format!("- {}: {}\n", section.name, ids));
        }
    }
    writeln!(output).unwrap();
    writeln!(output, "Navigation:").unwrap();
    for hint in &report.navigation {
        writeln!(output, "- {}: `{}`", hint.intent, hint.command).unwrap();
    }
    writeln!(output).unwrap();
    writeln!(output, "Progressive use:").unwrap();
    for item in &report.escalation {
        writeln!(output, "- {item}").unwrap();
    }
    output
}

pub fn render_query(report: &QueryReport) -> String {
    let mut output = String::new();
    render_target_header(&mut output, &report.target, &report.handle, report.view);
    writeln!(output, "value:").unwrap();
    render_value_lines(&mut output, &report.value);
    if !report.query_hints.is_empty() {
        writeln!(output, "query_hints:").unwrap();
        for hint in &report.query_hints {
            writeln!(output, "- `{hint}`").unwrap();
        }
    }
    output
}

pub fn render_refs(report: &RefsReport) -> String {
    let mut output = String::new();
    render_target_header(&mut output, &report.target, &report.handle, report.view);
    if report.outgoing.is_empty() {
        writeln!(output, "refs: <none>").unwrap();
    } else {
        writeln!(output, "refs:").unwrap();
        for edge in &report.outgoing {
            writeln!(
                output,
                "- {} -> {}: {}",
                edge.field,
                edge.kind,
                edge.ids.join(", ")
            )
            .unwrap();
        }
    }
    if !report.query_hints.is_empty() {
        writeln!(output, "query_hints:").unwrap();
        for hint in &report.query_hints {
            writeln!(output, "- `{hint}`").unwrap();
        }
    }
    output
}

fn sections(spec: &SkillSpec, view: View) -> Vec<SectionMap> {
    let mut maps = vec![
        SectionMap {
            name: "routes",
            role: "strategy choices",
            count: spec.routes.len(),
            ids: spec.routes.iter().map(|route| route.id.0.clone()).collect(),
        },
        SectionMap {
            name: "rules",
            role: "steering logic",
            count: spec.rules.len(),
            ids: spec.rules.iter().map(|rule| rule.id.0.clone()).collect(),
        },
        SectionMap {
            name: "states",
            role: "lifecycle phases",
            count: spec.states.len(),
            ids: spec.states.keys().cloned().collect(),
        },
        SectionMap {
            name: "dependencies",
            role: "static substrate",
            count: spec.dependencies.len(),
            ids: spec.dependencies.keys().cloned().collect(),
        },
        SectionMap {
            name: "commands",
            role: "executable templates",
            count: spec.commands.len(),
            ids: spec.commands.keys().cloned().collect(),
        },
        SectionMap {
            name: "recipes",
            role: "ordered procedures",
            count: spec.recipes.len(),
            ids: spec.recipes.keys().cloned().collect(),
        },
        SectionMap {
            name: "closures",
            role: "completion obligations",
            count: spec.closures.len(),
            ids: spec.closures.keys().cloned().collect(),
        },
        SectionMap {
            name: "tests",
            role: "behavior checks",
            count: spec.tests.len(),
            ids: spec.tests.iter().map(|test| test.name.clone()).collect(),
        },
    ];
    if view != View::Index {
        maps.extend([
            SectionMap {
                name: "trace",
                role: "decision evidence",
                count: usize::from(spec.trace.is_some()),
                ids: spec
                    .trace
                    .as_ref()
                    .map(|_| vec!["trace".to_owned()])
                    .unwrap_or_default(),
            },
            SectionMap {
                name: "proof",
                role: "verification metrics",
                count: usize::from(spec.proof.is_some()),
                ids: spec
                    .proof
                    .as_ref()
                    .map(|_| vec!["proof".to_owned()])
                    .unwrap_or_default(),
            },
            SectionMap {
                name: "imports",
                role: "lazy external context",
                count: spec.imports.len(),
                ids: spec.imports.keys().cloned().collect(),
            },
            SectionMap {
                name: "resources",
                role: "local reference material",
                count: spec.resources.len(),
                ids: spec.resources.keys().cloned().collect(),
            },
        ]);
    }
    maps
}

fn navigation(spec: &SkillSpec, spec_path: &str) -> Vec<NavigationHint> {
    let mut hints = vec![
        NavigationHint {
            intent: "orient",
            command: format!("skillspec sensemake {spec_path} --view index"),
        },
        NavigationHint {
            intent: "start guided task execution",
            command: format!(
                "skillspec run-loop {spec_path} --input '<task>' --trace-dir .skillspec/traces --guide agent"
            ),
        },
        NavigationHint {
            intent: "resume guided task execution",
            command: format!("skillspec run-loop {spec_path} --resume <run-dir> --guide agent"),
        },
        NavigationHint {
            intent: "checkpoint routine proof evidence",
            command: "skillspec progress batch <run-dir> --file <run-dir>/evidence-batch.jsonl --checkpoint \"checkpointing evidence\" --summary".to_owned(),
        },
        NavigationHint {
            intent: "task routing",
            command: format!("skillspec decide {spec_path} --input '<task>'"),
        },
        NavigationHint {
            intent: "inspect active rule",
            command: format!("skillspec query {spec_path} rule:<id> --view summary"),
        },
        NavigationHint {
            intent: "inspect outgoing refs",
            command: format!("skillspec refs {spec_path} rule:<id> --view summary"),
        },
        NavigationHint {
            intent: "inspect command readiness",
            command: format!("skillspec query {spec_path} command:<id>.requires"),
        },
        NavigationHint {
            intent: "check command dependencies",
            command: format!("skillspec deps check {spec_path} --command <id>"),
        },
        NavigationHint {
            intent: "check all dependencies",
            command: format!("skillspec deps check {spec_path}"),
        },
        NavigationHint {
            intent: "inspect lifecycle",
            command: format!("skillspec query {spec_path} state:<id> --view summary"),
        },
        NavigationHint {
            intent: "inspect scenario test",
            command: format!("skillspec query {spec_path} test:<name> --view summary"),
        },
        NavigationHint {
            intent: "prove completion",
            command: format!(
                "skillspec trace align {spec_path} --decision-trace <run_dir> --summary --proof-digest <run_dir>/proof-digest.json"
            ),
        },
    ];
    if spec.dependencies.contains_key("dependency_ledger")
        || spec.artifacts.contains_key("dependency_ledger")
    {
        hints.push(NavigationHint {
            intent: "inspect dependency ledger",
            command: "sed -n '1,240p' <skill-folder>/deps.toml # dependency_count = 0 is valid; byte-empty is not".to_owned(),
        });
    }
    if has_capability_bootstrap(spec) {
        hints.extend([
            NavigationHint {
                intent: "inspect capability bootstrap route",
                command: format!(
                    "skillspec query {spec_path} route:capability_bootstrap --view summary"
                ),
            },
            NavigationHint {
                intent: "rank capability seeds",
                command:
                    "skillspec capability search <capability> --domain <domain> --explain --json"
                        .to_owned(),
            },
            NavigationHint {
                intent: "broaden empty capability search before fallback",
                command:
                    "if selected is null and candidates is empty, search related capability/domain terms before using an unseeded local tool"
                        .to_owned(),
            },
            NavigationHint {
                intent: "verify selected seed",
                command: "skillspec capability verify <seed-id> --domain <domain> --json"
                    .to_owned(),
            },
            NavigationHint {
                intent: "patch seed metadata without rewriting it",
                command:
                    "skillspec capability update <seed-id> --domain <domain> --add-provides <capability> --priority <0-100>"
                        .to_owned(),
            },
        ]);
    }
    if has_rote_workspace_synthesis(spec) {
        hints.extend([
            NavigationHint {
                intent: "inspect rote workspace synthesis command",
                command: format!(
                    "skillspec query {spec_path} command:synthesize_from_workspace --view summary"
                ),
            },
            NavigationHint {
                intent: "synthesize from a rote durable workspace",
                command: "skillspec synthesize-from-workspace <workspace> --task '<task>' --out <skill-folder> --observation-approved"
                    .to_owned(),
            },
        ]);
    }
    if has_router_lifecycle(spec) {
        hints.extend([
            NavigationHint {
                intent: "inspect installed lifecycle, roots, index, and skill inventory status",
                command: "skillspec status --json".to_owned(),
            },
            NavigationHint {
                intent: "enable router mode and rebuild index",
                command: "skillspec router enable --json".to_owned(),
            },
            NavigationHint {
                intent: "disable router mode without uninstalling",
                command: "skillspec router disable --json".to_owned(),
            },
            NavigationHint {
                intent: "refresh installed router index",
                command:
                    "skillspec router index refresh --roots <skill-roots> --index <router-index>"
                        .to_owned(),
            },
        ]);
    }
    if has_durable_lifecycle(spec) {
        hints.extend([
            NavigationHint {
                intent: "install durable-executor lifecycle after rote preflight",
                command:
                    "skillspec durable-executor install <source-folder> --target <target> --json"
                        .to_owned(),
            },
            NavigationHint {
                intent: "update durable-executor lifecycle after rote preflight",
                command: "skillspec durable-executor update --json".to_owned(),
            },
            NavigationHint {
                intent: "delete durable-executor lifecycle",
                command: "skillspec durable-executor delete --json".to_owned(),
            },
            NavigationHint {
                intent: "enable durable-executor implicit first-hop after rote preflight",
                command: "skillspec durable-executor enable --json".to_owned(),
            },
            NavigationHint {
                intent: "disable durable-executor implicit first-hop",
                command: "skillspec durable-executor disable --json".to_owned(),
            },
        ]);
    }
    if has_source_import(spec) {
        hints.extend([
            NavigationHint {
                intent: "stage remote source URI before import",
                command: "skillspec source stage <github-skill-uri> --out <staging-root> --json"
                    .to_owned(),
            },
            NavigationHint {
                intent: "diagnose source shape and prose reliability debt",
                command: "skillspec doctor <source-skill-folder-or-repo-uri>".to_owned(),
            },
            NavigationHint {
                intent: "map import source",
                command:
                    "skillspec source map <source-skill> --out <draft>/.skillspec/source-map"
                        .to_owned(),
            },
            NavigationHint {
                intent: "inspect source structure",
                command:
                    "skillspec source query <draft>/.skillspec/source-map/source-map.json nodes --view index"
                        .to_owned(),
            },
            NavigationHint {
                intent: "inspect source dependencies",
                command:
                    "skillspec source query <draft>/.skillspec/source-map/source-map.json dependencies --view summary"
                        .to_owned(),
            },
            NavigationHint {
                intent: "one-shot port atomic prose skill",
                command:
                    "skillspec port-one-shot <source-skill> --out <draft> --target codex-skill --prove"
                        .to_owned(),
            },
        ]);
    }
    if has_workspace_authoring(spec) {
        hints.extend([
            NavigationHint {
                intent: "map workspace source root",
                command:
                    "skillspec workspace map <source-root> --out <build>/skillspec.workspace.yml --summary"
                        .to_owned(),
            },
            NavigationHint {
                intent: "validate workspace graph",
                command: "skillspec workspace validate <build>/skillspec.workspace.yml --summary"
                    .to_owned(),
            },
            NavigationHint {
                intent: "fanout import workspace packages",
                command:
                    "skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build> --summary"
                        .to_owned(),
            },
            NavigationHint {
                intent: "converge workspace build",
                command:
                    "skillspec workspace converge <build>/skillspec.workspace.yml --build-root <workspace-build> --summary"
                        .to_owned(),
            },
            NavigationHint {
                intent: "compile workspace build",
                command:
                    "skillspec workspace compile <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex-skill --summary"
                        .to_owned(),
            },
            NavigationHint {
                intent: "dry-run workspace install",
                command:
                    "skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --dry-run --summary"
                        .to_owned(),
            },
        ]);
    }
    if has_retire_existing_install(spec) {
        hints.extend([
            NavigationHint {
                intent: "inspect active-skill retirement gate",
                command: format!(
                    "skillspec query {spec_path} elicitation:approve_retire_existing_skill --view summary"
                ),
            },
            NavigationHint {
                intent: "install while retiring an old active skill",
                command:
                    "skillspec install skill <skill-folder> --target <target> --retire-existing"
                        .to_owned(),
            },
        ]);
    }
    hints
}

fn has_capability_bootstrap(spec: &SkillSpec) -> bool {
    spec.routes
        .iter()
        .any(|route| route.id.0 == "capability_bootstrap")
        || spec.resources.contains_key("local_capability_seed_store")
        || spec
            .commands
            .keys()
            .any(|id| id.contains("capability_seed"))
}

fn has_rote_workspace_synthesis(spec: &SkillSpec) -> bool {
    spec.commands.contains_key("synthesize_from_workspace")
        || spec
            .commands
            .values()
            .any(|command| command.template.contains("synthesize-from-workspace"))
}

fn has_durable_lifecycle(spec: &SkillSpec) -> bool {
    spec.commands.contains_key("durable_install")
        || spec.commands.contains_key("durable_update")
        || spec.commands.contains_key("durable_delete")
        || spec
            .commands
            .values()
            .any(|command| command.template.contains("durable-executor"))
}

fn has_router_lifecycle(spec: &SkillSpec) -> bool {
    spec.commands.contains_key("router_install")
        || spec.commands.contains_key("router_update")
        || spec.commands.contains_key("status_lifecycle_inventory")
        || spec.commands.contains_key("router_enable")
        || spec.commands.contains_key("router_disable")
        || spec.commands.values().any(|command| {
            command.template.contains("router enable")
                || command.template.contains("skillspec status")
        })
}

fn has_source_import(spec: &SkillSpec) -> bool {
    spec.commands.contains_key("import_skill_draft")
        || spec
            .commands
            .values()
            .any(|command| command.template.contains("import-skill"))
}

fn has_workspace_authoring(spec: &SkillSpec) -> bool {
    spec.commands.contains_key("workspace_map_source")
        || spec.commands.contains_key("workspace_import_packages")
        || spec.commands.contains_key("workspace_converge_build")
        || spec.commands.contains_key("workspace_compile_build")
        || spec.commands.contains_key("workspace_install_packages")
        || spec
            .commands
            .values()
            .any(|command| command.template.contains("skillspec workspace "))
}

fn has_doctor(spec: &SkillSpec) -> bool {
    spec.commands.contains_key("doctor_source_skill")
        || spec.artifacts.contains_key("doctor_report")
        || spec
            .commands
            .values()
            .any(|command| command.template.contains("skillspec doctor"))
}

fn has_retire_existing_install(spec: &SkillSpec) -> bool {
    spec.elicitations
        .contains_key("approve_retire_existing_skill")
        || spec
            .commands
            .values()
            .any(|command| command.template.contains("--retire-existing"))
}

#[derive(Clone, Debug)]
struct ParsedHandle {
    kind: String,
    id: Option<String>,
    field_path: Vec<String>,
}

impl ParsedHandle {
    fn parse(handle: &str) -> Result<Self> {
        let handle = handle.trim();
        if handle.is_empty() {
            return Err(Error::InvalidInput {
                message: "query handle cannot be empty".to_owned(),
            });
        }
        let (base, field_path) = match handle.split_once('.') {
            Some((base, fields)) => (
                base,
                fields
                    .split('.')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect(),
            ),
            None => (handle, Vec::new()),
        };
        if let Some((kind, id)) = base.split_once(':') {
            if id.is_empty() {
                return Err(Error::InvalidInput {
                    message: format!("query handle {handle:?} is missing an id"),
                });
            }
            return Ok(Self {
                kind: kind.to_owned(),
                id: Some(id.to_owned()),
                field_path,
            });
        }
        Ok(Self {
            kind: base.to_owned(),
            id: None,
            field_path,
        })
    }
}

fn select_value(spec: &SkillSpec, parsed: &ParsedHandle, view: View) -> Result<Value> {
    let base_view = if parsed.field_path.is_empty() {
        view
    } else {
        View::Full
    };
    let mut value = match (parsed.kind.as_str(), parsed.id.as_deref()) {
        ("routes", None) => collection(
            spec.routes
                .iter()
                .map(|route| route.id.0.as_str())
                .collect(),
            spec.routes.iter().map(route_summary).collect(),
            &spec.routes,
            base_view,
        )?,
        ("rules", None) => collection(
            spec.rules.iter().map(|rule| rule.id.0.as_str()).collect(),
            spec.rules.iter().map(rule_summary).collect(),
            &spec.rules,
            base_view,
        )?,
        ("states", None) => map_collection(&spec.states, base_view)?,
        ("dependencies", None) => map_collection(&spec.dependencies, base_view)?,
        ("commands", None) => map_collection_summary(&spec.commands, base_view, command_summary)?,
        ("recipes", None) => map_collection(&spec.recipes, base_view)?,
        ("closures", None) => map_collection(&spec.closures, base_view)?,
        ("tests", None) => collection(
            spec.tests.iter().map(|test| test.name.as_str()).collect(),
            spec.tests.iter().map(test_summary).collect(),
            &spec.tests,
            base_view,
        )?,
        ("trace", None) => option_value(&spec.trace, base_view)?,
        ("proof", None) => option_value(&spec.proof, base_view)?,
        ("imports", None) => map_collection(&spec.imports, base_view)?,
        ("resources", None) => map_collection(&spec.resources, base_view)?,
        ("code", None) => map_collection(&spec.code, base_view)?,
        ("artifacts", None) => map_collection(&spec.artifacts, base_view)?,
        ("snippets", None) => map_collection(&spec.snippets, base_view)?,
        ("elicitations", None) => map_collection(&spec.elicitations, base_view)?,
        ("route", Some(id)) => item(
            id,
            spec.routes
                .iter()
                .find(|route| route.id.0 == id)
                .map(|route| (route_summary(route), route)),
            base_view,
            "route",
        )?,
        ("rule", Some(id)) => item(
            id,
            spec.rules
                .iter()
                .find(|rule| rule.id.0 == id)
                .map(|rule| (rule_summary(rule), rule)),
            base_view,
            "rule",
        )?,
        ("state", Some(id)) => map_item(id, &spec.states, base_view, "state")?,
        ("dependency", Some(id)) => map_item(id, &spec.dependencies, base_view, "dependency")?,
        ("command", Some(id)) => item(
            id,
            spec.commands
                .get(id)
                .map(|command| (command_summary(id, command), command)),
            base_view,
            "command",
        )?,
        ("recipe", Some(id)) => map_item(id, &spec.recipes, base_view, "recipe")?,
        ("closure", Some(id)) => map_item(id, &spec.closures, base_view, "closure")?,
        ("import", Some(id)) => map_item(id, &spec.imports, base_view, "import")?,
        ("resource", Some(id)) => map_item(id, &spec.resources, base_view, "resource")?,
        ("code", Some(id)) => map_item(id, &spec.code, base_view, "code")?,
        ("artifact", Some(id)) => map_item(id, &spec.artifacts, base_view, "artifact")?,
        ("snippet", Some(id)) => map_item(id, &spec.snippets, base_view, "snippet")?,
        ("elicitation", Some(id)) => map_item(id, &spec.elicitations, base_view, "elicitation")?,
        ("test", Some(id)) => item(
            id,
            spec.tests
                .iter()
                .find(|test| test.name == id)
                .map(|test| (test_summary(test), test)),
            base_view,
            "test",
        )?,
        _ => {
            return Err(Error::InvalidInput {
                message: format!("unknown query handle {kind}", kind = parsed.kind),
            });
        }
    };
    if !parsed.field_path.is_empty() {
        value = project_field(value, &parsed.field_path, &parsed.kind)?;
    }
    Ok(value)
}

fn collection<T: Serialize>(
    ids: Vec<&str>,
    summaries: Vec<Value>,
    full: &T,
    view: View,
) -> Result<Value> {
    match view {
        View::Index => Ok(json!(ids)),
        View::Summary => Ok(Value::Array(summaries)),
        View::Full => Ok(serde_json::to_value(full)?),
    }
}

fn map_collection<T: Serialize>(map: &impl MapLike<T>, view: View) -> Result<Value> {
    match view {
        View::Index => Ok(json!(map.keys())),
        View::Summary => Ok(json!(map.keys())),
        View::Full => Ok(serde_json::to_value(map.as_value())?),
    }
}

fn map_collection_summary<T: Serialize>(
    map: &impl MapLike<T>,
    view: View,
    summarize: fn(&str, &T) -> Value,
) -> Result<Value> {
    match view {
        View::Index => Ok(json!(map.keys())),
        View::Summary => Ok(Value::Array(
            map.entries()
                .into_iter()
                .map(|(id, value)| summarize(id, value))
                .collect(),
        )),
        View::Full => Ok(serde_json::to_value(map.as_value())?),
    }
}

fn option_value<T: Serialize>(option: &Option<T>, view: View) -> Result<Value> {
    match (option, view) {
        (Some(value), View::Full) => Ok(serde_json::to_value(value)?),
        (Some(_), _) => Ok(json!(["present"])),
        (None, _) => Ok(json!([])),
    }
}

fn item<T: Serialize>(
    id: &str,
    item: Option<(Value, &T)>,
    view: View,
    kind: &str,
) -> Result<Value> {
    let Some((summary, full)) = item else {
        return Err(Error::InvalidInput {
            message: format!("unknown {kind} id {id:?}"),
        });
    };
    match view {
        View::Index => Ok(json!({
            "id": id,
            "fields": fields_for(full)?,
        })),
        View::Summary => Ok(summary),
        View::Full => Ok(serde_json::to_value(full)?),
    }
}

fn map_item<T: Serialize>(
    id: &str,
    map: &impl MapLike<T>,
    view: View,
    kind: &str,
) -> Result<Value> {
    item(
        id,
        map.get(id)
            .map(|value| (summary_for_value(id, value), value)),
        view,
        kind,
    )
}

trait MapLike<T: Serialize>: Serialize {
    fn keys(&self) -> Vec<String>;
    fn entries(&self) -> Vec<(&str, &T)>;
    fn get(&self, id: &str) -> Option<&T>;
    fn as_value(&self) -> &Self;
}

impl<T: Serialize> MapLike<T> for std::collections::BTreeMap<String, T> {
    fn keys(&self) -> Vec<String> {
        self.keys().cloned().collect()
    }

    fn entries(&self) -> Vec<(&str, &T)> {
        self.iter()
            .map(|(key, value)| (key.as_str(), value))
            .collect()
    }

    fn get(&self, id: &str) -> Option<&T> {
        std::collections::BTreeMap::get(self, id)
    }

    fn as_value(&self) -> &Self {
        self
    }
}

fn fields_for<T: Serialize>(value: &T) -> Result<Vec<String>> {
    match serde_json::to_value(value)? {
        Value::Object(object) => Ok(object.keys().cloned().collect()),
        _ => Ok(Vec::new()),
    }
}

fn summary_for_value<T: Serialize>(id: &str, value: &T) -> Value {
    match serde_json::to_value(value) {
        Ok(Value::Object(mut object)) => {
            object.insert("id".to_owned(), Value::String(id.to_owned()));
            Value::Object(object)
        }
        Ok(value) => json!({"id": id, "value": value}),
        Err(_) => json!({"id": id}),
    }
}

fn project_field(mut value: Value, path: &[String], kind: &str) -> Result<Value> {
    for part in path {
        match value {
            Value::Object(mut object) => {
                value = object.remove(part).ok_or_else(|| Error::InvalidInput {
                    message: format!("unknown field {part:?} on {kind}"),
                })?;
            }
            _ => {
                return Err(Error::InvalidInput {
                    message: format!("{kind} does not contain field {part:?}"),
                });
            }
        }
    }
    Ok(value)
}

fn route_summary(route: &crate::model::Route) -> Value {
    json!({
        "id": route.id.0,
        "label": route.label,
        "rank": route.rank,
        "description": route.description,
        "checks": route.checks,
        "handoff": route.handoff,
        "execution_plan": route.execution_plan,
    })
}

fn rule_summary(rule: &crate::model::Rule) -> Value {
    json!({
        "id": rule.id.0,
        "fields": non_empty_rule_fields(rule),
        "when": predicate_summary(&rule.when),
        "prefer": route_id(rule.prefer.as_ref()),
        "route_order": route_ids(&rule.route_order),
        "forbids": rule.forbid,
        "allows": rule.allow.keys().cloned().collect::<Vec<_>>(),
        "elicit": rule.elicit,
        "after_success": rule.after_success,
        "reason": rule.reason,
    })
}

fn command_summary(id: &str, command: &crate::model::CommandTemplate) -> Value {
    json!({
        "id": id,
        "description": command.description,
        "safety": command.safety,
        "requires": requires_summary(&command.requires),
        "parse_fields": command.parse.keys().cloned().collect::<Vec<_>>(),
        "success_when": command.success_when.keys().cloned().collect::<Vec<_>>(),
    })
}

fn test_summary(test: &crate::model::ScenarioTest) -> Value {
    json!({
        "name": test.name,
        "input": test.input,
        "expect_fields": non_empty_expectation_fields(&test.expect),
        "route": route_id(test.expect.route.as_ref()),
        "matched_rules": rule_ids(&test.expect.matched_rules),
        "matched_rules_exact": test.expect.matched_rules_exact.as_ref().map(|rules| rule_ids(rules)),
        "elicit": test.expect.elicit,
        "elicit_exact": test.expect.elicit_exact,
        "after_success": test.expect.after_success,
        "after_success_exact": test.expect.after_success_exact,
    })
}

fn predicate_summary(predicate: &Predicate) -> Value {
    json!({
        "user_says_any": predicate.user_says_any,
        "user_says_all_groups": predicate.user_says_all_groups,
        "task_recurrence_likely": predicate.task_recurrence_likely,
        "domain_object_task": predicate.domain_object_task,
        "interactive_prompt_likely": predicate.interactive_prompt_likely,
        "command_likely_long_running": predicate.command_likely_long_running,
    })
}

fn requires_summary(requires: &CommandRequires) -> Value {
    json!({
        "dependencies": requires.dependencies,
        "resources": requires.resources,
        "files": requires.files,
        "env": requires.env,
        "auth": requires.auth,
    })
}

fn route_id(route: Option<&RouteId>) -> Option<String> {
    route.map(|route| route.0.clone())
}

fn route_ids(routes: &[RouteId]) -> Vec<String> {
    routes.iter().map(|route| route.0.clone()).collect()
}

fn rule_ids(rules: &[crate::model::RuleId]) -> Vec<String> {
    rules.iter().map(|rule| rule.0.clone()).collect()
}

fn non_empty_rule_fields(rule: &crate::model::Rule) -> Vec<&'static str> {
    let mut fields = vec!["when"];
    if rule.prefer.is_some() {
        fields.push("prefer");
    }
    if !rule.route_order.is_empty() {
        fields.push("route_order");
    }
    if !rule.forbid.is_empty() {
        fields.push("forbid");
    }
    if !rule.allow.is_empty() {
        fields.push("allow");
    }
    if !rule.elicit.is_empty() {
        fields.push("elicit");
    }
    if !rule.after_success.is_empty() {
        fields.push("after_success");
    }
    if rule.reason.is_some() {
        fields.push("reason");
    }
    fields
}

fn non_empty_expectation_fields(expectation: &Expectation) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if expectation.route.is_some() {
        fields.push("route");
    }
    if !expectation.route_order.is_empty() {
        fields.push("route_order");
    }
    if !expectation.plan_phases.is_empty() {
        fields.push("plan_phases");
    }
    if !expectation.plan_jumps.is_empty() {
        fields.push("plan_jumps");
    }
    if !expectation.forbid.is_empty() {
        fields.push("forbid");
    }
    if expectation.forbid_exact.is_some() {
        fields.push("forbid_exact");
    }
    if !expectation.not_forbid.is_empty() {
        fields.push("not_forbid");
    }
    if !expectation.elicit.is_empty() {
        fields.push("elicit");
    }
    if expectation.elicit_exact.is_some() {
        fields.push("elicit_exact");
    }
    if !expectation.not_elicit.is_empty() {
        fields.push("not_elicit");
    }
    if !expectation.after_success.is_empty() {
        fields.push("after_success");
    }
    if expectation.after_success_exact.is_some() {
        fields.push("after_success_exact");
    }
    if !expectation.not_after_success.is_empty() {
        fields.push("not_after_success");
    }
    if !expectation.matched_rules.is_empty() {
        fields.push("matched_rules");
    }
    if expectation.matched_rules_exact.is_some() {
        fields.push("matched_rules_exact");
    }
    if !expectation.not_matched_rules.is_empty() {
        fields.push("not_matched_rules");
    }
    fields
}

fn outgoing_refs(spec: &SkillSpec, parsed: &ParsedHandle) -> Result<Vec<ReferenceEdge>> {
    let id = parsed.id.as_deref().unwrap_or_default();
    match parsed.kind.as_str() {
        "rule" => {
            let rule = spec
                .rules
                .iter()
                .find(|rule| rule.id.0 == id)
                .ok_or_else(|| Error::InvalidInput {
                    message: format!("unknown rule id {id:?}"),
                })?;
            let mut edges = Vec::new();
            if let Some(route) = &rule.prefer {
                edges.push(edge("prefer", "route", vec![route.0.clone()]));
            }
            if !rule.route_order.is_empty() {
                edges.push(edge("route_order", "route", route_ids(&rule.route_order)));
            }
            if !rule.forbid.is_empty() {
                edges.push(edge("forbid", "forbid", rule.forbid.clone()));
            }
            if !rule.elicit.is_empty() {
                edges.push(edge("elicit", "elicitation", rule.elicit.clone()));
            }
            if !rule.after_success.is_empty() {
                edges.push(edge(
                    "after_success",
                    "command_or_recipe_or_state",
                    rule.after_success.clone(),
                ));
            }
            Ok(edges)
        }
        "command" => {
            let command = spec.commands.get(id).ok_or_else(|| Error::InvalidInput {
                message: format!("unknown command id {id:?}"),
            })?;
            let mut edges = Vec::new();
            if !command.requires.dependencies.is_empty() {
                edges.push(edge(
                    "requires.dependencies",
                    "dependency",
                    command.requires.dependencies.clone(),
                ));
            }
            if !command.requires.files.is_empty() {
                edges.push(edge(
                    "requires.files",
                    "file",
                    command.requires.files.clone(),
                ));
            }
            if !command.requires.resources.is_empty() {
                edges.push(edge(
                    "requires.resources",
                    "resource",
                    command.requires.resources.clone(),
                ));
            }
            if !command.requires.env.is_empty() {
                edges.push(edge("requires.env", "env", command.requires.env.clone()));
            }
            if !command.requires.auth.is_empty() {
                edges.push(edge("requires.auth", "auth", command.requires.auth.clone()));
            }
            Ok(edges)
        }
        "state" => {
            let state = spec.states.get(id).ok_or_else(|| Error::InvalidInput {
                message: format!("unknown state id {id:?}"),
            })?;
            let mut edges = Vec::new();
            for (field, value) in [
                ("next", &state.next),
                ("yes", &state.yes),
                ("no", &state.no),
            ] {
                if let Some(value) = value {
                    edges.push(edge(field, "state", vec![value.clone()]));
                }
            }
            Ok(edges)
        }
        "recipe" => {
            let recipe = spec.recipes.get(id).ok_or_else(|| Error::InvalidInput {
                message: format!("unknown recipe id {id:?}"),
            })?;
            let mut edges = Vec::new();
            if !recipe.requires.imports.is_empty() {
                edges.push(edge("requires.imports", "import", recipe.requires.imports.clone()));
            }
            if !recipe.requires.resources.is_empty() {
                edges.push(edge(
                    "requires.resources",
                    "resource",
                    recipe.requires.resources.clone(),
                ));
            }
            if !recipe.requires.dependencies.is_empty() {
                edges.push(edge(
                    "requires.dependencies",
                    "dependency",
                    recipe.requires.dependencies.clone(),
                ));
            }
            if !recipe.requires.artifacts.is_empty() {
                edges.push(edge(
                    "requires.artifacts",
                    "artifact",
                    recipe.requires.artifacts.clone(),
                ));
            }
            for step in &recipe.steps {
                if let Some(edge) = recipe_step_edge(step) {
                    edges.push(edge);
                }
            }
            Ok(edges)
        }
        "route" => {
            let route = spec
                .routes
                .iter()
                .find(|route| route.id.0 == id)
                .ok_or_else(|| Error::InvalidInput {
                    message: format!("unknown route id {id:?}"),
                })?;
            let mut edges = Vec::new();
            if !route.checks.is_empty() {
                edges.push(edge("checks", "check", route.checks.clone()));
            }
            if let Some(handoff) = &route.handoff {
                edges.push(edge(
                    "handoff.to_skill",
                    "skill",
                    vec![handoff.to_skill.clone()],
                ));
            }
            if let Some(plan) = &route.execution_plan {
                let owner_skills = plan
                    .phases
                    .iter()
                    .map(|phase| phase.owner_skill.clone())
                    .collect::<Vec<_>>();
                if !owner_skills.is_empty() {
                    edges.push(edge("execution_plan.owner_skill", "skill", owner_skills));
                }
                let phase_routes = plan
                    .phases
                    .iter()
                    .filter_map(|phase| phase.route.as_ref().map(|route| route.0.clone()))
                    .collect::<Vec<_>>();
                if !phase_routes.is_empty() {
                    edges.push(edge("execution_plan.route", "route", phase_routes));
                }
                let handoff_targets = plan
                    .phases
                    .iter()
                    .filter_map(|phase| phase.handoff.as_ref().map(|handoff| handoff.to_skill.clone()))
                    .collect::<Vec<_>>();
                if !handoff_targets.is_empty() {
                    edges.push(edge(
                        "execution_plan.handoff.to_skill",
                        "skill",
                        handoff_targets,
                    ));
                }
                let jump_targets = plan
                    .phases
                    .iter()
                    .flat_map(|phase| phase.jumps.iter().map(|jump| jump.to_phase.clone()))
                    .collect::<Vec<_>>();
                if !jump_targets.is_empty() {
                    edges.push(edge("execution_plan.jump.to_phase", "phase", jump_targets));
                }
            }
            Ok(edges)
        }
        "test" => {
            let test = spec
                .tests
                .iter()
                .find(|test| test.name == id)
                .ok_or_else(|| Error::InvalidInput {
                    message: format!("unknown test id {id:?}"),
                })?;
            Ok(test_expectation_edges(&test.expect))
        }
        _ => Err(Error::InvalidInput {
            message: format!(
                "refs supports route:<id>, rule:<id>, state:<id>, command:<id>, recipe:<id>, and test:<name>; got {kind}",
                kind = parsed.kind
            ),
        }),
    }
}

fn edge(field: &str, kind: &str, ids: Vec<String>) -> ReferenceEdge {
    ReferenceEdge {
        field: field.to_owned(),
        kind: kind.to_owned(),
        ids,
    }
}

fn test_expectation_edges(expectation: &Expectation) -> Vec<ReferenceEdge> {
    let mut edges = Vec::new();
    if let Some(route) = &expectation.route {
        edges.push(edge("expect.route", "route", vec![route.0.clone()]));
    }
    if !expectation.route_order.is_empty() {
        edges.push(edge(
            "expect.route_order",
            "route",
            route_ids(&expectation.route_order),
        ));
    }
    push_string_edge(&mut edges, "expect.forbid", "forbid", &expectation.forbid);
    if let Some(ids) = &expectation.forbid_exact {
        push_string_edge(&mut edges, "expect.forbid_exact", "forbid", ids);
    }
    push_string_edge(
        &mut edges,
        "expect.not_forbid",
        "forbid",
        &expectation.not_forbid,
    );
    push_string_edge(
        &mut edges,
        "expect.elicit",
        "elicitation",
        &expectation.elicit,
    );
    if let Some(ids) = &expectation.elicit_exact {
        push_string_edge(&mut edges, "expect.elicit_exact", "elicitation", ids);
    }
    push_string_edge(
        &mut edges,
        "expect.not_elicit",
        "elicitation",
        &expectation.not_elicit,
    );
    push_string_edge(
        &mut edges,
        "expect.after_success",
        "command_or_recipe_or_state",
        &expectation.after_success,
    );
    if let Some(ids) = &expectation.after_success_exact {
        push_string_edge(
            &mut edges,
            "expect.after_success_exact",
            "command_or_recipe_or_state",
            ids,
        );
    }
    push_string_edge(
        &mut edges,
        "expect.not_after_success",
        "command_or_recipe_or_state",
        &expectation.not_after_success,
    );
    if !expectation.matched_rules.is_empty() {
        edges.push(edge(
            "expect.matched_rules",
            "rule",
            rule_ids(&expectation.matched_rules),
        ));
    }
    if let Some(rules) = &expectation.matched_rules_exact {
        edges.push(edge("expect.matched_rules_exact", "rule", rule_ids(rules)));
    }
    if !expectation.not_matched_rules.is_empty() {
        edges.push(edge(
            "expect.not_matched_rules",
            "rule",
            rule_ids(&expectation.not_matched_rules),
        ));
    }
    if !expectation.plan_phases.is_empty() {
        edges.push(edge(
            "expect.plan_phases",
            "phase",
            expectation.plan_phases.clone(),
        ));
    }
    edges
}

fn push_string_edge(edges: &mut Vec<ReferenceEdge>, field: &str, kind: &str, ids: &[String]) {
    if !ids.is_empty() {
        edges.push(edge(field, kind, ids.to_vec()));
    }
}

fn recipe_step_edge(step: &RecipeStep) -> Option<ReferenceEdge> {
    match step {
        RecipeStep::LoadImport(step) => Some(edge(
            "steps.load_import",
            "import",
            vec![step.load_import.clone()],
        )),
        RecipeStep::LoadResource(step) => Some(edge(
            "steps.load_resource",
            "resource",
            vec![step.load_resource.clone()],
        )),
        RecipeStep::RunCommand(step) => Some(edge(
            "steps.run_command",
            "command",
            vec![step.run_command.clone()],
        )),
        RecipeStep::RunCode(step) => {
            Some(edge("steps.run_code", "code", vec![step.run_code.clone()]))
        }
        RecipeStep::ProduceArtifact(step) => Some(edge(
            "steps.produce_artifact",
            "artifact",
            vec![step.produce_artifact.clone()],
        )),
        RecipeStep::ConsumeArtifact(step) => Some(edge(
            "steps.consume_artifact",
            "artifact",
            vec![step.consume_artifact.clone()],
        )),
        RecipeStep::Ask(step) => Some(edge("steps.ask", "elicitation", vec![step.ask.clone()])),
        RecipeStep::Branch(step) => {
            let mut ids = vec![step.branch.then.clone()];
            if let Some(otherwise) = &step.branch.otherwise {
                ids.push(otherwise.clone());
            }
            Some(edge("steps.branch", "state", ids))
        }
        RecipeStep::Note(_) => None,
    }
}

fn query_hints(path: &Path, parsed: &ParsedHandle) -> Vec<String> {
    let path = path.display();
    let Some(id) = parsed.id.as_deref() else {
        let handle = shell_arg(&parsed.kind);
        return vec![format!("skillspec query {path} {handle} --view summary")];
    };
    match parsed.kind.as_str() {
        "rule" => vec![
            format!(
                "skillspec query {path} {}",
                shell_arg(&format!("rule:{id}.forbid"))
            ),
            format!(
                "skillspec query {path} {}",
                shell_arg(&format!("rule:{id}.after_success"))
            ),
            format!(
                "skillspec refs {path} {} --view summary",
                shell_arg(&format!("rule:{id}"))
            ),
        ],
        "command" => vec![
            format!(
                "skillspec query {path} {}",
                shell_arg(&format!("command:{id}.requires"))
            ),
            format!("skillspec deps check {path} --command {id}"),
            format!(
                "skillspec refs {path} {} --view summary",
                shell_arg(&format!("command:{id}"))
            ),
        ],
        "state" => vec![
            format!(
                "skillspec query {path} {}",
                shell_arg(&format!("state:{id}.next"))
            ),
            format!(
                "skillspec refs {path} {} --view summary",
                shell_arg(&format!("state:{id}"))
            ),
        ],
        "route" => vec![format!(
            "skillspec query {path} {}",
            shell_arg(&format!("route:{id}.checks"))
        )],
        "test" => vec![
            format!(
                "skillspec query {path} {} --view full",
                shell_arg(&format!("test:{id}.expect"))
            ),
            format!(
                "skillspec refs {path} {} --view summary",
                shell_arg(&format!("test:{id}"))
            ),
        ],
        _ => Vec::new(),
    }
}

fn refs_hints(path: &Path, parsed: &ParsedHandle, outgoing: &[ReferenceEdge]) -> Vec<String> {
    let path = path.display();
    let mut hints = Vec::new();
    for edge in outgoing {
        for id in &edge.ids {
            let command = match edge.kind.as_str() {
                "route" => Some(format!("skillspec query {path} route:{id} --view summary")),
                "dependency" => Some(format!(
                    "skillspec query {path} dependency:{id} --view summary"
                )),
                "elicitation" => Some(format!(
                    "skillspec query {path} elicitation:{id} --view summary"
                )),
                "command" => Some(format!(
                    "skillspec query {path} command:{id} --view summary"
                )),
                "state" => Some(format!("skillspec query {path} state:{id} --view summary")),
                "import" => Some(format!("skillspec query {path} import:{id} --view summary")),
                "resource" => Some(format!(
                    "skillspec query {path} resource:{id} --view summary"
                )),
                "artifact" => Some(format!(
                    "skillspec query {path} artifact:{id} --view summary"
                )),
                "code" => Some(format!("skillspec query {path} code:{id} --view summary")),
                _ => None,
            };
            if let Some(command) = command {
                hints.push(command);
            }
        }
    }
    if hints.is_empty() {
        if let Some(id) = parsed.id.as_deref() {
            let handle = shell_arg(&format!("{}:{id}", parsed.kind));
            hints.push(format!("skillspec query {path} {handle} --view full",));
        }
    }
    hints
}

fn shell_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '.' | ':' | '='))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn render_target_header(output: &mut String, target: &QueryTarget, handle: &str, view: View) {
    writeln!(output, "handle: {handle}").unwrap();
    writeln!(output, "target: {}", target_name(target)).unwrap();
    writeln!(output, "view: {}", view_name(view)).unwrap();
    if !target.field_path.is_empty() {
        writeln!(output, "field_path: {}", target.field_path.join(".")).unwrap();
    }
}

fn target_name(target: &QueryTarget) -> String {
    match &target.id {
        Some(id) => format!("{}:{}", target.kind, id),
        None => target.kind.clone(),
    }
}

fn view_name(view: View) -> &'static str {
    match view {
        View::Index => "index",
        View::Summary => "summary",
        View::Full => "full",
    }
}

fn render_value_lines(output: &mut String, value: &Value) {
    match value {
        Value::Array(values) if values.iter().all(Value::is_string) => {
            let values = values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                output,
                "- {}",
                if values.is_empty() { "<none>" } else { &values }
            )
            .unwrap();
        }
        Value::Object(object) => {
            for (key, value) in object {
                writeln!(output, "- {key}: {}", compact_value(value)).unwrap();
            }
        }
        _ => {
            writeln!(output, "- {}", compact_value(value)).unwrap();
        }
    }
}

fn compact_value(value: &Value) -> String {
    match value {
        Value::Null => "<none>".to_owned(),
        Value::String(value) => value.clone(),
        Value::Array(values) => {
            if values.is_empty() {
                "<none>".to_owned()
            } else if values.iter().all(Value::is_string) {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                serde_json::to_string(value).unwrap_or_else(|_| "<unrenderable>".to_owned())
            }
        }
        Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "<unrenderable>".to_owned())
        }
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
    }
}
