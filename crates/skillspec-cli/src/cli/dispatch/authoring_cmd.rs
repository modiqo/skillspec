use crate::cli::args::{CompileTarget, RouteHarnessArg, RouterExecutionModeArg};
use skillspec::{domain::authoring, error::Result, report};
use std::io::Write;
use std::path::PathBuf;

pub(super) fn compile(path: PathBuf, target: CompileTarget) -> Result<()> {
    let markdown = authoring::compile(&path, target.into())?;
    std::io::stdout().lock().write_all(markdown.as_bytes())?;
    Ok(())
}

pub(super) fn import_skill(path: PathBuf, out: PathBuf, source_map: Option<PathBuf>) -> Result<()> {
    let imported = authoring::import_skill(&path, &out, source_map.as_deref())?;
    report::import_ok(&path, &out, &imported)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn port_one_shot(
    source: PathBuf,
    out: PathBuf,
    target: CompileTarget,
    prove: bool,
    force: bool,
    run_dir: Option<PathBuf>,
    phase: Option<String>,
    requirements: Vec<String>,
    json: bool,
) -> Result<()> {
    let output = authoring::port_one_shot(authoring::PortOneShotOptions {
        source,
        out,
        target: target.into(),
        prove,
        force,
        run_dir,
        phase,
        requirements,
    })?;
    if json {
        report::json(&output.report)?;
    } else {
        report::text(&output.rendered)?;
    }
    if prove && !output.report.ok {
        std::process::exit(1);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn synthesize_from_workspace(
    workspace: String,
    out: PathBuf,
    task: Option<String>,
    name: Option<String>,
    log_last: usize,
    workspace_stats_report: Option<PathBuf>,
    workspace_log: Option<PathBuf>,
    workspace_meta: Option<PathBuf>,
    workspace_deps: Option<PathBuf>,
    observation_approved: bool,
    force: bool,
    json: bool,
) -> Result<()> {
    let synthesis = authoring::synthesize_from_workspace(authoring::SynthesizeOptions {
        workspace,
        task,
        out,
        name,
        log_last,
        workspace_stats_report,
        workspace_log,
        workspace_meta,
        workspace_deps,
        observation_approved,
        force,
    })?;
    if json {
        report::json(&synthesis)
    } else {
        report::text(&authoring::render_synthesis(&synthesis))
    }
}

pub(super) fn index(
    roots: Vec<PathBuf>,
    out: PathBuf,
    visibility_manifest: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let report_value = authoring::index(authoring::IndexOptions {
        roots,
        out,
        visibility_manifest,
    })?;
    if json {
        report::json(&report_value)
    } else {
        report::text(&authoring::render_index(&report_value))
    }
}

pub(super) fn route(options: RouteCommandOptions) -> Result<()> {
    let route_report = authoring::route(authoring::RouteOptions {
        index: options.index,
        query: options.query,
        top: options.top,
        profile: options.profile,
        execution_mode: options.execution_mode.map(Into::into),
        current_harness: options.current_harness.map(Into::into),
        current_root: options.current_root,
    })?;
    if options.json {
        report::json(&route_report)
    } else {
        report::text(&authoring::render_route(&route_report))
    }
}

pub(super) struct RouteCommandOptions {
    pub(super) index: PathBuf,
    pub(super) query: String,
    pub(super) top: usize,
    pub(super) profile: Option<String>,
    pub(super) execution_mode: Option<RouterExecutionModeArg>,
    pub(super) current_harness: Option<RouteHarnessArg>,
    pub(super) current_root: Option<PathBuf>,
    pub(super) json: bool,
}
