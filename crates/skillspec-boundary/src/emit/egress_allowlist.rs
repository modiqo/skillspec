//! Emit a plain host allow-list for a proxy or container network policy.
//!
//! Deny-by-default is structural here: an allow-list consumed by an egress
//! proxy denies every host it does not name. That makes this the one target
//! whose enforcement semantics need no assumption about a harness.
//!
//! Hosts are exact. A parent domain is never emitted in place of the hosts
//! actually observed, because widening is the usual way a least-privilege
//! policy quietly becomes a permissive one.

use crate::effect::EffectClass;
use crate::emit::incompleteness_comment;
use crate::proposal::Proposal;
use std::collections::BTreeSet;
use std::fmt::Write;

/// Render the network grants as a host list.
pub fn emit(proposal: &Proposal) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# skillspec boundary emit --format egress-allowlist");
    let _ = writeln!(out, "# target: {}", proposal.target);
    if let Some(warning) = incompleteness_comment(proposal) {
        let _ = writeln!(out, "# {warning}");
        let _ = writeln!(
            out,
            "# An unresolved host is not in this list and will be denied."
        );
    }

    let hosts = proposal
        .grants
        .iter()
        .filter(|grant| matches!(grant.class, EffectClass::NetEgress | EffectClass::NetFetch))
        .map(|grant| grant.token.clone())
        .collect::<BTreeSet<_>>();

    if hosts.is_empty() {
        // "No host required" and "no host could be resolved" are different
        // facts, and conflating them would tell a reader the skill has no
        // network surface when in truth its surface is unreadable.
        let unresolved_network = proposal
            .unresolved
            .iter()
            .filter(|item| matches!(item.class, EffectClass::NetEgress | EffectClass::NetFetch))
            .count();
        if unresolved_network > 0 {
            let _ = writeln!(
                out,
                "# {unresolved_network} network effect(s) had no resolvable host, so none is listed."
            );
            let _ = writeln!(out, "# Every network call this skill makes will be denied.");
        } else {
            let _ = writeln!(out, "# No network host is required by this skill.");
        }
        return out;
    }
    for host in hosts {
        let _ = writeln!(out, "{host}");
    }
    out
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
    fn observed_hosts_are_listed_exactly_once() {
        let text = emitted("direct-chain");
        assert!(text.contains("archive.example.com"));
        assert_eq!(text.matches("archive.example.com").count(), 1);
    }

    #[test]
    fn a_skill_needing_no_network_says_so_rather_than_emitting_nothing() {
        // An empty file is indistinguishable from a failed run.
        let text = emitted("clean-formatter");
        assert!(text.contains("No network host is required"));
    }

    #[test]
    fn an_unresolved_host_is_absent_and_the_consequence_is_stated() {
        let text = emitted("dynamic-endpoint");
        assert!(text.contains("# incomplete:"));
        assert!(text.contains("will be denied"));
        assert!(!text.contains("$ENDPOINT"));
    }

    #[test]
    fn no_resolvable_host_is_not_reported_as_no_network_surface() {
        // The distinction matters: one says the skill does not use the network,
        // the other says we could not read what it uses.
        let text = emitted("dynamic-endpoint");
        assert!(text.contains("no resolvable host"));
        assert!(!text.contains("No network host is required"));
    }

    #[test]
    fn no_wildcard_or_parent_domain_is_emitted() {
        let text = emitted("github-reporter");
        assert!(!text.contains('*'));
        assert!(text.contains("api.github.com"));
        // The parent domain is never substituted for the observed hosts.
        assert!(!text.lines().any(|line| line.trim() == "github.com"));
    }
}
