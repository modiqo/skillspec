mod act;
mod align;
mod capability;
mod compiler;
mod decision;
mod deps;
mod error;
mod grammar;
mod importer;
mod imports;
mod install;
mod model;
mod parser;
mod progress;
mod report;
mod router;
mod router_lifecycle;
mod sensemake;
mod trace;
mod visibility;
mod workspace_synthesizer;

use clap::{Parser, Subcommand, ValueEnum};
use error::Result;
use install::HarnessTarget;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "skillspec")]
#[command(about = "Structured skills for agent behavior")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    #[command(about = "Validate a skill.spec.yml file")]
    Validate {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
    },
    #[command(about = "Run scenario tests declared in a SkillSpec")]
    Test {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
    },
    #[command(about = "Evaluate routing rules for a user task and emit JSON")]
    Decide {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// User task text to route. Strip skill invocation prefixes before passing it.
        #[arg(long, allow_hyphen_values = true)]
        input: String,
        /// Directory where append-only decision trace events should be written.
        #[arg(long)]
        trace_dir: Option<PathBuf>,
    },
    #[command(about = "Turn a SkillSpec decision into a current-route action checklist")]
    Act {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// User task text to route. Strip skill invocation prefixes before passing it.
        #[arg(long, allow_hyphen_values = true)]
        input: String,
        /// Directory where append-only decision trace events should be written.
        #[arg(long, conflicts_with = "run")]
        trace_dir: Option<PathBuf>,
        /// Existing trace run directory to associate with this action checklist.
        #[arg(long)]
        run: Option<PathBuf>,
        /// Expand this execution phase instead of the first pending phase.
        #[arg(long)]
        phase: Option<String>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "List selected-route execution phases in order")]
    Plan {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// User task text to route. Strip skill invocation prefixes before passing it.
        #[arg(long, allow_hyphen_values = true)]
        input: String,
        /// Directory where append-only decision trace events should be written.
        #[arg(long)]
        trace_dir: Option<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Explain routing decisions for a user task")]
    Explain {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// User task text to explain. Strip skill invocation prefixes before passing it.
        #[arg(long, allow_hyphen_values = true)]
        input: String,
        /// Directory where append-only decision trace events should be written.
        #[arg(long)]
        trace_dir: Option<PathBuf>,
    },
    #[command(about = "Teach one SkillSpec map and progressive navigation handles")]
    Sensemake {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = SenseViewArg::Index)]
        view: SenseViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Query one SkillSpec collection, item, or field path")]
    Query {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Query handle, such as routes, rule:<id>, or command:<id>.requires.
        handle: String,
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = SenseViewArg::Summary)]
        view: SenseViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show outgoing SkillSpec references for an item handle")]
    Refs {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Item handle, such as rule:<id>, command:<id>, state:<id>, or recipe:<id>.
        handle: String,
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = SenseViewArg::Summary)]
        view: SenseViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Teach the embedded SkillSpec grammar and porting coverage workflow")]
    Grammar {
        #[command(subcommand)]
        command: GrammarCommand,
    },
    #[command(about = "Inspect, compact, or align SkillSpec decision traces")]
    Trace {
        #[command(subcommand)]
        command: TraceCommand,
    },
    #[command(about = "Show or record SkillSpec execution progress for a trace run")]
    Progress {
        #[command(subcommand)]
        command: ProgressCommand,
    },
    #[command(about = "Check declared SkillSpec dependencies")]
    Deps {
        #[command(subcommand)]
        command: DepsCommand,
    },
    #[command(about = "Validate and report SkillSpec imports")]
    Imports {
        #[command(subcommand)]
        command: ImportsCommand,
    },
    #[command(about = "Compile a SkillSpec into harness guidance")]
    Compile {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Output target to render.
        #[arg(long)]
        target: CompileTarget,
    },
    #[command(about = "Create a mechanical draft SkillSpec from a local skill file or folder")]
    ImportSkill {
        /// Local SKILL.md file or skill folder to import.
        path: PathBuf,
        /// Output path for the generated skill.spec.yml draft.
        #[arg(long)]
        out: PathBuf,
    },
    #[command(
        about = "Synthesize a draft SkillSpec from a durable rote workspace (rote-specific)",
        long_about = "Synthesize a draft SkillSpec from rote-specific durable execution evidence. This optional integration requires a rote workspace name and validates `rote workspace stats`, `rote workspace inspect log`, and `rote workspace inspect meta` evidence before writing a scaffold."
    )]
    SynthesizeFromWorkspace {
        /// Durable rote workspace name that was created by durable execution.
        workspace: String,
        /// Output skill folder to create. The command writes skill.spec.yml and resources/observed-workspace/.
        #[arg(long)]
        out: PathBuf,
        /// Original user task that created the durable workspace.
        #[arg(long, allow_hyphen_values = true)]
        task: Option<String>,
        /// Skill id/name to use for the generated scaffold.
        #[arg(long)]
        name: Option<String>,
        /// Number of command-log rows to collect when --workspace-log is omitted.
        #[arg(long, default_value_t = 50)]
        log_last: usize,
        /// Pre-captured report from `rote workspace stats <workspace>`.
        #[arg(long)]
        workspace_stats_report: Option<PathBuf>,
        /// Pre-captured command log from `rote workspace inspect log --last <n>`.
        #[arg(long)]
        workspace_log: Option<PathBuf>,
        /// Pre-captured metadata from `rote workspace inspect meta`.
        #[arg(long)]
        workspace_meta: Option<PathBuf>,
        /// Optional pre-captured dependency graph from `rote workspace inspect deps`.
        #[arg(long)]
        workspace_deps: Option<PathBuf>,
        /// Overwrite an existing skill.spec.yml in the output folder.
        #[arg(long)]
        force: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Build a searchable skill catalog outside model context")]
    Index {
        /// Skill roots to scan. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// SQLite index file to write, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        out: PathBuf,
        /// Visibility manifest whose final states should override native metadata.
        #[arg(long)]
        visibility_manifest: Option<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Route a user request to candidate skills from an index")]
    Route {
        /// SQLite index file created by `skillspec index`, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// User task text to route.
        #[arg(long, allow_hyphen_values = true)]
        query: String,
        /// Number of candidates to return.
        #[arg(long, default_value_t = 5)]
        top: usize,
        /// Execution mode already selected by user or caller.
        #[arg(long, value_enum)]
        execution_mode: Option<RouterExecutionModeArg>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Inspect or tune installed skills")]
    Skills {
        #[command(subcommand)]
        command: SkillsCommand,
    },
    #[command(about = "Plan, apply, or restore harness-native skill visibility controls")]
    Visibility {
        #[command(subcommand)]
        command: VisibilityCommand,
    },
    #[command(about = "Install, uninstall, inspect, or repair the optional skill router")]
    Router {
        #[command(subcommand)]
        command: RouterCommand,
    },
    #[command(about = "Detect harness roots and install SkillSpec-backed skills")]
    Install {
        #[command(subcommand)]
        command: InstallCommand,
    },
    #[command(about = "Manage local capability seeds for durable bootstrap")]
    Capability {
        #[command(subcommand)]
        command: CapabilityCommand,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum CompileTarget {
    CodexSkill,
    ClaudeSkill,
    Markdown,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum RouterExecutionModeArg {
    Direct,
    Durable,
}

#[derive(Debug, Subcommand)]
enum SkillsCommand {
    #[command(about = "Audit skill routing metadata and context-budget risk")]
    Audit {
        /// Skill roots to scan. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Set one skill's native visibility state")]
    SetVisibility {
        /// Skill name from SKILL.md frontmatter.
        skill: String,
        /// Target visibility state.
        visibility: VisibilityArg,
        /// Skill roots to search. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// Reversible manifest path to write.
        #[arg(long)]
        manifest: PathBuf,
        /// Show changes without writing files or manifest.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Mark one skill off for native discovery and router routing")]
    Disable {
        /// Skill name from SKILL.md frontmatter.
        skill: String,
        /// Skill roots to search. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// Reversible manifest path to write.
        #[arg(long)]
        manifest: PathBuf,
        /// Show changes without writing files or manifest.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Mark one skill implicit/on for native discovery")]
    Enable {
        /// Skill name from SKILL.md frontmatter.
        skill: String,
        /// Skill roots to search. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// Reversible manifest path to write.
        #[arg(long)]
        manifest: PathBuf,
        /// Show changes without writing files or manifest.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum VisibilityCommand {
    #[command(about = "Show router-managed visibility changes without editing files")]
    Plan {
        /// Skill roots to scan. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// Visibility profile to apply.
        #[arg(long, value_enum, default_value_t = VisibilityProfileArg::RouterManaged)]
        profile: VisibilityProfileArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Apply harness-native visibility controls and write a manifest")]
    Apply {
        /// Skill roots to scan. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// Visibility profile to apply.
        #[arg(long, value_enum, default_value_t = VisibilityProfileArg::RouterManaged)]
        profile: VisibilityProfileArg,
        /// Reversible manifest path to write.
        #[arg(long)]
        manifest: PathBuf,
        /// Show changes without writing files or manifest.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Restore visibility files from a prior manifest")]
    Restore {
        /// Reversible manifest path produced by visibility apply or skills set-visibility.
        #[arg(long)]
        manifest: PathBuf,
        /// Show restore changes without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum VisibilityArg {
    Implicit,
    ManualOnly,
    NameOnly,
    Off,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum VisibilityProfileArg {
    RouterManaged,
}

#[derive(Debug, Subcommand)]
enum RouterCommand {
    #[command(
        about = "Install the explicit-only SkillSpec-backed skill-router, managed index, and preparedness check"
    )]
    Install {
        /// Skill roots to scan and manage.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// SQLite index file to write, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Reversible visibility manifest path. Defaults beside the index.
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Router skill folder/name.
        #[arg(long, default_value = router_lifecycle::default_router_name())]
        router_name: String,
        /// Show changes without writing files, index, manifest, or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Restore visibility and remove the managed skill-router")]
    Uninstall {
        /// Reversible visibility manifest path. Defaults from router config.
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Router skill folder/name.
        #[arg(long, default_value = router_lifecycle::default_router_name())]
        router_name: String,
        /// SQLite index file or router directory to remove unless --keep-index is set. Defaults from router config.
        #[arg(long)]
        index: Option<PathBuf>,
        /// Preserve the index file.
        #[arg(long)]
        keep_index: bool,
        /// Show changes without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Detect, repair, or inspect router index drift")]
    Index {
        #[command(subcommand)]
        command: RouterIndexCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RouterIndexCommand {
    #[command(about = "Repair router visibility and rebuild the index from current skill roots")]
    Refresh {
        /// Skill roots to scan. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// SQLite index file to write, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Visibility manifest whose final states should override native metadata. Defaults from router config when installed.
        #[arg(long)]
        visibility_manifest: Option<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Detect out-of-band prose or SkillSpec-backed skill changes")]
    Status {
        /// Skill roots to scan. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// SQLite index file to inspect, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Visibility manifest whose final states should override native metadata.
        #[arg(long)]
        visibility_manifest: Option<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum GrammarCommand {
    #[command(about = "Explain embedded grammar affordances progressively")]
    Sensemake {
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = GrammarViewArg::Index)]
        view: GrammarViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show a coverage checklist for semantic skill porting")]
    Checklist {
        /// Checklist workflow to show.
        #[arg(long = "for", value_enum, default_value_t = GrammarChecklistForArg::ImportSkill)]
        for_subject: GrammarChecklistForArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Print or describe the embedded SkillSpec JSON schema")]
    Schema {
        /// Emit the embedded JSON schema.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum TraceCommand {
    #[command(about = "Compact append-only trace events from a run directory into JSON")]
    Compact {
        /// Trace run directory produced by decide/explain --trace-dir.
        run_dir: PathBuf,
    },
    #[command(about = "Compare a SkillSpec to a decision trace")]
    Align {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Trace run directory produced by decide/explain --trace-dir.
        #[arg(long)]
        decision_trace: PathBuf,
        /// JSONL execution ledger with sanitized action evidence. Repeat for multiple ledgers.
        #[arg(long)]
        execution_trace: Vec<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ProgressCommand {
    #[command(about = "Show completed, current, blocked, and remaining phases")]
    Show {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Trace run directory produced by plan/decide/explain --trace-dir.
        #[arg(long)]
        run: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Append one structured execution/progress event to a run ledger")]
    Record {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// Event type to append.
        #[arg(value_enum)]
        event: ProgressEventArg,
        /// Phase id for phase or requirement events.
        phase: Option<String>,
        /// Requirement id for requirement events.
        requirement: Option<String>,
        /// Obligation, route, closure, or elicitation id for proof events.
        #[arg(long)]
        id: Option<String>,
        /// Event status, such as pass, fail, blocked, or pending.
        #[arg(long)]
        status: Option<String>,
        /// Evidence kind, such as rote_response, file, trace, or command.
        #[arg(long)]
        evidence_kind: Option<String>,
        /// Evidence reference, such as @7 or a relative file path.
        #[arg(long)]
        evidence_ref: Option<String>,
        /// Skill that emitted this progress event.
        #[arg(long)]
        source_skill: Option<String>,
        /// Human-readable event note.
        #[arg(long)]
        message: Option<String>,
        /// Emit JSON for the appended event.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Append a stats_collected token/workspace metrics event to a run ledger")]
    Stats {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// Rote workspace name.
        #[arg(long)]
        workspace: Option<String>,
        /// Phase id whose requirement(s) this stats event satisfies.
        #[arg(long)]
        phase: Option<String>,
        /// Requirement id satisfied by this stats event. Repeat for multiple requirements.
        #[arg(long)]
        requirement: Vec<String>,
        /// JSON file produced by `rote workspace stats <workspace> --json`.
        #[arg(long)]
        workspace_stats_json: Option<PathBuf>,
        /// Human-readable report produced by `rote workspace stats <workspace>`.
        #[arg(long)]
        workspace_stats_report: Option<PathBuf>,
        /// Total API request+response tokens.
        #[arg(long)]
        total_tokens: Option<u64>,
        /// One-time context-window tokens consumed during exploration.
        #[arg(long)]
        context_tokens: Option<u64>,
        /// Tokens in extracted query results.
        #[arg(long)]
        query_result_tokens: Option<u64>,
        /// Cached response/source tokens before query reduction.
        #[arg(long)]
        response_tokens_cached: Option<u64>,
        /// Tokens saved by query reduction or cache reuse.
        #[arg(long)]
        saved_tokens: Option<u64>,
        /// Percent reduction from cached/source tokens to query-result tokens.
        #[arg(long)]
        reduction_percent: Option<f64>,
        /// Human-readable event note.
        #[arg(long)]
        message: Option<String>,
        /// Emit JSON for the appended event.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Append final_response_sent report-section proof to a run ledger")]
    FinalResponse {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// Phase id whose requirement(s) this final response event satisfies.
        #[arg(long)]
        phase: Option<String>,
        /// Requirement id satisfied by this final response event. Repeat for multiple requirements.
        #[arg(long)]
        requirement: Vec<String>,
        /// Final response includes the direct result.
        #[arg(long)]
        result: bool,
        /// Final response includes evidence handles or files.
        #[arg(long)]
        evidence: bool,
        /// Final response includes the alignment summary.
        #[arg(long)]
        alignment: bool,
        /// Final response includes token usage and token savings.
        #[arg(long)]
        token_savings: bool,
        /// Human-readable event note.
        #[arg(long)]
        message: Option<String>,
        /// Emit JSON for the appended event.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum DepsCommand {
    #[command(about = "Check declared dependencies, optionally scoped to one command template")]
    Check {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Check only dependencies required by this command id.
        #[arg(long)]
        command: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ImportsCommand {
    #[command(about = "Check import paths, sections, nesting, and load order")]
    Check {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum InstallCommand {
    #[command(about = "List detected harness skill roots")]
    Targets,
    #[command(about = "Install a folder containing SKILL.md and skill.spec.yml")]
    Skill {
        /// Generated skill folder containing SKILL.md and skill.spec.yml.
        folder: PathBuf,
        /// Harness target to install into. Repeat for multiple targets.
        #[arg(long, value_enum)]
        target: Vec<InstallTargetArg>,
        /// Install into every harness root detected on this machine.
        #[arg(long)]
        all_detected: bool,
        /// Show the install plan without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Overwrite an existing installed skill folder without prompting.
        #[arg(long)]
        force: bool,
        /// Override the installed skill folder name.
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum CapabilityCommand {
    #[command(about = "Show the local capability seed store path")]
    Store,
    #[command(about = "Create or update a local capability seed")]
    Add {
        /// Stable seed id, such as preferred-voice-cli.
        id: String,
        /// Capability domain folder, such as voice or pdf.
        #[arg(long)]
        domain: String,
        /// Seed kind, such as cli, adapter, script, or flow.
        #[arg(long)]
        kind: String,
        /// CLI command name or path.
        #[arg(long)]
        command: Option<String>,
        /// Adapter id or name.
        #[arg(long)]
        adapter: Option<String>,
        /// Local script path.
        #[arg(long)]
        script: Option<String>,
        /// Capability provided by this seed. Repeat for multiple capabilities.
        #[arg(long)]
        provides: Vec<String>,
        /// User phrase alias for this seed. Repeat for multiple aliases.
        #[arg(long)]
        alias: Vec<String>,
        /// Default priority from 0 to 100, used only as a tie-breaker.
        #[arg(long)]
        priority: Option<u8>,
        /// Capability this seed is preferred for. Repeat for multiple capabilities.
        #[arg(long)]
        preferred_for: Vec<String>,
        /// Capability this seed should avoid. Repeat for multiple capabilities.
        #[arg(long)]
        avoid_for: Vec<String>,
        /// Tie-breaker metadata as key=value. Repeat for multiple entries.
        #[arg(long = "tie")]
        ties: Vec<String>,
        /// Environment variable used for auth. Repeat for multiple vars.
        #[arg(long)]
        auth_env: Vec<String>,
        /// Mark this seed as using an external service.
        #[arg(long)]
        external_service: bool,
        /// Mark this seed as potentially spending provider credits or money.
        #[arg(long)]
        may_cost_money: bool,
        /// Evidence command, such as "tool --help". Repeat for multiple checks.
        #[arg(long)]
        evidence_command: Vec<String>,
        /// Suggested domain SkillSpec id to generate after a successful trace.
        #[arg(long)]
        suggested_skill_id: Option<String>,
    },
    #[command(
        about = "Patch an existing local capability seed without rewriting unspecified fields"
    )]
    Update {
        /// Seed id to update.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Replace seed kind.
        #[arg(long)]
        kind: Option<String>,
        /// Set CLI command name or path.
        #[arg(long)]
        command: Option<String>,
        /// Clear CLI command.
        #[arg(long)]
        clear_command: bool,
        /// Set adapter id or name.
        #[arg(long)]
        adapter: Option<String>,
        /// Clear adapter id or name.
        #[arg(long)]
        clear_adapter: bool,
        /// Set local script path.
        #[arg(long)]
        script: Option<String>,
        /// Clear local script path.
        #[arg(long)]
        clear_script: bool,
        /// Add a capability provided by this seed. Repeat for multiple capabilities.
        #[arg(long)]
        add_provides: Vec<String>,
        /// Remove a provided capability. Repeat for multiple capabilities.
        #[arg(long)]
        remove_provides: Vec<String>,
        /// Add a user phrase alias. Repeat for multiple aliases.
        #[arg(long)]
        add_alias: Vec<String>,
        /// Remove a user phrase alias. Repeat for multiple aliases.
        #[arg(long)]
        remove_alias: Vec<String>,
        /// Set default priority from 0 to 100.
        #[arg(long)]
        priority: Option<u8>,
        /// Clear default priority.
        #[arg(long)]
        clear_priority: bool,
        /// Add a preferred capability. Repeat for multiple capabilities.
        #[arg(long)]
        add_preferred_for: Vec<String>,
        /// Remove a preferred capability. Repeat for multiple capabilities.
        #[arg(long)]
        remove_preferred_for: Vec<String>,
        /// Add an avoided capability. Useful when a seed stops working for a task.
        #[arg(long)]
        add_avoid_for: Vec<String>,
        /// Remove an avoided capability.
        #[arg(long)]
        remove_avoid_for: Vec<String>,
        /// Add or replace tie-breaker metadata as key=value. Repeat for multiple entries.
        #[arg(long)]
        add_tie: Vec<String>,
        /// Remove tie-breaker metadata by key. Repeat for multiple entries.
        #[arg(long)]
        remove_tie: Vec<String>,
        /// Add an auth environment variable. Repeat for multiple vars.
        #[arg(long)]
        add_auth_env: Vec<String>,
        /// Remove an auth environment variable. Repeat for multiple vars.
        #[arg(long)]
        remove_auth_env: Vec<String>,
        /// Set external service risk flag.
        #[arg(long)]
        external_service: Option<bool>,
        /// Set provider cost risk flag.
        #[arg(long)]
        may_cost_money: Option<bool>,
        /// Add evidence command, such as "tool --help". Repeat for multiple checks.
        #[arg(long)]
        add_evidence_command: Vec<String>,
        /// Remove an evidence command. Repeat for multiple checks.
        #[arg(long)]
        remove_evidence_command: Vec<String>,
        /// Set suggested domain SkillSpec id to generate after a successful trace.
        #[arg(long)]
        suggested_skill_id: Option<String>,
        /// Clear suggested domain SkillSpec id.
        #[arg(long)]
        clear_suggested_skill_id: bool,
        /// Mark verification status unverified without running checks.
        #[arg(long, conflicts_with = "mark_failed")]
        mark_unverified: bool,
        /// Mark verification status failed without running checks.
        #[arg(long, conflicts_with = "mark_unverified")]
        mark_failed: bool,
    },
    #[command(about = "List local capability seeds")]
    List {
        /// Limit results to one domain.
        #[arg(long)]
        domain: Option<String>,
    },
    #[command(about = "Search and rank local capability seeds for one capability/domain pair")]
    Search {
        /// Capability to search for, such as text_to_speech.
        capability: String,
        /// Limit results to one domain. If no candidates are found, callers should search related domains before using an unseeded fallback.
        #[arg(long)]
        domain: Option<String>,
        /// Include ranking reasons in the JSON output.
        #[arg(long)]
        explain: bool,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
        /// Exclude external service candidates.
        #[arg(long)]
        local_only: bool,
        /// Explicitly preferred seed id for this search.
        #[arg(long)]
        preferred_seed: Option<String>,
    },
    #[command(about = "Inspect one local capability seed")]
    Inspect {
        /// Seed id to inspect.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Verify one local capability seed's evidence")]
    Verify {
        /// Seed id to verify.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Update preferred capability and priority metadata for a seed")]
    Prefer {
        /// Seed id to update.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Capability this seed should be preferred for.
        #[arg(long = "for")]
        for_capability: String,
        /// Default priority from 0 to 100.
        #[arg(long)]
        priority: Option<u8>,
    },
    #[command(about = "Remove one local capability seed")]
    Remove {
        /// Seed id to remove.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
    },
    #[command(about = "Scan for seed proposals")]
    Scan,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum InstallTargetArg {
    Agents,
    Codex,
    ClaudeLocal,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SenseViewArg {
    Index,
    Summary,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum GrammarViewArg {
    Index,
    Summary,
    Porting,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum GrammarChecklistForArg {
    ImportSkill,
}

#[derive(Clone, Debug, ValueEnum)]
enum ProgressEventArg {
    PhaseStarted,
    RequirementStarted,
    RequirementSatisfied,
    RequirementFailed,
    StatsCollected,
    ObligationSatisfied,
    RouteFulfilled,
    AfterSuccessCompleted,
    EvidenceAttached,
    HandoffStarted,
    HandoffCompleted,
    PhaseCompleted,
    PhaseBlocked,
}

fn main() {
    if let Err(error) = run() {
        report::error(error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate { path } => {
            let spec = parser::load_spec(&path)?;
            report::validation_ok(&path, &spec)?;
        }
        Command::Test { path } => {
            let spec = parser::load_spec(&path)?;
            let result = decision::run_tests(&spec);
            report::test_result(&result)?;
            if !result.failed.is_empty() {
                std::process::exit(1);
            }
        }
        Command::Decide {
            path,
            input,
            trace_dir,
        } => {
            let spec = parser::load_spec(&path)?;
            ensure_trace_available(&spec, trace_dir.as_ref())?;
            let decision = decision::decide_with_events(&spec, &input);
            if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
            }
            report::json(&decision.decision)?;
        }
        Command::Act {
            path,
            input,
            trace_dir,
            run,
            phase,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            ensure_trace_available(&spec, trace_dir.as_ref().or(run.as_ref()))?;
            let decision = decision::decide_with_events(&spec, &input);
            let trace = if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
                Some(trace)
            } else {
                None
            };
            let mut act_report = act::build_report_for_phase(
                &spec,
                &decision.decision,
                trace.as_ref(),
                phase.as_deref(),
            )?;
            if let Some(run) = run {
                act_report.trace = Some(act::trace_for_run(&run));
            }
            if json {
                report::json(&act_report)?;
            } else {
                report::text(&act::render(&act_report))?;
            }
        }
        Command::Plan {
            path,
            input,
            trace_dir,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            ensure_trace_available(&spec, trace_dir.as_ref())?;
            let decision = decision::decide_with_events(&spec, &input);
            let trace = if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
                Some(trace)
            } else {
                None
            };
            let act_report = act::build_report(&spec, &decision.decision, trace.as_ref());
            if json {
                report::json(&act_report)?;
            } else {
                report::text(&act::render_plan(&act_report))?;
            }
        }
        Command::Explain {
            path,
            input,
            trace_dir,
        } => {
            let spec = parser::load_spec(&path)?;
            ensure_trace_available(&spec, trace_dir.as_ref())?;
            let decision = decision::decide_with_events(&spec, &input);
            if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
            }
            report::explain(&decision.decision)?;
        }
        Command::Sensemake { path, view, json } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::sensemake(&spec, &path, view.into());
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_sensemake(&report))?;
            }
        }
        Command::Query {
            path,
            handle,
            view,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::query(&spec, &path, &handle, view.into())?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_query(&report))?;
            }
        }
        Command::Refs {
            path,
            handle,
            view,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::refs(&spec, &path, &handle, view.into())?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_refs(&report))?;
            }
        }
        Command::Grammar { command } => match command {
            GrammarCommand::Sensemake { view, json } => {
                let report = grammar::sensemake(view.into());
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&grammar::render_sensemake(&report))?;
                }
            }
            GrammarCommand::Checklist { for_subject, json } => {
                let report = grammar::checklist(for_subject.into());
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&grammar::render_checklist(&report))?;
                }
            }
            GrammarCommand::Schema { json } => {
                if json {
                    report::json(&grammar::schema_json()?)?;
                } else {
                    report::text(&grammar::render_schema_summary())?;
                }
            }
        },
        Command::Trace { command } => match command {
            TraceCommand::Compact { run_dir } => {
                let trace = trace::compact(&run_dir)?;
                report::json(&trace)?;
            }
            TraceCommand::Align {
                path,
                decision_trace,
                execution_trace,
                json,
            } => {
                let spec = parser::load_spec(&path)?;
                let report =
                    align::align_decision_trace(&spec, &path, &decision_trace, &execution_trace)?;
                let alignment_report = align::write_report_json(&decision_trace, &report)?;
                report::alignment_written(&alignment_report)?;
                if json {
                    report::json(&report)?;
                } else {
                    report::align(&report)?;
                }
                if report.has_failures() {
                    std::process::exit(1);
                }
            }
        },
        Command::Progress { command } => match command {
            ProgressCommand::Show { path, run, json } => {
                let spec = parser::load_spec(&path)?;
                let report = progress::show(&spec, &run)?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&progress::render(&report))?;
                }
            }
            ProgressCommand::Record {
                run,
                event,
                phase,
                requirement,
                id,
                status,
                evidence_kind,
                evidence_ref,
                source_skill,
                message,
                json: _,
            } => {
                let event = progress::record(progress::RecordOptions {
                    run_dir: run,
                    event: event.into(),
                    phase,
                    requirement,
                    id,
                    status,
                    evidence_kind,
                    evidence_ref,
                    source_skill,
                    message,
                })?;
                report::json(&event)?;
            }
            ProgressCommand::Stats {
                run,
                workspace,
                phase,
                requirement,
                workspace_stats_json,
                workspace_stats_report,
                total_tokens,
                context_tokens,
                query_result_tokens,
                response_tokens_cached,
                saved_tokens,
                reduction_percent,
                message,
                json: _,
            } => {
                let event = progress::record_stats(progress::StatsRecordOptions {
                    run_dir: run,
                    workspace,
                    phase,
                    requirements: requirement,
                    workspace_stats_json,
                    workspace_stats_report,
                    total_tokens,
                    context_tokens,
                    query_result_tokens,
                    response_tokens_cached,
                    saved_tokens,
                    reduction_percent,
                    message,
                })?;
                report::json(&event)?;
            }
            ProgressCommand::FinalResponse {
                run,
                phase,
                requirement,
                result,
                evidence,
                alignment,
                token_savings,
                message,
                json: _,
            } => {
                let event =
                    progress::record_final_response(progress::FinalResponseRecordOptions {
                        run_dir: run,
                        phase,
                        requirements: requirement,
                        included_result: result,
                        included_evidence: evidence,
                        included_alignment: alignment,
                        included_token_savings: token_savings,
                        message,
                    })?;
                report::json(&event)?;
            }
        },
        Command::Deps { command } => match command {
            DepsCommand::Check { path, command } => {
                let spec = parser::load_spec(&path)?;
                let spec_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
                let report = deps::check(&spec, spec_dir, command.as_deref())?;
                report::json(&report)?;
                if !report.ok {
                    std::process::exit(1);
                }
            }
        },
        Command::Imports { command } => match command {
            ImportsCommand::Check { path } => {
                let spec = parser::load_spec_unresolved(&path)?;
                let report = imports::check(&spec, &path);
                report::json(&report)?;
                if !report.ok {
                    std::process::exit(1);
                }
            }
        },
        Command::Compile { path, target } => {
            let spec = parser::load_spec(&path)?;
            let markdown = compiler::compile(&spec, target.into());
            std::io::stdout().lock().write_all(markdown.as_bytes())?;
        }
        Command::ImportSkill { path, out } => {
            let imported = importer::import_skill_for_output(&path, &out)?;
            parser::write_spec(&out, &imported)?;
            report::import_ok(&path, &out, &imported)?;
        }
        Command::SynthesizeFromWorkspace {
            workspace,
            out,
            task,
            name,
            log_last,
            workspace_stats_report,
            workspace_log,
            workspace_meta,
            workspace_deps,
            force,
            json,
        } => {
            let synthesis = workspace_synthesizer::synthesize_from_workspace(
                workspace_synthesizer::SynthesizeOptions {
                    workspace,
                    task,
                    out,
                    name,
                    log_last,
                    workspace_stats_report,
                    workspace_log,
                    workspace_meta,
                    workspace_deps,
                    force,
                },
            )?;
            if json {
                report::json(&synthesis)?;
            } else {
                report::text(&workspace_synthesizer::render_report(&synthesis))?;
            }
        }
        Command::Index {
            roots,
            out,
            visibility_manifest,
            json,
        } => {
            let report = router::index(router::IndexOptions {
                roots,
                out,
                visibility_manifest,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router::render_index(&report))?;
            }
        }
        Command::Route {
            index,
            query,
            top,
            execution_mode,
            json,
        } => {
            let report = router::route(router::RouteOptions {
                index,
                query,
                top,
                execution_mode: execution_mode.map(Into::into),
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router::render_route(&report))?;
            }
        }
        Command::Skills { command } => match command {
            SkillsCommand::Audit { roots, json } => {
                let report = router::audit(&roots)?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router::render_audit(&report))?;
                }
            }
            SkillsCommand::SetVisibility {
                skill,
                visibility,
                roots,
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                    roots,
                    skill,
                    visibility: visibility.into(),
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_apply(&report))?;
                }
            }
            SkillsCommand::Disable {
                skill,
                roots,
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                    roots,
                    skill,
                    visibility: router::Visibility::Off,
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_apply(&report))?;
                }
            }
            SkillsCommand::Enable {
                skill,
                roots,
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                    roots,
                    skill,
                    visibility: router::Visibility::Implicit,
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_apply(&report))?;
                }
            }
        },
        Command::Visibility { command } => match command {
            VisibilityCommand::Plan {
                roots,
                profile,
                json,
            } => {
                let report = visibility::plan(visibility::VisibilityPlanOptions {
                    roots,
                    profile: profile.into(),
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_plan(&report))?;
                }
            }
            VisibilityCommand::Apply {
                roots,
                profile,
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::apply(visibility::VisibilityApplyOptions {
                    roots,
                    profile: profile.into(),
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_apply(&report))?;
                }
            }
            VisibilityCommand::Restore {
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::restore(visibility::VisibilityRestoreOptions {
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_restore(&report))?;
                }
            }
        },
        Command::Router { command } => match command {
            RouterCommand::Install {
                roots,
                index,
                manifest,
                router_name,
                dry_run,
                json,
            } => {
                let report = router_lifecycle::install(router_lifecycle::RouterInstallOptions {
                    roots,
                    index,
                    manifest,
                    router_name: Some(router_name),
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_install(&report))?;
                }
            }
            RouterCommand::Uninstall {
                manifest,
                router_name,
                index,
                keep_index,
                dry_run,
                json,
            } => {
                let report =
                    router_lifecycle::uninstall(router_lifecycle::RouterUninstallOptions {
                        manifest,
                        router_name: Some(router_name),
                        index,
                        keep_index,
                        dry_run,
                    })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_uninstall(&report))?;
                }
            }
            RouterCommand::Index { command } => match command {
                RouterIndexCommand::Refresh {
                    roots,
                    index,
                    visibility_manifest,
                    json,
                } => {
                    let report =
                        router_lifecycle::refresh(router_lifecycle::RouterRefreshOptions {
                            roots,
                            index,
                            visibility_manifest,
                        })?;
                    if json {
                        report::json(&report)?;
                    } else {
                        report::text(&router_lifecycle::render_refresh(&report))?;
                    }
                }
                RouterIndexCommand::Status {
                    roots,
                    index,
                    visibility_manifest,
                    json,
                } => {
                    let report = router::index_status(router::IndexStatusOptions {
                        roots,
                        index,
                        visibility_manifest,
                    })?;
                    if json {
                        report::json(&report)?;
                    } else {
                        report::text(&router::render_index_status(&report))?;
                    }
                }
            },
        },
        Command::Install { command } => match command {
            InstallCommand::Targets => {
                let targets = install::detect_targets()?;
                report::json(&targets)?;
            }
            InstallCommand::Skill {
                folder,
                target,
                all_detected,
                dry_run,
                force,
                name,
            } => {
                let targets = target
                    .into_iter()
                    .map(HarnessTarget::from)
                    .collect::<Vec<_>>();
                let report = install::install_skill(
                    &folder,
                    &targets,
                    all_detected,
                    dry_run,
                    force,
                    name.as_deref(),
                )?;
                report::json(&report)?;
            }
        },
        Command::Capability { command } => match command {
            CapabilityCommand::Store => {
                report::json(&capability::store()?)?;
            }
            CapabilityCommand::Add {
                id,
                domain,
                kind,
                command,
                adapter,
                script,
                provides,
                alias,
                priority,
                preferred_for,
                avoid_for,
                ties,
                auth_env,
                external_service,
                may_cost_money,
                evidence_command,
                suggested_skill_id,
            } => {
                let report = capability::add(capability::AddOptions {
                    id,
                    domain,
                    kind,
                    command,
                    adapter,
                    script,
                    provides,
                    aliases: alias,
                    priority,
                    preferred_for,
                    avoid_for,
                    ties,
                    auth_env,
                    external_service,
                    may_cost_money,
                    evidence_command,
                    suggested_skill_id,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Update {
                id,
                domain,
                kind,
                command,
                clear_command,
                adapter,
                clear_adapter,
                script,
                clear_script,
                add_provides,
                remove_provides,
                add_alias,
                remove_alias,
                priority,
                clear_priority,
                add_preferred_for,
                remove_preferred_for,
                add_avoid_for,
                remove_avoid_for,
                add_tie,
                remove_tie,
                add_auth_env,
                remove_auth_env,
                external_service,
                may_cost_money,
                add_evidence_command,
                remove_evidence_command,
                suggested_skill_id,
                clear_suggested_skill_id,
                mark_unverified,
                mark_failed,
            } => {
                let verification_status = if mark_failed {
                    Some(capability::VerificationStatus::Failed)
                } else if mark_unverified {
                    Some(capability::VerificationStatus::Unverified)
                } else {
                    None
                };
                let report = capability::update(capability::UpdateOptions {
                    id,
                    domain,
                    kind,
                    command,
                    clear_command,
                    adapter,
                    clear_adapter,
                    script,
                    clear_script,
                    add_provides,
                    remove_provides,
                    add_alias,
                    remove_alias,
                    priority,
                    clear_priority,
                    add_preferred_for,
                    remove_preferred_for,
                    add_avoid_for,
                    remove_avoid_for,
                    add_ties: add_tie,
                    remove_tie,
                    add_auth_env,
                    remove_auth_env,
                    external_service,
                    may_cost_money,
                    add_evidence_command,
                    remove_evidence_command,
                    suggested_skill_id,
                    clear_suggested_skill_id,
                    verification_status,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::List { domain } => {
                report::json(&capability::list(domain.as_deref())?)?;
            }
            CapabilityCommand::Search {
                capability: capability_id,
                domain,
                explain: _,
                json: _,
                local_only,
                preferred_seed,
            } => {
                let report = capability::search(capability::SearchOptions {
                    capability: capability_id,
                    domain,
                    local_only,
                    preferred_seed,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Inspect {
                id,
                domain,
                json: _,
            } => {
                report::json(&capability::inspect(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Verify {
                id,
                domain,
                json: _,
            } => {
                report::json(&capability::verify(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Prefer {
                id,
                domain,
                for_capability,
                priority,
            } => {
                let report = capability::prefer(capability::PreferOptions {
                    id,
                    domain,
                    for_capability,
                    priority,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Remove { id, domain } => {
                report::json(&capability::remove(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Scan => {
                report::json(&capability::scan()?)?;
            }
        },
    }

    Ok(())
}

fn ensure_trace_available(spec: &model::SkillSpec, trace_dir: Option<&PathBuf>) -> Result<()> {
    if spec
        .trace
        .as_ref()
        .is_some_and(|trace| trace.required && trace_dir.is_none())
    {
        return Err(error::Error::InvalidInput {
            message: "trace.required is true; pass --trace-dir or use a spec that does not require tracing"
                .to_owned(),
        });
    }
    Ok(())
}

impl From<InstallTargetArg> for HarnessTarget {
    fn from(value: InstallTargetArg) -> Self {
        match value {
            InstallTargetArg::Agents => Self::Agents,
            InstallTargetArg::Codex => Self::Codex,
            InstallTargetArg::ClaudeLocal => Self::ClaudeLocal,
        }
    }
}

impl From<RouterExecutionModeArg> for router::ExecutionMode {
    fn from(value: RouterExecutionModeArg) -> Self {
        match value {
            RouterExecutionModeArg::Direct => Self::Direct,
            RouterExecutionModeArg::Durable => Self::Durable,
        }
    }
}

impl From<VisibilityArg> for router::Visibility {
    fn from(value: VisibilityArg) -> Self {
        match value {
            VisibilityArg::Implicit => Self::Implicit,
            VisibilityArg::ManualOnly => Self::ManualOnly,
            VisibilityArg::NameOnly => Self::NameOnly,
            VisibilityArg::Off => Self::Off,
        }
    }
}

impl From<VisibilityProfileArg> for visibility::VisibilityProfile {
    fn from(value: VisibilityProfileArg) -> Self {
        match value {
            VisibilityProfileArg::RouterManaged => Self::RouterManaged,
        }
    }
}

impl From<CompileTarget> for compiler::Target {
    fn from(value: CompileTarget) -> Self {
        match value {
            CompileTarget::CodexSkill => compiler::Target::CodexSkill,
            CompileTarget::ClaudeSkill => compiler::Target::ClaudeSkill,
            CompileTarget::Markdown => compiler::Target::Markdown,
        }
    }
}

impl From<SenseViewArg> for sensemake::View {
    fn from(value: SenseViewArg) -> Self {
        match value {
            SenseViewArg::Index => Self::Index,
            SenseViewArg::Summary => Self::Summary,
            SenseViewArg::Full => Self::Full,
        }
    }
}

impl From<GrammarViewArg> for grammar::GrammarView {
    fn from(value: GrammarViewArg) -> Self {
        match value {
            GrammarViewArg::Index => Self::Index,
            GrammarViewArg::Summary => Self::Summary,
            GrammarViewArg::Porting => Self::Porting,
            GrammarViewArg::Full => Self::Full,
        }
    }
}

impl From<GrammarChecklistForArg> for grammar::ChecklistSubject {
    fn from(value: GrammarChecklistForArg) -> Self {
        match value {
            GrammarChecklistForArg::ImportSkill => Self::ImportSkill,
        }
    }
}

impl From<ProgressEventArg> for String {
    fn from(value: ProgressEventArg) -> Self {
        match value {
            ProgressEventArg::PhaseStarted => "phase_started",
            ProgressEventArg::RequirementStarted => "requirement_started",
            ProgressEventArg::RequirementSatisfied => "requirement_satisfied",
            ProgressEventArg::RequirementFailed => "requirement_failed",
            ProgressEventArg::StatsCollected => "stats_collected",
            ProgressEventArg::ObligationSatisfied => "obligation_satisfied",
            ProgressEventArg::RouteFulfilled => "route_fulfilled",
            ProgressEventArg::AfterSuccessCompleted => "after_success_completed",
            ProgressEventArg::EvidenceAttached => "evidence_attached",
            ProgressEventArg::HandoffStarted => "handoff_started",
            ProgressEventArg::HandoffCompleted => "handoff_completed",
            ProgressEventArg::PhaseCompleted => "phase_completed",
            ProgressEventArg::PhaseBlocked => "phase_blocked",
        }
        .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_required_requires_trace_dir() {
        let yaml = r#"
schema: skillspec/v0
id: trace.required
title: Trace Required
description: Requires trace output.
routes:
  - id: local
    label: Local
trace:
  mode: event_log
  required: true
tests:
  - name: route assertion
    input: run this
    expect:
      route: local
"#;
        let spec = serde_yaml::from_str::<model::SkillSpec>(yaml).unwrap();
        let trace_dir = PathBuf::from(".skillspec/traces");

        assert!(ensure_trace_available(&spec, None).is_err());
        assert!(ensure_trace_available(&spec, Some(&trace_dir)).is_ok());
    }
}
