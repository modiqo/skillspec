use super::ProgressEventArg;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum ProgressCommand {
    #[command(about = "Show completed, current, blocked, and remaining phases")]
    Show {
        /// Path to a skill.spec.yml file.
        path: PathBuf,
        /// Trace run directory produced by plan/decide/explain --trace-dir.
        #[arg(long)]
        run: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
        /// Suppress stdout after writing progress.json.
        #[arg(long)]
        quiet: bool,
    },
    #[command(about = "Append one structured execution/progress event to a run ledger")]
    Record {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// Event type to append.
        #[arg(value_enum)]
        event: ProgressEventArg,
        /// Phase id for phase or requirement events.
        phase: Option<String>,
        /// Requirement id for requirement events.
        requirement: Option<String>,
        /// Obligation, route, closure, or elicitation id for proof events.
        #[arg(long)]
        id: Option<String>,
        /// Event status, such as pass, fail, blocked, or pending.
        #[arg(long)]
        status: Option<String>,
        /// Evidence kind, such as file, trace, command, or response_id.
        #[arg(long)]
        evidence_kind: Option<String>,
        /// Evidence reference, such as @7 or a relative file path.
        #[arg(long)]
        evidence_ref: Option<String>,
        /// Skill that emitted this progress event.
        #[arg(long)]
        source_skill: Option<String>,
        /// Human-readable event note.
        #[arg(long)]
        message: Option<String>,
        /// Emit JSON for the appended event.
        #[arg(long)]
        json: bool,
        /// Suppress stdout after appending the event.
        #[arg(long)]
        quiet: bool,
    },
    #[command(about = "Append a stats_collected token/workspace metrics event to a run ledger")]
    Stats {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// Rote workspace name.
        #[arg(long)]
        workspace: Option<String>,
        /// Phase id whose requirement(s) this stats event satisfies.
        #[arg(long)]
        phase: Option<String>,
        /// Requirement id satisfied by this stats event. Repeat for multiple requirements.
        #[arg(long)]
        requirement: Vec<String>,
        /// JSON file produced by `rote workspace stats <workspace> --json`.
        #[arg(long)]
        workspace_stats_json: Option<PathBuf>,
        /// Human-readable report produced by `rote workspace stats <workspace>`.
        #[arg(long)]
        workspace_stats_report: Option<PathBuf>,
        /// Total API request+response tokens.
        #[arg(long)]
        total_tokens: Option<u64>,
        /// One-time context-window tokens consumed during exploration.
        #[arg(long)]
        context_tokens: Option<u64>,
        /// Tokens in extracted query results.
        #[arg(long)]
        query_result_tokens: Option<u64>,
        /// Cached response/source tokens before query reduction.
        #[arg(long)]
        response_tokens_cached: Option<u64>,
        /// Tokens saved by query reduction or cache reuse.
        #[arg(long)]
        saved_tokens: Option<u64>,
        /// Percent reduction from cached/source tokens to query-result tokens.
        #[arg(long)]
        reduction_percent: Option<f64>,
        /// Estimated tokens in the compact output visible to the agent.
        #[arg(long)]
        agent_visible_tokens: Option<u64>,
        /// Estimated tokens preserved in artifacts outside the prompt.
        #[arg(long)]
        artifact_tokens_preserved: Option<u64>,
        /// Estimated tokens avoided by showing compact output instead of full artifacts.
        #[arg(long)]
        avoided_tokens: Option<u64>,
        /// Source of the metric values, for example measured or estimated.
        #[arg(long)]
        metrics_source: Option<String>,
        /// Human-readable event note.
        #[arg(long)]
        message: Option<String>,
        /// Emit JSON for the appended event.
        #[arg(long)]
        json: bool,
        /// Suppress stdout after appending the event.
        #[arg(long)]
        quiet: bool,
    },
    #[command(about = "Append final_response_sent report-section proof to a run ledger")]
    FinalResponse {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// Phase id whose requirement(s) this final response event satisfies.
        #[arg(long)]
        phase: Option<String>,
        /// Requirement id satisfied by this final response event. Repeat for multiple requirements.
        #[arg(long)]
        requirement: Vec<String>,
        /// Final response includes the direct result.
        #[arg(long)]
        result: bool,
        /// Final response includes evidence handles or files.
        #[arg(long)]
        evidence: bool,
        /// Final response includes the alignment status or report path.
        #[arg(long)]
        alignment: bool,
        /// Final response includes token usage and token savings.
        #[arg(long)]
        token_savings: bool,
        /// Human-readable event note.
        #[arg(long)]
        message: Option<String>,
        /// Emit JSON for the appended event.
        #[arg(long)]
        json: bool,
        /// Suppress stdout after appending the event.
        #[arg(long)]
        quiet: bool,
    },
    #[command(
        about = "Checkpoint multiple structured progress events from JSONL or JSON array",
        long_about = "Append several structured progress/proof events to execution.jsonl in one checkpoint. Use --file with a JSONL batch, --quiet for background agent execution, or --summary for compact debug output. The legacy --events alias is still accepted."
    )]
    Batch {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// JSONL file or JSON array of execution events to append.
        #[arg(long = "file", visible_alias = "events", value_name = "EVIDENCE_BATCH")]
        events: PathBuf,
        /// Label printed in compact summary output.
        #[arg(long)]
        checkpoint: Option<String>,
        /// Emit compact checkpoint output instead of event counts.
        #[arg(long)]
        summary: bool,
        /// Suppress stdout after successfully appending the batch.
        #[arg(long)]
        quiet: bool,
        /// Emit JSON for the batch report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Checkpoint routine progress events from typed arguments",
        long_about = "Append routine successful progress/proof events to execution.jsonl in one checkpoint without hand-authoring an evidence JSONL file. Repeat flags as needed. Use PHASE/REQUIREMENT=KIND:REF for requirement rows and TARGET=KIND:REF for target rows."
    )]
    Checkpoint {
        /// Trace run directory containing execution.jsonl.
        run: PathBuf,
        /// Requirement proof as PHASE/REQUIREMENT=KIND:REF. Repeat for several requirements.
        #[arg(
            long = "requirement-satisfied",
            value_name = "PHASE/REQUIREMENT=KIND:REF"
        )]
        requirement_satisfied: Vec<String>,
        /// Phase completion proof as PHASE=KIND:REF. Repeat for several phases.
        #[arg(long = "phase-completed", value_name = "PHASE=KIND:REF")]
        phase_completed: Vec<String>,
        /// Route fulfillment proof as ROUTE=KIND:REF. Repeat for several routes.
        #[arg(long = "route-fulfilled", value_name = "ROUTE=KIND:REF")]
        route_fulfilled: Vec<String>,
        /// Route check proof as CHECK=KIND:REF. Repeat for several checks.
        #[arg(long = "route-check-completed", value_name = "CHECK=KIND:REF")]
        route_check_completed: Vec<String>,
        /// After-success closure proof as CLOSURE=KIND:REF. Repeat for several closures.
        #[arg(long = "after-success-completed", value_name = "CLOSURE=KIND:REF")]
        after_success_completed: Vec<String>,
        /// Obligation proof as OBLIGATION=KIND:REF. Repeat for several obligations.
        #[arg(long = "obligation-satisfied", value_name = "OBLIGATION=KIND:REF")]
        obligation_satisfied: Vec<String>,
        /// Elicitation proof as ELICITATION=KIND:REF. Repeat for several elicitations.
        #[arg(long = "elicitation-answered", value_name = "ELICITATION=KIND:REF")]
        elicitation_answered: Vec<String>,
        /// Attach extra evidence as KIND:REF. Repeat for several evidence items.
        #[arg(long = "evidence-attached", value_name = "KIND:REF")]
        evidence_attached: Vec<String>,
        /// Label printed in compact summary output.
        #[arg(long)]
        checkpoint: Option<String>,
        /// Emit compact checkpoint output instead of event counts.
        #[arg(long)]
        summary: bool,
        /// Suppress stdout after successfully appending the checkpoint.
        #[arg(long)]
        quiet: bool,
        /// Emit JSON for the checkpoint report.
        #[arg(long)]
        json: bool,
    },
}
