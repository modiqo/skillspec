use crate::cli::args::InstallCommand;
use skillspec::{error::Result, install, install::HarnessTarget, report};

pub(super) fn run(command: InstallCommand) -> Result<()> {
    match command {
        InstallCommand::Targets => {
            let targets = install::detect_targets()?;
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
                .map(HarnessTarget::from)
                .collect::<Vec<_>>();
            let report = install::install_skill(
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
