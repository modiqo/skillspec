use super::InstallTargetArg;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum DurableExecutorCommand {
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
