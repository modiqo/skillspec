use crate::error::Result;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write;

const GRAMMAR_MD: &str = include_str!("../../../../spec/grammar.md");
const SCHEMA_JSON: &str = include_str!("../../../../spec/skill.spec.schema.json");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GrammarView {
    Index,
    Summary,
    Porting,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistSubject {
    ImportSkill,
}

#[derive(Clone, Debug, Serialize)]
pub struct GrammarSenseReport {
    pub view: GrammarView,
    pub embedded_sources: EmbeddedSources,
    pub sections: Vec<GrammarSection>,
    pub progressive_sequence: Vec<CommandStep>,
    pub prose_mappings: Vec<ProseMapping>,
    pub coverage_checklist: Vec<ChecklistItem>,
    pub anti_patterns: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EmbeddedSources {
    pub grammar_markdown_bytes: usize,
    pub schema_json_bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct GrammarSection {
    pub name: &'static str,
    pub role: &'static str,
    pub use_for: &'static str,
    pub query_handle: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct CommandStep {
    pub phase: &'static str,
    pub command: &'static str,
    pub proves: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProseMapping {
    pub prose_signal: &'static str,
    pub skillspec_construct: &'static str,
    pub extraction_question: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChecklistItem {
    pub id: &'static str,
    pub prompt: &'static str,
    pub evidence: &'static str,
    pub status_values: &'static [&'static str],
}

#[derive(Clone, Debug, Serialize)]
pub struct GrammarChecklistReport {
    pub subject: ChecklistSubject,
    pub command_sequence: Vec<CommandStep>,
    pub coverage_matrix_columns: Vec<&'static str>,
    pub checklist: Vec<ChecklistItem>,
    pub grading: Vec<ChecklistItem>,
}

pub fn sensemake(view: GrammarView) -> GrammarSenseReport {
    GrammarSenseReport {
        view,
        embedded_sources: EmbeddedSources {
            grammar_markdown_bytes: GRAMMAR_MD.len(),
            schema_json_bytes: SCHEMA_JSON.len(),
        },
        sections: grammar_sections(),
        progressive_sequence: progressive_sequence(),
        prose_mappings: prose_mappings(),
        coverage_checklist: import_skill_checklist(),
        anti_patterns: anti_patterns(),
    }
}

pub fn checklist(subject: ChecklistSubject) -> GrammarChecklistReport {
    GrammarChecklistReport {
        subject,
        command_sequence: progressive_sequence(),
        coverage_matrix_columns: vec![
            "prose_span",
            "obligation",
            "skillspec_construct",
            "confidence",
            "status",
            "review_note",
        ],
        checklist: import_skill_checklist(),
        grading: quality_grades(),
    }
}

pub fn schema_json() -> Result<Value> {
    Ok(serde_json::from_str(SCHEMA_JSON)?)
}

pub fn render_schema_summary() -> String {
    format!(
        "SkillSpec embedded JSON schema\nbytes: {}\n\nUse `skillspec grammar schema --json` to print the schema.",
        SCHEMA_JSON.len()
    )
}

pub fn render_sensemake(report: &GrammarSenseReport) -> String {
    let mut output = String::new();
    writeln!(output, "SkillSpec grammar map").unwrap();
    writeln!(
        output,
        "embedded: grammar.md {} bytes, skill.spec.schema.json {} bytes",
        report.embedded_sources.grammar_markdown_bytes, report.embedded_sources.schema_json_bytes
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(output, "Grammar affordances:").unwrap();
    for section in &report.sections {
        writeln!(
            output,
            "- {}: {}. Use for {}.",
            section.name, section.role, section.use_for
        )
        .unwrap();
    }
    writeln!(output).unwrap();
    writeln!(output, "Progressive command sequence:").unwrap();
    for (index, step) in visible_sequence(report.view, &report.progressive_sequence)
        .iter()
        .enumerate()
    {
        writeln!(output, "{}. {}: `{}`", index + 1, step.phase, step.command).unwrap();
        if matches!(
            report.view,
            GrammarView::Summary | GrammarView::Porting | GrammarView::Full
        ) {
            writeln!(output, "   proves: {}", step.proves).unwrap();
        }
    }

    if matches!(
        report.view,
        GrammarView::Summary | GrammarView::Porting | GrammarView::Full
    ) {
        writeln!(output).unwrap();
        writeln!(output, "Prose-to-SkillSpec mappings:").unwrap();
        for mapping in &report.prose_mappings {
            writeln!(
                output,
                "- {} -> {}",
                mapping.prose_signal, mapping.skillspec_construct
            )
            .unwrap();
            if matches!(report.view, GrammarView::Porting | GrammarView::Full) {
                writeln!(output, "  ask: {}", mapping.extraction_question).unwrap();
            }
        }
    }

    if matches!(report.view, GrammarView::Porting | GrammarView::Full) {
        writeln!(output).unwrap();
        writeln!(output, "Import coverage checklist:").unwrap();
        for item in &report.coverage_checklist {
            writeln!(output, "- {}: {}", item.id, item.prompt).unwrap();
            writeln!(output, "  evidence: {}", item.evidence).unwrap();
        }
        writeln!(output).unwrap();
        writeln!(output, "Coverage matrix:").unwrap();
        writeln!(
            output,
            "  prose_span | obligation | skillspec_construct | confidence | status | review_note"
        )
        .unwrap();
    }

    if matches!(report.view, GrammarView::Full) {
        writeln!(output).unwrap();
        writeln!(output, "Anti-patterns:").unwrap();
        for anti_pattern in &report.anti_patterns {
            writeln!(output, "- {anti_pattern}").unwrap();
        }
    }

    writeln!(output).unwrap();
    writeln!(output, "Escalation: index -> summary -> porting -> full.").unwrap();
    output
}

pub fn render_checklist(report: &GrammarChecklistReport) -> String {
    let mut output = String::new();
    writeln!(output, "SkillSpec porting checklist: import-skill").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "Command sequence:").unwrap();
    for (index, step) in report.command_sequence.iter().enumerate() {
        writeln!(output, "{}. {}: `{}`", index + 1, step.phase, step.command).unwrap();
        writeln!(output, "   proves: {}", step.proves).unwrap();
    }
    writeln!(output).unwrap();
    writeln!(
        output,
        "Coverage matrix columns: {}",
        report.coverage_matrix_columns.join(" | ")
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(output, "Required checks:").unwrap();
    for item in &report.checklist {
        writeln!(output, "- {}: {}", item.id, item.prompt).unwrap();
        writeln!(output, "  evidence: {}", item.evidence).unwrap();
        writeln!(output, "  status: {}", item.status_values.join(" / ")).unwrap();
    }
    writeln!(output).unwrap();
    writeln!(output, "Contract quality grades:").unwrap();
    for item in &report.grading {
        writeln!(output, "- {}: {}", item.id, item.prompt).unwrap();
        writeln!(output, "  evidence: {}", item.evidence).unwrap();
    }
    output
}

fn visible_sequence(view: GrammarView, sequence: &[CommandStep]) -> &[CommandStep] {
    match view {
        GrammarView::Index => &sequence[..3],
        GrammarView::Summary => &sequence[..5],
        GrammarView::Porting | GrammarView::Full => sequence,
    }
}

fn grammar_sections() -> Vec<GrammarSection> {
    vec![
        GrammarSection {
            name: "activation",
            role: "trigger surface before the full skill loads",
            use_for: "frontmatter descriptions, trigger phrases, and priority",
            query_handle: "activation",
        },
        GrammarSection {
            name: "routes",
            role: "strategy choices",
            use_for: "major task modes, handoffs, and execution plans",
            query_handle: "routes or route:<id>",
        },
        GrammarSection {
            name: "rules",
            role: "steering logic",
            use_for: "preferences, forbids, elicitations, and after-success obligations",
            query_handle: "rules or rule:<id>",
        },
        GrammarSection {
            name: "elicitations",
            role: "bounded user questions",
            use_for: "approval, install scope, auth, location, or mode choices",
            query_handle: "elicitations",
        },
        GrammarSection {
            name: "dependencies",
            role: "static substrate requirements",
            use_for: "tools, files, env vars, packages, services, adapters, and browsers",
            query_handle: "dependencies",
        },
        GrammarSection {
            name: "imports",
            role: "runtime-loadable guidance",
            use_for: "policy, reference, procedure, examples, and on-demand skill material",
            query_handle: "imports",
        },
        GrammarSection {
            name: "resources",
            role: "provenance and supporting files",
            use_for: "source material, scripts, assets, examples, and non-runtime evidence",
            query_handle: "resources",
        },
        GrammarSection {
            name: "commands",
            role: "executable templates",
            use_for: "safe command surfaces with declared dependency checks",
            query_handle: "commands or command:<id>.requires",
        },
        GrammarSection {
            name: "recipes",
            role: "ordered procedures",
            use_for: "load, ask, run, branch, produce, and consume steps",
            query_handle: "recipes or recipe:<id>",
        },
        GrammarSection {
            name: "states",
            role: "lifecycle phases",
            use_for: "agent-visible start, ask, next, yes/no, and done flow",
            query_handle: "states or state:<id>",
        },
        GrammarSection {
            name: "closures",
            role: "completion obligations",
            use_for: "named after-success work and execution-proof targets",
            query_handle: "closures",
        },
        GrammarSection {
            name: "tests",
            role: "scenario expectations",
            use_for: "routing, matched rules, forbids, elicitations, and closures",
            query_handle: "tests",
        },
        GrammarSection {
            name: "trace",
            role: "proof and alignment contract",
            use_for: "decision traces and execution evidence alignment",
            query_handle: "trace",
        },
        GrammarSection {
            name: "tool_boundary",
            role: "phase-scoped tool and substrate permission boundary",
            use_for:
                "default deny, explicit allow lists, forbids, and permission-required surfaces",
            query_handle:
                "entry.tool_boundary, route:<id>.tool_boundary, route:<id>.execution_plan",
        },
    ]
}

fn progressive_sequence() -> Vec<CommandStep> {
    vec![
        CommandStep {
            phase: "learn grammar affordances",
            command: "skillspec grammar sensemake --view index",
            proves: "the harness knows the available grammar sections without loading source code",
        },
        CommandStep {
            phase: "expand for porting",
            command: "skillspec grammar sensemake --view porting",
            proves: "the harness sees prose-to-construct mappings and the coverage matrix",
        },
        CommandStep {
            phase: "map source package",
            command: "skillspec source map <source-skill> --out <draft>/.skillspec/source-map",
            proves: "Markdown files, frontmatter, byte ranges, line ranges, references, code blocks, dependencies, and review-required spans were indexed without loading the full source into model context",
        },
        CommandStep {
            phase: "inspect source map",
            command: "skillspec source query <draft>/.skillspec/source-map/source-map.json nodes --view index",
            proves: "the harness sees the source structure and exact handles before opening detailed spans",
        },
        CommandStep {
            phase: "review source obligations",
            command: "skillspec source query <draft>/.skillspec/source-map/source-map.json dependencies --view summary",
            proves: "dependency mentions and imported package signals are visible before proof or install",
        },
        CommandStep {
            phase: "check source freshness",
            command: "skillspec source stale <draft>/.skillspec/source-map/source-map.json --root <source-skill>",
            proves: "the source map still matches the staged source before mechanical import",
        },
        CommandStep {
            phase: "create mechanical draft",
            command: "skillspec import-skill <source-skill> --out <draft>/skill.spec.yml --source-map <draft>/.skillspec/source-map/source-map.json",
            proves: "frontmatter, headings, materialized code resources, imports, deps, and review notes were extracted from the fresh mapped source",
        },
        CommandStep {
            phase: "sensemake the draft",
            command: "skillspec sensemake <draft>/skill.spec.yml --view index",
            proves: "the draft's actual routes, rules, deps, commands, imports, and tests are visible",
        },
        CommandStep {
            phase: "inspect dependency ledger",
            command: "sed -n '1,240p' <draft>/deps.toml",
            proves: "the generated dependency ledger exists, is not byte-empty, and is ready for dependency authority review; dependency_count = 0 is valid when no dependencies exist",
        },
        CommandStep {
            phase: "apply import checklist",
            command: "skillspec grammar checklist --for import-skill",
            proves: "the harness has a coverage matrix and quality grades to fill before install",
        },
        CommandStep {
            phase: "validate structure",
            command: "skillspec validate <draft>/skill.spec.yml",
            proves: "the generated contract is parseable and reference-connected",
        },
        CommandStep {
            phase: "check imports",
            command: "skillspec imports check <draft>/skill.spec.yml",
            proves: "runtime guidance paths are package-local and load order is valid",
        },
        CommandStep {
            phase: "check dependencies",
            command: "skillspec deps check <draft>/skill.spec.yml",
            proves: "declared tools, files, env, services, adapters, and browsers are checkable or explicitly deferred",
        },
        CommandStep {
            phase: "run scenario tests",
            command: "skillspec test <draft>/skill.spec.yml",
            proves: "route, rule, forbid, elicitation, and closure expectations are behavior-tested",
        },
        CommandStep {
            phase: "run demo decision",
            command: "skillspec decide <draft>/skill.spec.yml --input '<realistic task>' --trace-dir <draft>/.skillspec/traces",
            proves: "the reviewed spec routes a realistic user task",
        },
        CommandStep {
            phase: "align proof",
            command: "skillspec trace align <draft>/skill.spec.yml --decision-trace <run_dir> --summary",
            proves: "decision replay is stable and remaining execution evidence gaps are explicit",
        },
    ]
}

fn prose_mappings() -> Vec<ProseMapping> {
    vec![
        ProseMapping {
            prose_signal: "frontmatter name/description and trigger language",
            skillspec_construct: "activation, applies_when",
            extraction_question:
                "What should make the harness select this skill before loading body text?",
        },
        ProseMapping {
            prose_signal: "major headings that describe different task modes",
            skillspec_construct: "routes",
            extraction_question: "Is this a strategy choice, or only documentation under a route?",
        },
        ProseMapping {
            prose_signal: "must, never, prefer, avoid, only, ask before",
            skillspec_construct: "rules.forbid, rules.prefer, rules.elicit, rules.after_success",
            extraction_question: "What obligation should constrain the next action?",
        },
        ProseMapping {
            prose_signal: "questions, approvals, install scope, auth, locations",
            skillspec_construct: "elicitations",
            extraction_question: "What bounded user choice must be answered before side effects?",
        },
        ProseMapping {
            prose_signal: "tools, scripts, packages, env vars, services, adapters, browser state",
            skillspec_construct: "dependencies, tool_boundary",
            extraction_question:
                "How can readiness be checked, and which tools or substrates are permitted before asking for permission?",
        },
        ProseMapping {
            prose_signal: "linked guidance that should be loaded only when needed",
            skillspec_construct: "imports",
            extraction_question: "When should this guidance enter the prompt path?",
        },
        ProseMapping {
            prose_signal: "source files, assets, examples, scripts, provenance",
            skillspec_construct: "resources",
            extraction_question: "Is this runtime guidance, or supporting/provenance material?",
        },
        ProseMapping {
            prose_signal: "fenced shell snippets or repeatable commands",
            skillspec_construct: "commands, code, recipes",
            extraction_question: "Is this safe to run, a template to check, or just an example?",
        },
        ProseMapping {
            prose_signal: "step-by-step lifecycle",
            skillspec_construct: "states, recipes, execution_plan",
            extraction_question:
                "Which steps are mandatory order, lifecycle navigation, or route-local execution?",
        },
        ProseMapping {
            prose_signal: "done criteria, report requirements, cleanup, validation",
            skillspec_construct: "closures, tests, trace",
            extraction_question: "What must be proven before final response or release?",
        },
    ]
}

fn import_skill_checklist() -> Vec<ChecklistItem> {
    vec![
        ChecklistItem {
            id: "activation",
            prompt: "Extract the trigger surface from frontmatter and first-order prose.",
            evidence: "activation.summary, activation.keywords, applies_when",
            status_values: &["strong", "partial", "missing", "review_required"],
        },
        ChecklistItem {
            id: "routes",
            prompt: "Identify major task modes instead of promoting every heading.",
            evidence: "routes with rank, descriptions, handoffs, or execution plans",
            status_values: &["good", "thin", "missing", "not_applicable"],
        },
        ChecklistItem {
            id: "rules",
            prompt: "Turn hard obligations, preferences, forbids, asks, and completion requirements into rules.",
            evidence: "rules with when/prefer/forbid/elicit/after_success/reason",
            status_values: &["good", "partial", "missing", "review_required"],
        },
        ChecklistItem {
            id: "elicitations",
            prompt: "Model user approvals and bounded choices before side effects. Quote question and note strings that contain colon-space or other YAML-sensitive punctuation.",
            evidence: "elicitations referenced by rules, recipes, or states, with YAML-safe quoted question and note strings when needed",
            status_values: &["good", "partial", "missing", "not_applicable"],
        },
        ChecklistItem {
            id: "artifact_dataflow",
            prompt: "Connect artifacts to executable producers and consumers only; routes and rules are control-flow, not artifact consumers.",
            evidence: "artifacts.produced_by and artifacts.consumed_by use kind=command, kind=code, or kind=recipe only",
            status_values: &["valid", "empty", "invalid_ref", "missing"],
        },
        ChecklistItem {
            id: "imports_resources",
            prompt: "Move on-demand guidance into imports and provenance/assets/scripts/code files into resources.",
            evidence: "package-local import/resource paths, code.source.file entries, and used_by or load=always",
            status_values: &["strong", "partial", "orphaned", "missing"],
        },
        ChecklistItem {
            id: "commands_deps",
            prompt: "Declare executable templates, checkable dependencies, and phase tool boundaries without pretending install permission exists.",
            evidence: "commands.requires, tool_boundary, deps.toml review, plus deps check output",
            status_values: &["present", "deferred", "missing", "unsafe"],
        },
        ChecklistItem {
            id: "dependency_ledger",
            prompt: "Inspect and complete the scaffolded deps.toml before proof or install; zero dependency entries are allowed only when dependency_count = 0 is recorded.",
            evidence: "deps.toml with schema_version, review_required, dependency_count, source authority, local_status, install risk, and degraded proof impact",
            status_values: &["reviewed", "zero_entries", "incomplete", "missing"],
        },
        ChecklistItem {
            id: "procedures",
            prompt: "Represent ordered prose as recipes, states, or route execution plans.",
            evidence: "recipes/states/execution_plan handles",
            status_values: &["good", "partial", "missing", "not_applicable"],
        },
        ChecklistItem {
            id: "tests",
            prompt: "Add scenario tests that prove route, rules, forbids, elicitations, and closures.",
            evidence: "skillspec test output and test expectations",
            status_values: &["reviewed", "generated", "missing", "failing"],
        },
        ChecklistItem {
            id: "proof",
            prompt: "Run a realistic decision and align the trace, reporting unproven execution gaps honestly.",
            evidence: "decision trace path and trace align status",
            status_values: &["pass", "unproven", "fail", "not_run"],
        },
    ]
}

fn quality_grades() -> Vec<ChecklistItem> {
    vec![
        ChecklistItem {
            id: "contract_quality.activation",
            prompt: "Grade trigger precision and risk of false activation.",
            evidence: "activation summary, keywords, applies_when, scenario tests",
            status_values: &["strong", "good", "partial", "weak"],
        },
        ChecklistItem {
            id: "contract_quality.dependencies",
            prompt:
                "Grade whether dependencies are checkable and tool boundaries are permission-aware.",
            evidence: "deps check output, tool_boundary, and provision/permission fields",
            status_values: &["strong", "partial", "missing", "unsafe"],
        },
        ChecklistItem {
            id: "contract_quality.route_coverage",
            prompt:
                "Grade whether major prose workflows map to routes without overfitting headings.",
            evidence: "sensemake route count, route descriptions, tests",
            status_values: &["good", "thin", "overfit", "missing"],
        },
        ChecklistItem {
            id: "contract_quality.execution_evidence",
            prompt: "Grade whether decision and execution obligations have proof.",
            evidence: "trace align decision and execution-proof layers",
            status_values: &["pass", "unproven", "fail"],
        },
        ChecklistItem {
            id: "contract_quality.hallucination_risk",
            prompt: "Grade how much was inferred beyond explicit prose evidence.",
            evidence: "coverage matrix confidence and review notes",
            status_values: &["low", "medium", "high"],
        },
    ]
}

fn anti_patterns() -> Vec<&'static str> {
    vec![
        "Do not turn every Markdown heading into a route.",
        "Do not treat imported Markdown as if it creates routes, rules, or tests automatically.",
        "Do not classify bundled scripts as PATH dependencies when they are package-local files.",
        "Do not treat a byte-empty deps.toml as a valid zero-dependency ledger; use dependency_count = 0 with schema/review metadata.",
        "Do not mark execution proof as pass without an execution ledger or closure evidence.",
        "Do not hide review_required fields; they are the honesty layer of a port.",
        "Do not install a generated skill until validation, imports, deps, tests, and a demo decision have run.",
    ]
}
