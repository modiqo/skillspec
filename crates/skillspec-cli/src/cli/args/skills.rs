use super::VisibilityArg;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum SkillsCommand {
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
