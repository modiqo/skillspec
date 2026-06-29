use crate::cli::args::ProgressCommand;
use skillspec::{error::Result, parser, progress, report};

pub(super) fn run(command: ProgressCommand) -> Result<()> {
    match command {
        ProgressCommand::Show { path, run, json } => {
            let spec = parser::load_spec(&path)?;
            let report = progress::show(&spec, &run)?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&progress::render(&report))?;
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
            json: _,
        } => {
            let event = progress::record(progress::RecordOptions {
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
            report::json(&event)?;
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
            json: _,
        } => {
            let event = progress::record_stats(progress::StatsRecordOptions {
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
            report::json(&event)?;
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
            json: _,
        } => {
            let event = progress::record_final_response(progress::FinalResponseRecordOptions {
                run_dir: run,
                phase,
                requirements: requirement,
                included_result: result,
                included_evidence: evidence,
                included_alignment: alignment,
                included_token_savings: token_savings,
                message,
            })?;
            report::json(&event)?;
        }
        ProgressCommand::Batch {
            run,
            events,
            checkpoint,
            summary,
            json,
        } => {
            let report = progress::record_batch(progress::BatchRecordOptions {
                run_dir: run,
                events,
                checkpoint,
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&progress::render_batch_report(&report, summary))?;
            }
        }
    }

    Ok(())
}
