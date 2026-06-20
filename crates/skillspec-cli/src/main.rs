mod compiler;
mod decision;
mod error;
mod importer;
mod model;
mod parser;
mod report;

use clap::{Parser, Subcommand};
use error::Result;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "skillspec")]
#[command(about = "Structured skills for agent behavior")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Validate {
        path: PathBuf,
    },
    Test {
        path: PathBuf,
    },
    Decide {
        path: PathBuf,
        #[arg(long)]
        input: String,
    },
    Explain {
        path: PathBuf,
        #[arg(long)]
        input: String,
    },
    Compile {
        path: PathBuf,
        #[arg(long)]
        target: CompileTarget,
    },
    ImportSkill {
        path: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum CompileTarget {
    CodexSkill,
    ClaudeSkill,
    Markdown,
}

fn main() {
    if let Err(error) = run() {
        report::error(error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate { path } => {
            let spec = parser::load_spec(&path)?;
            report::validation_ok(&path, &spec)?;
        }
        Command::Test { path } => {
            let spec = parser::load_spec(&path)?;
            let result = decision::run_tests(&spec);
            report::test_result(&result)?;
            if !result.failed.is_empty() {
                std::process::exit(1);
            }
        }
        Command::Decide { path, input } => {
            let spec = parser::load_spec(&path)?;
            let decision = decision::decide(&spec, &input);
            report::json(&decision)?;
        }
        Command::Explain { path, input } => {
            let spec = parser::load_spec(&path)?;
            let decision = decision::decide(&spec, &input);
            report::explain(&decision)?;
        }
        Command::Compile { path, target } => {
            let spec = parser::load_spec(&path)?;
            let markdown = compiler::compile(&spec, target.into());
            report::text(&markdown)?;
        }
        Command::ImportSkill { path, out } => {
            let imported = importer::import_skill(&path)?;
            parser::write_spec(&out, &imported)?;
            report::import_ok(&path, &out, &imported)?;
        }
    }

    Ok(())
}

impl From<CompileTarget> for compiler::Target {
    fn from(value: CompileTarget) -> Self {
        match value {
            CompileTarget::CodexSkill => compiler::Target::CodexSkill,
            CompileTarget::ClaudeSkill => compiler::Target::ClaudeSkill,
            CompileTarget::Markdown => compiler::Target::Markdown,
        }
    }
}
