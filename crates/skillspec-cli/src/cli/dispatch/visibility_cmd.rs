use crate::cli::args::VisibilityCommand;
use skillspec::{domain::harness, error::Result, report};

pub(super) fn run(command: VisibilityCommand) -> Result<()> {
    match command {
        VisibilityCommand::Plan {
            roots,
            profile,
            json,
        } => {
            let report = harness::plan_visibility(harness::VisibilityPlanOptions {
                roots,
                profile: profile.into(),
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_visibility_plan(&report))?;
            }
        }
        VisibilityCommand::Apply {
            roots,
            profile,
            manifest,
            dry_run,
            json,
        } => {
            let report = harness::apply_visibility(harness::VisibilityApplyOptions {
                roots,
                profile: profile.into(),
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_visibility_apply(&report))?;
            }
        }
        VisibilityCommand::Restore {
            manifest,
            dry_run,
            json,
        } => {
            let report = harness::restore_visibility(harness::VisibilityRestoreOptions {
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_visibility_restore(&report))?;
            }
        }
    }

    Ok(())
}
