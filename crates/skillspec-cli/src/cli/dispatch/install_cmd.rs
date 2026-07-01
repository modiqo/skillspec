use crate::cli::args::InstallCommand;
use skillspec::{domain::harness, error::Result, report};

pub(super) fn run(command: InstallCommand) -> Result<()> {
    match command {
        InstallCommand::Targets => {
            let targets = harness::detect_targets()?;
            report::json(&targets)?;
        }
        InstallCommand::Skill {
            folder,
            target,
            all_detected,
            dry_run,
            force,
            retire_existing,
            name,
        } => {
            let targets = target
                .into_iter()
                .map(harness::HarnessTarget::from)
                .collect::<Vec<_>>();
            let report = harness::install_skill(
                &folder,
                &targets,
                all_detected,
                dry_run,
                force,
                retire_existing,
                name.as_deref(),
            )?;
            report::json(&report)?;
        }
    }

    Ok(())
}
