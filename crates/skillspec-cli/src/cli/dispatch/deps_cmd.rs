use crate::cli::args::DepsCommand;
use skillspec::{domain::authoring, error::Result, report};

pub(super) fn run(command: DepsCommand) -> Result<()> {
    match command {
        DepsCommand::Check { path, command } => {
            let report = authoring::check_deps(&path, command.as_deref())?;
            report::json(&report)?;
            if !report.ok {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
