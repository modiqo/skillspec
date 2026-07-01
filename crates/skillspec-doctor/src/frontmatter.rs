use super::metrics;
use super::types::{
    FrontmatterDiscoveryFields, FrontmatterDiscoveryRiskReport, FrontmatterParseStatus,
    RiskCondition, RiskConditionKind, RiskConfidence, RiskEvidence, RiskLevel,
};
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const HARNESS_CAP_CHARS: usize = 1_536;
const SHORT_DESCRIPTION_CHARS: usize = 48;
const NEAR_LISTING_CAP_CHARS: usize = 1_200;

#[derive(Clone, Debug)]
pub(super) struct SkillSections {
    pub yaml_frontmatter: String,
    pub body: String,
    pub parse_status: FrontmatterParseStatus,
    pub parse_error: Option<String>,
}

pub(super) fn split_skill(content: &str) -> SkillSections {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(rest) = normalized.strip_prefix("---") else {
        return SkillSections {
            yaml_frontmatter: String::new(),
            body: normalized.to_owned(),
            parse_status: FrontmatterParseStatus::Missing,
            parse_error: None,
        };
    };
    let Some(rest) = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))
    else {
        return SkillSections {
            yaml_frontmatter: String::new(),
            body: normalized.to_owned(),
            parse_status: FrontmatterParseStatus::Missing,
            parse_error: None,
        };
    };

    let mut yaml = String::new();
    let mut raw = String::from("---\n");
    let mut consumed = normalized.len().saturating_sub(rest.len());
    for line in rest.split_inclusive('\n') {
        consumed += line.len();
        raw.push_str(line);
        if line.trim_end_matches(['\r', '\n']).trim() == "---" {
            let body = normalized.get(consumed..).unwrap_or("").to_owned();
            return SkillSections {
                yaml_frontmatter: yaml,
                body,
                parse_status: FrontmatterParseStatus::Parsed,
                parse_error: None,
            };
        }
        yaml.push_str(line);
    }

    SkillSections {
        yaml_frontmatter: yaml,
        body: normalized.to_owned(),
        parse_status: FrontmatterParseStatus::Unterminated,
        parse_error: Some("frontmatter starts with --- but has no closing ---".to_owned()),
    }
}

pub(super) fn analyze(path: &Path, sections: &SkillSections) -> FrontmatterDiscoveryRiskReport {
    let mut parse_status = sections.parse_status;
    let mut parse_error = sections.parse_error.clone();
    let parsed = if parse_status == FrontmatterParseStatus::Parsed {
        match serde_yaml::from_str::<Value>(&sections.yaml_frontmatter) {
            Ok(value) => Some(value),
            Err(source) => {
                parse_status = FrontmatterParseStatus::InvalidYaml;
                parse_error = Some(source.to_string());
                None
            }
        }
    } else {
        None
    };

    let name = parsed
        .as_ref()
        .and_then(|value| scalar_string(value, "name"));
    let description = parsed
        .as_ref()
        .and_then(|value| scalar_string(value, "description"));
    let when_to_use = parsed
        .as_ref()
        .and_then(|value| scalar_string(value, "when_to_use"));
    let disable_model_invocation = parsed
        .as_ref()
        .and_then(|value| scalar_bool(value, "disable-model-invocation"));
    let user_invocable = parsed
        .as_ref()
        .and_then(|value| scalar_bool(value, "user-invocable"));

    let description_text = description.as_deref().unwrap_or("").trim().to_owned();
    let when_text = when_to_use.as_deref().unwrap_or("").trim().to_owned();
    let combined = [&description_text, &when_text]
        .into_iter()
        .filter(|value| !value.is_empty())
        .map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let description_tokens = metrics::estimate_tokens(&description_text);
    let combined_discovery_tokens = metrics::estimate_tokens(&combined);
    let description_terms = terms(&description_text);
    let domain_term_count = domain_terms(&description_terms);
    let action_term_count = action_terms(&description_terms);
    let trigger_phrase_count = trigger_phrases(&combined);
    let generic_term_ratio = generic_term_ratio(&description_terms);
    let body_heading_overlap = body_heading_overlap(&description_text, &sections.body);
    let manual_only = disable_model_invocation.unwrap_or(false) || user_invocable == Some(false);
    let visibility_state = if disable_model_invocation.unwrap_or(false) {
        "user-invocable-only"
    } else if user_invocable == Some(false) {
        "hidden-from-menu"
    } else {
        "on"
    }
    .to_owned();

    let fields = FrontmatterDiscoveryFields {
        name,
        description,
        when_to_use,
        disable_model_invocation,
        user_invocable,
        parse_status,
        parse_error,
        description_chars: description_text.chars().count(),
        description_tokens,
        combined_discovery_chars: combined.chars().count(),
        combined_discovery_tokens,
        harness_cap_chars: HARNESS_CAP_CHARS,
        harness_profile: "auto".to_owned(),
        domain_term_count,
        action_term_count,
        trigger_phrase_count,
        generic_term_ratio,
        body_heading_overlap,
        manual_only,
        visibility_state,
    };

    let conditions = conditions(path, &fields);
    let score = conditions
        .iter()
        .map(|condition| usize::from(condition.score_delta))
        .sum::<usize>()
        .min(100) as u8;
    FrontmatterDiscoveryRiskReport {
        score,
        level: RiskLevel::from_score(score),
        fields,
        conditions,
    }
}

fn conditions(path: &Path, fields: &FrontmatterDiscoveryFields) -> Vec<RiskCondition> {
    let mut conditions = Vec::new();
    if fields.parse_status != FrontmatterParseStatus::Parsed
        || fields
            .description
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        conditions.push(condition(ConditionSpec {
            id: "missing_or_malformed_frontmatter",
            level: if fields.parse_status == FrontmatterParseStatus::Parsed {
                RiskLevel::High
            } else {
                RiskLevel::Critical
            },
            score_delta: if fields.parse_status == FrontmatterParseStatus::Parsed {
                18
            } else {
                24
            },
            measurement: measurements([
                ("description_chars", fields.description_chars),
                ("combined_discovery_chars", fields.combined_discovery_chars),
            ]),
            path,
            text_preview: "Skill frontmatter is missing, malformed, or lacks a usable description.",
            consequence: "The skill may be slash-invocable but weak or invisible for automatic discovery.",
            recommended_action: "Fix YAML frontmatter and provide a specific description with the main use case first.",
            basis_ids: vec![
                "claude_skill_frontmatter_discovery",
                "skilldex_format_conformance",
                "skill_metadata_supply_chain",
            ],
        }));
        return conditions;
    }

    if fields.description_chars < SHORT_DESCRIPTION_CHARS
        || (fields.domain_term_count < 2 && fields.action_term_count == 0)
    {
        conditions.push(condition(ConditionSpec {
            id: "ambiguous_short_description",
            level: RiskLevel::Medium,
            score_delta: 14,
            measurement: measurements([
                ("description_chars", fields.description_chars),
                ("domain_term_count", fields.domain_term_count),
                ("action_term_count", fields.action_term_count),
                ("trigger_phrase_count", fields.trigger_phrase_count),
            ]),
            path,
            text_preview: "Description is short or lacks enough domain/action terms.",
            consequence:
                "The harness has little specific text to match against natural user requests.",
            recommended_action:
                "Rewrite the description with domain, action, object, and common trigger phrases.",
            basis_ids: vec![
                "claude_skill_frontmatter_discovery",
                "skilldex_format_conformance",
                "skill_metadata_supply_chain",
            ],
        }));
    }

    if fields.description_chars >= SHORT_DESCRIPTION_CHARS
        && fields.generic_term_ratio >= 0.55
        && fields.domain_term_count < 3
    {
        conditions.push(condition(ConditionSpec {
            id: "overbroad_description",
            level: RiskLevel::Medium,
            score_delta: 10,
            measurement: measurements([
                ("description_chars", fields.description_chars),
                ("domain_term_count", fields.domain_term_count),
            ])
            .with_float("generic_term_ratio", fields.generic_term_ratio),
            path,
            text_preview:
                "Description uses broad generic terms without clear ownership boundaries.",
            consequence: "The skill may trigger for work it should not own.",
            recommended_action:
                "Add boundaries: what the skill handles, what it does not handle, and when to ask.",
            basis_ids: vec![
                "claude_skill_frontmatter_discovery",
                "skilldex_format_conformance",
                "skill_metadata_supply_chain",
            ],
        }));
    }

    if fields.combined_discovery_chars >= NEAR_LISTING_CAP_CHARS {
        let over_cap = fields.combined_discovery_chars > HARNESS_CAP_CHARS;
        conditions.push(condition(ConditionSpec {
            id: "description_listing_budget_risk",
            level: if over_cap {
                RiskLevel::High
            } else {
                RiskLevel::Medium
            },
            score_delta: if over_cap { 12 } else { 6 },
            measurement: measurements([
                ("combined_discovery_chars", fields.combined_discovery_chars),
                ("harness_cap_chars", fields.harness_cap_chars),
            ]),
            path,
            text_preview: "Discovery text is near or above a known skill-listing cap.",
            consequence:
                "Keywords needed for discovery may be truncated or removed in crowded skill environments.",
            recommended_action:
                "Put the key trigger first, trim low-value wording, or reduce low-priority skills to name-only.",
            basis_ids: vec!["claude_skill_frontmatter_discovery", "tiktoken_token_accounting"],
        }));
    }

    if fields.manual_only {
        conditions.push(condition(ConditionSpec {
            id: "manual_only_visibility",
            level: RiskLevel::Low,
            score_delta: 0,
            measurement: measurements([("combined_discovery_chars", fields.combined_discovery_chars)]),
            path,
            text_preview: "Frontmatter disables automatic invocation or menu visibility.",
            consequence:
                "Automatic discovery may be intentionally unavailable; users must invoke the skill explicitly.",
            recommended_action: "Report as informational unless the goal requires automatic routing.",
            basis_ids: vec!["claude_skill_frontmatter_discovery"],
        }));
    }

    conditions
}

struct ConditionSpec<'a> {
    id: &'a str,
    level: RiskLevel,
    score_delta: u8,
    measurement: BTreeMap<String, serde_json::Value>,
    path: &'a Path,
    text_preview: &'a str,
    consequence: &'a str,
    recommended_action: &'a str,
    basis_ids: Vec<&'a str>,
}

fn condition(spec: ConditionSpec<'_>) -> RiskCondition {
    RiskCondition {
        id: spec.id.to_owned(),
        kind: RiskConditionKind::DiscoveryRisk,
        level: spec.level,
        score_delta: spec.score_delta,
        confidence: RiskConfidence::Medium,
        measurement: spec.measurement,
        evidence: vec![RiskEvidence {
            path: spec.path.display().to_string(),
            line: Some(1),
            text_preview: spec.text_preview.to_owned(),
        }],
        basis_ids: spec.basis_ids.into_iter().map(str::to_owned).collect(),
        claim_scope: "discovery_risk_not_observed_routing_failure".to_owned(),
        threshold_source: "skillspec_policy_v0".to_owned(),
        consequence: spec.consequence.to_owned(),
        recommended_action: spec.recommended_action.to_owned(),
    }
}

trait MeasurementExt {
    fn with_float(self, key: &str, value: f32) -> Self;
}

impl MeasurementExt for BTreeMap<String, serde_json::Value> {
    fn with_float(mut self, key: &str, value: f32) -> Self {
        self.insert(key.to_owned(), serde_json::json!(value));
        self
    }
}

fn measurements<const N: usize>(items: [(&str, usize); N]) -> BTreeMap<String, serde_json::Value> {
    items
        .into_iter()
        .map(|(key, value)| (key.to_owned(), serde_json::json!(value)))
        .collect()
}

fn scalar_string(value: &Value, key: &str) -> Option<String> {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String(key.to_owned())))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn scalar_bool(value: &Value, key: &str) -> Option<bool> {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String(key.to_owned())))
        .and_then(Value::as_bool)
}

fn terms(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
        .map(str::trim)
        .filter(|term| term.len() > 2)
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

fn domain_terms(terms: &[String]) -> usize {
    terms
        .iter()
        .filter(|term| {
            !STOP_TERMS.contains(&term.as_str()) && !GENERIC_TERMS.contains(&term.as_str())
        })
        .collect::<BTreeSet<_>>()
        .len()
}

fn action_terms(terms: &[String]) -> usize {
    terms
        .iter()
        .filter(|term| ACTION_TERMS.contains(&term.as_str()))
        .collect::<BTreeSet<_>>()
        .len()
}

fn trigger_phrases(text: &str) -> usize {
    let lowered = text.to_ascii_lowercase();
    [
        "use when",
        "when the user",
        "user asks",
        "asks to",
        "use for",
        "trigger",
    ]
    .into_iter()
    .filter(|phrase| lowered.contains(phrase))
    .count()
}

fn generic_term_ratio(terms: &[String]) -> f32 {
    if terms.is_empty() {
        return 0.0;
    }
    let generic = terms
        .iter()
        .filter(|term| GENERIC_TERMS.contains(&term.as_str()))
        .count();
    generic as f32 / terms.len() as f32
}

fn body_heading_overlap(description: &str, body: &str) -> usize {
    let description_terms = terms(description).into_iter().collect::<BTreeSet<_>>();
    if description_terms.is_empty() {
        return 0;
    }
    body.lines()
        .filter(|line| line.trim_start().starts_with('#'))
        .flat_map(terms)
        .filter(|term| description_terms.contains(term))
        .collect::<BTreeSet<_>>()
        .len()
}

const STOP_TERMS: &[&str] = &[
    "and", "are", "for", "from", "into", "that", "the", "this", "when", "with", "your",
];

const GENERIC_TERMS: &[&str] = &[
    "analyze",
    "assistant",
    "create",
    "generate",
    "handle",
    "help",
    "helper",
    "manage",
    "process",
    "review",
    "skill",
    "summarize",
    "tool",
    "update",
    "use",
    "work",
    "workflow",
];

const ACTION_TERMS: &[&str] = &[
    "analyze",
    "audit",
    "build",
    "check",
    "compile",
    "convert",
    "create",
    "debug",
    "deploy",
    "diagnose",
    "generate",
    "import",
    "install",
    "map",
    "port",
    "prove",
    "review",
    "route",
    "summarize",
    "test",
    "validate",
    "verify",
];
