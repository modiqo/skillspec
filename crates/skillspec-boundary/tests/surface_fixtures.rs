//! End-to-end enumeration over the fixture packages in `fixtures/effects/`.
//!
//! These exercise the public API only. They are the tests that would catch an
//! extractor wired up incorrectly, as opposed to an extractor that is wrong in
//! detail; the in-module tables cover the latter.

use skillspec_boundary::{analyze, Effect, EffectClass, EffectSurface, PathClass, Reach};
use std::path::PathBuf;

fn fixture(name: &str) -> EffectSurface {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/effects")
        .join(name);
    analyze(&path).unwrap_or_else(|error| panic!("analyzing {name}: {error}"))
}

fn all(surface: &EffectSurface) -> Vec<&Effect> {
    surface
        .effects
        .iter()
        .chain(surface.unresolved.iter())
        .collect()
}

fn hosts(surface: &EffectSurface, class: EffectClass) -> Vec<String> {
    let mut hosts = all(surface)
        .into_iter()
        .filter(|effect| effect.class == class)
        .map(|effect| effect.target.grant_token())
        .collect::<Vec<_>>();
    hosts.sort();
    hosts.dedup();
    hosts
}

fn path_classes(surface: &EffectSurface) -> Vec<PathClass> {
    let mut classes = all(surface)
        .into_iter()
        .filter_map(|effect| effect.target.path_class())
        .collect::<Vec<_>>();
    classes.sort();
    classes.dedup();
    classes
}

#[test]
fn a_clean_skill_produces_a_small_complete_surface() {
    // The guard against over-enumeration. If this fixture grows a long effect
    // list, the extractors have started inventing findings.
    let surface = fixture("clean-formatter");

    assert!(surface.is_complete(), "clean fixture should be complete");
    assert_eq!(surface.summary.unresolved_count, 0);
    assert!(surface.summary.sensitive_path_classes.is_empty());
    assert!(hosts(&surface, EffectClass::NetEgress).is_empty());
    assert!(hosts(&surface, EffectClass::NetFetch).is_empty());

    let binaries = all(&surface)
        .into_iter()
        .filter(|effect| effect.class == EffectClass::ProcExec)
        .map(|effect| effect.target.grant_token())
        .collect::<Vec<_>>();
    assert_eq!(binaries, ["git"]);
}

#[test]
fn a_realistic_skill_yields_the_grants_it_actually_needs() {
    let surface = fixture("github-reporter");

    // Host is fixed even though the path is interpolated, so it stays grantable.
    assert_eq!(
        hosts(&surface, EffectClass::NetFetch),
        ["api.github.com", "docs.github.com"]
    );

    let credential_reads = all(&surface)
        .into_iter()
        .filter(|effect| effect.class == EffectClass::EnvRead)
        .filter(|effect| effect.grant_label().contains("GITHUB_TOKEN"))
        .count();
    assert_eq!(credential_reads, 1);

    assert!(all(&surface)
        .iter()
        .any(|effect| effect.class == EffectClass::FsWrite));
    assert!(surface.is_complete());
}

#[test]
fn an_interpolated_endpoint_leaves_the_surface_incomplete() {
    let surface = fixture("dynamic-endpoint");

    assert!(!surface.is_complete());
    assert!(surface.summary.unresolved_count >= 1);
    assert!(surface
        .unresolved
        .iter()
        .any(|effect| effect.class == EffectClass::NetEgress));
    // Nothing ungrantable may leak into the grantable list.
    assert!(surface.effects.iter().all(Effect::is_grantable));
}

#[test]
fn a_credential_read_is_surfaced_with_its_class() {
    let surface = fixture("secret-reader");

    assert!(path_classes(&surface).contains(&PathClass::Secret));
    assert!(surface.sensitive().count() >= 1);
    assert_eq!(surface.summary.sensitive_path_classes, ["secret"]);
}

#[test]
fn effects_in_an_unreferenced_script_are_reported_as_unmapped() {
    // The file ships in the package and nothing in SKILL.md reaches it, so a
    // reviewer following the documentation would never see these effects.
    let surface = fixture("unmapped-payload");

    let unmapped = all(&surface)
        .into_iter()
        .filter(|effect| effect.reach == Reach::Unmapped)
        .collect::<Vec<_>>();

    assert!(
        unmapped
            .iter()
            .any(|effect| effect.class == EffectClass::NetEgress),
        "expected the egress from the unreferenced script"
    );
    assert!(unmapped
        .iter()
        .any(|effect| effect.target.path_class() == Some(PathClass::Secret)));

    // The documented command is reachable by reading the skill, so it must not
    // be filed alongside them.
    let documented = all(&surface)
        .into_iter()
        .find(|effect| effect.target.grant_token() == "git")
        .expect("documented git invocation");
    assert_eq!(documented.reach, Reach::Activation);
}

#[test]
fn a_referenced_script_is_deferred_rather_than_unmapped() {
    let surface = fixture("dynamic-endpoint");
    let egress = surface
        .unresolved
        .iter()
        .find(|effect| effect.class == EffectClass::NetEgress)
        .expect("egress observation");
    assert_eq!(egress.reach, Reach::Deferred);
}

#[test]
fn wrappers_are_unwrapped_and_piped_execution_is_dynamic() {
    let surface = fixture("wrapped-exec");

    let privileged = all(&surface).into_iter().any(|effect| {
        matches!(
            &effect.target,
            skillspec_boundary::EffectTarget::Binary { name, privileged }
                if name == "apt-get" && *privileged
        )
    });
    assert!(
        privileged,
        "sudo should unwrap to apt-get and mark elevation"
    );

    // `curl … | bash` runs content that is not in the package.
    assert!(surface.unresolved.iter().any(|effect| {
        matches!(
            &effect.target,
            skillspec_boundary::EffectTarget::Binary { name, .. } if name == "bash"
        )
    }));
}

#[test]
fn the_exfiltration_shape_records_both_halves() {
    let surface = fixture("direct-chain");

    assert!(path_classes(&surface).contains(&PathClass::Secret));
    assert_eq!(
        hosts(&surface, EffectClass::NetEgress),
        ["archive.example.com"]
    );
}

#[test]
fn analysis_is_deterministic() {
    // Effect ids are positional, so any nondeterministic iteration order would
    // surface here. Drift comparison depends on this holding.
    let first = serde_json::to_string(&fixture("github-reporter")).unwrap();
    let second = serde_json::to_string(&fixture("github-reporter")).unwrap();
    assert_eq!(first, second);
}

#[test]
fn reports_never_carry_raw_concealment_characters() {
    // Every preview goes through the sanitizer, so no report surface can
    // reproduce a hidden-character run from an analyzed package.
    for name in [
        "clean-formatter",
        "github-reporter",
        "dynamic-endpoint",
        "secret-reader",
        "unmapped-payload",
        "wrapped-exec",
        "direct-chain",
    ] {
        let json = serde_json::to_string(&fixture(name)).unwrap();
        assert!(!json.chars().any(|ch| matches!(ch as u32,
            0x200b..=0x200d | 0x2060 | 0xfeff | 0xe0000..=0xe007f)));
    }
}
