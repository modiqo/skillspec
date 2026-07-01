use crate::support::*;

#[test]
fn validate_and_test_rich_spec_through_cli() {
    let dir = TempDir::new("validate-test");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let validate = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&validate);
    assert!(stdout(&validate).contains("ok:"));

    let test = Command::new(bin()).arg("test").arg(&spec).output().unwrap();
    assert_success(&test);
    assert!(stdout(&test).contains("skillspec test: 2/2 passed"));
}

#[test]
fn validate_writes_persistent_spec_cache() {
    let dir = TempDir::new("spec-cache");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let validate = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&validate);
    assert!(dir
        .path()
        .join(".skillspec/cache/spec-cache.json")
        .is_file());

    let validate_cached = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&validate_cached);
}

#[test]
fn run_loop_batches_common_planning_commands() {
    let dir = TempDir::new("run-loop");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let run_loop = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--input")
        .arg("browse the app")
        .arg("--view")
        .arg("index")
        .arg("--trace-dir")
        .arg(dir.path().join("traces"))
        .output()
        .unwrap();
    assert_success(&run_loop);
    let out = stdout(&run_loop);
    assert!(out.contains("SkillSpec run-loop summary"));
    assert!(out.contains("selected_route: browser"));
    assert!(out.contains("batched_commands: sensemake, decide, plan, act"));
    assert!(out.contains("avoided_cli_invocations: 3"));

    let run_loop_json = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--input")
        .arg("browse the app")
        .arg("--trace-dir")
        .arg(dir.path().join("traces-json"))
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&run_loop_json);
    let report = json_stdout(&run_loop_json);
    assert_eq!(report["decision"]["route"], "browser");
    assert_eq!(report["batched_commands"].as_array().unwrap().len(), 4);
}

#[test]
fn run_loop_guide_agent_writes_resume_state() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let dir = TempDir::new("run-loop-guide");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let run_loop = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--input")
        .arg("browse the app")
        .arg("--trace-dir")
        .arg(dir.path().join("traces"))
        .arg("--guide")
        .arg("agent")
        .arg("--json")
        .output()?;
    assert_success(&run_loop);
    let report = json_stdout(&run_loop);
    assert_eq!(report["schema"], "skillspec.guide-state/v0");
    assert_eq!(report["start"]["selected_route"], "browser");
    assert_eq!(report["start"]["first_phase"], "collect_cli_evidence");
    assert_eq!(report["current_gate"]["phase"], "collect_cli_evidence");
    let allowed_commands = report["current_gate"]["allowed_commands"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing allowed commands"))?;
    assert!(allowed_commands.iter().all(|command| command
        .as_str()
        .is_some_and(|command| !command.contains("skillspec act"))));
    assert!(allowed_commands
        .iter()
        .any(|command| command.as_str().is_some_and(|command| command
            .contains("progress checkpoint")
            && command.contains("--requirement-satisfied")
            && command.contains("--phase-completed")
            && command.contains("--quiet"))));
    assert!(allowed_commands.iter().any(|command| command
        .as_str()
        .is_some_and(|command| command.contains("progress show") && command.contains("--quiet"))));
    let allowed_now = report["current_gate"]["allowed_now"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing allowed_now"))?;
    assert!(allowed_now.iter().all(|item| {
        item.as_str().is_some_and(|item| {
            !item.contains("skillspec query") && !item.contains("skillspec refs")
        })
    }));
    assert!(report["end"]["final_progress_command"]
        .as_str()
        .is_some_and(
            |command| command.contains("progress final-response") && command.contains("--quiet")
        ));
    assert!(report["end"]["token_stats_command"]
        .as_str()
        .is_some_and(|command| command.contains("progress stats")
            && command.contains("--agent-visible-tokens")
            && command.contains("--artifact-tokens-preserved")
            && command.contains("--avoided-tokens")
            && command.contains("--metrics-source estimated")
            && command.contains("--quiet")));
    assert!(report["end"]["alignment_command"]
        .as_str()
        .is_some_and(|command| command.contains("trace align")
            && command.contains("--quiet")
            && !command.contains("--summary")));
    assert!(report["resume"]["command"]
        .as_str()
        .is_some_and(|command| command.contains("--guide agent") && command.contains("--json")));
    assert!(report["current_gate"]["recommended_queries"]
        .as_array()
        .is_some_and(|queries| queries.is_empty()));
    let progress_hints = report["current_gate"]["progress_to_record"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing progress hints"))?;
    assert!(progress_hints
        .iter()
        .any(|hint| hint["event"] == "phase_completed"));
    assert!(progress_hints
        .iter()
        .all(|hint| hint["command"]
            .as_str()
            .is_some_and(|command| !command.starts_with('{')
                && !command.contains("\"event\"")
                && (command.contains("--requirement-satisfied")
                    || command.contains("--phase-completed"))
                && !command.contains("skillspec progress record"))));

    let run_dir = PathBuf::from(
        report["start"]["run_dir"]
            .as_str()
            .ok_or_else(|| invalid_json_shape("missing run_dir"))?,
    );
    assert!(run_dir.join("guide-state.json").is_file());
    assert!(run_dir.join("guide-summary.md").is_file());
    Ok(())
}

#[test]
fn run_loop_guide_suppresses_proof_plumbing_for_diagnostic_routes(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("run-loop-guide-diagnostic");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: cli.diagnostic
title: Diagnostic Spec
description: Exercises quiet diagnostic guide behavior.
activation:
  summary: Diagnostic and proof routes.
entry:
  prompt: Decide before tools.
  decision_required: true
routes:
  - id: diagnostic
    label: Diagnostic
    execution_plan:
      mode: ordered
      phases:
        - id: inspect
          owner_skill: diagnostic
          requires: [doctor]
          forbid: [progress_batch, trace_align]
  - id: proof
    label: Proof
    execution_plan:
      mode: ordered
      phases:
        - id: execute
          owner_skill: proof
          requires: [evidence]
rules:
  - id: doctor_rule
    when:
      user_says_any: [doctor]
    prefer: diagnostic
    forbid: [progress_batch, trace_align]
  - id: proof_rule
    when:
      user_says_any: [execute]
    prefer: proof
trace:
  mode: event_log
  required: true
  record: [input_received, rule_matched, route_selected]
commands:
  doctor:
    template: echo doctor
    safety: read_only
  evidence:
    template: echo evidence
    safety: read_only
"#,
    );

    let diagnostic = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--input")
        .arg("doctor this skill")
        .arg("--trace-dir")
        .arg(dir.path().join("diagnostic-traces"))
        .arg("--guide")
        .arg("agent")
        .arg("--json")
        .output()?;
    assert_success(&diagnostic);
    let diagnostic_report = json_stdout(&diagnostic);
    assert_eq!(diagnostic_report["start"]["selected_route"], "diagnostic");
    let diagnostic_commands = diagnostic_report["current_gate"]["allowed_commands"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing diagnostic allowed commands"))?;
    assert!(diagnostic_commands.iter().all(|command| command
        .as_str()
        .is_some_and(|command| !command.contains("progress"))));
    assert!(diagnostic_report["current_gate"]["progress_to_record"]
        .as_array()
        .is_some_and(|hints| hints.is_empty()));
    assert_eq!(
        diagnostic_report["end"]["alignment_command"],
        "not required for this diagnostic route"
    );
    assert_eq!(
        diagnostic_report["end"]["token_stats_command"],
        "not required for this diagnostic route"
    );

    let proof = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--input")
        .arg("execute this skill")
        .arg("--trace-dir")
        .arg(dir.path().join("proof-traces"))
        .arg("--guide")
        .arg("agent")
        .arg("--json")
        .output()?;
    assert_success(&proof);
    let proof_report = json_stdout(&proof);
    assert_eq!(proof_report["start"]["selected_route"], "proof");
    let proof_commands = proof_report["current_gate"]["allowed_commands"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing proof allowed commands"))?;
    assert!(proof_commands
        .iter()
        .any(|command| command.as_str().is_some_and(|command| command
            .contains("progress checkpoint")
            && command.contains("--quiet"))));
    let proof_hints = proof_report["current_gate"]["progress_to_record"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing proof progress hints"))?;
    assert!(proof_hints
        .iter()
        .all(|hint| hint["command"]
            .as_str()
            .is_some_and(|command| !command.starts_with('{')
                && !command.contains("\"event\"")
                && (command.contains("--requirement-satisfied")
                    || command.contains("--phase-completed"))
                && !command.contains("skillspec progress record"))));
    Ok(())
}

#[test]
fn run_loop_guide_resume_advances_from_execution_ledger(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("run-loop-guide-resume");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let start = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--input")
        .arg("browse the app")
        .arg("--trace-dir")
        .arg(dir.path().join("traces"))
        .arg("--guide")
        .arg("agent")
        .arg("--json")
        .output()?;
    assert_success(&start);
    let start_report = json_stdout(&start);
    let run_dir = PathBuf::from(
        start_report["start"]["run_dir"]
            .as_str()
            .ok_or_else(|| invalid_json_shape("missing run_dir"))?,
    );

    let requirement = Command::new(bin())
        .arg("progress")
        .arg("record")
        .arg(&run_dir)
        .arg("requirement-satisfied")
        .arg("collect_cli_evidence")
        .arg("run_cli_only_through_rote_exec")
        .arg("--evidence-kind")
        .arg("command")
        .arg("--evidence-ref")
        .arg("guide-test")
        .output()?;
    assert_success(&requirement);

    let phase = Command::new(bin())
        .arg("progress")
        .arg("record")
        .arg(&run_dir)
        .arg("phase-completed")
        .arg("collect_cli_evidence")
        .arg("--evidence-kind")
        .arg("command")
        .arg("--evidence-ref")
        .arg("guide-test")
        .output()?;
    assert_success(&phase);

    let resume = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--resume")
        .arg(&run_dir)
        .arg("--guide")
        .arg("agent")
        .arg("--json")
        .output()?;
    assert_success(&resume);
    let resume_report = json_stdout(&resume);
    assert_eq!(resume_report["mode"], "resume");
    assert_eq!(resume_report["current_gate"]["phase"], "browser_handoff");
    let completed_phases = resume_report["path"]["completed_phases"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing completed phases"))?;
    assert!(completed_phases
        .iter()
        .any(|phase| phase == "collect_cli_evidence"));
    Ok(())
}

#[test]
fn run_loop_guide_does_not_treat_blocked_phase_as_final_when_work_can_continue(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("run-loop-guide-blocked-not-final");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let start = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--input")
        .arg("browse the app")
        .arg("--trace-dir")
        .arg(dir.path().join("traces"))
        .arg("--guide")
        .arg("agent")
        .arg("--json")
        .output()?;
    assert_success(&start);
    let start_report = json_stdout(&start);
    let run_dir = PathBuf::from(
        start_report["start"]["run_dir"]
            .as_str()
            .ok_or_else(|| invalid_json_shape("missing run_dir"))?,
    );

    let complete_first = Command::new(bin())
        .arg("progress")
        .arg("record")
        .arg(&run_dir)
        .arg("phase-completed")
        .arg("collect_cli_evidence")
        .arg("--evidence-kind")
        .arg("command")
        .arg("--evidence-ref")
        .arg("guide-test")
        .output()?;
    assert_success(&complete_first);

    let block_second = Command::new(bin())
        .arg("progress")
        .arg("record")
        .arg(&run_dir)
        .arg("phase-blocked")
        .arg("browser_handoff")
        .arg("--evidence-kind")
        .arg("report")
        .arg("--evidence-ref")
        .arg("needs-followup")
        .output()?;
    assert_success(&block_second);

    let resume = Command::new(bin())
        .arg("run-loop")
        .arg(&spec)
        .arg("--resume")
        .arg(&run_dir)
        .arg("--guide")
        .arg("agent")
        .arg("--json")
        .output()?;
    assert_success(&resume);
    let resume_report = json_stdout(&resume);
    let do_now = resume_report["current_gate"]["do_now"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing do_now"))?;
    assert!(do_now.iter().any(|item| item
        .as_str()
        .is_some_and(|item| item.contains("recoverable blockers"))));
    assert!(!do_now.iter().any(|item| item
        .as_str()
        .is_some_and(|item| item.contains("move to the end anchor"))));
    let when_to_advance = resume_report["current_gate"]["when_to_advance"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing when_to_advance"))?;
    assert!(when_to_advance.iter().any(|item| item
        .as_str()
        .is_some_and(|item| item.contains("user intervention"))));
    Ok(())
}

#[test]
fn port_one_shot_runs_import_qa_compile_and_records_stats() {
    let dir = TempDir::new("port-one-shot");
    write_file(&dir.path().join(".git/HEAD"), "ref: refs/heads/main\n");
    write_file(
        &dir.path().join(".git/config"),
        "[remote \"origin\"]\n\turl = https://github.com/example/skills.git\n",
    );
    let source = dir.path().join("source-skill");
    write_file(
        &source.join("SKILL.md"),
        r#"---
name: simple-port
description: Use this skill when a simple port fixture is needed for tests.
---
# Simple Port

Use this skill when the user asks for a simple port fixture.

Steps:

1. Inspect the input.
2. Run the local command.

```sh
echo ok
```
"#,
    );
    let out_dir = dir.path().join("draft");
    let run_dir = dir.path().join("trace-run");

    let port = Command::new(bin())
        .arg("port-one-shot")
        .arg(&source)
        .arg("--out")
        .arg(&out_dir)
        .arg("--target")
        .arg("codex-skill")
        .arg("--prove")
        .arg("--run-dir")
        .arg(&run_dir)
        .arg("--phase")
        .arg("import_skill")
        .arg("--requirement")
        .arg("estimated_token_metrics")
        .output()
        .unwrap();
    assert_success(&port);
    let out = stdout(&port);
    assert!(out.contains("SkillSpec port-one-shot summary"));
    assert!(out.contains("semantic_status: review_required"));
    assert!(out.contains("validate: ok"));
    assert!(out.contains("compile: ok"));
    assert!(out.contains("agent_visible_tokens"));
    assert!(out.contains("detected source git repo"));
    assert!(out.contains("harness restart"));
    assert!(out.contains("real agent interaction"));
    assert!(out.contains("open a PR"));
    assert!(out.contains("compiled.codex-skill.md"));
    assert!(out_dir.join("skill.spec.yml").is_file());
    assert!(out_dir
        .join(".skillspec/source-map/source-map.json")
        .is_file());
    assert!(out_dir.join(".skillspec/port/schema.json").is_file());
    assert!(out_dir.join(".skillspec/port/shape-crib.yml").is_file());
    assert!(out_dir
        .join(".skillspec/port/compiled.codex-skill.md")
        .is_file());
    assert!(out_dir
        .join(".skillspec/port/port-one-shot.report.md")
        .is_file());
    let ledger = fs::read_to_string(run_dir.join("execution.jsonl")).unwrap();
    assert!(ledger.contains("stats_collected"));
    assert!(ledger.contains("agent_visible_tokens"));

    let validate = Command::new(bin())
        .arg("validate")
        .arg(out_dir.join("skill.spec.yml"))
        .output()
        .unwrap();
    assert_success(&validate);
}

#[test]
fn help_lists_trace_align_arguments() {
    let top = Command::new(bin()).arg("--help").output().unwrap();
    assert_success(&top);
    assert!(stdout(&top).contains("trace"));
    assert!(stdout(&top).contains("sensemake"));
    assert!(stdout(&top).contains("run-loop"));
    assert!(stdout(&top).contains("port-one-shot"));
    assert!(stdout(&top).contains("query"));
    assert!(stdout(&top).contains("refs"));
    assert!(stdout(&top).contains("doctor"));
    assert!(stdout(&top).contains("source"));
    assert!(stdout(&top).contains("workspace"));
    assert!(stdout(&top).contains("capability"));
    assert!(stdout(&top).contains("visibility"));
    assert!(stdout(&top).contains("router"));
    assert!(stdout(&top).contains("durable-executor"));
    assert!(stdout(&top).contains("synthesize-from-workspace"));

    let version = Command::new(bin()).arg("--version").output().unwrap();
    assert_success(&version);
    assert!(stdout(&version).contains(env!("CARGO_PKG_VERSION")));

    let trace = Command::new(bin())
        .arg("trace")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&trace);
    let trace_help = stdout(&trace);
    assert!(trace_help.contains("Inspect, compact, or align"));
    assert!(trace_help.contains("align"));

    let align = Command::new(bin())
        .arg("trace")
        .arg("align")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&align);
    let align_help = stdout(&align);
    assert!(align_help.contains("trace align"));
    assert!(align_help.contains("[OPTIONS]"));
    assert!(align_help.contains("--decision-trace <DECISION_TRACE>"));
    assert!(align_help.contains("--execution-trace <EXECUTION_TRACE>"));
    assert!(align_help.contains("--proof-digest <PROOF_DIGEST>"));
    assert!(align_help.contains("<PATH>"));
    assert!(align_help.contains("--summary"));
    assert!(align_help.contains("--json"));

    let synthesize = Command::new(bin())
        .arg("synthesize-from-workspace")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&synthesize);
    let synthesize_help = stdout(&synthesize);
    assert!(synthesize_help.contains("rote-specific"));
    assert!(synthesize_help.contains("durable execution evidence"));
    assert!(synthesize_help.contains("rote workspace"));
    assert!(synthesize_help.contains("--workspace-stats-report"));
    assert!(synthesize_help.contains("--workspace-log"));
    assert!(synthesize_help.contains("--workspace-meta"));
    assert!(synthesize_help.contains("--observation-approved"));

    let source = Command::new(bin())
        .arg("source")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&source);
    let source_help = stdout(&source);
    assert!(source_help.contains("Map and query source packages"));
    assert!(source_help.contains("stage"));
    assert!(source_help.contains("map"));
    assert!(source_help.contains("query"));
    assert!(source_help.contains("coverage"));
    assert!(source_help.contains("stale"));

    let source_stage = Command::new(bin())
        .arg("source")
        .arg("stage")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&source_stage);
    let source_stage_help = stdout(&source_stage);
    assert!(source_stage_help.contains("GitHub"));
    assert!(source_stage_help.contains("--out"));
    assert!(source_stage_help.contains("--no-detect-candidates"));
    assert!(source_stage_help.contains("--json"));

    let workspace = Command::new(bin())
        .arg("workspace")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&workspace);
    let workspace_help = stdout(&workspace);
    assert!(workspace_help.contains(
        "Map, validate, import, converge, compile, and install multi-skill or plugin-shaped workspaces"
    ));
    assert!(workspace_help.contains("map"));
    assert!(workspace_help.contains("validate"));
    assert!(workspace_help.contains("import"));
    assert!(workspace_help.contains("converge"));
    assert!(workspace_help.contains("compile"));
    assert!(workspace_help.contains("install"));
    assert!(workspace_help.contains("plugin-shaped"));

    let workspace_map = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&workspace_map);
    let workspace_map_help = stdout(&workspace_map);
    assert!(workspace_map_help.contains("--install-slug-policy"));
    assert!(workspace_map_help.contains("local-name"));

    let workspace_install = Command::new(bin())
        .arg("workspace")
        .arg("install")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&workspace_install);
    let workspace_install_help = stdout(&workspace_install);
    assert!(workspace_install_help.contains("--install-slug-policy"));
    assert!(workspace_install_help.contains("local-name"));

    let run_loop = Command::new(bin())
        .arg("run-loop")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&run_loop);
    let run_loop_help = stdout(&run_loop);
    assert!(run_loop_help.contains("planning-loop report"));
    assert!(run_loop_help.contains("--input"));
    assert!(run_loop_help.contains("--resume"));
    assert!(run_loop_help.contains("--guide"));
    assert!(run_loop_help.contains("guide-state.json"));
    assert!(run_loop_help.contains("--phase"));

    let import_skill = Command::new(bin())
        .arg("import-skill")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&import_skill);
    let import_help = stdout(&import_skill);
    assert!(import_help.contains("--source-map"));
    assert!(import_help.contains("source-map.json"));

    let index = Command::new(bin())
        .arg("index")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&index);
    let index_help = stdout(&index);
    assert!(index_help.contains("router-specific"));
    assert!(index_help.contains("not source analysis"));
    assert!(index_help.contains("router index refresh"));

    let install_skill = Command::new(bin())
        .arg("install")
        .arg("skill")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&install_skill);
    let install_help = stdout(&install_skill);
    assert!(install_help.contains("--retire-existing"));
    assert!(install_help.contains("Back up and remove"));

    let doctor = Command::new(bin())
        .arg("doctor")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&doctor);
    let doctor_help = stdout(&doctor);
    assert!(doctor_help.contains("agent follow-through risk"));
    assert!(doctor_help.contains("score_model"));
    assert!(doctor_help.contains("frontmatter discovery risk"));
    assert!(doctor_help.contains("workspace risk"));
    assert!(doctor_help.contains("activation-loaded surface"));
    assert!(doctor_help.contains("GitHub repo URI"));
    assert!(doctor_help.contains("shape-only report"));
    assert!(doctor_help.contains("partial sparse checkout"));

    let durable = Command::new(bin())
        .arg("durable-executor")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&durable);
    let durable_help = stdout(&durable);
    assert!(durable_help.contains("install"));
    assert!(durable_help.contains("update"));
    assert!(durable_help.contains("delete"));

    let router = Command::new(bin())
        .arg("router")
        .arg("delete")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&router);
}
