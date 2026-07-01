use super::commands::{file_contains, has_hook_command, index_status_json, install_router_json};
use super::fixture::router_fixture;
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn router_install_writes_hooks_visibility_and_index(report: &mut HarnessLabReportBuilder) {
    let fixture = router_fixture("router-install");
    let install = install_router_json(&fixture);
    assert_eq!(install["router_skill_status"], "installed");
    assert_eq!(install["router_skill_dirs"].as_array().unwrap().len(), 3);
    assert_eq!(install["harness_hooks"].as_array().unwrap().len(), 2);
    assert_eq!(install["preparedness"]["ready"], true);
    assert_eq!(install["preparedness"]["index_stale"], false);
    assert_eq!(install["preparedness"]["indexed_skills"], 7);
    assert_eq!(install["durable_executor"]["present"], true);
    assert!(fixture.index.is_file());
    assert!(fixture.manifest.is_file());
    assert!(fixture.config.is_file());
    assert!(has_hook_command(
        &fixture.codex_hooks,
        "skillspec router guard"
    ));
    assert!(has_hook_command(&fixture.codex_hooks, "echo keep-codex"));
    assert!(has_hook_command(
        &fixture.claude_settings,
        "skillspec router guard"
    ));
    assert!(has_hook_command(
        &fixture.claude_settings,
        "echo keep-claude"
    ));

    for root in [
        fixture.lab.agents_root(),
        fixture.lab.codex_root(),
        fixture.lab.claude_root(),
    ] {
        assert!(root.join("skill-router/SKILL.md").is_file());
        assert!(root.join("skill-router/skill.spec.yml").is_file());
        assert!(root
            .join("skill-router/.skillspec-router-managed")
            .is_file());
    }
    let router_skill =
        std::fs::read_to_string(fixture.lab.codex_root().join("skill-router/SKILL.md")).unwrap();
    assert!(router_skill.contains("Fast Path"));
    assert!(router_skill.contains("decision: \"use_skill\""));
    assert!(router_skill.contains("bypass or ambiguous"));
    assert!(!router_skill.contains("visible discovery surface"));
    assert!(file_contains(
        &fixture.lab.agents_root().join("pdf/SKILL.md"),
        "disable-model-invocation: true"
    ));
    assert!(file_contains(
        &fixture.lab.codex_root().join("csv/agents/openai.yaml"),
        "allow_implicit_invocation: false"
    ));
    assert!(!file_contains(
        &fixture.lab.agents_root().join("skill-router/SKILL.md"),
        "disable-model-invocation: true"
    ));
    assert!(!file_contains(
        &fixture.lab.agents_root().join("durable-executor/SKILL.md"),
        "disable-model-invocation: true"
    ));

    let status = index_status_json(&fixture);
    assert_eq!(status["stale"], false);
    assert_eq!(status["indexed_skills"], 7);
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("router_install_writes_hooks_visibility_and_index");
    case.claim_pass(
        "install.router_skill_status",
        "installed",
        &install["router_skill_status"],
    );
    case.claim_pass(
        "install.router_skill_dirs",
        3,
        install["router_skill_dirs"].as_array().unwrap().len(),
    );
    case.claim_pass(
        "install.harness_hooks",
        2,
        install["harness_hooks"].as_array().unwrap().len(),
    );
    case.claim_pass(
        "install.preparedness.ready",
        true,
        &install["preparedness"]["ready"],
    );
    case.claim_pass(
        "install.preparedness.indexed_skills",
        7,
        &install["preparedness"]["indexed_skills"],
    );
    case.claim_pass(
        "install.durable_present",
        true,
        &install["durable_executor"]["present"],
    );
    case.claim_pass("files.index", true, fixture.index.is_file());
    case.claim_pass("files.config", true, fixture.config.is_file());
    case.claim_pass(
        "hooks.codex.guard",
        true,
        has_hook_command(&fixture.codex_hooks, "skillspec router guard"),
    );
    case.claim_pass(
        "hooks.codex.preserved",
        true,
        has_hook_command(&fixture.codex_hooks, "echo keep-codex"),
    );
    case.claim_pass(
        "hooks.claude.guard",
        true,
        has_hook_command(&fixture.claude_settings, "skillspec router guard"),
    );
    case.claim_pass(
        "hooks.claude.preserved",
        true,
        has_hook_command(&fixture.claude_settings, "echo keep-claude"),
    );
    case.claim_pass(
        "visibility.pdf.manual_only",
        true,
        file_contains(
            &fixture.lab.agents_root().join("pdf/SKILL.md"),
            "disable-model-invocation: true",
        ),
    );
    case.claim_pass(
        "visibility.csv.manual_only",
        true,
        file_contains(
            &fixture.lab.codex_root().join("csv/agents/openai.yaml"),
            "allow_implicit_invocation: false",
        ),
    );
    case.claim_pass(
        "visibility.router.implicit",
        false,
        file_contains(
            &fixture.lab.agents_root().join("skill-router/SKILL.md"),
            "disable-model-invocation: true",
        ),
    );
    case.claim_pass("index.status.stale", false, &status["stale"]);
    case.claim_pass("index.status.indexed_skills", 7, &status["indexed_skills"]);
    case.finish();
}
