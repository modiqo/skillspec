use crate::cli::args::{RouterCommand, RouterIndexCommand};
use skillspec::{domain::harness, error::Result, report};

pub(super) fn run(command: RouterCommand) -> Result<()> {
    match command {
        RouterCommand::Install {
            roots,
            index,
            manifest,
            router_name,
            dry_run,
            json,
        } => {
            let report = harness::install_router(harness::RouterInstallOptions {
                roots,
                index,
                manifest,
                router_name: Some(router_name),
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_router_install(&report))?;
            }
        }
        RouterCommand::Uninstall {
            manifest,
            router_name,
            index,
            keep_index,
            dry_run,
            json,
        } => {
            let report = harness::uninstall_router(harness::RouterUninstallOptions {
                manifest,
                router_name: Some(router_name),
                index,
                keep_index,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_router_uninstall(&report))?;
            }
        }
        RouterCommand::Update {
            backup_dir,
            dry_run,
            json,
        } => {
            let report = harness::update_router(harness::RouterUpdateOptions {
                backup_dir,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_router_update(&report))?;
            }
        }
        RouterCommand::Enable { dry_run, json } => {
            let report = harness::enable_router(harness::RouterModeOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_router_mode(&report))?;
            }
        }
        RouterCommand::Disable { dry_run, json } => {
            let report = harness::disable_router(harness::RouterModeOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_router_mode(&report))?;
            }
        }
        RouterCommand::Guard { config, hook, json } => {
            let report = harness::guard_router(harness::RouterGuardOptions { config, hook })?;
            if hook {
                report::text(&harness::render_router_guard_hook_json(&report)?)?;
            } else if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_router_guard(&report))?;
            }
        }
        RouterCommand::Index { command } => match command {
            RouterIndexCommand::Refresh {
                roots,
                index,
                visibility_manifest,
                json,
            } => {
                let report = harness::refresh_router_index(harness::RouterRefreshOptions {
                    roots,
                    index,
                    visibility_manifest,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&harness::render_router_refresh(&report))?;
                }
            }
            RouterIndexCommand::Status {
                roots,
                index,
                visibility_manifest,
                json,
            } => {
                let report = harness::router_index_status(harness::IndexStatusOptions {
                    roots,
                    index,
                    visibility_manifest,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&harness::render_router_index_status(&report))?;
                }
            }
        },
    }

    Ok(())
}
