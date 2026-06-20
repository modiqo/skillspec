use crate::decision::{DecisionEvent, DecisionWithEvents};
use crate::error::{Error, Result};
use crate::model::{SkillSpec, TraceConfig, TraceEventKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const TRACE_SCHEMA: &str = "skillspec.trace/v0";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceEnvelope {
    pub schema: String,
    pub run_id: String,
    pub seq: u64,
    pub timestamp_unix_ms: u128,
    pub skill_id: String,
    pub spec_schema: String,
    pub event: TraceEventKind,
    pub event_name: String,
    pub data: serde_json::Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceWriteResult {
    pub run_id: String,
    pub run_dir: PathBuf,
    pub event_count: usize,
    pub trace_jsonl: PathBuf,
    pub summary_json: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceSummary {
    pub schema: String,
    pub run_id: String,
    pub skill_id: String,
    pub event_count: usize,
    pub events: BTreeMap<TraceEventKind, usize>,
}

pub fn write_decision_trace(
    root: &Path,
    spec: &SkillSpec,
    decision: &DecisionWithEvents,
) -> Result<TraceWriteResult> {
    let run_id = new_run_id();
    let run_dir = root.join(&run_id);
    let events_dir = run_dir.join("events");
    fs::create_dir_all(&events_dir).map_err(|source| Error::Write {
        path: events_dir.clone(),
        source,
    })?;

    let selected = selected_events(spec.trace.as_ref(), &decision.events);
    for (index, event) in selected.iter().enumerate() {
        let envelope = envelope_for(spec, &run_id, index as u64 + 1, event)?;
        write_event(&events_dir, &envelope)?;
    }

    compact(&run_dir)
}

pub fn compact(run_dir: &Path) -> Result<TraceWriteResult> {
    let events_dir = run_dir.join("events");
    let mut event_files = fs::read_dir(&events_dir)
        .map_err(|source| Error::Read {
            path: events_dir.clone(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    event_files.sort();

    let mut envelopes = Vec::with_capacity(event_files.len());
    for path in event_files {
        let content = fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        let envelope =
            serde_json::from_str::<TraceEnvelope>(&content).map_err(|source| Error::ParseJson {
                path: path.clone(),
                source,
            })?;
        envelopes.push(envelope);
    }

    let trace_jsonl = run_dir.join("trace.jsonl");
    let mut trace_file = fs::File::create(&trace_jsonl).map_err(|source| Error::Write {
        path: trace_jsonl.clone(),
        source,
    })?;
    for envelope in &envelopes {
        serde_json::to_writer(&mut trace_file, envelope)?;
        writeln!(trace_file)?;
    }

    let summary = summary_for(&envelopes);
    let summary_json = run_dir.join("summary.json");
    let summary_content = serde_json::to_vec_pretty(&summary)?;
    fs::write(&summary_json, summary_content).map_err(|source| Error::Write {
        path: summary_json.clone(),
        source,
    })?;

    Ok(TraceWriteResult {
        run_id: summary.run_id,
        run_dir: run_dir.to_path_buf(),
        event_count: summary.event_count,
        trace_jsonl,
        summary_json,
    })
}

fn selected_events<'a>(
    config: Option<&TraceConfig>,
    events: &'a [DecisionEvent],
) -> Vec<&'a DecisionEvent> {
    let Some(config) = config else {
        return events.iter().collect();
    };
    if config.record.is_empty() {
        return events.iter().collect();
    }
    let record = config.record.iter().cloned().collect::<BTreeSet<_>>();
    events
        .iter()
        .filter(|event| record.contains(&event.kind()))
        .collect()
}

fn envelope_for(
    spec: &SkillSpec,
    run_id: &str,
    seq: u64,
    event: &DecisionEvent,
) -> Result<TraceEnvelope> {
    Ok(TraceEnvelope {
        schema: TRACE_SCHEMA.to_owned(),
        run_id: run_id.to_owned(),
        seq,
        timestamp_unix_ms: unix_ms(),
        skill_id: spec.id.clone(),
        spec_schema: spec.schema.clone(),
        event: event.kind(),
        event_name: event.name().to_owned(),
        data: serde_json::to_value(event)?,
    })
}

fn write_event(events_dir: &Path, envelope: &TraceEnvelope) -> Result<()> {
    let filename = format!("{:06}.{}.json", envelope.seq, envelope.event_name);
    let final_path = events_dir.join(filename);
    let temp_path = final_path.with_extension("json.tmp");
    let content = serde_json::to_vec_pretty(envelope)?;
    fs::write(&temp_path, content).map_err(|source| Error::Write {
        path: temp_path.clone(),
        source,
    })?;
    fs::rename(&temp_path, &final_path).map_err(|source| Error::Write {
        path: final_path,
        source,
    })?;
    Ok(())
}

fn summary_for(envelopes: &[TraceEnvelope]) -> TraceSummary {
    let mut events = BTreeMap::new();
    for envelope in envelopes {
        *events.entry(envelope.event.clone()).or_insert(0) += 1;
    }
    TraceSummary {
        schema: TRACE_SCHEMA.to_owned(),
        run_id: envelopes
            .first()
            .map(|envelope| envelope.run_id.clone())
            .unwrap_or_else(|| "empty".to_owned()),
        skill_id: envelopes
            .first()
            .map(|envelope| envelope.skill_id.clone())
            .unwrap_or_else(|| "unknown".to_owned()),
        event_count: envelopes.len(),
        events,
    }
}

fn new_run_id() -> String {
    format!("run-{}-{}", unix_ms(), std::process::id())
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
