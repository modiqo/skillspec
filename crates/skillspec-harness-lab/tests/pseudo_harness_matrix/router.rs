use super::commands::{file_contains, guard_json, install_router_json};
use super::fixture::{
    install_rote_execution_skills, pseudo_fixture, write_duplicate_durable_roots,
    write_out_of_band_markdown_skill,
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

pub fn local_action_without_rote_shell_reports_repair(report: &mut HarnessLabReportBuilder) {
    let fixture = pseudo_fixture("pseudo-router-local-action-missing-rote-shell");
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let run = simulate_prompt(
        &fixture,
        "use durable-executor to run a shell command and preserve proof",
    );
    assert_eq!(run.route["decision"], "bypass");
    assert_eq!(
        run.route["bypass_reason"],
        "required_execution_substrate_unavailable"
    );
    assert_eq!(run.route["execution_policy"]["kind"], "local_action");
    assert_eq!(
        run.route["execution_policy"]["preferred_skill"],
        "rote-shell"
    );
    assert!(run.loaded_skill.is_none());
    fixture.lab.assert_no_real_home_writes();

    let repair = run.route["execution_policy"]["repair"]
        .as_str()
        .unwrap_or_default();

    let mut case = report.case("local_action_without_rote_shell_reports_repair");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass("route.decision", "bypass", &run.route["decision"]);
    case.claim_pass(
        "route.bypass_reason",
        "required_execution_substrate_unavailable",
        &run.route["bypass_reason"],
    );
    case.claim_pass(
        "policy.kind",
        "local_action",
        &run.route["execution_policy"]["kind"],
    );
    case.claim_pass(
        "policy.substrate",
        "rote_shell",
        &run.route["execution_policy"]["substrate"],
    );
    case.claim_pass(
        "policy.preferred_skill",
        "rote-shell",
        &run.route["execution_policy"]["preferred_skill"],
    );
    case.claim_pass(
        "policy.repair_mentions_rote_shell",
        true,
        repair.contains("rote-shell"),
    );
    case.claim_pass("domain.loaded", false, run.loaded_skill.is_some());
    case.finish();
}

pub fn local_action_with_rote_shell_selects_rote_shell(report: &mut HarnessLabReportBuilder) {
    let fixture = pseudo_fixture("pseudo-router-local-action-rote-shell");
    install_rote_execution_skills(&fixture.lab);
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let run = simulate_prompt(
        &fixture,
        "use durable-executor to run a shell command and preserve proof",
    );
    assert_eq!(run.route["decision"], "use_skill");
    assert_eq!(run.route["selected"]["name"], "rote-shell");
    assert_eq!(run.route["execution_policy"]["kind"], "local_action");
    assert_eq!(run.route["execution_policy"]["availability"], "active");
    assert!(run.route["elicitation"].is_null());
    let loaded = run
        .loaded_skill
        .as_ref()
        .expect("expected rote-shell selection");
    assert_eq!(loaded.name, "rote-shell");
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("local_action_with_rote_shell_selects_rote_shell");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass("route.decision", "use_skill", &run.route["decision"]);
    case.claim_pass(
        "route.selected",
        "rote-shell",
        &run.route["selected"]["name"],
    );
    case.claim_pass(
        "policy.kind",
        "local_action",
        &run.route["execution_policy"]["kind"],
    );
    case.claim_pass(
        "policy.substrate",
        "rote_shell",
        &run.route["execution_policy"]["substrate"],
    );
    case.claim_pass(
        "policy.availability",
        "active",
        &run.route["execution_policy"]["availability"],
    );
    case.claim_pass(
        "route.elicitation_null",
        true,
        run.route["elicitation"].is_null(),
    );
    case.claim_pass("domain.loaded", true, run.loaded_skill.is_some());
    case.claim_pass("domain.loaded_name", "rote-shell", &loaded.name);
    case.claim_pass(
        "fixture.rote_shell_teaches_rote_exec",
        true,
        file_contains(
            fixture.lab.agents_root().join("rote-shell/SKILL.md"),
            "rote exec --",
        ),
    );
    case.claim_pass(
        "fixture.rote_browse_present",
        true,
        fixture
            .lab
            .agents_root()
            .join("rote-browse/SKILL.md")
            .is_file(),
    );
    case.finish();
}

pub fn browser_action_with_rote_browse_selects_rote_browse(report: &mut HarnessLabReportBuilder) {
    let fixture = pseudo_fixture("pseudo-router-browser-rote-browse");
    install_rote_execution_skills(&fixture.lab);
    let install = install_router_json(&fixture);
    assert_eq!(install["preparedness"]["ready"], true);

    let run = simulate_prompt(
        &fixture,
        "browse the authenticated dashboard and snapshot the current page",
    );
    assert_eq!(run.route["decision"], "use_skill");
    assert_eq!(run.route["selected"]["name"], "rote-browse");
    assert_eq!(run.route["execution_policy"]["kind"], "browse");
    assert_eq!(run.route["execution_policy"]["availability"], "active");
    assert!(run.route["elicitation"].is_null());
    let loaded = run
        .loaded_skill
        .as_ref()
        .expect("expected rote-browse selection");
    assert_eq!(loaded.name, "rote-browse");
    fixture.lab.assert_no_real_home_writes();

    let forbids = run.route["execution_policy"]["forbids"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let forbid_values = forbids
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();

    let mut case = report.case("browser_action_with_rote_browse_selects_rote_browse");
    case.claim_pass("install.ready", true, &install["preparedness"]["ready"]);
    case.claim_pass("route.decision", "use_skill", &run.route["decision"]);
    case.claim_pass(
        "route.selected",
        "rote-browse",
        &run.route["selected"]["name"],
    );
    case.claim_pass(
        "policy.kind",
        "browse",
        &run.route["execution_policy"]["kind"],
    );
    case.claim_pass(
        "policy.substrate",
        "rote_browse",
        &run.route["execution_policy"]["substrate"],
    );
    case.claim_pass(
        "policy.availability",
        "active",
        &run.route["execution_policy"]["availability"],
    );
    case.claim_pass(
        "policy.forbids_direct_browser_tooling",
        true,
        forbid_values.contains(&"direct_browser_tooling"),
    );
    case.claim_pass(
        "policy.prefers_current_authenticated_session",
        true,
        run.route["execution_policy"]["preferences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|preference| preference == "prefer_current_authenticated_browser_session"),
    );
    case.claim_pass(
        "route.elicitation_null",
        true,
        run.route["elicitation"].is_null(),
    );
    case.claim_pass("domain.loaded", true, run.loaded_skill.is_some());
    case.claim_pass("domain.loaded_name", "rote-browse", &loaded.name);
    case.claim_pass(
        "fixture.rote_browse_forbids_raw_playwright",
        true,
        file_contains(
            fixture.lab.agents_root().join("rote-browse/SKILL.md"),
            "raw Playwright",
        ),
    );
    case.claim_pass(
        "fixture.rote_shell_present",
        true,
        fixture
            .lab
            .agents_root()
            .join("rote-shell/SKILL.md")
            .is_file(),
    );
    case.finish();
}
