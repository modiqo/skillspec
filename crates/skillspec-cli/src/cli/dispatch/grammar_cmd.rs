use crate::cli::args::GrammarCommand;
use skillspec::{domain::authoring, error::Result, report};

pub(super) fn run(command: GrammarCommand) -> Result<()> {
    match command {
        GrammarCommand::Sensemake { view, json } => {
            let report = authoring::grammar_sensemake(view.into());
            if json {
                report::json(&report)?;
            } else {
                report::text(&authoring::render_grammar_sensemake(&report))?;
            }
        }
        GrammarCommand::Checklist { for_subject, json } => {
            let report = authoring::grammar_checklist(for_subject.into());
            if json {
                report::json(&report)?;
            } else {
                report::text(&authoring::render_grammar_checklist(&report))?;
            }
        }
        GrammarCommand::Schema { json } => {
            if json {
                report::json(&authoring::grammar_schema_json()?)?;
            } else {
                report::text(&authoring::render_grammar_schema_summary())?;
            }
        }
    }

    Ok(())
}
