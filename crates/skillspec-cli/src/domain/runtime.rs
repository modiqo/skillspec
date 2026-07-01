use crate::{act, decision, error, guide, model, parser, run_loop, sensemake, trace};
use std::path::Path;
use std::time::{Duration, Instant};

pub struct Traced<T> {
    pub value: T,
    pub trace: Option<trace::TraceWriteResult>,
}

pub enum RunLoopOutput {
    Guide(Box<guide::GuideReport>),
    Summary {
        report: Box<run_loop::RunLoopReport>,
        elapsed: Duration,
    },
}

pub fn validate(path: &Path) -> error::Result<model::SkillSpec> {
    parser::load_spec(path)
}

pub fn test(path: &Path) -> error::Result<decision::TestRun> {
    let spec = parser::load_spec(path)?;
    Ok(decision::run_tests(&spec))
}

pub fn decide(
    path: &Path,
    input: &str,
    trace_dir: Option<&Path>,
) -> error::Result<Traced<decision::DecisionWithEvents>> {
    let spec = parser::load_spec(path)?;
    ensure_trace_available(&spec, trace_dir)?;
    let decision = decision::decide_with_events(&spec, input);
    let trace = match trace_dir {
        Some(trace_dir) => Some(trace::write_decision_trace(
            trace_dir, path, &spec, &decision,
        )?),
        None => None,
    };
    Ok(Traced {
        value: decision,
        trace,
    })
}

pub fn act(
    path: &Path,
    input: &str,
    trace_dir: Option<&Path>,
    run: Option<&Path>,
    phase: Option<&str>,
) -> error::Result<Traced<act::ActReport>> {
    let spec = parser::load_spec(path)?;
    ensure_trace_available(&spec, trace_dir.or(run))?;
    let decision = decision::decide_with_events(&spec, input);
    let trace = match trace_dir {
        Some(trace_dir) => Some(trace::write_decision_trace(
            trace_dir, path, &spec, &decision,
        )?),
        None => None,
    };
    let mut report = act::build_report_for_phase(&spec, &decision.decision, trace.as_ref(), phase)?;
    if let Some(run) = run {
        report.trace = Some(act::trace_for_run(run));
    }
    Ok(Traced {
        value: report,
        trace,
    })
}

pub fn plan(
    path: &Path,
    input: &str,
    trace_dir: Option<&Path>,
) -> error::Result<Traced<act::ActReport>> {
    let spec = parser::load_spec(path)?;
    ensure_trace_available(&spec, trace_dir)?;
    let decision = decision::decide_with_events(&spec, input);
    let trace = match trace_dir {
        Some(trace_dir) => Some(trace::write_decision_trace(
            trace_dir, path, &spec, &decision,
        )?),
        None => None,
    };
    let report = act::build_report(&spec, &decision.decision, trace.as_ref());
    Ok(Traced {
        value: report,
        trace,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn run_loop(
    path: &Path,
    input: Option<&str>,
    resume: Option<&Path>,
    view: sensemake::View,
    trace_dir: Option<&Path>,
    phase: Option<&str>,
    guide_mode: Option<guide::GuideMode>,
) -> error::Result<RunLoopOutput> {
    let started = Instant::now();
    let spec = parser::load_spec(path)?;
    if let Some(guide_mode) = guide_mode {
        if resume.is_none() {
            ensure_trace_available(&spec, trace_dir)?;
        }
        let guide_report = guide::build_report(guide::BuildOptions {
            spec: &spec,
            spec_path: path,
            input,
            resume_run_dir: resume,
            trace_dir,
            phase_override: phase,
            guide_mode,
        })?;
        Ok(RunLoopOutput::Guide(Box::new(guide_report)))
    } else {
        let input = input.ok_or_else(|| error::Error::InvalidInput {
            message: "run-loop requires --input unless --guide --resume is used".to_owned(),
        })?;
        if resume.is_some() {
            return Err(error::Error::InvalidInput {
                message: "run-loop --resume requires --guide".to_owned(),
            });
        }
        ensure_trace_available(&spec, trace_dir)?;
        let report = run_loop::build_report(&spec, path, input, view, trace_dir, phase)?;
        Ok(RunLoopOutput::Summary {
            report: Box::new(report),
            elapsed: started.elapsed(),
        })
    }
}

pub fn explain(
    path: &Path,
    input: &str,
    trace_dir: Option<&Path>,
) -> error::Result<Traced<decision::DecisionWithEvents>> {
    decide(path, input, trace_dir)
}

pub fn sensemake(path: &Path, view: sensemake::View) -> error::Result<sensemake::SensemakeReport> {
    let spec = parser::load_spec(path)?;
    Ok(sensemake::sensemake(&spec, path, view))
}

pub fn query(
    path: &Path,
    handle: &str,
    view: sensemake::View,
) -> error::Result<sensemake::QueryReport> {
    let spec = parser::load_spec(path)?;
    sensemake::query(&spec, path, handle, view)
}

pub fn refs(
    path: &Path,
    handle: &str,
    view: sensemake::View,
) -> error::Result<sensemake::RefsReport> {
    let spec = parser::load_spec(path)?;
    sensemake::refs(&spec, path, handle, view)
}

pub fn trace_for_run(run: &Path) -> act::ActTrace {
    act::trace_for_run(run)
}

pub fn render_act(report: &act::ActReport) -> String {
    act::render(report)
}

pub fn render_plan(report: &act::ActReport) -> String {
    act::render_plan(report)
}

pub fn render_guide(report: &guide::GuideReport) -> String {
    guide::render_text(report)
}

pub fn render_run_loop(report: &run_loop::RunLoopReport, elapsed: Duration) -> String {
    run_loop::render_summary(report, elapsed)
}

pub fn render_sensemake(report: &sensemake::SensemakeReport) -> String {
    sensemake::render_sensemake(report)
}

pub fn render_query(report: &sensemake::QueryReport) -> String {
    sensemake::render_query(report)
}

pub fn render_refs(report: &sensemake::RefsReport) -> String {
    sensemake::render_refs(report)
}

fn ensure_trace_available(spec: &model::SkillSpec, trace_dir: Option<&Path>) -> error::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
