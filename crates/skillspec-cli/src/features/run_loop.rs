use crate::error::Result;
use crate::metrics::{self, MetricSummary};
use crate::model::SkillSpec;
use crate::sensemake::{self, SensemakeReport, View};
use std::path::Path;
use std::time::Duration;

pub type RunLoopReport = skillspec_runtime::run_loop::RunLoopReport<SensemakeReport>;
pub type TraceWriteResultSummary = skillspec_runtime::run_loop::TraceWriteResultSummary;

pub fn build_report(
    spec: &SkillSpec,
    spec_path: &Path,
    input: &str,
    view: View,
    trace_dir: Option<&Path>,
    phase: Option<&str>,
) -> Result<RunLoopReport> {
    let sensemake = sensemake::sensemake(spec, spec_path, view);
    skillspec_runtime::run_loop::build_report(spec, spec_path, input, sensemake, trace_dir, phase)
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
