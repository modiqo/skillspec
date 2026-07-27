//! Collapsing observations into the effect set a boundary is written against.
//!
//! Several sightings of the same action merge into one [`Effect`] that keeps
//! every piece of evidence. Merging is where two invariants have to hold:
//! reach keeps the *most visible* sighting, and confidence keeps the strongest.

use crate::effect::{
    Confidence, EffectClass, EffectEvidence, EffectObservation, EffectOrigin, EffectTarget, Reach,
    TargetResolution,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// One deduplicated effect, with all the evidence that produced it.
#[derive(Clone, Debug, Serialize)]
pub struct Effect {
    /// Stable within one report.
    pub id: String,
    pub class: EffectClass,
    pub target: EffectTarget,
    pub resolution: TargetResolution,
    pub reach: Reach,
    pub origin: EffectOrigin,
    pub confidence: Confidence,
    pub observations: Vec<EffectEvidence>,
}

impl Effect {
    /// The grant label this effect would need.
    pub fn grant_label(&self) -> String {
        format!("{}:{}", self.class, self.target.grant_token())
    }

    /// Whether this effect can become a grant at all.
    pub fn is_grantable(&self) -> bool {
        self.resolution.is_grantable()
    }
}

/// Merge observations into effects, ordered deterministically.
///
/// Ordering is by class, then target, then resolution, which is the natural
/// order of the merge key. Determinism matters beyond tidiness: drift
/// comparison comes down to diffing two of these lists.
pub fn merge(observations: Vec<EffectObservation>) -> Vec<Effect> {
    let mut merged: BTreeMap<(EffectClass, EffectTarget, TargetResolution), Effect> =
        BTreeMap::new();

    for observation in observations {
        let key = (
            observation.class,
            observation.target.clone(),
            observation.resolution,
        );
        match merged.get_mut(&key) {
            Some(effect) => {
                effect.reach = effect.reach.most_visible(observation.reach);
                effect.confidence = effect.confidence.strongest(observation.confidence);
                if !effect.observations.contains(&observation.evidence) {
                    effect.observations.push(observation.evidence);
                }
            }
            None => {
                merged.insert(
                    key,
                    Effect {
                        // Filled in below, once the order is final.
                        id: String::new(),
                        class: observation.class,
                        target: observation.target,
                        resolution: observation.resolution,
                        reach: observation.reach,
                        origin: observation.origin,
                        confidence: observation.confidence,
                        observations: vec![observation.evidence],
                    },
                );
            }
        }
    }

    merged
        .into_values()
        .enumerate()
        .map(|(index, mut effect)| {
            effect.id = format!("effect-{:04}", index + 1);
            effect.observations.sort();
            effect
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::merge;
    use crate::effect::{
        Confidence, EffectClass, EffectEvidence, EffectObservation, EffectOrigin, EffectTarget,
        PathClass, Reach, TargetResolution,
    };

    fn observation(
        class: EffectClass,
        target: EffectTarget,
        reach: Reach,
        confidence: Confidence,
        line: usize,
    ) -> EffectObservation {
        EffectObservation {
            class,
            target,
            resolution: TargetResolution::Literal,
            origin: EffectOrigin::ScriptFile,
            reach,
            confidence,
            evidence: EffectEvidence::new("scripts/run.sh", Some(line), "curl https://x.test"),
        }
    }

    fn host(name: &str) -> EffectTarget {
        EffectTarget::Host {
            host: name.to_owned(),
            scheme: Some("https".to_owned()),
            port: None,
            ip_literal: false,
            non_ascii: false,
        }
    }

    #[test]
    fn identical_observations_collapse_and_keep_every_evidence_record() {
        let effects = merge(vec![
            observation(
                EffectClass::NetEgress,
                host("a.test"),
                Reach::Deferred,
                Confidence::High,
                1,
            ),
            observation(
                EffectClass::NetEgress,
                host("a.test"),
                Reach::Deferred,
                Confidence::High,
                9,
            ),
        ]);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].observations.len(), 2);
    }

    #[test]
    fn duplicate_evidence_is_not_repeated() {
        let effects = merge(vec![
            observation(
                EffectClass::NetEgress,
                host("a.test"),
                Reach::Deferred,
                Confidence::High,
                1,
            ),
            observation(
                EffectClass::NetEgress,
                host("a.test"),
                Reach::Deferred,
                Confidence::High,
                1,
            ),
        ]);
        assert_eq!(effects[0].observations.len(), 1);
    }

    #[test]
    fn merging_keeps_the_most_visible_reach() {
        // The regression this guards: an effect visible in the documented body
        // must not be filed as though it lived only in an unmapped script.
        let effects = merge(vec![
            observation(
                EffectClass::NetEgress,
                host("a.test"),
                Reach::Unmapped,
                Confidence::High,
                1,
            ),
            observation(
                EffectClass::NetEgress,
                host("a.test"),
                Reach::Activation,
                Confidence::High,
                2,
            ),
        ]);
        assert_eq!(effects[0].reach, Reach::Activation);
    }

    #[test]
    fn merging_keeps_the_strongest_confidence() {
        let effects = merge(vec![
            observation(
                EffectClass::NetFetch,
                host("a.test"),
                Reach::Deferred,
                Confidence::Low,
                1,
            ),
            observation(
                EffectClass::NetFetch,
                host("a.test"),
                Reach::Deferred,
                Confidence::High,
                2,
            ),
        ]);
        assert_eq!(effects[0].confidence, Confidence::High);
    }

    #[test]
    fn different_hosts_stay_separate() {
        let effects = merge(vec![
            observation(
                EffectClass::NetEgress,
                host("a.test"),
                Reach::Deferred,
                Confidence::High,
                1,
            ),
            observation(
                EffectClass::NetEgress,
                host("b.test"),
                Reach::Deferred,
                Confidence::High,
                2,
            ),
        ]);
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn the_same_target_at_different_resolutions_stays_separate() {
        // A literal host and a dynamic one are different facts about the
        // package, and only one of them can become a grant.
        let mut dynamic = observation(
            EffectClass::NetEgress,
            host("a.test"),
            Reach::Deferred,
            Confidence::High,
            2,
        );
        dynamic.resolution = TargetResolution::Dynamic;
        let effects = merge(vec![
            observation(
                EffectClass::NetEgress,
                host("a.test"),
                Reach::Deferred,
                Confidence::High,
                1,
            ),
            dynamic,
        ]);
        assert_eq!(effects.len(), 2);
        assert_eq!(effects.iter().filter(|e| e.is_grantable()).count(), 1);
    }

    #[test]
    fn ids_are_assigned_in_stable_order() {
        let build = || {
            merge(vec![
                observation(
                    EffectClass::ProcExec,
                    EffectTarget::Binary {
                        name: "git".to_owned(),
                        privileged: false,
                    },
                    Reach::Deferred,
                    Confidence::High,
                    1,
                ),
                observation(
                    EffectClass::NetEgress,
                    host("a.test"),
                    Reach::Deferred,
                    Confidence::High,
                    2,
                ),
            ])
        };
        let first = build();
        let second = build();
        assert_eq!(first[0].id, "effect-0001");
        assert_eq!(
            first.iter().map(|e| e.grant_label()).collect::<Vec<_>>(),
            second.iter().map(|e| e.grant_label()).collect::<Vec<_>>()
        );
        // Class order is declaration order, so egress sorts before exec.
        assert_eq!(first[0].class, EffectClass::NetEgress);
    }

    #[test]
    fn grant_labels_compose_class_and_target() {
        let effects = merge(vec![observation(
            EffectClass::FsRead,
            EffectTarget::Path {
                pattern: "~/.aws/credentials".to_owned(),
                class: PathClass::Secret,
            },
            Reach::Deferred,
            Confidence::High,
            1,
        )]);
        assert_eq!(effects[0].grant_label(), "fs.read:secret");
    }

    #[test]
    fn merging_nothing_produces_nothing() {
        assert!(merge(Vec::new()).is_empty());
    }
}
