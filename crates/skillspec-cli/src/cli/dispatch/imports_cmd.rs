use crate::cli::args::ImportsCommand;
use skillspec::{domain::authoring, error::Result, report};

pub(super) fn run(command: ImportsCommand) -> Result<()> {
    match command {
        ImportsCommand::Check { path } => {
            let report = authoring::check_imports(&path)?;
            report::json(&report)?;
            if !report.ok {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
