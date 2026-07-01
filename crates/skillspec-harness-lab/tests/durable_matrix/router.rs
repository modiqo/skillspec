use super::commands::{
    file_contains, install_agents_json, install_router_json, router_status_json,
};
use super::fixture::{durable_fixture, write_plain_skill};
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn durable_install_refreshes_router_and_remains_implicit(report: &mut HarnessLabReportBuilder) {
    let fixture = durable_fixture("durable-router", "router hook");
    write_plain_skill(&fixture.lab, &fixture.lab.agents_root(), "pdf");
    let router = install_router_json(&fixture);
    assert_eq!(router["preparedness"]["ready"], true);

    let durable = install_agents_json(&fixture);
    assert!(durable["router_hook"].is_object());
    let install_dir = fixture.lab.agents_root().join("durable-executor");
    assert!(install_dir.join("SKILL.md").is_file());
    let durable_no_openai_sidecar = !install_dir.join("agents/openai.yaml").exists();
    let durable_not_manual_only = !file_contains(
        install_dir.join("SKILL.md"),
        "disable-model-invocation: true",
    );
    assert!(durable_no_openai_sidecar);
    assert!(durable_not_manual_only);

    let status = router_status_json(&fixture);
    assert_eq!(status["stale"], false);
    assert_eq!(status["indexed_skills"], 3);
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("durable_install_refreshes_router_and_remains_implicit");
    case.claim_pass(
        "router.preparedness.ready",
        true,
        &router["preparedness"]["ready"],
    );
    case.claim_pass(
        "durable.router_hook_present",
        true,
        durable["router_hook"].is_object(),
    );
    case.claim_pass(
        "durable.skill_md",
        true,
        install_dir.join("SKILL.md").is_file(),
    );
    case.claim_pass("durable.no_openai_sidecar", true, durable_no_openai_sidecar);
    case.claim_pass("durable.not_manual_only", true, durable_not_manual_only);
    case.claim_pass("router.status.stale", false, &status["stale"]);
    case.claim_pass("router.status.indexed_skills", 3, &status["indexed_skills"]);
    case.finish();
}
