//! Emit a `tool_boundary` block for a `skill.spec.yml`.
//!
//! This grammar belongs to SkillSpec, so it needs no upstream verification. The
//! model is `crates/skillspec-core/src/spec/model.rs::ToolBoundary`, and this
//! emitter does not extend it.
//!
//! The emitted block is a contract label set, not executable policy. A harness
//! maps the labels to concrete tool ids, adapter names, product permissions, or
//! approval prompts. SkillSpec does not enforce it.

use crate::emit::incompleteness_comment;
use crate::proposal::Proposal;
use std::fmt::Write;

/// Render the proposal as a YAML `tool_boundary` block.
pub fn emit(proposal: &Proposal) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# skillspec boundary emit --format skillspec");
    let _ = writeln!(out, "# target: {}", proposal.target);
    if let Some(warning) = incompleteness_comment(proposal) {
        let _ = writeln!(out, "# {warning}");
    }
    let _ = writeln!(
        out,
        "# SkillSpec does not enforce this boundary; a harness does."
    );
    let _ = writeln!(out, "tool_boundary:");
    let _ = writeln!(out, "  default: deny");

    write_list(&mut out, "allow", &proposal.boundary.allow);
    write_list(
        &mut out,
        "permission_required_for",
        &proposal.boundary.permission_required_for,
    );
    if !proposal.boundary.forbid.is_empty() {
        write_list(&mut out, "forbid", &proposal.boundary.forbid);
    }

    if !proposal.review_required.is_empty() {
        let _ = writeln!(out, "# Review before granting:");
        for item in &proposal.review_required {
            let _ = writeln!(out, "#   {} -> {}", item.grant, item.patterns.join(", "));
        }
    }
    out
}

fn write_list(out: &mut String, name: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    let _ = writeln!(out, "  {name}:");
    for value in values {
        let _ = writeln!(out, "    - \"{value}\"");
    }
}

#[cfg(test)]
mod tests {
    use crate::{analyze, proposal};
    use std::path::PathBuf;

    fn emitted(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/effects")
            .join(name);
        super::emit(&proposal::compile(&analyze(&path).expect("analysis")))
    }

    #[test]
    fn the_block_always_declares_a_deny_default() {
        for name in ["clean-formatter", "direct-chain"] {
            assert!(emitted(name).contains("default: deny"), "{name}");
        }
    }

    #[test]
    fn grants_are_quoted_labels_under_allow() {
        let text = emitted("clean-formatter");
        assert!(text.contains("  allow:"));
        assert!(text.contains("    - \"proc.exec:git\""));
    }

    #[test]
    fn sensitive_grants_appear_only_under_permission_required_for() {
        let text = emitted("secret-reader");
        let allow_section = text
            .split("permission_required_for")
            .next()
            .unwrap_or_default();
        assert!(!allow_section.contains("fs.read:secret"));
        assert!(text.contains("permission_required_for:"));
        assert!(text.contains("    - \"fs.read:secret\""));
    }

    #[test]
    fn review_items_name_the_literal_paths() {
        assert!(emitted("direct-chain").contains("~/.aws/credentials"));
    }

    #[test]
    fn an_incomplete_proposal_says_so_in_the_artifact() {
        let text = emitted("dynamic-endpoint");
        assert!(text.contains("# incomplete:"));
    }

    #[test]
    fn a_complete_proposal_carries_no_incompleteness_warning() {
        assert!(!emitted("clean-formatter").contains("# incomplete:"));
    }

    #[test]
    fn every_artifact_states_that_skillspec_does_not_enforce_it() {
        assert!(emitted("clean-formatter").contains("does not enforce"));
    }
}
