use skillspec_harness_lab::{
    assert_success, basic_skill_md, basic_skill_spec, json_stdout, HarnessLab,
};

#[test]
fn detects_sandbox_targets_from_lab_environment() {
    let lab = HarnessLab::new("detect-targets");

    let output = lab
        .command_in_project()
        .arg("install")
        .arg("targets")
        .output()
        .unwrap();
    assert_success(&output);

    let targets = json_stdout(&output);
    let targets = targets.as_array().expect("targets should be an array");
    assert_eq!(targets.len(), 3);

    assert_target(targets, "agents", &lab.agents_root());
    assert_target(targets, "codex", &lab.codex_root());
    assert_target(targets, "claude-local", &lab.claude_root());
    lab.assert_no_real_home_writes();
}

#[test]
fn command_outside_project_does_not_discover_claude_local_root() {
    let lab = HarnessLab::new("detect-no-claude-local");

    let output = lab
        .command()
        .arg("install")
        .arg("targets")
        .output()
        .unwrap();
    assert_success(&output);

    let targets = json_stdout(&output);
    let targets = targets.as_array().expect("targets should be an array");
    assert_eq!(targets.len(), 2);

    assert_target(targets, "agents", &lab.agents_root());
    assert_target(targets, "codex", &lab.codex_root());
    assert!(targets.iter().all(|target| target["id"] != "claude-local"));
    lab.assert_no_real_home_writes();
}

#[test]
fn installs_skill_into_all_sandbox_roots() {
    let lab = HarnessLab::new("install-all");
    let source_root = lab.root().join("fixtures");
    let skill_dir = lab.write_skill(
        &source_root,
        "example-skill",
        &basic_skill_md("example-skill"),
        Some(&basic_skill_spec("example.skill", "Example Skill")),
    );

    let output = lab
        .command_in_project()
        .arg("install")
        .arg("skill")
        .arg(&skill_dir)
        .arg("--all-detected")
        .output()
        .unwrap();
    assert_success(&output);

    let report = json_stdout(&output);
    assert_eq!(report["skill_name"], "example-skill");
    assert_eq!(report["dry_run"], false);
    assert_eq!(report["installs"].as_array().unwrap().len(), 3);

    for root in [lab.agents_root(), lab.codex_root(), lab.claude_root()] {
        assert!(root.join("example-skill/SKILL.md").is_file());
        assert!(root.join("example-skill/skill.spec.yml").is_file());
    }
    lab.assert_no_real_home_writes();
}

fn assert_target(targets: &[serde_json::Value], id: &str, expected_path: &std::path::Path) {
    let target = targets
        .iter()
        .find(|target| target["id"] == id)
        .unwrap_or_else(|| panic!("missing target {id} in {targets:#?}"));
    assert_eq!(target["detected"], true);
    let actual_path = std::path::PathBuf::from(target["path"].as_str().unwrap());
    assert_eq!(
        actual_path.canonicalize().unwrap(),
        expected_path.canonicalize().unwrap()
    );
}
