use crate::cli::args::VisibilityCommand;
use skillspec::{error::Result, report, visibility};

pub(super) fn run(command: VisibilityCommand) -> Result<()> {
    match command {
        VisibilityCommand::Plan {
            roots,
            profile,
            json,
        } => {
            let report = visibility::plan(visibility::VisibilityPlanOptions {
                roots,
                profile: profile.into(),
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&visibility::render_plan(&report))?;
            }
        }
        VisibilityCommand::Apply {
            roots,
            profile,
            manifest,
            dry_run,
            json,
        } => {
            let report = visibility::apply(visibility::VisibilityApplyOptions {
                roots,
                profile: profile.into(),
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&visibility::render_apply(&report))?;
            }
        }
        VisibilityCommand::Restore {
            manifest,
            dry_run,
            json,
        } => {
            let report =
                visibility::restore(visibility::VisibilityRestoreOptions { manifest, dry_run })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&visibility::render_restore(&report))?;
            }
        }
    }

    Ok(())
}
