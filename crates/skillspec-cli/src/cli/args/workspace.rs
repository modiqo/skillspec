use super::{
    InstallTargetArg, WorkspaceCompileTarget, WorkspaceInstallSlugPolicyArg,
    WorkspaceVisibilityPolicyArg,
};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum WorkspaceCommand {
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
