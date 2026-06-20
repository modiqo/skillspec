use crate::decision::{DecisionEvent, DecisionWithEvents};
use crate::error::{Error, Result};
use crate::model::{SkillSpec, TraceConfig, TraceEventKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_sha256: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_sha256: Option<String>,
    pub event_count: usize,
    pub events: BTreeMap<TraceEventKind, usize>,
}

pub fn write_decision_trace(
    root: &Path,
    spec_path: &Path,
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
    let spec_fingerprint = spec_fingerprint(spec, spec_path)?;
    let input_sha256 = input_sha256(&decision.decision.input);
    for (index, event) in selected.iter().enumerate() {
        let envelope = envelope_for(
            spec,
            &run_id,
            index as u64 + 1,
            event,
            &spec_fingerprint,
            &input_sha256,
        )?;
        write_event(&events_dir, &envelope)?;
    }

    compact(&run_dir)
}

pub fn compact(run_dir: &Path) -> Result<TraceWriteResult> {
    let envelopes = read_envelopes(run_dir)?;

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

pub fn read_envelopes(run_dir: &Path) -> Result<Vec<TraceEnvelope>> {
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
    Ok(envelopes)
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
    spec_fingerprint: &str,
    input_sha256: &str,
) -> Result<TraceEnvelope> {
    Ok(TraceEnvelope {
        schema: TRACE_SCHEMA.to_owned(),
        run_id: run_id.to_owned(),
        seq,
        timestamp_unix_ms: unix_ms(),
        skill_id: spec.id.clone(),
        spec_schema: spec.schema.clone(),
        spec_fingerprint: Some(spec_fingerprint.to_owned()),
        input_sha256: Some(input_sha256.to_owned()),
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
    let spec_fingerprint = envelopes
        .iter()
        .find_map(|envelope| envelope.spec_fingerprint.clone());
    let input_sha256 = envelopes
        .iter()
        .find_map(|envelope| envelope.input_sha256.clone());
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
        spec_fingerprint,
        input_sha256,
        event_count: envelopes.len(),
        events,
    }
}

pub fn spec_fingerprint(spec: &SkillSpec, spec_path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"skillspec.resolved_spec/v0\n");
    hasher.update(serde_json::to_vec(spec)?);

    let spec_dir = spec_path.parent().unwrap_or_else(|| Path::new("."));
    for (id, import) in &spec.imports {
        hasher.update(b"\nimport\n");
        hasher.update(id.as_bytes());
        hasher.update(b"\n");
        hasher.update(import.path.as_bytes());
        hasher.update(b"\n");
        if let Some(section) = &import.section {
            hasher.update(section.as_bytes());
        }
        hasher.update(b"\ncontent\n");
        let path = spec_dir.join(&import.path);
        let content = fs::read(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        hasher.update(content);
    }

    Ok(format!(
        "sha256:{}",
        hex_digest(hasher.finalize().as_slice())
    ))
}

pub fn input_sha256(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("sha256:{}", hex_digest(hasher.finalize().as_slice()))
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
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
