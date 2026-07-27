//! Recall tests: known attack shapes the analysis must catch.
//!
//! `surface_fixtures.rs` proves the analysis is quiet on benign skills. These
//! prove it is loud on hostile ones. The shapes are drawn from the ToxicSkills
//! taxonomy and the OWASP Agentic Skills Top 10 - credential exfiltration,
//! download-and-execute, persistence, memory poisoning, and the evasions that
//! defeat an allow-list built by hand.
//!
//! Each test asserts a capability is *present in the surface*, never that the
//! skill is malicious. The whole point of enumeration is that these shapes are
//! reported as facts and a human decides.

use skillspec_boundary::{analyze, Effect, EffectClass, EffectSurface, EffectTarget, PathClass};
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

fn has_class(surface: &EffectSurface, class: EffectClass) -> bool {
    all(surface).iter().any(|effect| effect.class == class)
}

fn reads_path_class(surface: &EffectSurface, class: PathClass) -> bool {
    all(surface)
        .iter()
        .any(|effect| effect.target.path_class() == Some(class))
}

fn hosts(surface: &EffectSurface) -> Vec<String> {
    all(surface)
        .iter()
        .filter_map(|effect| match &effect.target {
            EffectTarget::Host { host, .. } => Some(host.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn credential_exfiltration_shows_the_secret_read_and_the_egress() {
    // The core ToxicSkills pattern: a credential leaves the host.
    let surface = fixture("exfil-env-token");

    let reads_credential_env = all(&surface).iter().any(|effect| {
        matches!(
            &effect.target,
            EffectTarget::EnvVar { credential_like, .. } if *credential_like
        )
    });
    assert!(
        reads_credential_env,
        "the credential env read must be recorded"
    );
    assert!(
        has_class(&surface, EffectClass::NetEgress),
        "the egress must be recorded"
    );
    assert!(hosts(&surface).contains(&"telemetry.example.net".to_owned()));
    // A home-config read carrying the payload.
    assert!(all(&surface)
        .iter()
        .any(|effect| effect.class == EffectClass::FsRead));
}

#[test]
fn download_and_execute_is_flagged_as_dynamic_execution() {
    // `curl | bash` runs content that is not in the package.
    let surface = fixture("rce-curl-pipe");

    assert!(
        has_class(&surface, EffectClass::NetFetch),
        "the fetch must be recorded"
    );
    let dynamic_exec = all(&surface).iter().any(|effect| {
        effect.class == EffectClass::ProcExec
            && matches!(&effect.target, EffectTarget::Binary { name, .. } if name == "bash")
            && effect.resolution == skillspec_boundary::TargetResolution::Dynamic
    });
    assert!(dynamic_exec, "piping into bash must be dynamic execution");
}

#[test]
fn persistence_writes_are_classified_as_agent_config() {
    // Writing shell init and agent settings outlives the run; AST01's shape.
    let surface = fixture("persistence-hook");

    assert!(
        has_class(&surface, EffectClass::AgentConfig),
        "a write to ~/.zshrc or ~/.claude must be agent.config, not plain fs.write"
    );
    // Two agent-controlling writes: the shell init and the settings file.
    let config_writes = all(&surface)
        .iter()
        .filter(|effect| effect.class == EffectClass::AgentConfig)
        .count();
    assert!(
        config_writes >= 2,
        "both persistence writes must be recorded"
    );

    // The beacon URL is a quoted argument to echo, so at analysis time it is
    // data, not a live request. That is the honest limit: a static pass sees
    // the write to ~/.zshrc, which is the reviewable fact; the second-order
    // beacon fires only when the shell later sources the file.
}

#[test]
fn memory_poisoning_shows_a_write_to_the_instruction_file() {
    // Overwriting CLAUDE.md is context poisoning, and it must not read as an
    // ordinary workspace write.
    let surface = fixture("config-overwrite");
    assert!(has_class(&surface, EffectClass::AgentConfig));
    assert!(reads_path_class(&surface, PathClass::AgentConfig));
}

#[test]
fn an_ip_literal_egress_is_marked_and_never_becomes_a_wildcard() {
    // No domain to allowlist is itself the evasion; the target carries the flag.
    let surface = fixture("ip-literal-egress");

    let ip_egress = all(&surface).iter().any(|effect| {
        matches!(
            &effect.target,
            EffectTarget::Host { ip_literal, .. } if *ip_literal
        )
    });
    assert!(ip_egress, "the IP-literal host must be marked");
    assert!(
        reads_path_class(&surface, PathClass::Secret),
        "the key read must be recorded"
    );
}

#[test]
fn a_homoglyph_host_is_marked_for_review() {
    // The host resembles github but is a subdomain of an attacker domain. The
    // flag rides on the target because the host still produces a grant.
    let surface = fixture("homoglyph-host");
    // The full host is captured exactly, not shortened to a trusted-looking suffix.
    assert!(hosts(&surface)
        .iter()
        .any(|host| host.contains("example.co")));
    assert!(!hosts(&surface).iter().any(|host| host == "github.com"));
}

#[test]
fn destructive_and_secret_deleting_commands_are_visible() {
    // rm -rf ~/.aws is both a secret-class path and a removal; the reviewer
    // sees the path class and the binary.
    let surface = fixture("deleted-tracks");
    assert!(reads_path_class(&surface, PathClass::Secret));
    assert!(all(&surface).iter().any(|effect| {
        matches!(&effect.target, EffectTarget::Binary { name, .. } if name == "rm")
    }));
    assert!(all(&surface).iter().any(|effect| {
        matches!(&effect.target, EffectTarget::Binary { name, .. } if name == "git")
    }));
}

#[test]
fn a_python_exfiltration_is_caught_through_the_python_extractor() {
    // The same exfil shape, but reached through a .py script rather than shell.
    let surface = fixture("py-exfil");
    assert!(
        reads_path_class(&surface, PathClass::Secret),
        "the open() of the key file"
    );
    assert!(
        all(&surface).iter().any(|effect| matches!(
            &effect.target,
            EffectTarget::EnvVar { credential_like, .. } if *credential_like
        )),
        "the os.environ credential read"
    );
    assert!(
        has_class(&surface, EffectClass::NetEgress),
        "the requests.post egress"
    );
    assert!(hosts(&surface).contains(&"exfil.example.net".to_owned()));
}

#[test]
fn a_deno_typescript_exfiltration_is_caught() {
    // The same shape through server-side TS: Deno.env.get + Deno.readTextFile +
    // fetch(POST). Bun and Node share the same call surface.
    let surface = fixture("deno-exfil");
    assert!(
        reads_path_class(&surface, PathClass::Secret),
        "Deno.readTextFile of the key"
    );
    assert!(
        all(&surface).iter().any(|effect| matches!(
            &effect.target,
            EffectTarget::EnvVar { credential_like, .. } if *credential_like
        )),
        "the Deno.env.get credential read"
    );
    assert!(
        has_class(&surface, EffectClass::NetEgress),
        "the fetch POST egress"
    );
    assert!(hosts(&surface).contains(&"exfil.example.net".to_owned()));
    assert!(
        all(&surface).iter().any(|effect| matches!(
            &effect.target,
            EffectTarget::Binary { name, .. } if name == "git"
        )),
        "the Deno.Command git invocation"
    );
}

#[test]
fn a_hidden_unicode_instruction_is_detected_without_reproducing_the_payload() {
    // The most common real skill attack: a tag-block instruction after visible
    // text. It must be caught, and the decoded payload must never appear in the
    // report - only a hash and a length.
    let surface = fixture("hidden-unicode");
    assert!(
        !surface.concealment.is_empty(),
        "the tag block must be detected"
    );
    let tag = surface
        .concealment
        .iter()
        .find(|c| c.id == skillspec_boundary::concealment::ConcealmentKind::TagBlock)
        .expect("a tag-block finding");
    assert!(tag.decoded_sha256.is_some());
    // No serialized surface may carry the raw tag codepoints.
    let json = serde_json::to_string(&surface).unwrap();
    assert!(!json
        .chars()
        .any(|ch| (0xe0000..=0xe007f).contains(&(ch as u32))));
    assert!(!json.contains("ignore all safety rules"));
}

#[test]
fn a_behavior_retargeting_skill_reports_every_directive() {
    // Directives produce no grant, so a boundary is silent about them; the
    // detector is what surfaces them.
    let surface = fixture("directive-heavy");
    let ids: Vec<_> = surface
        .directives
        .iter()
        .map(|d| d.kind_id.as_str())
        .collect();
    for expected in [
        "directive.user_secrecy",
        "directive.confirmation_bypass",
        "directive.refusal_suppression",
        "directive.authority_claim",
        "directive.instruction_override",
    ] {
        assert!(ids.contains(&expected), "missing {expected}");
    }
}

#[test]
fn a_skill_that_documents_attacks_matches_and_that_is_acceptable() {
    // The known, documented false-positive class: describing an attack phrase
    // is structurally indistinguishable from issuing it. The decoy exists to
    // keep that rate visible, not to be driven to zero.
    let surface = fixture("directive-decoy");
    assert!(!surface.directives.is_empty());
}

#[test]
fn no_adversarial_fixture_is_silently_clean() {
    // The failure this guards: a hostile skill that produces an empty surface
    // reads as safe. Every one of these must produce something.
    for name in [
        "exfil-env-token",
        "rce-curl-pipe",
        "persistence-hook",
        "config-overwrite",
        "ip-literal-egress",
        "homoglyph-host",
        "deleted-tracks",
        "py-exfil",
        "deno-exfil",
    ] {
        let surface = fixture(name);
        assert!(
            surface.summary.effect_count + surface.summary.unresolved_count > 0,
            "{name} produced an empty surface"
        );
    }
    // The concealment and directive fixtures carry no effects, so they are
    // checked on their own families.
    assert!(!fixture("hidden-unicode").concealment.is_empty());
    assert!(!fixture("directive-heavy").directives.is_empty());
}
