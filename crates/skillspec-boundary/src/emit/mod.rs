//! Rendering a boundary proposal into a policy grammar something can enforce.
//!
//! Every target here can express deny-by-default. That is not a coincidence: a
//! target that cannot express it does not give the fail-closed property the
//! whole design rests on, so emitting into one would hand a reader weaker
//! protection than the report implies.
//!
//! # Verified upstream grammars
//!
//! Formats owned by other projects change without regard for this repo, and a
//! stale emitter produces a policy file that parses, applies nothing, and reads
//! as protection. Each emitter for a foreign grammar therefore records the date
//! its syntax was last checked against that project's published documentation,
//! and the stamp travels in the emitted artifact.
//!
//! ## Claude Code frontmatter is deliberately absent
//!
//! `allowed-tools` was checked against the Claude Code skills documentation on
//! 2026-07-27 and **is not a restriction**. The documentation states it grants
//! permission for the listed tools during the invoking turn and that it "does
//! not restrict which tools are available: every tool remains callable".
//!
//! Emitting an enumerated effect set into that field would pre-approve exactly
//! the effects the analysis found, turning a least-privilege report into a
//! permission grant. There is no correct way to emit this proposal into
//! `allowed-tools`, so no such target exists. See
//! `docs/design/security/37-boundary-proposal-compiler.md`.

pub mod egress_allowlist;
pub mod skillspec;

use crate::proposal::Proposal;
use skillspec_core::error::{Error, Result};

/// A policy grammar a proposal can be rendered into.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmitTarget {
    /// `tool_boundary` block for a `skill.spec.yml`.
    SkillSpec,
    /// Plain host list for a proxy or network policy.
    EgressAllowlist,
    /// The proposal itself, for another tool to consume.
    Json,
}

impl EmitTarget {
    /// Parse a `--format` value.
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "skillspec" => Ok(Self::SkillSpec),
            "egress-allowlist" => Ok(Self::EgressAllowlist),
            "json" => Ok(Self::Json),
            other => Err(Error::InvalidInput {
                message: format!(
                    "unknown boundary format {other:?}; supported formats are {}",
                    Self::names().join(", ")
                ),
            }),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SkillSpec => "skillspec",
            Self::EgressAllowlist => "egress-allowlist",
            Self::Json => "json",
        }
    }

    pub fn names() -> Vec<&'static str> {
        vec!["skillspec", "egress-allowlist", "json"]
    }
}

/// Render `proposal` into `target`.
pub fn emit(proposal: &Proposal, target: EmitTarget) -> Result<String> {
    match target {
        EmitTarget::SkillSpec => Ok(skillspec::emit(proposal)),
        EmitTarget::EgressAllowlist => Ok(egress_allowlist::emit(proposal)),
        EmitTarget::Json => serde_json::to_string_pretty(proposal)
            .map(|json| format!("{json}\n"))
            .map_err(Error::RenderJson),
    }
}

/// The incompleteness warning every commentable target carries.
///
/// A proposal derived from a surface that was not fully determined does not
/// cover everything the skill can do, and a reader who applies it without
/// knowing that has been misled about what they are protected from.
pub(crate) fn incompleteness_comment(proposal: &Proposal) -> Option<String> {
    if proposal.complete {
        return None;
    }
    Some(format!(
        "incomplete: {} effect(s) could not be resolved to a target",
        proposal.unresolved.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::EmitTarget;

    #[test]
    fn format_names_round_trip() {
        for name in EmitTarget::names() {
            assert_eq!(EmitTarget::parse(name).unwrap().as_str(), name);
        }
    }

    #[test]
    fn an_unknown_format_lists_the_supported_ones() {
        let error = EmitTarget::parse("claude-frontmatter")
            .unwrap_err()
            .to_string();
        assert!(error.contains("skillspec"));
        assert!(error.contains("egress-allowlist"));
    }

    #[test]
    fn format_parsing_is_case_insensitive_and_trims() {
        assert_eq!(
            EmitTarget::parse("  SkillSpec ").unwrap(),
            EmitTarget::SkillSpec
        );
    }
}
