mod capability;
mod deps;
mod durable;
mod grammar;
mod imports;
mod install;
mod progress;
mod router;
mod skills;
mod source;
mod trace;
mod types;
mod visibility;
mod workspace;

pub(in crate::cli) use capability::CapabilityCommand;
pub(in crate::cli) use deps::DepsCommand;
pub(in crate::cli) use durable::DurableExecutorCommand;
pub(in crate::cli) use grammar::GrammarCommand;
pub(in crate::cli) use imports::ImportsCommand;
pub(in crate::cli) use install::InstallCommand;
pub(in crate::cli) use progress::ProgressCommand;
pub(in crate::cli) use router::{RouterCommand, RouterIndexCommand};
pub(in crate::cli) use skills::SkillsCommand;
pub(in crate::cli) use source::SourceCommand;
pub(in crate::cli) use trace::TraceCommand;
pub(in crate::cli) use types::*;
pub(in crate::cli) use visibility::VisibilityCommand;
pub(in crate::cli) use workspace::WorkspaceCommand;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "skillspec")]
#[command(about = "Structured skills for agent behavior")]
#[command(version)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub(super) enum Command {
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
    #[command(
        about = "Batch routing, planning, action guidance, and optional resume anchors",
        long_about = "Load one skill.spec.yml once, then produce a compact planning-loop report containing sensemake navigation, routing decision, phase plan, and the first or requested action checklist. With --guide agent, write guide-state.json and guide-summary.md, print start/current/end anchors, and support --resume from an existing run directory. This is a read/planning convenience wrapper; it does not execute tools or mutate external systems except optional trace and guide-state output."
    )]
    RunLoop {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// User task text to route. Strip skill invocation prefixes before passing it.
        #[arg(long, allow_hyphen_values = true, conflicts_with = "resume")]
        input: Option<String>,
        /// Resume an existing guided run from its run directory.
        #[arg(long, value_name = "RUN_DIR", conflicts_with = "input")]
        resume: Option<PathBuf>,
        /// Sensemake detail level included in the batch report.
        #[arg(long, value_enum, default_value_t = SenseViewArg::Index)]
        view: SenseViewArg,
        /// Directory where append-only decision trace events should be written.
        #[arg(long)]
        trace_dir: Option<PathBuf>,
        /// Expand this execution phase instead of the first pending phase.
        #[arg(long)]
        phase: Option<String>,
        /// Emit a stateful agent guide with start/current/end anchors.
        #[arg(long, value_enum)]
        guide: Option<GuideModeArg>,
        /// Emit JSON instead of a compact human report.
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
        /// Query handle, such as routes, rule:<id>, command:<id>.requires, or test:<name>.expect.
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
        /// Item handle, such as rule:<id>, command:<id>, state:<id>, recipe:<id>, or test:<name>.
        handle: String,
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = SenseViewArg::Summary)]
        view: SenseViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Scan skills and skill workspaces for static drift, discovery, and context risk",
        long_about = "Scan a prose SKILL.md file, local folder, public GitHub skill folder, or public GitHub repo URI without executing tools or calling a model. GitHub folder URLs may use /tree/<branch>/...; /blob/<branch>/... is also accepted when the path resolves to a folder rather than SKILL.md. Doctor first builds a current-skill baseline: simple SKILL.md packages receive full agent follow-through risk analysis; multi-skill workspaces, root skills with subskills, and plugin-shaped workspaces receive aggregate workspace risk plus one package report per SKILL.md; non-skill code repos return a shape-only report so doctor does not waste work on ordinary code. Remote GitHub targets are staged with a temporary partial sparse checkout and cleaned up after the report. Human reports explain risk in plain language: how likely the current skill shape is to make an agent skip, reorder, improvise, use the wrong surface, or finish without proof. JSON preserves machine fields including score_model, structural_score, frontmatter discovery risk, activation-loaded surface, instruction-density risk, primacy-bias risk, embedded-code ambiguity, implicit dependency contracts, missing references, and missing proof/trace surfaces."
    )]
    Doctor {
        /// Local SKILL.md file/folder, public GitHub skill folder URL, or public GitHub repo URI.
        ///
        /// GitHub folder URLs may use /tree/<branch>/...; /blob/<branch>/...
        /// is also accepted when the path resolves to a folder rather than SKILL.md.
        path: String,
        /// Emit machine-readable JSON instead of the formatted human report.
        #[arg(long, conflicts_with_all = ["html", "markdown"])]
        json: bool,
        /// Emit a self-contained HTML report instead of the formatted human report.
        #[arg(long, conflicts_with_all = ["json", "markdown"])]
        html: bool,
        /// Emit GitHub-flavored Markdown instead of the formatted human report.
        #[arg(long, conflicts_with_all = ["json", "html"])]
        markdown: bool,
    },
    #[command(
        about = "Show installed SkillSpec lifecycle status, roots, router index state, and skill inventory"
    )]
    Status {
        /// Skill roots to scan for inventory. Defaults to router config roots, then detected harness roots.
        #[arg(long = "roots", num_args = 1..)]
        roots: Vec<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Map and query source packages for progressive import")]
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    #[command(
        about = "Map, validate, import, converge, compile, and install multi-skill or plugin-shaped workspaces"
    )]
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
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
    #[command(
        about = "Create a mechanical draft SkillSpec from a local skill file or folder",
        long_about = "Create a mechanical draft SkillSpec from a local SKILL.md file or single skill folder after confirming the source is one atomic package. Parent folders with multiple SKILL.md files, cross-skill references, or plugin markers are workspaces and should start with `skillspec workspace map <source-root> --out <build-dir>/skillspec.workspace.yml`. Existing reviewed skill.spec.yml files should be revised from grammar/current-spec handles, not re-imported. For large or code-heavy single-skill sources, run `skillspec source map`, inspect `source coverage` and focused `source query` handles, then pass the fresh source-map.json with --source-map. The importer materializes fenced code under resources/imported-code/, writes a scaffolded deps.toml beside the draft, declares that ledger as a file dependency/artifact, and seeds it with inferred CLI plus Python/JavaScript/TypeScript package imports for later semantic review."
    )]
    ImportSkill {
        /// Local SKILL.md file or skill folder to import.
        path: PathBuf,
        /// Output path for the generated skill.spec.yml draft.
        #[arg(long)]
        out: PathBuf,
        /// Fresh source-map.json produced by `skillspec source map` for the same source.
        #[arg(long = "source-map")]
        source_map: Option<PathBuf>,
    },
    #[command(
        about = "Port one atomic prose skill through grammar preflight, import, QA, compile, and metrics",
        long_about = "Bundle the safe single-skill porting ladder in one command: embedded grammar/schema/checklist artifacts, source map, doctor report, typed mechanical import, schema-derived shape crib, validate, imports check, deps check, scenario tests, compile, compact proof report, and optional direct-run estimated progress stats. This is for one atomic skill package only. Parent folders with multiple SKILL.md files, cross-skill references, or plugin markers must use workspace map/import. Existing reviewed skill.spec.yml files should use the revision path instead of one-shot import."
    )]
    PortOneShot {
        /// Local SKILL.md file or single skill folder to port.
        source: PathBuf,
        /// Output skill folder. The command writes <out>/skill.spec.yml and proof under <out>/.skillspec/.
        #[arg(long)]
        out: PathBuf,
        /// Compile target used for the QA compile gate.
        #[arg(long, value_enum, default_value = "codex-skill")]
        target: CompileTarget,
        /// Treat failed required QA gates as a failed proof run.
        #[arg(long)]
        prove: bool,
        /// Overwrite an existing <out>/skill.spec.yml draft.
        #[arg(long)]
        force: bool,
        /// Existing trace run directory where estimated direct-run token metrics should be recorded.
        #[arg(long)]
        run_dir: Option<PathBuf>,
        /// Phase id whose requirement(s) the estimated stats event satisfies.
        #[arg(long)]
        phase: Option<String>,
        /// Requirement id satisfied by the estimated stats event. Repeat for multiple requirements.
        #[arg(long = "requirement")]
        requirements: Vec<String>,
        /// Emit JSON instead of a compact human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Synthesize a draft SkillSpec from a durable rote workspace (rote-specific)",
        long_about = "Synthesize a draft SkillSpec from rote-specific durable execution evidence. This optional integration requires a rote workspace name and validates workspace stats, command log, and metadata evidence. If pre-captured evidence files are supplied, synthesis does not need live rote workspace lookup. The command refuses to write the scaffold until --observation-approved confirms the observed result and evidence summary were shown and accepted."
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
        /// Confirm the observed result and evidence summary were shown and accepted before synthesis.
        #[arg(long)]
        observation_approved: bool,
        /// Overwrite an existing skill.spec.yml in the output folder.
        #[arg(long)]
        force: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Build a router-specific skill catalog for skill-router/manual route lookup",
        long_about = "Build the SQLite catalog used by `skillspec route` and the optional skill-router. This is router-specific runtime discovery, not source analysis, workspace recon, or skill import. If router mode is disabled or not installed, this command only writes a standalone catalog; it does not activate the router or change harness visibility. For installed router maintenance, prefer `skillspec router index refresh`."
    )]
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
    #[command(about = "Decide whether a user request should load a skill from an index")]
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
    #[command(about = "Install, update, or delete the optional durable-executor first-hop skill")]
    DurableExecutor {
        #[command(subcommand)]
        command: DurableExecutorCommand,
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
