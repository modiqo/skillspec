use crate::{align, error, parser, progress, trace};
use std::path::{Path, PathBuf};

pub use progress::{
    BatchRecordOptions, FinalResponseRecordOptions, RecordOptions, StatsRecordOptions,
};

pub struct AlignmentOutput {
    pub report: align::AlignReport,
    pub alignment_report: PathBuf,
    pub proof_digest_path: Option<PathBuf>,
}

pub fn compact_trace(run_dir: &Path) -> error::Result<trace::TraceWriteResult> {
    trace::compact(run_dir)
}

pub fn align_decision_trace(
    spec_path: &Path,
    decision_trace: &Path,
    execution_traces: &[PathBuf],
    proof_digest: Option<&Path>,
) -> error::Result<AlignmentOutput> {
    let spec = parser::load_spec(spec_path)?;
    let report = align::align_decision_trace(&spec, spec_path, decision_trace, execution_traces)?;
    let alignment_report = align::write_report_json(decision_trace, &report)?;
    let proof_digest_path = match proof_digest {
        Some(path) => {
            let digest = align::build_proof_digest(&report, &alignment_report);
            Some(align::write_proof_digest_json(path, &digest)?)
        }
        None => None,
    };
    Ok(AlignmentOutput {
        report,
        alignment_report,
        proof_digest_path,
    })
}

pub fn show_progress(spec_path: &Path, run_dir: &Path) -> error::Result<progress::ProgressReport> {
    let spec = parser::load_spec(spec_path)?;
    progress::show(&spec, run_dir)
}

pub fn render_progress(report: &progress::ProgressReport) -> String {
    progress::render(report)
}

pub fn record(options: progress::RecordOptions) -> error::Result<progress::ExecutionEvent> {
    progress::record(options)
}

pub fn record_stats(
    options: progress::StatsRecordOptions,
) -> error::Result<progress::ExecutionEvent> {
    progress::record_stats(options)
}

pub fn record_final_response(
    options: progress::FinalResponseRecordOptions,
) -> error::Result<progress::ExecutionEvent> {
    progress::record_final_response(options)
}

pub fn record_batch(
    options: progress::BatchRecordOptions,
) -> error::Result<progress::BatchRecordReport> {
    progress::record_batch(options)
}

pub fn render_batch_report(report: &progress::BatchRecordReport, summary: bool) -> String {
    progress::render_batch_report(report, summary)
}
