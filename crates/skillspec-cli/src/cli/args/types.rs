use clap::ValueEnum;
use skillspec::{
    compiler, grammar, install::HarnessTarget, router, router_policy, sensemake, source_map,
    visibility, workspace,
};

#[derive(Clone, Debug, clap::ValueEnum)]
pub(in crate::cli) enum CompileTarget {
    CodexSkill,
    ClaudeSkill,
    Markdown,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(in crate::cli) enum WorkspaceCompileTarget {
    CodexSkill,
    ClaudeSkill,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(in crate::cli) enum WorkspaceVisibilityPolicyArg {
    EntryImplicit,
    AllImplicit,
    AllManual,
    None,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(in crate::cli) enum RouterExecutionModeArg {
    Direct,
    Durable,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(in crate::cli) enum RouteHarnessArg {
    Agents,
    Codex,
    ClaudeLocal,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(in crate::cli) enum RouterPolicyProfileModeArg {
    Route,
    SoftPassthrough,
    NativePassthrough,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(in crate::cli) enum RouterPolicyRuleModeArg {
    Soft,
    Hard,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(in crate::cli) enum RouterPolicyAnchorArg {
    None,
    Policy,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(in crate::cli) enum VisibilityArg {
    Implicit,
    ManualOnly,
    NameOnly,
    Off,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(in crate::cli) enum VisibilityProfileArg {
    RouterManaged,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(in crate::cli) enum InstallTargetArg {
    Agents,
    Codex,
    ClaudeLocal,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(in crate::cli) enum WorkspaceInstallSlugPolicyArg {
    WorkspacePath,
    LocalName,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(in crate::cli) enum SenseViewArg {
    Index,
    Summary,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(in crate::cli) enum GuideModeArg {
    Agent,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(in crate::cli) enum ChecklistStageArg {
    Entry,
    Loop,
    Exit,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(in crate::cli) enum SourceViewArg {
    Index,
    Summary,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(in crate::cli) enum GrammarViewArg {
    Index,
    Summary,
    Porting,
    Full,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(in crate::cli) enum GrammarChecklistForArg {
    ImportSkill,
}

#[derive(Clone, Debug, ValueEnum)]
pub(in crate::cli) enum ProgressEventArg {
    PhaseStarted,
    RequirementStarted,
    RequirementSatisfied,
    RequirementFailed,
    StatsCollected,
    ObligationSatisfied,
    RouteFulfilled,
    RouteCheckCompleted,
    AfterSuccessCompleted,
    ElicitationAnswered,
    ElicitationWaived,
    EvidenceAttached,
    HandoffStarted,
    HandoffCompleted,
    PhaseCompleted,
    PhaseBlocked,
}

impl From<InstallTargetArg> for HarnessTarget {
    fn from(value: InstallTargetArg) -> Self {
        match value {
            InstallTargetArg::Agents => Self::Agents,
            InstallTargetArg::Codex => Self::Codex,
            InstallTargetArg::ClaudeLocal => Self::ClaudeLocal,
        }
    }
}

impl From<WorkspaceInstallSlugPolicyArg> for workspace::WorkspaceInstallSlugPolicy {
    fn from(value: WorkspaceInstallSlugPolicyArg) -> Self {
        match value {
            WorkspaceInstallSlugPolicyArg::WorkspacePath => Self::WorkspacePath,
            WorkspaceInstallSlugPolicyArg::LocalName => Self::LocalName,
        }
    }
}

impl From<RouterExecutionModeArg> for router::ExecutionMode {
    fn from(value: RouterExecutionModeArg) -> Self {
        match value {
            RouterExecutionModeArg::Direct => Self::Direct,
            RouterExecutionModeArg::Durable => Self::Durable,
        }
    }
}

impl From<RouteHarnessArg> for router::RouteHarness {
    fn from(value: RouteHarnessArg) -> Self {
        match value {
            RouteHarnessArg::Agents => Self::Agents,
            RouteHarnessArg::Codex => Self::Codex,
            RouteHarnessArg::ClaudeLocal => Self::ClaudeLocal,
        }
    }
}

impl From<RouterPolicyProfileModeArg> for router_policy::PolicyProfileMode {
    fn from(value: RouterPolicyProfileModeArg) -> Self {
        match value {
            RouterPolicyProfileModeArg::Route => Self::Route,
            RouterPolicyProfileModeArg::SoftPassthrough => Self::SoftPassthrough,
            RouterPolicyProfileModeArg::NativePassthrough => Self::NativePassthrough,
        }
    }
}

impl From<RouterPolicyRuleModeArg> for router_policy::PolicyRuleMode {
    fn from(value: RouterPolicyRuleModeArg) -> Self {
        match value {
            RouterPolicyRuleModeArg::Soft => Self::Soft,
            RouterPolicyRuleModeArg::Hard => Self::Hard,
        }
    }
}

impl From<RouterPolicyAnchorArg> for router_policy::PolicyAnchor {
    fn from(value: RouterPolicyAnchorArg) -> Self {
        match value {
            RouterPolicyAnchorArg::None => Self::None,
            RouterPolicyAnchorArg::Policy => Self::Policy,
        }
    }
}

impl From<VisibilityArg> for router::Visibility {
    fn from(value: VisibilityArg) -> Self {
        match value {
            VisibilityArg::Implicit => Self::Implicit,
            VisibilityArg::ManualOnly => Self::ManualOnly,
            VisibilityArg::NameOnly => Self::NameOnly,
            VisibilityArg::Off => Self::Off,
        }
    }
}

impl From<VisibilityProfileArg> for visibility::VisibilityProfile {
    fn from(value: VisibilityProfileArg) -> Self {
        match value {
            VisibilityProfileArg::RouterManaged => Self::RouterManaged,
        }
    }
}

impl From<CompileTarget> for compiler::Target {
    fn from(value: CompileTarget) -> Self {
        match value {
            CompileTarget::CodexSkill => compiler::Target::CodexSkill,
            CompileTarget::ClaudeSkill => compiler::Target::ClaudeSkill,
            CompileTarget::Markdown => compiler::Target::Markdown,
        }
    }
}

impl From<WorkspaceCompileTarget> for compiler::Target {
    fn from(value: WorkspaceCompileTarget) -> Self {
        match value {
            WorkspaceCompileTarget::CodexSkill => compiler::Target::CodexSkill,
            WorkspaceCompileTarget::ClaudeSkill => compiler::Target::ClaudeSkill,
        }
    }
}

impl From<WorkspaceVisibilityPolicyArg> for skillspec::workspace::WorkspaceVisibilityPolicy {
    fn from(value: WorkspaceVisibilityPolicyArg) -> Self {
        match value {
            WorkspaceVisibilityPolicyArg::EntryImplicit => Self::EntryImplicit,
            WorkspaceVisibilityPolicyArg::AllImplicit => Self::AllImplicit,
            WorkspaceVisibilityPolicyArg::AllManual => Self::AllManual,
            WorkspaceVisibilityPolicyArg::None => Self::None,
        }
    }
}

impl From<SenseViewArg> for sensemake::View {
    fn from(value: SenseViewArg) -> Self {
        match value {
            SenseViewArg::Index => Self::Index,
            SenseViewArg::Summary => Self::Summary,
            SenseViewArg::Full => Self::Full,
        }
    }
}

impl From<SourceViewArg> for source_map::SourceView {
    fn from(value: SourceViewArg) -> Self {
        match value {
            SourceViewArg::Index => Self::Index,
            SourceViewArg::Summary => Self::Summary,
            SourceViewArg::Full => Self::Full,
        }
    }
}

impl From<GrammarViewArg> for grammar::GrammarView {
    fn from(value: GrammarViewArg) -> Self {
        match value {
            GrammarViewArg::Index => Self::Index,
            GrammarViewArg::Summary => Self::Summary,
            GrammarViewArg::Porting => Self::Porting,
            GrammarViewArg::Full => Self::Full,
        }
    }
}

impl From<GrammarChecklistForArg> for grammar::ChecklistSubject {
    fn from(value: GrammarChecklistForArg) -> Self {
        match value {
            GrammarChecklistForArg::ImportSkill => Self::ImportSkill,
        }
    }
}

impl From<ProgressEventArg> for String {
    fn from(value: ProgressEventArg) -> Self {
        match value {
            ProgressEventArg::PhaseStarted => "phase_started",
            ProgressEventArg::RequirementStarted => "requirement_started",
            ProgressEventArg::RequirementSatisfied => "requirement_satisfied",
            ProgressEventArg::RequirementFailed => "requirement_failed",
            ProgressEventArg::StatsCollected => "stats_collected",
            ProgressEventArg::ObligationSatisfied => "obligation_satisfied",
            ProgressEventArg::RouteFulfilled => "route_fulfilled",
            ProgressEventArg::RouteCheckCompleted => "route_check_completed",
            ProgressEventArg::AfterSuccessCompleted => "after_success_completed",
            ProgressEventArg::ElicitationAnswered => "elicitation_answered",
            ProgressEventArg::ElicitationWaived => "elicitation_waived",
            ProgressEventArg::EvidenceAttached => "evidence_attached",
            ProgressEventArg::HandoffStarted => "handoff_started",
            ProgressEventArg::HandoffCompleted => "handoff_completed",
            ProgressEventArg::PhaseCompleted => "phase_completed",
            ProgressEventArg::PhaseBlocked => "phase_blocked",
        }
        .to_owned()
    }
}
