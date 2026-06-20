use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuleId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RouteId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillSpec {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub applies_when: Vec<serde_yaml::Value>,
    #[serde(default)]
    pub entry: Option<Entry>,
    #[serde(default)]
    pub routes: Vec<Route>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub states: BTreeMap<String, State>,
    #[serde(default)]
    pub commands: BTreeMap<String, CommandTemplate>,
    #[serde(default)]
    pub snippets: BTreeMap<String, Snippet>,
    #[serde(default)]
    pub closures: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub proof: Option<Proof>,
    #[serde(default)]
    pub tests: Vec<ScenarioTest>,
    #[serde(default)]
    pub review_required: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub prompt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub id: RouteId,
    pub label: String,
    #[serde(default)]
    pub rank: Option<i64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub checks: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub id: RuleId,
    #[serde(default)]
    pub when: Predicate,
    #[serde(default)]
    pub prefer: Option<RouteId>,
    #[serde(default)]
    pub route_order: Vec<RouteId>,
    #[serde(default)]
    pub forbid: Vec<String>,
    #[serde(default)]
    pub allow: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub after_success: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Predicate {
    #[serde(default)]
    pub user_says_any: Vec<String>,
    #[serde(default)]
    pub task_recurrence_likely: Option<bool>,
    #[serde(default)]
    pub domain_object_task: Option<bool>,
    #[serde(default)]
    pub interactive_prompt_likely: Option<bool>,
    #[serde(default)]
    pub command_likely_long_running: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub r#do: Vec<String>,
    #[serde(default)]
    pub say: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub yes: Option<String>,
    #[serde(default)]
    pub no: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandTemplate {
    #[serde(default)]
    pub description: Option<String>,
    pub template: String,
    #[serde(default)]
    pub safety: Option<SafetyClass>,
    #[serde(default)]
    pub requires: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub parse: BTreeMap<String, String>,
    #[serde(default)]
    pub success_when: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClass {
    ReadOnly,
    LocalRead,
    LocalWrite,
    NetworkRead,
    NetworkWrite,
    BrowserAttach,
    CredentialRequest,
    Destructive,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snippet {
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof {
    #[serde(default)]
    pub metrics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioTest {
    pub name: String,
    pub input: String,
    pub expect: Expectation,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Expectation {
    #[serde(default)]
    pub route: Option<RouteId>,
    #[serde(default)]
    pub route_order: Vec<RouteId>,
    #[serde(default)]
    pub forbid: Vec<String>,
    #[serde(default)]
    pub after_success: Vec<String>,
}
