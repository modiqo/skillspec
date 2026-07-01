use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum ImportsCommand {
    #[command(about = "Check import paths, sections, nesting, and load order")]
    Check {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
    },
}
