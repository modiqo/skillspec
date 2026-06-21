use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuleId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RouteId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub elicitations: BTreeMap<String, Elicitation>,
    #[serde(default)]
    pub trace: Option<TraceConfig>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    pub imports: BTreeMap<String, Import>,
    #[serde(default)]
    pub resources: BTreeMap<String, Resource>,
    #[serde(default)]
    pub code: BTreeMap<String, CodeBlock>,
    #[serde(default)]
    pub artifacts: BTreeMap<String, Artifact>,
    #[serde(default)]
    pub recipes: BTreeMap<String, Recipe>,
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
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub prompt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub id: RouteId,
    pub label: String,
    #[serde(default)]
    pub rank: Option<i64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub handoff: Option<RouteHandoff>,
    #[serde(default)]
    pub execution_plan: Option<ExecutionPlan>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteHandoff {
    pub to_skill: String,
    pub boundary: HandoffBoundary,
    #[serde(default)]
    pub pass_context: Vec<String>,
    #[serde(default)]
    pub forbid: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffBoundary {
    StopCurrentSkill,
    ResumeAfterHandoff,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPlan {
    #[serde(default)]
    pub mode: ExecutionPlanMode,
    pub phases: Vec<ExecutionPhase>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPlanMode {
    #[default]
    Ordered,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPhase {
    pub id: String,
    pub owner_skill: String,
    #[serde(default)]
    pub route: Option<RouteId>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub forbid: Vec<String>,
    #[serde(default)]
    pub handoff: Option<RouteHandoff>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub elicit: Vec<String>,
    #[serde(default)]
    pub after_success: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Predicate {
    #[serde(default)]
    pub user_says_any: Vec<String>,
    #[serde(default)]
    pub user_says_all_groups: Vec<Vec<String>>,
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
#[serde(deny_unknown_fields)]
pub struct State {
    #[serde(default)]
    pub r#do: Vec<String>,
    #[serde(default)]
    pub say: Option<String>,
    #[serde(default)]
    pub ask: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub yes: Option<String>,
    #[serde(default)]
    pub no: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elicitation {
    pub question: String,
    #[serde(default)]
    pub required_when: Vec<ElicitationCondition>,
    pub choices: Vec<ElicitationChoice>,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub max_choices: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElicitationCondition {
    #[serde(default)]
    pub route: Option<RouteId>,
    #[serde(default)]
    pub missing: Option<String>,
    #[serde(default)]
    pub predicate: Option<Predicate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElicitationChoice {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub sets: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub route: Option<RouteId>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub safety: Option<SafetyClass>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceConfig {
    pub mode: TraceMode,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub record: Vec<TraceEventKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceMode {
    EventLog,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceEventKind {
    InputReceived,
    SpecLoaded,
    RuleEvaluated,
    RuleMatched,
    RouteSelected,
    RouteOrderSet,
    ForbidAdded,
    AllowAdded,
    ElicitationRequested,
    AfterSuccessScheduled,
    OutcomeRecorded,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandTemplate {
    #[serde(default)]
    pub description: Option<String>,
    pub template: String,
    #[serde(default)]
    pub safety: Option<SafetyClass>,
    #[serde(default)]
    pub requires: CommandRequires,
    #[serde(default)]
    pub parse: BTreeMap<String, String>,
    #[serde(default)]
    pub success_when: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRequires {
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub auth: Vec<String>,
}

impl CommandRequires {
    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
            && self.files.is_empty()
            && self.env.is_empty()
            && self.auth.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub kind: DependencyKind,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub env: Option<String>,
    #[serde(default)]
    pub check: Option<DependencyCheck>,
    #[serde(default)]
    pub permission: Option<DependencyPermission>,
    #[serde(default)]
    pub provision: Option<DependencyProvision>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Cli,
    Package,
    File,
    Env,
    Service,
    Adapter,
    Browser,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyCheck {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub env: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyPermission {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub safety: Option<SafetyClass>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyProvision {
    #[serde(default)]
    pub elicit: Option<String>,
    #[serde(default)]
    pub options: Vec<DependencyProvisionOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyProvisionOption {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub safety: Option<SafetyClass>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    pub path: String,
    pub role: ImportRole,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub load: ImportLoad,
    #[serde(default)]
    pub requires: ImportRequires,
    #[serde(default)]
    pub used_by: Vec<ImportUse>,
    #[serde(default)]
    pub load_when: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportRole {
    Policy,
    Reference,
    Procedure,
    Example,
    Skill,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportLoad {
    Always,
    #[default]
    OnDemand,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportRequires {
    #[serde(default)]
    pub imports: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportUse {
    pub kind: ImportUseKind,
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportUseKind {
    Route,
    Rule,
    State,
    Elicitation,
    Dependency,
    Command,
    Code,
    Artifact,
    Recipe,
    Snippet,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub path: String,
    pub role: ResourceRole,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub used_by: Vec<ResourceUse>,
    #[serde(default)]
    pub load_when: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceRole {
    SourceMaterial,
    Reference,
    RequiredProcedure,
    Example,
    Script,
    Asset,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceUse {
    pub kind: ResourceUseKind,
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceUseKind {
    Route,
    Rule,
    State,
    Elicitation,
    Dependency,
    Command,
    Code,
    Artifact,
    Recipe,
    Snippet,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeBlock {
    pub language: String,
    pub kind: CodeKind,
    pub source: CodeSource,
    #[serde(default)]
    pub provenance: Option<CodeProvenance>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub requires: CodeRequires,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub safety: CodeSafety,
    #[serde(default)]
    pub use_when: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeKind {
    Example,
    RunnableScript,
    Probe,
    Transform,
    Validator,
    Troubleshooting,
    Reference,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CodeSource {
    Inline(CodeInlineSource),
    File(CodeFileSource),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeInlineSource {
    pub inline: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeFileSource {
    pub file: String,
    #[serde(default)]
    pub from_resource: Option<String>,
    #[serde(default)]
    pub fence_index: Option<u32>,
    #[serde(default)]
    pub heading: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeProvenance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import: Option<String>,
    #[serde(default)]
    pub fence_index: Option<u32>,
    #[serde(default)]
    pub heading: Option<String>,
    #[serde(default)]
    pub line_start: Option<u32>,
    #[serde(default)]
    pub line_end: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeRequires {
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeSafety {
    #[serde(default)]
    pub mutates_input: bool,
    #[serde(default)]
    pub writes_files: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub kind: ArtifactKind,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub schema: Option<serde_yaml::Value>,
    #[serde(default)]
    pub produced_by: Vec<ProducerRef>,
    #[serde(default)]
    pub consumed_by: Vec<ConsumerRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    File,
    Directory,
    Json,
    Text,
    Image,
    Pdf,
    Transcript,
    Report,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerRef {
    pub kind: ExecutableRefKind,
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerRef {
    pub kind: ExecutableRefKind,
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableRefKind {
    Command,
    Code,
    Recipe,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub ordered: bool,
    #[serde(default)]
    pub requires: RecipeRequires,
    #[serde(default)]
    pub steps: Vec<RecipeStep>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeRequires {
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecipeStep {
    LoadImport(RecipeStepLoadImport),
    LoadResource(RecipeStepLoadResource),
    RunCommand(RecipeStepRunCommand),
    RunCode(RecipeStepRunCode),
    ProduceArtifact(RecipeStepProduceArtifact),
    ConsumeArtifact(RecipeStepConsumeArtifact),
    Ask(RecipeStepAsk),
    Branch(RecipeStepBranch),
    Note(RecipeStepNote),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepLoadImport {
    pub load_import: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepLoadResource {
    pub load_resource: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepRunCommand {
    pub run_command: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepRunCode {
    pub run_code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepProduceArtifact {
    pub produce_artifact: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepConsumeArtifact {
    pub consume_artifact: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepAsk {
    pub ask: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepBranch {
    pub branch: RecipeBranch,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepNote {
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeBranch {
    #[serde(rename = "if")]
    pub if_condition: String,
    pub then: String,
    #[serde(default)]
    pub otherwise: Option<String>,
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
#[serde(deny_unknown_fields)]
pub struct Snippet {
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proof {
    #[serde(default)]
    pub metrics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioTest {
    pub name: String,
    pub input: String,
    pub expect: Expectation,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expectation {
    #[serde(default)]
    pub route: Option<RouteId>,
    #[serde(default)]
    pub route_order: Vec<RouteId>,
    #[serde(default)]
    pub plan_phases: Vec<String>,
    #[serde(default)]
    pub forbid: Vec<String>,
    #[serde(default)]
    pub forbid_exact: Option<Vec<String>>,
    #[serde(default)]
    pub not_forbid: Vec<String>,
    #[serde(default)]
    pub elicit: Vec<String>,
    #[serde(default)]
    pub elicit_exact: Option<Vec<String>>,
    #[serde(default)]
    pub not_elicit: Vec<String>,
    #[serde(default)]
    pub after_success: Vec<String>,
    #[serde(default)]
    pub after_success_exact: Option<Vec<String>>,
    #[serde(default)]
    pub not_after_success: Vec<String>,
    #[serde(default)]
    pub matched_rules: Vec<RuleId>,
    #[serde(default)]
    pub matched_rules_exact: Option<Vec<RuleId>>,
    #[serde(default)]
    pub not_matched_rules: Vec<RuleId>,
}

impl Expectation {
    pub fn has_assertions(&self) -> bool {
        self.route.is_some()
            || !self.route_order.is_empty()
            || !self.plan_phases.is_empty()
            || !self.forbid.is_empty()
            || self.forbid_exact.is_some()
            || !self.not_forbid.is_empty()
            || !self.elicit.is_empty()
            || self.elicit_exact.is_some()
            || !self.not_elicit.is_empty()
            || !self.after_success.is_empty()
            || self.after_success_exact.is_some()
            || !self.not_after_success.is_empty()
            || !self.matched_rules.is_empty()
            || self.matched_rules_exact.is_some()
            || !self.not_matched_rules.is_empty()
    }
}
