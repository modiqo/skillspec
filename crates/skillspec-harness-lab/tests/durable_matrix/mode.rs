use super::commands::{disable_json, enable_json, file_contains, install_agents_codex_json};
use super::fixture::durable_fixture;
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn durable_disable_enable_toggles_implicit_invocation(report: &mut HarnessLabReportBuilder) {
    let fixture = durable_fixture("durable-mode", "toggle visibility");
    let install = install_agents_codex_json(&fixture);
    assert_eq!(install["managed_installs"].as_array().unwrap().len(), 2);
    let agents_install = fixture.lab.agents_root().join("durable-executor");
    let codex_install = fixture.lab.codex_root().join("durable-executor");

    let disable = disable_json(&fixture);
    assert_eq!(disable["enabled"], false);
    let disable_agents_frontmatter_manual_only = file_contains(
        agents_install.join("SKILL.md"),
        "disable-model-invocation: true",
    );
    let disable_agents_openai_manual_only = file_contains(
        agents_install.join("agents/openai.yaml"),
        "allow_implicit_invocation: false",
    );
    let disable_codex_openai_manual_only = file_contains(
        codex_install.join("agents/openai.yaml"),
        "allow_implicit_invocation: false",
    );
    assert!(disable_agents_frontmatter_manual_only);
    assert!(disable_agents_openai_manual_only);
    assert!(disable_codex_openai_manual_only);

    let enable = enable_json(&fixture);
    assert_eq!(enable["enabled"], true);
    assert_eq!(enable["rote_preflight"]["present"], true);
    let enable_agents_frontmatter_implicit = file_contains(
        agents_install.join("SKILL.md"),
        "disable-model-invocation: false",
    );
    let enable_agents_openai_implicit = file_contains(
        agents_install.join("agents/openai.yaml"),
        "allow_implicit_invocation: true",
    );
    let enable_codex_openai_implicit = file_contains(
        codex_install.join("agents/openai.yaml"),
        "allow_implicit_invocation: true",
    );
    assert!(enable_agents_frontmatter_implicit);
    assert!(enable_agents_openai_implicit);
    assert!(enable_codex_openai_implicit);
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("durable_disable_enable_toggles_implicit_invocation");
    case.claim_pass(
        "install.target_count",
        2,
        install["managed_installs"].as_array().unwrap().len(),
    );
    case.claim_pass("disable.enabled", false, &disable["enabled"]);
    case.claim_pass(
        "disable.agents_frontmatter_manual_only",
        true,
        disable_agents_frontmatter_manual_only,
    );
    case.claim_pass(
        "disable.agents_openai_manual_only",
        true,
        disable_agents_openai_manual_only,
    );
    case.claim_pass(
        "disable.codex_openai_manual_only",
        true,
        disable_codex_openai_manual_only,
    );
    case.claim_pass("enable.enabled", true, &enable["enabled"]);
    case.claim_pass(
        "enable.rote_present",
        true,
        &enable["rote_preflight"]["present"],
    );
    case.claim_pass(
        "enable.agents_frontmatter_implicit",
        true,
        enable_agents_frontmatter_implicit,
    );
    case.claim_pass(
        "enable.agents_openai_implicit",
        true,
        enable_agents_openai_implicit,
    );
    case.claim_pass(
        "enable.codex_openai_implicit",
        true,
        enable_codex_openai_implicit,
    );
    case.finish();
}
