use crate::cli::args::TraceCommand;
use skillspec::{align, error::Result, parser, report, trace};

pub(super) fn run(command: TraceCommand) -> Result<()> {
    match command {
        TraceCommand::Compact { run_dir } => {
            let trace = trace::compact(&run_dir)?;
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
            let spec = parser::load_spec(&path)?;
            let report =
                align::align_decision_trace(&spec, &path, &decision_trace, &execution_trace)?;
            let alignment_report = align::write_report_json(&decision_trace, &report)?;
            let proof_digest_path = match proof_digest {
                Some(path) => {
                    let digest = align::build_proof_digest(&report, &alignment_report);
                    Some(align::write_proof_digest_json(&path, &digest)?)
                }
                None => None,
            };
            if json {
                report::alignment_written(&alignment_report)?;
                if let Some(path) = &proof_digest_path {
                    report::proof_digest_written(path)?;
                }
                report::json(&report)?;
            } else if summary {
                report::align_summary(&report, &alignment_report, proof_digest_path.as_deref())?;
            } else {
                report::alignment_written(&alignment_report)?;
                if let Some(path) = &proof_digest_path {
                    report::proof_digest_written(path)?;
                }
                report::align(&report)?;
            }
            if report.has_failures() {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
