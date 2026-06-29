use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum TraceCommand {
    #[command(about = "Compact append-only trace events from a run directory into JSON")]
    Compact {
        /// Trace run directory produced by decide/explain --trace-dir.
        run_dir: PathBuf,
    },
    #[command(about = "Compare a SkillSpec to a decision trace")]
    Align {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Trace run directory produced by decide/explain --trace-dir.
        #[arg(long)]
        decision_trace: PathBuf,
        /// JSONL execution ledger with sanitized action evidence. Repeat for multiple ledgers.
        #[arg(long)]
        execution_trace: Vec<PathBuf>,
        /// Write a grouped missing-proof digest for one-shot final proof batching.
        #[arg(long)]
        proof_digest: Option<PathBuf>,
        /// Print only the completion-facing alignment and token summary while writing the full report to alignment.json.
        #[arg(long, conflicts_with = "json")]
        summary: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}
