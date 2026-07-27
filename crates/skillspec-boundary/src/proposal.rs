//! Compiling an effect surface into a least-privilege boundary proposal.
//!
//! The rules below interact, so their order is normative. A later rule may add
//! to or annotate the output of an earlier one; it may never widen a grant an
//! earlier rule withheld.
//!
//! ```text
//! 1 Default deny              structural, applies to every proposal
//! 2 Resolved -> grant         the base grant set
//! 3 Sensitive -> review       narrows rule 2; no later rule overrides it
//! 4 Unresolved -> incomplete  annotates, never narrows or widens
//! ```
//!
//! Rules 5 through 7 from `docs/design/security/37-boundary-proposal-compiler.md`
//! (declared-but-unused, observed-but-undeclared, and diffing against an
//! existing contract) need declarations this crate does not yet read. They are
//! represented in the schema as an absent `comparison` block rather than as a
//! silently empty one, so their absence is visible.
//!
//! A proposal is a policy artifact for something else to enforce. SkillSpec
//! does not intercept a call, does not sandbox a process, and does not
//! guarantee that an emitted policy is honored.

use crate::dedupe::Effect;
use crate::effect::{EffectClass, PathClass};
use crate::surface::EffectSurface;
use serde::Serialize;
use std::collections::BTreeMap;

/// Schema id for the serialized proposal.
pub const PROPOSAL_SCHEMA: &str = "skillspec.boundary.proposal.v0";

/// The default disposition for anything not granted.
///
/// Always [`BoundaryDefault::Deny`]. This is what makes an under-enumerated
/// surface fail closed, and the whole design rests on it, so it is a fixed
/// value rather than an option.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryDefault {
    Deny,
}

/// One permission label, with the effects that made it necessary.
#[derive(Clone, Debug, Serialize)]
pub struct Grant {
    pub label: String,
    pub class: EffectClass,
    pub token: String,
    /// Ids of the effects this grant covers.
    pub effects: Vec<String>,
}

/// Why a grant needs a human decision rather than being emitted as an allow.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewReason {
    /// The path class is one where a granted effect would be
    /// indistinguishable from the attack.
    SensitivePathClass,
}

/// A grant held back for review.
#[derive(Clone, Debug, Serialize)]
pub struct ReviewItem {
    pub grant: String,
    pub reason: ReviewReason,
    /// Literal paths behind the class, so a reader sees what is at stake.
    pub patterns: Vec<String>,
    pub effects: Vec<String>,
}

/// An effect that could not become a grant, and what that means at runtime.
#[derive(Clone, Debug, Serialize)]
pub struct UnresolvedItem {
    pub effect: String,
    pub class: EffectClass,
    pub reason: String,
    /// What happens if the reader applies the proposal as emitted.
    pub consequence: String,
}

/// The permission set itself.
#[derive(Clone, Debug, Serialize)]
pub struct Boundary {
    pub default: BoundaryDefault,
    pub allow: Vec<String>,
    pub permission_required_for: Vec<String>,
    pub forbid: Vec<String>,
}

/// Result of the internal consistency check (validation level V0).
///
/// Failures here are defects in this crate, not findings about the target.
#[derive(Clone, Debug, Serialize)]
pub struct CoverageReport {
    pub effects_checked: usize,
    pub violations: Vec<String>,
}

impl CoverageReport {
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

/// A least-privilege boundary derived from one effect surface.
#[derive(Clone, Debug, Serialize)]
pub struct Proposal {
    pub schema: &'static str,
    pub target: String,
    pub effect_surface: &'static str,
    /// False when the surface was not fully determined.
    pub complete: bool,
    pub boundary: Boundary,
    pub grants: Vec<Grant>,
    pub review_required: Vec<ReviewItem>,
    pub unresolved: Vec<UnresolvedItem>,
    pub coverage: CoverageReport,
}

/// Compile a boundary proposal from an effect surface.
pub fn compile(surface: &EffectSurface) -> Proposal {
    let mut allow: BTreeMap<String, Grant> = BTreeMap::new();
    let mut review: BTreeMap<String, ReviewItem> = BTreeMap::new();

    // Rule 2, narrowed by rule 3.
    for effect in &surface.effects {
        let label = effect.grant_label();
        if sensitive(effect) {
            let item = review.entry(label.clone()).or_insert_with(|| ReviewItem {
                grant: label.clone(),
                reason: ReviewReason::SensitivePathClass,
                patterns: Vec::new(),
                effects: Vec::new(),
            });
            let pattern = pattern_of(effect);
            if !item.patterns.contains(&pattern) {
                item.patterns.push(pattern);
            }
            item.effects.push(effect.id.clone());
            continue;
        }
        let grant = allow.entry(label.clone()).or_insert_with(|| Grant {
            label: label.clone(),
            class: effect.class,
            token: effect.target.grant_token(),
            effects: Vec::new(),
        });
        grant.effects.push(effect.id.clone());
    }

    // Rule 4.
    let unresolved = surface
        .unresolved
        .iter()
        .map(|effect| UnresolvedItem {
            effect: effect.id.clone(),
            class: effect.class,
            reason: unresolved_reason(effect),
            consequence: consequence_of(effect.class),
        })
        .collect::<Vec<_>>();

    let grants = allow.into_values().collect::<Vec<_>>();
    let review_required = review.into_values().collect::<Vec<_>>();

    // Rule 1.
    let boundary = Boundary {
        default: BoundaryDefault::Deny,
        allow: grants.iter().map(|grant| grant.label.clone()).collect(),
        permission_required_for: review_required
            .iter()
            .map(|item| item.grant.clone())
            .collect(),
        forbid: Vec::new(),
    };

    let coverage = check_coverage(surface, &boundary, &grants, &review_required, &unresolved);

    Proposal {
        schema: PROPOSAL_SCHEMA,
        target: surface.target.clone(),
        effect_surface: crate::surface::EFFECT_SURFACE_SCHEMA,
        complete: surface.is_complete(),
        boundary,
        grants,
        review_required,
        unresolved,
        coverage,
    }
}

/// Rule 3: the path classes that never become a silent allow grant.
fn sensitive(effect: &Effect) -> bool {
    effect
        .target
        .path_class()
        .is_some_and(PathClass::is_sensitive)
}

fn pattern_of(effect: &Effect) -> String {
    match &effect.target {
        crate::effect::EffectTarget::Path { pattern, .. } => pattern.clone(),
        other => other.grant_token(),
    }
}

fn unresolved_reason(effect: &Effect) -> String {
    match effect.resolution {
        crate::effect::TargetResolution::Dynamic => {
            "target is computed at runtime and is not in the package".to_owned()
        }
        _ => "an effect is present but no target could be attributed".to_owned(),
    }
}

/// What the reader should expect at runtime. Required on every unresolved item:
/// the report states the outcome plainly and lets the reader decide whether it
/// is the desired one or a break.
fn consequence_of(class: EffectClass) -> String {
    match class {
        EffectClass::NetEgress | EffectClass::NetFetch => {
            "under a deny-default boundary this request will be refused".to_owned()
        }
        EffectClass::ProcExec => {
            "under a deny-default boundary this execution will be refused".to_owned()
        }
        EffectClass::FsRead | EffectClass::FsWrite | EffectClass::AgentConfig => {
            "under a deny-default boundary this file access will be refused".to_owned()
        }
        _ => "under a deny-default boundary this call will be refused".to_owned(),
    }
}

/// Validation level V0: is the proposal internally consistent with the surface
/// it came from?
fn check_coverage(
    surface: &EffectSurface,
    boundary: &Boundary,
    grants: &[Grant],
    review: &[ReviewItem],
    unresolved: &[UnresolvedItem],
) -> CoverageReport {
    let mut violations = Vec::new();

    // Every effect accounted for exactly once.
    let mut covered: BTreeMap<&str, usize> = BTreeMap::new();
    for id in grants
        .iter()
        .flat_map(|grant| grant.effects.iter())
        .chain(review.iter().flat_map(|item| item.effects.iter()))
        .chain(unresolved.iter().map(|item| &item.effect))
    {
        *covered.entry(id.as_str()).or_default() += 1;
    }
    for effect in surface.all() {
        match covered.get(effect.id.as_str()) {
            None => violations.push(format!(
                "{} ({}) is in the surface but not in the proposal",
                effect.id,
                effect.grant_label()
            )),
            Some(count) if *count > 1 => violations.push(format!(
                "{} is accounted for {count} times; rule precedence is not being applied",
                effect.id
            )),
            Some(_) => {}
        }
    }

    // A grant with no contributing effect is a grant nobody asked for.
    for grant in grants {
        if grant.effects.is_empty() {
            violations.push(format!("{} has no contributing effect", grant.label));
        }
    }

    // Host grants must never widen to a parent domain.
    let hosts = grants
        .iter()
        .filter(|grant| matches!(grant.class, EffectClass::NetEgress | EffectClass::NetFetch))
        .map(|grant| grant.token.as_str())
        .collect::<Vec<_>>();
    for host in &hosts {
        if host.starts_with('*') {
            violations.push(format!("{host} is a wildcard host grant"));
        }
        for other in &hosts {
            if host != other && other.ends_with(&format!(".{host}")) {
                violations.push(format!("{host} is a parent domain of {other}"));
            }
        }
    }

    if boundary.default != BoundaryDefault::Deny {
        violations.push("boundary default is not deny".to_owned());
    }
    if !unresolved.is_empty() && surface.is_complete() {
        violations.push(
            "unresolved effects are present but the surface claims to be complete".to_owned(),
        );
    }

    CoverageReport {
        effects_checked: surface.all().count(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::{compile, BoundaryDefault};
    use crate::analyze;
    use std::path::PathBuf;

    fn fixture(name: &str) -> super::Proposal {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/effects")
            .join(name);
        compile(&analyze(&path).expect("analysis"))
    }

    #[test]
    fn every_proposal_defaults_to_deny() {
        // The property the whole design rests on, so it is not configurable.
        for name in ["clean-formatter", "direct-chain", "dynamic-endpoint"] {
            assert_eq!(
                fixture(name).boundary.default,
                BoundaryDefault::Deny,
                "{name}"
            );
        }
    }

    #[test]
    fn internal_coverage_passes_on_every_fixture() {
        // A failure here is a defect in the compiler, not a finding about the
        // skill: it means an effect fell out of the proposal entirely.
        for name in [
            "clean-formatter",
            "github-reporter",
            "dynamic-endpoint",
            "secret-reader",
            "unmapped-payload",
            "wrapped-exec",
            "direct-chain",
        ] {
            let proposal = fixture(name);
            assert!(
                proposal.coverage.passed(),
                "{name}: {:?}",
                proposal.coverage.violations
            );
            assert!(proposal.coverage.effects_checked > 0, "{name}");
        }
    }

    #[test]
    fn a_clean_skill_yields_a_short_allow_list_and_nothing_to_review() {
        let proposal = fixture("clean-formatter");
        assert_eq!(proposal.boundary.allow, ["proc.exec:git"]);
        assert!(proposal.boundary.permission_required_for.is_empty());
        assert!(proposal.complete);
    }

    #[test]
    fn a_sensitive_path_never_lands_in_the_allow_list() {
        // Rule 3. A skill that legitimately reads credentials exists; the
        // proposal's job is to make sure a human said so out loud.
        let proposal = fixture("secret-reader");
        assert!(!proposal
            .boundary
            .allow
            .iter()
            .any(|grant| grant.contains("secret")));
        assert!(proposal
            .boundary
            .permission_required_for
            .contains(&"fs.read:secret".to_owned()));
    }

    #[test]
    fn review_items_carry_the_literal_paths_at_stake() {
        let proposal = fixture("direct-chain");
        let item = proposal
            .review_required
            .iter()
            .find(|item| item.grant == "fs.read:secret")
            .expect("secret review item");
        assert!(item.patterns.contains(&"~/.aws/credentials".to_owned()));
        assert!(!item.effects.is_empty());
    }

    #[test]
    fn an_unresolved_effect_makes_the_proposal_incomplete_and_states_the_outcome() {
        // Rule 4. The proposal is still emitted; what must not happen is
        // emitting it as though it were complete.
        let proposal = fixture("dynamic-endpoint");
        assert!(!proposal.complete);
        let item = proposal
            .unresolved
            .iter()
            .find(|item| item.class == crate::effect::EffectClass::NetEgress)
            .expect("unresolved egress");
        assert!(item.consequence.contains("refused"));
        assert!(!item.reason.is_empty());
    }

    #[test]
    fn an_unresolved_effect_never_becomes_a_grant() {
        let proposal = fixture("dynamic-endpoint");
        for item in &proposal.unresolved {
            assert!(!proposal
                .grants
                .iter()
                .any(|grant| grant.effects.contains(&item.effect)));
        }
    }

    #[test]
    fn grants_are_deduplicated_and_keep_their_contributing_effects() {
        let proposal = fixture("github-reporter");
        let labels = proposal
            .boundary
            .allow
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(labels.len(), proposal.boundary.allow.len());
        assert!(proposal
            .grants
            .iter()
            .all(|grant| !grant.effects.is_empty()));
    }

    #[test]
    fn host_grants_stay_exact() {
        let proposal = fixture("github-reporter");
        assert!(proposal
            .boundary
            .allow
            .contains(&"net.fetch:api.github.com".to_owned()));
        assert!(!proposal
            .boundary
            .allow
            .iter()
            .any(|grant| grant.contains('*')));
    }
}
