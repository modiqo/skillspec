use super::commands::install_router_json;
use super::fixture::pseudo_fixture;
use super::simulator::{event_position, simulate_prompt};
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn durable_observer_remains_implicit_with_router(report: &mut HarnessLabReportBuilder) {
    let fixture = pseudo_fixture("pseudo-durable-observer");
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let run = simulate_prompt(&fixture, "what is the time today");
    assert!(run
        .catalog
        .implicit
        .iter()
        .any(|name| name == "skill-router"));
    assert!(run
        .catalog
        .implicit
        .iter()
        .any(|name| name == "durable-executor"));
    assert!(run.catalog.manual_only.iter().any(|name| name == "pdf"));
    assert_eq!(run.route["decision"], "bypass");
    assert!(run.loaded_skill.is_none());
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("durable_observer_remains_implicit_with_router");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass(
        "catalog.router_implicit",
        true,
        run.catalog
            .implicit
            .iter()
            .any(|name| name == "skill-router"),
    );
    case.claim_pass(
        "catalog.durable_implicit",
        true,
        run.catalog
            .implicit
            .iter()
            .any(|name| name == "durable-executor"),
    );
    case.claim_pass(
        "catalog.pdf_manual_only",
        true,
        run.catalog.manual_only.iter().any(|name| name == "pdf"),
    );
    case.claim_pass("route.decision", "bypass", &run.route["decision"]);
    case.claim_pass("domain.loaded", false, run.loaded_skill.is_some());
    case.claim_pass(
        "event_order.catalog_before_route",
        true,
        event_position(&run.events, "catalog_built")
            < event_position(&run.events, "route_decision"),
    );
    case.finish();
}
