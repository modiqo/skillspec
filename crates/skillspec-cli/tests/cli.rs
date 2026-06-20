use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

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
routes:
  - id: browser
    label: Browser
    rank: 10
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
    assert!(align_help.contains("Usage: skillspec trace align"));
    assert!(align_help.contains("--decision-trace <DECISION_TRACE>"));
    assert!(align_help.contains("<PATH>"));
    assert!(align_help.contains("--json"));
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
    assert!(loader_out.contains("thin loader"));
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
    assert!(stdout(&import).contains("review note"));

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
}

#[test]
fn schema_records_strict_typed_sections_and_extension_surfaces() {
    let schema_path = repo_root().join("spec/skill.spec.schema.json");
    let schema: Value = serde_json::from_str(&fs::read_to_string(schema_path).unwrap()).unwrap();

    assert_eq!(schema["additionalProperties"], false);
    for typed_def in [
        "route",
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
        .arg("examples/repo-readiness.skill.spec.yml")
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
