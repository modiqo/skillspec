use super::scoring::{RiskScore, StructuralScore};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Severity of a [`super::DoctorIssue`], ordered most severe first so issue
/// lists sort naturally by declaration order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

impl From<Severity> for RiskLevel {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Critical => Self::Critical,
            Severity::High => Self::High,
            Severity::Medium => Self::Medium,
            Severity::Low => Self::Low,
        }
    }
}

impl From<RiskLevel> for Severity {
    fn from(level: RiskLevel) -> Self {
        match level {
            RiskLevel::Critical => Self::Critical,
            RiskLevel::High => Self::High,
            RiskLevel::Medium => Self::Medium,
            RiskLevel::Low => Self::Low,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskConfidence {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskConditionKind {
    ContextPressure,
    DiscoveryRisk,
    PositionRisk,
    InstructionFollowingRisk,
    SkillDesignRisk,
    ExecutionRisk,
    ProofGap,
    ShapeRisk,
    WorkspaceAggregateRisk,
    SourceIntegrityRisk,
}

#[derive(Clone, Debug, Serialize)]
pub struct RiskEvidence {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub text_preview: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RiskCondition {
    pub id: String,
    pub kind: RiskConditionKind,
    pub level: RiskLevel,
    pub score_delta: u8,
    pub confidence: RiskConfidence,
    pub measurement: BTreeMap<String, serde_json::Value>,
    pub evidence: Vec<RiskEvidence>,
    pub basis_ids: Vec<String>,
    pub claim_scope: String,
    pub threshold_source: String,
    pub consequence: String,
    pub recommended_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RiskBasisReport {
    pub id: String,
    pub kind: String,
    pub citation: String,
    pub url: String,
    pub claim_used: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FrontmatterDiscoveryRiskReport {
    pub score: RiskScore,
    pub level: RiskLevel,
    pub fields: FrontmatterDiscoveryFields,
    pub conditions: Vec<RiskCondition>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FrontmatterDiscoveryFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_model_invocation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_invocable: Option<bool>,
    pub parse_status: FrontmatterParseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
    pub description_chars: usize,
    pub description_tokens: usize,
    pub combined_discovery_chars: usize,
    pub combined_discovery_tokens: usize,
    pub harness_cap_chars: usize,
    pub harness_profile: String,
    pub domain_term_count: usize,
    pub action_term_count: usize,
    pub trigger_phrase_count: usize,
    pub generic_term_ratio: f32,
    pub body_heading_overlap: usize,
    pub manual_only: bool,
    pub visibility_state: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontmatterParseStatus {
    Missing,
    Unterminated,
    InvalidYaml,
    Parsed,
}

#[derive(Clone, Debug, Serialize)]
pub struct AgentDriftRiskReport {
    pub schema: String,
    pub score: RiskScore,
    pub level: RiskLevel,
    pub threshold_source: String,
    pub summary: String,
    pub recommended_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter_discovery_risk: Option<FrontmatterDiscoveryRiskReport>,
    pub conditions: Vec<RiskCondition>,
    pub basis_registry: Vec<RiskBasisReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RawActivationRiskReport {
    pub score: RiskScore,
    pub level: RiskLevel,
    pub activation_estimated_tokens: usize,
    pub activation_lines: usize,
    pub modal_obligations: usize,
    pub late_modal_obligations: usize,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContractMitigationReport {
    pub present: bool,
    pub spec_path: String,
    pub routes: usize,
    pub rules: usize,
    pub commands: usize,
    pub dependencies: usize,
    pub tests: usize,
    pub level: ContractMitigationLevel,
    pub residual_risk_score: RiskScore,
    pub residual_risk_level: RiskLevel,
    pub summary: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractMitigationLevel {
    Weak,
    Partial,
    Strong,
}

impl ContractMitigationLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Weak => "weak",
            Self::Partial => "partial",
            Self::Strong => "strong",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceAgentDriftRiskReport {
    pub score: RiskScore,
    pub level: RiskLevel,
    pub summary: String,
    pub conditions: Vec<RiskCondition>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorPackageRiskReport {
    pub package_id: String,
    pub public_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    pub install_slug: String,
    pub path: String,
    pub source_content_sha256: String,
    pub risk_profile_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_risk_profile_path: Option<String>,
    pub shape_role: String,
    pub entrypoint: String,
    pub structural_score: StructuralScore,
    pub activation_estimated_tokens: usize,
    pub activation_lines: usize,
    pub frontmatter_discovery_risk: FrontmatterDiscoveryRiskReport,
    pub agent_drift_risk: AgentDriftRiskReport,
}
