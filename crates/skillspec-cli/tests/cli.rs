use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_skillspec")
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("skillspec-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn write_durable_source(path: &Path, description_suffix: &str) {
    write_file(
        &path.join("SKILL.md"),
        &format!(
            r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment. {description_suffix}
---
# Durable Executor
"#
        ),
    );
    write_file(
        &path.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: durable.executor
title: Durable Executor
description: Durable executor fixture.
routes:
  - id: durable
    label: Durable
"#,
    );
}

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) {
    write_file(path, content);
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn write_fake_rote(path: &Path) -> std::ffi::OsString {
    let bin_dir = path.join("bin");
    #[cfg(unix)]
    write_executable(&bin_dir.join("rote"), "#!/bin/sh\nexit 0\n");
    #[cfg(windows)]
    write_file(&bin_dir.join("rote.cmd"), "@echo off\r\nexit /B 0\r\n");

    let mut paths = vec![bin_dir];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse stdout as JSON: {error}\nstdout:\n{}",
            stdout(output)
        )
    })
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn collect_yml_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yml_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("yml") {
            files.push(path);
        }
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn assert_snapshot_eq(snapshot_path: &Path, actual: &str) {
    let expected = fs::read_to_string(snapshot_path).unwrap_or_else(|error| {
        panic!(
            "failed to read golden snapshot {}: {error}",
            snapshot_path.display()
        )
    });
    assert_eq!(
        normalize_newlines(&expected),
        normalize_newlines(actual),
        "golden snapshot changed: {}",
        snapshot_path.display()
    );
}

fn rich_spec() -> &'static str {
    r#"
schema: skillspec/v0
id: cli.rich
title: CLI Rich Spec
description: Exercises core CLI behavior.
activation:
  summary: Universal CLI/API/shell router with trace and alignment benefits.
  keywords:
    - git status
    - remote sync
  priority: broad_router
entry:
  prompt: Decide before tools.
  decision_required: true
  supersedes_skills: [browser:browser]
  forbid_before_decision: [node_repl, direct_cli_without_rote_exec]
  tool_boundary:
    default: deny
    allow: [skillspec_cli, local_files]
    permission_required_for: [any_unlisted_tool, any_new_data_source]
routes:
  - id: browser
    label: Browser
    rank: 10
    tool_boundary:
      allow: [rote_exec, rote_browse]
      forbid: [native_web_search]
    handoff:
      to_skill: rote-browse
      boundary: stop_current_skill
      pass_context: [user_intent, evidence_context]
      forbid: [direct_browser_tool_without_rote_browse]
      reason: Browser execution belongs to rote-browse.
    execution_plan:
      mode: ordered
      phases:
        - id: collect_cli_evidence
          owner_skill: durable-executor
          route: local
          requires: [run_cli_only_through_rote_exec]
          forbid: [direct_cli_without_rote_exec]
          tool_boundary:
            allow: [rote_exec]
            forbid: [direct_native_cli]
            permission_required_for: [any_unlisted_cli]
          jumps:
            - when: cli_evidence_missing
              to_phase: browser_handoff
              reason: Browser can collect fallback evidence.
        - id: browser_handoff
          owner_skill: rote-browse
          route: browser
          handoff:
            to_skill: rote-browse
            boundary: stop_current_skill
          forbid: [direct_browser_tool_without_rote_browse]
      reason: Shell evidence must be collected before browser handoff.
  - id: local
    label: Local
    rank: 20
rules:
  - id: browse_rule
    when:
      user_says_any: ["browse"]
    prefer: browser
    forbid: [native_search_as_answer]
    elicit: [mode]
    after_success: [cleanup]
    reason: Browser requests need browser evidence.
  - id: local_rule
    when:
      user_says_any: ["local"]
    prefer: local
    reason: Local requests stay local.
trace:
  mode: event_log
  required: true
  record:
    - input_received
    - rule_matched
    - route_selected
    - outcome_recorded
elicitations:
  mode:
    question: Which mode?
    choices:
      - id: fast
        label: Fast
      - id: careful
        label: Careful
    default: careful
dependencies:
  shell:
    kind: cli
    command: sh
    check:
      command: sh
commands:
  cleanup:
    description: Cleanup evidence.
    template: echo cleanup
    safety: read_only
    requires:
      dependencies: [shell]
tests:
  - name: browse selects browser
    input: browse the dashboard
    expect:
      route: browser
      plan_phases: [collect_cli_evidence, browser_handoff]
      plan_jumps:
        - collect_cli_evidence:cli_evidence_missing->browser_handoff
      forbid_exact: [native_search_as_answer]
      elicit_exact: [mode]
      after_success_exact: [cleanup]
      matched_rules_exact: [browse_rule]
      not_matched_rules: [local_rule]
  - name: local stays local
    input: local file task
    expect:
      route: local
      matched_rules_exact: [local_rule]
      not_forbid: [native_search_as_answer]
"#
}

fn deps_spec() -> &'static str {
    r#"
schema: skillspec/v0
id: cli.deps
title: CLI Dependency Spec
description: Exercises dependency checks.
routes:
  - id: local
    label: Local
dependencies:
  present_cli:
    kind: cli
    command: present-cli
    check:
      command: present-cli
  missing_file:
    kind: file
    path: absent.txt
  pypdf:
    kind: package
commands:
  present:
    template: present-cli --version
    requires:
      dependencies: [present_cli]
  missing:
    template: cat absent.txt
    requires:
      dependencies: [missing_file]
  package_only:
    template: python -c 'import pypdf'
    requires:
      dependencies: [pypdf]
tests:
  - name: route assertion
    input: check locally
    expect:
      route: local
"#
}

fn alignment_spec() -> &'static str {
    r#"
schema: skillspec/v0
id: cli.alignment
title: CLI Alignment Spec
description: Exercises execution alignment proof.
routes:
  - id: adapter_first_cli_fallback
    label: Adapter first, CLI fallback
rules:
  - id: cli_invocations_use_rote_exec
    when:
      user_says_any: ["run", "gh"]
    forbid:
      - direct_cli_without_rote_exec
      - untracked_stdout_scrollback
    after_success:
      - run_cli_only_through_rote_exec
      - report_workspace_evidence_and_token_math
  - id: external_service_tasks_are_adapter_first
    when:
      user_says_any: ["gh", "github"]
    prefer: adapter_first_cli_fallback
    forbid:
      - skipping_adapter_discovery
      - skipping_cli_readiness_check
    after_success:
      - discover_relevant_rote_adapters
      - preflight_cli_fallback
  - id: durable_work_requires_named_workspace
    when:
      user_says_any: ["gh"]
    forbid:
      - anonymous_workspace
    after_success:
      - compute_workspace_stats
  - id: long_noninteractive_jobs_use_background
    when:
      command_likely_long_running: true
closures:
  run_cli_only_through_rote_exec:
    description: Verify CLI invocations used rote exec.
  report_workspace_evidence_and_token_math:
    description: Report workspace evidence and token math.
  discover_relevant_rote_adapters:
    description: Discover relevant adapters before CLI fallback.
  preflight_cli_fallback:
    description: Verify CLI fallback readiness.
  compute_workspace_stats:
    description: Collect workspace stats.
trace:
  mode: event_log
  required: true
tests:
  - name: background phrase matches background rule
    input: run gh PR status as a tracked background process
    expect:
      route: adapter_first_cli_fallback
      matched_rules:
        - cli_invocations_use_rote_exec
        - external_service_tasks_are_adapter_first
        - durable_work_requires_named_workspace
        - long_noninteractive_jobs_use_background
"#
}

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
fn help_lists_trace_align_arguments() {
    let top = Command::new(bin()).arg("--help").output().unwrap();
    assert_success(&top);
    assert!(stdout(&top).contains("trace"));
    assert!(stdout(&top).contains("sensemake"));
    assert!(stdout(&top).contains("query"));
    assert!(stdout(&top).contains("refs"));
    assert!(stdout(&top).contains("doctor"));
    assert!(stdout(&top).contains("source"));
    assert!(stdout(&top).contains("capability"));
    assert!(stdout(&top).contains("visibility"));
    assert!(stdout(&top).contains("router"));
    assert!(stdout(&top).contains("durable-executor"));
    assert!(stdout(&top).contains("synthesize-from-workspace"));

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
    assert!(align_help.contains("<PATH>"));
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

    let source = Command::new(bin())
        .arg("source")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&source);
    let source_help = stdout(&source);
    assert!(source_help.contains("Map and query source packages"));
    assert!(source_help.contains("map"));
    assert!(source_help.contains("query"));
    assert!(source_help.contains("coverage"));
    assert!(source_help.contains("stale"));

    let import_skill = Command::new(bin())
        .arg("import-skill")
        .arg("--help")
        .output()
        .unwrap();
    assert_success(&import_skill);
    let import_help = stdout(&import_skill);
    assert!(import_help.contains("--source-map"));
    assert!(import_help.contains("source-map.json"));

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
    assert!(doctor_help.contains("structural score"));
    assert!(doctor_help.contains("activation-loaded surface percentage"));
    assert!(doctor_help.contains("single skill folder"));
    assert!(doctor_help.contains("sparse checkout"));

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

#[test]
fn skill_router_indexes_routes_and_audits_local_skills() {
    let dir = TempDir::new("skill-router");
    let root = dir.path().join("skills");
    let index = dir.path().join("skill-index.sqlite");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when the user needs to read, extract, OCR, merge, split, or transform PDF documents. Do not use for ordinary markdown files.
metadata:
  short-description: PDF extraction and transformation.
  routing:
    tags: [documents, extraction]
    triggers:
      - extract PDF text
      - OCR scanned PDF
    negative_triggers:
      - markdown
---
# PDF
"#,
    );
    write_file(
        &root.join("pdf/agents/openai.yaml"),
        r#"policy:
  allow_implicit_invocation: false
"#,
    );
    write_file(
        &root.join("pdf/skill.spec.yml"),
        r#"
schema: skillspec/v0
id: router.pdf
title: PDF Router Fixture
description: SkillSpec metadata for PDF routing.
activation:
  summary: Extract tables and text from PDFs.
  keywords: [pdf tables, pdf text]
routes:
  - id: extract
    label: Extract
rules:
  - id: avoid_markdown
    forbid: [markdown]
    reason: Markdown is not a PDF workflow.
tests:
  - name: route assertion
    input: extract pdf text
    expect:
      route: extract
"#,
    );
    write_file(
        &root.join("deploy/SKILL.md"),
        r#"---
name: deploy
description: Use when publishing an application to production environments, release targets, or hosting platforms. Do not use for document extraction.
disable-model-invocation: true
metadata:
  routing:
    tags: [release, hosting]
    triggers: [deploy application]
---
# Deploy
"#,
    );
    write_file(
        &root.join("alternate-pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when annotating simple PDF files and adding comments. Do not use for OCR or table extraction.
metadata:
  routing:
    tags: [annotation]
    triggers: [annotate PDF]
---
# PDF Annotation
"#,
    );
    write_file(
        &root.join("notes/SKILL.md"),
        r#"---
name: notes
description: Helps with notes.
---
# Notes
"#,
    );

    let index_output = Command::new(bin())
        .arg("index")
        .arg("--roots")
        .arg(&root)
        .arg("--out")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&index_output);
    let index_report = json_stdout(&index_output);
    assert_eq!(index_report["skills_indexed"], 4);
    assert!(index.is_file());

    let directory_status = Command::new(bin())
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(dir.path())
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&directory_status);
    let directory_status_report = json_stdout(&directory_status);
    assert_eq!(
        directory_status_report["index"],
        index.to_string_lossy().as_ref()
    );
    assert_eq!(directory_status_report["exists"], true);
    assert_eq!(directory_status_report["stale"], false);

    let route = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(dir.path())
        .arg("--query")
        .arg("extract pdf text from a scanned document")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&route);
    let route_report = json_stdout(&route);
    assert_eq!(route_report["selected"]["name"], "pdf");
    assert!(route_report["selected"]["path"]
        .as_str()
        .unwrap()
        .ends_with("/pdf/SKILL.md"));
    assert_eq!(route_report["selected"]["visibility"], "manual-only");
    assert_eq!(route_report["selected"]["has_skill_spec"], true);
    assert_eq!(
        route_report["elicitation"],
        "execution_mode_direct_or_durable"
    );

    let direct_route = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("deploy application")
        .arg("--execution-mode")
        .arg("direct")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&direct_route);
    let direct_report = json_stdout(&direct_route);
    assert_eq!(direct_report["selected"]["name"], "deploy");
    assert_eq!(direct_report["execution_mode"], "direct");
    assert_eq!(direct_report["elicitation"], Value::Null);

    let audit = Command::new(bin())
        .arg("skills")
        .arg("audit")
        .arg("--roots")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&audit);
    let audit_report = json_stdout(&audit);
    assert_eq!(audit_report["skills"], 4);
    assert_eq!(audit_report["vague_descriptions"], 1);
    assert_eq!(audit_report["missing_negative_boundaries"], 1);
    assert!(audit_report["duplicate_names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "pdf"));
}

#[test]
fn visibility_apply_restore_and_manifest_override_router_index() {
    let dir = TempDir::new("visibility");
    let codex_root = dir.path().join(".codex/skills");
    let claude_root = dir.path().join("repo/.claude/skills");
    let manifest = dir.path().join("visibility-manifest.json");
    let disable_manifest = dir.path().join("disable-manifest.json");
    let index = dir.path().join("skill-index.sqlite");

    write_file(
        &codex_root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting text, tables, and images from PDF documents. Do not use for deployment work.
---
# PDF
"#,
    );
    write_file(
        &claude_root.join("deploy/SKILL.md"),
        r#"---
name: deploy
description: Use when deploying applications to production hosting targets. Do not use for PDF extraction.
---
# Deploy
"#,
    );
    write_file(
        &codex_root.join("durable-executor/SKILL.md"),
        r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#,
    );

    let plan = Command::new(bin())
        .arg("visibility")
        .arg("plan")
        .arg("--roots")
        .arg(&codex_root)
        .arg(&claude_root)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&plan);
    let plan_report = json_stdout(&plan);
    assert_eq!(plan_report["changes"].as_array().unwrap().len(), 2);
    assert!(plan_report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|change| change["after_visibility"] == "manual-only"));
    assert!(!plan_report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|change| change["skill"] == "durable-executor"));

    let apply = Command::new(bin())
        .arg("visibility")
        .arg("apply")
        .arg("--roots")
        .arg(&codex_root)
        .arg(&claude_root)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&apply);
    let apply_report = json_stdout(&apply);
    assert_eq!(apply_report["changes"].as_array().unwrap().len(), 2);
    assert!(manifest.is_file());
    assert!(
        fs::read_to_string(codex_root.join("pdf/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );
    let claude_settings: Value = serde_json::from_str(
        &fs::read_to_string(dir.path().join("repo/.claude/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        claude_settings["skillOverrides"]["deploy"],
        "user-invocable-only"
    );
    assert!(!codex_root
        .join("durable-executor/agents/openai.yaml")
        .exists());

    let restore = Command::new(bin())
        .arg("visibility")
        .arg("restore")
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&restore);
    assert!(!codex_root.join("pdf/agents/openai.yaml").exists());
    assert!(!dir.path().join("repo/.claude/settings.json").exists());

    let disable = Command::new(bin())
        .arg("skills")
        .arg("disable")
        .arg("pdf")
        .arg("--roots")
        .arg(&codex_root)
        .arg("--manifest")
        .arg(&disable_manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable);
    let disable_report = json_stdout(&disable);
    assert_eq!(disable_report["changes"][0]["after_visibility"], "off");

    let index_output = Command::new(bin())
        .arg("index")
        .arg("--roots")
        .arg(&codex_root)
        .arg("--out")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&disable_manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&index_output);

    let route = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("extract pdf text")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&route);
    let route_report = json_stdout(&route);
    assert_eq!(route_report["selected"], Value::Null);
    assert!(route_report["candidates"].as_array().unwrap().is_empty());
}

#[test]
fn router_install_hooks_install_skill_and_uninstall_restores_visibility() {
    let dir = TempDir::new("router-lifecycle");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");
    let source = dir.path().join("note-source");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#,
    );
    write_file(
        &root.join("durable-executor/SKILL.md"),
        r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    let install_report = json_stdout(&install_router);
    assert_eq!(install_report["router_skill_status"], "installed");
    assert_eq!(install_report["durable_executor"]["present"], true);
    assert_eq!(
        install_report["visibility"]["changes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(install_report["preparedness"]["ready"], true);
    assert_eq!(install_report["preparedness"]["status_checked"], true);
    assert_eq!(install_report["preparedness"]["index_stale"], false);
    assert_eq!(install_report["preparedness"]["indexed_skills"], 3);
    assert!(root.join("skill-router/SKILL.md").is_file());
    assert!(root.join("skill-router/skill.spec.yml").is_file());
    assert!(root
        .join("skill-router/.skillspec-router-managed")
        .is_file());
    let router_skill = fs::read_to_string(root.join("skill-router/SKILL.md")).unwrap();
    assert!(router_skill.contains("skill.spec.yml"));
    assert!(router_skill.contains("explicit-only"));
    assert!(router_skill.contains("durable-executor"));
    assert!(!router_skill.contains("visible discovery surface"));
    let router_spec = fs::read_to_string(root.join("skill-router/skill.spec.yml")).unwrap();
    assert!(router_spec.contains("schema: skillspec/v0"));
    assert!(!router_spec.contains("--router-root"));
    let validate_router_spec = Command::new(bin())
        .arg("validate")
        .arg(root.join("skill-router/skill.spec.yml"))
        .output()
        .unwrap();
    assert_success(&validate_router_spec);
    assert!(root.join("pdf/agents/openai.yaml").is_file());
    assert!(!root.join("skill-router/agents/openai.yaml").exists());
    assert!(!root.join("durable-executor/agents/openai.yaml").exists());
    assert!(fs::read_to_string(root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!fs::read_to_string(root.join("skill-router/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!fs::read_to_string(root.join("durable-executor/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(index.is_file());
    assert!(skillspec_home.join("router/config.json").is_file());

    let clean_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&clean_status);
    let clean_report = json_stdout(&clean_status);
    assert_eq!(clean_report["stale"], false);
    assert_eq!(clean_report["indexed_skills"], 3);

    write_file(
        &source.join("SKILL.md"),
        r#"---
name: notes
description: Use when taking structured notes and summarizing meeting action items. Do not use for PDF extraction.
---
# Notes
"#,
    );
    write_file(
        &source.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: notes.skill
title: Notes
description: Notes fixture.
routes:
  - id: local
    label: Local
"#,
    );
    let install_skill = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("install")
        .arg("skill")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--name")
        .arg("notes")
        .output()
        .unwrap();
    assert_success(&install_skill);
    assert!(root.join("notes/agents/openai.yaml").is_file());

    let refreshed_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&refreshed_status);
    let refreshed_report = json_stdout(&refreshed_status);
    assert_eq!(refreshed_report["stale"], false);
    assert_eq!(refreshed_report["indexed_skills"], 4);
    assert!(fs::read_to_string(root.join("notes/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));

    let route_notes = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("summarize meeting action items as notes")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&route_notes);
    let route_report = json_stdout(&route_notes);
    assert_eq!(route_report["selected"]["name"], "notes");

    let uninstall_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("uninstall")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&uninstall_router);
    let uninstall_report = json_stdout(&uninstall_router);
    assert_eq!(uninstall_report["router_skill_status"], "removed");
    assert_eq!(uninstall_report["index_removed"], true);
    assert!(!root.join("skill-router").exists());
    assert!(!index.exists());
    assert!(!skillspec_home.join("router/config.json").exists());
    assert!(!root.join("pdf/agents/openai.yaml").exists());
    assert!(!root.join("durable-executor/agents/openai.yaml").exists());
    assert!(!root.join("notes/agents/openai.yaml").exists());
    assert!(!fs::read_to_string(root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!fs::read_to_string(root.join("notes/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
}

#[test]
#[cfg(unix)]
fn router_install_tracks_symlinked_harness_roots_and_uninstalls_all() {
    let dir = TempDir::new("router-symlink-roots");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = home.join(".codex/skills");
    let vendor_root = home.join(".vendor/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");

    fs::create_dir_all(&agents_root).unwrap();
    fs::create_dir_all(codex_root.parent().unwrap()).unwrap();
    fs::create_dir_all(vendor_root.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&agents_root, &codex_root).unwrap();
    std::os::unix::fs::symlink(&agents_root, &vendor_root).unwrap();

    write_file(
        &agents_root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#,
    );
    write_file(
        &agents_root.join("durable-executor/SKILL.md"),
        r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg(&vendor_root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    let install_report = json_stdout(&install_router);
    assert_eq!(install_report["router_skill_status"], "installed");
    assert_eq!(
        install_report["router_skill_dirs"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        install_report["router_skill_reports"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    for root in [&agents_root, &codex_root, &vendor_root] {
        assert!(root.join("skill-router/SKILL.md").is_file());
        assert!(root.join("skill-router/skill.spec.yml").is_file());
        assert!(root
            .join("skill-router/.skillspec-router-managed")
            .is_file());
    }

    let config = fs::read_to_string(skillspec_home.join("router/config.json")).unwrap();
    let config_json: Value = serde_json::from_str(&config).unwrap();
    assert_eq!(config_json["roots"].as_array().unwrap().len(), 3);
    assert_eq!(
        config_json["router_skill_dirs"].as_array().unwrap().len(),
        3
    );

    let uninstall_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("uninstall")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&uninstall_router);
    let uninstall_report = json_stdout(&uninstall_router);
    assert_eq!(uninstall_report["router_skill_status"], "removed");
    assert_eq!(
        uninstall_report["router_skill_reports"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    for root in [&agents_root, &codex_root, &vendor_root] {
        assert!(!root.join("skill-router").exists());
    }
    assert!(!index.exists());
    assert!(!skillspec_home.join("router/config.json").exists());
}

#[test]
fn router_install_handles_duplicate_skill_names_across_roots() {
    let dir = TempDir::new("router-duplicate-names");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = home.join(".codex/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");

    write_file(
        &agents_root.join("rote/SKILL.md"),
        r#"---
name: rote
description: Use rote before tool calls from the shared agents root.
---
# Rote
"#,
    );
    write_file(
        &codex_root.join("rote/SKILL.md"),
        r#"---
name: rote
description: Use rote before tool calls from the Codex root.
---
# Rote
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    let install_report = json_stdout(&install_router);
    assert_eq!(install_report["preparedness"]["ready"], true);
    assert_eq!(install_report["preparedness"]["index_stale"], false);
    assert_eq!(install_report["preparedness"]["indexed_skills"], 4);
    assert_eq!(install_report["preparedness"]["discovered_skills"], 4);
    assert!(skillspec_home.join("router/config.json").is_file());

    let status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&status);
    let status_report = json_stdout(&status);
    assert_eq!(status_report["stale"], false);
    assert_eq!(status_report["indexed_skills"], 4);
    assert_eq!(status_report["discovered_skills"], 4);
    assert!(status_report["new_skills"].as_array().unwrap().is_empty());
    assert!(status_report["changed_skills"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(status_report["missing_skills"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
#[cfg(unix)]
fn router_update_backs_up_and_repairs_all_recorded_router_roots() {
    let dir = TempDir::new("router-update");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = home.join(".codex/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");
    let backup_dir = skillspec_home.join("router/update-backup");

    fs::create_dir_all(&agents_root).unwrap();
    fs::create_dir_all(codex_root.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&agents_root, &codex_root).unwrap();

    write_file(
        &agents_root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    write_file(
        &agents_root.join("skill-router/SKILL.md"),
        r#"---
name: skill-router
description: stale router text
---
# Skill Router

Use this skill as the visible discovery surface for large local skill libraries.
"#,
    );
    fs::remove_file(codex_root.join("skill-router/skill.spec.yml")).unwrap();

    let update_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("update")
        .arg("--backup-dir")
        .arg(&backup_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&update_router);
    let update_report = json_stdout(&update_router);
    assert_eq!(
        update_report["router_skill_reports"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        update_report["backup"]["path"].as_str().unwrap(),
        backup_dir.to_string_lossy()
    );
    assert!(update_report["restart_warning"]
        .as_str()
        .unwrap()
        .contains("Restart active"));
    assert!(backup_dir.join("backup.json").is_file());
    assert!(backup_dir.join("router-skill-0/SKILL.md").is_file());
    assert!(
        fs::read_to_string(backup_dir.join("router-skill-0/SKILL.md"))
            .unwrap()
            .contains("visible discovery surface")
    );

    for root in [&agents_root, &codex_root] {
        let router_skill = fs::read_to_string(root.join("skill-router/SKILL.md")).unwrap();
        assert!(router_skill.contains("router mode is enabled"));
        assert!(router_skill.contains("explicit-only"));
        assert!(!router_skill.contains("visible discovery surface"));
        assert!(root.join("skill-router/skill.spec.yml").is_file());
        assert!(root
            .join("skill-router/.skillspec-router-managed")
            .is_file());
    }
    let config = fs::read_to_string(skillspec_home.join("router/config.json")).unwrap();
    let config_json: Value = serde_json::from_str(&config).unwrap();
    assert_eq!(
        config_json["router_skill_dirs"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn router_index_refresh_repairs_out_of_band_skills_and_advises_conversion() {
    let dir = TempDir::new("router-out-of-band");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#,
    );
    write_file(
        &root.join("durable-executor/SKILL.md"),
        r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    write_file(
        &root.join("legacy-prose/SKILL.md"),
        r#"---
name: legacy-prose
description: Use when a legacy prose-only workflow should be routed. Do not use for PDF extraction.
---
# Legacy Prose
"#,
    );
    write_file(
        &root.join("spec-backed/SKILL.md"),
        r#"---
name: spec-backed
description: Use when a SkillSpec-backed out-of-band workflow should be routed. Do not use for PDF extraction.
---
# Spec Backed
"#,
    );
    write_file(
        &root.join("spec-backed/skill.spec.yml"),
        r#"
schema: skillspec/v0
id: spec.backed
title: Spec Backed
description: Fixture for out-of-band SkillSpec-backed routing.
routes:
  - id: local
    label: Local
"#,
    );

    let stale_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&stale_status);
    let stale_report = json_stdout(&stale_status);
    assert_eq!(stale_report["stale"], true);
    let new_skills = stale_report["new_skills"].as_array().unwrap();
    let prose = new_skills
        .iter()
        .find(|entry| entry["name"] == "legacy-prose")
        .unwrap();
    assert_eq!(prose["has_skill_spec"], false);
    assert!(prose["advice"].as_str().unwrap().contains("import-skill"));
    let spec_backed = new_skills
        .iter()
        .find(|entry| entry["name"] == "spec-backed")
        .unwrap();
    assert_eq!(spec_backed["has_skill_spec"], true);
    assert!(spec_backed["advice"]
        .as_str()
        .unwrap()
        .contains("SkillSpec-backed"));
    assert!(stale_report["advice"]
        .as_array()
        .unwrap()
        .iter()
        .any(|advice| advice
            .as_str()
            .is_some_and(|text| text.contains("router index refresh"))));
    assert!(!root.join("legacy-prose/agents/openai.yaml").exists());
    assert!(!fs::read_to_string(root.join("legacy-prose/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));

    let refresh = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("refresh")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&refresh);
    let refresh_report = json_stdout(&refresh);
    assert_eq!(refresh_report["router_config_present"], true);
    assert_eq!(refresh_report["status_before"]["stale"], true);
    assert_eq!(refresh_report["preparedness"]["ready"], true);
    assert_eq!(refresh_report["index_report"]["skills_indexed"], 5);
    assert!(refresh_report["advice"]
        .as_array()
        .unwrap()
        .iter()
        .any(|advice| advice
            .as_str()
            .is_some_and(|text| text.contains("import-skill"))));
    assert!(root.join("legacy-prose/agents/openai.yaml").is_file());
    assert!(root.join("spec-backed/agents/openai.yaml").is_file());
    assert!(fs::read_to_string(root.join("legacy-prose/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(fs::read_to_string(root.join("spec-backed/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!root.join("durable-executor/agents/openai.yaml").exists());
    assert!(!fs::read_to_string(root.join("durable-executor/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));

    let clean_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&clean_status);
    let clean_report = json_stdout(&clean_status);
    assert_eq!(clean_report["stale"], false);
    assert_eq!(clean_report["indexed_skills"], 5);
    assert!(clean_report["new_skills"].as_array().unwrap().is_empty());
}

#[test]
fn router_install_reports_missing_optional_durable_executor() {
    let dir = TempDir::new("router-missing-durable");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images.
---
# PDF
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    let install_report = json_stdout(&install_router);
    assert_eq!(install_report["durable_executor"]["present"], false);
    assert_eq!(install_report["preparedness"]["ready"], true);
    assert!(install_report["durable_executor"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning
            .as_str()
            .is_some_and(|text| text.contains("durable first-hop is unavailable"))));
    assert!(root.join("skill-router/SKILL.md").is_file());
    assert!(root.join("skill-router/skill.spec.yml").is_file());
    assert!(!root.join("skill-router/agents/openai.yaml").exists());
    assert!(fs::read_to_string(root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!fs::read_to_string(root.join("skill-router/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!root.join("durable-executor").exists());
}

#[test]
fn router_disable_and_enable_toggle_visibility_and_reindex_all_roots() {
    let dir = TempDir::new("router-enable-disable");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = home.join(".codex/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");

    write_file(
        &agents_root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text.
---
# PDF
"#,
    );
    write_file(
        &codex_root.join("csv/SKILL.md"),
        r#"---
name: csv
description: Use when working with CSV files.
---
# CSV
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    assert!(fs::read_to_string(agents_root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(
        fs::read_to_string(codex_root.join("csv/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );
    assert!(
        !fs::read_to_string(agents_root.join("skill-router/SKILL.md"))
            .unwrap()
            .contains("disable-model-invocation: true")
    );

    let disable_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable_router);
    let disable_report = json_stdout(&disable_router);
    assert_eq!(disable_report["enabled"], false);
    assert!(disable_report["index_report"].is_null());
    assert!(fs::read_to_string(agents_root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: false"));
    assert!(
        fs::read_to_string(agents_root.join("pdf/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: true")
    );
    assert!(
        fs::read_to_string(codex_root.join("csv/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: true")
    );
    assert!(
        fs::read_to_string(agents_root.join("skill-router/SKILL.md"))
            .unwrap()
            .contains("disable-model-invocation: true")
    );
    assert!(
        fs::read_to_string(codex_root.join("skill-router/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );

    write_file(
        &codex_root.join("markdown/SKILL.md"),
        r#"---
name: markdown
description: Use when editing markdown.
---
# Markdown
"#,
    );

    let enable_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&enable_router);
    let enable_report = json_stdout(&enable_router);
    assert_eq!(enable_report["enabled"], true);
    assert_eq!(enable_report["preparedness"]["ready"], true);
    assert_eq!(enable_report["index_report"]["skills_indexed"], 5);
    assert!(fs::read_to_string(agents_root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(
        fs::read_to_string(codex_root.join("markdown/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );
    assert!(
        !fs::read_to_string(agents_root.join("skill-router/SKILL.md"))
            .unwrap()
            .contains("disable-model-invocation: true")
    );
    assert!(
        fs::read_to_string(codex_root.join("skill-router/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: true")
    );
}

#[test]
fn durable_executor_lifecycle_installs_updates_and_deletes_managed_dirs() {
    let dir = TempDir::new("durable-lifecycle");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let root = home.join(".agents/skills");
    let source = dir.path().join("source");
    write_durable_source(&source, "initial");

    let install = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install);
    let install_report = json_stdout(&install);
    assert_eq!(install_report["skill_name"], "durable-executor");
    assert_eq!(install_report["rote_preflight"]["present"], true);
    assert_eq!(install_report["managed_installs"][0]["status"], "installed");
    assert!(root.join("durable-executor/SKILL.md").is_file());
    assert!(root
        .join("durable-executor/.skillspec-durable-executor-managed")
        .is_file());
    assert!(skillspec_home
        .join("durable-executor/config.json")
        .is_file());

    fs::remove_file(root.join("durable-executor/.skillspec-durable-executor-managed")).unwrap();
    let unsafe_update = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("update")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&unsafe_update);
    assert!(stderr(&unsafe_update).contains("managed marker"));
    write_file(
        &root.join("durable-executor/.skillspec-durable-executor-managed"),
        "schema: skillspec/durable-executor-managed/v1\n",
    );

    write_durable_source(&source, "updated");
    let update = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("update")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&update);
    let update_report = json_stdout(&update);
    assert_eq!(update_report["rote_preflight"]["present"], true);
    assert_eq!(update_report["managed_installs"][0]["status"], "updated");
    assert!(update_report["backup"]["path"].as_str().is_some());
    assert!(fs::read_to_string(root.join("durable-executor/SKILL.md"))
        .unwrap()
        .contains("updated"));
    assert!(root
        .join("durable-executor/.skillspec-durable-executor-managed")
        .is_file());

    fs::remove_file(root.join("durable-executor/.skillspec-durable-executor-managed")).unwrap();
    let unsafe_delete = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("durable-executor")
        .arg("delete")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&unsafe_delete);
    assert!(stderr(&unsafe_delete).contains("managed marker"));
    assert!(root.join("durable-executor").exists());

    write_file(
        &root.join("durable-executor/.skillspec-durable-executor-managed"),
        "schema: skillspec/durable-executor-managed/v1\n",
    );
    let delete = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("durable-executor")
        .arg("delete")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&delete);
    let delete_report = json_stdout(&delete);
    assert_eq!(delete_report["managed_installs"][0]["status"], "removed");
    assert_eq!(delete_report["config_removed"], true);
    assert!(!root.join("durable-executor").exists());
    assert!(!skillspec_home.join("durable-executor/config.json").exists());
}

#[test]
fn durable_executor_disable_and_enable_toggle_implicit_invocation() {
    let dir = TempDir::new("durable-enable-disable");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let source = dir.path().join("source");
    let agents_install = home.join(".agents/skills/durable-executor");
    let codex_install = home.join(".codex/skills/durable-executor");
    write_durable_source(&source, "toggle visibility");

    let install = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--target")
        .arg("codex")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install);

    let disable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("durable-executor")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable);
    let disable_report = json_stdout(&disable);
    assert_eq!(disable_report["enabled"], false);
    assert!(fs::read_to_string(agents_install.join("SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(
        fs::read_to_string(agents_install.join("agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );
    assert!(fs::read_to_string(codex_install.join("agents/openai.yaml"))
        .unwrap()
        .contains("allow_implicit_invocation: false"));

    let config_path = skillspec_home.join("durable-executor/config.json");
    let config: Value = serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(config["enabled"], false);

    let enable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&enable);
    let enable_report = json_stdout(&enable);
    assert_eq!(enable_report["enabled"], true);
    assert!(fs::read_to_string(agents_install.join("SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: false"));
    assert!(
        fs::read_to_string(agents_install.join("agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: true")
    );
    assert!(fs::read_to_string(codex_install.join("agents/openai.yaml"))
        .unwrap()
        .contains("allow_implicit_invocation: true"));
}

#[test]
fn durable_executor_install_requires_rote_on_path() {
    let dir = TempDir::new("durable-requires-rote");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let empty_path = dir.path().join("empty-path");
    fs::create_dir_all(&empty_path).unwrap();
    let root = home.join(".agents/skills");
    let source = dir.path().join("source");
    write_durable_source(&source, "missing rote");

    let install = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &empty_path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&install);
    assert!(stderr(&install).contains("requires `rote` on PATH"));
    assert!(!root.join("durable-executor").exists());
    assert!(!skillspec_home.join("durable-executor/config.json").exists());

    let dry_run = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &empty_path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert_success(&dry_run);
    let dry_run_report = json_stdout(&dry_run);
    assert_eq!(dry_run_report["rote_preflight"]["present"], false);
    assert!(!root.join("durable-executor").exists());
    assert!(!skillspec_home.join("durable-executor/config.json").exists());
}

#[test]
fn durable_executor_enable_requires_rote_on_path() {
    let dir = TempDir::new("durable-enable-requires-rote");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let empty_path = dir.path().join("empty-path");
    fs::create_dir_all(&empty_path).unwrap();
    let source = dir.path().join("source");
    let install_dir = home.join(".agents/skills/durable-executor");
    write_durable_source(&source, "enable missing rote");

    let install = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install);

    let disable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &empty_path)
        .arg("durable-executor")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable);

    let enable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &empty_path)
        .arg("durable-executor")
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&enable);
    assert!(stderr(&enable).contains("requires `rote` on PATH"));
    assert!(fs::read_to_string(install_dir.join("agents/openai.yaml"))
        .unwrap()
        .contains("allow_implicit_invocation: false"));

    let config_path = skillspec_home.join("durable-executor/config.json");
    let config: Value = serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(config["enabled"], false);
}

#[test]
fn status_reports_lifecycle_roots_index_and_skill_inventory() {
    let dir = TempDir::new("status-lifecycle-inventory");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let durable_source = dir.path().join("durable-source");
    write_durable_source(&durable_source, "status inventory");
    write_file(
        &root.join("alpha/SKILL.md"),
        r#"---
name: alpha
description: Alpha SkillSpec-backed skill.
---
# Alpha
"#,
    );
    write_file(
        &root.join("alpha/skill.spec.yml"),
        r#"
schema: skillspec/v0
id: alpha
title: Alpha
description: Alpha SkillSpec-backed skill.
routes:
  - id: alpha
    label: Alpha
"#,
    );
    write_file(
        &root.join("legacy/SKILL.md"),
        r#"---
name: legacy
description: Legacy prose-only skill.
---
# Legacy
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    let install_durable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&durable_source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_durable);

    let disable_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable_router);

    let disable_durable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("durable-executor")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable_durable);

    let status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("status")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&status);
    let report = json_stdout(&status);
    assert_eq!(report["router"]["installed"], true);
    assert_eq!(report["router"]["enabled"], false);
    assert_eq!(report["router"]["disabled"], true);
    assert_eq!(report["durable_executor"]["installed"], true);
    assert_eq!(report["durable_executor"]["enabled"], false);
    assert_eq!(report["durable_executor"]["disabled"], true);
    assert_eq!(report["roots"]["scan_source"], "router_config");
    assert_eq!(report["roots"]["scanned_count"], 1);
    assert!(report["roots"]["supported_count"].as_u64().unwrap() >= 2);
    assert_eq!(report["skills"]["legacy_count"], 1);
    assert!(report["skills"]["skillspec_backed_count"].as_u64().unwrap() >= 3);
    assert!(report["skills"]["legacy"]
        .as_array()
        .unwrap()
        .iter()
        .any(|skill| skill["name"] == "legacy"));
    assert!(report["skills"]["skillspec_backed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|skill| skill["name"] == "alpha"));
    assert_eq!(report["router"]["index_status"]["exists"], true);
    assert_eq!(
        report["router"]["index_status"]["discovered_skills"],
        report["skills"]["total"]
    );
}

#[test]
fn durable_executor_install_refreshes_router_and_remains_implicit() {
    let dir = TempDir::new("durable-router-hook");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let source = dir.path().join("durable-source");
    write_durable_source(&source, "router hook");
    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images.
---
# PDF
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    let install_durable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_durable);
    let durable_report = json_stdout(&install_durable);
    assert!(durable_report["router_hook"].is_object());
    assert!(root.join("durable-executor/SKILL.md").is_file());
    assert!(!root.join("durable-executor/agents/openai.yaml").exists());

    let status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&status);
    let status_report = json_stdout(&status);
    assert_eq!(status_report["stale"], false);
    assert_eq!(status_report["indexed_skills"], 3);
}

#[test]
fn router_install_rejects_invalid_router_name() {
    let dir = TempDir::new("router-invalid-name");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images.
---
# PDF
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--router-name")
        .arg("../skill-router")
        .output()
        .unwrap();
    assert_failure(&install_router);
    assert!(stderr(&install_router).contains("router name must start"));
    assert!(!home.join(".agents/skill-router").exists());
}

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
    template: skillspec synthesize-from-workspace <workspace> --task '<task>' --out <skill-folder>
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
        "skillspec synthesize-from-workspace <workspace> --task '<task>' --out <skill-folder>"
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
    assert!(out.contains("diagnose prose reliability debt"));
    assert!(out.contains("skillspec doctor <source-skill-folder-or-uri> --json"));
    assert!(out.contains("run doctor before import"));
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
    assert!(out.contains("skillspec decide"));
    assert!(out.contains("skillspec query"));
    assert!(out.contains("skillspec refs"));
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
fn deps_check_distinguishes_missing_deferred_and_command_scope() {
    let dir = TempDir::new("deps");
    let spec = dir.path().join("skill.spec.yml");
    let cli_dir = dir.path().join("bin");
    write_file(&cli_dir.join("present-cli"), "");
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

#[test]
fn compile_targets_render_loader_and_full_markdown() {
    let dir = TempDir::new("compile");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let loader = Command::new(bin())
        .arg("compile")
        .arg(&spec)
        .arg("--target")
        .arg("codex-skill")
        .output()
        .unwrap();
    assert_success(&loader);
    let loader_out = stdout(&loader);
    assert!(loader_out.contains(
        "description: \"Universal CLI/API/shell router with trace and alignment benefits."
    ));
    assert!(loader_out.contains("thin loader"));
    assert!(loader_out.contains("## Entry Gate"));
    assert!(loader_out.contains("skillspec act ./skill.spec.yml --input='<user task>'"));
    assert!(loader_out.contains("current-route action checklist"));
    assert!(loader_out.contains("selected route and matched rules in the checklist override"));
    assert!(loader_out.contains("## Authoring And Revision Contract"));
    assert!(loader_out.contains("skillspec grammar sensemake --view porting"));
    assert!(loader_out.contains("skillspec grammar checklist --for import-skill"));
    assert!(loader_out.contains("## Durable Handoff Contract"));
    assert!(loader_out.contains("Forbidden before the decision"));
    assert!(loader_out.contains("skill.spec.yml"));
    assert!(!loader_out.contains("## Rules"));

    let markdown = Command::new(bin())
        .arg("compile")
        .arg(&spec)
        .arg("--target")
        .arg("markdown")
        .output()
        .unwrap();
    assert_success(&markdown);
    let markdown_out = stdout(&markdown);
    assert!(markdown_out.contains("## Authoring And Revision Contract"));
    assert!(markdown_out.contains("skillspec act <skill-folder>/skill.spec.yml"));
    assert!(markdown_out.contains("OODA loop for the selected route"));
    assert!(markdown_out.contains("## Rules"));
    assert!(markdown_out.contains("## Scenario Tests"));
    assert!(markdown_out.contains("browse_rule"));
}

#[test]
fn import_skill_creates_valid_structured_draft() {
    let dir = TempDir::new("import");
    let skill_dir = dir.path().join("source-skill");
    let out = dir.path().join("skill.spec.yml");
    write_file(
        &skill_dir.join("SKILL.md"),
        r#"# Imported Skill

Always validate inputs before writing files.

```bash
echo "hello"
```
"#,
    );
    write_file(
        &skill_dir.join("reference.md"),
        r#"# Reference

```python
print("hello")
```
"#,
    );

    let import = Command::new(bin())
        .arg("import-skill")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&import);
    assert!(out.is_file());
    assert!(dir.path().join("source/SKILL_md.old").is_file());
    assert!(!dir.path().join("source/SKILL.md").is_file());
    let import_out = stdout(&import);
    assert!(import_out.contains("review note"));
    assert!(import_out.contains("skillspec grammar sensemake --view porting"));
    assert!(import_out.contains("skillspec sensemake"));
    assert!(import_out.contains("skillspec grammar checklist --for import-skill"));
    assert!(import_out.contains("deps ledger: wrote deps.toml"));
    assert!(import_out.contains("byte-empty ledger is not"));

    let validate = Command::new(bin())
        .arg("validate")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&validate);

    let content = fs::read_to_string(&out).unwrap();
    assert!(content.contains("review_required"));
    assert!(content.contains("imports:"));
    assert!(content.contains("reference:"));
    assert!(content.contains("import: reference"));
    assert!(content.contains("command_block_1"));
    assert!(content.contains("python3"));
    assert!(content.contains("dependency_ledger"));
    assert!(content.contains("path: source/SKILL_md.old"));

    let draft_sensemake = Command::new(bin())
        .arg("sensemake")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&draft_sensemake);
    let draft_sensemake_out = stdout(&draft_sensemake);
    assert!(draft_sensemake_out.contains("inspect dependency ledger"));
    assert!(draft_sensemake_out.contains("dependency_count = 0 is valid"));

    let ledger = dir.path().join("deps.toml");
    assert!(ledger.is_file());
    let ledger_content = fs::read_to_string(&ledger).unwrap();
    assert!(ledger_content.contains("schema_version = 1"));
    assert!(ledger_content.contains("dependency_count = "));
    assert!(ledger_content.contains("id = \"python3\""));

    let deps_check = Command::new(bin())
        .arg("deps")
        .arg("check")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&deps_check);
    assert!(stdout(&deps_check).contains("deps.toml exists"));
}

#[test]
fn source_map_guides_progressive_import_and_stale_gate() {
    let dir = TempDir::new("source-map");
    let skill_dir = dir.path().join("source-skill");
    let map_dir = dir.path().join("source-map");
    let out = dir.path().join("draft").join("skill.spec.yml");
    write_file(
        &skill_dir.join("SKILL.md"),
        r#"---
name: progressive-skill
description: Use when a large Markdown skill must be mapped before import.
---

# Progressive Skill

Always inspect dependencies before proof.

See [reference](reference.md).

```python
import json
import pypdf
from reportlab.pdfgen import canvas
```

```ts
import { chromium } from "playwright";
```
"#,
    );
    write_file(
        &skill_dir.join("reference.md"),
        "# Reference\n\nNever skip referenced local files.\n",
    );

    let map = Command::new(bin())
        .arg("source")
        .arg("map")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&map_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["files"], 2);
    assert!(map_dir.join("source-map.json").is_file());
    assert!(map_dir.join("source-map.md").is_file());

    let nodes = Command::new(bin())
        .arg("source")
        .arg("query")
        .arg(map_dir.join("source-map.json"))
        .arg("nodes")
        .arg("--view")
        .arg("index")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&nodes);
    let nodes = json_stdout(&nodes);
    let nodes = nodes.as_array().unwrap();
    assert!(nodes
        .iter()
        .any(|node| node["id"] == "frontmatter:skill-md"));
    assert!(nodes
        .iter()
        .any(|node| node["id"] == "heading:skill-md.progressive-skill"));
    assert!(nodes.iter().any(|node| node["kind"] == "code"));

    let deps = Command::new(bin())
        .arg("source")
        .arg("query")
        .arg(map_dir.join("source-map.json"))
        .arg("dependencies")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&deps);
    let deps = json_stdout(&deps);
    let deps_text = serde_json::to_string(&deps).unwrap();
    assert!(deps_text.contains("pypdf"));
    assert!(deps_text.contains("reportlab"));
    assert!(deps_text.contains("playwright"));
    assert!(
        !deps.as_array().unwrap().iter().any(|entry| entry["signals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|signal| signal == "json"))
    );

    let coverage = Command::new(bin())
        .arg("source")
        .arg("coverage")
        .arg(map_dir.join("source-map.json"))
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&coverage);
    let coverage = json_stdout(&coverage);
    assert!(coverage["total_nodes"].as_u64().unwrap() > 0);
    assert!(coverage["review_required"].as_u64().unwrap() > 0);

    let import = Command::new(bin())
        .arg("import-skill")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&out)
        .arg("--source-map")
        .arg(map_dir.join("source-map.json"))
        .output()
        .unwrap();
    assert_success(&import);
    assert!(out.is_file());

    let stale_fresh = Command::new(bin())
        .arg("source")
        .arg("stale")
        .arg(map_dir.join("source-map.json"))
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&stale_fresh);
    assert_eq!(json_stdout(&stale_fresh)["ok"], true);

    write_file(
        &skill_dir.join("SKILL.md"),
        "# Progressive Skill\n\nChanged after source map.\n",
    );

    let stale = Command::new(bin())
        .arg("source")
        .arg("stale")
        .arg(map_dir.join("source-map.json"))
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&stale);
    assert_eq!(json_stdout(&stale)["ok"], false);

    let stale_import = Command::new(bin())
        .arg("import-skill")
        .arg(&skill_dir)
        .arg("--out")
        .arg(dir.path().join("stale").join("skill.spec.yml"))
        .arg("--source-map")
        .arg(map_dir.join("source-map.json"))
        .output()
        .unwrap();
    assert_failure(&stale_import);
    assert!(stderr(&stale_import).contains("source map"));
    assert!(stderr(&stale_import).contains("stale"));
}

#[test]
fn doctor_reports_prose_skill_context_and_reliability_debt() {
    let dir = TempDir::new("doctor-prose");
    let skill_dir = dir.path().join("source-skill");
    let mut skill = String::from(
        r#"---
name: dense-prose
description: Use when a dense prose skill mixes instructions, snippets, and dependency assumptions.
---

# Dense Prose Skill

Use the shell and Python to inspect the project, fetch external data, create a report, and install missing packages when needed.
See [missing local reference](missing.md).

```
pip install pypdf
```

```python
import pypdf
from reportlab.pdfgen import canvas
```

"#,
    );
    for index in 1..=520 {
        skill.push_str(&format!(
            "{index}. Always run verification step {index} before continuing.\n"
        ));
    }
    skill.push_str("\nNever skip the final proof summary.\n");
    write_file(&skill_dir.join("SKILL.md"), &skill);

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&skill_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    let report = json_stdout(&output);
    assert!(report["structural_score"].as_u64().unwrap() < 40);
    assert!(report["large_surface_percentage"].as_u64().unwrap() >= 90);
    assert_eq!(report["counts"]["unlabeled_code_blocks_in_skill"], 1);
    assert!(report["counts"]["numbered_steps"].as_u64().unwrap() >= 520);

    let issues_text = serde_json::to_string(&report["issues"]).unwrap();
    assert!(issues_text.contains("large_activation_body"));
    assert!(issues_text.contains("primacy_bias_late_obligations"));
    assert!(issues_text.contains("code_mixed_with_activation_instructions"));
    assert!(issues_text.contains("unlabeled_code_fences"));
    assert!(issues_text.contains("implicit_dependency_contract"));
    assert!(issues_text.contains("ambiguous_execution_substrate"));
    assert!(issues_text.contains("missing_behavior_contract"));
    assert!(issues_text.contains("missing_trace_proof_surface"));
    assert!(issues_text.contains("missing_referenced_files"));

    let text = Command::new(bin())
        .arg("doctor")
        .arg(&skill_dir)
        .output()
        .unwrap();
    assert_success(&text);
    let text = stdout(&text);
    assert!(text.contains("large_surface:"));
    assert!(text.contains("docs/00-skills-reliability-gap.md"));
    assert!(text.contains("docs/08-contract-trace-methodology.md"));
}

#[test]
fn doctor_rejects_parent_folder_with_multiple_skills() {
    let dir = TempDir::new("doctor-multi");
    let root = dir.path().join("skills");
    write_file(
        &root.join("pdf").join("SKILL.md"),
        "---\nname: pdf\ndescription: PDF skill.\n---\n# PDF\n",
    );
    write_file(
        &root.join("csv").join("SKILL.md"),
        "---\nname: csv\ndescription: CSV skill.\n---\n# CSV\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .output()
        .unwrap();
    assert_failure(&output);
    assert!(stderr(&output).contains("requires exactly one SKILL.md"));
}

#[test]
fn import_skill_scaffolds_dependency_ledger_from_code_imports() {
    let dir = TempDir::new("import-deps-ledger");
    let skill_dir = dir.path().join("source-skill");
    let out = dir.path().join("draft").join("skill.spec.yml");
    write_file(
        &skill_dir.join("SKILL.md"),
        r#"# Imported Dependencies

```python
import json
import pypdf
from reportlab.pdfgen import canvas
```

```ts
import { chromium } from "playwright";
import fs from "fs";
const helper = require("@scope/helper/path");
```
"#,
    );

    let import = Command::new(bin())
        .arg("import-skill")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&import);

    let ledger = out.parent().unwrap().join("deps.toml");
    assert!(ledger.is_file());
    let ledger_content = fs::read_to_string(&ledger).unwrap();
    assert!(ledger_content.contains("id = \"python3\""));
    assert!(ledger_content.contains("id = \"deno\""));
    assert!(ledger_content.contains("id = \"pypdf\""));
    assert!(ledger_content.contains("id = \"reportlab\""));
    assert!(ledger_content.contains("id = \"playwright\""));
    assert!(ledger_content.contains("id = \"@scope/helper\""));
    assert!(!ledger_content.contains("id = \"json\""));
    assert!(!ledger_content.contains("id = \"fs\""));
}

#[test]
fn import_skill_writes_relative_out_without_parent() {
    let dir = TempDir::new("import-relative-out");
    let skill_dir = dir.path().join("source-skill");
    write_file(
        &skill_dir.join("SKILL.md"),
        r#"# Relative Output

```python
print("hello")
```
"#,
    );

    let import = Command::new(bin())
        .current_dir(dir.path())
        .arg("import-skill")
        .arg("source-skill")
        .arg("--out")
        .arg("skill.spec.yml")
        .output()
        .unwrap();
    assert_success(&import);

    assert!(dir.path().join("skill.spec.yml").is_file());
    assert!(dir.path().join("deps.toml").is_file());
}

#[test]
fn install_skill_supports_dry_run_and_claude_local_install() {
    let dir = TempDir::new("install");
    let home = dir.path().join("home");
    let repo = dir.path().join("repo");
    let skill = dir.path().join("skill-source");
    fs::create_dir_all(home.join(".agents/skills")).unwrap();
    fs::create_dir_all(home.join(".codex/skills")).unwrap();
    fs::create_dir_all(repo.join(".claude")).unwrap();
    write_file(
        &skill.join("SKILL.md"),
        "# Installable Skill\n\nThin loader for skill.spec.yml.\n",
    );
    write_file(&skill.join("deps.toml"), "# dependency manifest\n");
    write_file(
        &skill.join("source/SKILL_md.old"),
        "# Original Skill\n\nPreserved source material.\n",
    );
    write_file(
        &skill.join("source/reference.md"),
        "# Reference
",
    );
    write_file(
        &skill.join("resources/helper.py"),
        "print('helper')
",
    );
    write_file(
        &skill.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: installable.skill
title: Installable Skill
description: Install target fixture.
routes:
  - id: local
    label: Local
dependencies:
  deps_toml:
    kind: file
    path: deps.toml
imports:
  reference:
    path: source/reference.md
    role: reference
    used_by:
      - kind: route
        id: local
resources:
  preserved_source:
    path: source/SKILL_md.old
    role: source_material
    used_by:
      - kind: route
        id: local
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

    let dry_run = Command::new(bin())
        .current_dir(&repo)
        .env("HOME", &home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("claude-local")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert_success(&dry_run);
    let planned = json_stdout(&dry_run);
    assert_eq!(planned["dry_run"], true);
    assert_eq!(planned["installs"][0]["status"], "planned");
    assert!(!repo.join(".claude/skills/skill-source/SKILL.md").exists());

    let install = Command::new(bin())
        .current_dir(&repo)
        .env("HOME", &home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("claude-local")
        .arg("--name")
        .arg("installed-skill")
        .output()
        .unwrap();
    assert_success(&install);
    let installed = json_stdout(&install);
    assert_eq!(installed["installs"][0]["status"], "installed");
    assert!(repo
        .join(".claude/skills/installed-skill/SKILL.md")
        .is_file());
    assert!(repo
        .join(".claude/skills/installed-skill/skill.spec.yml")
        .is_file());
    assert!(repo
        .join(".claude/skills/installed-skill/deps.toml")
        .is_file());
    assert!(repo
        .join(".claude/skills/installed-skill/source/SKILL_md.old")
        .is_file());
    assert!(repo
        .join(".claude/skills/installed-skill/source/reference.md")
        .is_file());
    assert!(repo
        .join(".claude/skills/installed-skill/resources/helper.py")
        .is_file());
}

#[test]
fn install_skill_rejects_nested_discoverable_skill_md_support_file() {
    let dir = TempDir::new("install-nested-skill-md");
    let home = dir.path().join("home");
    let skill = dir.path().join("skill-source");
    fs::create_dir_all(home.join(".agents/skills")).unwrap();
    write_file(
        &skill.join("SKILL.md"),
        "# Installable Skill\n\nThin loader for skill.spec.yml.\n",
    );
    write_file(
        &skill.join("source/SKILL.md"),
        "# Original Skill\n\nThis nested name should not be installable.\n",
    );
    write_file(
        &skill.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: installable.skill
title: Installable Skill
description: Install target fixture.
routes:
  - id: local
    label: Local
resources:
  preserved_source:
    path: source/SKILL.md
    role: source_material
    used_by:
      - kind: route
        id: local
"#,
    );

    let install = Command::new(bin())
        .env("HOME", &home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("agents")
        .output()
        .unwrap();
    assert_failure(&install);
    assert!(stderr(&install).contains("nested discoverable SKILL.md"));
}

#[test]
fn install_skill_detects_existing_target_before_overwrite() {
    let dir = TempDir::new("install-existing");
    let home = dir.path().join("home");
    let skill = dir.path().join("skill-source");
    let install_dir = home.join(".agents/skills/skill-source");
    fs::create_dir_all(&install_dir).unwrap();
    write_file(&install_dir.join("SKILL.md"), "# Old Skill\n");
    write_file(&install_dir.join("skill.spec.yml"), "schema: old\n");
    write_file(&install_dir.join("stale.txt"), "left alone\n");
    write_file(
        &skill.join("SKILL.md"),
        "# Installable Skill\n\nThin loader for skill.spec.yml.\n",
    );
    write_file(
        &skill.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: installable.skill
title: Installable Skill
description: Install target fixture.
routes:
  - id: local
    label: Local
"#,
    );

    let dry_run = Command::new(bin())
        .env("HOME", &home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("agents")
        .arg("--dry-run")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_success(&dry_run);
    let planned = json_stdout(&dry_run);
    assert_eq!(planned["installs"][0]["status"], "planned");
    assert_eq!(planned["installs"][0]["existed"], true);
    assert_eq!(
        fs::read_to_string(install_dir.join("SKILL.md")).unwrap(),
        "# Old Skill\n"
    );

    let refused = Command::new(bin())
        .env("HOME", &home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("agents")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_failure(&refused);
    assert!(stderr(&refused).contains("rerun with --force to overwrite"));
    assert_eq!(
        fs::read_to_string(install_dir.join("SKILL.md")).unwrap(),
        "# Old Skill\n"
    );

    let forced = Command::new(bin())
        .env("HOME", &home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("agents")
        .arg("--force")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_success(&forced);
    let installed = json_stdout(&forced);
    assert_eq!(installed["installs"][0]["status"], "installed");
    assert_eq!(installed["installs"][0]["existed"], true);
    assert_eq!(
        fs::read_to_string(install_dir.join("SKILL.md")).unwrap(),
        "# Installable Skill\n\nThin loader for skill.spec.yml.\n"
    );
    assert!(install_dir.join("stale.txt").is_file());
}

#[test]
fn install_skill_can_retire_existing_target_with_backup() {
    let dir = TempDir::new("install-retire-existing");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let skill = dir.path().join("skill-source");
    let install_dir = home.join(".agents/skills/skill-source");
    fs::create_dir_all(&install_dir).unwrap();
    write_file(&install_dir.join("SKILL.md"), "# Old Skill\n");
    write_file(&install_dir.join("skill.spec.yml"), "schema: old\n");
    write_file(&install_dir.join("stale.txt"), "old-only\n");
    write_file(
        &skill.join("SKILL.md"),
        "# Installable Skill\n\nThin loader for skill.spec.yml.\n",
    );
    write_file(
        &skill.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: installable.skill
title: Installable Skill
description: Install target fixture.
routes:
  - id: local
    label: Local
"#,
    );

    let dry_run = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("agents")
        .arg("--retire-existing")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert_success(&dry_run);
    let planned = json_stdout(&dry_run);
    assert_eq!(planned["installs"][0]["status"], "planned");
    assert_eq!(planned["installs"][0]["retired_existing"], true);
    assert!(planned["installs"][0]["backup_path"]
        .as_str()
        .unwrap()
        .contains("backups/retired-skills"));
    assert!(!skillspec_home.join("backups/retired-skills").exists());

    let retired = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("agents")
        .arg("--retire-existing")
        .output()
        .unwrap();
    assert_success(&retired);
    let report = json_stdout(&retired);
    assert_eq!(report["installs"][0]["status"], "installed");
    assert_eq!(report["installs"][0]["retired_existing"], true);
    let backup_path = PathBuf::from(report["installs"][0]["backup_path"].as_str().unwrap());
    assert!(backup_path.join("SKILL.md").is_file());
    assert_eq!(
        fs::read_to_string(backup_path.join("SKILL.md")).unwrap(),
        "# Old Skill\n"
    );
    assert!(backup_path.join("stale.txt").is_file());
    assert_eq!(
        fs::read_to_string(install_dir.join("SKILL.md")).unwrap(),
        "# Installable Skill\n\nThin loader for skill.spec.yml.\n"
    );
    assert!(!install_dir.join("stale.txt").exists());

    let conflict = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("agents")
        .arg("--force")
        .arg("--retire-existing")
        .output()
        .unwrap();
    assert_failure(&conflict);
    assert!(stderr(&conflict).contains("mutually exclusive"));
}

#[cfg(unix)]
#[test]
fn install_skill_retire_existing_groups_symlinked_roots() {
    let dir = TempDir::new("install-retire-symlinked-roots");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_parent = home.join(".codex");
    let codex_root = codex_parent.join("skills");
    let install_dir = agents_root.join("skill-source");
    let skill = dir.path().join("skill-source");
    fs::create_dir_all(&install_dir).unwrap();
    fs::create_dir_all(&codex_parent).unwrap();
    symlink(&agents_root, &codex_root).unwrap();
    write_file(&install_dir.join("SKILL.md"), "# Old Skill\n");
    write_file(&install_dir.join("skill.spec.yml"), "schema: old\n");
    write_file(&install_dir.join("stale.txt"), "old-only\n");
    write_file(
        &skill.join("SKILL.md"),
        "# Installable Skill\n\nThin loader for skill.spec.yml.\n",
    );
    write_file(
        &skill.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: installable.skill
title: Installable Skill
description: Install target fixture.
routes:
  - id: local
    label: Local
"#,
    );

    let retired = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("install")
        .arg("skill")
        .arg(&skill)
        .arg("--target")
        .arg("agents")
        .arg("--target")
        .arg("codex")
        .arg("--retire-existing")
        .output()
        .unwrap();
    assert_success(&retired);
    let report = json_stdout(&retired);
    let installs = report["installs"].as_array().unwrap();
    assert_eq!(installs.len(), 2);
    assert_eq!(installs[0]["retired_existing"], true);
    assert_eq!(installs[1]["retired_existing"], true);
    assert_eq!(installs[0]["backup_path"], installs[1]["backup_path"]);

    let backup_path = PathBuf::from(installs[0]["backup_path"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(backup_path.join("SKILL.md")).unwrap(),
        "# Old Skill\n"
    );
    assert!(backup_path.join("stale.txt").is_file());
    assert!(!backup_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("codex/skill-source")
        .exists());
    assert_eq!(
        fs::read_to_string(install_dir.join("SKILL.md")).unwrap(),
        "# Installable Skill\n\nThin loader for skill.spec.yml.\n"
    );
    assert!(!install_dir.join("stale.txt").exists());
}

#[test]
fn install_skill_supports_folder_shaped_examples() {
    let dir = TempDir::new("install-example");
    let home = dir.path().join("home");
    fs::create_dir_all(home.join(".agents/skills")).unwrap();
    fs::create_dir_all(home.join(".codex/skills")).unwrap();

    let dry_run = Command::new(bin())
        .current_dir(repo_root())
        .env("HOME", &home)
        .arg("install")
        .arg("skill")
        .arg("examples/durable-executor")
        .arg("--target")
        .arg("agents")
        .arg("--target")
        .arg("codex")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert_success(&dry_run);
    let planned = json_stdout(&dry_run);
    assert_eq!(planned["skill_name"], "durable-executor");
    assert_eq!(planned["dry_run"], true);
    assert_eq!(planned["installs"].as_array().unwrap().len(), 2);
    assert!(planned["installs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|install| install["status"] == "planned"));
}

#[test]
fn schema_records_strict_typed_sections_and_extension_surfaces() {
    let schema_path = repo_root().join("spec/skill.spec.schema.json");
    let schema: Value = serde_json::from_str(&fs::read_to_string(schema_path).unwrap()).unwrap();

    assert_eq!(schema["additionalProperties"], false);
    for typed_def in [
        "route",
        "activation",
        "rule",
        "predicate",
        "state",
        "dependency",
        "import",
        "import_requires",
        "import_use",
        "resource",
        "code_block",
        "artifact",
        "recipe",
        "command",
        "expectation",
    ] {
        assert_eq!(
            schema["$defs"][typed_def]["additionalProperties"], false,
            "{typed_def} should be strict"
        );
    }

    assert_eq!(
        schema["properties"]["metadata"]["additionalProperties"],
        true
    );
    assert_eq!(
        schema["properties"]["closures"]["additionalProperties"],
        true
    );
    assert_eq!(
        schema["$defs"]["rule"]["properties"]["allow"]["additionalProperties"],
        true
    );
    assert_eq!(
        schema["$defs"]["elicitation_choice"]["properties"]["sets"]["additionalProperties"],
        true
    );
}

#[test]
fn published_json_schema_validates_every_example() {
    let root = repo_root();
    let schema_path = root.join("spec/skill.spec.schema.json");
    let schema: Value = serde_json::from_str(&fs::read_to_string(&schema_path).unwrap()).unwrap();
    jsonschema::meta::validate(&schema).unwrap_or_else(|error| {
        panic!(
            "published JSON Schema is not valid at {}: {error}",
            schema_path.display()
        )
    });
    let validator = jsonschema::validator_for(&schema).unwrap();

    let mut examples = Vec::new();
    collect_yml_files(&root.join("examples"), &mut examples);
    examples.sort();
    assert!(!examples.is_empty(), "expected at least one example spec");

    let mut failures = Vec::new();
    for path in examples {
        let yaml: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));
        let instance = serde_json::to_value(yaml).unwrap();
        let errors = validator
            .iter_errors(&instance)
            .map(|error| format!("{error} at {}", error.instance_path()))
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            failures.push(format!(
                "{}\n{}",
                path.strip_prefix(&root).unwrap().display(),
                errors.join("\n")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "example specs failed JSON Schema validation:\n{}",
        failures.join("\n\n")
    );
}

#[test]
fn compiler_markdown_output_matches_golden_snapshot() {
    let root = repo_root();
    let output = Command::new(bin())
        .current_dir(&root)
        .arg("compile")
        .arg("examples/repo-readiness/skill.spec.yml")
        .arg("--target")
        .arg("markdown")
        .output()
        .unwrap();
    assert_success(&output);

    assert_snapshot_eq(
        &root.join("fixtures/golden/compile-repo-readiness.markdown.md"),
        &stdout(&output),
    );
}

#[test]
fn importer_output_matches_golden_snapshot() {
    let root = repo_root();
    let dir = TempDir::new("import-golden");
    let out = dir.path().join("skill.spec.yml");
    let output = Command::new(bin())
        .current_dir(&root)
        .arg("import-skill")
        .arg("fixtures/skills")
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&output);

    assert_snapshot_eq(
        &root.join("fixtures/golden/import-fixtures-skill.spec.yml"),
        &fs::read_to_string(out).unwrap(),
    );
    assert!(dir
        .path()
        .join("resources/imported-code/skill_code_1.sh")
        .is_file());
}

#[test]
fn synthesize_from_workspace_generates_valid_review_scaffold() {
    let dir = TempDir::new("synthesize-workspace");
    let stats = dir.path().join("stats.txt");
    let log = dir.path().join("log.json");
    let meta = dir.path().join("meta.txt");
    let deps = dir.path().join("deps.txt");
    let out = dir.path().join("profile-enricher");

    write_file(
        &stats,
        r#"
Workspace: profile-enrichment
Total tokens: 12000
Source tokens: 9000
Result tokens: 1200
"#,
    );
    write_file(
        &log,
        r#"
[
  {"sequence":1,"command":"parallel web enrich --profile input.json --out enriched.json"},
  {"sequence":2,"command":"jq . enriched.json"}
]
"#,
    );
    write_file(
        &meta,
        r#"
name = profile-enrichment
strategy = durable
"#,
    );
    write_file(
        &deps,
        r#"
1 -> 2
"#,
    );

    let output = Command::new(bin())
        .arg("synthesize-from-workspace")
        .arg("profile-enrichment")
        .arg("--task")
        .arg("use parallel web to enrich this profile")
        .arg("--out")
        .arg(&out)
        .arg("--workspace-stats-report")
        .arg(&stats)
        .arg("--workspace-log")
        .arg(&log)
        .arg("--workspace-meta")
        .arg(&meta)
        .arg("--workspace-deps")
        .arg(&deps)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["workspace"], "profile-enrichment");
    assert_eq!(report["observed_command_candidates"], 2);
    assert!(report["inferred_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| dependency == "parallel_cli"));

    let spec = out.join("skill.spec.yml");
    assert!(spec.is_file());
    assert!(out.join("resources/observed-workspace/report.md").is_file());
    assert!(out.join("resources/observed-workspace/stats.txt").is_file());
    assert!(out.join("resources/observed-workspace/log.txt").is_file());
    assert!(out.join("resources/observed-workspace/meta.txt").is_file());
    assert!(out.join("resources/observed-workspace/deps.txt").is_file());
    assert!(out
        .join("resources/observed-workspace/coverage-matrix.md")
        .is_file());

    let yaml = fs::read_to_string(&spec).unwrap();
    assert!(yaml.contains("parallel_cli"));
    assert!(yaml.contains("observed_workspace_report"));
    assert!(yaml.contains("observed_command_1"));

    let validate = Command::new(bin())
        .arg("validate")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&validate);
    let test = Command::new(bin()).arg("test").arg(&spec).output().unwrap();
    assert_success(&test);
    let imports = Command::new(bin())
        .arg("imports")
        .arg("check")
        .arg(&spec)
        .output()
        .unwrap();
    assert_success(&imports);
}

#[test]
fn synthesize_from_workspace_requires_command_log_entries() {
    let dir = TempDir::new("synthesize-workspace-empty-log");
    let stats = dir.path().join("stats.txt");
    let log = dir.path().join("log.txt");
    let meta = dir.path().join("meta.txt");
    let out = dir.path().join("profile-enricher");

    write_file(&stats, "Workspace: profile-enrichment\nTotal tokens: 10\n");
    write_file(&log, "[]\n");
    write_file(&meta, "name = profile-enrichment\n");

    let output = Command::new(bin())
        .arg("synthesize-from-workspace")
        .arg("profile-enrichment")
        .arg("--out")
        .arg(&out)
        .arg("--workspace-stats-report")
        .arg(&stats)
        .arg("--workspace-log")
        .arg(&log)
        .arg("--workspace-meta")
        .arg(&meta)
        .output()
        .unwrap();
    assert_failure(&output);
    assert!(stderr(&output).contains("workspace command log evidence has no command entries"));
    assert!(!out.join("skill.spec.yml").exists());
}

#[test]
fn conformance_fixtures_have_expected_validation_outcomes() {
    let root = repo_root();
    let mut valid = Vec::new();
    collect_yml_files(&root.join("conformance/valid"), &mut valid);
    valid.sort();
    assert!(!valid.is_empty(), "expected valid conformance fixtures");

    for path in valid {
        let output = Command::new(bin())
            .current_dir(&root)
            .arg("validate")
            .arg(&path)
            .output()
            .unwrap();
        assert_success(&output);
    }

    let mut invalid = Vec::new();
    collect_yml_files(&root.join("conformance/invalid"), &mut invalid);
    invalid.sort();
    assert!(!invalid.is_empty(), "expected invalid conformance fixtures");

    for path in invalid {
        let output = Command::new(bin())
            .current_dir(&root)
            .arg("validate")
            .arg(&path)
            .output()
            .unwrap();
        assert_failure(&output);
    }
}
