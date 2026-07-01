use crate::cli::args::SkillsCommand;
use skillspec::{domain::harness, error::Result, report};

pub(super) fn run(command: SkillsCommand) -> Result<()> {
    match command {
        SkillsCommand::Audit { roots, json } => {
            let report = harness::audit_skills(&roots)?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_skill_audit(&report))?;
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
            let report = harness::set_visibility(harness::SetVisibilityOptions {
                roots,
                skill,
                visibility: visibility.into(),
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_visibility_apply(&report))?;
            }
        }
        SkillsCommand::Disable {
            skill,
            roots,
            manifest,
            dry_run,
            json,
        } => {
            let report = harness::set_visibility(harness::SetVisibilityOptions {
                roots,
                skill,
                visibility: harness::Visibility::Off,
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_visibility_apply(&report))?;
            }
        }
        SkillsCommand::Enable {
            skill,
            roots,
            manifest,
            dry_run,
            json,
        } => {
            let report = harness::set_visibility(harness::SetVisibilityOptions {
                roots,
                skill,
                visibility: harness::Visibility::Implicit,
                manifest,
                dry_run,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&harness::render_visibility_apply(&report))?;
            }
        }
    }

    Ok(())
}
