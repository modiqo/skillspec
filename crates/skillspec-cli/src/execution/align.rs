use crate::decision;
use crate::error::{Error, Result};
use crate::model::{RouteId, SkillSpec, TraceEventKind};
use crate::trace::{self, TraceEnvelope};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

mod ledger;
mod types;

pub use types::*;

use ledger::ExecutionLedger;

const ALIGN_SCHEMA: &str = "skillspec.align/v0";
const PROOF_DIGEST_SCHEMA: &str = "skillspec.align.proof_digest/v0";

/// Align a loaded spec against a decision trace run directory.
///
/// This first-pass aligner proves decision reproducibility and derives
/// execution obligations. It does not parse human-readable tool transcripts;
/// absent structured execution evidence is reported as `unproven`.
pub fn align_decision_trace(
    spec: &SkillSpec,
    spec_path: &Path,
    decision_trace: &Path,
    execution_traces: &[PathBuf],
) -> Result<AlignReport> {
    let _ = trace::compact(decision_trace)?;
    let envelopes = trace::read_envelopes(decision_trace)?;
    if envelopes.is_empty() {
        return Err(Error::InvalidInput {
            message: format!("decision trace {} has no events", decision_trace.display()),
        });
    }

    let mut checks = Vec::new();
    let input = trace_input(&envelopes)?;
    let expected_input_sha256 = trace::input_sha256(&input);
    let expected_spec_fingerprint = trace::spec_fingerprint(spec, spec_path)?;
    let expected_decision = decision::decide_with_events(spec, &input).decision;

    push_eq_check(
        &mut checks,
        "skill_id",
        "trace skill id matches the current spec",
        serde_json::json!(spec.id),
        serde_json::json!(first_skill_id(&envelopes)),
    );
    push_eq_check(
        &mut checks,
        "spec_schema",
        "trace spec schema matches the current spec",
        serde_json::json!(spec.schema),
        serde_json::json!(first_spec_schema(&envelopes)),
    );
    push_optional_eq_check(
        &mut checks,
        "spec_fingerprint",
        "trace spec fingerprint matches the current resolved spec graph",
        serde_json::json!(expected_spec_fingerprint),
        first_spec_fingerprint(&envelopes),
    );
    push_optional_eq_check(
        &mut checks,
        "input_sha256",
        "trace input hash matches the captured input",
        serde_json::json!(expected_input_sha256),
        first_input_sha256(&envelopes),
    );

    let outcome = last_event_data(&envelopes, TraceEventKind::OutcomeRecorded);
    let trace_route = outcome
        .and_then(|data| data.get("route"))
        .cloned()
        .or_else(|| last_route_selected(&envelopes).and_then(|data| data.get("route").cloned()));
    push_eq_check(
        &mut checks,
        "route_selected",
        "rerunning the spec on captured input selects the same route",
        route_value(expected_decision.route.as_ref()),
        trace_route.unwrap_or(serde_json::Value::Null),
    );

    let expected_route_selection = serde_json::to_value(&expected_decision.route_selection)?;
    let trace_route_selection = outcome
        .and_then(|data| data.get("route_selection").cloned())
        .or_else(|| last_route_selected(&envelopes).and_then(route_selection_from_event));
    push_optional_eq_check(
        &mut checks,
        "route_selection_basis",
        "trace records the same route-selection basis as a fresh decision",
        expected_route_selection,
        trace_route_selection,
    );

    let expected_rules = expected_decision
        .matched_rules
        .iter()
        .map(|matched| matched.id.0.clone())
        .collect::<Vec<_>>();
    let trace_rules = outcome
        .and_then(|data| data.get("matched_rules"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    push_eq_check(
        &mut checks,
        "matched_rules",
        "rerunning the spec on captured input matches the same rules",
        serde_json::json!(expected_rules),
        trace_rules,
    );

    push_recorded_set_check(
        &mut checks,
        "forbids",
        "trace records the same forbidden substitutions as a fresh decision",
        &expected_decision.forbid,
        collect_string_array_events(&envelopes, TraceEventKind::ForbidAdded, "forbid"),
    );
    push_recorded_set_check(
        &mut checks,
        "elicitations",
        "trace records the same required elicitations as a fresh decision",
        &expected_decision.elicit,
        collect_string_array_events(&envelopes, TraceEventKind::ElicitationRequested, "elicit"),
    );
    push_recorded_set_check(
        &mut checks,
        "after_success",
        "trace records the same after-success closures as a fresh decision",
        &expected_decision.after_success,
        collect_string_array_events(
            &envelopes,
            TraceEventKind::AfterSuccessScheduled,
            "after_success",
        ),
    );
    push_background_rule_trigger_check(spec, &input, &expected_rules, &mut checks);

    let execution_ledger = ExecutionLedger::read(execution_traces)?;
    let mut obligations = obligations_for(spec, &expected_decision);
    add_user_requirement_obligations(&mut obligations, &input);
    let proof_rows = apply_execution_evidence(&mut obligations, &execution_ledger);
    let status = report_status(&checks, &obligations);
    let summary = summary_for(
        status,
        &checks,
        &obligations,
        &expected_decision,
        &execution_ledger,
    );

    Ok(AlignReport {
        schema: ALIGN_SCHEMA.to_owned(),
        ok: status != AlignStatus::Fail,
        status,
        summary,
        spec: spec_path.display().to_string(),
        decision_trace: decision_trace.display().to_string(),
        execution_traces: execution_ledger.paths.clone(),
        checks,
        obligations,
        proof_rows,
    })
}

pub fn write_report_json(decision_trace: &Path, report: &AlignReport) -> Result<PathBuf> {
    let path = decision_trace.join("alignment.json");
    let content = serde_json::to_vec_pretty(report)?;
    fs::write(&path, content).map_err(|source| Error::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

fn summary_for(
    status: AlignStatus,
    checks: &[AlignCheck],
    obligations: &[AlignObligation],
    decision: &decision::Decision,
    execution_ledger: &ExecutionLedger,
) -> AlignSummary {
    let has_execution_trace = execution_ledger.has_events();
    let decision_checks = status_counts(checks.iter().map(|check| check.status));
    let execution_obligations =
        status_counts(obligations.iter().map(|obligation| obligation.status));
    let unproven_obligation_kinds = unproven_obligation_kinds(obligations);
    let scope = if has_execution_trace {
        AlignScope::DecisionAndExecutionTrace
    } else {
        AlignScope::DecisionTraceOnly
    };
    let decision_alignment = layer_status(&decision_checks);
    let execution_alignment = if has_execution_trace {
        layer_status(&execution_obligations)
    } else {
        AlignLayerStatus::NotEvaluated
    };
    let conclusion = align_conclusion(
        status,
        &decision_checks,
        &execution_obligations,
        has_execution_trace,
    );
    let status_meaning = align_status_meaning(
        status,
        &decision_checks,
        &execution_obligations,
        has_execution_trace,
    );
    let layers = align_layers(
        &decision_checks,
        &execution_obligations,
        has_execution_trace,
    );
    let evidence_gaps = evidence_gaps(checks, obligations);
    let phase_requirement_gaps =
        phase_requirement_gaps(decision, execution_ledger, has_execution_trace);
    let completion = completion_summary_for(
        status,
        decision,
        &decision_checks,
        execution_ledger,
        has_execution_trace,
        &evidence_gaps,
    );
    let tokens = execution_ledger.token_summary();

    AlignSummary {
        scope,
        decision_alignment,
        execution_alignment,
        conclusion,
        status_meaning,
        layers,
        selected_route: decision.route.as_ref().map(|route| route.0.clone()),
        route_selection_basis: decision
            .route_selection
            .as_ref()
            .map(|selection| route_selection_basis_name(&selection.basis).to_owned()),
        route_selection_rule: decision
            .route_selection
            .as_ref()
            .and_then(|selection| selection.rule_id.as_ref())
            .map(|rule| rule.0.clone()),
        matched_rules: decision
            .matched_rules
            .iter()
            .map(|matched| matched.id.0.clone())
            .collect(),
        decision_checks,
        execution_obligations,
        unproven_obligation_kinds,
        evidence_gaps,
        phase_requirement_gaps,
        completion,
        tokens,
    }
}

/// Build a grouped missing-proof digest so agents can batch final proof rows
/// once, then run one final alignment.
pub fn build_proof_digest(report: &AlignReport, alignment_report: &Path) -> AlignProofDigest {
    let suggested_batch_file = Path::new(&report.decision_trace)
        .join("final-proof.jsonl")
        .display()
        .to_string();
    let mut groups = Vec::new();

    if !report.summary.phase_requirement_gaps.is_empty() {
        let items = report
            .summary
            .phase_requirement_gaps
            .iter()
            .map(|gap| AlignProofDigestItem {
                id: gap.requirement.clone(),
                source: format!("phase `{}` requirement `{}`", gap.phase, gap.requirement),
                needed: gap.needed.clone(),
                phase: Some(gap.phase.clone()),
                requirement: Some(gap.requirement.clone()),
                obligation_kind: None,
                recommended_event: Some("requirement_satisfied".to_owned()),
                required_fields: vec![
                    "event=requirement_satisfied".to_owned(),
                    "phase".to_owned(),
                    "requirement".to_owned(),
                    "evidence.kind".to_owned(),
                    "evidence.ref".to_owned(),
                ],
                expected_evidence: None,
                observed_evidence: None,
                note: match gap.status {
                    AlignPhaseRequirementGapStatus::Failed => "Fix or re-run the failed requirement, then record real satisfied evidence; do not mark it pass from the failure row.".to_owned(),
                    AlignPhaseRequirementGapStatus::Missing => "Record this only if real evidence exists that the requirement was satisfied.".to_owned(),
                    AlignPhaseRequirementGapStatus::NotEvaluated => "Supply an execution ledger first, then record this only if real evidence exists.".to_owned(),
                },
            })
            .collect::<Vec<_>>();
        groups.push(AlignProofDigestGroup {
            kind: "phase_requirement".to_owned(),
            count: items.len(),
            recommended_event: Some("requirement_satisfied".to_owned()),
            required_fields: vec![
                "event".to_owned(),
                "phase".to_owned(),
                "requirement".to_owned(),
                "evidence.kind".to_owned(),
                "evidence.ref".to_owned(),
            ],
            items,
        });
    }

    let mut by_group: BTreeMap<String, Vec<AlignProofDigestItem>> = BTreeMap::new();
    for gap in &report.summary.evidence_gaps {
        let (group, event, required_fields, note) = proof_digest_group_for(gap);
        let proof = proof_row_for_gap(report, gap);
        by_group
            .entry(group.to_owned())
            .or_default()
            .push(AlignProofDigestItem {
                id: gap.id.clone(),
                source: gap.source.clone(),
                needed: gap.needed.clone(),
                phase: None,
                requirement: None,
                obligation_kind: gap.obligation_kind,
                recommended_event: event.map(str::to_owned),
                required_fields: required_fields
                    .iter()
                    .map(|field| (*field).to_owned())
                    .collect(),
                expected_evidence: proof.map(|row| row.expected_evidence.clone()),
                observed_evidence: proof.map(|row| row.observed_evidence.clone()),
                note: note.to_owned(),
            });
    }

    for (kind, items) in by_group {
        let recommended_event = items.iter().find_map(|item| item.recommended_event.clone());
        let required_fields = items
            .first()
            .map(|item| item.required_fields.clone())
            .unwrap_or_default();
        groups.push(AlignProofDigestGroup {
            kind,
            count: items.len(),
            recommended_event,
            required_fields,
            items,
        });
    }

    let missing_count = groups.iter().map(|group| group.count).sum();
    AlignProofDigest {
        schema: PROOF_DIGEST_SCHEMA.to_owned(),
        status: report.status,
        alignment: report.summary.completion.alignment.clone(),
        alignment_report: alignment_report.display().to_string(),
        suggested_batch_file,
        missing_count,
        recommended_loop: vec![
            "Run trace align once with --summary --proof-digest.".to_owned(),
            "Read this digest and create final-proof.jsonl with real evidence for every batchable item.".to_owned(),
            "Run skillspec progress batch <run-dir> --file <run-dir>/final-proof.jsonl --checkpoint \"checkpointing evidence\" --summary once in the foreground.".to_owned(),
            "Run trace align --summary once more and report only the compact final summary.".to_owned(),
            "Do not rerun alignment after each individual proof row.".to_owned(),
        ],
        groups,
    }
}

pub fn write_proof_digest_json(path: &Path, digest: &AlignProofDigest) -> Result<PathBuf> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let content = serde_json::to_vec_pretty(digest)?;
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(path.to_path_buf())
}

fn completion_summary_for(
    status: AlignStatus,
    decision: &decision::Decision,
    decision_checks: &AlignStatusCounts,
    ledger: &ExecutionLedger,
    has_execution_trace: bool,
    evidence_gaps: &[AlignEvidenceGap],
) -> AlignCompletionSummary {
    let requirement_summary = requirement_completion_summary(decision, ledger, has_execution_trace);
    let mut missing_proof = requirement_summary.missing_proof;
    if missing_proof.is_empty() {
        missing_proof.extend(evidence_gaps.iter().take(3).map(|gap| {
            format!(
                "{} `{}` needs {}",
                compact_evidence_gap_kind_name(gap.kind),
                gap.id,
                gap.needed
            )
        }));
    }
    if missing_proof.is_empty() {
        missing_proof.push("none".to_owned());
    }

    AlignCompletionSummary {
        decision_replay: compact_layer_status(layer_status(decision_checks)).to_owned(),
        phase_order: phase_order_summary(decision, ledger, has_execution_trace),
        requirements: requirement_summary.requirements,
        missing_proof,
        forbidden_actions: forbidden_actions_summary(ledger, has_execution_trace),
        alignment: terminal_alignment_status(status).to_owned(),
    }
}

#[derive(Clone, Debug)]
struct RequirementCompletionSummary {
    requirements: String,
    missing_proof: Vec<String>,
}

fn requirement_completion_summary(
    decision: &decision::Decision,
    ledger: &ExecutionLedger,
    has_execution_trace: bool,
) -> RequirementCompletionSummary {
    let requirements = phase_requirements(decision);
    if requirements.is_empty() {
        return RequirementCompletionSummary {
            requirements: "none declared".to_owned(),
            missing_proof: Vec::new(),
        };
    }

    let mut proven = 0_usize;
    let mut failed = 0_usize;
    let mut missing = Vec::new();
    for (phase, requirement) in &requirements {
        if ledger.has_requirement_satisfied(phase, requirement) {
            proven += 1;
        } else if ledger.has_requirement_failed(phase, requirement) {
            failed += 1;
            missing.push(format!(
                "requirement `{requirement}` in phase `{phase}` has a failed progress event"
            ));
        } else if has_execution_trace {
            missing.push(format!(
                "requirement `{requirement}` in phase `{phase}` has no progress event"
            ));
        } else {
            missing.push(format!(
                "requirement `{requirement}` in phase `{phase}` was not checked; no execution trace supplied"
            ));
        }
    }

    let requirements = if failed > 0 {
        format!("{proven}/{} proven, {failed} failed", requirements.len())
    } else {
        format!("{proven}/{} proven", requirements.len())
    };

    RequirementCompletionSummary {
        requirements,
        missing_proof: missing,
    }
}

fn phase_requirement_gaps(
    decision: &decision::Decision,
    ledger: &ExecutionLedger,
    has_execution_trace: bool,
) -> Vec<AlignPhaseRequirementGap> {
    phase_requirements(decision)
        .into_iter()
        .filter_map(|(phase, requirement)| {
            if ledger.has_requirement_satisfied(&phase, &requirement) {
                return None;
            }
            if ledger.has_requirement_failed(&phase, &requirement) {
                return Some(AlignPhaseRequirementGap {
                    phase: phase.clone(),
                    requirement: requirement.clone(),
                    status: AlignPhaseRequirementGapStatus::Failed,
                    needed: format!(
                        "requirement `{requirement}` in phase `{phase}` has a failed progress event"
                    ),
                });
            }
            let status = if has_execution_trace {
                AlignPhaseRequirementGapStatus::Missing
            } else {
                AlignPhaseRequirementGapStatus::NotEvaluated
            };
            let needed = if has_execution_trace {
                format!("requirement `{requirement}` in phase `{phase}` has no progress event")
            } else {
                format!(
                    "requirement `{requirement}` in phase `{phase}` was not checked; no execution trace supplied"
                )
            };
            Some(AlignPhaseRequirementGap {
                phase,
                requirement,
                status,
                needed,
            })
        })
        .collect()
}

fn phase_requirements(decision: &decision::Decision) -> Vec<(String, String)> {
    decision
        .execution_plan
        .as_ref()
        .map(|plan| {
            plan.phases
                .iter()
                .flat_map(|phase| {
                    phase
                        .requires
                        .iter()
                        .map(|requirement| (phase.id.clone(), requirement.clone()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn phase_order_summary(
    decision: &decision::Decision,
    ledger: &ExecutionLedger,
    has_execution_trace: bool,
) -> String {
    let expected = decision
        .execution_plan
        .as_ref()
        .map(|plan| {
            plan.phases
                .iter()
                .map(|phase| phase.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if expected.is_empty() {
        return "not applicable".to_owned();
    }
    if !has_execution_trace {
        return "not evaluated".to_owned();
    }
    let phase_events = ledger.phase_events();
    if phase_events.is_empty() {
        return "partial; no phase progress events recorded".to_owned();
    }

    let expected_index = expected
        .iter()
        .enumerate()
        .map(|(index, phase)| (phase.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut last_index = None;
    for event in phase_events {
        let Some(phase) = event.phase.as_deref() else {
            continue;
        };
        let Some(index) = expected_index.get(phase).copied() else {
            return format!("fail; unexpected phase `{phase}` recorded");
        };
        if last_index.is_some_and(|last| index < last) {
            return format!("fail; phase `{phase}` was recorded out of order");
        }
        last_index = Some(index);
    }
    "pass".to_owned()
}

fn forbidden_actions_summary(ledger: &ExecutionLedger, has_execution_trace: bool) -> String {
    if !has_execution_trace {
        return "not checked; no execution trace supplied".to_owned();
    }
    let violations = ledger.forbidden_violation_count();
    if violations == 0 {
        "no violations recorded".to_owned()
    } else {
        format!("{violations} violation event(s) recorded")
    }
}

fn compact_layer_status(status: AlignLayerStatus) -> &'static str {
    match status {
        AlignLayerStatus::Pass => "pass",
        AlignLayerStatus::Fail => "fail",
        AlignLayerStatus::Incomplete => "partial",
        AlignLayerStatus::NotEvaluated => "not evaluated",
    }
}

fn terminal_alignment_status(status: AlignStatus) -> &'static str {
    match status {
        AlignStatus::Pass => "pass",
        AlignStatus::Fail => "fail",
        AlignStatus::Unproven => "partial",
    }
}

fn compact_evidence_gap_kind_name(kind: AlignEvidenceGapKind) -> &'static str {
    match kind {
        AlignEvidenceGapKind::DecisionTrace => "decision trace",
        AlignEvidenceGapKind::ExecutionObligation => "execution obligation",
    }
}

fn align_status_meaning(
    status: AlignStatus,
    decision_checks: &AlignStatusCounts,
    execution_obligations: &AlignStatusCounts,
    has_execution_trace: bool,
) -> String {
    match status {
        AlignStatus::Pass => "pass means the current spec reproduced the decision trace and the supplied execution evidence proves every active obligation".to_owned(),
        AlignStatus::Fail if decision_checks.fail > 0 => {
            "fail means the current spec no longer reproduces the recorded decision trace; treat this as spec drift or a trace/spec mismatch".to_owned()
        }
        AlignStatus::Fail => {
            "fail means supplied execution evidence contradicts at least one active obligation from the decision".to_owned()
        }
        AlignStatus::Unproven if !has_execution_trace && decision_checks.unproven == 0 => {
            "decision alignment passed; execution was not evaluated because no execution trace was supplied".to_owned()
        }
        AlignStatus::Unproven if !has_execution_trace => {
            "decision alignment is incomplete because the reasoning trace is missing deterministic facts; execution was not evaluated because no execution trace was supplied".to_owned()
        }
        AlignStatus::Unproven if execution_obligations.unproven > 0 => {
            "decision alignment passed or had no failures; supplied execution evidence is incomplete for one or more active obligations".to_owned()
        }
        AlignStatus::Unproven => {
            "alignment is incomplete because required evidence is missing, but no contradiction was found".to_owned()
        }
    }
}

fn align_layers(
    decision_checks: &AlignStatusCounts,
    execution_obligations: &AlignStatusCounts,
    has_execution_trace: bool,
) -> Vec<AlignLayerSummary> {
    vec![
        AlignLayerSummary {
            id: AlignLayerKind::DecisionReplay,
            label: "decision replay".to_owned(),
            measures: "Re-run the current resolved SkillSpec on the captured input, then compare identity, route selection, matched rules, forbids, elicitations, and after-success scheduling against the trace.".to_owned(),
            interpretation: layer_interpretation(
                decision_checks,
                "decision replay",
                "the spec-to-input decision is reproducible",
                "the decision trace is missing some deterministic facts",
                "the current spec no longer reproduces the recorded decision",
            ),
            counts: decision_checks.clone(),
        },
        AlignLayerSummary {
            id: AlignLayerKind::ExecutionProof,
            label: "execution proof".to_owned(),
            measures: "When an execution trace is supplied, derive obligations from the selected route and matched rules, then check structured evidence that the route/checks/closures were fulfilled and forbids were not violated.".to_owned(),
            interpretation: if has_execution_trace {
                layer_interpretation(
                    execution_obligations,
                    "execution proof",
                    "every active execution obligation has structured proof",
                    "the supplied execution evidence is incomplete",
                    "structured execution evidence contradicts the active contract",
                )
            } else {
                "execution proof: not evaluated because no execution trace was supplied".to_owned()
            },
            counts: execution_obligations.clone(),
        },
    ]
}

fn layer_status(counts: &AlignStatusCounts) -> AlignLayerStatus {
    if counts.fail > 0 {
        AlignLayerStatus::Fail
    } else if counts.unproven > 0 {
        AlignLayerStatus::Incomplete
    } else {
        AlignLayerStatus::Pass
    }
}

fn layer_interpretation(
    counts: &AlignStatusCounts,
    label: &str,
    pass_text: &str,
    unproven_text: &str,
    fail_text: &str,
) -> String {
    if counts.fail > 0 {
        format!("{label}: {fail_text}")
    } else if counts.unproven > 0 {
        format!("{label}: {unproven_text}")
    } else {
        format!("{label}: {pass_text}")
    }
}

fn route_selection_basis_name(basis: &decision::RouteSelectionBasis) -> &'static str {
    match basis {
        decision::RouteSelectionBasis::RulePrefer => "rule_prefer",
        decision::RouteSelectionBasis::RouteOrderDefault => "route_order_default",
        decision::RouteSelectionBasis::DefaultRouteOrder => "default_route_order",
    }
}

fn status_counts(statuses: impl IntoIterator<Item = AlignCheckStatus>) -> AlignStatusCounts {
    let mut counts = AlignStatusCounts::default();
    for status in statuses {
        counts.total += 1;
        match status {
            AlignCheckStatus::Pass => counts.pass += 1,
            AlignCheckStatus::Fail => counts.fail += 1,
            AlignCheckStatus::Unproven => counts.unproven += 1,
        }
    }
    counts
}

fn unproven_obligation_kinds(obligations: &[AlignObligation]) -> Vec<AlignObligationKindCount> {
    let mut counts: BTreeMap<AlignObligationKind, AlignObligationKindCount> = BTreeMap::new();
    for obligation in obligations {
        let entry = counts
            .entry(obligation.kind)
            .or_insert(AlignObligationKindCount {
                kind: obligation.kind,
                total: 0,
                unproven: 0,
            });
        entry.total += 1;
        if obligation.status == AlignCheckStatus::Unproven {
            entry.unproven += 1;
        }
    }
    counts
        .into_values()
        .filter(|count| count.unproven > 0)
        .collect()
}

fn evidence_gaps(checks: &[AlignCheck], obligations: &[AlignObligation]) -> Vec<AlignEvidenceGap> {
    let mut gaps = Vec::new();
    for check in checks
        .iter()
        .filter(|check| check.status == AlignCheckStatus::Unproven)
    {
        gaps.push(AlignEvidenceGap {
            id: check.id.clone(),
            kind: AlignEvidenceGapKind::DecisionTrace,
            obligation_kind: None,
            source: format!("checks.{}", check.id),
            needed: check.message.clone(),
        });
    }
    for obligation in obligations
        .iter()
        .filter(|obligation| obligation.status == AlignCheckStatus::Unproven)
    {
        gaps.push(AlignEvidenceGap {
            id: obligation.id.clone(),
            kind: AlignEvidenceGapKind::ExecutionObligation,
            obligation_kind: Some(obligation.kind),
            source: obligation.source.clone(),
            needed: obligation.message.clone(),
        });
    }
    gaps
}

fn proof_digest_group_for(
    gap: &AlignEvidenceGap,
) -> (
    &'static str,
    Option<&'static str>,
    Vec<&'static str>,
    &'static str,
) {
    match (gap.kind, gap.obligation_kind) {
        (AlignEvidenceGapKind::DecisionTrace, _) => (
            "decision_trace",
            None,
            vec![],
            "Progress events cannot prove missing deterministic decision facts; recreate or compact the decision trace instead.",
        ),
        (AlignEvidenceGapKind::ExecutionObligation, Some(AlignObligationKind::Route)) => (
            "route_fulfillment",
            Some("route_fulfilled"),
            vec!["event=route_fulfilled", "id", "status=pass", "evidence.kind", "evidence.ref"],
            "Record this only after the selected route actually completed.",
        ),
        (AlignEvidenceGapKind::ExecutionObligation, Some(AlignObligationKind::RouteCheck)) => (
            "route_check",
            Some("route_check_completed"),
            vec![
                "event=route_check_completed",
                "id",
                "status=pass",
                "evidence.kind",
                "evidence.ref",
            ],
            "Record this only after the route check actually passed.",
        ),
        (AlignEvidenceGapKind::ExecutionObligation, Some(AlignObligationKind::Forbid)) => (
            "forbid_no_violation",
            Some("obligation_satisfied"),
            vec![
                "event=obligation_satisfied",
                "id",
                "status=pass",
                "evidence.kind=no_violation",
                "evidence.ref",
            ],
            "Record this only when the execution ledger or reviewed artifacts prove the forbidden action did not happen.",
        ),
        (AlignEvidenceGapKind::ExecutionObligation, Some(AlignObligationKind::Elicitation)) => (
            "elicitation",
            Some("elicitation_answered or elicitation_waived"),
            vec![
                "event=elicitation_answered|elicitation_waived",
                "id",
                "status=pass",
                "evidence.kind",
                "evidence.ref",
            ],
            "Use answered when the user supplied the answer; use waived only when the user or route explicitly allowed a waiver.",
        ),
        (AlignEvidenceGapKind::ExecutionObligation, Some(AlignObligationKind::AfterSuccess)) => (
            "after_success",
            Some("after_success_completed"),
            vec![
                "event=after_success_completed",
                "id",
                "status=pass",
                "evidence.kind",
                "evidence.ref",
            ],
            "Record this only after the closure actually completed.",
        ),
        (AlignEvidenceGapKind::ExecutionObligation, Some(AlignObligationKind::UserRequirement))
        | (AlignEvidenceGapKind::ExecutionObligation, None) => (
            "execution_obligation",
            Some("obligation_satisfied"),
            vec!["event=obligation_satisfied", "id", "status=pass", "evidence.kind", "evidence.ref"],
            "Record this only when concrete evidence proves the obligation.",
        ),
    }
}

fn proof_row_for_gap<'a>(
    report: &'a AlignReport,
    gap: &AlignEvidenceGap,
) -> Option<&'a AlignProofRow> {
    let prefix = format!("{} (", gap.id);
    report
        .proof_rows
        .iter()
        .find(|row| row.obligation.starts_with(&prefix))
}

fn align_conclusion(
    status: AlignStatus,
    checks: &AlignStatusCounts,
    obligations: &AlignStatusCounts,
    has_execution_trace: bool,
) -> String {
    match status {
        AlignStatus::Pass => {
            "decision alignment passed and supplied execution evidence proves every active obligation".to_owned()
        }
        AlignStatus::Fail => format!(
            "{} deterministic check(s) failed; the current spec no longer aligns with the decision trace",
            checks.fail
        ),
        AlignStatus::Unproven => align_unproven_conclusion(checks, obligations, has_execution_trace),
    }
}

fn align_unproven_conclusion(
    checks: &AlignStatusCounts,
    obligations: &AlignStatusCounts,
    has_execution_trace: bool,
) -> String {
    if !has_execution_trace {
        if checks.unproven == 0 {
            return "decision alignment passed; execution was not evaluated because no execution trace was supplied".to_owned();
        }
        return format!(
            "decision alignment incomplete: {} deterministic trace check(s) are missing from the reasoning record; execution was not evaluated because no execution trace was supplied",
            checks.unproven
        );
    }

    let mut gaps = Vec::new();
    if checks.unproven > 0 {
        gaps.push(format!("{} deterministic trace check(s)", checks.unproven));
    }
    if obligations.unproven > 0 {
        gaps.push(format!("{} execution obligation(s)", obligations.unproven));
    }
    let gap_text = match gaps.len() {
        0 => "required evidence".to_owned(),
        1 => gaps[0].clone(),
        _ => format!(
            "{} and {}",
            gaps[..gaps.len() - 1].join(", "),
            gaps[gaps.len() - 1]
        ),
    };
    format!(
        "decision replay found no deterministic drift, but proof is incomplete: {gap_text} remain unproven"
    )
}

fn trace_input(envelopes: &[TraceEnvelope]) -> Result<String> {
    last_event_data(envelopes, TraceEventKind::InputReceived)
        .and_then(|data| data.get("input"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::InvalidInput {
            message: "decision trace does not contain an input_received event with input"
                .to_owned(),
        })
}

fn first_skill_id(envelopes: &[TraceEnvelope]) -> String {
    envelopes
        .first()
        .map(|envelope| envelope.skill_id.clone())
        .unwrap_or_default()
}

fn first_spec_schema(envelopes: &[TraceEnvelope]) -> String {
    envelopes
        .first()
        .map(|envelope| envelope.spec_schema.clone())
        .unwrap_or_default()
}

fn first_spec_fingerprint(envelopes: &[TraceEnvelope]) -> Option<serde_json::Value> {
    envelopes
        .iter()
        .find_map(|envelope| envelope.spec_fingerprint.clone())
        .map(serde_json::Value::String)
}

fn first_input_sha256(envelopes: &[TraceEnvelope]) -> Option<serde_json::Value> {
    envelopes
        .iter()
        .find_map(|envelope| envelope.input_sha256.clone())
        .map(serde_json::Value::String)
}

fn last_event_data(
    envelopes: &[TraceEnvelope],
    event: TraceEventKind,
) -> Option<&serde_json::Value> {
    envelopes
        .iter()
        .rev()
        .find(|envelope| envelope.event == event)
        .map(|envelope| &envelope.data)
}

fn last_route_selected(envelopes: &[TraceEnvelope]) -> Option<&serde_json::Value> {
    last_event_data(envelopes, TraceEventKind::RouteSelected)
}

fn route_selection_from_event(data: &serde_json::Value) -> Option<serde_json::Value> {
    data.get("basis")?;
    let mut selection = serde_json::Map::new();
    if let Some(route) = data.get("route") {
        selection.insert("route".to_owned(), route.clone());
    }
    if let Some(basis) = data.get("basis") {
        selection.insert("basis".to_owned(), basis.clone());
    }
    if let Some(rule_id) = data.get("rule_id") {
        selection.insert("rule_id".to_owned(), rule_id.clone());
    }
    if let Some(reason) = data.get("reason") {
        selection.insert("reason".to_owned(), reason.clone());
    }
    Some(serde_json::Value::Object(selection))
}

fn route_value(route: Option<&RouteId>) -> serde_json::Value {
    route
        .map(|route| serde_json::Value::String(route.0.clone()))
        .unwrap_or(serde_json::Value::Null)
}

fn push_eq_check(
    checks: &mut Vec<AlignCheck>,
    id: &str,
    message: &str,
    expected: serde_json::Value,
    actual: serde_json::Value,
) {
    let status = if expected == actual {
        AlignCheckStatus::Pass
    } else {
        AlignCheckStatus::Fail
    };
    checks.push(AlignCheck {
        id: id.to_owned(),
        status,
        message: message.to_owned(),
        expected: Some(expected),
        actual: Some(actual),
    });
}

fn push_optional_eq_check(
    checks: &mut Vec<AlignCheck>,
    id: &str,
    message: &str,
    expected: serde_json::Value,
    actual: Option<serde_json::Value>,
) {
    match actual {
        Some(actual) => push_eq_check(checks, id, message, expected, actual),
        None => checks.push(AlignCheck {
            id: id.to_owned(),
            status: AlignCheckStatus::Unproven,
            message: format!("{message}; trace did not record this field"),
            expected: Some(expected),
            actual: None,
        }),
    }
}

fn push_recorded_set_check(
    checks: &mut Vec<AlignCheck>,
    id: &str,
    message: &str,
    expected: &[String],
    actual: Option<Vec<String>>,
) {
    let expected_set = expected.iter().cloned().collect::<BTreeSet<_>>();
    match actual {
        Some(actual) => {
            let actual_set = actual.into_iter().collect::<BTreeSet<_>>();
            push_eq_check(
                checks,
                id,
                message,
                serde_json::json!(expected_set),
                serde_json::json!(actual_set),
            );
        }
        None if expected.is_empty() => checks.push(AlignCheck {
            id: id.to_owned(),
            status: AlignCheckStatus::Pass,
            message: format!("{message}; no obligations were expected"),
            expected: Some(serde_json::json!([])),
            actual: None,
        }),
        None => checks.push(AlignCheck {
            id: id.to_owned(),
            status: AlignCheckStatus::Unproven,
            message: format!("{message}; trace did not record this event kind"),
            expected: Some(serde_json::json!(expected)),
            actual: None,
        }),
    }
}

fn collect_string_array_events(
    envelopes: &[TraceEnvelope],
    event: TraceEventKind,
    field: &str,
) -> Option<Vec<String>> {
    let mut saw_event = false;
    let mut values = Vec::new();
    for envelope in envelopes.iter().filter(|envelope| envelope.event == event) {
        saw_event = true;
        if let Some(items) = envelope
            .data
            .get(field)
            .and_then(serde_json::Value::as_array)
        {
            values.extend(
                items
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned),
            );
        }
    }
    saw_event.then_some(values)
}

fn user_requires_tracked_background(input: &str) -> bool {
    let normalized = input.to_ascii_lowercase();
    [
        "tracked background",
        "background process",
        "in the background",
        "as background",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn push_background_rule_trigger_check(
    spec: &SkillSpec,
    input: &str,
    matched_rules: &[String],
    checks: &mut Vec<AlignCheck>,
) {
    let rule_id = "long_noninteractive_jobs_use_background";
    if !user_requires_tracked_background(input) {
        return;
    }
    if !spec.rules.iter().any(|rule| rule.id.0 == rule_id) {
        return;
    }
    let expected = serde_json::json!(rule_id);
    let actual = serde_json::json!(matched_rules);
    let status = if matched_rules.iter().any(|rule| rule == rule_id) {
        AlignCheckStatus::Pass
    } else {
        AlignCheckStatus::Fail
    };
    checks.push(AlignCheck {
        id: "tracked_background_rule_triggered".to_owned(),
        status,
        message:
            "a request for a tracked background process should activate the background-process rule"
                .to_owned(),
        expected: Some(expected),
        actual: Some(actual),
    });
}

fn add_user_requirement_obligations(obligations: &mut Vec<AlignObligation>, input: &str) {
    if user_requires_tracked_background(input) {
        obligations.push(AlignObligation {
            id: "user_tracked_background_process".to_owned(),
            kind: AlignObligationKind::UserRequirement,
            status: AlignCheckStatus::Unproven,
            source: "user.input".to_owned(),
            message: "user explicitly requested a tracked background process".to_owned(),
        });
    }
}

fn apply_execution_evidence(
    obligations: &mut [AlignObligation],
    ledger: &ExecutionLedger,
) -> Vec<AlignProofRow> {
    obligations
        .iter_mut()
        .map(|obligation| {
            let proof = evaluate_obligation(obligation, ledger);
            obligation.status = proof_status_to_check_status(proof.status);
            proof
        })
        .collect()
}

fn proof_status_to_check_status(status: AlignProofStatus) -> AlignCheckStatus {
    match status {
        AlignProofStatus::Satisfied => AlignCheckStatus::Pass,
        AlignProofStatus::PartiallySatisfied | AlignProofStatus::Unproven => {
            AlignCheckStatus::Unproven
        }
        AlignProofStatus::Violated => AlignCheckStatus::Fail,
    }
}

fn evaluate_obligation(obligation: &AlignObligation, ledger: &ExecutionLedger) -> AlignProofRow {
    if !ledger.has_events() {
        return proof_row(
            obligation,
            expected_evidence_for(obligation),
            "no execution trace was supplied".to_owned(),
            AlignProofStatus::Unproven,
            "decision replay can run without execution proof, but alignment cannot prove actions without a structured execution ledger".to_owned(),
        );
    }

    match obligation.id.as_str() {
        "adapter_first_cli_fallback" => evaluate_adapter_cli_route_obligation(obligation, ledger),
        "background_process" | "one_shot_process" => {
            evaluate_process_route_obligation(obligation, ledger)
        }
        "user_tracked_background_process" | "long_noninteractive_jobs_use_background" => {
            evaluate_background_obligation(obligation, ledger)
        }
        "cli_invocations_use_rote_exec"
        | "run_cli_only_through_rote_exec"
        | "direct_cli_without_rote_exec"
        | "direct_shell_command_without_rote_exec"
        | "direct_harness_cli_call_without_rote_exec" => {
            evaluate_rote_exec_obligation(obligation, ledger)
        }
        "untracked_stdout_scrollback" => evaluate_output_capture_obligation(obligation, ledger),
        "external_service_tasks_are_adapter_first"
        | "skipping_adapter_discovery"
        | "discover_relevant_rote_adapters"
        | "identify_required_services_and_tools" => {
            evaluate_adapter_discovery_obligation(obligation, ledger)
        }
        "skipping_cli_readiness_check"
        | "verify_adapter_or_cli_readiness"
        | "preflight_cli_fallback" => evaluate_cli_readiness_obligation(obligation, ledger),
        "durable_work_requires_named_workspace"
        | "rote_exec_outside_workspace"
        | "anonymous_workspace" => evaluate_workspace_obligation(obligation, ledger),
        "compute_workspace_trace" => evaluate_workspace_trace_obligation(obligation, ledger),
        "compute_workspace_stats" => evaluate_stats_obligation(obligation, ledger),
        "report_workspace_evidence_and_token_math"
        | "record_final_response_sent_event"
        | "final_summary_without_trace_math"
        | "final_summary_without_workspace"
        | "summarize_evidence" => evaluate_final_response_obligation(obligation, ledger),
        "direct_mcp_tool_call" => evaluate_forbidden_event_absence(
            obligation,
            ledger,
            &["mcp_tool_call"],
            "no direct MCP tool call event was present",
        ),
        "native_search_as_answer" => evaluate_forbidden_event_absence(
            obligation,
            ledger,
            &["native_search"],
            "no native search-as-answer event was present",
        ),
        "native_codex_web_search" => evaluate_forbidden_event_absence(
            obligation,
            ledger,
            &["codex_web_search", "native_codex_web_search"],
            "no native Codex web-search event was present",
        ),
        _ => evaluate_generic_obligation(obligation, ledger),
    }
}

fn evaluate_adapter_cli_route_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    let discovery = ledger.has_adapter_discovery();
    let readiness = ledger.cli_readiness_ready();
    let rote_exec = ledger.all_processes_use_rote_exec() == Some(true);
    if discovery && readiness && rote_exec {
        proof_row(
            obligation,
            "adapter discovery, CLI readiness, and rote_exec process evidence".to_owned(),
            format!(
                "{}; {}; CLI readiness passed",
                ledger
                    .adapter_discovery_summary()
                    .unwrap_or_else(|| "adapter discovery ran".to_owned()),
                ledger.command_summary()
            ),
            AlignProofStatus::Satisfied,
            "the selected adapter-first CLI fallback route is fully proven".to_owned(),
        )
    } else if discovery || readiness || ledger.has_process_start() {
        proof_row(
            obligation,
            "adapter discovery, CLI readiness, and rote_exec process evidence".to_owned(),
            format!(
                "adapter_discovery={}, cli_readiness={}, rote_exec={}",
                discovery, readiness, rote_exec
            ),
            AlignProofStatus::PartiallySatisfied,
            "some route evidence exists, but the selected route is not fully proven".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "adapter discovery and CLI fallback execution evidence".to_owned(),
            "no route execution evidence was present".to_owned(),
            AlignProofStatus::Unproven,
            "the ledger does not prove the selected route was executed".to_owned(),
        )
    }
}

fn evaluate_process_route_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    if ledger.has_process_start() {
        proof_row(
            obligation,
            "process execution event with captured output".to_owned(),
            ledger.command_summary(),
            AlignProofStatus::Satisfied,
            "the route has process execution evidence".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "process execution event".to_owned(),
            "no process execution evidence was present".to_owned(),
            AlignProofStatus::Unproven,
            "the ledger does not prove this process route ran".to_owned(),
        )
    }
}

fn evaluate_background_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    let observed = ledger
        .background_lease_summary()
        .unwrap_or_else(|| "no background process lease evidence".to_owned());
    if ledger.has_background_start() && ledger.has_background_terminal_event() {
        proof_row(
            obligation,
            "background_process_started plus process_wait_finished or process_status_checked for the same work".to_owned(),
            observed,
            AlignProofStatus::Satisfied,
            "the execution ledger proves the process was tracked in the background and reached an observed terminal/status state".to_owned(),
        )
    } else if ledger.has_background_start() {
        proof_row(
            obligation,
            "background process start and follow-up status/wait evidence".to_owned(),
            observed,
            AlignProofStatus::PartiallySatisfied,
            "a background lease was created, but the ledger does not prove it was waited or status-checked".to_owned(),
        )
    } else if ledger.has_process_start() {
        proof_row(
            obligation,
            "tracked background process lease".to_owned(),
            format!("{}; no background lease", ledger.command_summary()),
            AlignProofStatus::Violated,
            "a process ran, but the ledger shows no tracked background lease for a background-process requirement".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "tracked background process lease".to_owned(),
            observed,
            AlignProofStatus::Unproven,
            "no process execution evidence was available".to_owned(),
        )
    }
}

fn evaluate_rote_exec_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    let observed = ledger.command_summary();
    if ledger.any_direct_process() {
        return proof_row(
            obligation,
            "process events executed by rote_exec, with command arguments redacted".to_owned(),
            observed,
            AlignProofStatus::Violated,
            "at least one process event was marked as direct harness/CLI/shell execution"
                .to_owned(),
        );
    }
    match ledger.all_processes_use_rote_exec() {
        Some(true) => proof_row(
            obligation,
            "process events executed by rote_exec, with command arguments redacted".to_owned(),
            observed,
            AlignProofStatus::Satisfied,
            "all process-start events identify rote_exec/rote as the executor".to_owned(),
        ),
        Some(false) => proof_row(
            obligation,
            "process events executed by rote_exec, with command arguments redacted".to_owned(),
            observed,
            AlignProofStatus::Unproven,
            "process events exist, but they do not prove rote_exec was the executor".to_owned(),
        ),
        None => proof_row(
            obligation,
            "process events executed by rote_exec".to_owned(),
            "no process-start event was present".to_owned(),
            AlignProofStatus::Unproven,
            "there is no command execution evidence to evaluate".to_owned(),
        ),
    }
}

fn evaluate_output_capture_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    match ledger.all_process_output_captured() {
        Some(true) => proof_row(
            obligation,
            "process events with stdout or stderr captured".to_owned(),
            format!("{} with output capture", ledger.command_summary()),
            AlignProofStatus::Satisfied,
            "every process-start event captured stdout and/or stderr".to_owned(),
        ),
        Some(false) => proof_row(
            obligation,
            "process events with stdout or stderr captured".to_owned(),
            format!(
                "{} without complete output-capture proof",
                ledger.command_summary()
            ),
            AlignProofStatus::Unproven,
            "processes ran, but output capture was not proven for every process".to_owned(),
        ),
        None => proof_row(
            obligation,
            "captured stdout/stderr evidence".to_owned(),
            "no process-start event was present".to_owned(),
            AlignProofStatus::Unproven,
            "there is no process output evidence to evaluate".to_owned(),
        ),
    }
}

fn evaluate_adapter_discovery_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    if ledger.has_adapter_discovery() {
        proof_row(
            obligation,
            "adapter_discovery_finished before CLI fallback".to_owned(),
            ledger
                .adapter_discovery_summary()
                .unwrap_or_else(|| "adapter discovery event was present".to_owned()),
            AlignProofStatus::Satisfied,
            "the execution ledger proves adapter discovery happened".to_owned(),
        )
    } else if ledger.has_process_start() {
        proof_row(
            obligation,
            "adapter discovery before CLI fallback".to_owned(),
            format!("{}; no adapter discovery event", ledger.command_summary()),
            AlignProofStatus::Unproven,
            "CLI/process evidence exists, but adapter discovery is not proven".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "adapter discovery evidence".to_owned(),
            "no adapter discovery event was present".to_owned(),
            AlignProofStatus::Unproven,
            "there is no adapter-selection evidence to evaluate".to_owned(),
        )
    }
}

fn evaluate_cli_readiness_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    if ledger.cli_readiness_ready() {
        proof_row(
            obligation,
            "cli_readiness_check_finished or dependency_check_finished with ready=true/exit_code=0"
                .to_owned(),
            "CLI readiness check completed successfully".to_owned(),
            AlignProofStatus::Satisfied,
            "the execution ledger proves the CLI fallback was checked before use".to_owned(),
        )
    } else if ledger.has_cli_readiness() {
        proof_row(
            obligation,
            "successful CLI readiness evidence".to_owned(),
            "CLI readiness event exists but does not prove readiness".to_owned(),
            AlignProofStatus::Violated,
            "a readiness event was captured but indicates failure or non-ready state".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "CLI readiness or dependency check evidence".to_owned(),
            "no CLI readiness event was present".to_owned(),
            AlignProofStatus::Unproven,
            "the ledger does not prove the CLI fallback was preflighted".to_owned(),
        )
    }
}

fn evaluate_workspace_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    if ledger.any_anonymous_workspace() {
        return proof_row(
            obligation,
            "non-anonymous workspace evidence".to_owned(),
            "an anonymous workspace event was present".to_owned(),
            AlignProofStatus::Violated,
            "durable work requires a named workspace".to_owned(),
        );
    }
    match ledger.all_processes_have_workspace() {
        Some(false) => proof_row(
            obligation,
            "every durable process references a named workspace".to_owned(),
            "at least one process event had no workspace".to_owned(),
            AlignProofStatus::Violated,
            "durable process evidence must not run outside the workspace".to_owned(),
        ),
        _ if ledger.has_named_workspace() => proof_row(
            obligation,
            "workspace_created/workspace_selected with anonymous=false, or process events referencing a workspace".to_owned(),
            "named workspace evidence was present".to_owned(),
            AlignProofStatus::Satisfied,
            "the ledger proves durable work was attached to a named workspace".to_owned(),
        ),
        _ => proof_row(
            obligation,
            "named workspace evidence".to_owned(),
            "no named workspace event was present".to_owned(),
            AlignProofStatus::Unproven,
            "the ledger does not prove durable work used a named workspace".to_owned(),
        ),
    }
}

fn evaluate_workspace_trace_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    if ledger.has_workspace_trace_collected() {
        proof_row(
            obligation,
            "workspace_trace_collected or trace_collected event".to_owned(),
            "workspace trace collection event was present".to_owned(),
            AlignProofStatus::Satisfied,
            "the execution ledger proves dependency/workspace trace collection happened".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "workspace trace collection event".to_owned(),
            "no workspace trace collection event was present".to_owned(),
            AlignProofStatus::Unproven,
            "the ledger does not prove workspace trace collection".to_owned(),
        )
    }
}

fn evaluate_stats_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    if ledger.has_stats_collected() {
        proof_row(
            obligation,
            "stats_collected event".to_owned(),
            "stats were collected".to_owned(),
            AlignProofStatus::Satisfied,
            "the execution ledger proves token/workspace stats were collected".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "stats_collected event".to_owned(),
            "no stats collection event was present".to_owned(),
            AlignProofStatus::Unproven,
            "the ledger does not prove token/workspace stats were collected".to_owned(),
        )
    }
}

fn evaluate_final_response_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    if ledger.final_response_included_evidence() && ledger.final_response_included_alignment() {
        proof_row(
            obligation,
            "final_response_sent with evidence, alignment, and token-savings fields".to_owned(),
            "final response included evidence, alignment, and token-savings sections".to_owned(),
            AlignProofStatus::Satisfied,
            "the execution ledger proves the final response reported the required completion evidence".to_owned(),
        )
    } else if ledger.has_final_response() {
        proof_row(
            obligation,
            "final_response_sent with evidence, alignment, and token-savings fields".to_owned(),
            "final response event exists but is missing required report fields".to_owned(),
            AlignProofStatus::PartiallySatisfied,
            "the final response was captured, but the ledger does not prove every required section was included".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "final_response_sent event".to_owned(),
            "no final response event was present".to_owned(),
            AlignProofStatus::Unproven,
            "the ledger does not prove what was reported to the user".to_owned(),
        )
    }
}

fn evaluate_forbidden_event_absence(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
    event_names: &[&str],
    absence_message: &str,
) -> AlignProofRow {
    if ledger.has_negative_event(event_names) {
        proof_row(
            obligation,
            "absence of forbidden execution events".to_owned(),
            format!("forbidden event {:?} was present", event_names),
            AlignProofStatus::Violated,
            "structured execution evidence contradicts a forbid".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            "absence of forbidden execution events".to_owned(),
            absence_message.to_owned(),
            AlignProofStatus::Satisfied,
            "the execution ledger has no event contradicting this forbid".to_owned(),
        )
    }
}

fn evaluate_generic_obligation(
    obligation: &AlignObligation,
    ledger: &ExecutionLedger,
) -> AlignProofRow {
    if ledger.has_event_for_id(
        &[
            "obligation_satisfied",
            "route_fulfilled",
            "route_check_completed",
            "after_success_completed",
            "elicitation_answered",
            "elicitation_waived",
        ],
        &obligation.id,
    ) {
        proof_row(
            obligation,
            format!("execution event proving `{}`", obligation.id),
            format!("structured event references `{}`", obligation.id),
            AlignProofStatus::Satisfied,
            "the execution ledger explicitly marks this obligation as satisfied".to_owned(),
        )
    } else {
        proof_row(
            obligation,
            expected_evidence_for(obligation),
            "no matching structured execution event was present".to_owned(),
            AlignProofStatus::Unproven,
            "the aligner has no matcher or explicit proof event for this obligation".to_owned(),
        )
    }
}

fn proof_row(
    obligation: &AlignObligation,
    expected_evidence: String,
    observed_evidence: String,
    status: AlignProofStatus,
    explanation: String,
) -> AlignProofRow {
    AlignProofRow {
        requirement: requirement_for(obligation),
        obligation: format!(
            "{} ({})",
            obligation.id,
            obligation_kind_label(obligation.kind)
        ),
        expected_evidence,
        observed_evidence,
        status,
        explanation,
    }
}

fn requirement_for(obligation: &AlignObligation) -> String {
    match obligation.id.as_str() {
        "user_tracked_background_process" => {
            "User requested work as a tracked background process".to_owned()
        }
        "cli_invocations_use_rote_exec"
        | "run_cli_only_through_rote_exec"
        | "direct_cli_without_rote_exec"
        | "direct_shell_command_without_rote_exec"
        | "direct_harness_cli_call_without_rote_exec" => {
            "CLI work must be captured through rote exec".to_owned()
        }
        "untracked_stdout_scrollback" => {
            "Command output must be captured as structured evidence".to_owned()
        }
        "external_service_tasks_are_adapter_first"
        | "skipping_adapter_discovery"
        | "discover_relevant_rote_adapters"
        | "identify_required_services_and_tools" => {
            "External-service work must discover adapters before CLI fallback".to_owned()
        }
        "skipping_cli_readiness_check"
        | "verify_adapter_or_cli_readiness"
        | "preflight_cli_fallback" => "CLI fallback must prove readiness before use".to_owned(),
        "durable_work_requires_named_workspace"
        | "rote_exec_outside_workspace"
        | "anonymous_workspace" => "Durable work must use a named workspace".to_owned(),
        "compute_workspace_trace" => "Workspace dependency trace must be collected".to_owned(),
        "compute_workspace_stats" => "Workspace/token stats must be collected".to_owned(),
        "report_workspace_evidence_and_token_math"
        | "final_summary_without_trace_math"
        | "final_summary_without_workspace"
        | "summarize_evidence" => {
            "Final response must report result, evidence, alignment, and token math".to_owned()
        }
        _ => format!("Spec obligation `{}` must be satisfied", obligation.id),
    }
}

fn expected_evidence_for(obligation: &AlignObligation) -> String {
    match obligation.kind {
        AlignObligationKind::Route => {
            "structured execution evidence proving the selected route was fulfilled".to_owned()
        }
        AlignObligationKind::RouteCheck => {
            "route_check_completed or equivalent structured evidence".to_owned()
        }
        AlignObligationKind::Forbid => {
            "structured evidence proving the forbidden substitution did not occur".to_owned()
        }
        AlignObligationKind::Elicitation => {
            "elicitation_answered or elicitation_waived event".to_owned()
        }
        AlignObligationKind::AfterSuccess => {
            "after_success_completed or closure-specific structured evidence".to_owned()
        }
        AlignObligationKind::UserRequirement => {
            "structured execution event proving the user requirement".to_owned()
        }
    }
}

fn obligation_kind_label(kind: AlignObligationKind) -> &'static str {
    match kind {
        AlignObligationKind::Route => "route",
        AlignObligationKind::RouteCheck => "route_check",
        AlignObligationKind::Forbid => "forbid",
        AlignObligationKind::Elicitation => "elicitation",
        AlignObligationKind::AfterSuccess => "after_success",
        AlignObligationKind::UserRequirement => "user_requirement",
    }
}

fn obligations_for(spec: &SkillSpec, decision: &decision::Decision) -> Vec<AlignObligation> {
    let mut obligations = Vec::new();
    if let Some(route) = &decision.route {
        obligations.push(AlignObligation {
            id: route.0.clone(),
            kind: AlignObligationKind::Route,
            status: AlignCheckStatus::Unproven,
            source: "decision.route".to_owned(),
            message: "selected route needs structured execution evidence to prove fulfillment"
                .to_owned(),
        });
        if let Some(route_spec) = spec.routes.iter().find(|candidate| candidate.id == *route) {
            for check in &route_spec.checks {
                obligations.push(AlignObligation {
                    id: check.clone(),
                    kind: AlignObligationKind::RouteCheck,
                    status: AlignCheckStatus::Unproven,
                    source: format!("routes.{}.checks", route.0),
                    message: "route check needs structured execution evidence to prove fulfillment"
                        .to_owned(),
                });
            }
        }
    }
    for forbid in &decision.forbid {
        obligations.push(AlignObligation {
            id: forbid.clone(),
            kind: AlignObligationKind::Forbid,
            status: AlignCheckStatus::Unproven,
            source: "decision.forbid".to_owned(),
            message: "forbid compliance needs structured execution evidence to prove no violation"
                .to_owned(),
        });
    }
    for elicitation in &decision.elicit {
        obligations.push(AlignObligation {
            id: elicitation.clone(),
            kind: AlignObligationKind::Elicitation,
            status: AlignCheckStatus::Unproven,
            source: "decision.elicit".to_owned(),
            message: "elicitation fulfillment needs structured execution evidence or a waiver"
                .to_owned(),
        });
    }
    for closure in &decision.after_success {
        obligations.push(AlignObligation {
            id: closure.clone(),
            kind: AlignObligationKind::AfterSuccess,
            status: AlignCheckStatus::Unproven,
            source: "decision.after_success".to_owned(),
            message:
                "after-success closure needs structured execution evidence to prove fulfillment"
                    .to_owned(),
        });
    }
    obligations
}

fn report_status(checks: &[AlignCheck], obligations: &[AlignObligation]) -> AlignStatus {
    if checks
        .iter()
        .any(|check| check.status == AlignCheckStatus::Fail)
        || obligations
            .iter()
            .any(|obligation| obligation.status == AlignCheckStatus::Fail)
    {
        return AlignStatus::Fail;
    }
    if checks
        .iter()
        .any(|check| check.status == AlignCheckStatus::Unproven)
        || obligations
            .iter()
            .any(|obligation| obligation.status == AlignCheckStatus::Unproven)
    {
        return AlignStatus::Unproven;
    }
    AlignStatus::Pass
}
