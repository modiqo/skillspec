mod compiler;
mod decision;
mod error;
mod importer;
mod model;
mod parser;
mod report;
mod trace;

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
        #[arg(long, allow_hyphen_values = true)]
        input: String,
        #[arg(long)]
        trace_dir: Option<PathBuf>,
    },
    Explain {
        path: PathBuf,
        #[arg(long, allow_hyphen_values = true)]
        input: String,
        #[arg(long)]
        trace_dir: Option<PathBuf>,
    },
    Trace {
        #[command(subcommand)]
        command: TraceCommand,
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

#[derive(Debug, Subcommand)]
enum TraceCommand {
    Compact { run_dir: PathBuf },
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
        Command::Decide {
            path,
            input,
            trace_dir,
        } => {
            let spec = parser::load_spec(&path)?;
            let decision = decision::decide_with_events(&spec, &input);
            if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &spec, &decision)?;
                report::trace_written(&trace)?;
            }
            report::json(&decision.decision)?;
        }
        Command::Explain {
            path,
            input,
            trace_dir,
        } => {
            let spec = parser::load_spec(&path)?;
            let decision = decision::decide_with_events(&spec, &input);
            if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &spec, &decision)?;
                report::trace_written(&trace)?;
            }
            report::explain(&decision.decision)?;
        }
        Command::Trace { command } => match command {
            TraceCommand::Compact { run_dir } => {
                let trace = trace::compact(&run_dir)?;
                report::json(&trace)?;
            }
        },
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
