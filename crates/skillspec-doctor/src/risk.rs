use super::types::{
    AgentDriftRiskReport, FrontmatterDiscoveryRiskReport, RiskBasisReport, RiskCondition,
    RiskConditionKind, RiskConfidence, RiskEvidence, RiskLevel,
};
use super::{DoctorBasis, DoctorIssue};
use std::collections::BTreeMap;

pub(super) fn agent_report(
    structural_score: u8,
    issues: &[DoctorIssue],
    frontmatter: Option<FrontmatterDiscoveryRiskReport>,
    basis: &[DoctorBasis],
) -> AgentDriftRiskReport {
    let score = 100u8.saturating_sub(structural_score);
    let mut conditions = issues.iter().map(condition_from_issue).collect::<Vec<_>>();
    if let Some(frontmatter) = &frontmatter {
        conditions.extend(frontmatter.conditions.clone());
    }
    conditions.sort_by_key(|condition| condition.level);
    AgentDriftRiskReport {
        schema: "skillspec.doctor.agent_drift_risk.v0".to_owned(),
        score,
        level: RiskLevel::from_score(score),
        threshold_source: "skillspec_policy_v0".to_owned(),
        summary: summary(score),
        recommended_mode: recommended_mode(score).to_owned(),
        frontmatter_discovery_risk: frontmatter,
        conditions,
        basis_registry: basis.iter().map(basis_report).collect(),
    }
}

fn condition_from_issue(issue: &DoctorIssue) -> RiskCondition {
    RiskCondition {
        id: issue.id.clone(),
        kind: kind_for_issue(&issue.id),
        level: RiskLevel::from_severity(&issue.severity),
        score_delta: issue.score_penalty,
        confidence: RiskConfidence::Medium,
        measurement: BTreeMap::new(),
        evidence: vec![RiskEvidence {
            path: issue.location.clone().unwrap_or_default(),
            line: None,
            text_preview: issue.evidence.clone(),
        }],
        basis_ids: issue.basis.clone(),
        claim_scope: "static_risk_not_observed_failure".to_owned(),
        threshold_source: "skillspec_policy_v0".to_owned(),
        consequence: issue.title.clone(),
        recommended_action: issue.remediation.clone(),
    }
}

fn kind_for_issue(id: &str) -> RiskConditionKind {
    match id {
        "large_activation_body" | "large_activation_surface" | "medium_activation_surface" => {
            RiskConditionKind::ContextPressure
        }
        "ambiguous_short_description"
        | "description_listing_budget_risk"
        | "manual_only_visibility"
        | "missing_or_malformed_frontmatter"
        | "overbroad_description" => RiskConditionKind::DiscoveryRisk,
        "primacy_bias_late_obligations" => RiskConditionKind::PositionRisk,
        "instruction_density" => RiskConditionKind::InstructionFollowingRisk,
        "missing_behavior_contract" | "missing_trace_proof_surface" => RiskConditionKind::ProofGap,
        "workspace_shape_entry_with_subskills"
        | "multi_skill_workspace_shape"
        | "plugin_workspace_shape" => RiskConditionKind::ShapeRisk,
        "workspace_cross_skill_reference_risk" | "workspace_name_collision_risk" => {
            RiskConditionKind::WorkspaceAggregateRisk
        }
        "missing_referenced_files" => RiskConditionKind::SourceIntegrityRisk,
        _ => RiskConditionKind::ExecutionRisk,
    }
}

fn summary(score: u8) -> String {
    match RiskLevel::from_score(score) {
        RiskLevel::Low => "Low static risk that an agent will drift from the skill instructions."
            .to_owned(),
        RiskLevel::Medium => {
            "Medium static risk: review discovery, instruction load, and proof gaps.".to_owned()
        }
        RiskLevel::High => {
            "High static risk that an agent will miss steps, pick the wrong surface, or report without proof.".to_owned()
        }
        RiskLevel::Critical => {
            "Critical static risk: port to a structured SkillSpec contract before relying on this skill.".to_owned()
        }
    }
}

fn recommended_mode(score: u8) -> &'static str {
    match RiskLevel::from_score(score) {
        RiskLevel::Low => "usable_with_review",
        RiskLevel::Medium => "review_before_install",
        RiskLevel::High | RiskLevel::Critical => "port_to_skillspec_before_install",
    }
}

fn basis_report(basis: &DoctorBasis) -> RiskBasisReport {
    RiskBasisReport {
        id: basis.id.clone(),
        kind: basis.kind.clone(),
        citation: basis.citation.clone(),
        url: basis.source.clone(),
        claim_used: basis.claim.clone(),
    }
}
