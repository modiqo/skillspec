use clap::{Parser, Subcommand, ValueEnum};
use skillspec::{
    compiler, grammar, install::HarnessTarget, router, router_lifecycle, sensemake, source_map,
    visibility, workspace,
};
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

#[derive(Clone, Debug, clap::ValueEnum)]
pub(super) enum CompileTarget {
    CodexSkill,
    ClaudeSkill,
    Markdown,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(super) enum WorkspaceCompileTarget {
    CodexSkill,
    ClaudeSkill,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(super) enum WorkspaceVisibilityPolicyArg {
    EntryImplicit,
    AllImplicit,
    AllManual,
    None,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(super) enum RouterExecutionModeArg {
    Direct,
    Durable,
}

#[derive(Debug, Subcommand)]
pub(super) enum SourceCommand {
    #[command(
        about = "Stage a public GitHub skill URI locally before doctor, source map, or import",
        long_about = "Stage a public GitHub repository, tree URL, blob-style folder URL, owner/repo shorthand, or owner/repo/path shorthand into a local sparse checkout. The command parses the URI into repo, branch, and path, materializes the requested skill folder or SKILL.md candidates, and prints the exact local source path to pass to doctor, source map, import-skill, workspace map, or port-one-shot. Use this for import/port prompts that contain a URI; do not use web search or raw GitHub fallback to locate the same source."
    )]
    Stage {
        /// Public GitHub repo URI, tree URI, blob-style folder URI, owner/repo shorthand, or owner/repo/path shorthand.
        uri: String,
        /// Output directory to create for the persistent sparse checkout. Defaults to .skillspec/staged/<repo>-<timestamp>.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Skip candidate discovery for repo-root targets.
        #[arg(long)]
        no_detect_candidates: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Create source-map.json and source-map.md from a SKILL.md file or skill folder"
    )]
    Map {
        /// Source SKILL.md file or skill folder to map.
        path: PathBuf,
        /// Output directory for source-map.json and source-map.md.
        #[arg(long)]
        out: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Query one source-map handle or collection")]
    Query {
        /// Path to source-map.json.
        map: PathBuf,
        /// Query handle, such as files, nodes, dependencies, code, or heading:<file>.<slug>.
        handle: String,
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = SourceViewArg::Summary)]
        view: SourceViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show source-map coverage and review-required counts")]
    Coverage {
        /// Path to source-map.json.
        map: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Check that source files still match source-map.json hashes")]
    Stale {
        /// Path to source-map.json.
        map: PathBuf,
        /// Source root to compare against. Defaults to the map's recorded source root.
        #[arg(long)]
        root: Option<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum WorkspaceCommand {
    #[command(
        about = "Create a skillspec.workspace.yml graph from a folder with SKILL.md packages or plugin-shaped roots",
        long_about = "Create a skillspec.workspace.yml graph from a local source root. This is authoring structure recon, not router indexing. It discovers atomic skill packages, plugin-shaped namespace roots, skill-safe public names, deterministic install slugs, cross-package references, inferred file dependencies, duplicate public names, and duplicate install slugs before fanout import. The default workspace-path install slug policy is side-by-side and plugin safe; use local-name for replacement installs that must retire canonical existing skill folders. Plugin slash-command references are recorded as workflow links without becoming hard dependency edges."
    )]
    Map {
        /// Local source root containing one or more skill packages.
        source_root: PathBuf,
        /// Output path for skillspec.workspace.yml.
        #[arg(long)]
        out: PathBuf,
        /// Install folder slug policy to write into the manifest.
        #[arg(long, value_enum, default_value_t = WorkspaceInstallSlugPolicyArg::WorkspacePath)]
        install_slug_policy: WorkspaceInstallSlugPolicyArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
        /// Emit a compact metric summary with report paths instead of the full human report.
        #[arg(long, conflicts_with = "json")]
        summary: bool,
    },
    #[command(
        about = "Validate a skillspec.workspace.yml package graph",
        long_about = "Validate a skillspec.workspace.yml package graph before fanout import. Checks package paths, one SKILL.md per package, dependency references, self-dependencies, cycles, duplicate install slugs, uncovered hard cross-package references, and public-name collision warnings. Plugin slash-command workflow references are allowed without depends_on edges; file references still require dependency coverage."
    )]
    Validate {
        /// Path to skillspec.workspace.yml.
        manifest: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
        /// Emit a compact metric summary with report paths instead of the full human report.
        #[arg(long, conflicts_with = "json")]
        summary: bool,
    },
    #[command(
        about = "Import every package in a validated skillspec.workspace.yml graph",
        long_about = "Run the existing single-package doctor, source map, and import-skill pipeline for every package in a validated skillspec.workspace.yml graph. Independent dependency-ready packages are imported in parallel, unchanged packages with intact artifacts are reused from <build-root>/.skillspec/workspace-cache.json, and outputs are written under one mirrored build root. Successful package outputs are preserved when another package fails; dependents of failed packages are reported as blocked."
    )]
    Import {
        /// Path to skillspec.workspace.yml.
        manifest: PathBuf,
        /// Build root where mirrored package outputs should be written.
        #[arg(long)]
        out: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
        /// Emit a compact metric summary with report paths instead of the full human report.
        #[arg(long, conflicts_with = "json")]
        summary: bool,
    },
    #[command(
        about = "Verify generated workspace package drafts against the manifest",
        long_about = "Converge a workspace build by verifying every manifest package has a ready generated skill.spec.yml or an explicit failure, checking package specs and dependency readiness, and writing workspace-converge.report.md. This does not compile, install, or refresh router indexes."
    )]
    Converge {
        /// Path to skillspec.workspace.yml.
        manifest: PathBuf,
        /// Build root containing mirrored package outputs.
        #[arg(long = "build-root")]
        build_root: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
        /// Emit a compact metric summary with report paths instead of the full human report.
        #[arg(long, conflicts_with = "json")]
        summary: bool,
    },
    #[command(
        about = "Compile ready workspace package drafts into harness loaders",
        long_about = "Compile every ready package in a converged workspace build into a generated SKILL.md loader for the requested harness target. This rechecks workspace convergence, skips packages whose dependencies did not compile, writes workspace-compile.report.md, and does not install skills or refresh router indexes."
    )]
    Compile {
        /// Path to skillspec.workspace.yml.
        manifest: PathBuf,
        /// Build root containing mirrored package outputs.
        #[arg(long = "build-root")]
        build_root: PathBuf,
        /// Harness loader target to generate.
        #[arg(long, value_enum)]
        target: WorkspaceCompileTarget,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
        /// Emit a compact metric summary with report paths instead of the full human report.
        #[arg(long, conflicts_with = "json")]
        summary: bool,
    },
    #[command(
        about = "Install compiled workspace packages into harness roots",
        long_about = "Install a compiled workspace build into one or more harness skill roots. This preflights every package first, uses manifest install_slug folder names or an explicit install slug policy override, blocks folder and public-name collisions unless explicitly retired where supported, installs dependencies before dependents, reports workspace visibility policy, optionally applies native visibility metadata, writes workspace-install.report.md, and does not refresh router indexes."
    )]
    Install {
        /// Path to skillspec.workspace.yml.
        manifest: PathBuf,
        /// Build root containing compiled workspace package outputs.
        #[arg(long = "build-root")]
        build_root: PathBuf,
        /// Harness target to install into. Repeat for multiple targets.
        #[arg(long, value_enum)]
        target: Vec<InstallTargetArg>,
        /// Install into every harness root detected on this machine.
        #[arg(long)]
        all_detected: bool,
        /// Show the full workspace install plan without writing harness files.
        #[arg(long)]
        dry_run: bool,
        /// Back up and remove an existing active install folder before installing this workspace package.
        #[arg(long)]
        retire_existing: bool,
        /// Override manifest install slugs for this install plan without editing the manifest.
        #[arg(long, value_enum)]
        install_slug_policy: Option<WorkspaceInstallSlugPolicyArg>,
        /// Visibility policy recorded for installed packages.
        #[arg(long, value_enum, default_value_t = WorkspaceVisibilityPolicyArg::EntryImplicit)]
        visibility_policy: WorkspaceVisibilityPolicyArg,
        /// Apply native harness visibility metadata after successful install.
        #[arg(long)]
        apply_visibility: bool,
        /// Reversible visibility manifest to write when --apply-visibility is used. Defaults under the build root.
        #[arg(long)]
        visibility_manifest: Option<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
        /// Emit a compact metric summary with report paths instead of the full human report.
        #[arg(long, conflicts_with = "json")]
        summary: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum SkillsCommand {
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
pub(super) enum VisibilityCommand {
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
pub(super) enum VisibilityArg {
    Implicit,
    ManualOnly,
    NameOnly,
    Off,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(super) enum VisibilityProfileArg {
    RouterManaged,
}

#[derive(Debug, Subcommand)]
pub(super) enum RouterCommand {
    #[command(
        about = "Install the managed SkillSpec-backed skill-router, visibility state, index, and preparedness check"
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
    #[command(
        about = "Restore visibility and remove the managed skill-router",
        alias = "delete"
    )]
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
    #[command(
        about = "Back up and update every managed skill-router install, visibility state, and index"
    )]
    Update {
        /// Backup directory to create before mutation. Defaults under the router config directory.
        #[arg(long)]
        backup_dir: Option<PathBuf>,
        /// Show changes without writing files, backups, index, manifest, or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Enable router mode, rebuild the index, and make routed skills explicit-only"
    )]
    Enable {
        /// Show changes without writing files, index, manifest, or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Disable router mode without uninstalling it")]
    Disable {
        /// Show changes without writing files, manifest, or config.
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
pub(super) enum DurableExecutorCommand {
    #[command(
        about = "Install durable-executor from an explicit local source folder after checking rote"
    )]
    Install {
        /// Local durable-executor skill folder containing SKILL.md and skill.spec.yml.
        source: PathBuf,
        /// Harness target to install into. Repeat for multiple targets.
        #[arg(long, value_enum)]
        target: Vec<InstallTargetArg>,
        /// Install into every harness root detected on this machine.
        #[arg(long)]
        all_detected: bool,
        /// Show the install plan without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Overwrite an existing durable-executor folder without prompting.
        #[arg(long)]
        force: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Back up and refresh every managed durable-executor install after checking rote"
    )]
    Update {
        /// Override the source folder recorded at install time.
        #[arg(long)]
        source: Option<PathBuf>,
        /// Backup directory to create before mutation. Defaults under the durable-executor config directory.
        #[arg(long)]
        backup_dir: Option<PathBuf>,
        /// Show changes without writing files or backups.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Delete every managed durable-executor install",
        alias = "uninstall"
    )]
    Delete {
        /// Show changes without removing files.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Enable durable-executor as the implicit durable first-hop after checking rote"
    )]
    Enable {
        /// Show changes without writing visibility metadata or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Disable durable-executor implicit invocation without uninstalling it")]
    Disable {
        /// Show changes without writing visibility metadata or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum RouterIndexCommand {
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
pub(super) enum GrammarCommand {
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
pub(super) enum TraceCommand {
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
        /// Write a grouped missing-proof digest for one-shot final proof batching.
        #[arg(long)]
        proof_digest: Option<PathBuf>,
        /// Print only the completion-facing alignment and token summary while writing the full report to alignment.json.
        #[arg(long, conflicts_with = "json")]
        summary: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum ProgressCommand {
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
        /// Evidence kind, such as file, trace, command, or response_id.
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
        /// Estimated tokens in the compact output visible to the agent.
        #[arg(long)]
        agent_visible_tokens: Option<u64>,
        /// Estimated tokens preserved in artifacts outside the prompt.
        #[arg(long)]
        artifact_tokens_preserved: Option<u64>,
        /// Estimated tokens avoided by showing compact output instead of full artifacts.
        #[arg(long)]
        avoided_tokens: Option<u64>,
        /// Source of the metric values, for example measured or estimated.
        #[arg(long)]
        metrics_source: Option<String>,
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
    #[command(
        about = "Checkpoint multiple structured progress events from JSONL or JSON array",
        long_about = "Append several structured progress/proof events to execution.jsonl in one foreground checkpoint. Use --file with a JSONL batch and --summary for compact agent-facing output. The legacy --events alias is still accepted."
    )]
    Batch {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// JSONL file or JSON array of execution events to append.
        #[arg(long = "file", visible_alias = "events", value_name = "EVIDENCE_BATCH")]
        events: PathBuf,
        /// Label printed in compact summary output.
        #[arg(long)]
        checkpoint: Option<String>,
        /// Emit compact checkpoint output instead of event counts.
        #[arg(long)]
        summary: bool,
        /// Emit JSON for the batch report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum DepsCommand {
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
pub(super) enum ImportsCommand {
    #[command(about = "Check import paths, sections, nesting, and load order")]
    Check {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum InstallCommand {
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
        /// Back up and remove an existing active skill before installing this one.
        #[arg(long)]
        retire_existing: bool,
        /// Override the installed skill folder name.
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub(super) enum CapabilityCommand {
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
pub(super) enum InstallTargetArg {
    Agents,
    Codex,
    ClaudeLocal,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum WorkspaceInstallSlugPolicyArg {
    WorkspacePath,
    LocalName,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum SenseViewArg {
    Index,
    Summary,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum GuideModeArg {
    Agent,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum SourceViewArg {
    Index,
    Summary,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum GrammarViewArg {
    Index,
    Summary,
    Porting,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum GrammarChecklistForArg {
    ImportSkill,
}

#[derive(Clone, Debug, ValueEnum)]
pub(super) enum ProgressEventArg {
    PhaseStarted,
    RequirementStarted,
    RequirementSatisfied,
    RequirementFailed,
    StatsCollected,
    ObligationSatisfied,
    RouteFulfilled,
    RouteCheckCompleted,
    AfterSuccessCompleted,
    ElicitationAnswered,
    ElicitationWaived,
    EvidenceAttached,
    HandoffStarted,
    HandoffCompleted,
    PhaseCompleted,
    PhaseBlocked,
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

impl From<WorkspaceInstallSlugPolicyArg> for workspace::WorkspaceInstallSlugPolicy {
    fn from(value: WorkspaceInstallSlugPolicyArg) -> Self {
        match value {
            WorkspaceInstallSlugPolicyArg::WorkspacePath => Self::WorkspacePath,
            WorkspaceInstallSlugPolicyArg::LocalName => Self::LocalName,
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

impl From<WorkspaceCompileTarget> for compiler::Target {
    fn from(value: WorkspaceCompileTarget) -> Self {
        match value {
            WorkspaceCompileTarget::CodexSkill => compiler::Target::CodexSkill,
            WorkspaceCompileTarget::ClaudeSkill => compiler::Target::ClaudeSkill,
        }
    }
}

impl From<WorkspaceVisibilityPolicyArg> for skillspec::workspace::WorkspaceVisibilityPolicy {
    fn from(value: WorkspaceVisibilityPolicyArg) -> Self {
        match value {
            WorkspaceVisibilityPolicyArg::EntryImplicit => Self::EntryImplicit,
            WorkspaceVisibilityPolicyArg::AllImplicit => Self::AllImplicit,
            WorkspaceVisibilityPolicyArg::AllManual => Self::AllManual,
            WorkspaceVisibilityPolicyArg::None => Self::None,
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

impl From<SourceViewArg> for source_map::SourceView {
    fn from(value: SourceViewArg) -> Self {
        match value {
            SourceViewArg::Index => Self::Index,
            SourceViewArg::Summary => Self::Summary,
            SourceViewArg::Full => Self::Full,
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
            ProgressEventArg::RouteCheckCompleted => "route_check_completed",
            ProgressEventArg::AfterSuccessCompleted => "after_success_completed",
            ProgressEventArg::ElicitationAnswered => "elicitation_answered",
            ProgressEventArg::ElicitationWaived => "elicitation_waived",
            ProgressEventArg::EvidenceAttached => "evidence_attached",
            ProgressEventArg::HandoffStarted => "handoff_started",
            ProgressEventArg::HandoffCompleted => "handoff_completed",
            ProgressEventArg::PhaseCompleted => "phase_completed",
            ProgressEventArg::PhaseBlocked => "phase_blocked",
        }
        .to_owned()
    }
}
