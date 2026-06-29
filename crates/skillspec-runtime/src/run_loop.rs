use crate::act::{self, ActReport};
use crate::decision::{self, Decision};
use crate::trace;
use serde::Serialize;
use skillspec_core::error::Result;
use skillspec_core::model::SkillSpec;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct RunLoopReport<SensemakeReport> {
    pub spec_id: String,
    pub spec_path: String,
    pub input: String,
    pub sensemake: SensemakeReport,
    pub decision: Decision,
    pub act: ActReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<TraceWriteResultSummary>,
    pub batched_commands: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceWriteResultSummary {
    pub run_id: String,
    pub run_dir: String,
    pub trace_jsonl: String,
    pub summary_json: String,
}

pub fn build_report<SensemakeReport>(
    spec: &SkillSpec,
    spec_path: &Path,
    input: &str,
    sensemake: SensemakeReport,
    trace_dir: Option<&Path>,
    phase: Option<&str>,
) -> Result<RunLoopReport<SensemakeReport>> {
    let decision_with_events = decision::decide_with_events(spec, input);
    let trace = if let Some(trace_dir) = trace_dir {
        Some(trace::write_decision_trace(
            trace_dir,
            spec_path,
            spec,
            &decision_with_events,
        )?)
    } else {
        None
    };
    let act =
        act::build_report_for_phase(spec, &decision_with_events.decision, trace.as_ref(), phase)?;
    Ok(RunLoopReport {
        spec_id: spec.id.clone(),
        spec_path: spec_path.display().to_string(),
        input: input.to_owned(),
        sensemake,
        decision: decision_with_events.decision,
        act,
        trace: trace.map(|trace| TraceWriteResultSummary {
            run_id: trace.run_id,
            run_dir: trace.run_dir.display().to_string(),
            trace_jsonl: trace.trace_jsonl.display().to_string(),
            summary_json: trace.summary_json.display().to_string(),
        }),
        batched_commands: vec![
            "sensemake".to_owned(),
            "decide".to_owned(),
            "plan".to_owned(),
            "act".to_owned(),
        ],
    })
}
