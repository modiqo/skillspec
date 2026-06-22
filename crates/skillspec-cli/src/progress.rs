use crate::act;
use crate::decision;
use crate::error::{Error, Result};
use crate::model::SkillSpec;
use crate::trace::{self, TraceEnvelope};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
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
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_unix_ms: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RecordOptions {
    pub run_dir: PathBuf,
    pub event: String,
    pub phase: Option<String>,
    pub requirement: Option<String>,
    pub status: Option<String>,
    pub evidence_kind: Option<String>,
    pub evidence_ref: Option<String>,
    pub source_skill: Option<String>,
    pub message: Option<String>,
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
        status: options.status,
        evidence,
        source,
        at_unix_ms: Some(unix_ms()),
        message: options.message,
    };
    append_execution_event(&execution_ledger_path(&options.run_dir), &event)?;
    Ok(event)
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

fn append_execution_event(path: &Path, event: &ExecutionEvent) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::to_writer(&mut file, event)?;
    writeln!(file)?;
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
