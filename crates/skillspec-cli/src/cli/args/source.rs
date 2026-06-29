use super::SourceViewArg;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum SourceCommand {
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
