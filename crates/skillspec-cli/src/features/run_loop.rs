use crate::act::{self, ActReport};
use crate::decision::{self, Decision};
use crate::error::Result;
use crate::metrics::{self, MetricSummary};
use crate::model::SkillSpec;
use crate::sensemake::{self, SensemakeReport, View};
use crate::trace;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, Serialize)]
pub struct RunLoopReport {
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

pub fn build_report(
    spec: &SkillSpec,
    spec_path: &Path,
    input: &str,
    view: View,
    trace_dir: Option<&Path>,
    phase: Option<&str>,
) -> Result<RunLoopReport> {
    let sensemake = sensemake::sensemake(spec, spec_path, view);
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

pub fn render_summary(report: &RunLoopReport, elapsed: Duration) -> String {
    let mut metrics = MetricSummary::new(elapsed, 0);
    metrics.cli_calls = 1;
    metrics::render_with_metrics(metrics, |metrics| {
        let mut output = String::new();
        output.push_str("SkillSpec run-loop summary\n\n");
        output.push_str(&format!("- spec: {}\n", report.spec_path));
        output.push_str(&format!("- input: {}\n", report.input));
        output.push_str(&format!(
            "- selected_route: {}\n",
            report
                .decision
                .route
                .as_ref()
                .map(|route| route.0.as_str())
                .unwrap_or("none")
        ));
        output.push_str(&format!(
            "- matched_rules: {}\n",
            report
                .decision
                .matched_rules
                .iter()
                .map(|rule| rule.id.0.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        output.push_str(&format!("- phases: {}\n", report.act.phases.len()));
        output.push_str(&format!(
            "- current_phase: {}\n",
            report
                .act
                .current_phase
                .as_ref()
                .map(|phase| phase.id.as_str())
                .unwrap_or("none")
        ));
        if let Some(trace) = &report.trace {
            output.push_str(&format!("- trace: {}\n", trace.run_dir));
        }
        output.push_str(&format!(
            "- batched_commands: {}\n",
            report.batched_commands.join(", ")
        ));
        output.push_str("- avoided_cli_invocations: 3\n");
        output.push('\n');
        metrics::push_metric_block(&mut output, metrics);
        output.push_str("\nnext:\n");
        output.push_str("- use `skillspec act <spec> --input <task> --phase <phase>` for exact phase detail if needed\n");
        output.push_str("- use `skillspec query` or `skillspec refs` for focused handles instead of reading the full spec\n");
        output
    })
}
