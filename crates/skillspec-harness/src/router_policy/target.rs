use crate::router::RouteCandidate;
use skillspec_core::error::{Error, Result};

use super::model::{PolicyTargetKind, PolicyTargetKind::*};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PolicyTarget {
    pub(crate) kind: PolicyTargetKind,
    pub(crate) value: String,
}

pub(crate) fn parse_targets(values: &[String]) -> Result<Vec<PolicyTarget>> {
    values.iter().map(|value| parse_target(value)).collect()
}

pub(crate) fn parse_target(value: &str) -> Result<PolicyTarget> {
    let Some((kind, raw_value)) = value.split_once(':') else {
        return Err(Error::InvalidInput {
            message: format!(
                "invalid policy target {value:?}; expected skill:<name>, tag:<tag>, source:<source>, or has_skill_spec:<true|false>"
            ),
        });
    };
    let kind = match kind {
        "skill" => Skill,
        "tag" => Tag,
        "source" => Source,
        "has_skill_spec" => HasSkillSpec,
        _ => {
            return Err(Error::InvalidInput {
                message: format!("unknown policy target kind {kind:?}"),
            });
        }
    };
    let value = raw_value.trim();
    if value.is_empty() {
        return Err(Error::InvalidInput {
            message: format!("policy target {value:?} is missing a value"),
        });
    }
    Ok(PolicyTarget {
        kind,
        value: value.to_owned(),
    })
}

pub(crate) fn target_matches(candidate: &RouteCandidate, target: &PolicyTarget) -> bool {
    match target.kind {
        Skill => candidate.name == target.value,
        Tag => candidate.tags.iter().any(|tag| tag == &target.value),
        Source => candidate.source.contains(&target.value),
        HasSkillSpec => {
            let expected = matches!(
                target.value.as_str(),
                "1" | "true" | "yes" | "y" | "skillspec"
            );
            candidate.has_skill_spec == expected
        }
    }
}

pub(crate) fn query_contains_phrase(normalized_query: &str, phrase: &str) -> bool {
    let normalized_phrase = normalize_phrase(phrase);
    !normalized_phrase.is_empty() && normalized_query.contains(&normalized_phrase)
}

pub(crate) fn normalize_phrase(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
