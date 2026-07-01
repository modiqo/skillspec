use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct PolicyInitOptions {
    pub index: PathBuf,
}

#[derive(Clone, Debug)]
pub struct PolicyListOptions {
    pub index: PathBuf,
}

#[derive(Clone, Debug)]
pub struct PolicyShowOptions {
    pub index: PathBuf,
    pub profile: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PolicyGetOptions {
    pub index: PathBuf,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct PolicySetProfileOptions {
    pub index: PathBuf,
    pub name: String,
    pub mode: PolicyProfileMode,
    pub active: bool,
    pub strict: bool,
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PolicySetRuleOptions {
    pub index: PathBuf,
    pub id: String,
    pub profile: String,
    pub priority: i64,
    pub mode: PolicyRuleMode,
    pub anchor: PolicyAnchor,
    pub enabled: bool,
    pub when_any: Vec<String>,
    pub when_all: Vec<String>,
    pub when_none: Vec<String>,
    pub prefer: Vec<String>,
    pub allow: Vec<String>,
    pub suppress: Vec<String>,
    pub forbid: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PolicyRemoveRuleOptions {
    pub index: PathBuf,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct ProfileApplyOptions {
    pub index: PathBuf,
    pub profile: String,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct ProfileClearOptions {
    pub index: PathBuf,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct ProfileStatusOptions {
    pub index: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyInitReport {
    pub index: PathBuf,
    pub initialized: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyListReport {
    pub index: PathBuf,
    pub profiles: Vec<PolicyProfileReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyShowReport {
    pub index: PathBuf,
    pub profile: Option<PolicyProfileReport>,
    pub rules: Vec<PolicyRuleReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyGetReport {
    pub index: PathBuf,
    pub id: String,
    pub profile: Option<PolicyProfileReport>,
    pub rules: Vec<PolicyRuleReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicySetProfileReport {
    pub index: PathBuf,
    pub profile: PolicyProfileReport,
    pub active_profile: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicySetRuleReport {
    pub index: PathBuf,
    pub rule: PolicyRuleReport,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyRemoveRuleReport {
    pub index: PathBuf,
    pub removed: bool,
    pub id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProfileStatusReport {
    pub index: PathBuf,
    pub active_profile: Option<PolicyProfileReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProfileApplyReport {
    pub index: PathBuf,
    pub profile: String,
    pub applied: bool,
    pub dry_run: bool,
    pub mode: PolicyProfileMode,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProfileClearReport {
    pub index: PathBuf,
    pub cleared: bool,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyProfileReport {
    pub name: String,
    pub mode: PolicyProfileMode,
    pub strict: bool,
    pub default_decision: Option<String>,
    pub active: bool,
    pub description: Option<String>,
    pub updated_at_unix: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyRuleReport {
    pub id: String,
    pub profile: String,
    pub priority: i64,
    pub mode: PolicyRuleMode,
    pub anchor: PolicyAnchor,
    pub ordinal: i64,
    pub enabled: bool,
    pub predicates: PolicyPredicatesReport,
    pub preferences: Vec<PolicyPreferenceReport>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PolicyPredicatesReport {
    pub any_keywords: Vec<String>,
    pub all_keywords: Vec<String>,
    pub none_keywords: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyPreferenceReport {
    pub ordinal: i64,
    pub effect: PolicyEffect,
    pub target_kind: PolicyTargetKind,
    pub target_value: String,
    pub weight: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoutePolicyReport {
    pub profile: String,
    pub mode: PolicyProfileMode,
    pub strict: bool,
    pub matched_rules: Vec<RoutePolicyRuleMatch>,
    pub forced_bypass: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoutePolicyRuleMatch {
    pub id: String,
    pub priority: i64,
    pub mode: PolicyRuleMode,
    pub anchor: PolicyAnchor,
    pub effects: Vec<RoutePolicyEffectMatch>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoutePolicyEffectMatch {
    pub effect: PolicyEffect,
    pub target_kind: PolicyTargetKind,
    pub target_value: String,
    pub matched_candidates: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyProfileMode {
    Route,
    SoftPassthrough,
    NativePassthrough,
}

impl PolicyProfileMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::SoftPassthrough => "soft-passthrough",
            Self::NativePassthrough => "native-passthrough",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "route" => Some(Self::Route),
            "soft-passthrough" => Some(Self::SoftPassthrough),
            "native-passthrough" => Some(Self::NativePassthrough),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyRuleMode {
    Soft,
    Hard,
}

impl PolicyRuleMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Hard => "hard",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "soft" => Some(Self::Soft),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyAnchor {
    None,
    Policy,
}

impl PolicyAnchor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Policy => "policy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "policy" => Some(Self::Policy),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyEffect {
    Prefer,
    Allow,
    Suppress,
    Forbid,
}

impl PolicyEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prefer => "prefer",
            Self::Allow => "allow",
            Self::Suppress => "suppress",
            Self::Forbid => "forbid",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "prefer" => Some(Self::Prefer),
            "allow" => Some(Self::Allow),
            "suppress" => Some(Self::Suppress),
            "forbid" => Some(Self::Forbid),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyTargetKind {
    Skill,
    Tag,
    Source,
    HasSkillSpec,
}

impl PolicyTargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Tag => "tag",
            Self::Source => "source",
            Self::HasSkillSpec => "has_skill_spec",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "skill" => Some(Self::Skill),
            "tag" => Some(Self::Tag),
            "source" => Some(Self::Source),
            "has_skill_spec" => Some(Self::HasSkillSpec),
            _ => None,
        }
    }
}
