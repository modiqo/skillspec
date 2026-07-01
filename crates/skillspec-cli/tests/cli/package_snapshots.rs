use crate::support::*;

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
fn crate_package_artifacts_match_published_root_files() {
    let root = repo_root();
    for relative_path in [
        "LICENSE-APACHE",
        "LICENSE-MIT",
        "spec/grammar.md",
        "spec/skill.spec.schema.json",
    ] {
        let published = fs::read_to_string(root.join(relative_path)).unwrap();
        let crate_copy =
            fs::read_to_string(root.join("crates/skillspec-cli").join(relative_path)).unwrap();
        assert_eq!(
            crate_copy, published,
            "crate-local {relative_path} must match the root file before release"
        );
    }
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
    let path = write_failing_rote(dir.path());
    write_success_cli(&dir.path().join("bin"), "parallel-cli");

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
  {"sequence":1,"command":"rote exec -- which parallel-cli"},
  {"sequence":2,"command":"rote exec -- parallel-cli --version"},
  {"sequence":3,"command":"rote exec -- parallel-cli auth status"},
  {"sequence":4,"command":"rote exec -- parallel-cli auth"},
  {"sequence":5,"command":"rote exec -- parallel-cli enrich run --data '[{\"name\":\"Example Person\"}]' --intent 'Find public professional profile facts' --processor base-fast --json --dry-run"},
  {"sequence":6,"command":"rote exec -- parallel-cli enrich run --data '[{\"name\":\"Example Person\"}]' --target enriched-profiles.csv --intent 'Find public professional profile facts' --processor base-fast --json --dry-run"},
  {"sequence":7,"command":"rote exec -- parallel-cli enrich run --data '[{\"name\":\"Example Person\"}]' --target enriched-profiles.csv --intent 'Find public professional profile facts' --processor base-fast --json"}
]
"#,
    );
    write_file(
        &meta,
        r#"
name = profile-enrichment
strategy = completed-cli
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
        .arg("--observation-approved")
        .arg("--json")
        .env("PATH", &path)
        .output()
        .unwrap();
    assert_success(&output);
    let report = json_stdout(&output);
    assert!(report["workspace"].is_null());
    assert_eq!(
        report["deps_path"],
        out.join("deps.toml").display().to_string()
    );
    assert!(report["command_candidates"].as_u64().unwrap() >= 5);
    assert!(report["inferred_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| dependency == "parallel_cli"));

    let spec = out.join("skill.spec.yml");
    assert!(spec.is_file());
    assert!(out.join("deps.toml").is_file());
    assert!(!out.join("resources/observed-workspace/report.md").exists());
    assert!(!out.join("resources/observed-workspace/stats.txt").exists());
    assert!(!out.join("resources/observed-workspace/log.txt").exists());
    assert!(!out.join("resources/observed-workspace/meta.txt").exists());
    assert!(!out.join("resources/observed-workspace/deps.txt").exists());

    let yaml = fs::read_to_string(&spec).unwrap();
    assert!(yaml.contains("id: parallel_profile_enricher"));
    assert!(yaml.contains("parallel_cli"));
    assert!(yaml.contains("dependency_ledger"));
    assert!(yaml.contains("path: deps.toml"));
    assert!(yaml.contains("profile_enrichment_cli"));
    assert!(yaml.contains("provide_profile_enrichment_inputs"));
    assert!(yaml.contains("people_json"));
    assert!(yaml.contains("cli_enrich_dry_run"));
    assert!(yaml.contains("use_auth_status_subcommand"));
    assert!(yaml.contains("omit_target_option"));
    assert!(!yaml.to_ascii_lowercase().contains("durable"));
    assert!(!yaml.to_ascii_lowercase().contains("workspace"));
    assert!(!yaml.to_ascii_lowercase().contains("observed"));
    assert!(!yaml.to_ascii_lowercase().contains("rote"));
    assert!(!yaml.contains("Example Person"));
    assert!(!yaml.contains("Find public professional profile facts"));

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

    let ledger = fs::read_to_string(out.join("deps.toml")).unwrap();
    assert!(ledger.contains("generated_by = \"skillspec synthesize-from-workspace\""));
    assert!(ledger.contains("dependency_count = "));
    assert!(ledger.contains("id = \"parallel-cli\""));
    assert!(!ledger.contains("id = \"dependency_ledger\""));

    let deps_check = Command::new(bin())
        .arg("deps")
        .arg("check")
        .arg(&spec)
        .env("PATH", &path)
        .output()
        .unwrap();
    assert_success(&deps_check);
    let deps_report = json_stdout(&deps_check);
    assert!(deps_report["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| dependency["id"] == "dependency_ledger"
            && dependency["status"] == "present"
            && dependency["message"] == "deps.toml exists"));
}

#[test]
fn synthesize_from_workspace_preserves_detected_cli_name() {
    let dir = TempDir::new("synthesize-workspace-cli-name");
    let stats = dir.path().join("stats.txt");
    let log = dir.path().join("log.json");
    let meta = dir.path().join("meta.txt");
    let out = dir.path().join("stripe-flow");
    let path = write_failing_rote(dir.path());

    write_file(&stats, "Workspace: stripe-flow\nTotal tokens: 200\n");
    write_file(
        &log,
        r#"
[
  {"sequence":1,"command":"rote exec -- which stripe"},
  {"sequence":2,"command":"rote exec -- STRIPE_API_KEY=test stripe customers list --limit 1"}
]
"#,
    );
    write_file(&meta, "name = stripe-flow\n");

    let output = Command::new(bin())
        .arg("synthesize-from-workspace")
        .arg("stripe-flow")
        .arg("--task")
        .arg("run stripe customers list")
        .arg("--out")
        .arg(&out)
        .arg("--workspace-stats-report")
        .arg(&stats)
        .arg("--workspace-log")
        .arg(&log)
        .arg("--workspace-meta")
        .arg(&meta)
        .arg("--observation-approved")
        .arg("--json")
        .env("PATH", &path)
        .output()
        .unwrap();
    assert_success(&output);
    let report = json_stdout(&output);
    let dependencies = report["inferred_dependencies"].as_array().unwrap();
    assert!(dependencies.iter().any(|dependency| dependency == "stripe"));
    assert!(!dependencies
        .iter()
        .any(|dependency| dependency == "stripe_cli"));
    assert!(!dependencies.iter().any(|dependency| dependency == "which"));

    let yaml = fs::read_to_string(out.join("skill.spec.yml")).unwrap();
    assert!(yaml.contains("  stripe:"));
    assert!(yaml.contains("command: stripe"));
    assert!(yaml.contains("stripe --version"));
    let ledger = fs::read_to_string(out.join("deps.toml")).unwrap();
    assert!(ledger.contains("id = \"stripe\""));
    assert!(!ledger.contains("id = \"stripe_cli\""));
    assert!(!yaml.contains("stripe_cli"));
    assert!(!yaml.contains("parallel_cli"));
    assert!(!yaml.contains("parallel-cli"));
}

#[test]
fn synthesize_from_workspace_requires_observation_approval() {
    let dir = TempDir::new("synthesize-workspace-approval");
    let stats = dir.path().join("stats.txt");
    let log = dir.path().join("log.txt");
    let meta = dir.path().join("meta.txt");
    let out = dir.path().join("profile-enricher");

    write_file(&stats, "Workspace: profile-enrichment\nTotal tokens: 10\n");
    write_file(
        &log,
        r#"[{"sequence":1,"command":"parallel web enrich --profile input.json --out enriched.json"}]"#,
    );
    write_file(&meta, "name = profile-enrichment\n");

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
        .output()
        .unwrap();
    assert_failure(&output);
    assert!(stderr(&output).contains("CLI interaction approval is required"));
    assert!(stderr(&output).contains("Command candidates: 1"));
    assert!(stderr(&output).contains("--observation-approved"));
    assert!(!out.join("skill.spec.yml").exists());
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
        .arg("--observation-approved")
        .output()
        .unwrap();
    assert_failure(&output);
    assert!(stderr(&output).contains("CLI interaction transcript has no command entries"));
    assert!(!out.join("skill.spec.yml").exists());
}

#[test]
fn synthesize_from_workspace_live_collection_reports_context() {
    let dir = TempDir::new("synthesize-workspace-live-failure");
    let path = write_failing_rote(dir.path());
    let out = dir.path().join("profile-enricher");

    let output = Command::new(bin())
        .current_dir(dir.path())
        .arg("synthesize-from-workspace")
        .arg("profile-enrichment")
        .arg("--out")
        .arg(&out)
        .arg("--observation-approved")
        .env("PATH", &path)
        .output()
        .unwrap();
    assert_failure(&output);
    let error = stderr(&output);
    assert!(error.contains("`rote workspace stats profile-enrichment` failed"));
    assert!(error.contains("source id: profile-enrichment"));
    assert!(error.contains("invocation cwd:"));
    assert!(error.contains("evidence overrides: stats=live, log=live, meta=live, deps=live"));
    assert!(error.contains("Fallback without workspace name also failed"));
    assert!(error.contains("live rote should not be called"));
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
