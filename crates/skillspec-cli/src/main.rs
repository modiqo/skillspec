mod align;
mod capability;
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
mod sensemake;
mod trace;

use clap::{Parser, Subcommand, ValueEnum};
use error::Result;
use install::HarnessTarget;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "skillspec")]
#[command(about = "Structured skills for agent behavior")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
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
    #[command(about = "Teach the SkillSpec grammar map and progressive navigation handles")]
    Sensemake {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = SenseViewArg::Index)]
        view: SenseViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Query one SkillSpec collection, item, or field path")]
    Query {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Query handle, such as routes, rule:<id>, or command:<id>.requires.
        handle: String,
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = SenseViewArg::Summary)]
        view: SenseViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show outgoing SkillSpec references for an item handle")]
    Refs {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Item handle, such as rule:<id>, command:<id>, state:<id>, or recipe:<id>.
        handle: String,
        /// Output detail level.
        #[arg(long, value_enum, default_value_t = SenseViewArg::Summary)]
        view: SenseViewArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Inspect, compact, or align SkillSpec decision traces")]
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
    #[command(about = "Manage local capability seeds for durable bootstrap")]
    Capability {
        #[command(subcommand)]
        command: CapabilityCommand,
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
        /// JSONL execution ledger with sanitized action evidence. Repeat for multiple ledgers.
        #[arg(long)]
        execution_trace: Vec<PathBuf>,
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
        /// Overwrite an existing installed skill folder without prompting.
        #[arg(long)]
        force: bool,
        /// Override the installed skill folder name.
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum CapabilityCommand {
    #[command(about = "Show the local capability seed store path")]
    Store,
    #[command(about = "Create or update a local capability seed")]
    Add {
        /// Stable seed id, such as preferred-voice-cli.
        id: String,
        /// Capability domain folder, such as voice or pdf.
        #[arg(long)]
        domain: String,
        /// Seed kind, such as cli, adapter, script, or flow.
        #[arg(long)]
        kind: String,
        /// CLI command name or path.
        #[arg(long)]
        command: Option<String>,
        /// Adapter id or name.
        #[arg(long)]
        adapter: Option<String>,
        /// Local script path.
        #[arg(long)]
        script: Option<String>,
        /// Capability provided by this seed. Repeat for multiple capabilities.
        #[arg(long)]
        provides: Vec<String>,
        /// User phrase alias for this seed. Repeat for multiple aliases.
        #[arg(long)]
        alias: Vec<String>,
        /// Default priority from 0 to 100, used only as a tie-breaker.
        #[arg(long)]
        priority: Option<u8>,
        /// Capability this seed is preferred for. Repeat for multiple capabilities.
        #[arg(long)]
        preferred_for: Vec<String>,
        /// Capability this seed should avoid. Repeat for multiple capabilities.
        #[arg(long)]
        avoid_for: Vec<String>,
        /// Tie-breaker metadata as key=value. Repeat for multiple entries.
        #[arg(long = "tie")]
        ties: Vec<String>,
        /// Environment variable used for auth. Repeat for multiple vars.
        #[arg(long)]
        auth_env: Vec<String>,
        /// Mark this seed as using an external service.
        #[arg(long)]
        external_service: bool,
        /// Mark this seed as potentially spending provider credits or money.
        #[arg(long)]
        may_cost_money: bool,
        /// Evidence command, such as "tool --help". Repeat for multiple checks.
        #[arg(long)]
        evidence_command: Vec<String>,
        /// Suggested domain SkillSpec id to generate after a successful trace.
        #[arg(long)]
        suggested_skill_id: Option<String>,
    },
    #[command(
        about = "Patch an existing local capability seed without rewriting unspecified fields"
    )]
    Update {
        /// Seed id to update.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Replace seed kind.
        #[arg(long)]
        kind: Option<String>,
        /// Set CLI command name or path.
        #[arg(long)]
        command: Option<String>,
        /// Clear CLI command.
        #[arg(long)]
        clear_command: bool,
        /// Set adapter id or name.
        #[arg(long)]
        adapter: Option<String>,
        /// Clear adapter id or name.
        #[arg(long)]
        clear_adapter: bool,
        /// Set local script path.
        #[arg(long)]
        script: Option<String>,
        /// Clear local script path.
        #[arg(long)]
        clear_script: bool,
        /// Add a capability provided by this seed. Repeat for multiple capabilities.
        #[arg(long)]
        add_provides: Vec<String>,
        /// Remove a provided capability. Repeat for multiple capabilities.
        #[arg(long)]
        remove_provides: Vec<String>,
        /// Add a user phrase alias. Repeat for multiple aliases.
        #[arg(long)]
        add_alias: Vec<String>,
        /// Remove a user phrase alias. Repeat for multiple aliases.
        #[arg(long)]
        remove_alias: Vec<String>,
        /// Set default priority from 0 to 100.
        #[arg(long)]
        priority: Option<u8>,
        /// Clear default priority.
        #[arg(long)]
        clear_priority: bool,
        /// Add a preferred capability. Repeat for multiple capabilities.
        #[arg(long)]
        add_preferred_for: Vec<String>,
        /// Remove a preferred capability. Repeat for multiple capabilities.
        #[arg(long)]
        remove_preferred_for: Vec<String>,
        /// Add an avoided capability. Useful when a seed stops working for a task.
        #[arg(long)]
        add_avoid_for: Vec<String>,
        /// Remove an avoided capability.
        #[arg(long)]
        remove_avoid_for: Vec<String>,
        /// Add or replace tie-breaker metadata as key=value. Repeat for multiple entries.
        #[arg(long)]
        add_tie: Vec<String>,
        /// Remove tie-breaker metadata by key. Repeat for multiple entries.
        #[arg(long)]
        remove_tie: Vec<String>,
        /// Add an auth environment variable. Repeat for multiple vars.
        #[arg(long)]
        add_auth_env: Vec<String>,
        /// Remove an auth environment variable. Repeat for multiple vars.
        #[arg(long)]
        remove_auth_env: Vec<String>,
        /// Set external service risk flag.
        #[arg(long)]
        external_service: Option<bool>,
        /// Set provider cost risk flag.
        #[arg(long)]
        may_cost_money: Option<bool>,
        /// Add evidence command, such as "tool --help". Repeat for multiple checks.
        #[arg(long)]
        add_evidence_command: Vec<String>,
        /// Remove an evidence command. Repeat for multiple checks.
        #[arg(long)]
        remove_evidence_command: Vec<String>,
        /// Set suggested domain SkillSpec id to generate after a successful trace.
        #[arg(long)]
        suggested_skill_id: Option<String>,
        /// Clear suggested domain SkillSpec id.
        #[arg(long)]
        clear_suggested_skill_id: bool,
        /// Mark verification status unverified without running checks.
        #[arg(long, conflicts_with = "mark_failed")]
        mark_unverified: bool,
        /// Mark verification status failed without running checks.
        #[arg(long, conflicts_with = "mark_unverified")]
        mark_failed: bool,
    },
    #[command(about = "List local capability seeds")]
    List {
        /// Limit results to one domain.
        #[arg(long)]
        domain: Option<String>,
    },
    #[command(about = "Search and rank local capability seeds for one capability/domain pair")]
    Search {
        /// Capability to search for, such as text_to_speech.
        capability: String,
        /// Limit results to one domain. If no candidates are found, callers should search related domains before using an unseeded fallback.
        #[arg(long)]
        domain: Option<String>,
        /// Include ranking reasons in the JSON output.
        #[arg(long)]
        explain: bool,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
        /// Exclude external service candidates.
        #[arg(long)]
        local_only: bool,
        /// Explicitly preferred seed id for this search.
        #[arg(long)]
        preferred_seed: Option<String>,
    },
    #[command(about = "Inspect one local capability seed")]
    Inspect {
        /// Seed id to inspect.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Verify one local capability seed's evidence")]
    Verify {
        /// Seed id to verify.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Update preferred capability and priority metadata for a seed")]
    Prefer {
        /// Seed id to update.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Capability this seed should be preferred for.
        #[arg(long = "for")]
        for_capability: String,
        /// Default priority from 0 to 100.
        #[arg(long)]
        priority: Option<u8>,
    },
    #[command(about = "Remove one local capability seed")]
    Remove {
        /// Seed id to remove.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
    },
    #[command(about = "Scan for seed proposals")]
    Scan,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum InstallTargetArg {
    Agents,
    Codex,
    ClaudeLocal,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SenseViewArg {
    Index,
    Summary,
    Full,
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
        Command::Sensemake { path, view, json } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::sensemake(&spec, &path, view.into());
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_sensemake(&report))?;
            }
        }
        Command::Query {
            path,
            handle,
            view,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::query(&spec, &path, &handle, view.into())?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_query(&report))?;
            }
        }
        Command::Refs {
            path,
            handle,
            view,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::refs(&spec, &path, &handle, view.into())?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_refs(&report))?;
            }
        }
        Command::Trace { command } => match command {
            TraceCommand::Compact { run_dir } => {
                let trace = trace::compact(&run_dir)?;
                report::json(&trace)?;
            }
            TraceCommand::Align {
                path,
                decision_trace,
                execution_trace,
                json,
            } => {
                let spec = parser::load_spec(&path)?;
                let report =
                    align::align_decision_trace(&spec, &path, &decision_trace, &execution_trace)?;
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
            std::io::stdout().lock().write_all(markdown.as_bytes())?;
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
                force,
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
                    force,
                    name.as_deref(),
                )?;
                report::json(&report)?;
            }
        },
        Command::Capability { command } => match command {
            CapabilityCommand::Store => {
                report::json(&capability::store()?)?;
            }
            CapabilityCommand::Add {
                id,
                domain,
                kind,
                command,
                adapter,
                script,
                provides,
                alias,
                priority,
                preferred_for,
                avoid_for,
                ties,
                auth_env,
                external_service,
                may_cost_money,
                evidence_command,
                suggested_skill_id,
            } => {
                let report = capability::add(capability::AddOptions {
                    id,
                    domain,
                    kind,
                    command,
                    adapter,
                    script,
                    provides,
                    aliases: alias,
                    priority,
                    preferred_for,
                    avoid_for,
                    ties,
                    auth_env,
                    external_service,
                    may_cost_money,
                    evidence_command,
                    suggested_skill_id,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Update {
                id,
                domain,
                kind,
                command,
                clear_command,
                adapter,
                clear_adapter,
                script,
                clear_script,
                add_provides,
                remove_provides,
                add_alias,
                remove_alias,
                priority,
                clear_priority,
                add_preferred_for,
                remove_preferred_for,
                add_avoid_for,
                remove_avoid_for,
                add_tie,
                remove_tie,
                add_auth_env,
                remove_auth_env,
                external_service,
                may_cost_money,
                add_evidence_command,
                remove_evidence_command,
                suggested_skill_id,
                clear_suggested_skill_id,
                mark_unverified,
                mark_failed,
            } => {
                let verification_status = if mark_failed {
                    Some(capability::VerificationStatus::Failed)
                } else if mark_unverified {
                    Some(capability::VerificationStatus::Unverified)
                } else {
                    None
                };
                let report = capability::update(capability::UpdateOptions {
                    id,
                    domain,
                    kind,
                    command,
                    clear_command,
                    adapter,
                    clear_adapter,
                    script,
                    clear_script,
                    add_provides,
                    remove_provides,
                    add_alias,
                    remove_alias,
                    priority,
                    clear_priority,
                    add_preferred_for,
                    remove_preferred_for,
                    add_avoid_for,
                    remove_avoid_for,
                    add_ties: add_tie,
                    remove_tie,
                    add_auth_env,
                    remove_auth_env,
                    external_service,
                    may_cost_money,
                    add_evidence_command,
                    remove_evidence_command,
                    suggested_skill_id,
                    clear_suggested_skill_id,
                    verification_status,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::List { domain } => {
                report::json(&capability::list(domain.as_deref())?)?;
            }
            CapabilityCommand::Search {
                capability: capability_id,
                domain,
                explain: _,
                json: _,
                local_only,
                preferred_seed,
            } => {
                let report = capability::search(capability::SearchOptions {
                    capability: capability_id,
                    domain,
                    local_only,
                    preferred_seed,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Inspect {
                id,
                domain,
                json: _,
            } => {
                report::json(&capability::inspect(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Verify {
                id,
                domain,
                json: _,
            } => {
                report::json(&capability::verify(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Prefer {
                id,
                domain,
                for_capability,
                priority,
            } => {
                let report = capability::prefer(capability::PreferOptions {
                    id,
                    domain,
                    for_capability,
                    priority,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Remove { id, domain } => {
                report::json(&capability::remove(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Scan => {
                report::json(&capability::scan()?)?;
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

impl From<SenseViewArg> for sensemake::View {
    fn from(value: SenseViewArg) -> Self {
        match value {
            SenseViewArg::Index => Self::Index,
            SenseViewArg::Summary => Self::Summary,
            SenseViewArg::Full => Self::Full,
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
