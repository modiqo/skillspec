use serde::{Deserialize, Serialize};

pub const GUIDE_SCHEMA: &str = "skillspec.guide-state/v0";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideMode {
    Agent,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideStartMode {
    Start,
    Resume,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuideReport {
    pub schema: String,
    pub mode: GuideStartMode,
    pub guide: GuideMode,
    pub start: StartAnchor,
    pub path: GuidePath,
    pub current_gate: CurrentGate,
    pub end: EndAnchor,
    pub resume: ResumeAnchor,
    pub warnings: Vec<GuideWarning>,
    pub state_paths: GuideStatePaths,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartAnchor {
    pub spec: String,
    pub spec_id: String,
    pub run_dir: String,
    pub input_sha256: String,
    pub spec_fingerprint: String,
    pub decision_fingerprint: String,
    pub selected_route: Option<String>,
    pub route_selection: Option<RouteSelectionAnchor>,
    pub matched_rules: Vec<String>,
    pub route_candidates_seen: usize,
    pub first_phase: Option<String>,
    pub current_phase: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteSelectionAnchor {
    pub basis: String,
    pub rule_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuidePath {
    pub phase_order: Vec<String>,
    pub completed_phases: Vec<String>,
    pub blocked_phases: Vec<String>,
    pub remaining_phases: Vec<String>,
    pub required_transitions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrentGate {
    pub phase: Option<String>,
    pub owner_skill: Option<String>,
    pub route_scope: Option<String>,
    pub description: Option<String>,
    pub open_requirements: Vec<String>,
    pub checks: Vec<String>,
    pub do_now: Vec<String>,
    pub do_not: Vec<String>,
    pub allowed_now: Vec<String>,
    pub allowed_commands: Vec<String>,
    pub recommended_queries: Vec<String>,
    pub progress_to_record: Vec<ProgressRecordHint>,
    pub when_to_advance: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProgressRecordHint {
    pub event: String,
    pub phase: Option<String>,
    pub requirement: Option<String>,
    pub command: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EndAnchor {
    pub done_when: Vec<String>,
    pub route_fulfillment_event: String,
    pub token_stats_command: String,
    pub final_progress_command: String,
    pub alignment_command: String,
    pub final_response_must_include: Vec<String>,
    pub proof_paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResumeAnchor {
    pub command: String,
    pub guide_state: String,
    pub guide_summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuideStatePaths {
    pub guide_state: String,
    pub guide_summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuideWarning {
    pub kind: GuideWarningKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideWarningKind {
    SpecChangedDecisionStable,
    SpecChangedNoPriorGuide,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PriorGuideState {
    pub start: PriorStartAnchor,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PriorStartAnchor {
    pub spec_fingerprint: String,
    pub input_sha256: String,
    pub decision_fingerprint: String,
}
