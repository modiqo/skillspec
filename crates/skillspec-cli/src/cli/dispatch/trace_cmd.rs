use crate::cli::args::TraceCommand;
use skillspec::{domain::evidence, error::Result, report};

pub(super) fn run(command: TraceCommand) -> Result<()> {
    match command {
        TraceCommand::Compact { run_dir } => {
            let trace = evidence::compact_trace(&run_dir)?;
            report::json(&trace)?;
        }
        TraceCommand::Align {
            path,
            decision_trace,
            execution_trace,
            proof_digest,
            summary,
            json,
        } => {
            let output = evidence::align_decision_trace(
                &path,
                &decision_trace,
                &execution_trace,
                proof_digest.as_deref(),
            )?;
            if json {
                report::alignment_written(&output.alignment_report)?;
                if let Some(path) = &output.proof_digest_path {
                    report::proof_digest_written(path)?;
                }
                report::json(&output.report)?;
            } else if summary {
                report::align_summary(
                    &output.report,
                    &output.alignment_report,
                    output.proof_digest_path.as_deref(),
                )?;
            } else {
                report::alignment_written(&output.alignment_report)?;
                if let Some(path) = &output.proof_digest_path {
                    report::proof_digest_written(path)?;
                }
                report::align(&output.report)?;
            }
            if output.report.has_failures() {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
