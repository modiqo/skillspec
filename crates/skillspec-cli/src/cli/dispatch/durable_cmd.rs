use crate::cli::args::DurableExecutorCommand;
use skillspec::{domain::harness, error::Result, report};

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
                .map(harness::HarnessTarget::from)
                .collect::<Vec<_>>();
            let report = harness::install_durable(harness::DurableInstallOptions {
                source,
                targets,
                all_detected,
                dry_run,
                force,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_durable_install(&report))?;
            }
        }
        DurableExecutorCommand::Update {
            source,
            backup_dir,
            dry_run,
            json,
        } => {
            let report = harness::update_durable(harness::DurableUpdateOptions {
                source,
                backup_dir,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_durable_update(&report))?;
            }
        }
        DurableExecutorCommand::Delete { dry_run, json } => {
            let report = harness::delete_durable(harness::DurableDeleteOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_durable_delete(&report))?;
            }
        }
        DurableExecutorCommand::Enable { dry_run, json } => {
            let report = harness::enable_durable(harness::DurableModeOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_durable_mode(&report))?;
            }
        }
        DurableExecutorCommand::Disable { dry_run, json } => {
            let report = harness::disable_durable(harness::DurableModeOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_durable_mode(&report))?;
            }
        }
    }

    Ok(())
}
