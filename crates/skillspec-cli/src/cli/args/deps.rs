use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum DepsCommand {
    #[command(about = "Check declared dependencies, optionally scoped to one command template")]
    Check {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Check only dependencies required by this command id.
        #[arg(long)]
        command: Option<String>,
    },
}
