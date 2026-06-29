use crate::cli::args::GrammarCommand;
use skillspec::{error::Result, grammar, report};

pub(super) fn run(command: GrammarCommand) -> Result<()> {
    match command {
        GrammarCommand::Sensemake { view, json } => {
            let report = grammar::sensemake(view.into());
            if json {
                report::json(&report)?;
            } else {
                report::text(&grammar::render_sensemake(&report))?;
            }
        }
        GrammarCommand::Checklist { for_subject, json } => {
            let report = grammar::checklist(for_subject.into());
            if json {
                report::json(&report)?;
            } else {
                report::text(&grammar::render_checklist(&report))?;
            }
        }
        GrammarCommand::Schema { json } => {
            if json {
                report::json(&grammar::schema_json()?)?;
            } else {
                report::text(&grammar::render_schema_summary())?;
            }
        }
    }

    Ok(())
}
