use super::SourceViewArg;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum SourceCommand {
    #[command(
        about = "Stage a public GitHub skill URI locally for candidate discovery",
        long_about = "Stage a public GitHub repository, tree URL, blob-style folder URL, owner/repo shorthand, or owner/repo/path shorthand into a local sparse checkout for candidate discovery or explicit persistent staging. The command parses the URI into repo, branch, and path, materializes the requested skill folder or SKILL.md candidates, and prints the exact local source path to pass to doctor, import-skill, workspace map, or port-one-shot. Normal source-map work can call `skillspec source map <github-uri>` directly; it uses the same sparse staging code and reports the mapped source_path. Do not use web search or raw GitHub fallback to locate the same source."
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
        about = "Create source-map.json and source-map.md from a local source or public GitHub skill URI",
        long_about = "Create source-map.json and source-map.md from a local SKILL.md file, skill folder, public GitHub repo/tree URL, or owner/repo shorthand. Public GitHub sources are staged with SkillSpec's sparse checkout logic, then mapped from the selected local source path. If a repo URI contains multiple SKILL.md candidates, the command refuses to guess and prints candidate source paths."
    )]
    Map {
        /// Source SKILL.md file, skill folder, public GitHub skill URI, or owner/repo shorthand to map.
        source: String,
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
    #[command(
        about = "Show the next source block to port and prove",
        long_about = "Render a deterministic progressive review lens over source-map.json. Each unit is one parsed Markdown block with its source hash, countdown position, classifications, references, and required SkillSpec target kinds. Use this during semantic promotion: inspect one unit, port it into structural SkillSpec constructs, validate, record the unit in promotion proof, then advance with the returned next_cursor."
    )]
    Lens {
        /// Path to source-map.json.
        map: PathBuf,
        /// 1-based unit cursor to show.
        #[arg(long, default_value_t = 1)]
        cursor: usize,
        /// Number of units to show. Defaults to one to force progressive review.
        #[arg(long, default_value_t = 1)]
        limit: usize,
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
