use clap::Subcommand;
use skillspec::router_lifecycle;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RouterCommand {
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
    #[command(about = "Verify router-first readiness, repair drift, and emit native hook output")]
    Guard {
        /// Router config path. Defaults to the installed router config.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Emit harness hook JSON for UserPromptSubmit.
        #[arg(long)]
        hook: bool,
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
pub(in crate::cli) enum RouterIndexCommand {
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
