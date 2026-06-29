use crate::support::*;

#[test]
#[cfg(unix)]
fn capability_add_inspect_verify_search_prefer_and_remove() {
    let dir = TempDir::new("capability");
    let skillspec_home = dir.path().join("skillspec-home");
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(
        &bin_dir.join("voice-cli"),
        "#!/bin/sh\nprintf 'remote voice text to speech voice generation\\n'\n",
    );
    write_executable(
        &bin_dir.join("say"),
        "#!/bin/sh\nprintf 'macOS say text to speech local voice\\n'\n",
    );
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let add = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("capability")
        .arg("add")
        .arg("remote-voice-cli")
        .arg("--domain")
        .arg("voice")
        .arg("--kind")
        .arg("cli")
        .arg("--command")
        .arg("voice-cli")
        .arg("--provides")
        .arg("text_to_speech")
        .arg("--provides")
        .arg("voice_generation")
        .arg("--alias")
        .arg("voice message")
        .arg("--priority")
        .arg("80")
        .arg("--preferred-for")
        .arg("text_to_speech")
        .arg("--tie")
        .arg("quality=high")
        .arg("--auth-env")
        .arg("VOICE_PROVIDER_API_KEY")
        .arg("--external-service")
        .arg("--may-cost-money")
        .arg("--evidence-command")
        .arg("voice-cli --help")
        .arg("--suggested-skill-id")
        .arg("voice.provider")
        .output()
        .unwrap();
    assert_success(&add);
    let add_report = json_stdout(&add);
    assert_eq!(add_report["status"], "written");
    assert!(skillspec_home
        .join("capabilities/voice/remote-voice-cli.yml")
        .is_file());

    let inspect = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("capability")
        .arg("inspect")
        .arg("remote-voice-cli")
        .arg("--domain")
        .arg("voice")
        .output()
        .unwrap();
    assert_success(&inspect);
    let inspected = json_stdout(&inspect);
    assert_eq!(inspected["seed"]["rank"]["tie_breakers"]["quality"], "high");
    assert_eq!(
        inspected["seed"]["promotion"]["suggested_skill_id"],
        "voice.provider"
    );

    let verify = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("capability")
        .arg("verify")
        .arg("remote-voice-cli")
        .arg("--domain")
        .arg("voice")
        .output()
        .unwrap();
    assert_success(&verify);
    let verified = json_stdout(&verify);
    assert_eq!(verified["status"], "verified");
    assert!(verified["outcomes"].as_array().unwrap().len() >= 2);

    let update = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("capability")
        .arg("update")
        .arg("remote-voice-cli")
        .arg("--domain")
        .arg("voice")
        .arg("--add-provides")
        .arg("speech_synthesis")
        .arg("--add-alias")
        .arg("read aloud")
        .arg("--add-preferred-for")
        .arg("speech_synthesis")
        .arg("--add-avoid-for")
        .arg("voice_agent")
        .arg("--priority")
        .arg("35")
        .arg("--add-tie")
        .arg("latency=low")
        .arg("--mark-unverified")
        .output()
        .unwrap();
    assert_success(&update);
    let updated = json_stdout(&update);
    assert_eq!(updated["status"], "updated");
    assert_eq!(updated["seed"]["command"], "voice-cli");
    assert!(updated["seed"]["provides"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "text_to_speech"));
    assert!(updated["seed"]["provides"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "speech_synthesis"));
    assert!(updated["seed"]["aliases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|alias| alias == "voice message"));
    assert!(updated["seed"]["aliases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|alias| alias == "read aloud"));
    assert_eq!(updated["seed"]["rank"]["default_priority"], 35);
    assert!(updated["seed"]["rank"]["preferred_for"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "speech_synthesis"));
    assert!(updated["seed"]["rank"]["avoid_for"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "voice_agent"));
    assert_eq!(updated["seed"]["rank"]["tie_breakers"]["quality"], "high");
    assert_eq!(updated["seed"]["rank"]["tie_breakers"]["latency"], "low");
    assert_eq!(updated["seed"]["verification"]["status"], "unverified");

    let search = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("capability")
        .arg("search")
        .arg("text_to_speech")
        .arg("--domain")
        .arg("voice")
        .arg("--explain")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&search);
    let ranked = json_stdout(&search);
    assert_eq!(ranked["selected"], "remote-voice-cli");
    assert_eq!(ranked["candidates"][0]["id"], "remote-voice-cli");
    assert!(ranked["candidates"][0]["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason.as_str().unwrap().contains("direct provides match")));
    assert!(ranked["candidates"][0]["required_gates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|gate| gate == "provider_cost_approval"));
    assert!(ranked["candidates"][0]["required_gates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|gate| gate == "secret_use_approval"));

    let prefer = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("capability")
        .arg("prefer")
        .arg("remote-voice-cli")
        .arg("--domain")
        .arg("voice")
        .arg("--for")
        .arg("realistic_voice")
        .arg("--priority")
        .arg("90")
        .output()
        .unwrap();
    assert_success(&prefer);
    let preferred = json_stdout(&prefer);
    assert_eq!(preferred["seed"]["rank"]["default_priority"], 90);
    assert!(preferred["seed"]["rank"]["preferred_for"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "realistic_voice"));

    let mark_failed = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("capability")
        .arg("update")
        .arg("remote-voice-cli")
        .arg("--domain")
        .arg("voice")
        .arg("--remove-preferred-for")
        .arg("text_to_speech")
        .arg("--add-avoid-for")
        .arg("text_to_speech")
        .arg("--priority")
        .arg("0")
        .arg("--mark-failed")
        .output()
        .unwrap();
    assert_success(&mark_failed);
    let failed = json_stdout(&mark_failed);
    assert_eq!(failed["seed"]["rank"]["default_priority"], 0);
    assert!(!failed["seed"]["rank"]["preferred_for"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "text_to_speech"));
    assert!(failed["seed"]["rank"]["avoid_for"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "text_to_speech"));
    assert_eq!(failed["seed"]["verification"]["status"], "failed");
    assert_eq!(failed["seed"]["command"], "voice-cli");

    let remove = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("capability")
        .arg("remove")
        .arg("remote-voice-cli")
        .arg("--domain")
        .arg("voice")
        .output()
        .unwrap();
    assert_success(&remove);
    assert!(!skillspec_home
        .join("capabilities/voice/remote-voice-cli.yml")
        .exists());
}

#[test]
#[cfg(unix)]
fn capability_search_explains_close_candidates_and_local_only_filter() {
    let dir = TempDir::new("capability-ranking");
    let skillspec_home = dir.path().join("skillspec-home");
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(
        &bin_dir.join("voice-cli"),
        "#!/bin/sh\nprintf 'remote voice text to speech voice generation\\n'\n",
    );
    write_executable(
        &bin_dir.join("say"),
        "#!/bin/sh\nprintf 'macOS say text to speech local voice\\n'\n",
    );
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    for (id, command, priority, external) in [
        ("remote-voice-cli", "voice-cli", "80", true),
        ("macos-say", "say", "75", false),
    ] {
        let mut add = Command::new(bin());
        add.env("SKILLSPEC_HOME", &skillspec_home)
            .env("PATH", &path)
            .arg("capability")
            .arg("add")
            .arg(id)
            .arg("--domain")
            .arg("voice")
            .arg("--kind")
            .arg("cli")
            .arg("--command")
            .arg(command)
            .arg("--provides")
            .arg("text_to_speech")
            .arg("--priority")
            .arg(priority)
            .arg("--evidence-command")
            .arg(format!("{command} --help"));
        if external {
            add.arg("--external-service").arg("--may-cost-money");
        }
        let output = add.output().unwrap();
        assert_success(&output);

        let verify = Command::new(bin())
            .env("SKILLSPEC_HOME", &skillspec_home)
            .env("PATH", &path)
            .arg("capability")
            .arg("verify")
            .arg(id)
            .arg("--domain")
            .arg("voice")
            .output()
            .unwrap();
        assert_success(&verify);
    }

    let close = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("capability")
        .arg("search")
        .arg("text_to_speech")
        .arg("--domain")
        .arg("voice")
        .arg("--explain")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&close);
    let close_report = json_stdout(&close);
    assert_eq!(close_report["selected"], Value::Null);
    assert_eq!(
        close_report["ask_policy"]["reason"],
        "top_candidates_within_10_points"
    );
    assert_eq!(close_report["candidates"].as_array().unwrap().len(), 2);

    let local_only = Command::new(bin())
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("capability")
        .arg("search")
        .arg("text_to_speech")
        .arg("--domain")
        .arg("voice")
        .arg("--local-only")
        .output()
        .unwrap();
    assert_success(&local_only);
    let local_report = json_stdout(&local_only);
    assert_eq!(local_report["selected"], "macos-say");
    assert_eq!(local_report["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(local_report["candidates"][0]["id"], "macos-say");
}

#[test]
fn sensemake_teaches_capability_bootstrap_when_spec_uses_it() {
    let dir = TempDir::new("sensemake-capability");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: durable.executor
title: Durable Executor
description: Capability bootstrap fixture.
routes:
  - id: capability_bootstrap
    label: Capability Bootstrap
resources:
  local_capability_seed_store:
    path: ~/.skillspec/capabilities
    role: reference
    used_by:
      - kind: route
        id: capability_bootstrap
commands:
  search_capability_seed_store:
    template: skillspec capability search {{capability_id}} --domain {{domain_id}} --explain --json
tests:
  - name: route assertion
    input: create a voice message
    expect:
      route: capability_bootstrap
"#,
    );

    let output = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("inspect capability bootstrap route"));
    assert!(
        out.contains("skillspec capability search <capability> --domain <domain> --explain --json")
    );
    assert!(out.contains("query ranked local seeds"));
}

#[test]
fn sensemake_teaches_rote_workspace_synthesis_when_spec_uses_it() {
    let dir = TempDir::new("sensemake-rote-workspace");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: skillspec.multiplexer
title: SkillSpec Multiplexer
description: Rote workspace synthesis fixture.
commands:
  synthesize_from_workspace:
    description: Create a draft SkillSpec from durable rote workspace evidence.
    template: skillspec synthesize-from-workspace <workspace> --task '<task>' --out <skill-folder> --observation-approved
    safety: local_write
    requires:
      dependencies: [rote_cli]
dependencies:
  rote_cli:
    kind: cli
    command: rote
"#,
    );

    let output = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("inspect rote workspace synthesis command"));
    assert!(out.contains(
        "skillspec synthesize-from-workspace <workspace> --task '<task>' --out <skill-folder> --observation-approved"
    ));
    assert!(out.contains("synthesize_from_workspace is rote-specific"));
}

#[test]
fn sensemake_teaches_doctor_when_spec_uses_it() {
    let dir = TempDir::new("sensemake-doctor");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: skillspec.multiplexer
title: SkillSpec Multiplexer
description: Doctor fixture.
artifacts:
  doctor_report:
    kind: report
    path: .skillspec/reports/doctor.json
commands:
  doctor_source_skill:
    description: Diagnose prose reliability debt before import.
    template: skillspec doctor <source-skill-folder> --json
    safety: local_read
  import_skill_draft:
    description: Import a staged prose skill.
    template: skillspec import-skill <source-skill-folder> --out <draft-dir>/skill.spec.yml
    safety: local_write
"#,
    );

    let output = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("diagnose source shape and prose reliability debt"));
    assert!(out.contains("skillspec doctor <source-skill-folder-or-repo-uri>"));
    assert!(out.contains("cheap current-skill baseline"));
}

#[test]
fn sensemake_teaches_workspace_authoring_when_spec_uses_it() {
    let dir = TempDir::new("sensemake-workspace");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: skillspec.multiplexer
title: SkillSpec Multiplexer
description: Workspace authoring fixture.
commands:
  workspace_map_source:
    description: Map a multi-skill or plugin-shaped source root.
    template: skillspec workspace map <source-root> --out <build>/skillspec.workspace.yml
    safety: local_write
  workspace_validate_manifest:
    description: Validate the workspace graph.
    template: skillspec workspace validate <build>/skillspec.workspace.yml
    safety: local_read
  workspace_import_packages:
    description: Fanout import the workspace graph.
    template: skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build>
    safety: local_write
"#,
    );

    let output = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("map workspace source root"));
    assert!(out.contains(
        "skillspec workspace map <source-root> --out <build>/skillspec.workspace.yml --summary"
    ));
    assert!(out.contains("fanout import workspace packages"));
    assert!(out.contains(
        "skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build> --summary"
    ));
    assert!(out.contains("multi-skill or plugin-shaped source roots"));
}

#[test]
fn sensemake_teaches_router_index_boundary_when_spec_uses_router_lifecycle() {
    let dir = TempDir::new("sensemake-router-index-boundary");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: skillspec.multiplexer
title: SkillSpec Multiplexer
description: Router lifecycle fixture.
commands:
  router_install:
    description: Install router mode.
    template: skillspec router install --roots <skill-roots> --index <router-index>
    safety: local_write
  router_enable:
    description: Enable router mode.
    template: skillspec router enable --json
    safety: local_write
  router_guard:
    description: Verify router guard readiness.
    template: skillspec router guard --json
    safety: local_write
  status_lifecycle_inventory:
    description: Inspect lifecycle status.
    template: skillspec status --json
    safety: read_only
"#,
    );

    let output = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("direct `skillspec index`"));
    assert!(out.contains("router-specific catalog construction"));
    assert!(
        out.contains("skillspec router index refresh --roots <skill-roots> --index <router-index>")
    );
    assert!(out.contains("skillspec router guard --json"));
    assert!(out.contains("skillspec status --json"));
}

#[test]
fn sensemake_teaches_retire_existing_install_when_spec_uses_it() {
    let dir = TempDir::new("sensemake-retire-existing");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: skillspec.multiplexer
title: SkillSpec Multiplexer
description: Retire existing install fixture.
routes:
  - id: compile_and_install_reviewed_skill
    label: Compile and install reviewed skill
elicitations:
  approve_retire_existing_skill:
    question: Should SkillSpec retire an existing active skill before installing the reviewed replacement?
    required_when:
      - route: compile_and_install_reviewed_skill
    choices:
      - id: retire_existing
        label: Retire existing
        description: Back up and remove the old active skill before installing the replacement.
      - id: stop_before_install
        label: Stop before install
        description: Do not write harness roots until the replacement choice is clear.
commands:
  install_skill:
    description: Install while retiring any old active skill.
    template: skillspec install skill <skill-folder> --target <target> --retire-existing
    safety: local_write
"#,
    );

    let output = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("inspect active-skill retirement gate"));
    assert!(
        out.contains("skillspec install skill <skill-folder> --target <target> --retire-existing")
    );
    assert!(out.contains("ask for retirement approval"));
}

#[test]
fn sensemake_and_query_teach_progressive_navigation() {
    let dir = TempDir::new("sensemake");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let sensemake = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&sensemake);
    let out = stdout(&sensemake);
    assert!(out.contains("SkillSpec map: CLI Rich Spec (cli.rich)"));
    assert!(out.contains("- routes: strategy choices (2)"));
    assert!(out.contains("- rules: steering logic (2)"));
    assert!(out.contains("- states: lifecycle phases (0)"));
    assert!(out.contains("skillspec run-loop"));
    assert!(out.contains("--guide agent"));
    assert!(out.contains("--resume <run-dir>"));
    assert!(out.contains("skillspec decide"));
    assert!(out.contains("skillspec query"));
    assert!(out.contains("skillspec refs"));
    assert!(out.contains("prefer run-loop --guide agent"));
    assert!(out.contains("escalate index -> summary -> full only when needed"));

    let sensemake_json = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&sensemake_json);
    let report = json_stdout(&sensemake_json);
    assert_eq!(report["spec_id"], "cli.rich");
    assert!(report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|section| { section["name"] == "commands" && section["count"] == 1 }));

    let rule = Command::new(bin())
        .arg("query")
        .arg(&spec)
        .arg("rule:browse_rule")
        .arg("--view")
        .arg("summary")
        .output()
        .unwrap();
    assert_success(&rule);
    let rule_out = stdout(&rule);
    assert!(rule_out.contains("target: rule:browse_rule"));
    assert!(rule_out.contains("forbids"));
    assert!(rule_out.contains("native_search_as_answer"));
    assert!(rule_out.contains("after_success"));
    assert!(rule_out.contains("cleanup"));

    let requires = Command::new(bin())
        .arg("query")
        .arg(&spec)
        .arg("command:cleanup.requires")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&requires);
    let requires_report = json_stdout(&requires);
    assert_eq!(requires_report["target"]["kind"], "command");
    assert_eq!(requires_report["target"]["id"], "cleanup");
    assert_eq!(requires_report["target"]["field_path"][0], "requires");
    assert_eq!(requires_report["value"]["dependencies"][0], "shell");

    let forbid = Command::new(bin())
        .arg("query")
        .arg(&spec)
        .arg("rule:browse_rule.forbid")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&forbid);
    let forbid_report = json_stdout(&forbid);
    assert_eq!(forbid_report["value"][0], "native_search_as_answer");

    let route = Command::new(bin())
        .arg("query")
        .arg(&spec)
        .arg("route:browser")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&route);
    let route_report = json_stdout(&route);
    assert_eq!(route_report["value"]["handoff"]["to_skill"], "rote-browse");
    assert_eq!(
        route_report["value"]["handoff"]["boundary"],
        "stop_current_skill"
    );
    assert_eq!(
        route_report["value"]["execution_plan"]["phases"][0]["id"],
        "collect_cli_evidence"
    );

    let refs = Command::new(bin())
        .arg("refs")
        .arg(&spec)
        .arg("rule:browse_rule")
        .output()
        .unwrap();
    assert_success(&refs);
    let refs_out = stdout(&refs);
    assert!(refs_out.contains("prefer -> route: browser"));
    assert!(refs_out.contains("forbid -> forbid: native_search_as_answer"));
    assert!(refs_out.contains("after_success -> command_or_recipe_or_state: cleanup"));

    let route_refs = Command::new(bin())
        .arg("refs")
        .arg(&spec)
        .arg("route:browser")
        .output()
        .unwrap();
    assert_success(&route_refs);
    let route_refs_out = stdout(&route_refs);
    assert!(route_refs_out.contains("handoff.to_skill -> skill: rote-browse"));
    assert!(route_refs_out.contains("execution_plan.owner_skill -> skill: durable-executor"));
    assert!(route_refs_out.contains("execution_plan.route -> route: local"));
    assert!(route_refs_out.contains("execution_plan.jump.to_phase -> phase: browser_handoff"));

    let test = Command::new(bin())
        .arg("query")
        .arg(&spec)
        .arg("test:browse selects browser")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&test);
    let test_report = json_stdout(&test);
    assert_eq!(test_report["target"]["kind"], "test");
    assert_eq!(test_report["target"]["id"], "browse selects browser");
    assert_eq!(test_report["value"]["route"], "browser");
    assert!(test_report["value"]["expect_fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "plan_phases"));
    assert!(test_report["query_hints"]
        .as_array()
        .unwrap()
        .iter()
        .any(|hint| hint.as_str().is_some_and(|text| {
            text.contains("'test:browse selects browser.expect' --view full")
        })));

    let test_expect = Command::new(bin())
        .arg("query")
        .arg(&spec)
        .arg("test:browse selects browser.expect")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&test_expect);
    let test_expect_report = json_stdout(&test_expect);
    assert_eq!(test_expect_report["target"]["field_path"][0], "expect");
    assert_eq!(test_expect_report["value"]["route"], "browser");
    assert_eq!(
        test_expect_report["value"]["matched_rules_exact"][0],
        "browse_rule"
    );

    let test_refs = Command::new(bin())
        .arg("refs")
        .arg(&spec)
        .arg("test:browse selects browser")
        .output()
        .unwrap();
    assert_success(&test_refs);
    let test_refs_out = stdout(&test_refs);
    assert!(test_refs_out.contains("expect.route -> route: browser"));
    assert!(test_refs_out.contains("expect.elicit_exact -> elicitation: mode"));
    assert!(
        test_refs_out.contains("expect.after_success_exact -> command_or_recipe_or_state: cleanup")
    );
    assert!(test_refs_out.contains("expect.matched_rules_exact -> rule: browse_rule"));

    let missing = Command::new(bin())
        .arg("query")
        .arg(&spec)
        .arg("rule:nope")
        .output()
        .unwrap();
    assert_failure(&missing);
    assert!(stderr(&missing).contains("unknown rule id"));
}

#[test]
fn grammar_commands_teach_embedded_porting_workflow() {
    let porting = Command::new(bin())
        .arg("grammar")
        .arg("sensemake")
        .arg("--view")
        .arg("porting")
        .output()
        .unwrap();
    assert_success(&porting);
    let out = stdout(&porting);
    assert!(out.contains("SkillSpec grammar map"));
    assert!(out.contains("embedded: grammar.md"));
    assert!(out.contains("Progressive command sequence:"));
    assert!(out.contains("skillspec grammar sensemake --view porting"));
    assert!(out.contains("skillspec source map <source-skill> --out <draft>/.skillspec/source-map"));
    assert!(out.contains(
        "skillspec source query <draft>/.skillspec/source-map/source-map.json dependencies --view summary"
    ));
    assert!(out.contains(
        "skillspec source stale <draft>/.skillspec/source-map/source-map.json --root <source-skill>"
    ));
    assert!(out.contains(
        "skillspec import-skill <source-skill> --out <draft>/skill.spec.yml --source-map <draft>/.skillspec/source-map/source-map.json"
    ));
    assert!(out.contains("Prose-to-SkillSpec mappings:"));
    assert!(out.contains("Import coverage checklist:"));
    assert!(out.contains("Coverage matrix:"));

    let json = Command::new(bin())
        .arg("grammar")
        .arg("sensemake")
        .arg("--view")
        .arg("summary")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&json);
    let report = json_stdout(&json);
    assert_eq!(report["view"], "summary");
    assert!(report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|section| section["name"] == "routes"));
    assert!(report["prose_mappings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|mapping| mapping["skillspec_construct"]
            == "rules.forbid, rules.prefer, rules.elicit, rules.after_success"));

    let checklist = Command::new(bin())
        .arg("grammar")
        .arg("checklist")
        .arg("--for")
        .arg("import-skill")
        .output()
        .unwrap();
    assert_success(&checklist);
    let checklist_out = stdout(&checklist);
    assert!(checklist_out.contains("SkillSpec porting checklist: import-skill"));
    assert!(checklist_out.contains("inspect dependency ledger"));
    assert!(checklist_out.contains("dependency_count = 0"));
    assert!(checklist_out.contains("Coverage matrix columns:"));
    assert!(checklist_out.contains("Contract quality grades:"));

    let schema = Command::new(bin())
        .arg("grammar")
        .arg("schema")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&schema);
    let schema_report = json_stdout(&schema);
    assert_eq!(
        schema_report["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema_report["title"], "SkillSpec v0");
}
