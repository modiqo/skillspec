pub(crate) use serde_json::{json, Value};
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
pub(crate) use std::os::unix::fs::{symlink, PermissionsExt};

pub(crate) fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_skillspec")
}

pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("skillspec-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

pub(crate) fn hook_commands(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    value
        .get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|hooks| hooks.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|group| group.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|hook| hook.get("command").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

pub(crate) fn has_hook_command(path: &Path, needle: &str) -> bool {
    hook_commands(path)
        .iter()
        .any(|command| command.contains(needle))
}

pub(crate) fn invalid_json_shape(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

pub(crate) fn write_durable_source(path: &Path, description_suffix: &str) {
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
pub(crate) fn write_executable(path: &Path, content: &str) {
    write_file(path, content);
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

pub(crate) fn write_fake_rote(path: &Path) -> std::ffi::OsString {
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

pub(crate) fn write_failing_rote(path: &Path) -> std::ffi::OsString {
    let bin_dir = path.join("bin");
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("rote"),
        "#!/bin/sh\necho live rote should not be called >&2\nexit 42\n",
    );
    #[cfg(windows)]
    write_file(
        &bin_dir.join("rote.cmd"),
        "@echo off\r\necho live rote should not be called 1>&2\r\nexit /B 42\r\n",
    );

    let mut paths = vec![bin_dir];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).unwrap()
}

pub(crate) fn write_success_cli(bin_dir: &Path, name: &str) {
    #[cfg(unix)]
    write_executable(&bin_dir.join(name), "#!/bin/sh\nexit 0\n");
    #[cfg(windows)]
    write_file(
        &bin_dir.join(format!("{name}.cmd")),
        "@echo off\r\nexit /B 0\r\n",
    );
}

pub(crate) fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

pub(crate) fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

pub(crate) fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub(crate) fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse stdout as JSON: {error}\nstdout:\n{}",
            stdout(output)
        )
    })
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

pub(crate) fn collect_yml_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yml_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("yml") {
            files.push(path);
        }
    }
}

pub(crate) fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

pub(crate) fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn assert_snapshot_eq(snapshot_path: &Path, actual: &str) {
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

pub(crate) fn rich_spec() -> &'static str {
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

pub(crate) fn deps_spec() -> &'static str {
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

pub(crate) fn alignment_spec() -> &'static str {
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
