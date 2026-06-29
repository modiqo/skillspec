use crate::cli::args::{RouterCommand, RouterIndexCommand};
use skillspec::{error::Result, report, router, router_lifecycle};

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
            let report = router_lifecycle::install(router_lifecycle::RouterInstallOptions {
                roots,
                index,
                manifest,
                router_name: Some(router_name),
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router_lifecycle::render_install(&report))?;
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
            let report = router_lifecycle::uninstall(router_lifecycle::RouterUninstallOptions {
                manifest,
                router_name: Some(router_name),
                index,
                keep_index,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router_lifecycle::render_uninstall(&report))?;
            }
        }
        RouterCommand::Update {
            backup_dir,
            dry_run,
            json,
        } => {
            let report = router_lifecycle::update(router_lifecycle::RouterUpdateOptions {
                backup_dir,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router_lifecycle::render_update(&report))?;
            }
        }
        RouterCommand::Enable { dry_run, json } => {
            let report = router_lifecycle::enable(router_lifecycle::RouterModeOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router_lifecycle::render_mode(&report))?;
            }
        }
        RouterCommand::Disable { dry_run, json } => {
            let report =
                router_lifecycle::disable(router_lifecycle::RouterModeOptions { dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router_lifecycle::render_mode(&report))?;
            }
        }
        RouterCommand::Guard { config, hook, json } => {
            let report =
                router_lifecycle::guard(router_lifecycle::RouterGuardOptions { config, hook })?;
            if hook {
                report::text(&router_lifecycle::render_guard_hook_json(&report)?)?;
            } else if json {
                report::json(&report)?;
            } else {
                report::text(&router_lifecycle::render_guard(&report))?;
            }
        }
        RouterCommand::Index { command } => match command {
            RouterIndexCommand::Refresh {
                roots,
                index,
                visibility_manifest,
                json,
            } => {
                let report = router_lifecycle::refresh(router_lifecycle::RouterRefreshOptions {
                    roots,
                    index,
                    visibility_manifest,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_refresh(&report))?;
                }
            }
            RouterIndexCommand::Status {
                roots,
                index,
                visibility_manifest,
                json,
            } => {
                let report = router::index_status(router::IndexStatusOptions {
                    roots,
                    index,
                    visibility_manifest,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router::render_index_status(&report))?;
                }
            }
        },
    }

    Ok(())
}
