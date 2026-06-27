use serde::Serialize;

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
    /// Execution trace files used for proof, if supplied.
    pub execution_traces: Vec<String>,
    /// Deterministic checks derived from the decision trace and current spec.
    pub checks: Vec<AlignCheck>,
    /// Execution-side duties that require structured evidence to prove.
    pub obligations: Vec<AlignObligation>,
    /// User-facing alignment proof rows that connect requirements to evidence.
    pub proof_rows: Vec<AlignProofRow>,
}

/// Condensed alignment explanation for humans and JSON consumers.
#[derive(Clone, Debug, Serialize)]
pub struct AlignSummary {
    /// Evidence scope used for this alignment run.
    pub scope: AlignScope,
    /// Result of replaying the decision trace against the current spec.
    pub decision_alignment: AlignLayerStatus,
    /// Result of checking action evidence, or `not_evaluated` when no execution trace was supplied.
    pub execution_alignment: AlignLayerStatus,
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
    /// Phase requirements that still need explicit structured proof.
    pub phase_requirement_gaps: Vec<AlignPhaseRequirementGap>,
    /// Compact completion-facing summary suitable for final agent responses.
    pub completion: AlignCompletionSummary,
    /// Token consumption and savings evidence supplied by the execution ledger.
    pub tokens: AlignTokenSummary,
}

/// Compact alignment summary intended for the end of an inference cycle.
#[derive(Clone, Debug, Serialize)]
pub struct AlignCompletionSummary {
    pub decision_replay: String,
    pub phase_order: String,
    pub requirements: String,
    pub missing_proof: Vec<String>,
    pub forbidden_actions: String,
    pub alignment: String,
}

/// Token usage and savings evidence captured from structured execution events.
#[derive(Clone, Debug, Serialize)]
pub struct AlignTokenSummary {
    pub consumption: String,
    pub savings: String,
    pub evidence: Vec<String>,
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

/// Evidence scope available to this alignment run.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignScope {
    /// Only the SkillSpec decision/reasoning trace was supplied.
    DecisionTraceOnly,
    /// A decision trace and at least one execution evidence file were supplied.
    DecisionAndExecutionTrace,
}

/// Per-layer alignment result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignLayerStatus {
    /// All checks in this layer passed.
    Pass,
    /// At least one check in this layer failed.
    Fail,
    /// No check failed, but required evidence in this layer is incomplete.
    Incomplete,
    /// This layer was not evaluated because the required evidence source was not supplied.
    NotEvaluated,
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

/// One missing phase requirement proof item.
#[derive(Clone, Debug, Serialize)]
pub struct AlignPhaseRequirementGap {
    pub phase: String,
    pub requirement: String,
    pub status: AlignPhaseRequirementGapStatus,
    pub needed: String,
}

/// Why a phase requirement remains in the proof digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignPhaseRequirementGapStatus {
    /// No execution ledger was supplied, so the requirement could not be checked.
    NotEvaluated,
    /// An execution ledger was supplied, but no matching requirement proof row exists.
    Missing,
    /// A matching requirement_failed row exists and must be resolved before pass.
    Failed,
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

/// User-facing status for an obligation proof row.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignProofStatus {
    /// Structured evidence proves the obligation.
    Satisfied,
    /// Evidence proves part of the obligation, but required proof is incomplete.
    PartiallySatisfied,
    /// Structured evidence contradicts the obligation.
    Violated,
    /// Required evidence is missing.
    Unproven,
}

/// One user-facing alignment proof row.
#[derive(Clone, Debug, Serialize)]
pub struct AlignProofRow {
    pub requirement: String,
    pub obligation: String,
    pub expected_evidence: String,
    pub observed_evidence: String,
    pub status: AlignProofStatus,
    pub explanation: String,
}

/// Grouped proof-planning digest emitted beside an alignment report.
#[derive(Clone, Debug, Serialize)]
pub struct AlignProofDigest {
    /// Schema identifier for the digest payload.
    pub schema: String,
    /// Overall alignment status from the source report.
    pub status: AlignStatus,
    /// Completion-facing alignment label, such as pass, partial, or fail.
    pub alignment: String,
    /// Alignment report this digest summarizes.
    pub alignment_report: String,
    /// Suggested batch file path for final proof events.
    pub suggested_batch_file: String,
    /// Number of missing proof items across all digest groups.
    pub missing_count: usize,
    /// How agents should use this digest without creating a visible progress loop.
    pub recommended_loop: Vec<String>,
    /// Missing proof grouped by the event shape needed to close it.
    pub groups: Vec<AlignProofDigestGroup>,
}

/// A batchable group of missing proof items.
#[derive(Clone, Debug, Serialize)]
pub struct AlignProofDigestGroup {
    pub kind: String,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_event: Option<String>,
    pub required_fields: Vec<String>,
    pub items: Vec<AlignProofDigestItem>,
}

/// One item in a proof digest group.
#[derive(Clone, Debug, Serialize)]
pub struct AlignProofDigestItem {
    pub id: String,
    pub source: String,
    pub needed: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obligation_kind: Option<AlignObligationKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_event: Option<String>,
    pub required_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_evidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_evidence: Option<String>,
    pub note: String,
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
    /// A direct user requirement that should be proven by execution evidence.
    UserRequirement,
}

impl AlignReport {
    /// Returns true when the report should cause a non-zero CLI exit.
    pub fn has_failures(&self) -> bool {
        self.status == AlignStatus::Fail
    }
}
