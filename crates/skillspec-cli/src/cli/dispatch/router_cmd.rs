use crate::cli::args::{
    RouterCommand, RouterIndexCommand, RouterPolicyCommand, RouterProfileCommand,
};
use skillspec::{
    domain::{authoring, harness},
    error::Result,
    report,
};

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
        RouterCommand::Guard {
            config,
            hook,
            harness: current_harness,
            json,
        } => {
            let report = harness::guard_router(harness::RouterGuardOptions {
                config,
                hook,
                current_harness: current_harness.map(Into::into),
            })?;
            if hook {
                report::text(&harness::render_router_guard_hook_json(&report)?)?;
            } else if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_router_guard(&report))?;
            }
        }
        RouterCommand::Index { command } => run_index(command)?,
        RouterCommand::Policy { command } => run_policy(command)?,
        RouterCommand::Profile { command } => run_profile(command)?,
    }

    Ok(())
}

fn run_index(command: RouterIndexCommand) -> Result<()> {
    match command {
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
    }
    Ok(())
}

fn run_policy(command: RouterPolicyCommand) -> Result<()> {
    match command {
        RouterPolicyCommand::Init { index, json } => {
            let output = harness::router_policy_init(harness::PolicyInitOptions { index })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_policy_init(&output))?;
            }
        }
        RouterPolicyCommand::List { index, json } => {
            let output = harness::router_policy_list(harness::PolicyListOptions { index })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_policy_list(&output))?;
            }
        }
        RouterPolicyCommand::Show {
            index,
            profile,
            json,
        } => {
            let output =
                harness::router_policy_show(harness::PolicyShowOptions { index, profile })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_policy_show(&output))?;
            }
        }
        RouterPolicyCommand::Get { id, index, json } => {
            let output = harness::router_policy_get(harness::PolicyGetOptions { index, id })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_policy_get(&output))?;
            }
        }
        RouterPolicyCommand::SetProfile {
            name,
            index,
            mode,
            active,
            strict,
            description,
            json,
        } => {
            let output = harness::router_policy_set_profile(harness::PolicySetProfileOptions {
                index,
                name,
                mode: mode.into(),
                active,
                strict,
                description,
            })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_policy_set_profile(&output))?;
            }
        }
        RouterPolicyCommand::SetRule {
            id,
            index,
            profile,
            priority,
            mode,
            anchor,
            enabled,
            when_any,
            when_all,
            when_none,
            prefer,
            allow,
            suppress,
            forbid,
            json,
        } => {
            let output = harness::router_policy_set_rule(harness::PolicySetRuleOptions {
                index,
                id,
                profile,
                priority,
                mode: mode.into(),
                anchor: anchor.into(),
                enabled,
                when_any,
                when_all,
                when_none,
                prefer,
                allow,
                suppress,
                forbid,
            })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_policy_set_rule(&output))?;
            }
        }
        RouterPolicyCommand::RemoveRule { id, index, json } => {
            let output =
                harness::router_policy_remove_rule(harness::PolicyRemoveRuleOptions { index, id })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_policy_remove_rule(&output))?;
            }
        }
        RouterPolicyCommand::Explain {
            index,
            query,
            profile,
            top,
            json,
        } => {
            let output = authoring::route(authoring::RouteOptions {
                index,
                query,
                top,
                profile,
                execution_mode: None,
                current_harness: None,
                current_root: None,
            })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&authoring::render_route(&output))?;
            }
        }
    }
    Ok(())
}

fn run_profile(command: RouterProfileCommand) -> Result<()> {
    match command {
        RouterProfileCommand::Status { index, json } => {
            let output = harness::router_profile_status(harness::ProfileStatusOptions { index })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_profile_status(&output))?;
            }
        }
        RouterProfileCommand::Apply {
            profile,
            index,
            dry_run,
            json,
        } => {
            let output = harness::router_profile_apply(harness::ProfileApplyOptions {
                index,
                profile,
                dry_run,
            })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_profile_apply(&output))?;
            }
        }
        RouterProfileCommand::Clear {
            index,
            dry_run,
            json,
        } => {
            let output =
                harness::router_profile_clear(harness::ProfileClearOptions { index, dry_run })?;
            if json {
                report::json(&output)?;
            } else {
                report::text(&harness::render_router_profile_clear(&output))?;
            }
        }
    }
    Ok(())
}
