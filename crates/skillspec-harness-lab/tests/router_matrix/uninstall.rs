use super::commands::{
    file_contains, has_hook_command, install_router_json, uninstall_router_json,
};
use super::fixture::router_fixture;
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn router_uninstall_removes_router_and_restores_visibility(
    report: &mut HarnessLabReportBuilder,
) {
    let fixture = router_fixture("router-uninstall");
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);
    assert!(has_hook_command(
        &fixture.codex_hooks,
        "skillspec router guard"
    ));
    assert!(file_contains(
        &fixture.lab.agents_root().join("pdf/SKILL.md"),
        "disable-model-invocation: true"
    ));

    let uninstall = uninstall_router_json(&fixture);
    assert_eq!(uninstall["router_skill_status"], "removed");
    assert_eq!(uninstall["index_removed"], true);
    assert_eq!(uninstall["config_removed"], true);
    assert!(!fixture.index.exists());
    assert!(!fixture.config.exists());
    assert!(!fixture.lab.agents_root().join("skill-router").exists());
    assert!(!fixture.lab.codex_root().join("skill-router").exists());
    assert!(!fixture.lab.claude_root().join("skill-router").exists());
    assert!(!has_hook_command(
        &fixture.codex_hooks,
        "skillspec router guard"
    ));
    assert!(has_hook_command(&fixture.codex_hooks, "echo keep-codex"));
    assert!(!has_hook_command(
        &fixture.claude_settings,
        "skillspec router guard",
    ));
    assert!(has_hook_command(
        &fixture.claude_settings,
        "echo keep-claude"
    ));
    assert!(!file_contains(
        &fixture.lab.agents_root().join("pdf/SKILL.md"),
        "disable-model-invocation: true"
    ));
    assert!(!file_contains(
        &fixture.lab.codex_root().join("csv/agents/openai.yaml"),
        "allow_implicit_invocation: false"
    ));
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("router_uninstall_removes_router_and_restores_visibility");
    case.claim_pass(
        "uninstall.router_skill_status",
        "removed",
        &uninstall["router_skill_status"],
    );
    case.claim_pass("uninstall.index_removed", true, &uninstall["index_removed"]);
    case.claim_pass(
        "uninstall.config_removed",
        true,
        &uninstall["config_removed"],
    );
    case.claim_pass("files.index_removed", false, fixture.index.exists());
    case.claim_pass("files.config_removed", false, fixture.config.exists());
    case.claim_pass(
        "files.agents_router_removed",
        false,
        fixture.lab.agents_root().join("skill-router").exists(),
    );
    case.claim_pass(
        "files.codex_router_removed",
        false,
        fixture.lab.codex_root().join("skill-router").exists(),
    );
    case.claim_pass(
        "files.claude_router_removed",
        false,
        fixture.lab.claude_root().join("skill-router").exists(),
    );
    case.claim_pass(
        "hooks.codex.guard_removed",
        false,
        has_hook_command(&fixture.codex_hooks, "skillspec router guard"),
    );
    case.claim_pass(
        "hooks.codex.preserved",
        true,
        has_hook_command(&fixture.codex_hooks, "echo keep-codex"),
    );
    case.claim_pass(
        "hooks.claude.guard_removed",
        false,
        has_hook_command(&fixture.claude_settings, "skillspec router guard"),
    );
    case.claim_pass(
        "hooks.claude.preserved",
        true,
        has_hook_command(&fixture.claude_settings, "echo keep-claude"),
    );
    case.claim_pass(
        "visibility.pdf.restored",
        false,
        file_contains(
            &fixture.lab.agents_root().join("pdf/SKILL.md"),
            "disable-model-invocation: true",
        ),
    );
    case.claim_pass(
        "visibility.csv.restored",
        false,
        file_contains(
            &fixture.lab.codex_root().join("csv/agents/openai.yaml"),
            "allow_implicit_invocation: false",
        ),
    );
    case.finish();
}
