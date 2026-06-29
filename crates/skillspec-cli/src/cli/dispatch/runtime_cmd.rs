use crate::cli::args::{GuideModeArg, SenseViewArg};
use skillspec::error::Result;
use skillspec::{domain::runtime, guide, report};
use std::path::PathBuf;

pub(super) fn validate(path: PathBuf) -> Result<()> {
    let spec = runtime::validate(&path)?;
    report::validation_ok(&path, &spec)
}

pub(super) fn test(path: PathBuf) -> Result<()> {
    let result = runtime::test(&path)?;
    report::test_result(&result)?;
    if !result.failed.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

pub(super) fn decide(path: PathBuf, input: String, trace_dir: Option<PathBuf>) -> Result<()> {
    let decision = runtime::decide(&path, &input, trace_dir.as_deref())?;
    if let Some(trace) = &decision.trace {
        report::trace_written(&trace)?;
    }
    report::json(&decision.value.decision)
}

pub(super) fn act(
    path: PathBuf,
    input: String,
    trace_dir: Option<PathBuf>,
    run: Option<PathBuf>,
    phase: Option<String>,
    json: bool,
) -> Result<()> {
    let act_report = runtime::act(
        &path,
        &input,
        trace_dir.as_deref(),
        run.as_deref(),
        phase.as_deref(),
    )?;
    if let Some(trace) = &act_report.trace {
        report::trace_written(&trace)?;
    }
    if json {
        report::json(&act_report.value)
    } else {
        report::text(&runtime::render_act(&act_report.value))
    }
}

pub(super) fn plan(
    path: PathBuf,
    input: String,
    trace_dir: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let act_report = runtime::plan(&path, &input, trace_dir.as_deref())?;
    if let Some(trace) = &act_report.trace {
        report::trace_written(&trace)?;
    }
    if json {
        report::json(&act_report.value)
    } else {
        report::text(&runtime::render_plan(&act_report.value))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_loop(
    path: PathBuf,
    input: Option<String>,
    resume: Option<PathBuf>,
    view: SenseViewArg,
    trace_dir: Option<PathBuf>,
    phase: Option<String>,
    guide_mode_arg: Option<GuideModeArg>,
    json: bool,
) -> Result<()> {
    let output = runtime::run_loop(
        &path,
        input.as_deref(),
        resume.as_deref(),
        view.into(),
        trace_dir.as_deref(),
        phase.as_deref(),
        guide_mode_arg.map(guide_mode),
    )?;
    match output {
        runtime::RunLoopOutput::Guide(guide_report) if json => report::json(&guide_report),
        runtime::RunLoopOutput::Guide(guide_report) => {
            report::text(&runtime::render_guide(&guide_report))
        }
        runtime::RunLoopOutput::Summary {
            report: summary, ..
        } if json => report::json(&summary),
        runtime::RunLoopOutput::Summary {
            report: summary,
            elapsed,
        } => report::text(&runtime::render_run_loop(&summary, elapsed)),
    }
}

pub(super) fn explain(path: PathBuf, input: String, trace_dir: Option<PathBuf>) -> Result<()> {
    let decision = runtime::explain(&path, &input, trace_dir.as_deref())?;
    if let Some(trace) = &decision.trace {
        report::trace_written(&trace)?;
    }
    report::explain(&decision.value.decision)
}

pub(super) fn sensemake(path: PathBuf, view: SenseViewArg, json: bool) -> Result<()> {
    let report_value = runtime::sensemake(&path, view.into())?;
    if json {
        report::json(&report_value)
    } else {
        report::text(&runtime::render_sensemake(&report_value))
    }
}

pub(super) fn query(path: PathBuf, handle: String, view: SenseViewArg, json: bool) -> Result<()> {
    let report_value = runtime::query(&path, &handle, view.into())?;
    if json {
        report::json(&report_value)
    } else {
        report::text(&runtime::render_query(&report_value))
    }
}

pub(super) fn refs(path: PathBuf, handle: String, view: SenseViewArg, json: bool) -> Result<()> {
    let report_value = runtime::refs(&path, &handle, view.into())?;
    if json {
        report::json(&report_value)
    } else {
        report::text(&runtime::render_refs(&report_value))
    }
}

fn guide_mode(mode: GuideModeArg) -> guide::GuideMode {
    match mode {
        GuideModeArg::Agent => guide::GuideMode::Agent,
        GuideModeArg::Full => guide::GuideMode::Full,
    }
}
