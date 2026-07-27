//! Comparing two effect surfaces across revisions of the same skill.
//!
//! This is the highest-precision signal in the design: it needs no judgment
//! about whether an effect is appropriate, only about whether it is *new*. A
//! skill hostile from its first commit shows no drift, so drift is the gate for
//! an update, never for a first install - that rule lives in the CLI, not here.
//!
//! Two surfaces are only comparable when they were produced by the same
//! extractor version. A newer extractor finding a new effect is not the skill
//! changing, and a diff across versions is reported as non-comparable rather
//! than as spurious growth.
//!
//! See `docs/design/security/39-concealment-and-effect-drift.md`.

use crate::dedupe::Effect;
use crate::effect::{EffectClass, PathClass, Reach};
use crate::surface::EffectSurface;
use serde::Serialize;
use std::collections::BTreeMap;

/// Schema id for the serialized drift report.
pub const DRIFT_SCHEMA: &str = "skillspec.boundary.drift.v0";

/// The kind of change a drift finding represents.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeClass {
    /// A new effect in a sensitive path class.
    SensitiveExpansion,
    /// A new network host.
    EgressExpansion,
    /// A new executed binary.
    ExecExpansion,
    /// A new environment read.
    EnvExpansion,
    /// A new effect whose target could not be resolved: something became less
    /// determinable between revisions.
    UnresolvedExpansion,
    /// An existing capability moved to a less visible reach - out of the
    /// documented body into an unreferenced file.
    ReachRegression,
    /// A capability present before and gone now. Never gated.
    Contraction,
}

impl ChangeClass {
    /// Whether a change of this class warrants a human looking again before the
    /// update is trusted.
    pub fn requires_review(self) -> bool {
        matches!(
            self,
            Self::SensitiveExpansion
                | Self::EgressExpansion
                | Self::UnresolvedExpansion
                | Self::ReachRegression
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SensitiveExpansion => "sensitive_expansion",
            Self::EgressExpansion => "egress_expansion",
            Self::ExecExpansion => "exec_expansion",
            Self::EnvExpansion => "env_expansion",
            Self::UnresolvedExpansion => "unresolved_expansion",
            Self::ReachRegression => "reach_regression",
            Self::Contraction => "contraction",
        }
    }
}

/// One change between the two revisions.
#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub class: ChangeClass,
    /// The capability grant label the change concerns.
    pub grant: String,
    pub detail: String,
    pub requires_review: bool,
}

/// One side of a comparison.
#[derive(Clone, Debug, Serialize)]
pub struct DriftSide {
    pub label: String,
    pub extractor_version: String,
}

/// The result of comparing two surfaces.
#[derive(Clone, Debug, Serialize)]
pub struct DriftReport {
    pub schema: &'static str,
    pub from: DriftSide,
    pub to: DriftSide,
    /// False when the two were produced by different extractor versions.
    pub comparable: bool,
    pub changes: Vec<Change>,
    pub summary: DriftSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct DriftSummary {
    pub requires_review: usize,
    pub informational: usize,
    pub contractions: usize,
}

impl DriftReport {
    /// Whether any change warrants review.
    pub fn requires_review(&self) -> bool {
        self.summary.requires_review > 0
    }
}

/// A capability's identity for comparison, plus how it presents.
struct Capability {
    reach: Reach,
    class: EffectClass,
    sensitive: bool,
}

/// Compare two effect surfaces produced from revisions of one skill.
///
/// `from` is the baseline (older) revision, `to` the candidate (newer) one.
pub fn diff(
    from: &EffectSurface,
    to: &EffectSurface,
    from_label: &str,
    to_label: &str,
) -> DriftReport {
    let comparable = from.analysis.extractor_version == to.analysis.extractor_version;

    let base = capabilities(from);
    let head = capabilities(to);
    let mut changes = Vec::new();

    if comparable {
        for (grant, cap) in &head {
            match base.get(grant) {
                None => changes.push(expansion(grant, cap)),
                Some(prior) => {
                    // A less visible reach is a regression: activation is the
                    // minimum under the ordering, so a larger value is less
                    // visible.
                    if cap.reach > prior.reach {
                        changes.push(Change {
                            class: ChangeClass::ReachRegression,
                            grant: grant.clone(),
                            detail: format!(
                                "moved from {} to {} reach",
                                prior.reach.as_str(),
                                cap.reach.as_str()
                            ),
                            requires_review: ChangeClass::ReachRegression.requires_review(),
                        });
                    }
                }
            }
        }
        for grant in base.keys() {
            if !head.contains_key(grant) {
                changes.push(Change {
                    class: ChangeClass::Contraction,
                    grant: grant.clone(),
                    detail: "present in the baseline, absent now".to_owned(),
                    requires_review: false,
                });
            }
        }
        changes.sort_by(|a, b| {
            b.requires_review
                .cmp(&a.requires_review)
                .then_with(|| a.grant.cmp(&b.grant))
        });
    }

    let summary = DriftSummary {
        requires_review: changes
            .iter()
            .filter(|change| change.requires_review)
            .count(),
        informational: changes
            .iter()
            .filter(|change| !change.requires_review && change.class != ChangeClass::Contraction)
            .count(),
        contractions: changes
            .iter()
            .filter(|change| change.class == ChangeClass::Contraction)
            .count(),
    };

    DriftReport {
        schema: DRIFT_SCHEMA,
        from: DriftSide {
            label: from_label.to_owned(),
            extractor_version: from.analysis.extractor_version.clone(),
        },
        to: DriftSide {
            label: to_label.to_owned(),
            extractor_version: to.analysis.extractor_version.clone(),
        },
        comparable,
        changes,
        summary,
    }
}

/// A capability keyed by grant label; the most visible reach wins when a
/// capability appears more than once.
fn capabilities(surface: &EffectSurface) -> BTreeMap<String, Capability> {
    let mut map: BTreeMap<String, Capability> = BTreeMap::new();
    for effect in surface.all() {
        let grant = effect.grant_label();
        let cap = Capability {
            reach: effect.reach,
            class: effect.class,
            sensitive: is_sensitive(effect),
        };
        map.entry(grant)
            .and_modify(|existing| {
                existing.reach = existing.reach.most_visible(cap.reach);
                existing.sensitive |= cap.sensitive;
            })
            .or_insert(cap);
    }
    map
}

fn expansion(grant: &str, cap: &Capability) -> Change {
    let class = if cap.sensitive {
        ChangeClass::SensitiveExpansion
    } else {
        match cap.class {
            EffectClass::NetEgress | EffectClass::NetFetch => ChangeClass::EgressExpansion,
            EffectClass::ProcExec => ChangeClass::ExecExpansion,
            EffectClass::EnvRead => ChangeClass::EnvExpansion,
            _ => ChangeClass::UnresolvedExpansion,
        }
    };
    // A new capability at unmapped reach is worth review whatever its class.
    let requires_review = class.requires_review() || cap.reach == Reach::Unmapped;
    Change {
        class,
        grant: grant.to_owned(),
        detail: format!("new capability ({} reach)", cap.reach.as_str()),
        requires_review,
    }
}

fn is_sensitive(effect: &Effect) -> bool {
    effect
        .target
        .path_class()
        .is_some_and(PathClass::is_sensitive)
        || matches!(
            &effect.target,
            crate::effect::EffectTarget::EnvVar {
                credential_like: true,
                ..
            }
        )
}

/// Render a drift report for a human or a CI log.
pub fn render(report: &DriftReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "SkillSpec Boundary Drift");
    let _ = writeln!(out, "========================");
    let _ = writeln!(out, "From: {}", report.from.label);
    let _ = writeln!(out, "To:   {}", report.to.label);
    let _ = writeln!(out);

    if !report.comparable {
        let _ = writeln!(
            out,
            "Not comparable: the two revisions were analyzed by different extractor\nversions ({} vs {}). A new extractor finding a new effect is not the skill\nchanging, so no diff is reported.",
            report.from.extractor_version, report.to.extractor_version
        );
        return out;
    }

    if report.changes.is_empty() {
        let _ = writeln!(out, "No change in the effect surface.");
        return out;
    }

    let review: Vec<_> = report
        .changes
        .iter()
        .filter(|change| change.requires_review)
        .collect();
    let informational: Vec<_> = report
        .changes
        .iter()
        .filter(|change| !change.requires_review && change.class != ChangeClass::Contraction)
        .collect();
    let contractions: Vec<_> = report
        .changes
        .iter()
        .filter(|change| change.class == ChangeClass::Contraction)
        .collect();

    if !review.is_empty() {
        let _ = writeln!(out, "Needs review");
        for change in &review {
            let _ = writeln!(
                out,
                "- {:<20} {}  ({})",
                change.class.as_str(),
                change.grant,
                change.detail
            );
        }
        let _ = writeln!(out);
    }
    if !informational.is_empty() {
        let _ = writeln!(out, "Also changed");
        for change in &informational {
            let _ = writeln!(out, "- {:<20} {}", change.class.as_str(), change.grant);
        }
        let _ = writeln!(out);
    }
    if !contractions.is_empty() {
        let _ = writeln!(
            out,
            "Removed ({} capabilities no longer present)",
            contractions.len()
        );
    }

    let _ = writeln!(out);
    if report.requires_review() {
        let _ = writeln!(
            out,
            "This update grew the effect surface in {} way(s) that warrant review\nbefore it is trusted.",
            report.summary.requires_review
        );
    } else {
        let _ = writeln!(out, "No change in this update warrants review.");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{diff, ChangeClass};
    use crate::{analyze, surface::EffectSurface};
    use std::path::PathBuf;

    fn fixture(name: &str) -> EffectSurface {
        analyze(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/effects")
                .join(name),
        )
        .expect("analysis")
    }

    #[test]
    fn a_surface_does_not_drift_from_itself() {
        let surface = fixture("github-reporter");
        let report = diff(&surface, &surface, "a", "b");
        assert!(report.comparable);
        assert!(report.changes.is_empty());
        assert!(!report.requires_review());
    }

    #[test]
    fn a_new_egress_host_is_an_expansion_that_needs_review() {
        let base = fixture("clean-formatter");
        let head = fixture("direct-chain");
        let report = diff(&base, &head, "base", "head");

        let egress = report
            .changes
            .iter()
            .find(|change| change.class == ChangeClass::EgressExpansion)
            .expect("an egress expansion");
        assert!(egress.requires_review);
        assert!(egress.grant.contains("archive.example.com"));
    }

    #[test]
    fn a_new_credential_read_is_a_sensitive_expansion() {
        let report = diff(
            &fixture("clean-formatter"),
            &fixture("py-exfil"),
            "base",
            "head",
        );
        assert!(report
            .changes
            .iter()
            .any(|change| change.class == ChangeClass::SensitiveExpansion));
        assert!(report.requires_review());
    }

    #[test]
    fn a_removed_capability_is_a_contraction_and_never_gated() {
        // The reverse direction: going from the rich surface to the clean one
        // only removes capabilities.
        let report = diff(
            &fixture("direct-chain"),
            &fixture("clean-formatter"),
            "base",
            "head",
        );
        assert!(report
            .changes
            .iter()
            .any(|change| change.class == ChangeClass::Contraction));
        assert!(!report.requires_review(), "contraction alone must not gate");
        assert!(report.summary.contractions > 0);
    }

    #[test]
    fn review_changes_sort_before_informational_ones() {
        let report = diff(
            &fixture("clean-formatter"),
            &fixture("unmapped-payload"),
            "base",
            "head",
        );
        let first_informational = report
            .changes
            .iter()
            .position(|change| !change.requires_review);
        let last_review = report
            .changes
            .iter()
            .rposition(|change| change.requires_review);
        if let (Some(review), Some(info)) = (last_review, first_informational) {
            assert!(review < info, "review changes must come first");
        }
    }

    #[test]
    fn surfaces_from_different_extractor_versions_are_not_comparable() {
        let base = fixture("clean-formatter");
        let mut head = fixture("direct-chain");
        head.analysis.extractor_version = "different".to_owned();
        let report = diff(&base, &head, "base", "head");
        assert!(!report.comparable);
        // No changes are asserted across incomparable versions.
        assert!(report.changes.is_empty());
    }
}
