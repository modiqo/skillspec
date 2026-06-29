use crate::cli::args::{GuideModeArg, SenseViewArg};
use skillspec::error::Result;
use skillspec::{act, decision, error, guide, model, parser, report, run_loop, sensemake, trace};
use std::path::PathBuf;
use std::time::Instant;

pub(super) fn validate(path: PathBuf) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    report::validation_ok(&path, &spec)
}

pub(super) fn test(path: PathBuf) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    let result = decision::run_tests(&spec);
    report::test_result(&result)?;
    if !result.failed.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

pub(super) fn decide(path: PathBuf, input: String, trace_dir: Option<PathBuf>) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    ensure_trace_available(&spec, trace_dir.as_ref())?;
    let decision = decision::decide_with_events(&spec, &input);
    if let Some(trace_dir) = trace_dir {
        let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
        report::trace_written(&trace)?;
    }
    report::json(&decision.decision)
}

pub(super) fn act(
    path: PathBuf,
    input: String,
    trace_dir: Option<PathBuf>,
    run: Option<PathBuf>,
    phase: Option<String>,
    json: bool,
) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    ensure_trace_available(&spec, trace_dir.as_ref().or(run.as_ref()))?;
    let decision = decision::decide_with_events(&spec, &input);
    let trace = if let Some(trace_dir) = trace_dir {
        let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
        report::trace_written(&trace)?;
        Some(trace)
    } else {
        None
    };
    let mut act_report =
        act::build_report_for_phase(&spec, &decision.decision, trace.as_ref(), phase.as_deref())?;
    if let Some(run) = run {
        act_report.trace = Some(act::trace_for_run(&run));
    }
    if json {
        report::json(&act_report)
    } else {
        report::text(&act::render(&act_report))
    }
}

pub(super) fn plan(
    path: PathBuf,
    input: String,
    trace_dir: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    ensure_trace_available(&spec, trace_dir.as_ref())?;
    let decision = decision::decide_with_events(&spec, &input);
    let trace = if let Some(trace_dir) = trace_dir {
        let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
        report::trace_written(&trace)?;
        Some(trace)
    } else {
        None
    };
    let act_report = act::build_report(&spec, &decision.decision, trace.as_ref());
    if json {
        report::json(&act_report)
    } else {
        report::text(&act::render_plan(&act_report))
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
    let started = Instant::now();
    let spec = parser::load_spec(&path)?;
    if let Some(guide_mode_arg) = guide_mode_arg {
        if resume.is_none() {
            ensure_trace_available(&spec, trace_dir.as_ref())?;
        }
        let guide_report = guide::build_report(guide::BuildOptions {
            spec: &spec,
            spec_path: &path,
            input: input.as_deref(),
            resume_run_dir: resume.as_deref(),
            trace_dir: trace_dir.as_deref(),
            phase_override: phase.as_deref(),
            guide_mode: guide_mode(guide_mode_arg),
        })?;
        if json {
            report::json(&guide_report)
        } else {
            report::text(&guide::render_text(&guide_report))
        }
    } else {
        let input = input.ok_or_else(|| error::Error::InvalidInput {
            message: "run-loop requires --input unless --guide --resume is used".to_owned(),
        })?;
        if resume.is_some() {
            return Err(error::Error::InvalidInput {
                message: "run-loop --resume requires --guide".to_owned(),
            });
        }
        ensure_trace_available(&spec, trace_dir.as_ref())?;
        let run_loop_report = run_loop::build_report(
            &spec,
            &path,
            &input,
            view.into(),
            trace_dir.as_deref(),
            phase.as_deref(),
        )?;
        let elapsed = started.elapsed();
        if json {
            report::json(&run_loop_report)
        } else {
            report::text(&run_loop::render_summary(&run_loop_report, elapsed))
        }
    }
}

pub(super) fn explain(path: PathBuf, input: String, trace_dir: Option<PathBuf>) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    ensure_trace_available(&spec, trace_dir.as_ref())?;
    let decision = decision::decide_with_events(&spec, &input);
    if let Some(trace_dir) = trace_dir {
        let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
        report::trace_written(&trace)?;
    }
    report::explain(&decision.decision)
}

pub(super) fn sensemake(path: PathBuf, view: SenseViewArg, json: bool) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    let report_value = sensemake::sensemake(&spec, &path, view.into());
    if json {
        report::json(&report_value)
    } else {
        report::text(&sensemake::render_sensemake(&report_value))
    }
}

pub(super) fn query(path: PathBuf, handle: String, view: SenseViewArg, json: bool) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    let report_value = sensemake::query(&spec, &path, &handle, view.into())?;
    if json {
        report::json(&report_value)
    } else {
        report::text(&sensemake::render_query(&report_value))
    }
}

pub(super) fn refs(path: PathBuf, handle: String, view: SenseViewArg, json: bool) -> Result<()> {
    let spec = parser::load_spec(&path)?;
    let report_value = sensemake::refs(&spec, &path, &handle, view.into())?;
    if json {
        report::json(&report_value)
    } else {
        report::text(&sensemake::render_refs(&report_value))
    }
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

fn guide_mode(mode: GuideModeArg) -> guide::GuideMode {
    match mode {
        GuideModeArg::Agent => guide::GuideMode::Agent,
        GuideModeArg::Full => guide::GuideMode::Full,
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
