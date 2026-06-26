use crate::act;
use crate::decision;
use crate::error::{Error, Result};
use crate::model::SkillSpec;
use crate::trace::{self, TraceEnvelope};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const PROGRESS_SCHEMA: &str = "skillspec.progress/v0";
const EXECUTION_SCHEMA: &str = "skillspec.execution.v1";

#[derive(Clone, Debug, Serialize)]
pub struct ProgressReport {
    pub schema: String,
    pub run_id: String,
    pub run_dir: String,
    pub input: String,
    pub selected_route: Option<String>,
    pub completed_phases: Vec<String>,
    pub current_phase: Option<String>,
    pub blocked_phases: Vec<String>,
    pub remaining_phases: Vec<String>,
    pub open_requirements: Vec<String>,
    pub phases: Vec<PhaseProgress>,
    pub execution_proof: ExecutionProof,
}

#[derive(Clone, Debug, Serialize)]
pub struct PhaseProgress {
    pub id: String,
    pub owner_skill: String,
    pub status: PhaseStatus,
    pub requirements: Vec<RequirementProgress>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    Completed,
    Current,
    Blocked,
    Remaining,
}

#[derive(Clone, Debug, Serialize)]
pub struct RequirementProgress {
    pub id: String,
    pub status: RequirementStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    Satisfied,
    Failed,
    Open,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExecutionProof {
    pub ledger: String,
    pub event_count: usize,
    pub decision_replay: String,
    pub execution_ledger: String,
    pub forbidden_actions: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionEvent {
    #[serde(default = "default_execution_schema")]
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_unix_ms: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included_result: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included_alignment: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included_evidence: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included_token_savings: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_result_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_tokens_cached: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduction_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_visible_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_tokens_preserved: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avoided_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_source: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RecordOptions {
    pub run_dir: PathBuf,
    pub event: String,
    pub phase: Option<String>,
    pub requirement: Option<String>,
    pub id: Option<String>,
    pub status: Option<String>,
    pub evidence_kind: Option<String>,
    pub evidence_ref: Option<String>,
    pub source_skill: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct StatsRecordOptions {
    pub run_dir: PathBuf,
    pub workspace: Option<String>,
    pub phase: Option<String>,
    pub requirements: Vec<String>,
    pub workspace_stats_json: Option<PathBuf>,
    pub workspace_stats_report: Option<PathBuf>,
    pub total_tokens: Option<u64>,
    pub context_tokens: Option<u64>,
    pub query_result_tokens: Option<u64>,
    pub response_tokens_cached: Option<u64>,
    pub saved_tokens: Option<u64>,
    pub reduction_percent: Option<f64>,
    pub agent_visible_tokens: Option<u64>,
    pub artifact_tokens_preserved: Option<u64>,
    pub avoided_tokens: Option<u64>,
    pub metrics_source: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FinalResponseRecordOptions {
    pub run_dir: PathBuf,
    pub phase: Option<String>,
    pub requirements: Vec<String>,
    pub included_result: bool,
    pub included_evidence: bool,
    pub included_alignment: bool,
    pub included_token_savings: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BatchRecordOptions {
    pub run_dir: PathBuf,
    pub events: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct BatchRecordReport {
    pub schema: String,
    pub run_id: String,
    pub run_dir: String,
    pub ledger: String,
    pub events_file: String,
    pub appended: usize,
    pub by_event: BTreeMap<String, usize>,
}

pub fn show(spec: &SkillSpec, run_dir: &Path) -> Result<ProgressReport> {
    let _ = trace::compact(run_dir)?;
    let envelopes = trace::read_envelopes(run_dir)?;
    if envelopes.is_empty() {
        return Err(Error::InvalidInput {
            message: format!("decision trace {} has no events", run_dir.display()),
        });
    }

    let input = trace_input(&envelopes)?;
    let decision = decision::decide_with_events(spec, &input).decision;
    let act_report = act::build_report(spec, &decision, None);
    let ledger_path = execution_ledger_path(run_dir);
    let events = read_execution_events(&ledger_path)?;
    let report = build_progress_report(run_dir, &input, &act_report, &ledger_path, &events);
    write_progress_json(run_dir, &report)?;
    Ok(report)
}

pub fn record(options: RecordOptions) -> Result<ExecutionEvent> {
    fs::create_dir_all(&options.run_dir).map_err(|source| Error::Write {
        path: options.run_dir.clone(),
        source,
    })?;
    let run_id = options
        .run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();
    let evidence = evidence_value(&options);
    let source = options.source_skill.as_ref().map(|skill| {
        serde_json::json!({
            "skill": skill,
        })
    });
    let event = ExecutionEvent {
        schema: EXECUTION_SCHEMA.to_owned(),
        run_id: Some(run_id),
        event: options.event,
        phase: options.phase,
        requirement: options.requirement,
        id: options.id,
        status: options.status,
        evidence,
        source,
        at_unix_ms: Some(unix_ms()),
        message: options.message,
        included_result: None,
        included_alignment: None,
        included_evidence: None,
        included_token_savings: None,
        workspace: None,
        total_tokens: None,
        context_tokens: None,
        query_result_tokens: None,
        response_tokens_cached: None,
        saved_tokens: None,
        reduction_percent: None,
        agent_visible_tokens: None,
        artifact_tokens_preserved: None,
        avoided_tokens: None,
        metrics_source: None,
    };
    append_execution_event(&execution_ledger_path(&options.run_dir), &event)?;
    Ok(event)
}

pub fn record_stats(options: StatsRecordOptions) -> Result<ExecutionEvent> {
    validate_requirement_recording(options.phase.as_ref(), &options.requirements)?;
    let has_direct_token_field = options.total_tokens.is_some()
        || options.context_tokens.is_some()
        || options.query_result_tokens.is_some()
        || options.response_tokens_cached.is_some()
        || options.saved_tokens.is_some()
        || options.reduction_percent.is_some()
        || options.agent_visible_tokens.is_some()
        || options.artifact_tokens_preserved.is_some()
        || options.avoided_tokens.is_some();
    if options.workspace_stats_json.is_none()
        && options.workspace_stats_report.is_none()
        && !has_direct_token_field
    {
        return Err(Error::InvalidInput {
            message: "progress stats requires --workspace-stats-json, --workspace-stats-report, or at least one explicit token metric".to_owned(),
        });
    }

    fs::create_dir_all(&options.run_dir).map_err(|source| Error::Write {
        path: options.run_dir.clone(),
        source,
    })?;
    let run_id = options
        .run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();

    let stats_json = match &options.workspace_stats_json {
        Some(path) => {
            let content = fs::read_to_string(path).map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;
            Some(
                serde_json::from_str::<serde_json::Value>(&content).map_err(|source| {
                    Error::ParseJson {
                        path: path.clone(),
                        source,
                    }
                })?,
            )
        }
        None => None,
    };
    let stats = stats_json.as_ref();
    let stats_report = match &options.workspace_stats_report {
        Some(path) => {
            let content = fs::read_to_string(path).map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;
            Some(parse_workspace_stats_report(&content))
        }
        None => None,
    };
    let report = stats_report.as_ref();
    let response_tokens_cached = options
        .response_tokens_cached
        .or_else(|| json_u64(stats, &["response_tokens_cached"]))
        .or_else(|| json_u64(stats, &["cached_tokens"]))
        .or_else(|| json_u64(stats, &["token_savings", "source_tokens"]))
        .or_else(|| report.and_then(|stats| stats.response_tokens_cached));
    let query_result_tokens = options
        .query_result_tokens
        .or_else(|| json_u64(stats, &["query_result_tokens"]))
        .or_else(|| json_u64(stats, &["token_savings", "result_tokens"]))
        .or_else(|| report.and_then(|stats| stats.query_result_tokens));
    let saved_tokens = options
        .saved_tokens
        .or_else(|| json_u64(stats, &["saved_tokens"]))
        .or_else(|| json_u64(stats, &["token_savings", "tokens_saved"]))
        .or_else(|| report.and_then(|stats| stats.saved_tokens));
    let reduction_percent = options
        .reduction_percent
        .or_else(|| json_f64(stats, &["reduction_percent"]))
        .or_else(|| json_f64(stats, &["token_savings", "reduction_percent"]))
        .or_else(|| report.and_then(|stats| stats.reduction_percent))
        .or_else(|| {
            let source = response_tokens_cached?;
            let saved = saved_tokens
                .or_else(|| query_result_tokens.map(|result| source.saturating_sub(result)))?;
            (source > 0).then_some((saved as f64 / source as f64) * 100.0)
        });

    let source = stats_source(&options);
    let event = ExecutionEvent {
        schema: EXECUTION_SCHEMA.to_owned(),
        run_id: Some(run_id),
        event: "stats_collected".to_owned(),
        phase: None,
        requirement: None,
        id: None,
        status: None,
        evidence: None,
        source,
        at_unix_ms: Some(unix_ms()),
        message: options.message,
        included_result: None,
        included_alignment: None,
        included_evidence: None,
        included_token_savings: None,
        workspace: options
            .workspace
            .or_else(|| json_string(stats, &["name"]))
            .or_else(|| report.and_then(|stats| stats.workspace.clone())),
        total_tokens: options
            .total_tokens
            .or_else(|| json_u64(stats, &["total_tokens"]))
            .or_else(|| json_u64(stats, &["metrics", "total_tokens"]))
            .or_else(|| report.and_then(|stats| stats.total_tokens)),
        context_tokens: options
            .context_tokens
            .or_else(|| json_u64(stats, &["context_tokens"]))
            .or_else(|| json_u64(stats, &["metrics", "context_tokens"]))
            .or_else(|| report.and_then(|stats| stats.context_tokens)),
        query_result_tokens,
        response_tokens_cached,
        saved_tokens,
        reduction_percent,
        agent_visible_tokens: options.agent_visible_tokens,
        artifact_tokens_preserved: options.artifact_tokens_preserved,
        avoided_tokens: options.avoided_tokens.or_else(|| {
            options
                .artifact_tokens_preserved
                .zip(options.agent_visible_tokens)
                .map(|(artifact, visible)| artifact.saturating_sub(visible))
        }),
        metrics_source: options.metrics_source,
    };
    let ledger_path = execution_ledger_path(&options.run_dir);
    append_execution_event(&ledger_path, &event)?;
    append_requirement_satisfied_events(
        &ledger_path,
        event.run_id.as_deref().unwrap_or("unknown"),
        options.phase.as_deref(),
        &options.requirements,
        "stats_collected",
    )?;
    Ok(event)
}

pub fn record_final_response(options: FinalResponseRecordOptions) -> Result<ExecutionEvent> {
    validate_requirement_recording(options.phase.as_ref(), &options.requirements)?;
    fs::create_dir_all(&options.run_dir).map_err(|source| Error::Write {
        path: options.run_dir.clone(),
        source,
    })?;
    let run_id = options
        .run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();
    let event = ExecutionEvent {
        schema: EXECUTION_SCHEMA.to_owned(),
        run_id: Some(run_id),
        event: "final_response_sent".to_owned(),
        phase: None,
        requirement: None,
        id: None,
        status: None,
        evidence: None,
        source: None,
        at_unix_ms: Some(unix_ms()),
        message: options.message,
        included_result: Some(options.included_result),
        included_alignment: Some(options.included_alignment),
        included_evidence: Some(options.included_evidence),
        included_token_savings: Some(options.included_token_savings),
        workspace: None,
        total_tokens: None,
        context_tokens: None,
        query_result_tokens: None,
        response_tokens_cached: None,
        saved_tokens: None,
        reduction_percent: None,
        agent_visible_tokens: None,
        artifact_tokens_preserved: None,
        avoided_tokens: None,
        metrics_source: None,
    };
    let ledger_path = execution_ledger_path(&options.run_dir);
    append_execution_event(&ledger_path, &event)?;
    append_requirement_satisfied_events(
        &ledger_path,
        event.run_id.as_deref().unwrap_or("unknown"),
        options.phase.as_deref(),
        &options.requirements,
        "final_response_sent",
    )?;
    Ok(event)
}

pub fn record_batch(options: BatchRecordOptions) -> Result<BatchRecordReport> {
    fs::create_dir_all(&options.run_dir).map_err(|source| Error::Write {
        path: options.run_dir.clone(),
        source,
    })?;
    let run_id = options
        .run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();
    let ledger_path = execution_ledger_path(&options.run_dir);
    let mut events = read_batch_events(&options.events)?;
    if events.is_empty() {
        return Err(Error::InvalidInput {
            message: "progress batch requires at least one event".to_owned(),
        });
    }

    for event in &mut events {
        normalize_batch_event(event, &run_id)?;
    }

    let mut by_event = BTreeMap::<String, usize>::new();
    for event in &events {
        *by_event.entry(event.event.clone()).or_default() += 1;
        append_execution_event(&ledger_path, event)?;
    }

    Ok(BatchRecordReport {
        schema: "skillspec.progress.batch.v0".to_owned(),
        run_id,
        run_dir: options.run_dir.display().to_string(),
        ledger: ledger_path.display().to_string(),
        events_file: options.events.display().to_string(),
        appended: events.len(),
        by_event,
    })
}

pub fn render_batch_report(report: &BatchRecordReport) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "progress batch: appended {} events\n",
        report.appended
    ));
    output.push_str(&format!("- ledger: {}\n", report.ledger));
    output.push_str("- event counts:\n");
    for (event, count) in &report.by_event {
        output.push_str(&format!("  - {event}: {count}\n"));
    }
    output
}

#[derive(Clone, Debug, Default)]
struct WorkspaceStatsReport {
    workspace: Option<String>,
    total_tokens: Option<u64>,
    context_tokens: Option<u64>,
    query_result_tokens: Option<u64>,
    response_tokens_cached: Option<u64>,
    saved_tokens: Option<u64>,
    reduction_percent: Option<f64>,
}

pub fn render(report: &ProgressReport) -> String {
    let mut output = String::new();
    output.push_str("SkillSpec progress\n\n");
    match &report.selected_route {
        Some(route) => output.push_str(&format!("Route: {route}\n")),
        None => output.push_str("Route: none\n"),
    }
    output.push_str(&format!("Run: {}\n", report.run_dir));

    output.push_str("\nCompleted:\n");
    if report.completed_phases.is_empty() {
        output.push_str("- none\n");
    } else {
        write_bullets(&mut output, &report.completed_phases);
    }

    output.push_str("\nCurrent:\n");
    match &report.current_phase {
        Some(phase) => {
            output.push_str(&format!("- {phase}\n"));
            if !report.open_requirements.is_empty() {
                output.push_str(&format!(
                    "  open requirements: {}\n",
                    report.open_requirements.join(", ")
                ));
            }
        }
        None => output.push_str("- none; all phases are completed or blocked\n"),
    }

    if !report.blocked_phases.is_empty() {
        output.push_str("\nBlocked:\n");
        write_bullets(&mut output, &report.blocked_phases);
    }

    output.push_str("\nRemaining:\n");
    if report.remaining_phases.is_empty() {
        output.push_str("- none\n");
    } else {
        write_bullets(&mut output, &report.remaining_phases);
    }

    output.push_str("\nExecution proof:\n");
    output.push_str(&format!(
        "- decision replay: {}\n",
        report.execution_proof.decision_replay
    ));
    output.push_str(&format!(
        "- execution ledger: {}\n",
        report.execution_proof.execution_ledger
    ));
    output.push_str(&format!(
        "- events recorded: {}\n",
        report.execution_proof.event_count
    ));
    output.push_str(&format!(
        "- forbidden actions: {}\n",
        report.execution_proof.forbidden_actions
    ));

    output.trim_end().to_owned()
}

fn build_progress_report(
    run_dir: &Path,
    input: &str,
    act_report: &act::ActReport,
    ledger_path: &Path,
    events: &[ExecutionEvent],
) -> ProgressReport {
    let completed = phase_event_set(events, "phase_completed");
    let blocked = phase_event_set(events, "phase_blocked");
    let failed_requirements = requirement_event_set(events, "requirement_failed");
    let satisfied_requirements = requirement_event_set(events, "requirement_satisfied");

    let current_phase = act_report
        .phases
        .iter()
        .find(|phase| !completed.contains(&phase.id) && !blocked.contains(&phase.id))
        .map(|phase| phase.id.clone());

    let phases = act_report
        .phases
        .iter()
        .map(|phase| {
            let status = if completed.contains(&phase.id) {
                PhaseStatus::Completed
            } else if blocked.contains(&phase.id) {
                PhaseStatus::Blocked
            } else if current_phase
                .as_ref()
                .is_some_and(|current| current == &phase.id)
            {
                PhaseStatus::Current
            } else {
                PhaseStatus::Remaining
            };
            let requirements = phase
                .requires
                .iter()
                .map(|requirement| {
                    let key = requirement_key(&phase.id, requirement);
                    let status = if failed_requirements.contains(&key) {
                        RequirementStatus::Failed
                    } else if satisfied_requirements.contains(&key) {
                        RequirementStatus::Satisfied
                    } else {
                        RequirementStatus::Open
                    };
                    RequirementProgress {
                        id: requirement.clone(),
                        status,
                    }
                })
                .collect();
            PhaseProgress {
                id: phase.id.clone(),
                owner_skill: phase.owner_skill.clone(),
                status,
                requirements,
            }
        })
        .collect::<Vec<_>>();

    let open_requirements = phases
        .iter()
        .find(|phase| Some(&phase.id) == current_phase.as_ref())
        .map(|phase| {
            phase
                .requirements
                .iter()
                .filter(|requirement| requirement.status == RequirementStatus::Open)
                .map(|requirement| requirement.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let remaining_phases = phases
        .iter()
        .filter(|phase| phase.status == PhaseStatus::Remaining)
        .map(|phase| phase.id.clone())
        .collect();
    let completed_phases = phases
        .iter()
        .filter(|phase| phase.status == PhaseStatus::Completed)
        .map(|phase| phase.id.clone())
        .collect();
    let blocked_phases = phases
        .iter()
        .filter(|phase| phase.status == PhaseStatus::Blocked)
        .map(|phase| phase.id.clone())
        .collect();

    let run_id = run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();

    ProgressReport {
        schema: PROGRESS_SCHEMA.to_owned(),
        run_id,
        run_dir: run_dir.display().to_string(),
        input: input.to_owned(),
        selected_route: act_report.selected_route.clone(),
        completed_phases,
        current_phase,
        blocked_phases,
        remaining_phases,
        open_requirements,
        phases,
        execution_proof: ExecutionProof {
            ledger: ledger_path.display().to_string(),
            event_count: events.len(),
            decision_replay: "pass".to_owned(),
            execution_ledger: if events.is_empty() {
                "missing; no execution events recorded".to_owned()
            } else {
                "present".to_owned()
            },
            forbidden_actions: forbidden_summary(events),
        },
    }
}

fn read_execution_events(path: &Path) -> Result<Vec<ExecutionEvent>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut events = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let event =
            serde_json::from_str::<ExecutionEvent>(line).map_err(|source| Error::ParseJson {
                path: path.to_path_buf(),
                source,
            })?;
        events.push(event);
    }
    Ok(events)
}

fn read_batch_events(path: &Path) -> Result<Vec<ExecutionEvent>> {
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<ExecutionEvent>>(trimmed).map_err(|source| {
            Error::ParseJson {
                path: path.to_path_buf(),
                source,
            }
        });
    }

    let mut events = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let event =
            serde_json::from_str::<ExecutionEvent>(line).map_err(|source| Error::ParseJson {
                path: path.to_path_buf(),
                source,
            })?;
        events.push(event);
    }
    Ok(events)
}

fn normalize_batch_event(event: &mut ExecutionEvent, run_id: &str) -> Result<()> {
    event.schema = EXECUTION_SCHEMA.to_owned();
    event.run_id.get_or_insert_with(|| run_id.to_owned());
    event.event = event.event.replace('-', "_");
    if !is_known_progress_event(&event.event) {
        return Err(Error::InvalidInput {
            message: format!("unknown progress batch event {:?}", event.event),
        });
    }
    if event.requirement.is_some() && event.phase.is_none() {
        return Err(Error::InvalidInput {
            message: format!(
                "progress batch event {:?} records a requirement without a phase",
                event.event
            ),
        });
    }
    event.at_unix_ms.get_or_insert_with(unix_ms);
    Ok(())
}

fn is_known_progress_event(event: &str) -> bool {
    matches!(
        event,
        "phase_started"
            | "requirement_started"
            | "requirement_satisfied"
            | "requirement_failed"
            | "stats_collected"
            | "obligation_satisfied"
            | "route_fulfilled"
            | "after_success_completed"
            | "evidence_attached"
            | "handoff_started"
            | "handoff_completed"
            | "phase_completed"
            | "phase_blocked"
            | "final_response_sent"
            | "forbidden_action"
            | "forbidden_action_observed"
            | "forbid_violated"
    )
}

fn append_execution_event(path: &Path, event: &ExecutionEvent) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        })?;
    let mut row = serde_json::to_vec(event)?;
    row.push(b'\n');
    file.write_all(&row).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn validate_requirement_recording(phase: Option<&String>, requirements: &[String]) -> Result<()> {
    if !requirements.is_empty() && phase.is_none() {
        return Err(Error::InvalidInput {
            message: "--requirement requires --phase".to_owned(),
        });
    }
    Ok(())
}

fn append_requirement_satisfied_events(
    ledger_path: &Path,
    run_id: &str,
    phase: Option<&str>,
    requirements: &[String],
    evidence_ref: &str,
) -> Result<()> {
    let Some(phase) = phase else {
        return Ok(());
    };
    for requirement in requirements {
        let event = ExecutionEvent {
            schema: EXECUTION_SCHEMA.to_owned(),
            run_id: Some(run_id.to_owned()),
            event: "requirement_satisfied".to_owned(),
            phase: Some(phase.to_owned()),
            requirement: Some(requirement.to_owned()),
            id: None,
            status: Some("pass".to_owned()),
            evidence: Some(serde_json::json!({
                "kind": "progress_event",
                "ref": evidence_ref,
            })),
            source: None,
            at_unix_ms: Some(unix_ms()),
            message: None,
            included_result: None,
            included_alignment: None,
            included_evidence: None,
            included_token_savings: None,
            workspace: None,
            total_tokens: None,
            context_tokens: None,
            query_result_tokens: None,
            response_tokens_cached: None,
            saved_tokens: None,
            reduction_percent: None,
            agent_visible_tokens: None,
            artifact_tokens_preserved: None,
            avoided_tokens: None,
            metrics_source: None,
        };
        append_execution_event(ledger_path, &event)?;
    }
    Ok(())
}

fn write_progress_json(run_dir: &Path, report: &ProgressReport) -> Result<()> {
    let path = run_dir.join("progress.json");
    let content = serde_json::to_vec_pretty(report)?;
    fs::write(&path, content).map_err(|source| Error::Write { path, source })?;
    Ok(())
}

fn execution_ledger_path(run_dir: &Path) -> PathBuf {
    run_dir.join("execution.jsonl")
}

fn phase_event_set(events: &[ExecutionEvent], event_name: &str) -> BTreeSet<String> {
    events
        .iter()
        .filter(|event| event.event == event_name)
        .filter_map(|event| event.phase.clone())
        .collect()
}

fn requirement_event_set(events: &[ExecutionEvent], event_name: &str) -> BTreeSet<String> {
    events
        .iter()
        .filter(|event| event.event == event_name)
        .filter_map(|event| {
            Some(requirement_key(
                event.phase.as_ref()?,
                event.requirement.as_ref()?,
            ))
        })
        .collect()
}

fn requirement_key(phase: &str, requirement: &str) -> String {
    format!("{phase}::{requirement}")
}

fn forbidden_summary(events: &[ExecutionEvent]) -> String {
    let violations = events
        .iter()
        .filter(|event| {
            matches!(
                event.event.as_str(),
                "forbidden_action" | "forbidden_action_observed" | "forbid_violated"
            )
        })
        .count();
    if violations == 0 {
        "none observed".to_owned()
    } else {
        format!("{violations} violation event(s) recorded")
    }
}

fn evidence_value(options: &RecordOptions) -> Option<serde_json::Value> {
    match (&options.evidence_kind, &options.evidence_ref) {
        (None, None) => None,
        (kind, reference) => Some(serde_json::json!({
            "kind": kind,
            "ref": reference,
        })),
    }
}

fn stats_source(options: &StatsRecordOptions) -> Option<serde_json::Value> {
    let has_summary_metrics = options.agent_visible_tokens.is_some()
        || options.artifact_tokens_preserved.is_some()
        || options.avoided_tokens.is_some()
        || options.metrics_source.is_some();
    match (
        &options.workspace_stats_json,
        &options.workspace_stats_report,
    ) {
        (Some(json_path), Some(report_path)) => Some(serde_json::json!({
            "kind": "rote_workspace_stats",
            "json_path": json_path.display().to_string(),
            "report_path": report_path.display().to_string(),
        })),
        (Some(path), None) => Some(serde_json::json!({
            "kind": "rote_workspace_stats",
            "format": "json",
            "path": path.display().to_string(),
        })),
        (None, Some(path)) => Some(serde_json::json!({
            "kind": "rote_workspace_stats",
            "format": "report",
            "path": path.display().to_string(),
        })),
        (None, None) if has_summary_metrics => Some(serde_json::json!({
            "kind": "summary_metrics",
            "metrics_source": options.metrics_source.as_deref().unwrap_or("estimated"),
        })),
        (None, None) => None,
    }
}

fn parse_workspace_stats_report(content: &str) -> WorkspaceStatsReport {
    let mut report = WorkspaceStatsReport::default();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if report.workspace.is_none() {
            report.workspace =
                key_value(trimmed, "workspace").or_else(|| key_value(trimmed, "name"));
        }

        report.total_tokens = report.total_tokens.or_else(|| {
            metric_u64(
                trimmed,
                &[
                    "total tokens",
                    "tokens total",
                    "api request+response tokens",
                ],
            )
        });
        report.context_tokens = report.context_tokens.or_else(|| {
            metric_u64(
                trimmed,
                &[
                    "context tokens",
                    "workspace context tokens",
                    "context-window tokens",
                ],
            )
        });
        report.response_tokens_cached = report.response_tokens_cached.or_else(|| {
            metric_u64(
                trimmed,
                &[
                    "source tokens",
                    "cached response tokens",
                    "response tokens cached",
                    "cached tokens",
                ],
            )
        });
        report.query_result_tokens = report.query_result_tokens.or_else(|| {
            metric_u64(
                trimmed,
                &[
                    "result tokens",
                    "query-result tokens",
                    "query result tokens",
                ],
            )
        });
        report.saved_tokens = report
            .saved_tokens
            .or_else(|| metric_u64(trimmed, &["tokens saved", "saved tokens"]));
        report.reduction_percent = report
            .reduction_percent
            .or_else(|| reduction_percent(trimmed));
    }
    report
}

fn key_value(line: &str, key: &str) -> Option<String> {
    let (candidate, value) = line.split_once(':')?;
    candidate
        .trim()
        .eq_ignore_ascii_case(key)
        .then(|| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn metric_u64(line: &str, labels: &[&str]) -> Option<u64> {
    let lower = line.to_ascii_lowercase();
    for label in labels {
        if let Some(position) = lower.find(label) {
            let label_end = position + label.len();
            if let Some(number) = number_after_label_separator(line, label_end) {
                return Some(number);
            }
            if let Some(number) = number_before_label(line, position) {
                return Some(number);
            }
        }
    }
    None
}

fn number_after_label_separator(line: &str, label_end: usize) -> Option<u64> {
    let suffix = line.get(label_end..)?.trim_start();
    let rest = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('='))?
        .trim_start();
    first_u64(rest)
}

fn number_before_label(line: &str, label_start: usize) -> Option<u64> {
    let prefix = line.get(..label_start)?;
    let trimmed_end = prefix.trim_end();
    let mut start = trimmed_end.len();
    for (index, ch) in trimmed_end.char_indices().rev() {
        if ch.is_ascii_digit() || ch == ',' {
            start = index;
        } else {
            break;
        }
    }
    (start < trimmed_end.len())
        .then(|| &trimmed_end[start..])
        .and_then(parse_u64_token)
}

fn first_u64(text: &str) -> Option<u64> {
    let mut start = None;
    let mut end = 0;
    for (index, ch) in text.char_indices() {
        if ch.is_ascii_digit() {
            if start.is_none() {
                start = Some(index);
            }
            end = index + ch.len_utf8();
        } else if ch == ',' && start.is_some() {
            end = index + ch.len_utf8();
        } else if start.is_some() {
            break;
        }
    }
    let start = start?;
    parse_u64_token(&text[start..end])
}

fn parse_u64_token(token: &str) -> Option<u64> {
    let digits = token
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    (!digits.is_empty())
        .then_some(digits)
        .and_then(|digits| digits.parse::<u64>().ok())
}

fn reduction_percent(line: &str) -> Option<f64> {
    if !line.to_ascii_lowercase().contains("reduction") {
        return None;
    }
    let percent_index = line.find('%')?;
    let prefix = line.get(..percent_index)?.trim_end();
    let mut start = prefix.len();
    for (index, ch) in prefix.char_indices().rev() {
        if ch.is_ascii_digit() || ch == '.' {
            start = index;
        } else {
            break;
        }
    }
    (start < prefix.len())
        .then(|| &prefix[start..])
        .and_then(|number| number.parse::<f64>().ok())
}

fn json_string(value: Option<&serde_json::Value>, path: &[&str]) -> Option<String> {
    json_path(value?, path)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn json_u64(value: Option<&serde_json::Value>, path: &[&str]) -> Option<u64> {
    let value = json_path(value?, path)?;
    value.as_u64().or_else(|| {
        value
            .as_i64()
            .and_then(|number| (number >= 0).then_some(number as u64))
    })
}

fn json_f64(value: Option<&serde_json::Value>, path: &[&str]) -> Option<f64> {
    let value = json_path(value?, path)?;
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|number| number as f64))
        .or_else(|| value.as_u64().map(|number| number as f64))
}

fn json_path<'a>(mut value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    for key in path {
        value = value.get(*key)?;
    }
    Some(value)
}

fn trace_input(envelopes: &[TraceEnvelope]) -> Result<String> {
    envelopes
        .iter()
        .rev()
        .find(|envelope| envelope.event_name == "input_received")
        .and_then(|envelope| envelope.data.get("input"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::InvalidInput {
            message: "decision trace does not contain an input_received event with input"
                .to_owned(),
        })
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn default_execution_schema() -> String {
    EXECUTION_SCHEMA.to_owned()
}

fn write_bullets(output: &mut String, items: &[String]) {
    for item in items {
        output.push_str(&format!("- {item}\n"));
    }
}
