use crate::support::*;

#[test]
fn validate_rejects_unknown_fields_through_cli() {
    let dir = TempDir::new("validate-negative");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.typo
title: CLI Typo
description: Should fail strict parsing.
routes:
  - id: local
    label: Local
rules:
  - id: typo_rule
    preferr: local
tests:
  - name: route assertion
    input: run
    expect:
      route: local
"#,
    );

    let output = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&output);
    assert!(stderr(&output).contains("unknown field"));
}

#[test]
fn validate_resolves_import_paths_and_sections_from_spec_directory() {
    let dir = TempDir::new("validate-imports");
    let skill_dir = dir.path().join("skill");
    let spec = skill_dir.join("skill.spec.yml");
    write_file(
        &skill_dir.join("references/guide.md"),
        r#"# Guide

## Required Procedure

Follow this section.
"#,
    );
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.imports
title: CLI Imports
description: Exercises import resolution.
imports:
  guide:
    path: references/guide.md
    role: procedure
    section: Required Procedure
    load: always
"#,
    );

    let validate = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&validate);

    let imports = Command::new(bin())
        .arg("imports")
        .arg("check")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&imports);
    let report = json_stdout(&imports);
    assert_eq!(report["ok"], true);
    assert_eq!(report["imports"][0]["id"], "guide");
    assert_eq!(report["imports"][0]["section_found"], true);
}

#[test]
fn validate_rejects_missing_import_files_and_sections() {
    let dir = TempDir::new("validate-imports-negative");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&dir.path().join("guide.md"), "# Guide\n");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.imports_missing
title: CLI Imports Missing
description: Exercises missing import resolution.
imports:
  guide:
    path: guide.md
    role: reference
    section: Missing Section
    load: always
"#,
    );

    let missing_section = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&missing_section);
    assert!(stderr(&missing_section).contains("section"));

    let missing_section_report = Command::new(bin())
        .arg("imports")
        .arg("check")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&missing_section_report);
    let report = json_stdout(&missing_section_report);
    assert_eq!(report["ok"], false);
    assert_eq!(report["imports"][0]["status"], "missing_section");

    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.imports_missing
title: CLI Imports Missing
description: Exercises missing import resolution.
imports:
  guide:
    path: missing.md
    role: reference
    load: always
"#,
    );

    let missing_file = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&missing_file);
    assert!(stderr(&missing_file).contains("missing_file"));

    let missing_file_report = Command::new(bin())
        .arg("imports")
        .arg("check")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&missing_file_report);
    let report = json_stdout(&missing_file_report);
    assert_eq!(report["ok"], false);
    assert_eq!(report["imports"][0]["status"], "missing_file");
    assert!(report["imports"][0]["resolved_path"]
        .as_str()
        .unwrap()
        .ends_with("missing.md"));
}

#[test]
fn validate_rejects_missing_generated_package_sidecars() {
    let dir = TempDir::new("validate-package-sidecars");
    let skill_dir = dir.path().join("skill");
    let spec = skill_dir.join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.sidecars
title: CLI Sidecars
description: Exercises generated package sidecar validation.
routes:
  - id: local
    label: Local
dependencies:
  dependency_ledger:
    kind: file
    path: deps.toml
resources:
  helper_script:
    path: resources/helper.py
    role: script
    used_by:
      - kind: code
        id: helper
code:
  helper:
    language: python
    kind: runnable_script
    source:
      file: resources/helper.py
      from_resource: helper_script
"#,
    );

    let missing = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&missing);
    let missing_error = stderr(&missing);
    assert!(missing_error.contains("package sidecar validation failed"));
    assert!(missing_error.contains("deps.toml missing"));
    assert!(missing_error.contains("resources/helper.py missing"));

    write_file(&skill_dir.join("deps.toml"), "");
    let empty_ledger = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&empty_ledger);
    assert!(stderr(&empty_ledger).contains("deps.toml exists but is empty"));

    write_file(
        &skill_dir.join("deps.toml"),
        "schema_version = 1\ndependency_count = 0\n",
    );
    write_file(&skill_dir.join("resources/helper.py"), "print('ok')\n");
    let valid = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&valid);

    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.sidecars
title: CLI Sidecars
description: Exercises generated package sidecar validation.
routes:
  - id: local
    label: Local
resources:
  helper_script:
    path: resources/other.py
    role: script
    used_by:
      - kind: code
        id: helper
code:
  helper:
    language: python
    kind: runnable_script
    source:
      file: resources/helper.py
      from_resource: helper_script
"#,
    );
    write_file(&skill_dir.join("resources/other.py"), "print('other')\n");
    let mismatch = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&mismatch);
    assert!(stderr(&mismatch).contains("does not match code file"));
}

#[test]
fn imports_check_reports_nested_load_order_across_path_depths() {
    let dir = TempDir::new("imports-nested");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&dir.path().join("procedures/a.md"), "# A\n");
    write_file(&dir.path().join("references/deep/b.md"), "# B\n");
    write_file(&dir.path().join("shared/c.md"), "# C\n");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.imports_nested
title: CLI Nested Imports
description: Exercises nested import load order.
routes:
  - id: local
    label: Local
imports:
  a:
    path: procedures/a.md
    role: procedure
    requires:
      imports: [b]
    used_by:
      - kind: route
        id: local
  b:
    path: references/deep/b.md
    role: reference
    requires:
      imports: [c]
    used_by:
      - kind: route
        id: local
  c:
    path: shared/c.md
    role: policy
    used_by:
      - kind: route
        id: local
tests:
  - name: route assertion
    input: local task
    expect:
      route: local
"#,
    );

    let output = Command::new(bin())
        .arg("imports")
        .arg("check")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["ok"], true);
    assert_eq!(report["load_order"], serde_json::json!(["c", "b", "a"]));
}

#[test]
fn test_command_reports_failed_expectations() {
    let dir = TempDir::new("test-negative");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.failing_test
title: CLI Failing Test
description: Should report expectation failures.
routes:
  - id: local
    label: Local
  - id: browser
    label: Browser
tests:
  - name: wrong route
    input: anything
    expect:
      route: browser
"#,
    );

    let output = Command::new(bin()).arg("test").arg(&spec).output().unwrap();
    assert_failure(&output);
    let out = stdout(&output);
    assert!(out.contains("skillspec test: 0/1 passed"));
    assert!(out.contains("FAIL wrong route"));
    assert!(out.contains("expected route browser"));
}

#[test]
fn decide_enforces_required_trace_and_trace_compaction() {
    let dir = TempDir::new("trace");
    let spec = dir.path().join("skill.spec.yml");
    let trace_root = dir.path().join("traces");
    write_file(&spec, rich_spec());

    let missing_trace = Command::new(bin())
        .arg("decide")
        .arg(&spec)
        .arg("--input=browse the dashboard")
        .output()
        .unwrap();
    assert_failure(&missing_trace);
    assert!(stderr(&missing_trace).contains("trace.required is true"));

    let decide = Command::new(bin())
        .arg("decide")
        .arg(&spec)
        .arg("--input=browse the dashboard")
        .arg("--trace-dir")
        .arg(&trace_root)
        .output()
        .unwrap();
    assert_success(&decide);
    assert!(stderr(&decide).contains("trace: wrote"));

    let decision = json_stdout(&decide);
    assert_eq!(decision["route"], "browser");
    assert_eq!(decision["matched_rules"][0]["id"], "browse_rule");

    let run_dir = fs::read_dir(&trace_root)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .expect("expected trace run directory");
    assert!(run_dir.join("trace.jsonl").is_file());
    assert!(run_dir.join("summary.json").is_file());

    let compact = Command::new(bin())
        .arg("trace")
        .arg("compact")
        .arg(&run_dir)
        .output()
        .unwrap();
    assert_success(&compact);
    let compacted = json_stdout(&compact);
    assert!(compacted["event_count"].as_u64().unwrap() >= 4);
    assert!(Path::new(compacted["trace_jsonl"].as_str().unwrap()).is_file());
    assert!(Path::new(compacted["summary_json"].as_str().unwrap()).is_file());
    let summary: Value = serde_json::from_str(
        &fs::read_to_string(compacted["summary_json"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(summary["skill_id"], "cli.rich");
    assert!(summary["spec_fingerprint"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(summary["input_sha256"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));

    let align = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&align);
    let report = json_stdout(&align);
    assert_eq!(report["status"], "unproven");
    assert_eq!(report["ok"], true);
    assert_eq!(report["summary"]["scope"], "decision_trace_only");
    assert_eq!(report["summary"]["decision_alignment"], "incomplete");
    assert_eq!(report["summary"]["execution_alignment"], "not_evaluated");
    assert_eq!(
        report["summary"]["conclusion"],
        "decision alignment incomplete: 3 deterministic trace check(s) are missing from the reasoning record; execution was not evaluated because no execution trace was supplied"
    );
    assert_eq!(
        report["summary"]["status_meaning"],
        "decision alignment is incomplete because the reasoning trace is missing deterministic facts; execution was not evaluated because no execution trace was supplied"
    );
    assert_eq!(report["summary"]["layers"].as_array().unwrap().len(), 2);
    assert_eq!(report["summary"]["layers"][0]["id"], "decision_replay");
    assert!(report["summary"]["layers"][0]["measures"]
        .as_str()
        .unwrap()
        .contains("Re-run the current resolved SkillSpec"));
    assert_eq!(report["summary"]["layers"][1]["id"], "execution_proof");
    assert!(report["summary"]["layers"][1]["interpretation"]
        .as_str()
        .unwrap()
        .contains("not evaluated because no execution trace was supplied"));
    assert_eq!(report["summary"]["selected_route"], "browser");
    assert_eq!(report["summary"]["route_selection_basis"], "rule_prefer");
    assert_eq!(report["summary"]["route_selection_rule"], "browse_rule");
    assert_eq!(report["summary"]["decision_checks"]["pass"], 7);
    assert_eq!(report["summary"]["decision_checks"]["unproven"], 3);
    assert_eq!(report["summary"]["execution_obligations"]["unproven"], 4);
    let gaps = report["summary"]["evidence_gaps"].as_array().unwrap();
    assert!(gaps
        .iter()
        .any(|gap| { gap["kind"] == "decision_trace" && gap["id"] == "forbids" }));
    assert!(gaps.iter().any(|gap| {
        gap["kind"] == "execution_obligation"
            && gap["obligation_kind"] == "forbid"
            && gap["id"] == "native_search_as_answer"
    }));
    let checks = report["checks"].as_array().unwrap();
    assert!(checks
        .iter()
        .any(|check| { check["id"] == "route_selected" && check["status"] == "pass" }));
    assert!(checks
        .iter()
        .any(|check| { check["id"] == "route_selection_basis" && check["status"] == "pass" }));
    assert!(checks
        .iter()
        .any(|check| { check["id"] == "forbids" && check["status"] == "unproven" }));

    let align_text = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .output()
        .unwrap();
    assert_success(&align_text);
    let align_text_stdout = stdout(&align_text);
    assert!(align_text_stdout.contains("alignment: decision=incomplete, execution=not_evaluated"));
    assert!(align_text_stdout.contains("scope: decision_trace_only"));
    assert!(align_text_stdout.contains("summary: decision alignment incomplete: 3 deterministic trace check(s) are missing from the reasoning record; execution was not evaluated because no execution trace was supplied"));
    assert!(align_text_stdout.contains("meaning: decision alignment is incomplete because the reasoning trace is missing deterministic facts; execution was not evaluated because no execution trace was supplied"));
    assert!(align_text_stdout.contains("model:"));
    assert!(align_text_stdout.contains("decision_replay: Re-run the current resolved SkillSpec"));
    assert!(align_text_stdout.contains("execution_proof: When an execution trace is supplied"));
    assert!(align_text_stdout.contains("proof: execution obligations not evaluated because no execution trace was supplied (4 obligation(s) require execution evidence)"));
    assert!(align_text_stdout.contains("execution_requirements_by_kind:"));
    assert!(align_text_stdout.contains("evidence_needed_for_execution_trace:"));
    assert!(align_text_stdout.contains("execution_evidence_needed:"));
    assert!(align_text_stdout.contains("status: not_evaluated"));
    assert!(align_text_stdout.contains("execution_obligations_not_evaluated:"));
    assert!(align_text_stdout.contains("native_search_as_answer: not_evaluated"));
    assert!(align_text_stdout.contains("execution_obligation native_search_as_answer (forbid)"));
    assert!(align_text_stdout.contains("decision: route browser via rule_prefer (browse_rule)"));
    assert!(
        align_text_stdout.contains("proof: decision checks 7 pass, 0 fail, 3 unproven (10 total)")
    );
    assert!(align_text_stdout.contains("route=1/1"));
    assert!(align_text_stdout.contains("forbid=1/1"));
    assert!(align_text_stdout.contains("elicitation=1/1"));
    assert!(align_text_stdout.contains("after_success=1/1"));

    let changed = fs::read_to_string(&spec).unwrap().replace(
        "description: Exercises core CLI behavior.",
        "description: Exercises core CLI behavior after drift.",
    );
    write_file(&spec, &changed);
    let drift = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&drift);
    let drift_report = json_stdout(&drift);
    assert_eq!(drift_report["status"], "fail");
    assert!(drift_report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| { check["id"] == "spec_fingerprint" && check["status"] == "fail" }));
}

#[test]
fn act_generates_current_route_ooda_checklist() {
    let dir = TempDir::new("act");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());
    let trace_dir = dir.path().join("traces");

    let act = Command::new(bin())
        .arg("act")
        .arg(&spec)
        .arg("--input")
        .arg("browse the profile and collect evidence")
        .arg("--trace-dir")
        .arg(&trace_dir)
        .output()
        .unwrap();
    assert_success(&act);
    assert!(stderr(&act).contains("trace: wrote"));
    let text = stdout(&act);
    assert!(text.contains("SkillSpec action checklist"));
    assert!(text.contains("Selected route: browser"));
    assert!(text.contains("Route authority: The selected route and matched rules override"));
    assert!(text.contains("OODA loop:"));
    assert!(text.contains("Current phase:"));
    assert!(text.contains("collect_cli_evidence owned by durable-executor"));
    assert!(text.contains("requires: run_cli_only_through_rote_exec"));
    assert!(text.contains("PHASE TOOL BOUNDARY - HARD"));
    assert!(text.contains("- default: deny"));
    assert!(text.contains("rote_exec"));
    assert!(text.contains("any_unlisted_tool"));
    assert!(text.contains("any_unlisted_cli"));
    assert!(text.contains("stop and ask for explicit permission"));
    assert!(text.contains("Allowed now:"));
    assert!(text.contains("rote flow search, a named rote workspace, and `rote exec --`"));
    assert!(text.contains("Forbidden:"));
    assert!(text.contains("native_search_as_answer"));
    assert!(text.contains("direct_cli_without_rote_exec"));
    assert!(text.contains("Required elicitations:"));
    assert!(text.contains("mode"));
    assert!(text.contains("Required transitions:"));
    assert!(text
        .contains("complete phase `collect_cli_evidence` before starting phase `browser_handoff`"));
    assert!(text.contains(
        "phase `browser_handoff` hands off to `rote-browse` with boundary `stop_current_skill`"
    ));
    assert!(text.contains(
        "if `cli_evidence_missing`, jump from phase `collect_cli_evidence` to `browser_handoff`"
    ));
    assert!(text.contains("Before each tool call:"));
    assert!(text.contains("[ ] Does this action violate any listed forbid?"));

    let act_json = Command::new(bin())
        .arg("act")
        .arg(&spec)
        .arg("--input")
        .arg("browse the profile and collect evidence")
        .arg("--trace-dir")
        .arg(dir.path().join("json-traces"))
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&act_json);
    let report = json_stdout(&act_json);
    assert_eq!(report["selected_route"], "browser");
    assert_eq!(report["route_selection"]["basis"], "rule_prefer");
    assert_eq!(report["current_phase"]["id"], "collect_cli_evidence");
    assert_eq!(report["current_phase"]["owner_skill"], "durable-executor");
    assert_eq!(report["tool_boundary"]["default"], "deny");
    assert!(report["tool_boundary"]["allow"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "rote_exec"));
    assert!(report["tool_boundary"]["permission_required_for"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "any_unlisted_cli"));
    assert!(report["tool_boundary"]["forbid"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "native_web_search"));
    assert!(report["forbidden"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "native_search_as_answer"));
    assert!(report["required_transitions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item
            .as_str()
            .unwrap()
            .contains("phase `browser_handoff` hands off to `rote-browse`")));
    assert!(report["before_tool_call"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("explicitly allowed")));
}

#[test]
fn plan_lists_ordered_execution_phases() {
    let dir = TempDir::new("plan");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());
    let trace_dir = dir.path().join("traces");

    let plan = Command::new(bin())
        .arg("plan")
        .arg(&spec)
        .arg("--input")
        .arg("browse the profile and collect evidence")
        .arg("--trace-dir")
        .arg(&trace_dir)
        .output()
        .unwrap();
    assert_success(&plan);
    assert!(stderr(&plan).contains("trace: wrote"));
    let text = stdout(&plan);
    assert!(text.contains("SkillSpec phase plan"));
    assert!(text.contains("Selected route: browser"));
    assert!(text.contains("1. collect_cli_evidence owned by durable-executor"));
    assert!(text.contains("2. browser_handoff owned by rote-browse"));
    assert!(text.contains("Current phase: collect_cli_evidence"));
    assert!(text
        .contains("complete phase `collect_cli_evidence` before starting phase `browser_handoff`"));

    let plan_json = Command::new(bin())
        .arg("plan")
        .arg(&spec)
        .arg("--input")
        .arg("browse the profile and collect evidence")
        .arg("--trace-dir")
        .arg(dir.path().join("json-traces"))
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&plan_json);
    let report = json_stdout(&plan_json);
    assert_eq!(report["selected_route"], "browser");
    assert_eq!(report["phases"][0]["id"], "collect_cli_evidence");
    assert_eq!(report["phases"][1]["id"], "browser_handoff");
}

#[test]
fn act_can_expand_a_named_phase() {
    let dir = TempDir::new("act-phase");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());
    let trace_dir = dir.path().join("traces");

    let act = Command::new(bin())
        .arg("act")
        .arg(&spec)
        .arg("--input")
        .arg("browse the profile and collect evidence")
        .arg("--trace-dir")
        .arg(&trace_dir)
        .arg("--phase")
        .arg("browser_handoff")
        .output()
        .unwrap();
    assert_success(&act);
    let text = stdout(&act);
    assert!(text.contains("Current phase:"));
    assert!(text.contains("browser_handoff owned by rote-browse"));
    assert!(text.contains("direct_browser_tool_without_rote_browse"));

    let missing = Command::new(bin())
        .arg("act")
        .arg(&spec)
        .arg("--input")
        .arg("browse the profile and collect evidence")
        .arg("--trace-dir")
        .arg(dir.path().join("missing-traces"))
        .arg("--phase")
        .arg("missing_phase")
        .output()
        .unwrap();
    assert_failure(&missing);
    assert!(stderr(&missing).contains("unknown execution phase"));
}

#[test]
fn progress_records_phase_completion_and_lists_remaining_work() {
    let dir = TempDir::new("progress");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());
    let trace_dir = dir.path().join("traces");

    let plan = Command::new(bin())
        .arg("plan")
        .arg(&spec)
        .arg("--input")
        .arg("browse the profile and collect evidence")
        .arg("--trace-dir")
        .arg(&trace_dir)
        .output()
        .unwrap();
    assert_success(&plan);

    let run_dir = fs::read_dir(&trace_dir)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .expect("expected trace run directory");

    let initial = Command::new(bin())
        .arg("progress")
        .arg("show")
        .arg(&spec)
        .arg("--run")
        .arg(&run_dir)
        .output()
        .unwrap();
    assert_success(&initial);
    let initial_text = stdout(&initial);
    assert!(initial_text.contains("SkillSpec progress"));
    assert!(initial_text.contains("Current:"));
    assert!(initial_text.contains("- collect_cli_evidence"));
    assert!(initial_text.contains("execution ledger: missing"));

    let record = Command::new(bin())
        .arg("progress")
        .arg("record")
        .arg(&run_dir)
        .arg("phase-completed")
        .arg("collect_cli_evidence")
        .arg("--evidence-kind")
        .arg("rote_response")
        .arg("--evidence-ref")
        .arg("@7")
        .output()
        .unwrap();
    assert_success(&record);
    let event = json_stdout(&record);
    assert_eq!(event["event"], "phase_completed");
    assert_eq!(event["phase"], "collect_cli_evidence");

    let obligation = Command::new(bin())
        .arg("progress")
        .arg("record")
        .arg(&run_dir)
        .arg("obligation-satisfied")
        .arg("--id")
        .arg("browser")
        .arg("--evidence-kind")
        .arg("trace")
        .arg("--evidence-ref")
        .arg("@route")
        .output()
        .unwrap();
    assert_success(&obligation);
    let obligation_event = json_stdout(&obligation);
    assert_eq!(obligation_event["event"], "obligation_satisfied");
    assert_eq!(obligation_event["id"], "browser");

    let progressed = Command::new(bin())
        .arg("progress")
        .arg("show")
        .arg(&spec)
        .arg("--run")
        .arg(&run_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&progressed);
    let report = json_stdout(&progressed);
    assert_eq!(report["completed_phases"][0], "collect_cli_evidence");
    assert_eq!(report["current_phase"], "browser_handoff");
    assert_eq!(report["execution_proof"]["event_count"], 2);
    assert!(run_dir.join("execution.jsonl").exists());
    assert!(run_dir.join("progress.json").exists());
}

#[test]
fn trace_align_summarizes_progress_ledger_and_token_stats() {
    let dir = TempDir::new("align-progress");
    let spec = dir.path().join("skill.spec.yml");
    let trace_root = dir.path().join("traces");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.progress-align
title: CLI Progress Alignment
description: Exercises progress-ledger alignment summary.
routes:
  - id: port_skill
    label: Port skill
    execution_plan:
      mode: ordered
      phases:
        - id: extract_source
          owner_skill: skillspec
          requires: [source_map_stale]
        - id: install_skill
          owner_skill: skillspec
          requires: [install_codex]
rules:
  - id: port_request
    when:
      user_says_any: ["port"]
    prefer: port_skill
trace:
  mode: event_log
  required: true
"#,
    );

    let decide = Command::new(bin())
        .arg("decide")
        .arg(&spec)
        .arg("--input=port this skill")
        .arg("--trace-dir")
        .arg(&trace_root)
        .output()
        .unwrap();
    assert_success(&decide);

    let run_dir = fs::read_dir(&trace_root)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .expect("expected trace run directory");
    let execution_trace = run_dir.join("execution.jsonl");
    write_file(
        &execution_trace,
        r#"{"event":"phase_started","phase":"extract_source","evidence":{"kind":"checklist","ref":"skillspec-act"}}
{"event":"requirement_satisfied","phase":"extract_source","requirement":"source_map_stale","evidence":{"kind":"command","ref":"skillspec source stale source-map.json"}}
{"event":"phase_completed","phase":"extract_source","evidence":{"kind":"trace","ref":"@phase-1"}}
{"event":"stats_collected","workspace":"skillspec-align-progress","total_tokens":1234,"response_tokens_cached":500,"reduction_percent":28.5}
"#,
    );

    let align = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&align);
    let report = json_stdout(&align);
    assert_eq!(report["status"], "unproven");
    assert_eq!(report["summary"]["completion"]["decision_replay"], "pass");
    assert_eq!(report["summary"]["completion"]["phase_order"], "pass");
    assert_eq!(
        report["summary"]["completion"]["requirements"],
        "1/2 proven"
    );
    assert_eq!(report["summary"]["completion"]["alignment"], "partial");
    assert_eq!(
        report["summary"]["completion"]["forbidden_actions"],
        "no violations recorded"
    );
    assert!(report["summary"]["completion"]["missing_proof"][0]
        .as_str()
        .unwrap()
        .contains("requirement `install_codex`"));
    assert_eq!(
        report["summary"]["tokens"]["consumption"],
        "total 1234 tokens"
    );
    assert_eq!(
        report["summary"]["tokens"]["savings"],
        "500 tokens saved or cached; 28.5% reduction"
    );
    let alignment_json = run_dir.join("alignment.json");
    assert!(alignment_json.exists());
    let persisted: Value = serde_json::from_str(&fs::read_to_string(&alignment_json).unwrap())
        .expect("alignment.json should be valid JSON");
    assert_eq!(persisted["summary"]["completion"]["alignment"], "partial");

    let align_text = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .output()
        .unwrap();
    assert_success(&align_text);
    let text = stdout(&align_text);
    assert!(text.contains("alignment_summary:"));
    assert!(text.contains("  Decision replay: pass"));
    assert!(text.contains("  Phase order: pass"));
    assert!(text.contains("  Requirements: 1/2 proven"));
    assert!(text.contains(
        "  Missing proof: requirement `install_codex` in phase `install_skill` has no progress event"
    ));
    assert!(text.contains("  Forbidden actions: no violations recorded"));
    assert!(text.contains("  Alignment: partial"));
    assert!(text.contains("token_usage:"));
    assert!(text.contains("  Token consumption: total 1234 tokens"));
    assert!(text.contains("  Token savings: 500 tokens saved or cached; 28.5% reduction"));

    let align_summary = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .arg("--summary")
        .output()
        .unwrap();
    assert_success(&align_summary);
    let summary = stdout(&align_summary);
    assert!(summary.contains("alignment_summary:"));
    assert!(summary.contains("  Decision replay: pass"));
    assert!(summary.contains("  Requirements: 1/2 proven"));
    assert!(summary.contains("  Alignment: partial"));
    assert!(summary.contains("summary_meaning:"));
    assert!(summary.contains(
        "  Decision replay: replays the current spec against the captured input; pass means routing is reproducible."
    ));
    assert!(summary.contains(
        "  Execution proof: checks execution.jsonl for structured evidence; partial or unproven means evidence is missing or incomplete, not that decision replay failed."
    ));
    assert!(summary.contains("token_usage:"));
    assert!(summary.contains("  Token consumption: total 1234 tokens"));
    assert!(summary.contains("  Token savings: 500 tokens saved or cached; 28.5% reduction"));
    assert!(summary.contains("alignment_report:"));
    assert!(!summary.contains("checks:"));
    assert!(!summary.contains("obligations:"));
    assert!(!stderr(&align_summary).contains("alignment: wrote"));

    let proof_digest = run_dir.join("proof-digest.json");
    let align_with_digest = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .arg("--summary")
        .arg("--proof-digest")
        .arg(&proof_digest)
        .output()
        .unwrap();
    assert_success(&align_with_digest);
    let digest_summary = stdout(&align_with_digest);
    assert!(digest_summary.contains("proof_digest:"));
    assert!(proof_digest.exists());
    let digest: Value = serde_json::from_str(&fs::read_to_string(&proof_digest).unwrap())
        .expect("proof digest should be valid JSON");
    assert_eq!(digest["schema"], "skillspec.align.proof_digest/v0");
    assert_eq!(digest["alignment"], "partial");
    assert!(digest["missing_count"].as_u64().unwrap() >= 1);
    assert!(digest["groups"]
        .as_array()
        .unwrap()
        .iter()
        .any(|group| group["kind"] == "phase_requirement"));
    assert!(digest["groups"].as_array().unwrap().iter().any(|group| {
        group["kind"] == "route_fulfillment" && group["recommended_event"] == "route_fulfilled"
    }));
}

#[test]
fn progress_stats_records_rote_workspace_token_evidence_for_alignment() {
    let dir = TempDir::new("progress-stats");
    let spec = dir.path().join("skill.spec.yml");
    let trace_root = dir.path().join("traces");
    let stats_json = dir.path().join("workspace-stats.json");
    write_file(&spec, alignment_spec());
    write_file(
        &stats_json,
        r#"{
  "name": "stats-bridge-workspace",
  "metrics": {
    "total_tokens": 1234,
    "context_tokens": 456
  },
  "token_savings": {
    "source_tokens": 1000,
    "result_tokens": 250,
    "tokens_saved": 750
  }
}
"#,
    );

    let decide = Command::new(bin())
        .arg("decide")
        .arg(&spec)
        .arg("--input=run gh PR status as a tracked background process")
        .arg("--trace-dir")
        .arg(&trace_root)
        .output()
        .unwrap();
    assert_success(&decide);
    let run_dir = fs::read_dir(&trace_root)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .expect("expected trace run directory");

    let progress_stats = Command::new(bin())
        .arg("progress")
        .arg("stats")
        .arg(&run_dir)
        .arg("--workspace-stats-json")
        .arg(&stats_json)
        .output()
        .unwrap();
    assert_success(&progress_stats);
    let event = json_stdout(&progress_stats);
    assert_eq!(event["event"], "stats_collected");
    assert_eq!(event["workspace"], "stats-bridge-workspace");
    assert_eq!(event["total_tokens"], 1234);
    assert_eq!(event["context_tokens"], 456);
    assert_eq!(event["response_tokens_cached"], 1000);
    assert_eq!(event["query_result_tokens"], 250);
    assert_eq!(event["saved_tokens"], 750);

    let execution_trace = run_dir.join("execution.jsonl");
    let align = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&align);
    let report = json_stdout(&align);
    assert_eq!(
        report["summary"]["tokens"]["consumption"],
        "total 1234 tokens"
    );
    assert_eq!(
        report["summary"]["tokens"]["savings"],
        "750 tokens saved by query reduction (1000 cached response tokens reduced to 250 query-result tokens, 75.0% reduction)"
    );
}

#[test]
fn progress_stats_records_human_workspace_report_for_alignment() {
    let dir = TempDir::new("progress-stats-report");
    let spec = dir.path().join("skill.spec.yml");
    let trace_root = dir.path().join("traces");
    let stats_report = dir.path().join("workspace-stats.txt");
    write_file(&spec, alignment_spec());
    write_file(
        &stats_report,
        r#"
Workspace: stats-report-workspace

  Name: stats-report-workspace
  Commands: 8
  Responses: 3

Summary: 596 total tokens, 140 context tokens, and 1,010 saved tokens.

Token Savings:
  Source tokens:      1510 (if agent read full responses)
  Result tokens:      500 (what agent actually consumed)
  Tokens saved:       1010 (66.9% reduction)
"#,
    );

    let decide = Command::new(bin())
        .arg("decide")
        .arg(&spec)
        .arg("--input=run gh PR status as a tracked background process")
        .arg("--trace-dir")
        .arg(&trace_root)
        .output()
        .unwrap();
    assert_success(&decide);
    let run_dir = fs::read_dir(&trace_root)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .expect("expected trace run directory");

    let progress_stats = Command::new(bin())
        .arg("progress")
        .arg("stats")
        .arg(&run_dir)
        .arg("--workspace-stats-report")
        .arg(&stats_report)
        .output()
        .unwrap();
    assert_success(&progress_stats);
    let event = json_stdout(&progress_stats);
    assert_eq!(event["event"], "stats_collected");
    assert_eq!(event["workspace"], "stats-report-workspace");
    assert_eq!(event["total_tokens"], 596);
    assert_eq!(event["context_tokens"], 140);
    assert_eq!(event["response_tokens_cached"], 1510);
    assert_eq!(event["query_result_tokens"], 500);
    assert_eq!(event["saved_tokens"], 1010);

    let execution_trace = run_dir.join("execution.jsonl");
    let align = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&align);
    let report = json_stdout(&align);
    assert_eq!(
        report["summary"]["tokens"]["consumption"],
        "total 596 tokens"
    );
    assert_eq!(
        report["summary"]["tokens"]["savings"],
        "1010 tokens saved by query reduction (1510 cached response tokens reduced to 500 query-result tokens, 66.9% reduction)"
    );
}

#[test]
fn progress_stats_records_estimated_summary_metrics_for_alignment() {
    let dir = TempDir::new("progress-stats-summary");
    let spec = dir.path().join("skill.spec.yml");
    let trace_root = dir.path().join("traces");
    write_file(&spec, alignment_spec());

    let decide = Command::new(bin())
        .arg("decide")
        .arg(&spec)
        .arg("--input=run gh PR status as a tracked background process")
        .arg("--trace-dir")
        .arg(&trace_root)
        .output()
        .unwrap();
    assert_success(&decide);
    let run_dir = fs::read_dir(&trace_root)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .expect("expected trace run directory");

    let progress_stats = Command::new(bin())
        .arg("progress")
        .arg("stats")
        .arg(&run_dir)
        .arg("--agent-visible-tokens")
        .arg("190")
        .arg("--artifact-tokens-preserved")
        .arg("96190")
        .arg("--avoided-tokens")
        .arg("96000")
        .arg("--metrics-source")
        .arg("estimated")
        .output()
        .unwrap();
    assert_success(&progress_stats);
    let event = json_stdout(&progress_stats);
    assert_eq!(event["event"], "stats_collected");
    assert_eq!(event["agent_visible_tokens"], 190);
    assert_eq!(event["artifact_tokens_preserved"], 96190);
    assert_eq!(event["avoided_tokens"], 96000);
    assert_eq!(event["metrics_source"], "estimated");
    assert_eq!(event["source"]["kind"], "summary_metrics");

    let execution_trace = run_dir.join("execution.jsonl");
    let align = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&align);
    let report = json_stdout(&align);
    assert_eq!(
        report["summary"]["tokens"]["consumption"],
        "estimated agent-visible output 190 tokens (estimated; not measured model usage)"
    );
    assert_eq!(
        report["summary"]["tokens"]["savings"],
        "estimated 96000 tokens kept out of chat (96190 artifact tokens preserved; 190 agent-visible tokens; source: estimated)"
    );
}

#[test]
fn progress_stats_refuses_empty_token_evidence() {
    let dir = TempDir::new("progress-stats-empty");
    let run_dir = dir.path().join("run-empty-stats");
    fs::create_dir_all(&run_dir).unwrap();

    let progress_stats = Command::new(bin())
        .arg("progress")
        .arg("stats")
        .arg(&run_dir)
        .output()
        .unwrap();
    assert_failure(&progress_stats);
    assert!(stderr(&progress_stats).contains(
        "progress stats requires --workspace-stats-json, --workspace-stats-report, or at least one explicit token metric"
    ));
    assert!(!run_dir.join("execution.jsonl").exists());
}

#[test]
fn progress_final_response_records_report_section_proof() {
    let dir = TempDir::new("progress-final-response");
    let run_dir = dir.path().join("run-final-response");
    fs::create_dir_all(&run_dir).unwrap();

    let progress_final = Command::new(bin())
        .arg("progress")
        .arg("final-response")
        .arg(&run_dir)
        .arg("--phase")
        .arg("durable_closure")
        .arg("--requirement")
        .arg("record_final_response_sent_event")
        .arg("--requirement")
        .arg("report_workspace_evidence_and_token_math")
        .arg("--result")
        .arg("--evidence")
        .arg("--alignment")
        .arg("--token-savings")
        .output()
        .unwrap();
    assert_success(&progress_final);
    let event = json_stdout(&progress_final);
    assert_eq!(event["event"], "final_response_sent");
    assert_eq!(event["included_result"], true);
    assert_eq!(event["included_evidence"], true);
    assert_eq!(event["included_alignment"], true);
    assert_eq!(event["included_token_savings"], true);

    let ledger = fs::read_to_string(run_dir.join("execution.jsonl")).unwrap();
    assert!(ledger.contains("\"event\":\"final_response_sent\""));
    assert!(ledger.contains("\"included_token_savings\":true"));
    assert!(ledger.contains("\"event\":\"requirement_satisfied\""));
    assert!(ledger.contains("\"requirement\":\"record_final_response_sent_event\""));
    assert!(ledger.contains("\"requirement\":\"report_workspace_evidence_and_token_math\""));
}

#[test]
fn progress_batch_records_multiple_events_with_compact_output() {
    let dir = TempDir::new("progress-batch");
    let run_dir = dir.path().join("run-progress-batch");
    fs::create_dir_all(&run_dir).unwrap();
    let events = dir.path().join("final-proof.jsonl");
    write_file(
        &events,
        r#"{"event":"route-fulfilled","id":"local_skill_port","status":"pass","evidence":{"kind":"directory","ref":"./draft"}}
{"event":"route-check-completed","id":"qa_gate","status":"pass","evidence":{"kind":"command","ref":"skillspec test"}}
{"event":"after_success_completed","id":"generate_value_report","status":"pass","evidence":{"kind":"report","ref":"alignment.json"}}
{"event":"elicitation_answered","id":"approve_scope","status":"pass","evidence":{"kind":"user_approval","ref":"chat"}}
{"event":"obligation_satisfied","id":"install_from_remote_checkout","status":"pass","message":"remote source was staged only"}
"#,
    );

    let progress_batch = Command::new(bin())
        .arg("progress")
        .arg("batch")
        .arg(&run_dir)
        .arg("--events")
        .arg(&events)
        .output()
        .unwrap();
    assert_success(&progress_batch);
    let text = stdout(&progress_batch);
    assert!(text.contains("progress batch: appended 5 events"));
    assert!(text.contains("- route_fulfilled: 1"));
    assert!(text.contains("- route_check_completed: 1"));
    assert!(text.contains("- after_success_completed: 1"));
    assert!(text.contains("- elicitation_answered: 1"));
    assert!(text.contains("- obligation_satisfied: 1"));

    let ledger = fs::read_to_string(run_dir.join("execution.jsonl")).unwrap();
    assert!(ledger.contains("\"event\":\"route_fulfilled\""));
    assert!(ledger.contains("\"event\":\"route_check_completed\""));
    assert!(ledger.contains("\"event\":\"after_success_completed\""));
    assert!(ledger.contains("\"event\":\"elicitation_answered\""));
    assert!(ledger.contains("\"event\":\"obligation_satisfied\""));
    assert!(ledger.contains("\"run_id\":\"run-progress-batch\""));
}

#[test]
fn progress_batch_summary_checkpoints_evidence_with_file_alias() {
    let dir = TempDir::new("progress-batch-summary");
    let run_dir = dir.path().join("run-progress-batch-summary");
    fs::create_dir_all(&run_dir).unwrap();
    let events = dir.path().join("evidence-batch.jsonl");
    write_file(
        &events,
        r#"{"event":"requirement-satisfied","phase":"plan","requirement":"dry_run","status":"pass","evidence":{"kind":"command","ref":"dry-run.log"}}
{"event":"requirement-satisfied","phase":"plan","requirement":"auth_research","status":"pass","evidence":{"kind":"file","ref":"auth.md"}}
{"event":"requirement-satisfied","phase":"create","requirement":"create","status":"pass","evidence":{"kind":"command","ref":"create.log"}}
{"event":"requirement-satisfied","phase":"verify","requirement":"post_ops","status":"pass","evidence":{"kind":"command","ref":"probe.log"}}
{"event":"route-fulfilled","id":"create_adapter","status":"pass","evidence":{"kind":"trace","ref":"alignment.json"}}
"#,
    );

    let progress_batch = Command::new(bin())
        .arg("progress")
        .arg("batch")
        .arg(&run_dir)
        .arg("--file")
        .arg(&events)
        .arg("--checkpoint")
        .arg("checkpointing evidence")
        .arg("--summary")
        .output()
        .unwrap();
    assert_success(&progress_batch);
    let text = stdout(&progress_batch);
    assert!(text.contains("[checkpointing evidence...]"));
    assert!(text.contains("status: ok"));
    assert!(text.contains("records: 5"));
    assert!(text.contains("requirements: auth_research, create, dry_run, post_ops"));
    assert!(text.contains(&format!(
        "trace: {}",
        run_dir.join("execution.jsonl").display()
    )));
    assert!(!text.contains("progress batch: appended"));

    let ledger = fs::read_to_string(run_dir.join("execution.jsonl")).unwrap();
    assert_eq!(ledger.lines().count(), 5);
    assert!(ledger.contains("\"event\":\"requirement_satisfied\""));
    assert!(ledger.contains("\"requirement\":\"dry_run\""));
    assert!(ledger.contains("\"event\":\"route_fulfilled\""));
}

#[test]
fn trace_align_uses_execution_ledger_without_leaking_command_args() {
    let dir = TempDir::new("align-execution");
    let spec = dir.path().join("skill.spec.yml");
    let trace_root = dir.path().join("traces");
    let execution_trace = dir.path().join("execution.jsonl");
    write_file(&spec, alignment_spec());

    let test = Command::new(bin()).arg("test").arg(&spec).output().unwrap();
    assert_success(&test);

    let decide = Command::new(bin())
        .arg("decide")
        .arg(&spec)
        .arg("--input=run gh PR status as a tracked background process")
        .arg("--trace-dir")
        .arg(&trace_root)
        .output()
        .unwrap();
    assert_success(&decide);

    let run_dir = fs::read_dir(&trace_root)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .expect("expected trace run directory");

    write_file(
        &execution_trace,
        r#"{"event":"workspace_created","workspace":"gh-pr-checks-conikeec","anonymous":false}
{"event":"adapter_discovery_finished","workspace":"gh-pr-checks-conikeec","service":"github","matches":[],"fallback_needed":true}
{"event":"cli_readiness_check_finished","workspace":"gh-pr-checks-conikeec","command":"gh auth status private@example.com","executor":"rote_exec","operation_kind":"auth_status","exit_code":0,"ready":true,"stdout_captured":true,"stderr_captured":true}
{"event":"background_process_started","workspace":"gh-pr-checks-conikeec","lease_id":"proc-10","command":"gh pr status --repo private/repo --author secret-user","executor":"rote_exec","operation_kind":"pr_status","stdout_captured":true,"stderr_captured":true}
{"event":"process_wait_finished","workspace":"gh-pr-checks-conikeec","lease_id":"proc-10","exit_code":0,"timed_out":false}
{"event":"workspace_trace_collected","workspace":"gh-pr-checks-conikeec"}
{"event":"stats_collected","workspace":"gh-pr-checks-conikeec","response_tokens_cached":6799,"query_result_tokens":826,"reduction_percent":87.9}
{"event":"final_response_sent","included_result":true,"included_alignment":true,"included_evidence":true,"included_token_savings":true}
"#,
    );

    let align = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&align);
    let out = stdout(&align);
    assert!(!out.contains("private/repo"));
    assert!(!out.contains("private@example.com"));
    assert!(!out.contains("secret-user"));
    let report: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(report["status"], "pass");
    assert_eq!(report["summary"]["scope"], "decision_and_execution_trace");
    assert_eq!(report["summary"]["decision_alignment"], "pass");
    assert_eq!(report["summary"]["execution_alignment"], "pass");
    assert_eq!(report["summary"]["decision_checks"]["fail"], 0);
    assert_eq!(report["summary"]["execution_obligations"]["pass"], 12);
    assert_eq!(report["summary"]["execution_obligations"]["unproven"], 0);
    assert_eq!(
        report["summary"]["tokens"]["consumption"],
        "query-result data 826 tokens recorded"
    );
    assert_eq!(
        report["summary"]["tokens"]["savings"],
        "5973 tokens saved by query reduction (6799 cached response tokens reduced to 826 query-result tokens, 87.9% reduction)"
    );
    assert!(report["checks"].as_array().unwrap().iter().any(|check| {
        check["id"] == "tracked_background_rule_triggered" && check["status"] == "pass"
    }));
    let proof_rows = report["proof_rows"].as_array().unwrap();
    assert!(proof_rows.iter().any(|row| {
        row["requirement"] == "User requested work as a tracked background process"
            && row["status"] == "satisfied"
            && row["observed_evidence"]
                .as_str()
                .unwrap()
                .contains("proc-10")
    }));
    assert!(proof_rows.iter().any(|row| {
        row["requirement"] == "CLI work must be captured through rote exec"
            && row["status"] == "satisfied"
            && row["observed_evidence"].as_str().unwrap().contains("gh")
    }));

    let align_text = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .output()
        .unwrap();
    assert_success(&align_text);
    let text = stdout(&align_text);
    assert!(text.contains("alignment: decision=pass, execution=pass"));
    assert!(text.contains("scope: decision_and_execution_trace"));
    assert!(text.contains("alignment_evidence:"));
    assert!(text.contains("status: satisfied"));
    assert!(text.contains("command(s) gh ran with arguments redacted"));
    assert!(!text.contains("private/repo"));
    assert!(!text.contains("private@example.com"));
    assert!(!text.contains("secret-user"));
}

#[test]
fn trace_align_fails_when_execution_obligation_is_violated() {
    let dir = TempDir::new("align-execution-violation");
    let spec = dir.path().join("skill.spec.yml");
    let trace_root = dir.path().join("traces");
    let execution_trace = dir.path().join("execution.jsonl");
    write_file(&spec, alignment_spec());

    let decide = Command::new(bin())
        .arg("decide")
        .arg(&spec)
        .arg("--input=run gh PR status as a tracked background process")
        .arg("--trace-dir")
        .arg(&trace_root)
        .output()
        .unwrap();
    assert_success(&decide);

    let run_dir = fs::read_dir(&trace_root)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .expect("expected trace run directory");

    write_file(
        &execution_trace,
        r#"{"event":"workspace_created","workspace":"gh-pr-checks-conikeec","anonymous":false}
{"event":"process_started","workspace":"gh-pr-checks-conikeec","command":"gh pr status --repo private/repo --author secret-user","executor":"direct_cli","through_rote":false,"stdout_captured":true,"stderr_captured":true}
"#,
    );

    let align = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg(&spec)
        .arg("--decision-trace")
        .arg(&run_dir)
        .arg("--execution-trace")
        .arg(&execution_trace)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&align);
    let out = stdout(&align);
    assert!(!out.contains("private/repo"));
    assert!(!out.contains("secret-user"));
    let report: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(report["ok"], false);
    assert_eq!(report["status"], "fail");
    assert_eq!(report["summary"]["execution_alignment"], "fail");
    assert_eq!(report["summary"]["completion"]["alignment"], "fail");
    assert!(report["proof_rows"].as_array().unwrap().iter().any(|row| {
        row["requirement"] == "CLI work must be captured through rote exec"
            && row["status"] == "violated"
    }));
}

#[test]
fn deps_check_distinguishes_missing_deferred_and_command_scope() {
    let dir = TempDir::new("deps");
    let spec = dir.path().join("skill.spec.yml");
    let cli_dir = dir.path().join("bin");
    write_success_cli(&cli_dir, "present-cli");
    let test_path = std::env::join_paths([cli_dir]).unwrap();
    write_file(&spec, deps_spec());

    let all = Command::new(bin())
        .env("PATH", &test_path)
        .arg("deps")
        .arg("check")
        .arg(&spec)
        .output()
        .unwrap();
    assert_failure(&all);
    let report = json_stdout(&all);
    assert_eq!(report["ok"], false);
    let statuses = report["dependencies"].as_array().unwrap();
    assert!(statuses.iter().any(|dep| dep["status"] == "missing"));
    assert!(statuses.iter().any(|dep| dep["status"] == "deferred"));

    let scoped = Command::new(bin())
        .env("PATH", &test_path)
        .arg("deps")
        .arg("check")
        .arg(&spec)
        .arg("--command")
        .arg("present")
        .output()
        .unwrap();
    assert_success(&scoped);
    let scoped_report = json_stdout(&scoped);
    assert_eq!(scoped_report["ok"], true);
    assert_eq!(scoped_report["dependencies"].as_array().unwrap().len(), 1);
    assert_eq!(scoped_report["dependencies"][0]["id"], "present_cli");
}
