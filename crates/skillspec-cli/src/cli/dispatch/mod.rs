use super::args::Command;
mod authoring_cmd;
mod capability_cmd;
mod checklist_cmd;
mod deps_cmd;
mod doctor_cmd;
mod durable_cmd;
mod grammar_cmd;
mod imports_cmd;
mod install_cmd;
mod progress_cmd;
mod router_cmd;
mod runtime_cmd;
mod skills_cmd;
mod source_cmd;
mod status_cmd;
mod trace_cmd;
mod visibility_cmd;
mod workspace_cmd;

use skillspec::error::Result;

pub(super) fn run(command: Command) -> Result<()> {
    match command {
        Command::Validate { path } => {
            runtime_cmd::validate(path)?;
        }
        Command::Test { path } => {
            runtime_cmd::test(path)?;
        }
        Command::Decide {
            path,
            input,
            trace_dir,
        } => {
            runtime_cmd::decide(path, input, trace_dir)?;
        }
        Command::Act {
            path,
            input,
            trace_dir,
            run,
            phase,
            json,
        } => {
            runtime_cmd::act(path, input, trace_dir, run, phase, json)?;
        }
        Command::Plan {
            path,
            input,
            trace_dir,
            json,
        } => {
            runtime_cmd::plan(path, input, trace_dir, json)?;
        }
        Command::RunLoop {
            path,
            input,
            resume,
            view,
            trace_dir,
            phase,
            guide: guide_mode_arg,
            json,
        } => {
            runtime_cmd::run_loop(
                path,
                input,
                resume,
                view,
                trace_dir,
                phase,
                guide_mode_arg,
                json,
            )?;
        }
        Command::Explain {
            path,
            input,
            trace_dir,
        } => {
            runtime_cmd::explain(path, input, trace_dir)?;
        }
        Command::Sensemake { path, view, json } => {
            runtime_cmd::sensemake(path, view, json)?;
        }
        Command::Query {
            path,
            handle,
            view,
            json,
        } => {
            runtime_cmd::query(path, handle, view, json)?;
        }
        Command::Refs {
            path,
            handle,
            view,
            json,
        } => {
            runtime_cmd::refs(path, handle, view, json)?;
        }
        Command::Doctor {
            command,
            path,
            json,
            html,
            markdown,
        } => match command {
            Some(command) => checklist_cmd::doctor(command)?,
            None => doctor_cmd::run(path, json, html, markdown)?,
        },
        Command::Import { command } => checklist_cmd::import(command)?,
        Command::Run { command } => checklist_cmd::run(command)?,
        Command::Status { roots, json } => {
            status_cmd::run(roots, json)?;
        }
        Command::Source { command } => source_cmd::run(command)?,
        Command::Workspace { command } => workspace_cmd::run(command)?,
        Command::Grammar { command } => grammar_cmd::run(command)?,
        Command::Trace { command } => trace_cmd::run(command)?,
        Command::Progress { command } => progress_cmd::run(command)?,
        Command::Deps { command } => deps_cmd::run(command)?,
        Command::Imports { command } => imports_cmd::run(command)?,
        Command::Compile { path, target } => {
            authoring_cmd::compile(path, target)?;
        }
        Command::ImportSkill {
            path,
            out,
            source_map,
        } => {
            authoring_cmd::import_skill(path, out, source_map)?;
        }
        Command::PortOneShot {
            source,
            out,
            target,
            prove,
            force,
            run_dir,
            phase,
            requirements,
            json,
        } => {
            authoring_cmd::port_one_shot(
                source,
                out,
                target,
                prove,
                force,
                run_dir,
                phase,
                requirements,
                json,
            )?;
        }
        Command::SynthesizeFromWorkspace {
            workspace,
            out,
            task,
            name,
            log_last,
            workspace_stats_report,
            workspace_log,
            workspace_meta,
            workspace_deps,
            observation_approved,
            force,
            json,
        } => {
            authoring_cmd::synthesize_from_workspace(
                workspace,
                out,
                task,
                name,
                log_last,
                workspace_stats_report,
                workspace_log,
                workspace_meta,
                workspace_deps,
                observation_approved,
                force,
                json,
            )?;
        }
        Command::Index {
            roots,
            out,
            visibility_manifest,
            json,
        } => {
            authoring_cmd::index(roots, out, visibility_manifest, json)?;
        }
        Command::Route {
            index,
            query,
            top,
            profile,
            execution_mode,
            current_harness,
            current_root,
            json,
        } => {
            authoring_cmd::route(authoring_cmd::RouteCommandOptions {
                index,
                query,
                top,
                profile,
                execution_mode,
                current_harness,
                current_root,
                json,
            })?;
        }
        Command::Skills { command } => skills_cmd::run(command)?,
        Command::Visibility { command } => visibility_cmd::run(command)?,
        Command::Router { command } => router_cmd::run(command)?,
        Command::DurableExecutor { command } => durable_cmd::run(command)?,
        Command::Install { command } => install_cmd::run(command)?,
        Command::Capability { command } => capability_cmd::run(command)?,
    }

    Ok(())
}
