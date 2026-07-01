use super::commands::{file_contains, guard_json, install_router_json};
use super::fixture::{
    pseudo_fixture, write_duplicate_durable_roots, write_out_of_band_markdown_skill,
};
use super::simulator::{event_bool, event_position, simulate_prompt};
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn router_bypass_does_not_load_domain_skill(report: &mut HarnessLabReportBuilder) {
    let fixture = pseudo_fixture("pseudo-router-bypass");
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let run = simulate_prompt(&fixture, "what is the time today");
    assert_eq!(run.route["decision"], "bypass");
    assert!(run.loaded_skill.is_none());
    assert!(event_bool(&run.events, "hook_invoked", "first_hop_ready"));
    assert!(
        event_position(&run.events, "hook_invoked") < event_position(&run.events, "catalog_built")
    );
    assert!(
        event_position(&run.events, "catalog_built")
            < event_position(&run.events, "route_decision")
    );
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("router_bypass_does_not_load_domain_skill");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass(
        "hook.first_hop_ready",
        true,
        event_bool(&run.events, "hook_invoked", "first_hop_ready"),
    );
    case.claim_pass(
        "event_order.hook_before_catalog",
        true,
        event_position(&run.events, "hook_invoked") < event_position(&run.events, "catalog_built"),
    );
    case.claim_pass(
        "event_order.catalog_before_route",
        true,
        event_position(&run.events, "catalog_built")
            < event_position(&run.events, "route_decision"),
    );
    case.claim_pass("route.decision", "bypass", &run.route["decision"]);
    case.claim_pass("route.selected_null", true, run.route["selected"].is_null());
    case.claim_pass("domain.loaded", false, run.loaded_skill.is_some());
    case.claim_pass(
        "catalog.router_implicit",
        true,
        run.catalog
            .implicit
            .iter()
            .any(|name| name == "skill-router"),
    );
    case.finish();
}

pub fn router_selected_domain_loads_one_skill(report: &mut HarnessLabReportBuilder) {
    let fixture = pseudo_fixture("pseudo-router-selected");
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let run = simulate_prompt(&fixture, "summarize meeting action items as notes");
    let loaded = run
        .loaded_skill
        .as_ref()
        .expect("expected selected notes skill");
    assert_eq!(run.route["decision"], "use_skill");
    assert_eq!(loaded.name, "notes");
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("router_selected_domain_loads_one_skill");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass("route.decision", "use_skill", &run.route["decision"]);
    case.claim_pass("route.selected", "notes", &run.route["selected"]["name"]);
    case.claim_pass("domain.loaded", true, run.loaded_skill.is_some());
    case.claim_pass("domain.loaded_name", "notes", &loaded.name);
    case.claim_pass("domain.trampoline", false, loaded.trampoline);
    case.claim_pass(
        "event_order.route_before_load",
        true,
        event_position(&run.events, "route_decision")
            < event_position(&run.events, "domain_skill_loaded"),
    );
    case.finish();
}

pub fn router_guard_repairs_before_catalog_build(report: &mut HarnessLabReportBuilder) {
    let fixture = pseudo_fixture("pseudo-router-repair");
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);
    write_out_of_band_markdown_skill(&fixture.lab);
    let guard = guard_json(&fixture);
    assert_eq!(guard["repaired"], true);

    let run = simulate_prompt(&fixture, "what is the time today");
    let markdown_sidecar = fixture.lab.codex_root().join("markdown/agents/openai.yaml");
    assert!(file_contains(
        &markdown_sidecar,
        "allow_implicit_invocation: false"
    ));
    assert!(run
        .catalog
        .manual_only
        .iter()
        .any(|name| name == "markdown"));
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("router_guard_repairs_before_catalog_build");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass("guard.repaired", true, &guard["repaired"]);
    case.claim_pass(
        "event_order.hook_before_catalog",
        true,
        event_position(&run.events, "hook_invoked") < event_position(&run.events, "catalog_built"),
    );
    case.claim_pass(
        "out_of_band.markdown_manual_only",
        true,
        file_contains(&markdown_sidecar, "allow_implicit_invocation: false"),
    );
    case.claim_pass(
        "catalog.markdown_manual_only",
        true,
        run.catalog
            .manual_only
            .iter()
            .any(|name| name == "markdown"),
    );
    case.claim_pass("route.decision", "bypass", &run.route["decision"]);
    case.claim_pass("domain.loaded", false, run.loaded_skill.is_some());
    case.finish();
}

pub fn duplicate_root_candidates_collapse_to_one_logical_selection(
    report: &mut HarnessLabReportBuilder,
) {
    let fixture = pseudo_fixture("pseudo-router-duplicates");
    write_duplicate_durable_roots(&fixture.lab);
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let run = simulate_prompt(
        &fixture,
        "use durable-executor to preserve proof for this workflow",
    );
    assert_eq!(run.route["decision"], "use_skill");
    let loaded = run
        .loaded_skill
        .as_ref()
        .expect("expected collapsed durable-executor selection");
    assert_eq!(loaded.name, "durable-executor");
    let expected_path = fixture.lab.claude_root().join("durable-executor/SKILL.md");
    assert_eq!(loaded.path, expected_path);
    fixture.lab.assert_no_real_home_writes();

    let durable_candidates = run
        .route
        .get("candidates")
        .and_then(|value| value.as_array())
        .unwrap()
        .iter()
        .filter(|candidate| candidate["name"] == "durable-executor")
        .count();

    let mut case = report.case("duplicate_root_candidates_collapse_to_one_logical_selection");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass("route.decision", "use_skill", &run.route["decision"]);
    case.claim_pass(
        "route.selected",
        "durable-executor",
        &run.route["selected"]["name"],
    );
    case.claim_pass("route.durable_candidate_count", 1, durable_candidates);
    case.claim_pass("domain.loaded", true, run.loaded_skill.is_some());
    case.claim_pass("domain.loaded_name", "durable-executor", &loaded.name);
    case.claim_pass(
        "domain.selected_path",
        fixture.lab.normalize_path(&expected_path),
        fixture.lab.normalize_path(&loaded.path),
    );
    case.finish();
}
