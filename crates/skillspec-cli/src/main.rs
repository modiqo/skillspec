mod align;
mod compiler;
mod decision;
mod deps;
mod error;
mod importer;
mod imports;
mod install;
mod model;
mod parser;
mod report;
mod trace;

use clap::{Parser, Subcommand};
use error::Result;
use install::HarnessTarget;
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
    #[command(about = "Validate a skill.spec.yml file")]
    Validate {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
    },
    #[command(about = "Run scenario tests declared in a SkillSpec")]
    Test {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
    },
    #[command(about = "Evaluate routing rules for a user task and emit JSON")]
    Decide {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// User task text to route. Strip skill invocation prefixes before passing it.
        #[arg(long, allow_hyphen_values = true)]
        input: String,
        /// Directory where append-only decision trace events should be written.
        #[arg(long)]
        trace_dir: Option<PathBuf>,
    },
    #[command(about = "Explain routing decisions for a user task")]
    Explain {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// User task text to explain. Strip skill invocation prefixes before passing it.
        #[arg(long, allow_hyphen_values = true)]
        input: String,
        /// Directory where append-only decision trace events should be written.
        #[arg(long)]
        trace_dir: Option<PathBuf>,
    },
    #[command(about = "Inspect or compact SkillSpec decision traces")]
    Trace {
        #[command(subcommand)]
        command: TraceCommand,
    },
    #[command(about = "Check declared SkillSpec dependencies")]
    Deps {
        #[command(subcommand)]
        command: DepsCommand,
    },
    #[command(about = "Validate and report SkillSpec imports")]
    Imports {
        #[command(subcommand)]
        command: ImportsCommand,
    },
    #[command(about = "Compile a SkillSpec into harness guidance")]
    Compile {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Output target to render.
        #[arg(long)]
        target: CompileTarget,
    },
    #[command(about = "Create a mechanical draft SkillSpec from a local skill file or folder")]
    ImportSkill {
        /// Local SKILL.md file or skill folder to import.
        path: PathBuf,
        /// Output path for the generated skill.spec.yml draft.
        #[arg(long)]
        out: PathBuf,
    },
    #[command(about = "Detect harness roots and install SkillSpec-backed skills")]
    Install {
        #[command(subcommand)]
        command: InstallCommand,
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
    #[command(about = "Compact append-only trace events from a run directory into JSON")]
    Compact {
        /// Trace run directory produced by decide/explain --trace-dir.
        run_dir: PathBuf,
    },
    #[command(about = "Compare a SkillSpec to a decision trace")]
    Align {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Trace run directory produced by decide/explain --trace-dir.
        #[arg(long)]
        decision_trace: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum DepsCommand {
    #[command(about = "Check declared dependencies, optionally scoped to one command template")]
    Check {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Check only dependencies required by this command id.
        #[arg(long)]
        command: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ImportsCommand {
    #[command(about = "Check import paths, sections, nesting, and load order")]
    Check {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum InstallCommand {
    #[command(about = "List detected harness skill roots")]
    Targets,
    #[command(about = "Install a folder containing SKILL.md and skill.spec.yml")]
    Skill {
        /// Generated skill folder containing SKILL.md and skill.spec.yml.
        folder: PathBuf,
        /// Harness target to install into. Repeat for multiple targets.
        #[arg(long, value_enum)]
        target: Vec<InstallTargetArg>,
        /// Install into every harness root detected on this machine.
        #[arg(long)]
        all_detected: bool,
        /// Show the install plan without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Override the installed skill folder name.
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum InstallTargetArg {
    Agents,
    Codex,
    ClaudeLocal,
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
            ensure_trace_available(&spec, trace_dir.as_ref())?;
            let decision = decision::decide_with_events(&spec, &input);
            if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
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
            ensure_trace_available(&spec, trace_dir.as_ref())?;
            let decision = decision::decide_with_events(&spec, &input);
            if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
            }
            report::explain(&decision.decision)?;
        }
        Command::Trace { command } => match command {
            TraceCommand::Compact { run_dir } => {
                let trace = trace::compact(&run_dir)?;
                report::json(&trace)?;
            }
            TraceCommand::Align {
                path,
                decision_trace,
                json,
            } => {
                let spec = parser::load_spec(&path)?;
                let report = align::align_decision_trace(&spec, &path, &decision_trace)?;
                if json {
                    report::json(&report)?;
                } else {
                    report::align(&report)?;
                }
                if report.has_failures() {
                    std::process::exit(1);
                }
            }
        },
        Command::Deps { command } => match command {
            DepsCommand::Check { path, command } => {
                let spec = parser::load_spec(&path)?;
                let spec_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
                let report = deps::check(&spec, spec_dir, command.as_deref())?;
                report::json(&report)?;
                if !report.ok {
                    std::process::exit(1);
                }
            }
        },
        Command::Imports { command } => match command {
            ImportsCommand::Check { path } => {
                let spec = parser::load_spec_unresolved(&path)?;
                let report = imports::check(&spec, &path);
                report::json(&report)?;
                if !report.ok {
                    std::process::exit(1);
                }
            }
        },
        Command::Compile { path, target } => {
            let spec = parser::load_spec(&path)?;
            let markdown = compiler::compile(&spec, target.into());
            report::text(&markdown)?;
        }
        Command::ImportSkill { path, out } => {
            let imported = importer::import_skill_for_output(&path, &out)?;
            parser::write_spec(&out, &imported)?;
            report::import_ok(&path, &out, &imported)?;
        }
        Command::Install { command } => match command {
            InstallCommand::Targets => {
                let targets = install::detect_targets()?;
                report::json(&targets)?;
            }
            InstallCommand::Skill {
                folder,
                target,
                all_detected,
                dry_run,
                name,
            } => {
                let targets = target
                    .into_iter()
                    .map(HarnessTarget::from)
                    .collect::<Vec<_>>();
                let report = install::install_skill(
                    &folder,
                    &targets,
                    all_detected,
                    dry_run,
                    name.as_deref(),
                )?;
                report::json(&report)?;
            }
        },
    }

    Ok(())
}

fn ensure_trace_available(spec: &model::SkillSpec, trace_dir: Option<&PathBuf>) -> Result<()> {
    if spec
        .trace
        .as_ref()
        .is_some_and(|trace| trace.required && trace_dir.is_none())
    {
        return Err(error::Error::InvalidInput {
            message: "trace.required is true; pass --trace-dir or use a spec that does not require tracing"
                .to_owned(),
        });
    }
    Ok(())
}

impl From<InstallTargetArg> for HarnessTarget {
    fn from(value: InstallTargetArg) -> Self {
        match value {
            InstallTargetArg::Agents => Self::Agents,
            InstallTargetArg::Codex => Self::Codex,
            InstallTargetArg::ClaudeLocal => Self::ClaudeLocal,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_required_requires_trace_dir() {
        let yaml = r#"
schema: skillspec/v0
id: trace.required
title: Trace Required
description: Requires trace output.
routes:
  - id: local
    label: Local
trace:
  mode: event_log
  required: true
tests:
  - name: route assertion
    input: run this
    expect:
      route: local
"#;
        let spec = serde_yaml::from_str::<model::SkillSpec>(yaml).unwrap();
        let trace_dir = PathBuf::from(".skillspec/traces");

        assert!(ensure_trace_available(&spec, None).is_err());
        assert!(ensure_trace_available(&spec, Some(&trace_dir)).is_ok());
    }
}
