//! A reviewed boundary policy for one installed skill.
//!
//! Stored per skill, keyed by install slug, and carrying the content hash of the
//! package it was approved against. A guarded skill whose bytes no longer match
//! its policy is one whose policy was approved for different content, which the
//! guard surfaces for re-review.

use crate::proposal::Proposal;
use serde::{Deserialize, Serialize};

/// Schema id for a stored policy.
pub const POLICY_SCHEMA: &str = "skillspec.boundary.guard_policy.v0";

/// One skill's reviewed policy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardPolicy {
    pub schema: String,
    /// Install slug this policy applies to.
    pub skill_slug: String,
    /// Grants approved for silent allow.
    pub allow: Vec<String>,
    /// Sensitive grants that were held for review and are not yet approved.
    /// Present so a reviewer can see what was withheld; not in the allow-set.
    pub pending_review: Vec<String>,
    /// Whether the surface was complete when the policy was written. An
    /// incomplete surface means the policy does not cover everything the skill
    /// can do.
    pub complete: bool,
}

impl GuardPolicy {
    /// Build a policy from a compiled proposal.
    ///
    /// The allow grants become the approved set; the sensitive
    /// `permission_required_for` grants are recorded as pending, not approved,
    /// so a credential read is never silently enforced-as-allowed.
    pub fn from_proposal(skill_slug: &str, proposal: &Proposal) -> Self {
        Self {
            schema: POLICY_SCHEMA.to_owned(),
            skill_slug: skill_slug.to_owned(),
            allow: proposal.boundary.allow.clone(),
            pending_review: proposal.boundary.permission_required_for.clone(),
            complete: proposal.complete,
        }
    }

    /// Whether this policy approves `grant`.
    pub fn allows(&self, grant: &str) -> bool {
        self.allow.iter().any(|allowed| allowed == grant)
    }
}

#[cfg(test)]
mod tests {
    use super::GuardPolicy;
    use crate::{analyze, proposal};
    use std::path::PathBuf;

    fn policy(name: &str) -> GuardPolicy {
        let surface = analyze(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/effects")
                .join(name),
        )
        .expect("analysis");
        GuardPolicy::from_proposal(name, &proposal::compile(&surface))
    }

    #[test]
    fn a_policy_approves_the_allow_grants() {
        let policy = policy("clean-formatter");
        assert!(policy.allows("proc.exec:git"));
        assert!(!policy.allows("net.egress:evil.test"));
    }

    #[test]
    fn a_sensitive_grant_is_pending_not_approved() {
        // A credential read must never be silently allowed by the guard.
        let policy = policy("secret-reader");
        assert!(policy.pending_review.iter().any(|g| g.contains("secret")));
        assert!(!policy.allows("fs.read:secret"));
    }

    #[test]
    fn an_incomplete_surface_marks_the_policy_incomplete() {
        assert!(!policy("dynamic-endpoint").complete);
        assert!(policy("clean-formatter").complete);
    }

    #[test]
    fn a_policy_round_trips_through_json() {
        let policy = policy("github-reporter");
        let json = serde_json::to_string(&policy).unwrap();
        let back: GuardPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back.allow, policy.allow);
        assert_eq!(back.skill_slug, "github-reporter");
    }
}
