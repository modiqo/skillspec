use crate::decision;
use crate::error::{Error, Result};
use crate::model::{RouteId, SkillSpec, TraceEventKind};
use crate::trace::{self, TraceEnvelope};
use serde::Serialize;
use std::collections::BTreeSet;
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
    /// SkillSpec path used for the comparison.
    pub spec: String,
    /// Decision trace run directory used for the comparison.
    pub decision_trace: String,
    /// Deterministic checks derived from the decision trace and current spec.
    pub checks: Vec<AlignCheck>,
    /// Execution-side duties that require structured evidence to prove.
    pub obligations: Vec<AlignObligation>,
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

    Ok(AlignReport {
        schema: ALIGN_SCHEMA.to_owned(),
        ok: status != AlignStatus::Fail,
        status,
        spec: spec_path.display().to_string(),
        decision_trace: decision_trace.display().to_string(),
        checks,
        obligations,
    })
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
