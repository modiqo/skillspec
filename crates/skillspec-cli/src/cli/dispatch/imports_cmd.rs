use crate::cli::args::ImportsCommand;
use skillspec::{error::Result, imports, parser, report};

pub(super) fn run(command: ImportsCommand) -> Result<()> {
    match command {
        ImportsCommand::Check { path } => {
            let spec = parser::load_spec_unresolved(&path)?;
            let report = imports::check(&spec, &path);
            report::json(&report)?;
            if !report.ok {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
