use super::commands::install_router_json;
use super::fixture::{pseudo_fixture, write_imported_widget_skill};
use super::simulator::{event_bool, event_position, simulate_prompt};
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn imported_trampoline_handoff_is_visible(report: &mut HarnessLabReportBuilder) {
    let fixture = pseudo_fixture("pseudo-imported-trampoline");
    write_imported_widget_skill(&fixture.lab);
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let run = simulate_prompt(&fixture, "use widget-flow for the fixture task");
    let loaded = run
        .loaded_skill
        .as_ref()
        .expect("expected widget-flow skill to load");
    assert_eq!(run.route["decision"], "use_skill");
    assert_eq!(loaded.name, "widget-flow");
    assert!(loaded.trampoline);
    assert!(event_bool(&run.events, "trampoline_checked", "trampoline"));
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("imported_trampoline_handoff_is_visible");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass("route.decision", "use_skill", &run.route["decision"]);
    case.claim_pass(
        "route.selected",
        "widget-flow",
        &run.route["selected"]["name"],
    );
    case.claim_pass("domain.loaded", true, run.loaded_skill.is_some());
    case.claim_pass("domain.loaded_name", "widget-flow", &loaded.name);
    case.claim_pass("domain.trampoline", true, loaded.trampoline);
    case.claim_pass(
        "event.trampoline_checked",
        true,
        event_bool(&run.events, "trampoline_checked", "trampoline"),
    );
    case.claim_pass(
        "event_order.load_before_trampoline_check",
        true,
        event_position(&run.events, "domain_skill_loaded")
            < event_position(&run.events, "trampoline_checked"),
    );
    case.finish();
}
