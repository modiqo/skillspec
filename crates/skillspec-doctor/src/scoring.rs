//! Single source of truth for doctor's follow-through risk rubric.
//!
//! Everything that turns findings into headline numbers lives here: the
//! penalty weights (SkillSpec policy v0), the penalty-to-score arithmetic,
//! the policy bands that map a score to a [`RiskLevel`], and the verdict
//! phrasing derived from those bands.
//!
//! Doctor reports carry two score polarities: follow-through risk (higher
//! is riskier) and structural soundness (higher is healthier). They are
//! duals on the same 0-100 scale, and applying the band table to the wrong
//! polarity silently inverts a report's headline. [`RiskScore`] and
//! [`StructuralScore`] encode the polarity in the type system so that can
//! no longer happen: only [`RiskScore`] can be banded, and a verdict can
//! only be derived through the banded level.

use super::types::{ContractMitigationLevel, RiskLevel};
use serde::Serialize;
use std::fmt;

/// Penalty weights and caps (SkillSpec policy v0).
///
/// Every score penalty doctor can assess is named here so the rubric can be
/// read, reviewed, and tuned in one place. Values are penalty points on the
/// shared 0-100 score scale.
pub(crate) mod penalty {
    /// `invalid_behavior_contract`: skill.spec.yml is present but fails validation.
    pub(crate) const INVALID_BEHAVIOR_CONTRACT: u8 = 18;
    /// `large_activation_body`: activation body exceeds the size guidance.
    pub(crate) const LARGE_ACTIVATION_BODY: u8 = 16;
    /// `large_activation_body`: escalated weight at twice the size guidance.
    pub(crate) const OVERSIZED_ACTIVATION_BODY: u8 = 24;
    /// `large_activation_surface`: >= 75% of package text loads at activation.
    pub(crate) const LARGE_ACTIVATION_SURFACE: u8 = 18;
    /// `medium_activation_surface`: >= 50% of package text loads at activation.
    pub(crate) const MEDIUM_ACTIVATION_SURFACE: u8 = 8;
    /// `instruction_density`: dense modal obligations or numbered steps.
    pub(crate) const INSTRUCTION_DENSITY: u8 = 14;
    /// `primacy_bias_late_obligations`: load-bearing instructions appear late.
    pub(crate) const PRIMACY_BIAS_LATE_OBLIGATIONS: u8 = 10;
    /// `code_mixed_with_activation_instructions`: fenced code inside the body.
    pub(crate) const CODE_MIXED_WITH_ACTIVATION: u8 = 12;
    /// `unlabeled_code_fences`: code fences without a language label.
    pub(crate) const UNLABELED_CODE_FENCES: u8 = 6;
    /// `ambiguous_execution_substrate`: operational prose without a contract.
    pub(crate) const AMBIGUOUS_EXECUTION_SUBSTRATE: u8 = 18;
    /// `implicit_dependency_contract`: dependency signals without deps.toml.
    pub(crate) const IMPLICIT_DEPENDENCY_CONTRACT: u8 = 16;
    /// `missing_referenced_files`: local Markdown references do not resolve.
    pub(crate) const MISSING_REFERENCED_FILES: u8 = 8;
    /// `unmapped_package_surface`: package files unreachable from Markdown.
    pub(crate) const UNMAPPED_PACKAGE_SURFACE: u8 = 8;
    /// `missing_behavior_contract`: no skill.spec.yml at all.
    pub(crate) const MISSING_BEHAVIOR_CONTRACT: u8 = 20;
    /// `missing_trace_proof_surface`: no tests and no trace/progress surface.
    pub(crate) const MISSING_TRACE_PROOF_SURFACE: u8 = 10;
    /// Shape classifications that inform routing but do not score.
    pub(crate) const INFORMATIONAL: u8 = 0;

    /// `activation_token_load`: fallback text-only profile of a large body.
    pub(crate) const FALLBACK_ACTIVATION_TOKEN_LOAD: u8 = 18;

    /// `workspace_repeated_skill_content`: byte-identical SKILL.md copies.
    pub(crate) const WORKSPACE_REPEATED_SKILL_CONTENT: u8 = 8;
    /// `workspace_reused_frontmatter_names`: names repeat across packages.
    pub(crate) const WORKSPACE_REUSED_FRONTMATTER_NAMES: u8 = 6;
    /// `workspace_cross_skill_reference_risk`: undeclared package references.
    pub(crate) const WORKSPACE_CROSS_SKILL_REFERENCE: u8 = 16;
    /// `workspace_name_collision_risk`: install slugs collide.
    pub(crate) const WORKSPACE_NAME_COLLISION: u8 = 14;

    /// `missing_or_malformed_frontmatter`: frontmatter fails to parse.
    pub(crate) const MISSING_FRONTMATTER: u8 = 24;
    /// `missing_or_malformed_frontmatter`: parsed but no usable description.
    pub(crate) const EMPTY_FRONTMATTER_DESCRIPTION: u8 = 18;
    /// `ambiguous_short_description`: too short or too few specific terms.
    pub(crate) const AMBIGUOUS_SHORT_DESCRIPTION: u8 = 14;
    /// `overbroad_description`: generic wording without ownership boundaries.
    pub(crate) const OVERBROAD_DESCRIPTION: u8 = 10;
    /// `description_listing_budget_risk`: discovery text above the harness cap.
    pub(crate) const DISCOVERY_OVER_CAP: u8 = 12;
    /// `description_listing_budget_risk`: discovery text near the harness cap.
    pub(crate) const DISCOVERY_NEAR_CAP: u8 = 6;
    /// `manual_only_visibility`: informational; intentional configuration.
    pub(crate) const MANUAL_ONLY_VISIBILITY: u8 = 0;

    /// Cap on any single frontmatter condition when it joins the structural
    /// rubric as a `DoctorIssue`, so one discovery finding cannot dominate
    /// the package score.
    pub(crate) const FRONTMATTER_CONDITION_ISSUE_CAP: u8 = 20;
    /// Cap on the whole frontmatter discovery score when it is folded into a
    /// text-only fallback package profile, which has no other frontmatter
    /// representation.
    pub(crate) const FRONTMATTER_PROFILE_RISK_CAP: u8 = 30;
}

impl ContractMitigationLevel {
    /// Risk points a valid contract removes from the raw activation risk.
    pub(crate) fn risk_reduction(self) -> u8 {
        match self {
            Self::Strong => 30,
            Self::Partial => 18,
            Self::Weak => 8,
        }
    }
}

/// Follow-through risk on the 0-100 scale; higher means riskier.
///
/// This is the only polarity the policy bands apply to. Values are clamped
/// to 100 at every constructor, so `100 - score` is always well defined.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RiskScore(u8);

impl RiskScore {
    pub(crate) fn new(score: u8) -> Self {
        Self(score.min(100))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    /// The dual structural view of this risk.
    pub(crate) fn structural(self) -> StructuralScore {
        StructuralScore(100 - self.0)
    }

    /// The policy band this score falls into (SkillSpec policy v0).
    ///
    /// This is the single band table in the crate; every level and verdict
    /// doctor reports is derived through it.
    pub(crate) fn level(self) -> RiskLevel {
        match self.0 {
            0..=24 => RiskLevel::Low,
            25..=49 => RiskLevel::Medium,
            50..=74 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    /// Residual risk after a contract mitigation is applied.
    pub(crate) fn mitigated_by(self, level: ContractMitigationLevel) -> Self {
        Self(self.0.saturating_sub(level.risk_reduction()))
    }
}

impl fmt::Display for RiskScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Structural soundness on the 0-100 scale; higher means healthier.
///
/// A structural score cannot be banded directly; interpretation always goes
/// through [`StructuralScore::risk`], which is what makes the verdict and
/// the risk level agree by construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct StructuralScore(u8);

impl StructuralScore {
    pub(crate) fn new(score: u8) -> Self {
        Self(score.min(100))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    /// The dual risk view of this soundness score.
    pub(crate) fn risk(self) -> RiskScore {
        RiskScore(100 - self.0)
    }

    /// The reliability-debt verdict, derived from the policy band of the
    /// dual risk score so the two can never disagree.
    pub(crate) fn verdict(self) -> String {
        format!("{} reliability debt", self.risk().level().as_str())
    }
}

impl fmt::Display for StructuralScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Accumulated penalty points, before clamping to the 0-100 score scale.
///
/// All of doctor's score computations funnel through this type: sum the
/// penalty deltas, optionally fold in a capped extra contribution, then
/// read the total out in the polarity the report needs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Penalties(u16);

impl Penalties {
    pub(crate) fn from_deltas(deltas: impl IntoIterator<Item = u8>) -> Self {
        Self(
            deltas
                .into_iter()
                .fold(0u16, |total, delta| total.saturating_add(u16::from(delta))),
        )
    }

    /// Fold in an extra contribution limited to `cap` points, e.g. the
    /// frontmatter discovery score joining a package profile.
    pub(crate) fn plus_capped(self, points: u8, cap: u8) -> Self {
        Self(self.0.saturating_add(u16::from(points.min(cap))))
    }

    pub(crate) fn risk_score(self) -> RiskScore {
        RiskScore::new(u8::try_from(self.0.min(100)).unwrap_or(100))
    }

    pub(crate) fn structural_score(self) -> StructuralScore {
        self.risk_score().structural()
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::Severity;
    use super::penalty;
    use super::{ContractMitigationLevel, Penalties, RiskLevel, RiskScore, StructuralScore};
    use serde_json::json;

    #[test]
    fn scores_and_severities_keep_their_wire_format() {
        // Reports serialize the newtypes as bare numbers and severities as
        // snake_case strings; downstream JSON consumers depend on this.
        assert_eq!(serde_json::to_value(RiskScore::new(44)).unwrap(), json!(44));
        assert_eq!(
            serde_json::to_value(StructuralScore::new(56)).unwrap(),
            json!(56)
        );
        assert_eq!(serde_json::to_value(Severity::High).unwrap(), json!("high"));
        assert_eq!(
            serde_json::to_value(RiskLevel::Medium).unwrap(),
            json!("medium")
        );
    }

    #[test]
    fn severities_order_most_severe_first() {
        let mut severities = [
            Severity::Low,
            Severity::Critical,
            Severity::Medium,
            Severity::High,
        ];
        severities.sort();
        assert_eq!(
            severities,
            [
                Severity::Critical,
                Severity::High,
                Severity::Medium,
                Severity::Low,
            ]
        );
    }

    #[test]
    fn severity_and_risk_level_conversions_round_trip() {
        for severity in [
            Severity::Critical,
            Severity::High,
            Severity::Medium,
            Severity::Low,
        ] {
            let level = RiskLevel::from(severity);
            assert_eq!(Severity::from(level), severity);
            assert_eq!(level.as_str(), severity.as_str());
        }
    }

    #[test]
    fn risk_bands_match_policy_v0() {
        let expectations = [
            (0, RiskLevel::Low),
            (24, RiskLevel::Low),
            (25, RiskLevel::Medium),
            (49, RiskLevel::Medium),
            (50, RiskLevel::High),
            (74, RiskLevel::High),
            (75, RiskLevel::Critical),
            (100, RiskLevel::Critical),
        ];
        for (score, level) in expectations {
            assert_eq!(RiskScore::new(score).level(), level, "score {score}");
        }
    }

    #[test]
    fn risk_and_structural_scores_are_duals() {
        for value in 0..=100u8 {
            assert_eq!(RiskScore::new(value).structural().risk().get(), value);
            assert_eq!(StructuralScore::new(value).risk().structural().get(), value);
        }
    }

    #[test]
    fn constructors_clamp_to_the_score_scale() {
        assert_eq!(RiskScore::new(255).get(), 100);
        assert_eq!(StructuralScore::new(255).get(), 100);
        assert_eq!(RiskScore::new(255).structural().get(), 0);
    }

    #[test]
    fn verdict_always_agrees_with_the_banded_risk_level() {
        // Regression: the verdict used to band the structural score with
        // thresholds offset from the risk-level bands, so one report could
        // say "medium reliability debt" and level "low" at the same time.
        for value in 0..=100u8 {
            let structural = StructuralScore::new(value);
            let level = structural.risk().level();
            assert_eq!(
                structural.verdict(),
                format!("{} reliability debt", level.as_str()),
                "structural score {value}"
            );
        }
    }

    #[test]
    fn no_penalties_mean_a_clean_score() {
        let penalties = Penalties::from_deltas([]);
        assert_eq!(penalties.risk_score().get(), 0);
        assert_eq!(penalties.structural_score().get(), 100);
    }

    #[test]
    fn penalties_accumulate_and_clamp_at_the_scale_maximum() {
        assert_eq!(Penalties::from_deltas([16, 20, 14]).risk_score().get(), 50);
        assert_eq!(
            Penalties::from_deltas([40, 40, 40])
                .structural_score()
                .get(),
            0
        );
    }

    #[test]
    fn penalties_saturate_instead_of_overflowing() {
        let penalties = Penalties::from_deltas(std::iter::repeat_n(u8::MAX, 4096));
        assert_eq!(penalties.risk_score().get(), 100);
    }

    #[test]
    fn capped_contributions_never_exceed_their_cap() {
        let capped = Penalties::default()
            .plus_capped(90, penalty::FRONTMATTER_PROFILE_RISK_CAP)
            .risk_score();
        assert_eq!(capped.get(), penalty::FRONTMATTER_PROFILE_RISK_CAP);

        let under_cap = Penalties::default().plus_capped(12, 30).risk_score();
        assert_eq!(under_cap.get(), 12);
    }

    #[test]
    fn stronger_contracts_remove_more_risk() {
        let strong = ContractMitigationLevel::Strong.risk_reduction();
        let partial = ContractMitigationLevel::Partial.risk_reduction();
        let weak = ContractMitigationLevel::Weak.risk_reduction();
        assert!(strong > partial && partial > weak);
        assert_eq!(
            RiskScore::new(50).mitigated_by(ContractMitigationLevel::Strong),
            RiskScore::new(20)
        );
        assert_eq!(
            RiskScore::new(5).mitigated_by(ContractMitigationLevel::Strong),
            RiskScore::new(0)
        );
    }
}
