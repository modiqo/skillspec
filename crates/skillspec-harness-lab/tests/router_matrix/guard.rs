use super::commands::{
    file_contains, guard_hook_json, guard_json, index_status_json, install_router_json,
};
use super::fixture::{router_fixture, write_out_of_band_skill};
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn router_guard_repairs_out_of_band_skill_and_hook_context(
    report: &mut HarnessLabReportBuilder,
) {
    let fixture = router_fixture("router-guard");
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    write_out_of_band_skill(&fixture.lab);
    let stale = index_status_json(&fixture);
    assert_eq!(stale["stale"], true);
    assert!(stale["new_skills"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["name"] == "markdown"));
    assert!(!file_contains(
        &fixture.lab.codex_root().join("markdown/agents/openai.yaml"),
        "allow_implicit_invocation: false"
    ));

    let guard = guard_json(&fixture);
    assert_eq!(guard["installed"], true);
    assert_eq!(guard["enabled"], true);
    assert_eq!(guard["repaired"], true);
    assert_eq!(guard["first_hop_ready"], true);
    assert_eq!(guard["status_before"]["stale"], true);
    assert_eq!(guard["status_after"]["stale"], false);
    assert_eq!(guard["index_report"]["skills_indexed"], 8);
    assert!(file_contains(
        &fixture.lab.codex_root().join("markdown/agents/openai.yaml"),
        "allow_implicit_invocation: false"
    ));

    let hook = guard_hook_json(&fixture);
    assert!(hook["decision"].is_null());
    assert!(hook["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap()
        .contains("first_hop_ready=true"));
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("router_guard_repairs_out_of_band_skill_and_hook_context");
    case.claim_pass("status.before.stale", true, &stale["stale"]);
    case.claim_pass(
        "status.before.new_markdown",
        true,
        stale["new_skills"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["name"] == "markdown"),
    );
    case.claim_pass("guard.installed", true, &guard["installed"]);
    case.claim_pass("guard.enabled", true, &guard["enabled"]);
    case.claim_pass("guard.repaired", true, &guard["repaired"]);
    case.claim_pass("guard.first_hop_ready", true, &guard["first_hop_ready"]);
    case.claim_pass(
        "guard.status_before_stale",
        true,
        &guard["status_before"]["stale"],
    );
    case.claim_pass(
        "guard.status_after_stale",
        false,
        &guard["status_after"]["stale"],
    );
    case.claim_pass(
        "guard.indexed_skills",
        8,
        &guard["index_report"]["skills_indexed"],
    );
    case.claim_pass(
        "visibility.markdown.manual_only",
        true,
        file_contains(
            &fixture.lab.codex_root().join("markdown/agents/openai.yaml"),
            "allow_implicit_invocation: false",
        ),
    );
    case.claim_pass("hook.decision_absent", true, hook["decision"].is_null());
    case.claim_pass(
        "hook.context.first_hop_ready",
        true,
        hook["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .contains("first_hop_ready=true"),
    );
    case.finish();
}
