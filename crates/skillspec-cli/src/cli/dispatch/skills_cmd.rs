use crate::cli::args::SkillsCommand;
use skillspec::{error::Result, report, router, visibility};

pub(super) fn run(command: SkillsCommand) -> Result<()> {
    match command {
        SkillsCommand::Audit { roots, json } => {
            let report = router::audit(&roots)?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router::render_audit(&report))?;
            }
        }
        SkillsCommand::SetVisibility {
            skill,
            visibility,
            roots,
            manifest,
            dry_run,
            json,
        } => {
            let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                roots,
                skill,
                visibility: visibility.into(),
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&visibility::render_apply(&report))?;
            }
        }
        SkillsCommand::Disable {
            skill,
            roots,
            manifest,
            dry_run,
            json,
        } => {
            let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                roots,
                skill,
                visibility: router::Visibility::Off,
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&visibility::render_apply(&report))?;
            }
        }
        SkillsCommand::Enable {
            skill,
            roots,
            manifest,
            dry_run,
            json,
        } => {
            let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                roots,
                skill,
                visibility: router::Visibility::Implicit,
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&visibility::render_apply(&report))?;
            }
        }
    }

    Ok(())
}
