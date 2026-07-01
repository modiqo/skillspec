use super::commands::{
    disable_router_json, enable_router_json, file_contains, has_hook_command, install_router_json,
};
use super::fixture::{router_fixture, write_out_of_band_skill};
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn router_disable_enable_restores_and_reapplies_visibility(
    report: &mut HarnessLabReportBuilder,
) {
    let fixture = router_fixture("router-mode");
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

    let disable = disable_router_json(&fixture);
    assert_eq!(disable["enabled"], false);
    assert!(disable["index_report"].is_null());
    assert!(!has_hook_command(
        &fixture.codex_hooks,
        "skillspec router guard"
    ));
    assert!(has_hook_command(&fixture.codex_hooks, "echo keep-codex"));
    assert!(file_contains(
        &fixture.lab.agents_root().join("pdf/SKILL.md"),
        "disable-model-invocation: false"
    ));
    assert!(file_contains(
        &fixture.lab.codex_root().join("csv/agents/openai.yaml"),
        "allow_implicit_invocation: true"
    ));
    assert!(file_contains(
        &fixture.lab.agents_root().join("skill-router/SKILL.md"),
        "disable-model-invocation: true"
    ));

    write_out_of_band_skill(&fixture.lab);
    let enable = enable_router_json(&fixture);
    assert_eq!(enable["enabled"], true);
    assert_eq!(enable["preparedness"]["ready"], true);
    assert_eq!(enable["index_report"]["skills_indexed"], 8);
    assert!(has_hook_command(
        &fixture.codex_hooks,
        "skillspec router guard"
    ));
    assert!(file_contains(
        &fixture.lab.agents_root().join("pdf/SKILL.md"),
        "disable-model-invocation: true"
    ));
    assert!(file_contains(
        &fixture.lab.codex_root().join("markdown/agents/openai.yaml"),
        "allow_implicit_invocation: false"
    ));
    assert!(!file_contains(
        &fixture.lab.agents_root().join("skill-router/SKILL.md"),
        "disable-model-invocation: true"
    ));
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("router_disable_enable_restores_and_reapplies_visibility");
    case.claim_pass("disable.enabled", false, &disable["enabled"]);
    case.claim_pass(
        "disable.index_report_null",
        true,
        disable["index_report"].is_null(),
    );
    case.claim_pass(
        "disable.hook_removed",
        false,
        has_hook_command(&fixture.codex_hooks, "skillspec router guard"),
    );
    case.claim_pass(
        "disable.hook_preserved",
        true,
        has_hook_command(&fixture.codex_hooks, "echo keep-codex"),
    );
    case.claim_pass(
        "disable.pdf.restored_implicit",
        true,
        file_contains(
            &fixture.lab.agents_root().join("pdf/SKILL.md"),
            "disable-model-invocation: false",
        ),
    );
    case.claim_pass(
        "disable.csv.restored_implicit",
        true,
        file_contains(
            &fixture.lab.codex_root().join("csv/agents/openai.yaml"),
            "allow_implicit_invocation: true",
        ),
    );
    case.claim_pass(
        "disable.router.manual_only",
        true,
        file_contains(
            &fixture.lab.agents_root().join("skill-router/SKILL.md"),
            "disable-model-invocation: true",
        ),
    );
    case.claim_pass("enable.enabled", true, &enable["enabled"]);
    case.claim_pass(
        "enable.preparedness.ready",
        true,
        &enable["preparedness"]["ready"],
    );
    case.claim_pass(
        "enable.indexed_skills",
        8,
        &enable["index_report"]["skills_indexed"],
    );
    case.claim_pass(
        "enable.hook_installed",
        true,
        has_hook_command(&fixture.codex_hooks, "skillspec router guard"),
    );
    case.claim_pass(
        "enable.markdown.manual_only",
        true,
        file_contains(
            &fixture.lab.codex_root().join("markdown/agents/openai.yaml"),
            "allow_implicit_invocation: false",
        ),
    );
    case.claim_pass(
        "enable.router.implicit",
        false,
        file_contains(
            &fixture.lab.agents_root().join("skill-router/SKILL.md"),
            "disable-model-invocation: true",
        ),
    );
    case.finish();
}
