use crate::cli::args::DepsCommand;
use skillspec::{deps, error::Result, parser, report};

pub(super) fn run(command: DepsCommand) -> Result<()> {
    match command {
        DepsCommand::Check { path, command } => {
            let spec = parser::load_spec(&path)?;
            let spec_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            let report = deps::check(&spec, spec_dir, command.as_deref())?;
            report::json(&report)?;
            if !report.ok {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
