use super::{GrammarChecklistForArg, GrammarViewArg};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum GrammarCommand {
    #[command(about = "Explain embedded grammar affordances progressively")]
    Sensemake {
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = GrammarViewArg::Index)]
        view: GrammarViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show a coverage checklist for semantic skill porting")]
    Checklist {
        /// Checklist workflow to show.
        #[arg(long = "for", value_enum, default_value_t = GrammarChecklistForArg::ImportSkill)]
        for_subject: GrammarChecklistForArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Print or describe the embedded SkillSpec JSON schema")]
    Schema {
        /// Emit the embedded JSON schema.
        #[arg(long)]
        json: bool,
    },
}
