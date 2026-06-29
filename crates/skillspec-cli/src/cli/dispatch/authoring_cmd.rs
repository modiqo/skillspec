use crate::cli::args::{CompileTarget, RouterExecutionModeArg};
use skillspec::error::Result;
use skillspec::{
    compiler, error, importer, parser, port_one_shot, report, router, router_lifecycle, source_map,
    workspace, workspace_synthesizer,
};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

pub(super) fn compile(path: PathBuf, target: CompileTarget) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    let markdown = compiler::compile(&spec, target.into());
    std::io::stdout().lock().write_all(markdown.as_bytes())?;
    Ok(())
}

pub(super) fn import_skill(path: PathBuf, out: PathBuf, source_map: Option<PathBuf>) -> Result<()> {
    workspace::guard_single_skill_source(&path, "skillspec import-skill")?;
    if let Some(source_map_path) = source_map {
        let source_root = source_map::source_root_for(&path);
        let stale_report = source_map::stale(&source_map_path, Some(&source_root))?;
        if !stale_report.ok {
            return Err(error::Error::InvalidInput {
                message: format!(
                    "source map {} is stale for {}; rerun `skillspec source map {} --out <map-dir>` before import",
                    source_map_path.display(),
                    source_root.display(),
                    path.display()
                ),
            });
        }
    }
    let imported = importer::import_skill_for_output(&path, &out)?;
    parser::write_spec(&out, &imported)?;
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
    let started = Instant::now();
    let mut port_report = port_one_shot::run(port_one_shot::PortOneShotOptions {
        source,
        out,
        target: target.into(),
        prove,
        force,
        run_dir,
        phase,
        requirements,
    })?;
    let elapsed = started.elapsed();
    let preview = port_one_shot::render_summary(&port_report, elapsed);
    port_one_shot::record_estimated_stats(&mut port_report, elapsed, preview.len() as u64)?;
    let rendered = port_one_shot::render_summary(&port_report, elapsed);
    port_one_shot::write_report(&port_report, &rendered)?;
    if json {
        report::json(&port_report)?;
    } else {
        report::text(&rendered)?;
    }
    if prove && !port_report.ok {
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
    let synthesis = workspace_synthesizer::synthesize_from_workspace(
        workspace_synthesizer::SynthesizeOptions {
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
        },
    )?;
    if json {
        report::json(&synthesis)
    } else {
        report::text(&workspace_synthesizer::render_report(&synthesis))
    }
}

pub(super) fn index(
    roots: Vec<PathBuf>,
    out: PathBuf,
    visibility_manifest: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let mut report_value = router::index(router::IndexOptions {
        roots,
        out,
        visibility_manifest,
    })?;
    report_value
        .warnings
        .extend(router_lifecycle::direct_index_warnings());
    if json {
        report::json(&report_value)
    } else {
        report::text(&router::render_index(&report_value))
    }
}

pub(super) fn route(
    index: PathBuf,
    query: String,
    top: usize,
    execution_mode: Option<RouterExecutionModeArg>,
    json: bool,
) -> Result<()> {
    let route_report = router::route(router::RouteOptions {
        index,
        query,
        top,
        execution_mode: execution_mode.map(Into::into),
    })?;
    if json {
        report::json(&route_report)
    } else {
        report::text(&router::render_route(&route_report))
    }
}
