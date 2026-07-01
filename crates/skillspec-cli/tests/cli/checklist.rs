use crate::support::*;

#[test]
fn doctor_checklist_is_shape_specific_for_single_skill() {
    let dir = TempDir::new("doctor-checklist-single");
    let source = dir.path().join("source");
    write_file(
        &source.join("SKILL.md"),
        "---\nname: single\ndescription: Single skill.\n---\n# Single\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg("checklist")
        .arg(&source)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["schema"], "skillspec/checklist/v0");
    assert_eq!(report["kind"], "doctor");
    assert_eq!(report["stage"], "entry");
    assert_eq!(report["status"], "ready");
    assert_eq!(report["entity"]["shape"], "simple_skill");
    assert_eq!(report["activation_policy"], "single_activation_skill");
    assert_eq!(report["steps"][0]["id"], "single_skill_entry");
    assert!(report["forbid"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str() == Some("bulk_promote_scaffolds")));

    let legacy_doctor = Command::new(bin())
        .arg("doctor")
        .arg(&source)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&legacy_doctor);
    assert_eq!(json_stdout(&legacy_doctor)["shape"]["kind"], "simple_skill");
}

#[test]
fn doctor_checklist_preserves_plugin_activation_policy() {
    let dir = TempDir::new("doctor-checklist-plugin");
    let root = dir.path().join("repo");
    write_file(
        &root.join(".agent-plugin").join("marketplace.json"),
        r#"{"name":"privacy-legal","version":"1.0.0"}"#,
    );
    write_file(
        &root.join("skills").join("redaction").join("SKILL.md"),
        "---\nname: redaction\ndescription: Redaction.\n---\n# Redaction\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg("checklist")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["entity"]["shape"], "plugin_workspace");
    assert_eq!(report["activation_policy"], "preserve_plugin_activation");
    assert_eq!(report["steps"][0]["id"], "plugin_entry");
    let forbids = report["forbid"].as_array().unwrap();
    assert!(forbids
        .iter()
        .any(|item| item.as_str() == Some("flatten_plugin_shape")));
}

#[test]
fn import_checklist_requires_build_root_for_workspace_loop() {
    let dir = TempDir::new("import-checklist-blocked");
    let manifest = dir.path().join("skillspec.workspace.yml");
    write_file(
        &manifest,
        r#"
schema: skillspec/workspace/v0
source_root: .
workspace_slug: sample
output_root: .
source_shape:
  kind: multi_skill_workspace
  skill_files: 1
packages:
  one:
    package_id: one
    path: one
    kind: entry
    entrypoint: SKILL.md
    public_name: One
    install_slug: one
"#,
    );

    let output = Command::new(bin())
        .arg("import")
        .arg("checklist")
        .arg(&manifest)
        .arg("--stage")
        .arg("loop")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&output);
    let report = json_stdout(&output);
    assert_eq!(report["status"], "blocked");
    assert!(report["blockers"][0]
        .as_str()
        .unwrap()
        .contains("--build-root"));
}

#[test]
fn import_checklist_loop_points_at_next_workspace_package() {
    let dir = TempDir::new("import-checklist-workspace-loop");
    let source = dir.path().join("skills");
    write_file(
        &source.join("alpha").join("SKILL.md"),
        "---\nname: alpha\ndescription: Alpha skill.\n---\n# Alpha\n\nIf alpha applies, run alpha.\n",
    );
    write_file(
        &source.join("beta").join("SKILL.md"),
        "---\nname: beta\ndescription: Beta skill.\n---\n# Beta\n\nIf beta applies, run beta.\n",
    );
    let build = dir.path().join("build");
    let manifest = build.join("skillspec.workspace.yml");
    let workspace_build = dir.path().join("workspace-build");

    let map = Command::new(bin())
        .arg("workspace")
        .arg("map")
        .arg(&source)
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
        .arg(&workspace_build)
        .output()
        .unwrap();
    assert_success(&import);

    let checklist = Command::new(bin())
        .arg("import")
        .arg("checklist")
        .arg(&manifest)
        .arg("--build-root")
        .arg(&workspace_build)
        .arg("--stage")
        .arg("loop")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&checklist);
    let report = json_stdout(&checklist);
    assert_eq!(report["kind"], "import");
    assert_eq!(report["stage"], "loop");
    assert_eq!(report["status"], "ready");
    assert_eq!(report["steps"][0]["id"], "promote_workspace_package");
    assert!(report["position"]["package_id"].as_str().is_some());
    assert_eq!(report["position"]["package_index"], 1);
    assert_eq!(report["position"]["package_count"], 2);
    let commands = report["steps"][0]["commands"].as_array().unwrap();
    assert!(commands
        .iter()
        .any(|command| command.as_str().unwrap().contains("skillspec source lens")));
    assert!(report["steps"][0]["forbid"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str() == Some("apply_ruby_yaml_generator_across_packages")));
}

#[test]
fn run_checklist_handles_specs_and_blocked_run_dirs() {
    let dir = TempDir::new("run-checklist");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let spec_output = Command::new(bin())
        .arg("run")
        .arg("checklist")
        .arg(&spec)
        .arg("--stage")
        .arg("entry")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&spec_output);
    let spec_report = json_stdout(&spec_output);
    assert_eq!(spec_report["kind"], "run");
    assert_eq!(spec_report["steps"][0]["id"], "start_guided_run");
    assert!(spec_report["next_command"]
        .as_str()
        .unwrap()
        .contains("skillspec run-loop"));

    let run_dir = dir.path().join("run-without-guide-state");
    fs::create_dir_all(&run_dir).unwrap();
    let blocked = Command::new(bin())
        .arg("run")
        .arg("checklist")
        .arg(&run_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&blocked);
    let blocked_report = json_stdout(&blocked);
    assert_eq!(blocked_report["status"], "blocked");
    assert_eq!(blocked_report["steps"][0]["id"], "missing_guide_state");
}

#[test]
fn sensemake_teaches_checklist_commands() {
    let dir = TempDir::new("sensemake-checklist");
    let spec = dir.path().join("skill.spec.yml");
    write_file(
        &spec,
        r#"
schema: skillspec/v0
id: checklist.nav
title: Checklist Navigation
description: Exercises checklist navigation hints.
commands:
  doctor_source_skill:
    template: skillspec doctor <source-skill-folder> --json
    safety: read_only
  import_source_skill:
    template: skillspec import-skill <source-skill-folder> --out <draft>/skill.spec.yml
    safety: read_only
  map_workspace:
    template: skillspec workspace map <source-root> --out <build>/skillspec.workspace.yml
    safety: read_only
"#,
    );

    let output = Command::new(bin())
        .arg("sensemake")
        .arg(&spec)
        .arg("--view")
        .arg("full")
        .output()
        .unwrap();
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("skillspec run checklist"));
    assert!(out.contains("skillspec doctor checklist"));
    assert!(out.contains("skillspec import checklist"));
}
