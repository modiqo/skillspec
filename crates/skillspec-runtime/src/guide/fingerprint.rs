use crate::act::ActReport;
use crate::decision::Decision;
use serde::Serialize;
use sha2::{Digest, Sha256};
use skillspec_core::error::Result;

#[derive(Serialize)]
struct DecisionFingerprint<'a> {
    schema: &'static str,
    input_sha256: &'a str,
    selected_route: Option<&'a str>,
    route_selection_basis: Option<&'a str>,
    route_selection_rule: Option<&'a str>,
    matched_rules: Vec<&'a str>,
    route_order: Vec<&'a str>,
    phase_order: Vec<&'a str>,
    phase_requirements: Vec<PhaseRequirements<'a>>,
    phase_forbids: Vec<PhaseForbids<'a>>,
    route_forbids: &'a [String],
    elicitations: &'a [String],
    after_success: &'a [String],
}

#[derive(Serialize)]
struct PhaseRequirements<'a> {
    phase: &'a str,
    requires: &'a [String],
}

#[derive(Serialize)]
struct PhaseForbids<'a> {
    phase: &'a str,
    forbid: &'a [String],
}

pub fn decision_fingerprint(
    decision: &Decision,
    act: &ActReport,
    input_sha256: &str,
) -> Result<String> {
    let payload = DecisionFingerprint {
        schema: "skillspec.decision/v0",
        input_sha256,
        selected_route: decision.route.as_ref().map(|route| route.0.as_str()),
        route_selection_basis: act
            .route_selection
            .as_ref()
            .map(|selection| selection.basis.as_str()),
        route_selection_rule: act
            .route_selection
            .as_ref()
            .and_then(|selection| selection.rule_id.as_deref()),
        matched_rules: act
            .matched_rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect(),
        route_order: decision
            .route_order
            .iter()
            .map(|route| route.0.as_str())
            .collect(),
        phase_order: act.phases.iter().map(|phase| phase.id.as_str()).collect(),
        phase_requirements: act
            .phases
            .iter()
            .map(|phase| PhaseRequirements {
                phase: phase.id.as_str(),
                requires: phase.requires.as_slice(),
            })
            .collect(),
        phase_forbids: act
            .phases
            .iter()
            .map(|phase| PhaseForbids {
                phase: phase.id.as_str(),
                forbid: phase.forbid.as_slice(),
            })
            .collect(),
        route_forbids: act.forbidden.as_slice(),
        elicitations: act.elicitations.as_slice(),
        after_success: act.after_success.as_slice(),
    };
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&payload)?);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}
