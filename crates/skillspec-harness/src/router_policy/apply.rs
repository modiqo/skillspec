use rusqlite::Connection;
use skillspec_core::error::{Error, Result};

use crate::router::{RouteBypassReason, RouteCandidate};

use super::model::{
    PolicyAnchor, PolicyEffect, PolicyProfileMode, PolicyRuleMode, PolicyTargetKind,
    RoutePolicyEffectMatch, RoutePolicyReport, RoutePolicyRuleMatch,
};
use super::store::{read_active_policy, read_named_policy, ActivePolicy, ActiveRule};
use super::target::{normalize_phrase, query_contains_phrase, target_matches};

#[derive(Clone, Debug)]
pub(crate) struct PolicyApplication {
    pub(crate) report: Option<RoutePolicyReport>,
    pub(crate) forced_bypass: Option<PolicyBypass>,
}

#[derive(Clone, Debug)]
pub(crate) struct PolicyBypass {
    pub(crate) reason: RouteBypassReason,
    pub(crate) decision_reason: String,
}

pub(crate) fn apply_to_candidates(
    conn: &Connection,
    profile_override: Option<&str>,
    query: &str,
    candidates: &mut Vec<RouteCandidate>,
) -> Result<PolicyApplication> {
    super::create_schema(conn)?;
    let Some(policy) = load_policy(conn, profile_override)? else {
        return Ok(PolicyApplication {
            report: None,
            forced_bypass: None,
        });
    };

    let normalized_query = normalize_phrase(query);
    let mut matched_rules = Vec::new();
    for rule in &policy.rules {
        if !rule_matches(rule, &normalized_query) {
            continue;
        }
        let mut effect_matches = Vec::new();
        for preference in &rule.preferences {
            let mut matched = Vec::new();
            for candidate in candidates.iter_mut() {
                if !target_matches(candidate, &preference.target) {
                    continue;
                }
                matched.push(candidate.name.clone());
                apply_effect(
                    candidate,
                    preference.effect,
                    preference.weight,
                    rule,
                    preference.target.kind,
                );
            }
            effect_matches.push(RoutePolicyEffectMatch {
                effect: preference.effect,
                target_kind: preference.target.kind,
                target_value: preference.target.value.clone(),
                matched_candidates: matched,
            });
        }
        matched_rules.push(RoutePolicyRuleMatch {
            id: rule.id.clone(),
            priority: rule.priority,
            mode: rule.mode,
            anchor: rule.anchor,
            effects: effect_matches,
        });
    }

    candidates.retain(|candidate| !candidate.policy_forbidden);
    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
    });

    let forced_bypass = passthrough_bypass(
        &policy,
        matched_rules.iter().any(rule_match_selects_candidate),
    );
    let notes = policy_notes(&policy);
    let report = RoutePolicyReport {
        profile: policy.profile.name,
        mode: policy.profile.mode,
        strict: policy.profile.strict,
        matched_rules,
        forced_bypass: forced_bypass.is_some(),
        notes,
    };

    Ok(PolicyApplication {
        report: Some(report),
        forced_bypass,
    })
}

fn load_policy(conn: &Connection, profile_override: Option<&str>) -> Result<Option<ActivePolicy>> {
    if let Some(profile) = profile_override {
        return read_named_policy(conn, profile)?
            .map(Some)
            .ok_or_else(|| Error::InvalidInput {
                message: format!("router policy profile {profile:?} does not exist"),
            });
    }
    read_active_policy(conn)
}

fn rule_matches(rule: &ActiveRule, normalized_query: &str) -> bool {
    if !rule.predicates.any_keywords.is_empty()
        && !rule
            .predicates
            .any_keywords
            .iter()
            .any(|phrase| query_contains_phrase(normalized_query, phrase))
    {
        return false;
    }
    if !rule
        .predicates
        .all_keywords
        .iter()
        .all(|phrase| query_contains_phrase(normalized_query, phrase))
    {
        return false;
    }
    if rule
        .predicates
        .none_keywords
        .iter()
        .any(|phrase| query_contains_phrase(normalized_query, phrase))
    {
        return false;
    }
    true
}

fn apply_effect(
    candidate: &mut RouteCandidate,
    effect: PolicyEffect,
    weight: f64,
    rule: &ActiveRule,
    target_kind: PolicyTargetKind,
) {
    match effect {
        PolicyEffect::Prefer | PolicyEffect::Allow => {
            let multiplier = if rule.mode == PolicyRuleMode::Hard {
                100.0
            } else {
                1.0
            };
            let delta = weight * multiplier;
            candidate.policy_score += delta;
            candidate.score += delta;
            if rule.anchor == PolicyAnchor::Policy && target_kind == PolicyTargetKind::Skill {
                candidate.policy_anchor = true;
            }
        }
        PolicyEffect::Suppress => {
            candidate.policy_score -= weight;
            candidate.score -= weight;
        }
        PolicyEffect::Forbid => {
            candidate.policy_score -= weight;
            candidate.score -= weight;
            candidate.policy_forbidden = true;
        }
    }
    candidate.policy_reason = Some(match &candidate.policy_reason {
        Some(existing) => format!("{existing}; {} {}", rule.id, effect.as_str()),
        None => format!("{} {}", rule.id, effect.as_str()),
    });
}

fn passthrough_bypass(policy: &ActivePolicy, has_matching_selection: bool) -> Option<PolicyBypass> {
    if !matches!(
        policy.profile.mode,
        PolicyProfileMode::SoftPassthrough | PolicyProfileMode::NativePassthrough
    ) || has_matching_selection
    {
        return None;
    }
    Some(PolicyBypass {
        reason: RouteBypassReason::PolicyPassthrough,
        decision_reason: format!(
            "router policy profile {} is {} and no matching policy rule selected a skill",
            policy.profile.name,
            policy.profile.mode.as_str()
        ),
    })
}

fn rule_match_selects_candidate(rule_match: &RoutePolicyRuleMatch) -> bool {
    rule_match.effects.iter().any(|effect| {
        matches!(effect.effect, PolicyEffect::Prefer | PolicyEffect::Allow)
            && !effect.matched_candidates.is_empty()
    })
}

fn policy_notes(policy: &ActivePolicy) -> Vec<String> {
    let mut notes = Vec::new();
    if policy.profile.mode == PolicyProfileMode::NativePassthrough {
        notes.push(
            "native-passthrough profile is active; route-time behavior is soft passthrough until lifecycle visibility mutation is applied".to_owned(),
        );
    }
    notes
}
