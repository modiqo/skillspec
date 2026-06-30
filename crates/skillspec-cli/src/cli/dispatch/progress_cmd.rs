use crate::cli::args::ProgressCommand;
use skillspec::{domain::evidence, error::Result, report};

pub(super) fn run(command: ProgressCommand) -> Result<()> {
    match command {
        ProgressCommand::Show {
            path,
            run,
            json,
            quiet,
        } => {
            let report = evidence::show_progress(&path, &run)?;
            if json {
                report::json(&report)?;
            } else if quiet {
                // Quiet progress checks still write progress.json.
            } else {
                report::text(&evidence::render_progress(&report))?;
            }
        }
        ProgressCommand::Record {
            run,
            event,
            phase,
            requirement,
            id,
            status,
            evidence_kind,
            evidence_ref,
            source_skill,
            message,
            json,
            quiet,
        } => {
            let event = evidence::record(evidence::RecordOptions {
                run_dir: run,
                event: event.into(),
                phase,
                requirement,
                id,
                status,
                evidence_kind,
                evidence_ref,
                source_skill,
                message,
            })?;
            if json || !quiet {
                report::json(&event)?;
            }
        }
        ProgressCommand::Stats {
            run,
            workspace,
            phase,
            requirement,
            workspace_stats_json,
            workspace_stats_report,
            total_tokens,
            context_tokens,
            query_result_tokens,
            response_tokens_cached,
            saved_tokens,
            reduction_percent,
            agent_visible_tokens,
            artifact_tokens_preserved,
            avoided_tokens,
            metrics_source,
            message,
            json,
            quiet,
        } => {
            let event = evidence::record_stats(evidence::StatsRecordOptions {
                run_dir: run,
                workspace,
                phase,
                requirements: requirement,
                workspace_stats_json,
                workspace_stats_report,
                total_tokens,
                context_tokens,
                query_result_tokens,
                response_tokens_cached,
                saved_tokens,
                reduction_percent,
                agent_visible_tokens,
                artifact_tokens_preserved,
                avoided_tokens,
                metrics_source,
                message,
            })?;
            if json || !quiet {
                report::json(&event)?;
            }
        }
        ProgressCommand::FinalResponse {
            run,
            phase,
            requirement,
            result,
            evidence,
            alignment,
            token_savings,
            message,
            json,
            quiet,
        } => {
            let event = evidence::record_final_response(evidence::FinalResponseRecordOptions {
                run_dir: run,
                phase,
                requirements: requirement,
                included_result: result,
                included_evidence: evidence,
                included_alignment: alignment,
                included_token_savings: token_savings,
                message,
            })?;
            if json || !quiet {
                report::json(&event)?;
            }
        }
        ProgressCommand::Batch {
            run,
            events,
            checkpoint,
            summary,
            quiet,
            json,
        } => {
            let report = evidence::record_batch(evidence::BatchRecordOptions {
                run_dir: run,
                events,
                checkpoint,
            })?;
            if json {
                report::json(&report)?;
            } else if quiet {
                // Successful quiet checkpoints intentionally produce no output.
            } else {
                report::text(&evidence::render_batch_report(&report, summary))?;
            }
        }
    }

    Ok(())
}
