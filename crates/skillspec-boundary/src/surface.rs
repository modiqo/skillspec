//! The effect-surface report.
//!
//! Resolved effects and unresolved ones are **sibling keys**, not variants of
//! one list. A consumer generating policy has to handle both, and separating
//! them means a consumer that ignores unresolved effects has a visible bug
//! rather than a silently permissive boundary.

use crate::bounds::SkippedFile;
use crate::dedupe::Effect;
use crate::effect::{EffectClass, PathClass, Reach};
use serde::Serialize;
use std::collections::BTreeMap;

/// Schema id for the serialized effect surface.
pub const EFFECT_SURFACE_SCHEMA: &str = "skillspec.boundary.effect_surface.v0";

/// Extractor version, recorded so a drift comparison can refuse to compare
/// surfaces produced by different analysis code. A new extractor finding a new
/// effect is not the skill changing.
pub const EXTRACTOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What was analyzed and under what limits.
#[derive(Clone, Debug, Serialize)]
pub struct AnalysisReport {
    pub extractor_version: String,
    pub extractors: Vec<String>,
    /// Always `lexical` in v0; AST extraction is a later milestone.
    pub extraction_mode: &'static str,
    pub files_analyzed: usize,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files_skipped: Vec<SkippedFile>,
}

/// Counts a reader scans before reading detail.
#[derive(Clone, Debug, Serialize)]
pub struct SurfaceSummary {
    pub effect_count: usize,
    pub unresolved_count: usize,
    pub by_class: BTreeMap<String, usize>,
    pub by_reach: BTreeMap<String, usize>,
    /// Sensitive path classes present anywhere in the surface.
    pub sensitive_path_classes: Vec<String>,
}

/// Everything enumerated from one package.
#[derive(Clone, Debug, Serialize)]
pub struct EffectSurface {
    pub schema: &'static str,
    pub target: String,
    /// `local` or `remote_github`.
    pub source_kind: String,
    /// Repository a remote target was staged from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_from: Option<String>,
    pub skill_path: String,
    pub analysis: AnalysisReport,
    /// Text a reader will not see but a model will. Not effects; no grants.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub concealment: Vec<crate::concealment::Concealment>,
    /// Instructions that retarget the agent's behavior. Not effects; no grants.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub directives: Vec<crate::directive::Directive>,
    /// Effects that can become grants.
    pub effects: Vec<Effect>,
    /// Effects whose target could not be determined.
    pub unresolved: Vec<Effect>,
    pub summary: SurfaceSummary,
}

impl EffectSurface {
    /// Build a surface from merged effects.
    pub fn new(
        target: String,
        skill_path: String,
        analysis: AnalysisReport,
        effects: Vec<Effect>,
    ) -> Self {
        let (effects, unresolved): (Vec<_>, Vec<_>) =
            effects.into_iter().partition(Effect::is_grantable);
        let summary = summarize(&effects, &unresolved);
        Self {
            schema: EFFECT_SURFACE_SCHEMA,
            target,
            source_kind: "local".to_owned(),
            staged_from: None,
            skill_path,
            analysis,
            concealment: Vec::new(),
            directives: Vec::new(),
            effects,
            unresolved,
            summary,
        }
    }

    /// Every effect, resolved and unresolved, in report order.
    pub fn all(&self) -> impl Iterator<Item = &Effect> {
        self.effects.iter().chain(self.unresolved.iter())
    }

    /// Whether the skill has a capability a directive could abuse: it reaches
    /// the network or reads a sensitive path.
    ///
    /// A behavior directive is only meaningful next to a capability. "Do not
    /// mention this" is benign in a skill that touches nothing and an attack in
    /// one that reads credentials and sends data, and the difference is exactly
    /// this predicate.
    pub fn has_capability(&self) -> bool {
        !self.summary.sensitive_path_classes.is_empty()
            || self.all().any(|effect| {
                matches!(effect.class, EffectClass::NetEgress | EffectClass::NetFetch)
            })
    }

    /// Directives worth raising an alarm about.
    ///
    /// A strong family - an injection, a self-disclosure, an unfounded
    /// authorization, a refusal override - is concerning wherever it appears. A
    /// contextual family - secrecy, confirmation bypass, activation overbreadth -
    /// is concerning only when the skill has a capability it could abuse, or when
    /// the instruction itself names a sensitive subject. Reach alone is not the
    /// trigger, because a referenced style guide legitimately says "do not
    /// report a convention as a failure"; a credential-secrecy instruction does
    /// not. Every directive is still reported - this is only the subset that
    /// flags a skill and gates a check.
    pub fn concerning_directives(&self) -> impl Iterator<Item = &crate::directive::Directive> {
        let has_capability = self.has_capability();
        self.directives.iter().filter(move |directive| {
            directive.is_strong() || has_capability || directive.mentions_sensitive_subject()
        })
    }

    /// Whether the surface was fully determined.
    ///
    /// An unresolved effect and a truncated analysis both mean the same thing
    /// for a downstream proposal: the package was not fully read, so the
    /// boundary derived from it is incomplete.
    pub fn is_complete(&self) -> bool {
        self.unresolved.is_empty() && !self.analysis.truncated
    }

    /// Effects whose path class needs a human decision.
    pub fn sensitive(&self) -> impl Iterator<Item = &Effect> {
        self.effects
            .iter()
            .chain(self.unresolved.iter())
            .filter(|effect| {
                effect
                    .target
                    .path_class()
                    .is_some_and(PathClass::is_sensitive)
            })
    }
}

fn summarize(effects: &[Effect], unresolved: &[Effect]) -> SurfaceSummary {
    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_reach: BTreeMap<String, usize> = BTreeMap::new();
    let mut sensitive: Vec<String> = Vec::new();

    for effect in effects.iter().chain(unresolved.iter()) {
        *by_class
            .entry(effect.class.as_str().to_owned())
            .or_default() += 1;
        *by_reach
            .entry(effect.reach.as_str().to_owned())
            .or_default() += 1;
        if let Some(class) = effect.target.path_class() {
            if class.is_sensitive() && !sensitive.iter().any(|seen| seen == class.as_str()) {
                sensitive.push(class.as_str().to_owned());
            }
        }
    }
    sensitive.sort();

    SurfaceSummary {
        effect_count: effects.len(),
        unresolved_count: unresolved.len(),
        by_class,
        by_reach,
        sensitive_path_classes: sensitive,
    }
}

/// Count of effects in a class, for callers that want one number.
pub fn count_of(surface: &EffectSurface, class: EffectClass) -> usize {
    surface
        .effects
        .iter()
        .chain(surface.unresolved.iter())
        .filter(|effect| effect.class == class)
        .count()
}

/// Whether any effect was found only in a file nothing references.
pub fn has_unmapped_effects(surface: &EffectSurface) -> bool {
    surface
        .effects
        .iter()
        .chain(surface.unresolved.iter())
        .any(|effect| effect.reach == Reach::Unmapped)
}
