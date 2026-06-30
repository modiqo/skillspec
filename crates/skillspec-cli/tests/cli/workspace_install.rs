use crate::support::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[test]
fn workspace_map_and_validate_reports_package_graph() {
    let dir = TempDir::new("workspace-map");
    let root = dir.path().join("skills");
    let manifest = dir.path().join("build").join("skillspec.workspace.yml");
    write_file(
        &root.join("coding-standards").join("SKILL.md"),
        r#"---
name: coding-standards
description: TypeScript coding standards package.
---
# Coding Standards
"#,
    );
    write_file(
        &root.join("coding-standards").join("TESTING.md"),
        "# Testing\n",
    );
    write_file(
        &root.join("code-review").join("SKILL.md"),
        r#"---
name: code-review
description: Review code.
disable-model-invocation: true
---
# Code Review

Treat `../coding-standards/` as the standards package.
Read `../coding-standards/SKILL.md`.
"#,
    );
    write_file(
        &root.join("review-wrapper").join("SKILL.md"),
        r#"---
name: review-wrapper
description: Wrapper skill.
---
# Review Wrapper

Run `/coding-standards` before the wrapper.
"#,
    );

    let map = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg(&root)
        .arg("--out")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["package_count"], 3);
    assert_eq!(map_report["source_shape"]["kind"], "multi_skill_workspace");
    assert_eq!(map_report["source_shape"]["skill_files"], 3);
    let edges = map_report["dependency_edges"].as_array().unwrap();
    assert!(edges
        .iter()
        .any(|edge| edge["from"] == "code-review" && edge["to"] == "coding-standards"));
    assert!(edges
        .iter()
        .any(|edge| edge["from"] == "review-wrapper" && edge["to"] == "coding-standards"));
    let references = map_report["references"].as_array().unwrap();
    assert!(references
        .iter()
        .any(|reference| reference["from_package"] == "review-wrapper"
            && reference["raw"] == "/coding-standards"));
    assert!(!references
        .iter()
        .any(|reference| reference["from_package"] == "code-review"
            && reference["raw"] == "/coding-standards"));
    assert!(manifest.is_file());
    assert!(PathBuf::from(format!("{}.report.md", manifest.display())).is_file());

    let manifest_yaml = fs::read_to_string(&manifest).unwrap();
    assert!(manifest_yaml.contains("install_slug_policy: workspace-path"));
    assert!(manifest_yaml.contains("install_slug: skills--code-review"));
    assert!(manifest_yaml.contains("install_slug: skills--review-wrapper"));
    assert!(manifest_yaml.contains("depends_on:\n    - coding-standards"));

    let map_summary = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg(&root)
        .arg("--out")
        .arg(&manifest)
        .arg("--summary")
        .output()
        .unwrap();
    assert_success(&map_summary);
    let map_summary = stdout(&map_summary);
    assert!(map_summary.contains("Workspace map summary"));
    assert!(map_summary.contains("metrics:"));
    assert!(map_summary.contains("wall_clock:"));
    assert!(map_summary.contains("agent_visible_tokens: ~"));
    assert!(map_summary.contains("artifact_tokens_preserved: ~"));
    assert!(map_summary.contains("avoided_tokens: ~"));
    assert!(map_summary.contains("metrics_source: estimated"));
    assert!(map_summary.contains("- install_slug_policy: workspace-path"));
    assert!(map_summary.contains("- source_shape: multi_skill_workspace"));
    assert!(map_summary.contains("- source_skill_files: 3"));
    assert!(map_summary.contains(&format!("- manifest: {}", normalize_path(&manifest))));
    assert!(!map_summary.contains("## Packages"));

    let validate = Command::new(bin())
        .arg("workspace")
        .arg("validate")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&validate);
    let validate_report = json_stdout(&validate);
    assert_eq!(validate_report["ok"], true);

    let validate_summary = Command::new(bin())
        .arg("workspace")
        .arg("validate")
        .arg(&manifest)
        .arg("--summary")
        .output()
        .unwrap();
    assert_success(&validate_summary);
    let validate_summary = stdout(&validate_summary);
    assert!(validate_summary.contains("Workspace validate summary"));
    assert!(validate_summary.contains("metrics:"));
    assert!(validate_summary.contains("agent_visible_tokens: ~"));
}

#[test]
fn workspace_map_local_name_policy_handles_root_level_simple_skill() {
    let dir = TempDir::new("workspace-simple-local-name");
    let root = dir.path().join("source-skill");
    let manifest = dir.path().join("build").join("skillspec.workspace.yml");
    write_file(
        &root.join("SKILL.md"),
        r#"---
name: root-skill
description: Root package.
---
# Root Skill
"#,
    );

    let map = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg(&root)
        .arg("--out")
        .arg(&manifest)
        .arg("--install-slug-policy")
        .arg("local-name")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["install_slug_policy"], "local-name");
    assert_eq!(map_report["source_shape"]["kind"], "single_skill");

    let manifest_yaml = fs::read_to_string(&manifest).unwrap();
    assert!(manifest_yaml.contains("install_slug_policy: local-name"));
    assert!(manifest_yaml.contains("install_slug: root-skill"));
    assert!(!manifest_yaml.contains("install_slug: source-skill--skill"));
}

#[test]
fn workspace_map_preserves_plugin_namespaces() {
    let dir = TempDir::new("workspace-plugin-map");
    let root = dir.path().join("claude-for-legal");
    let manifest = dir.path().join("build").join("skillspec.workspace.yml");
    let build = dir.path().join("workspace-build");

    write_file(
        &root
            .join("commercial-legal")
            .join(".claude-plugin")
            .join("plugin.json"),
        r#"{"name":"commercial-legal","version":"1.0.0"}"#,
    );
    write_file(
        &root
            .join("privacy-legal")
            .join(".claude-plugin")
            .join("plugin.json"),
        r#"{"name":"privacy-legal","version":"1.0.0"}"#,
    );
    write_file(
        &root
            .join("commercial-legal")
            .join("skills")
            .join("cold-start-interview")
            .join("SKILL.md"),
        r#"---
name: cold-start-interview
description: Commercial intake.
---
# Commercial Intake
"#,
    );
    write_file(
        &root
            .join("commercial-legal")
            .join("skills")
            .join("review")
            .join("SKILL.md"),
        r#"---
name: review
description: Review a commercial agreement.
---
# Review

Run `/cold-start-interview`.
Use `/privacy-legal:use-case-triage` when privacy review is needed.
Read `../cold-start-interview/SKILL.md`.
"#,
    );
    write_file(
        &root
            .join("privacy-legal")
            .join("skills")
            .join("cold-start-interview")
            .join("SKILL.md"),
        r#"---
name: cold-start-interview
description: Privacy intake.
---
# Privacy Intake
"#,
    );
    write_file(
        &root
            .join("privacy-legal")
            .join("skills")
            .join("use-case-triage")
            .join("SKILL.md"),
        r#"---
name: use-case-triage
description: Privacy use-case triage.
---
# Use Case Triage
"#,
    );

    let map = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg(&root)
        .arg("--out")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["package_count"], 4);
    assert_eq!(map_report["source_shape"]["kind"], "plugin_workspace");
    assert_eq!(map_report["source_shape"]["skill_files"], 4);
    assert_eq!(
        map_report["source_shape"]["plugin_roots"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(map_report["duplicate_public_names"]
        .as_array()
        .unwrap()
        .is_empty());

    let namespaces = map_report["plugin_namespaces"].as_array().unwrap();
    assert!(namespaces.iter().any(|namespace| {
        namespace["namespace"] == "commercial-legal"
            && namespace["path"] == "commercial-legal"
            && namespace["packages"].as_array().unwrap().len() == 2
    }));
    assert!(namespaces.iter().any(|namespace| {
        namespace["namespace"] == "privacy-legal"
            && namespace["path"] == "privacy-legal"
            && namespace["packages"].as_array().unwrap().len() == 2
    }));

    let references = map_report["references"].as_array().unwrap();
    assert!(references.iter().any(|reference| {
        reference["from_package"] == "commercial-legal.skills.review"
            && reference["raw"] == "/cold-start-interview"
            && reference["kind"] == "skill_invocation"
            && reference["target_package"] == "commercial-legal.skills.cold-start-interview"
    }));
    assert!(references.iter().any(|reference| {
        reference["from_package"] == "commercial-legal.skills.review"
            && reference["raw"] == "/privacy-legal:use-case-triage"
            && reference["kind"] == "skill_invocation"
            && reference["target_package"] == "privacy-legal.skills.use-case-triage"
    }));
    assert!(references.iter().any(|reference| {
        reference["from_package"] == "commercial-legal.skills.review"
            && reference["raw"] == "../cold-start-interview/SKILL.md"
            && reference["kind"] == "file"
            && reference["target_package"] == "commercial-legal.skills.cold-start-interview"
    }));

    let edges = map_report["dependency_edges"].as_array().unwrap();
    assert!(edges.iter().any(|edge| {
        edge["from"] == "commercial-legal.skills.review"
            && edge["to"] == "commercial-legal.skills.cold-start-interview"
    }));
    assert!(!edges.iter().any(|edge| {
        edge["from"] == "commercial-legal.skills.review"
            && edge["to"] == "privacy-legal.skills.use-case-triage"
    }));

    let manifest_yaml = fs::read_to_string(&manifest).unwrap();
    assert!(manifest_yaml.contains("namespace: commercial-legal"));
    assert!(manifest_yaml.contains("local_name: review"));
    assert!(manifest_yaml.contains("public_name: commercial-legal-review"));
    assert!(manifest_yaml.contains("public_name: privacy-legal-cold-start-interview"));

    let validate = Command::new(bin())
        .arg("workspace")
        .arg("validate")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&validate);
    let validate_report = json_stdout(&validate);
    assert_eq!(validate_report["ok"], true);

    let import = Command::new(bin())
        .arg("workspace")
        .arg("import")
        .arg(&manifest)
        .arg("--out")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&import);

    promote_workspace_scaffolds_with_per_package_proof(&build);

    let compile = Command::new(bin())
        .arg("workspace")
        .arg("compile")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("codex-skill")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&compile);
    let compile_report = json_stdout(&compile);
    assert_eq!(compile_report["ok"], true);

    let commercial_loader = fs::read_to_string(
        build
            .join("commercial-legal")
            .join("skills")
            .join("review")
            .join("SKILL.md"),
    )
    .unwrap();
    assert!(commercial_loader.contains("name: commercial-legal-review"));
    let privacy_loader = fs::read_to_string(
        build
            .join("privacy-legal")
            .join("skills")
            .join("cold-start-interview")
            .join("SKILL.md"),
    )
    .unwrap();
    assert!(privacy_loader.contains("name: privacy-legal-cold-start-interview"));
}

#[test]
fn workspace_map_local_name_policy_reports_plugin_slug_collisions() {
    let dir = TempDir::new("workspace-plugin-local-name-collision");
    let root = dir.path().join("plugin-workspace");
    let manifest = dir.path().join("build").join("skillspec.workspace.yml");

    write_file(
        &root
            .join("alpha")
            .join(".claude-plugin")
            .join("plugin.json"),
        r#"{"name":"alpha","version":"1.0.0"}"#,
    );
    write_file(
        &root.join("beta").join(".claude-plugin").join("plugin.json"),
        r#"{"name":"beta","version":"1.0.0"}"#,
    );
    write_file(
        &root
            .join("alpha")
            .join("skills")
            .join("review")
            .join("SKILL.md"),
        "---\nname: review\ndescription: Alpha review.\n---\n# Review\n",
    );
    write_file(
        &root
            .join("beta")
            .join("skills")
            .join("review")
            .join("SKILL.md"),
        "---\nname: review\ndescription: Beta review.\n---\n# Review\n",
    );

    let map = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg(&root)
        .arg("--out")
        .arg(&manifest)
        .arg("--install-slug-policy")
        .arg("local-name")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["install_slug_policy"], "local-name");
    assert!(map_report["duplicate_install_slugs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|duplicate| duplicate["value"] == "review"));

    let validate = Command::new(bin())
        .arg("workspace")
        .arg("validate")
        .arg(&manifest)
        .output()
        .unwrap();
    assert_failure(&validate);
    assert!(
        stdout(&validate).contains("plugin-shaped workspaces must preserve plugin/package shape")
    );
    assert!(stdout(&validate).contains("duplicate install_slug \"review\""));
}

#[test]
fn workspace_import_fans_out_packages_under_build_root() {
    let dir = TempDir::new("workspace-import");
    let root = dir.path().join("skills");
    let manifest = dir.path().join("build").join("skillspec.workspace.yml");
    let build = dir.path().join("workspace-build");
    write_file(
        &root.join("coding-standards").join("SKILL.md"),
        r#"---
name: coding-standards
description: TypeScript coding standards package.
---
# Coding Standards

Use strict tests.
"#,
    );
    write_file(
        &root.join("code-review").join("SKILL.md"),
        r#"---
name: code-review
description: Review code.
disable-model-invocation: true
---
# Code Review

Always preserve the review checklist.
Read `../coding-standards/SKILL.md`.
"#,
    );

    let map = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg(&root)
        .arg("--out")
        .arg(&manifest)
        .output()
        .unwrap();
    assert_success(&map);

    let import = Command::new(bin())
        .arg("workspace")
        .arg("import")
        .arg(&manifest)
        .arg("--out")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&import);
    let report = json_stdout(&import);
    assert_eq!(report["ok"], true);
    assert_eq!(report["built"].as_array().unwrap().len(), 2);
    assert!(report["failed"].as_array().unwrap().is_empty());
    assert!(report["blocked"].as_array().unwrap().is_empty());

    let shared_spec = build.join("coding-standards").join("skill.spec.yml");
    let review_spec = build.join("code-review").join("skill.spec.yml");
    assert!(shared_spec.is_file());
    assert!(review_spec.is_file());
    assert!(build.join("skillspec.workspace.yml").is_file());
    assert!(build.join("workspace-import.report.md").is_file());
    assert!(build
        .join("code-review")
        .join(".skillspec/source-map/source-map.json")
        .is_file());
    assert!(build
        .join("code-review")
        .join(".skillspec/reports/doctor.json")
        .is_file());
    assert!(build
        .join("code-review")
        .join(".skillspec/workspace-import.json")
        .is_file());
    let review_import_evidence: Value = serde_json::from_str(
        &fs::read_to_string(build.join("code-review/.skillspec/workspace-import.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(review_import_evidence["package_count"], 2);
    assert!(review_import_evidence["package_index"].as_u64().unwrap() >= 1);
    assert!(review_import_evidence["remaining_after"].as_u64().unwrap() < 2);

    let import_summary = Command::new(bin())
        .arg("workspace")
        .arg("import")
        .arg(&manifest)
        .arg("--out")
        .arg(&build)
        .arg("--summary")
        .output()
        .unwrap();
    assert_success(&import_summary);
    let import_summary = stdout(&import_summary);
    assert!(import_summary.contains("Workspace import summary"));
    assert!(import_summary.contains("- built: 0"));
    assert!(import_summary.contains("- cached: 2"));
    assert!(import_summary.contains("metrics:"));
    assert!(import_summary.contains("wall_clock:"));
    assert!(import_summary.contains("agent_visible_tokens: ~"));
    assert!(import_summary.contains("artifact_tokens_preserved: ~"));
    assert!(import_summary.contains("avoided_tokens: ~"));
    assert!(import_summary.contains("cache_hits: 2"));
    assert!(import_summary.contains("cache_misses: 0"));
    assert!(import_summary.contains("report:"));
    assert!(build.join(".skillspec/workspace-cache.json").is_file());

    let validate_shared = Command::new(bin())
        .arg("validate")
        .arg(&shared_spec)
        .output()
        .unwrap();
    assert_success(&validate_shared);
    let validate_review = Command::new(bin())
        .arg("validate")
        .arg(&review_spec)
        .output()
        .unwrap();
    assert_success(&validate_review);

    let scaffold_converge = Command::new(bin())
        .arg("workspace")
        .arg("converge")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&scaffold_converge);
    let scaffold_converge_report = json_stdout(&scaffold_converge);
    assert_eq!(scaffold_converge_report["ok"], false);
    assert!(scaffold_converge_report["ready"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        scaffold_converge_report["blocked"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let scaffold_message = scaffold_converge_report["packages"][0]["message"]
        .as_str()
        .unwrap();
    assert!(scaffold_message.contains("generated mechanical scaffold"));
    assert!(scaffold_message.contains("semantic promotion"));
    assert!(scaffold_converge_report["next"][0]
        .as_str()
        .unwrap()
        .contains("complete scaffold promotion"));
    assert!(!scaffold_converge_report["next"][0]
        .as_str()
        .unwrap()
        .contains("workspace compile"));

    write_placeholder_loaders_for_all_workspace_specs(&build);
    let scaffold_install = Command::new(bin())
        .arg("workspace")
        .arg("install")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("agents")
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&scaffold_install);
    let scaffold_install_report = json_stdout(&scaffold_install);
    assert_eq!(scaffold_install_report["ok"], false);
    assert!(scaffold_install_report["planned"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        scaffold_install_report["blocked"].as_array().unwrap().len(),
        2
    );
    assert!(scaffold_install_report["packages"][0]["message"]
        .as_str()
        .unwrap()
        .contains("generated mechanical scaffold"));
    assert!(scaffold_install_report["next"][0]
        .as_str()
        .unwrap()
        .contains("complete scaffold promotion"));
    assert!(!scaffold_install_report["next"][0]
        .as_str()
        .unwrap()
        .contains("without --dry-run"));

    write_prose_wrappers_for_all_workspace_specs(&build);
    write_workspace_promotion_proofs_for_all(&build);
    let wrapper_converge = Command::new(bin())
        .arg("workspace")
        .arg("converge")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&wrapper_converge);
    let wrapper_converge_report = json_stdout(&wrapper_converge);
    assert_eq!(wrapper_converge_report["ok"], false);
    let wrapper_messages = workspace_package_messages(&wrapper_converge_report);
    assert!(
        wrapper_messages.contains("delegates execution to original prose instructions"),
        "messages:\n{wrapper_messages}"
    );
    assert!(
        wrapper_messages.contains("runtime source material"),
        "messages:\n{wrapper_messages}"
    );
    remove_workspace_promotion_proofs_for_all(&build);

    promote_all_workspace_scaffolds_without_proof(&build);
    let unproven_converge = Command::new(bin())
        .arg("workspace")
        .arg("converge")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&unproven_converge);
    let unproven_converge_report = json_stdout(&unproven_converge);
    assert_eq!(unproven_converge_report["ok"], false);
    assert_eq!(
        unproven_converge_report["blocked"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(unproven_converge_report["packages"][0]["message"]
        .as_str()
        .unwrap()
        .contains("missing workspace promotion proof"));

    write_workspace_promotion_proofs_without_review_session_for_all(&build);
    let no_countdown_converge = Command::new(bin())
        .arg("workspace")
        .arg("converge")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&no_countdown_converge);
    let no_countdown_converge_report = json_stdout(&no_countdown_converge);
    assert_eq!(no_countdown_converge_report["ok"], false);
    let no_countdown_messages = workspace_package_messages(&no_countdown_converge_report);
    assert!(
        no_countdown_messages.contains("missing per-package review_session countdown"),
        "messages:\n{no_countdown_messages}"
    );

    write_workspace_promotion_proofs_without_coverage_for_all(&build);
    let uncovered_converge = Command::new(bin())
        .arg("workspace")
        .arg("converge")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&uncovered_converge);
    let uncovered_converge_report = json_stdout(&uncovered_converge);
    assert_eq!(uncovered_converge_report["ok"], false);
    let uncovered_messages = workspace_package_messages(&uncovered_converge_report);
    assert!(
        uncovered_messages.contains("missing source obligation coverage proof"),
        "messages:\n{uncovered_messages}"
    );

    write_workspace_promotion_proofs_with_route_only_coverage_for_all(&build);
    let shallow_converge = Command::new(bin())
        .arg("workspace")
        .arg("converge")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&shallow_converge);
    let shallow_converge_report = json_stdout(&shallow_converge);
    assert_eq!(shallow_converge_report["ok"], false);
    let shallow_messages = workspace_package_messages(&shallow_converge_report);
    assert!(
        shallow_messages.contains("requires one of target kind(s)"),
        "messages:\n{shallow_messages}"
    );

    write_workspace_promotion_proofs_for_all(&build);

    let converge = Command::new(bin())
        .arg("workspace")
        .arg("converge")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&converge);
    let converge_report = json_stdout(&converge);
    assert_eq!(converge_report["ok"], true);
    assert_eq!(converge_report["ready"].as_array().unwrap().len(), 2);
    assert!(converge_report["failed"].as_array().unwrap().is_empty());
    assert!(converge_report["blocked"].as_array().unwrap().is_empty());
    assert!(build.join("workspace-converge.report.md").is_file());

    let compile = Command::new(bin())
        .arg("workspace")
        .arg("compile")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("codex-skill")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&compile);
    let compile_report = json_stdout(&compile);
    assert_eq!(compile_report["ok"], true);
    assert_eq!(compile_report["compiled"].as_array().unwrap().len(), 2);
    assert!(compile_report["failed"].as_array().unwrap().is_empty());
    assert!(compile_report["blocked"].as_array().unwrap().is_empty());
    assert!(build.join("workspace-compile.report.md").is_file());
    let review_loader = build.join("code-review").join("SKILL.md");
    assert!(review_loader.is_file());
    let loader = fs::read_to_string(review_loader).unwrap();
    assert!(loader.contains("name: code-review"));
    assert!(loader.contains("skillspec run-loop <skill_dir>/skill.spec.yml"));
    assert!(loader.contains("--guide agent"));
    assert!(loader.contains("skill.spec.yml"));
    assert!(!loader.contains("## Runtime Contract"));
    assert!(!loader.contains("## Completion Report"));

    let replacement_home = dir.path().join("replacement-home");
    fs::create_dir_all(replacement_home.join(".agents/skills")).unwrap();
    write_file(
        &replacement_home
            .join(".agents/skills")
            .join("code-review")
            .join("SKILL.md"),
        "---\nname: code-review\ndescription: Old prose skill.\n---\n# Old\n",
    );
    let replacement_plan = Command::new(bin())
        .env("HOME", &replacement_home)
        .arg("workspace")
        .arg("install")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("agents")
        .arg("--install-slug-policy")
        .arg("local-name")
        .arg("--retire-existing")
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&replacement_plan);
    let replacement_plan = json_stdout(&replacement_plan);
    assert_eq!(replacement_plan["install_slug_policy"], "local-name");
    let review_package = replacement_plan["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["package_id"] == "code-review")
        .unwrap();
    assert_eq!(review_package["install_slug"], "code-review");
    let review_target = &review_package["targets"][0];
    assert!(review_target["path"]
        .as_str()
        .unwrap()
        .ends_with("/.agents/skills/code-review"));
    assert_eq!(review_target["existed"], true);
    assert_eq!(review_target["retired_existing"], true);
    assert!(!replacement_home
        .join(".agents/skills/skills--code-review/SKILL.md")
        .exists());

    let collision_home = dir.path().join("collision-home");
    let collision_skillspec_home = dir.path().join("collision-skillspec-home");
    fs::create_dir_all(collision_home.join(".agents/skills")).unwrap();
    let legacy_collision_dir = collision_home
        .join(".agents/skills")
        .join("legacy-code-review");
    write_file(
        &legacy_collision_dir.join("SKILL.md"),
        "---\nname: code-review\ndescription: Existing skill.\n---\n# Existing\n",
    );
    let blocked_install = Command::new(bin())
        .env("HOME", &collision_home)
        .arg("workspace")
        .arg("install")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&blocked_install);
    let blocked_install_report = json_stdout(&blocked_install);
    assert_eq!(blocked_install_report["ok"], false);
    assert!(blocked_install_report["planned"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "coding-standards"));
    assert!(blocked_install_report["blocked"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "code-review"));
    assert!(!collision_home
        .join(".agents/skills/skills--coding-standards/SKILL.md")
        .exists());

    let retire_collision_plan = Command::new(bin())
        .env("HOME", &collision_home)
        .env("SKILLSPEC_HOME", &collision_skillspec_home)
        .arg("workspace")
        .arg("install")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("agents")
        .arg("--retire-existing")
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&retire_collision_plan);
    let retire_collision_plan = json_stdout(&retire_collision_plan);
    let review_package = retire_collision_plan["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["package_id"] == "code-review")
        .unwrap();
    let review_target = &review_package["targets"][0];
    assert!(review_target["public_name_collisions"][0]
        .as_str()
        .unwrap()
        .ends_with("/.agents/skills/legacy-code-review"));
    let public_name_retirement = &review_target["retired_public_name_collisions"][0];
    assert!(public_name_retirement["path"]
        .as_str()
        .unwrap()
        .ends_with("/.agents/skills/legacy-code-review"));
    assert!(public_name_retirement["backup_path"]
        .as_str()
        .unwrap()
        .contains("backups/retired-skills"));
    assert!(legacy_collision_dir.join("SKILL.md").is_file());

    let retire_collision_install = Command::new(bin())
        .env("HOME", &collision_home)
        .env("SKILLSPEC_HOME", &collision_skillspec_home)
        .arg("workspace")
        .arg("install")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("agents")
        .arg("--retire-existing")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&retire_collision_install);
    let retire_collision_install = json_stdout(&retire_collision_install);
    let review_package = retire_collision_install["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["package_id"] == "code-review")
        .unwrap();
    let public_name_retirement = &review_package["targets"][0]["retired_public_name_collisions"][0];
    let backup_path = PathBuf::from(public_name_retirement["backup_path"].as_str().unwrap());
    assert!(backup_path.join("SKILL.md").is_file());
    assert!(!legacy_collision_dir.exists());
    assert!(collision_home
        .join(".agents/skills/skills--code-review/SKILL.md")
        .is_file());

    let home = dir.path().join("home");
    fs::create_dir_all(home.join(".agents/skills")).unwrap();
    let install_dry_run = Command::new(bin())
        .env("HOME", &home)
        .arg("workspace")
        .arg("install")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("agents")
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_dry_run);
    let install_plan = json_stdout(&install_dry_run);
    assert_eq!(install_plan["ok"], true);
    assert_eq!(install_plan["dry_run"], true);
    assert_eq!(install_plan["visibility_policy"], "entry-implicit");
    assert_eq!(install_plan["planned"].as_array().unwrap().len(), 2);
    assert!(
        install_plan["visibility"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["package_id"] == "code-review"
                && item["target_visibility"] == "implicit")
    );
    assert!(install_plan["visibility"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["package_id"] == "coding-standards"
            && item["target_visibility"] == "manual-only"));
    assert!(build.join("workspace-install.report.md").is_file());
    assert!(!home
        .join(".agents/skills/skills--code-review/SKILL.md")
        .exists());

    let install = Command::new(bin())
        .env("HOME", &home)
        .arg("workspace")
        .arg("install")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("agents")
        .arg("--apply-visibility")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install);
    let install_report = json_stdout(&install);
    assert_eq!(install_report["ok"], true);
    assert_eq!(install_report["dry_run"], false);
    assert_eq!(install_report["apply_visibility"], true);
    assert_eq!(install_report["installed"].as_array().unwrap().len(), 2);
    assert!(install_report["planned"].as_array().unwrap().is_empty());
    assert!(install_report["router_refresh_recommended"]
        .as_bool()
        .unwrap());
    assert!(home
        .join(".agents/skills/skills--coding-standards/SKILL.md")
        .is_file());
    assert!(home
        .join(".agents/skills/skills--coding-standards/skill.spec.yml")
        .is_file());
    assert!(home
        .join(".agents/skills/skills--code-review/SKILL.md")
        .is_file());
    assert!(home
        .join(".agents/skills/skills--code-review/skill.spec.yml")
        .is_file());
    assert!(home
        .join(".agents/skills/skills--code-review/source/SKILL_md.old")
        .is_file());
    assert!(home
        .join(".agents/skills/skills--code-review/.skillspec/source-map/source-map.json")
        .is_file());
    let support_visibility =
        fs::read_to_string(home.join(".agents/skills/skills--coding-standards/agents/openai.yaml"))
            .unwrap();
    assert!(support_visibility.contains("allow_implicit_invocation: false"));
    let entry_visibility = home.join(".agents/skills/skills--code-review/agents/openai.yaml");
    assert!(
        !entry_visibility.exists(),
        "entry package should remain implicit without a disabling sidecar"
    );
    assert!(build.join("workspace-install.manifest.json").is_file());
    assert!(build.join("workspace-visibility.manifest.json").is_file());
    let install_manifest =
        fs::read_to_string(build.join("workspace-install.manifest.json")).unwrap();
    assert!(install_manifest.contains("\"visibility\""));
    assert!(install_manifest.contains("\"target\": \"manual-only\""));
}

fn promote_workspace_scaffolds_with_per_package_proof(build: &std::path::Path) {
    promote_all_workspace_scaffolds_without_proof(build);
    write_workspace_promotion_proofs_for_all(build);
}

fn promote_all_workspace_scaffolds_without_proof(build: &std::path::Path) {
    let specs = workspace_package_specs(build);
    assert!(
        !specs.is_empty(),
        "expected workspace build to contain package specs"
    );
    for spec_path in specs {
        promote_workspace_scaffold(build, &spec_path);
    }
}

fn write_workspace_promotion_proofs_for_all(build: &std::path::Path) {
    let specs = workspace_package_specs(build);
    assert!(
        !specs.is_empty(),
        "expected workspace build to contain package specs"
    );
    for spec_path in specs {
        write_workspace_promotion_proof(spec_path.parent().unwrap());
    }
}

fn write_workspace_promotion_proofs_without_coverage_for_all(build: &std::path::Path) {
    let specs = workspace_package_specs(build);
    assert!(
        !specs.is_empty(),
        "expected workspace build to contain package specs"
    );
    for spec_path in specs {
        write_workspace_promotion_proof_without_coverage(spec_path.parent().unwrap());
    }
}

fn write_workspace_promotion_proofs_with_route_only_coverage_for_all(build: &std::path::Path) {
    let specs = workspace_package_specs(build);
    assert!(
        !specs.is_empty(),
        "expected workspace build to contain package specs"
    );
    for spec_path in specs {
        write_workspace_promotion_proof_with_route_only_coverage(spec_path.parent().unwrap());
    }
}

fn write_workspace_promotion_proofs_without_review_session_for_all(build: &std::path::Path) {
    let specs = workspace_package_specs(build);
    assert!(
        !specs.is_empty(),
        "expected workspace build to contain package specs"
    );
    for spec_path in specs {
        let out = spec_path.parent().unwrap();
        write_workspace_promotion_proof(out);
        let proof_path = out.join(".skillspec/workspace-promotion.json");
        let mut proof: Value =
            serde_json::from_str(&fs::read_to_string(&proof_path).unwrap()).unwrap();
        proof.as_object_mut().unwrap().remove("review_session");
        write_file(
            &proof_path,
            &format!("{}\n", serde_json::to_string_pretty(&proof).unwrap()),
        );
    }
}

fn remove_workspace_promotion_proofs_for_all(build: &std::path::Path) {
    for spec_path in workspace_package_specs(build) {
        let _ = fs::remove_file(
            spec_path
                .parent()
                .unwrap()
                .join(".skillspec/workspace-promotion.json"),
        );
    }
}

fn write_placeholder_loaders_for_all_workspace_specs(build: &std::path::Path) {
    let specs = workspace_package_specs(build);
    assert!(
        !specs.is_empty(),
        "expected workspace build to contain package specs"
    );
    for spec_path in specs {
        write_file(&spec_path.parent().unwrap().join("SKILL.md"), "# Loader\n");
    }
}

fn workspace_package_messages(report: &Value) -> String {
    report["packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|package| package["message"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn workspace_package_specs(build: &std::path::Path) -> Vec<std::path::PathBuf> {
    fn visit(dir: &std::path::Path, specs: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, specs);
            } else if path.file_name().and_then(|name| name.to_str()) == Some("skill.spec.yml") {
                specs.push(path);
            }
        }
    }

    let mut specs = Vec::new();
    visit(build, &mut specs);
    specs.sort();
    specs
}

fn promote_workspace_scaffold(build: &std::path::Path, spec_path: &std::path::Path) {
    let out = spec_path.parent().unwrap();
    let package_key = out
        .strip_prefix(build)
        .unwrap()
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("_");
    let route_token = sanitize_skill_id(&package_key);
    let title = format!("Reviewed Workspace Package {route_token}");
    write_file(
        spec_path,
        &format!(
            r#"schema: skillspec/v0
id: reviewed.{route_token}
title: {title}
description: Reviewed workspace package promoted from mechanical import.

routes:
  - id: execute
    label: Execute {title}
    rank: 1

rules:
  - id: route_by_package_name
    when:
      user_says_any:
        - {route_token}
        - {title}
    prefer: execute
    reason: Reviewed package route is source-backed.

resources:
  source_map_review:
    path: ".skillspec/source-map/source-map.json"
    role: reference
    used_by:
      - kind: route
        id: execute

metadata:
  source_preserved_as_evidence: source/SKILL_md.old
  source_map_preserved_as_evidence: ".skillspec/source-map/source-map.json"
"#
        ),
    );
    write_file(
        &out.join("deps.toml"),
        r#"# Reviewed dependency ledger.
schema_version = 1
generated_by = "manual semantic promotion"
review_required = false
dependency_count = 0
"#,
    );
}

fn write_prose_wrappers_for_all_workspace_specs(build: &std::path::Path) {
    let specs = workspace_package_specs(build);
    assert!(
        !specs.is_empty(),
        "expected workspace build to contain package specs"
    );
    for spec_path in specs {
        write_prose_wrapper_spec(build, &spec_path);
    }
}

fn write_prose_wrapper_spec(build: &std::path::Path, spec_path: &std::path::Path) {
    let out = spec_path.parent().unwrap();
    let package_key = out
        .strip_prefix(build)
        .unwrap()
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("_");
    let route_token = sanitize_skill_id(&package_key);
    let title = format!("Wrapped Workspace Package {route_token}");
    write_file(
        spec_path,
        &format!(
            r#"schema: skillspec/v0
id: wrapped.{route_token}
title: {title}
description: Wrapper around original prose instructions.
entry:
  prompt: Load the original instructions as authoritative runtime instructions.

routes:
  - id: execute
    label: Execute {title}
    rank: 1
    execution_plan:
      mode: ordered
      phases:
        - id: load_source_instructions
          owner_skill: wrapped.{route_token}
          description: Load the promoted original instructions and follow them.

rules:
  - id: route_by_package_name
    when:
      user_says_any:
        - {route_token}
    prefer: execute
    reason: Wrapper route delegates to source.

resources:
  skill_source:
    path: resources/source/SKILL_source.txt
    role: source_material
    description: Promoted original instructions for runtime guidance.
    used_by:
      - kind: route
        id: execute
"#
        ),
    );
    write_file(
        &out.join("resources/source/SKILL_source.txt"),
        "# Original instructions\n",
    );
    write_file(
        &out.join("deps.toml"),
        r#"# Reviewed dependency ledger.
schema_version = 1
generated_by = "manual semantic promotion"
review_required = false
dependency_count = 0
"#,
    );
}

fn write_workspace_promotion_proof(out: &std::path::Path) {
    write_workspace_promotion_proof_with_coverage(out, true);
}

fn write_workspace_promotion_proof_without_coverage(out: &std::path::Path) {
    write_workspace_promotion_proof_with_coverage(out, false);
}

fn write_workspace_promotion_proof_with_route_only_coverage(out: &std::path::Path) {
    write_workspace_promotion_proof_with_coverage_kind(out, true, true);
}

fn write_workspace_promotion_proof_with_coverage(out: &std::path::Path, include_coverage: bool) {
    write_workspace_promotion_proof_with_coverage_kind(out, include_coverage, false);
}

fn write_workspace_promotion_proof_with_coverage_kind(
    out: &std::path::Path,
    include_coverage: bool,
    route_only_coverage: bool,
) {
    let evidence_path = out.join(".skillspec/workspace-import.json");
    let evidence: Value = serde_json::from_str(&fs::read_to_string(&evidence_path).unwrap())
        .expect("workspace import evidence should be valid JSON");
    let package_id = evidence["package_id"]
        .as_str()
        .expect("package evidence should include package_id");
    let source_path = PathBuf::from(
        evidence["source_path"]
            .as_str()
            .expect("package evidence should include source_path"),
    );
    let spec_path = PathBuf::from(
        evidence["spec_path"]
            .as_str()
            .expect("package evidence should include spec_path"),
    );
    let source_map_path = PathBuf::from(
        evidence["source_map_path"]
            .as_str()
            .expect("package evidence should include source_map_path"),
    );
    let source_map_sha256 = file_hash(&source_map_path);
    let source_sha256 = package_source_hash(&source_path);
    let source_obligation_coverage = include_coverage.then(|| {
        let source_map: Value =
            serde_json::from_str(&fs::read_to_string(&source_map_path).unwrap()).unwrap();
        let obligations = if route_only_coverage {
            route_only_source_obligation_entries(&source_map)
        } else {
            source_obligation_entries(&source_map)
        };
        let promoted = obligations
            .iter()
            .filter(|entry| entry["disposition"] == "promoted")
            .count();
        let not_applicable = obligations
            .iter()
            .filter(|entry| entry["disposition"] == "not_applicable")
            .count();
        json!({
            "schema": "skillspec/source-obligation-coverage/v0",
            "total": obligations.len(),
            "promoted": promoted,
            "not_applicable": not_applicable,
            "unresolved": 0,
            "obligations": obligations
        })
    });
    let mut proof = json!({
        "schema": "skillspec/workspace-promotion/v0",
        "package_id": package_id,
        "status": "reviewed",
        "source_sha256": source_sha256,
        "spec_sha256": file_hash(&spec_path),
        "source_map_sha256": source_map_sha256,
        "review_session": {
            "package_id": package_id,
            "package_index": evidence["package_index"].as_u64().expect("package evidence should include package_index"),
            "package_count": evidence["package_count"].as_u64().expect("package evidence should include package_count"),
            "remaining_after": evidence["remaining_after"].as_u64().expect("package evidence should include remaining_after"),
            "reviewed_source": "SKILL.md",
            "source_sha256": source_sha256
        },
        "review": {
            "activation_reviewed": true,
            "routes_reviewed": true,
            "rules_reviewed": true,
            "dependencies_reviewed": true,
            "checks_or_tests_reviewed": true,
            "proof_reviewed": true
        }
    });
    if let Some(coverage) = source_obligation_coverage {
        proof["source_obligation_coverage"] = coverage;
    }
    write_file(
        &out.join(".skillspec/workspace-promotion.json"),
        &format!("{}\n", serde_json::to_string_pretty(&proof).unwrap()),
    );
}

fn source_obligation_entries(source_map: &Value) -> Vec<Value> {
    let mut obligations = Vec::new();
    let mut classified_targets = BTreeSet::new();
    for classification in source_map["classifications"].as_array().unwrap() {
        let status = classification["coverage_status"]
            .as_str()
            .unwrap_or_default();
        if matches!(status, "review_required" | "blocked") {
            let target = classification["target"].as_str().unwrap_or_default();
            let entry = source_obligation_entry(
                classification["id"].as_str().unwrap(),
                node_hash(source_map, target),
                classification_targets(classification),
            );
            obligations.push(entry);
            if !target.is_empty() {
                classified_targets.insert(target.to_owned());
            }
        }
    }

    for reference in source_map["references"].as_array().unwrap() {
        let target_kind = reference["target_kind"].as_str().unwrap_or_default();
        if matches!(target_kind, "local_file" | "external_uri") {
            obligations.push(source_obligation_entry(
                reference["id"].as_str().unwrap(),
                node_hash(source_map, reference["source"].as_str().unwrap_or_default()),
                Some(vec![json!({
                    "kind": "resource",
                    "id": "source_map_review"
                })]),
            ));
        }
    }

    for node in source_map["nodes"].as_array().unwrap() {
        let id = node["id"].as_str().unwrap();
        if classified_targets.contains(id) {
            continue;
        }
        let kind = node["kind"].as_str().unwrap_or_default();
        if matches!(kind, "root" | "frontmatter") {
            continue;
        }
        if node["coverage_status"].as_str() == Some("not_applicable") {
            continue;
        }
        let has_text = node["title"]
            .as_str()
            .or_else(|| node["text_preview"].as_str())
            .is_some_and(|text| !text.trim().is_empty());
        if has_text {
            obligations.push(source_obligation_entry(
                id,
                node["hash"].as_str().map(str::to_owned),
                Some(vec![json!({
                    "kind": "route",
                    "id": "execute"
                })]),
            ));
        }
    }

    obligations.sort_by(|left, right| {
        left["source"]
            .as_str()
            .unwrap()
            .cmp(right["source"].as_str().unwrap())
    });
    obligations
}

fn route_only_source_obligation_entries(source_map: &Value) -> Vec<Value> {
    source_obligation_entries(source_map)
        .into_iter()
        .map(|mut entry| {
            if entry["disposition"] == "promoted" {
                entry["targets"] = json!([
                    {
                        "kind": "route",
                        "id": "execute"
                    }
                ]);
            }
            entry
        })
        .collect()
}

fn source_obligation_entry(
    source: &str,
    source_hash: Option<String>,
    targets: Option<Vec<Value>>,
) -> Value {
    match targets {
        Some(targets) => {
            let mut entry = json!({
                "source": source,
                "disposition": "promoted",
                "targets": targets
            });
            if let Some(source_hash) = source_hash {
                entry["source_hash"] = json!(source_hash);
            }
            entry
        }
        None => {
            let mut entry = json!({
                "source": source,
                "disposition": "not_applicable",
                "reason": "reviewed source lens block; dependency mention has no external package requirement in this fixture"
            });
            if let Some(source_hash) = source_hash {
                entry["source_hash"] = json!(source_hash);
            }
            entry
        }
    }
}

fn classification_targets(classification: &Value) -> Option<Vec<Value>> {
    let kind = classification["kind"].as_str().unwrap_or_default();
    let suggested = classification["suggested_constructs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect::<BTreeSet<_>>();
    if kind == "dependency_mention" || suggested.contains("dependency") {
        return None;
    }
    if kind == "modal_obligation" || kind == "forbid_candidate" || suggested.contains("rule") {
        return Some(vec![json!({
            "kind": "rule",
            "id": "route_by_package_name"
        })]);
    }
    if kind == "code_block" || suggested.contains("code") || suggested.contains("resource") {
        return Some(vec![json!({
            "kind": "resource",
            "id": "source_map_review"
        })]);
    }
    Some(vec![json!({
        "kind": "route",
        "id": "execute"
    })])
}

fn node_hash(source_map: &Value, node_id: &str) -> Option<String> {
    source_map["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"].as_str() == Some(node_id))
        .and_then(|node| node["hash"].as_str())
        .map(str::to_owned)
}

fn sanitize_skill_id(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_separator = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            output.push('_');
            last_was_separator = true;
        }
    }
    let trimmed = output.trim_matches('_').to_owned();
    if trimmed.is_empty() {
        "package".to_owned()
    } else {
        trimmed
    }
}

fn package_source_hash(source: &Path) -> String {
    let mut paths = Vec::new();
    collect_hashable_files(source, &mut paths);
    paths.sort();
    let mut hasher = Sha256::new();
    for path in paths {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update([0]);
        hasher.update(file_hash(&path).as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn collect_hashable_files(path: &Path, files: &mut Vec<PathBuf>) {
    if should_skip_hash_path(path) {
        return;
    }
    if path.is_file() {
        files.push(path.to_path_buf());
        return;
    }
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_hashable_files(&path, files);
        } else if !should_skip_hash_path(&path) {
            files.push(path);
        }
    }
}

fn should_skip_hash_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.') || matches!(name, "target" | "node_modules"))
}

fn file_hash(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(fs::read(path).unwrap());
    format!("{:x}", hasher.finalize())
}

#[test]
fn workspace_import_preserves_successes_and_blocks_dependents() {
    let dir = TempDir::new("workspace-import-failure");
    let root = dir.path().join("skills");
    let manifest = dir.path().join("build").join("skillspec.workspace.yml");
    let build = dir.path().join("workspace-build");
    write_file(
        &root.join("bad").join("SKILL.md"),
        "---\nname: bad\ndescription: Bad.\n---\n# Bad\n",
    );
    write_file(
        &root.join("good").join("SKILL.md"),
        "---\nname: good\ndescription: Good.\n---\n# Good\n",
    );
    write_file(
        &root.join("uses-bad").join("SKILL.md"),
        r#"---
name: uses-bad
description: Uses bad.
---
# Uses Bad

Read `../bad/SKILL.md`.
"#,
    );

    let map = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg(&root)
        .arg("--out")
        .arg(&manifest)
        .output()
        .unwrap();
    assert_success(&map);

    fs::write(root.join("bad").join("SKILL.md"), [0xff, 0xfe]).unwrap();

    let import = Command::new(bin())
        .arg("workspace")
        .arg("import")
        .arg(&manifest)
        .arg("--out")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&import);
    let report = json_stdout(&import);
    assert_eq!(report["ok"], false);
    assert!(report["built"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "good"));
    assert!(report["failed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "bad"));
    assert!(report["blocked"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "uses-bad"));
    assert!(build.join("good").join("skill.spec.yml").is_file());
    assert!(!build.join("uses-bad").join("skill.spec.yml").is_file());
    assert!(build.join("workspace-import.report.md").is_file());

    let converge = Command::new(bin())
        .arg("workspace")
        .arg("converge")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&converge);
    let converge_report = json_stdout(&converge);
    assert_eq!(converge_report["ok"], false);
    assert!(converge_report["ready"].as_array().unwrap().is_empty());
    assert!(converge_report["failed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "bad"));
    assert!(converge_report["blocked"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "good"));
    assert!(converge_report["blocked"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "uses-bad"));
    let good_converge = converge_report["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["package_id"] == "good")
        .unwrap();
    assert!(good_converge["message"]
        .as_str()
        .unwrap()
        .contains("generated mechanical scaffold"));
    assert!(build.join("workspace-converge.report.md").is_file());

    let compile = Command::new(bin())
        .arg("workspace")
        .arg("compile")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&build)
        .arg("--target")
        .arg("codex-skill")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&compile);
    let compile_report = json_stdout(&compile);
    assert_eq!(compile_report["ok"], false);
    assert!(compile_report["compiled"].as_array().unwrap().is_empty());
    assert!(compile_report["failed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "bad"));
    assert!(compile_report["blocked"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "good"));
    assert!(compile_report["blocked"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "uses-bad"));
    assert!(build.join("workspace-compile.report.md").is_file());
    assert!(!build.join("good").join("SKILL.md").is_file());
    assert!(!build.join("uses-bad").join("SKILL.md").is_file());
}

#[test]
fn workspace_validate_rejects_cycles() {
    let dir = TempDir::new("workspace-cycle");
    let root = dir.path().join("skills");
    write_file(
        &root.join("a").join("SKILL.md"),
        "---\nname: a\ndescription: A.\n---\n# A\n",
    );
    write_file(
        &root.join("b").join("SKILL.md"),
        "---\nname: b\ndescription: B.\n---\n# B\n",
    );
    let manifest = dir.path().join("skillspec.workspace.yml");
    write_file(
        &manifest,
        &format!(
            r#"schema: skillspec/workspace/v0
source_root: {}
workspace_slug: skills
output_root: {}/.skillspec/workspace-build
packages:
  a:
    package_id: a
    path: a
    kind: helper
    entrypoint: SKILL.md
    public_name: a
    install_slug: skills--a
    depends_on:
      - b
  b:
    package_id: b
    path: b
    kind: helper
    entrypoint: SKILL.md
    public_name: b
    install_slug: skills--b
    depends_on:
      - a
"#,
            root.display(),
            root.display()
        ),
    );

    let validate = Command::new(bin())
        .arg("workspace")
        .arg("validate")
        .arg(&manifest)
        .output()
        .unwrap();
    assert_failure(&validate);
    assert!(stdout(&validate).contains("dependency cycle"));
}

#[test]
fn import_skill_rejects_parent_folder_with_multiple_skills() {
    let dir = TempDir::new("import-multi");
    let root = dir.path().join("skills");
    let out = dir.path().join("draft").join("skill.spec.yml");
    write_file(
        &root.join("pdf").join("SKILL.md"),
        "---\nname: pdf\ndescription: PDF skill.\n---\n# PDF\n",
    );
    write_file(
        &root.join("csv").join("SKILL.md"),
        "---\nname: csv\ndescription: CSV skill.\n---\n# CSV\n",
    );

    let import = Command::new(bin())
        .arg("import-skill")
        .arg(&root)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert_failure(&import);
    let err = stderr(&import);
    assert!(err.contains("expects one atomic skill package"));
    assert!(err.contains("skillspec workspace map"));
}

#[test]
fn import_skill_keeps_reference_only_imports_connected() {
    let dir = TempDir::new("import-reference-only");
    let skill_dir = dir.path().join("source-skill");
    let out = dir.path().join("draft").join("skill.spec.yml");
    write_file(
        &skill_dir.join("SKILL.md"),
        r#"---
name: reference-only
description: Reference-only import fixture.
---
# Reference Only

Load VOCABULARY.md when terms matter.
"#,
    );
    write_file(
        &skill_dir.join("VOCABULARY.md"),
        "# Vocabulary\n\nNo code here.\n",
    );

    let import = Command::new(bin())
        .arg("import-skill")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&import);

    let validate = Command::new(bin())
        .arg("validate")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&validate);
    let yaml = fs::read_to_string(&out).unwrap();
    assert!(yaml.contains("kind: snippet"));
    assert!(yaml.contains("id: source_summary"));
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
