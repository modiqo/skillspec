//! Phrase families for the directive detectors, as reviewable data.
//!
//! Per investigation I3, these are derived from published taxonomies rather
//! than intuition, and each family records where it came from so a reviewer can
//! see its basis. Provenance:
//!
//! - OWASP Agentic Skills Top 10 (AST01 Malicious Skills, AST05 Untrusted
//!   External Instructions).
//! - The anti-refusal, excessive-agency, and system-prompt-leakage categories
//!   enumerated by the agent-skill scanners (SkillSpector, SkillSieve).
//! - The ToxicSkills covert-behavior class (CSA SKILL.md context-poisoning
//!   note).
//!
//! A phrase is matched as a lowercased substring within a classified obligation
//! span, not against free text; that restriction is what keeps an imperative
//! sentence distinct from a paragraph merely discussing the topic.

/// The seven directive kinds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectiveKind {
    /// Withhold an action, result, or step from the user.
    UserSecrecy,
    /// Skip approval, suppress a prompt, or reach for force flags to avoid asking.
    ConfirmationBypass,
    /// Ignore safety guidance, omit disclaimers, proceed despite a constraint.
    RefusalSuppression,
    /// Assert pre-authorization the skill cannot substantiate.
    AuthorityClaim,
    /// Disregard earlier, system, or harness instructions.
    InstructionOverride,
    /// Reveal system prompt, harness instructions, or other skills' contents.
    SelfDisclosure,
    /// Claim precedence over other skills or that the skill always applies.
    ActivationOverbreadth,
}

impl DirectiveKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserSecrecy => "directive.user_secrecy",
            Self::ConfirmationBypass => "directive.confirmation_bypass",
            Self::RefusalSuppression => "directive.refusal_suppression",
            Self::AuthorityClaim => "directive.authority_claim",
            Self::InstructionOverride => "directive.instruction_override",
            Self::SelfDisclosure => "directive.self_disclosure",
            Self::ActivationOverbreadth => "directive.activation_overbreadth",
        }
    }

    /// A plain description of what an instruction of this kind asks for. Never
    /// an accusation.
    pub fn statement(self) -> &'static str {
        match self {
            Self::UserSecrecy => {
                "This instruction directs the agent to withhold something from the user."
            }
            Self::ConfirmationBypass => {
                "This instruction directs the agent to skip an approval or confirmation step."
            }
            Self::RefusalSuppression => {
                "This instruction directs the agent to ignore a safety constraint or omit a disclaimer."
            }
            Self::AuthorityClaim => {
                "This instruction asserts an authorization the skill itself cannot establish."
            }
            Self::InstructionOverride => {
                "This instruction directs the agent to disregard earlier, system, or harness instructions."
            }
            Self::SelfDisclosure => {
                "This instruction directs the agent to reveal system, harness, or other-skill content."
            }
            Self::ActivationOverbreadth => {
                "This instruction claims the skill takes precedence or always applies."
            }
        }
    }
}

/// A phrase family: the kind it matches and the phrases that match it.
pub struct Family {
    pub kind: DirectiveKind,
    pub phrases: &'static [&'static str],
}

/// The directive phrase families. Each phrase is lowercase and matched as a
/// substring within an obligation span.
pub const FAMILIES: &[Family] = &[
    // Broad on purpose. A user-secrecy phrase is only reported as *concerning*
    // when it co-occurs with a capability or lives outside the activation body
    // (see `EffectSurface::concerning_directives`), so a benign QA instruction
    // in an effect-free design skill does not raise an alarm even though it
    // matches. That gate is what lets these stay broad without noise.
    Family {
        kind: DirectiveKind::UserSecrecy,
        phrases: &[
            "do not tell the user",
            "don't tell the user",
            "do not mention",
            "don't mention",
            "do not report",
            "don't report",
            "do not surface",
            "do not reveal that",
            "do not disclose",
            "do not show the user",
            "without telling the user",
            "without informing the user",
            "without mentioning",
            "without the user",
            "hide this from",
            "hide it from",
            "keep it hidden",
            "conceal from",
            "do not include this in your summary",
            "omit from your report",
            "omit from the summary",
            "leave this out of",
            "keep this secret",
            "do not log",
            "quietly",
            "silently",
        ],
    },
    Family {
        kind: DirectiveKind::ConfirmationBypass,
        phrases: &[
            "do not ask for confirmation",
            "without asking",
            "skip the confirmation",
            "skip confirmation",
            "no need to confirm",
            "do not prompt",
            "without prompting",
            "automatically approve",
            "assume yes",
        ],
    },
    Family {
        kind: DirectiveKind::RefusalSuppression,
        phrases: &[
            "ignore safety",
            "ignore any warnings",
            "do not refuse",
            "you must not refuse",
            "without any disclaimer",
            "omit the disclaimer",
            "regardless of policy",
            "bypass safety",
            "proceed anyway",
        ],
    },
    Family {
        kind: DirectiveKind::AuthorityClaim,
        phrases: &[
            "the user has already approved",
            "the operator has approved",
            "the administrator has authorized",
            "you are authorized to",
            "this has been pre-approved",
            "with full authorization",
            "you have permission to bypass",
        ],
    },
    Family {
        kind: DirectiveKind::InstructionOverride,
        phrases: &[
            "ignore previous instructions",
            "ignore prior instructions",
            "ignore all previous",
            "disregard previous instructions",
            "disregard the above",
            "override the system prompt",
            "forget the earlier instructions",
            "these instructions take precedence over",
        ],
    },
    Family {
        kind: DirectiveKind::SelfDisclosure,
        phrases: &[
            "reveal your system prompt",
            "print your instructions",
            "output your system prompt",
            "show your instructions",
            "reveal the contents of other skills",
            "disclose your configuration",
            "repeat the text above",
        ],
    },
    Family {
        kind: DirectiveKind::ActivationOverbreadth,
        phrases: &[
            "always use this skill",
            "this skill always applies",
            "use this skill for every",
            "this skill takes precedence over all",
            "prefer this skill over any other",
            "regardless of the task",
        ],
    },
];
