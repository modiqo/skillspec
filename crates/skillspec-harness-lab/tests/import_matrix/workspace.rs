use super::helpers::{read_file, workspace_import_output, workspace_map_output};
use skillspec_harness_lab::{assert_success, json_stdout, HarnessLab, HarnessLabReportBuilder};

pub fn workspace_import_fans_out_multiple_skills(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("workspace-import-fanout");
    let source = lab.root().join("skills");
    lab.write_file(
        &source.join("coding-standards").join("SKILL.md"),
        "---\nname: coding-standards\ndescription: TypeScript standards.\n---\n# Coding Standards\n",
    );
    lab.write_file(
        &source.join("code-review").join("SKILL.md"),
        r#"---
name: code-review
description: Review code.
---
# Code Review

Read `../coding-standards/SKILL.md`.
"#,
    );
    let manifest = lab.root().join("build").join("skillspec.workspace.yml");
    let build = lab.root().join("workspace-build");

    let map = workspace_map_output(&lab, &source, &manifest);
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["package_count"], 2);

    let import = workspace_import_output(&lab, &manifest, &build);
    assert_success(&import);
    let import_report = json_stdout(&import);
    assert_eq!(import_report["ok"], true);
    assert_eq!(import_report["built"].as_array().unwrap().len(), 2);
    assert!(import_report["failed"].as_array().unwrap().is_empty());
    assert!(import_report["blocked"].as_array().unwrap().is_empty());
    let shared_spec = build.join("coding-standards").join("skill.spec.yml");
    let review_spec = build.join("code-review").join("skill.spec.yml");
    assert!(shared_spec.is_file());
    assert!(review_spec.is_file());
    assert!(build.join("workspace-import.report.md").is_file());
    lab.assert_no_real_home_writes();

    let mut case = report.case("workspace_import_fans_out_multiple_skills");
    case.claim_pass("workspace.map.exit_success", true, map.status.success());
    case.claim_pass(
        "workspace.map.package_count",
        2,
        &map_report["package_count"],
    );
    case.claim_pass(
        "workspace.import.exit_success",
        true,
        import.status.success(),
    );
    case.claim_pass("workspace.import.ok", true, &import_report["ok"]);
    case.claim_pass(
        "workspace.import.built_count",
        2,
        import_report["built"].as_array().unwrap().len(),
    );
    case.claim_pass(
        "workspace.import.failed_count",
        0,
        import_report["failed"].as_array().unwrap().len(),
    );
    case.claim_pass("workspace.import.shared_spec", true, shared_spec.is_file());
    case.claim_pass("workspace.import.review_spec", true, review_spec.is_file());
    case.finish();
}

pub fn workspace_import_preserves_plugin_namespace(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("workspace-import-plugin");
    let source = lab.root().join("claude-plugin");
    lab.write_file(
        &source
            .join("commercial")
            .join(".claude-plugin")
            .join("plugin.json"),
        r#"{"name":"commercial-legal","version":"1.0.0"}"#,
    );
    lab.write_file(
        &source
            .join("commercial")
            .join("skills")
            .join("review")
            .join("SKILL.md"),
        "---\nname: review\ndescription: Review.\n---\n# Review\n",
    );
    let manifest = lab.root().join("build").join("skillspec.workspace.yml");
    let build = lab.root().join("workspace-build");

    let map = workspace_map_output(&lab, &source, &manifest);
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["package_count"], 1);
    let namespaces = map_report["plugin_namespaces"].as_array().unwrap();
    assert!(namespaces
        .iter()
        .any(|namespace| namespace["namespace"] == "commercial-legal"));

    let import = workspace_import_output(&lab, &manifest, &build);
    assert_success(&import);
    let import_report = json_stdout(&import);
    assert_eq!(import_report["ok"], true);
    assert_eq!(import_report["built"].as_array().unwrap().len(), 1);
    let plugin_spec = build
        .join("commercial")
        .join("skills")
        .join("review")
        .join("skill.spec.yml");
    assert!(plugin_spec.is_file());
    let manifest_text = read_file(&manifest);
    assert!(manifest_text.contains("namespace: commercial-legal"));
    assert!(manifest_text.contains("public_name: commercial-legal-review"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("workspace_import_preserves_plugin_namespace");
    case.claim_pass("workspace.map.exit_success", true, map.status.success());
    case.claim_pass(
        "workspace.map.package_count",
        1,
        &map_report["package_count"],
    );
    case.claim_pass(
        "workspace.map.namespace",
        true,
        namespaces
            .iter()
            .any(|namespace| namespace["namespace"] == "commercial-legal"),
    );
    case.claim_pass(
        "workspace.import.exit_success",
        true,
        import.status.success(),
    );
    case.claim_pass("workspace.import.ok", true, &import_report["ok"]);
    case.claim_pass(
        "workspace.import.built_count",
        1,
        import_report["built"].as_array().unwrap().len(),
    );
    case.claim_pass("workspace.import.plugin_spec", true, plugin_spec.is_file());
    case.claim_pass(
        "workspace.manifest.public_name",
        true,
        manifest_text.contains("public_name: commercial-legal-review"),
    );
    case.finish();
}
