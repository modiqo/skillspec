use super::commands::{install_router_json, route_json};
use super::fixture::router_fixture;
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn router_routes_clear_intent_and_bypasses_ordinary_tasks(
    report: &mut HarnessLabReportBuilder,
) {
    let fixture = router_fixture("router-route");
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let notes = route_json(
        &fixture.lab,
        &fixture.index,
        "summarize meeting action items as notes",
    );
    assert_eq!(notes["decision"], "use_skill");
    assert_eq!(notes["selected"]["name"], "notes");

    let time = route_json(&fixture.lab, &fixture.index, "what is the time today");
    assert_ne!(time["decision"], "use_skill");
    assert!(time["selected"].is_null());
    assert!(time["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .all(|candidate| candidate["name"] != "skill-router"));
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("router_routes_clear_intent_and_bypasses_ordinary_tasks");
    case.claim_pass("route.notes.decision", "use_skill", &notes["decision"]);
    case.claim_pass("route.notes.selected", "notes", &notes["selected"]["name"]);
    case.claim_pass(
        "route.time.not_use_skill",
        true,
        time["decision"] != "use_skill",
    );
    case.claim_pass(
        "route.time.selected_is_null",
        true,
        time["selected"].is_null(),
    );
    case.claim_pass(
        "route.time.excludes_router",
        true,
        time["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|candidate| candidate["name"] != "skill-router"),
    );
    case.finish();
}
