//! Human rendering of an effect surface.
//!
//! Ordered for a reviewer deciding whether to install, not for a scanner
//! reading severities. The consequence paragraph leads because it is the
//! product; everything under it is evidence for it.
//!
//! Two lines are mandatory in every rendering and are not decoration: that the
//! analysis is static, and that SkillSpec does not enforce the boundary. They
//! are the accurate description of what the tool did.

use crate::dedupe::Effect;
use crate::effect::{EffectClass, PathClass, Reach};
use crate::surface::EffectSurface;
use std::collections::BTreeMap;
use std::fmt::Write;

/// Render the default text report.
///
/// Ordering follows document 41: concealment leads because it invalidates the
/// reliability of everything below it, then directives because they change how
/// the reader should read the effects, then the effect surface itself.
pub fn render(surface: &EffectSurface) -> String {
    let mut out = String::new();
    header(surface, &mut out);
    concealment(surface, &mut out);
    directives(surface, &mut out);
    consequence(surface, &mut out);
    unresolved(surface, &mut out);
    effects(surface, &mut out);
    skipped(surface, &mut out);
    footer(surface, &mut out);
    out
}

fn concealment(surface: &EffectSurface, out: &mut String) {
    if surface.concealment.is_empty() {
        return;
    }
    let _ = writeln!(out, "Concealment");
    for finding in &surface.concealment {
        let _ = writeln!(
            out,
            "- {}  {}:{}",
            finding.id.as_str(),
            finding.path,
            finding.line
        );
        let _ = writeln!(out, "  {}", finding.statement);
        if let (Some(sha), Some(bytes)) = (&finding.decoded_sha256, finding.decoded_bytes) {
            let short = sha.get(..12).unwrap_or(sha);
            let _ = writeln!(
                out,
                "  decoded: sha256 {short}…  {bytes} bytes  (use --reveal to write it to a file)"
            );
        }
    }
    let _ = writeln!(out);
}

fn directives(surface: &EffectSurface, out: &mut String) {
    if surface.directives.is_empty() {
        return;
    }
    let _ = writeln!(out, "Directives");
    for finding in &surface.directives {
        let _ = writeln!(
            out,
            "- {}  {}:{}",
            finding.kind_id, finding.path, finding.line
        );
        let _ = writeln!(out, "  \"{}\"", finding.text);
    }
    let _ = writeln!(out);
}

fn header(surface: &EffectSurface, out: &mut String) {
    let _ = writeln!(out, "SkillSpec Boundary");
    let _ = writeln!(out, "==================");
    let _ = writeln!(
        out,
        "Target: {}        Effects: {}{}",
        surface.target,
        surface.summary.effect_count,
        match surface.summary.unresolved_count {
            0 => String::new(),
            count => format!(" ({count} unresolved)"),
        }
    );
    if let Some(repo) = &surface.staged_from {
        let _ = writeln!(out, "Staged from: {repo}");
    }
    let _ = writeln!(out);
}

/// The plain-English statement of what the package can reach.
///
/// Built only from facts already in the surface. It never says a skill is
/// malicious, and it never says one is safe.
fn consequence(surface: &EffectSurface, out: &mut String) {
    let mut clauses = Vec::new();

    // Sensitive path reads and credential env reads are the same idea to a
    // reader - "this touches something it should not" - so they share one
    // clause. A credential env read has no path class and would otherwise miss
    // the headline entirely, though it is the highest-signal half of an exfil.
    let mut reads = sensitive_reads(surface);
    reads.extend(credential_reads(surface));
    if !reads.is_empty() {
        clauses.push(format!("read {}", join(&reads, 3)));
    }
    let egress = hosts(surface, EffectClass::NetEgress);
    if !egress.is_empty() {
        clauses.push(format!("send data to {}", join(&egress, 2)));
    }
    let installs = hosts(surface, EffectClass::PkgInstall);
    if !installs.is_empty() && clauses.is_empty() {
        clauses.push(format!("install {}", join(&installs, 2)));
    }

    if clauses.is_empty() {
        let _ = writeln!(
            out,
            "No network egress and no sensitive path reads were found. See the effect\nlist below for what this skill does touch."
        );
    } else {
        let _ = writeln!(
            out,
            "If executed, this skill can {}.",
            clauses.join(" and ")
        );
    }

    if let Some(note) = unmapped_note(surface) {
        let _ = writeln!(out, "{note}");
    }
    let _ = writeln!(out);
}

/// Call out effects that live only where documentation does not reach.
fn unmapped_note(surface: &EffectSurface) -> Option<String> {
    let hidden = surface
        .all()
        .filter(|effect| effect.reach == Reach::Unmapped)
        .filter(|effect| notable(effect))
        .collect::<Vec<_>>();
    let path = hidden.first()?.observations.first()?.path.clone();
    Some(format!(
        "Some of that is in {path}, which no part of {} references - following\nthis skill's documentation would not show it to you.",
        surface.skill_path
    ))
}

fn unresolved(surface: &EffectSurface, out: &mut String) {
    if surface.unresolved.is_empty() {
        return;
    }
    let _ = writeln!(out, "Unresolved");
    for effect in &surface.unresolved {
        let evidence = effect.observations.first();
        let _ = writeln!(
            out,
            "- {:<12} target not determinable at analysis time    {}",
            effect.class.as_str(),
            evidence
                .map(|evidence| location(&evidence.path, evidence.line))
                .unwrap_or_default()
        );
        if let Some(evidence) = evidence {
            let _ = writeln!(out, "  {}", evidence.text_preview);
        }
    }
    let _ = writeln!(
        out,
        "  Under a deny-default policy these calls will be refused."
    );
    let _ = writeln!(out);
}

fn effects(surface: &EffectSurface, out: &mut String) {
    if surface.effects.is_empty() {
        return;
    }
    let _ = writeln!(out, "Effects");

    let mut sensitive_rows = Vec::new();
    let mut grouped: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for effect in &surface.effects {
        if effect
            .target
            .path_class()
            .is_some_and(PathClass::is_sensitive)
        {
            let evidence = effect.observations.first();
            sensitive_rows.push(format!(
                "- {:<24} {:<22} {}",
                effect.grant_label(),
                pattern_of(effect),
                evidence
                    .map(|evidence| location(&evidence.path, evidence.line))
                    .unwrap_or_default()
            ));
            continue;
        }
        let token = match effect.reach {
            Reach::Unmapped => format!("{} (unmapped)", effect.target.grant_token()),
            _ => effect.target.grant_token(),
        };
        let bucket = grouped.entry(effect.class.as_str()).or_default();
        if !bucket.contains(&token) {
            bucket.push(token);
        }
    }

    if !sensitive_rows.is_empty() {
        let _ = writeln!(out, "  Needs a decision before this is granted:");
        for row in &sensitive_rows {
            let _ = writeln!(out, "  {row}");
        }
        let _ = writeln!(out);
    }
    for (class, mut tokens) in grouped {
        tokens.sort();
        let _ = writeln!(out, "  {:<14} {}", class, tokens.join(", "));
    }
    let _ = writeln!(out);
}

fn skipped(surface: &EffectSurface, out: &mut String) {
    if surface.analysis.files_skipped.is_empty() {
        return;
    }
    let _ = writeln!(out, "Not analyzed");
    for file in &surface.analysis.files_skipped {
        let _ = writeln!(out, "- {} ({})", file.path, file.reason.as_str());
    }
    let _ = writeln!(out);
}

fn footer(surface: &EffectSurface, out: &mut String) {
    if !surface.is_complete() {
        let _ = writeln!(
            out,
            "This surface is incomplete. A boundary derived from it will not cover\neverything this skill can do."
        );
    }
    let _ = writeln!(
        out,
        "Static analysis: nothing in this package was executed. SkillSpec does not\nenforce a boundary; a harness, hook, or network policy does."
    );
}

fn notable(effect: &Effect) -> bool {
    matches!(effect.class, EffectClass::NetEgress | EffectClass::NetFetch)
        || effect
            .target
            .path_class()
            .is_some_and(PathClass::is_sensitive)
}

fn sensitive_reads(surface: &EffectSurface) -> Vec<String> {
    let mut patterns = surface
        .all()
        .filter(|effect| {
            matches!(effect.class, EffectClass::FsRead | EffectClass::EnvRead)
                && effect
                    .target
                    .path_class()
                    .is_some_and(PathClass::is_sensitive)
        })
        .map(pattern_of)
        .collect::<Vec<_>>();
    patterns.sort();
    patterns.dedup();
    patterns
}

fn credential_reads(surface: &EffectSurface) -> Vec<String> {
    let mut names = surface
        .all()
        .filter(|effect| effect.class == EffectClass::EnvRead)
        .filter_map(|effect| match &effect.target {
            crate::effect::EffectTarget::EnvVar {
                name,
                credential_like: true,
            } => Some(name.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn hosts(surface: &EffectSurface, class: EffectClass) -> Vec<String> {
    let mut tokens = surface
        .all()
        .filter(|effect| effect.class == class)
        .map(|effect| effect.target.grant_token())
        .collect::<Vec<_>>();
    tokens.sort();
    tokens.dedup();
    tokens
}

fn pattern_of(effect: &Effect) -> String {
    match &effect.target {
        crate::effect::EffectTarget::Path { pattern, .. } => pattern.clone(),
        other => other.grant_token(),
    }
}

fn location(path: &str, line: Option<usize>) -> String {
    match line {
        Some(line) => format!("{path}:{line}"),
        None => path.to_owned(),
    }
}

fn join(items: &[String], limit: usize) -> String {
    let shown = items.iter().take(limit).cloned().collect::<Vec<_>>();
    let rest = items.len().saturating_sub(shown.len());
    let joined = match shown.len() {
        0 => String::new(),
        1 => shown[0].clone(),
        _ => format!(
            "{} and {}",
            shown[..shown.len() - 1].join(", "),
            shown[shown.len() - 1]
        ),
    };
    if rest > 0 {
        format!("{joined} (+{rest} more)")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::{join, render};
    use crate::analyze;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/effects")
            .join(name);
        render(&analyze(&path).expect("analysis"))
    }

    #[test]
    fn every_rendering_states_the_analysis_is_static_and_unenforced() {
        // Not decoration: it is the accurate description of what ran.
        for name in ["clean-formatter", "unmapped-payload"] {
            let text = fixture(name);
            assert!(
                text.contains("nothing in this package was executed"),
                "{name}"
            );
            assert!(text.contains("SkillSpec does not"), "{name}");
        }
    }

    #[test]
    fn a_clean_skill_says_so_without_claiming_safety() {
        let text = fixture("clean-formatter");
        assert!(text.contains("No network egress and no sensitive path reads"));
        assert!(!text.to_lowercase().contains("safe"));
        assert!(!text.to_lowercase().contains("malicious"));
    }

    #[test]
    fn the_consequence_line_names_what_can_be_reached() {
        let text = fixture("direct-chain");
        assert!(text.contains("If executed, this skill can"));
        assert!(text.contains("~/.aws/credentials"));
        assert!(text.contains("archive.example.com"));
    }

    #[test]
    fn a_credential_env_read_reaches_the_headline() {
        // The highest-signal half of an exfiltration must not be buried in the
        // effect list; it belongs in the consequence sentence.
        let text = fixture("exfil-env-token");
        assert!(text.contains("AWS_SECRET_ACCESS_KEY"));
        assert!(text.contains("send data to telemetry.example.net"));
    }

    #[test]
    fn effects_only_reachable_through_an_unreferenced_file_are_called_out() {
        let text = fixture("unmapped-payload");
        assert!(text.contains("scripts/collect.sh"));
        assert!(text.contains("would not show it to you"));
    }

    #[test]
    fn an_unresolved_target_is_reported_with_its_consequence() {
        let text = fixture("dynamic-endpoint");
        assert!(text.contains("Unresolved"));
        assert!(text.contains("will be refused"));
        assert!(text.contains("This surface is incomplete"));
    }

    #[test]
    fn sensitive_paths_are_presented_as_a_decision_rather_than_a_grant() {
        let text = fixture("secret-reader");
        assert!(text.contains("Needs a decision"));
        assert!(text.contains("fs.read:secret"));
    }

    #[test]
    fn a_complete_surface_does_not_claim_incompleteness() {
        assert!(!fixture("clean-formatter").contains("incomplete"));
    }

    #[test]
    fn lists_are_capped_and_report_the_remainder() {
        let items = ["a", "b", "c", "d"].map(str::to_owned).to_vec();
        assert_eq!(join(&items, 2), "a and b (+2 more)");
        assert_eq!(join(&items[..1], 2), "a");
        assert_eq!(join(&[], 2), "");
    }
}
