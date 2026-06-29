use super::InstallTargetArg;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum InstallCommand {
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
