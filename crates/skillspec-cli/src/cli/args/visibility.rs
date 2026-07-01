use super::VisibilityProfileArg;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum VisibilityCommand {
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
