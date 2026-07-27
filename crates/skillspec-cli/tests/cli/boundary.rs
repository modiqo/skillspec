use crate::support::*;
use std::process::Command;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn fixture(name: &str) -> std::path::PathBuf {
    repo_root().join("fixtures/effects").join(name)
}

#[test]
fn boundary_reports_what_a_skill_can_reach() -> TestResult {
    let output = Command::new(bin())
        .arg("boundary")
        .arg(fixture("direct-chain"))
        .output()?;
    assert_success(&output);

    let text = stdout(&output);
    assert!(text.contains("SkillSpec Boundary"));
    assert!(text.contains("If executed, this skill can"));
    assert!(text.contains("~/.aws/credentials"));
    assert!(text.contains("archive.example.com"));
    // The two mandatory statements about what the tool actually did.
    assert!(text.contains("nothing in this package was executed"));
    assert!(text.contains("SkillSpec does not"));
    Ok(())
}

#[test]
fn boundary_json_carries_the_schema_and_both_effect_lists() -> TestResult {
    let output = Command::new(bin())
        .arg("boundary")
        .arg(fixture("dynamic-endpoint"))
        .arg("--json")
        .output()?;
    assert_success(&output);

    let value = json_stdout(&output);
    assert_eq!(
        value["schema"].as_str(),
        Some("skillspec.boundary.effect_surface.v0")
    );
    assert_eq!(value["source_kind"].as_str(), Some("local"));
    // Unresolved effects are a sibling key, so a policy generator has to handle
    // them explicitly rather than inheriting a permissive default.
    assert!(value["unresolved"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("unresolved should be an array"))?
        .iter()
        .any(|effect| effect["class"] == "net.egress"));
    assert!(value["analysis"]["extraction_mode"] == "lexical");
    Ok(())
}

#[test]
fn boundary_reports_a_clean_skill_without_claiming_it_is_safe() -> TestResult {
    let output = Command::new(bin())
        .arg("boundary")
        .arg(fixture("clean-formatter"))
        .output()?;
    assert_success(&output);

    let text = stdout(&output).to_lowercase();
    assert!(text.contains("no network egress and no sensitive path reads"));
    assert!(!text.contains("malicious"));
    assert!(!text.contains("this skill is safe"));
    Ok(())
}

#[test]
fn boundary_emit_renders_a_deny_default_tool_boundary() -> TestResult {
    let output = Command::new(bin())
        .arg("boundary")
        .arg("emit")
        .arg(fixture("secret-reader"))
        .output()?;
    assert_success(&output);

    let text = stdout(&output);
    assert!(text.contains("tool_boundary:"));
    assert!(text.contains("default: deny"));
    // A sensitive class is held for review, never emitted as an allow.
    assert!(text.contains("permission_required_for:"));
    let before_review = text
        .split("permission_required_for")
        .next()
        .unwrap_or_default();
    assert!(!before_review.contains("fs.read:secret"));
    Ok(())
}

#[test]
fn boundary_emit_writes_to_a_file_when_asked() -> TestResult {
    let dir = TempDir::new("boundary-emit");
    let out = dir.path().join("nested/policy.yml");
    let output = Command::new(bin())
        .arg("boundary")
        .arg("emit")
        .arg(fixture("direct-chain"))
        .arg("--out")
        .arg(&out)
        .output()?;
    assert_success(&output);
    assert!(std::fs::read_to_string(&out)?.contains("default: deny"));
    Ok(())
}

#[test]
fn boundary_emit_supports_the_egress_allowlist_format() -> TestResult {
    let output = Command::new(bin())
        .arg("boundary")
        .arg("emit")
        .arg(fixture("direct-chain"))
        .arg("--format")
        .arg("egress-allowlist")
        .output()?;
    assert_success(&output);
    assert!(stdout(&output).contains("archive.example.com"));
    Ok(())
}

#[test]
fn boundary_emit_rejects_an_unsupported_format() -> TestResult {
    // allowed-tools grants rather than restricts, so no such target exists.
    let output = Command::new(bin())
        .arg("boundary")
        .arg("emit")
        .arg(fixture("clean-formatter"))
        .arg("--format")
        .arg("claude-frontmatter")
        .output()?;
    assert_failure(&output);
    assert!(stderr(&output).contains("unknown boundary format"));
    Ok(())
}

#[test]
fn boundary_requires_a_target() -> TestResult {
    let output = Command::new(bin()).arg("boundary").output()?;
    assert_failure(&output);
    assert!(stderr(&output).contains("boundary requires a target"));
    Ok(())
}

#[test]
fn boundary_rejects_a_local_path_that_does_not_exist() -> TestResult {
    // A path-shaped target must not fall through to the remote parser, or the
    // user gets an error about GitHub URLs for a directory they meant to name.
    let output = Command::new(bin())
        .arg("boundary")
        .arg("./no-such-skill-here")
        .output()?;
    assert_failure(&output);
    assert!(stderr(&output).contains("does not exist locally"));
    Ok(())
}

#[test]
fn boundary_rejects_an_unsupported_remote_target() -> TestResult {
    let output = Command::new(bin())
        .arg("boundary")
        .arg("https://example.com/not-github")
        .output()?;
    assert_failure(&output);
    assert!(stderr(&output).contains("public GitHub"));
    Ok(())
}
