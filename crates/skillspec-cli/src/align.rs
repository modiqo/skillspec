use crate::decision;
use crate::error::{Error, Result};
use crate::model::{RouteId, SkillSpec, TraceEventKind};
use crate::trace::{self, TraceEnvelope};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const ALIGN_SCHEMA: &str = "skillspec.align/v0";

/// Machine-readable result from comparing a SkillSpec with a decision trace.
///
/// Alignment is intentionally split into deterministic decision checks and
/// execution obligations. A report can be `ok` while still `unproven` when the
/// decision trace is reproducible but no structured execution evidence was
/// supplied.
#[derive(Clone, Debug, Serialize)]
pub struct AlignReport {
    /// Schema identifier for the report payload.
    pub schema: String,
    /// True when no deterministic check failed.
    pub ok: bool,
    /// Overall report classification.
    pub status: AlignStatus,
    /// Condensed explanation of why the report has this status.
    pub summary: AlignSummary,
    /// SkillSpec path used for the comparison.
    pub spec: String,
    /// Decision trace run directory used for the comparison.
    pub decision_trace: String,
    /// Deterministic checks derived from the decision trace and current spec.
    pub checks: Vec<AlignCheck>,
    /// Execution-side duties that require structured evidence to prove.
    pub obligations: Vec<AlignObligation>,
}

/// Condensed alignment explanation for humans and JSON consumers.
#[derive(Clone, Debug, Serialize)]
pub struct AlignSummary {
    /// One-sentence interpretation of the report status.
    pub conclusion: String,
    /// What the overall status means for a human reader.
    pub status_meaning: String,
    /// The two-layer measurement model used by alignment.
    pub layers: Vec<AlignLayerSummary>,
    /// Selected route from the fresh decision, when any route was selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_route: Option<String>,
    /// Route-selection mechanism, such as rule_prefer or default_route_order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_selection_basis: Option<String>,
    /// Rule responsible for route selection, when route selection came from a rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_selection_rule: Option<String>,
    /// Matched rule ids from the fresh decision.
    pub matched_rules: Vec<String>,
    /// Deterministic decision-trace check totals.
    pub decision_checks: AlignStatusCounts,
    /// Execution obligation proof totals.
    pub execution_obligations: AlignStatusCounts,
    /// Unproven obligations grouped by kind for fast triage.
    pub unproven_obligation_kinds: Vec<AlignObligationKindCount>,
    /// Missing proof items that explain an `unproven` status.
    pub evidence_gaps: Vec<AlignEvidenceGap>,
}

/// One layer of the alignment measurement model.
#[derive(Clone, Debug, Serialize)]
pub struct AlignLayerSummary {
    pub id: AlignLayerKind,
    pub label: String,
    pub measures: String,
    pub interpretation: String,
    pub counts: AlignStatusCounts,
}

/// High-level alignment measurement layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignLayerKind {
    /// Replays the current spec on captured input and compares the decision facts.
    DecisionReplay,
    /// Checks structured evidence for obligations implied by the active decision.
    ExecutionProof,
}

/// One missing proof item that prevents a full `pass`.
#[derive(Clone, Debug, Serialize)]
pub struct AlignEvidenceGap {
    pub id: String,
    pub kind: AlignEvidenceGapKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obligation_kind: Option<AlignObligationKind>,
    pub source: String,
    pub needed: String,
}

/// Source category for a missing proof item.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignEvidenceGapKind {
    /// A deterministic trace fact was absent from the decision trace.
    DecisionTrace,
    /// An execution-side obligation lacked structured proof.
    ExecutionObligation,
}

/// Counts for pass/fail/unproven status groups.
#[derive(Clone, Debug, Default, Serialize)]
pub struct AlignStatusCounts {
    pub total: usize,
    pub pass: usize,
    pub fail: usize,
    pub unproven: usize,
}

/// Count of obligations by source kind.
#[derive(Clone, Debug, Serialize)]
pub struct AlignObligationKindCount {
    pub kind: AlignObligationKind,
    pub total: usize,
    pub unproven: usize,
}

/// Overall alignment state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignStatus {
    /// Every deterministic check passed and every obligation is proven.
    Pass,
    /// At least one deterministic check failed.
    Fail,
    /// No deterministic check failed, but one or more facts are missing proof.
    Unproven,
}

/// One deterministic comparison performed by `skillspec trace align`.
#[derive(Clone, Debug, Serialize)]
pub struct AlignCheck {
    /// Stable check identifier.
    pub id: String,
    /// Result for this check.
    pub status: AlignCheckStatus,
    /// Human-readable explanation.
    pub message: String,
    /// Expected value when the check has a comparable value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<serde_json::Value>,
    /// Actual value recorded in the trace, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<serde_json::Value>,
}

/// Status for an individual check or obligation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignCheckStatus {
    /// The available evidence satisfies the check.
    Pass,
    /// The available evidence contradicts the expected value.
    Fail,
    /// The required evidence was not present.
    Unproven,
}

/// A route, guard, elicitation, or closure that execution evidence must prove.
#[derive(Clone, Debug, Serialize)]
pub struct AlignObligation {
    /// Stable obligation identifier, usually a route, forbid, elicitation, or closure id.
    pub id: String,
    /// Obligation category.
    pub kind: AlignObligationKind,
    /// Current proof status.
    pub status: AlignCheckStatus,
    /// Where the obligation came from in the decision.
    pub source: String,
    /// Human-readable proof requirement.
    pub message: String,
}

/// Source category for an execution obligation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignObligationKind {
    /// The selected route must be fulfilled by execution.
    Route,
    /// A route-local check must be fulfilled by execution.
    RouteCheck,
    /// A forbidden substitution must not have occurred.
    Forbid,
    /// A required elicitation must have been answered or waived.
    Elicitation,
    /// A post-success closure must have been executed.
    AfterSuccess,
}

impl AlignReport {
    /// Returns true when the report should cause a non-zero CLI exit.
    pub fn has_failures(&self) -> bool {
        self.status == AlignStatus::Fail
    }
}

/// Align a loaded spec against a decision trace run directory.
///
/// This first-pass aligner proves decision reproducibility and derives
/// execution obligations. It does not parse human-readable tool transcripts;
/// absent structured execution evidence is reported as `unproven`.
pub fn align_decision_trace(
    spec: &SkillSpec,
    spec_path: &Path,
    decision_trace: &Path,
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

    let obligations = obligations_for(spec, &expected_decision);
    let status = report_status(&checks, &obligations);
    let summary = summary_for(status, &checks, &obligations, &expected_decision);

    Ok(AlignReport {
        schema: ALIGN_SCHEMA.to_owned(),
        ok: status != AlignStatus::Fail,
        status,
        summary,
        spec: spec_path.display().to_string(),
        decision_trace: decision_trace.display().to_string(),
        checks,
        obligations,
    })
}

fn summary_for(
    status: AlignStatus,
    checks: &[AlignCheck],
    obligations: &[AlignObligation],
    decision: &decision::Decision,
) -> AlignSummary {
    let decision_checks = status_counts(checks.iter().map(|check| check.status));
    let execution_obligations =
        status_counts(obligations.iter().map(|obligation| obligation.status));
    let unproven_obligation_kinds = unproven_obligation_kinds(obligations);
    let conclusion = align_conclusion(status, &decision_checks, &execution_obligations);
    let status_meaning = align_status_meaning(status).to_owned();
    let layers = align_layers(&decision_checks, &execution_obligations);
    let evidence_gaps = evidence_gaps(checks, obligations);

    AlignSummary {
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
    }
}

fn align_status_meaning(status: AlignStatus) -> &'static str {
    match status {
        AlignStatus::Pass => {
            "pass means the current spec reproduced the trace decision and every active execution obligation has structured proof"
        }
        AlignStatus::Fail => {
            "fail means the current spec contradicts the recorded decision trace; treat this as spec drift or a trace/spec mismatch"
        }
        AlignStatus::Unproven => {
            "unproven means no contradiction was found, but the trace lacks structured evidence for every fact alignment needs to prove"
        }
    }
}

fn align_layers(
    decision_checks: &AlignStatusCounts,
    execution_obligations: &AlignStatusCounts,
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
            measures: "Derive obligations from the selected route and matched rules, then require structured evidence that the route/checks/closures were fulfilled and forbids were not violated.".to_owned(),
            interpretation: layer_interpretation(
                execution_obligations,
                "execution proof",
                "every active execution obligation has structured proof",
                "the decision is known, but execution evidence is incomplete",
                "structured execution evidence contradicts the active contract",
            ),
            counts: execution_obligations.clone(),
        },
    ]
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

fn align_conclusion(
    status: AlignStatus,
    checks: &AlignStatusCounts,
    obligations: &AlignStatusCounts,
) -> String {
    match status {
        AlignStatus::Pass => {
            "all deterministic decision checks and execution obligations are proven".to_owned()
        }
        AlignStatus::Fail => format!(
            "{} deterministic check(s) failed; the current spec no longer aligns with the decision trace",
            checks.fail
        ),
        AlignStatus::Unproven => align_unproven_conclusion(checks, obligations),
    }
}

fn align_unproven_conclusion(
    checks: &AlignStatusCounts,
    obligations: &AlignStatusCounts,
) -> String {
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
