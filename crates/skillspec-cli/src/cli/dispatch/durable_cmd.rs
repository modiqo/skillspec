use crate::cli::args::DurableExecutorCommand;
use skillspec::{durable_lifecycle, error::Result, install::HarnessTarget, report};

pub(super) fn run(command: DurableExecutorCommand) -> Result<()> {
    match command {
        DurableExecutorCommand::Install {
            source,
            target,
            all_detected,
            dry_run,
            force,
            json,
        } => {
            let targets = target
                .into_iter()
                .map(HarnessTarget::from)
                .collect::<Vec<_>>();
            let report = durable_lifecycle::install(durable_lifecycle::DurableInstallOptions {
                source,
                targets,
                all_detected,
                dry_run,
                force,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&durable_lifecycle::render_install(&report))?;
            }
        }
        DurableExecutorCommand::Update {
            source,
            backup_dir,
            dry_run,
            json,
        } => {
            let report = durable_lifecycle::update(durable_lifecycle::DurableUpdateOptions {
                source,
                backup_dir,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&durable_lifecycle::render_update(&report))?;
            }
        }
        DurableExecutorCommand::Delete { dry_run, json } => {
            let report =
                durable_lifecycle::delete(durable_lifecycle::DurableDeleteOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&durable_lifecycle::render_delete(&report))?;
            }
        }
        DurableExecutorCommand::Enable { dry_run, json } => {
            let report =
                durable_lifecycle::enable(durable_lifecycle::DurableModeOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&durable_lifecycle::render_mode(&report))?;
            }
        }
        DurableExecutorCommand::Disable { dry_run, json } => {
            let report =
                durable_lifecycle::disable(durable_lifecycle::DurableModeOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&durable_lifecycle::render_mode(&report))?;
            }
        }
    }

    Ok(())
}
