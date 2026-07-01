use skillspec_harness_lab::{
    assert_success, baseline_path, basic_skill_md, basic_skill_spec, compare_or_update_baseline,
    compare_reports, json_stdout, CaseStatus, HarnessLab, HarnessLabReport,
    HarnessLabReportBuilder,
};

const CORE_BASELINE: &str = include_str!("../baselines/09-harness-lab-core.json");

#[test]
fn core_phase_writes_machine_readable_report_card() {
    let report = run_core_phase();
    let lab = HarnessLab::new("write-report-card");
    let report_path = lab.write_report(&report);

    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, "09-harness-lab-core");
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 3);
    assert_eq!(report.summary.cases_passed, 3);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 9);
    assert_eq!(report.summary.claims_failed, 0);

    let comparison = compare_reports(&report, &report);
    assert_eq!(comparison.status, CaseStatus::Pass);
    assert!(comparison.regressions.is_empty());
    assert!(report_path.is_file());

    let written: skillspec_harness_lab::HarnessLabReport =
        serde_json::from_slice(&std::fs::read(report_path).unwrap()).unwrap();
    assert_eq!(written.phase, "09-harness-lab-core");
    assert_eq!(written.summary.status, CaseStatus::Pass);

    assert_matches_or_updates_baseline(&report);
}

#[test]
fn report_comparison_detects_missing_claim_regression() {
    let baseline = run_core_phase();
    let mut candidate = baseline.clone();
    candidate.cases[0].claims.pop();

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "claim_missing"));
}

fn run_core_phase() -> skillspec_harness_lab::HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new("09-harness-lab-core");
    detects_sandbox_targets_from_lab_environment(&mut report);
    command_outside_project_does_not_discover_claude_local_root(&mut report);
    installs_skill_into_all_sandbox_roots(&mut report);
    report.build()
}

fn assert_matches_or_updates_baseline(report: &HarnessLabReport) {
    let baseline_path = baseline_path("09-harness-lab-core");
    let comparison = compare_or_update_baseline("09-harness-lab-core", CORE_BASELINE, report);
    assert_eq!(
        comparison.status,
        CaseStatus::Pass,
        "harness lab report regressed against {}\n{}",
        baseline_path.display(),
        serde_json::to_string_pretty(&comparison).unwrap()
    );
}

fn detects_sandbox_targets_from_lab_environment(report: &mut HarnessLabReportBuilder) {
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

    let mut case = report.case("detects_sandbox_targets_from_lab_environment");
    case.claim_pass("install.targets.count", 3, targets.len());
    for (id, path) in [
        ("agents", lab.agents_root()),
        ("codex", lab.codex_root()),
        ("claude-local", lab.claude_root()),
    ] {
        case.claim_pass(
            format!("install.targets.{id}.detected"),
            true,
            target_detected(targets, id),
        );
        case.claim_pass(
            format!("install.targets.{id}.path"),
            lab.normalize_path(&path),
            lab.normalize_path(&target_path(targets, id)),
        );
    }
    case.finish();
}

fn command_outside_project_does_not_discover_claude_local_root(
    report: &mut HarnessLabReportBuilder,
) {
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

    let mut case = report.case("command_outside_project_does_not_discover_claude_local_root");
    case.claim_pass("install.targets.count_without_project", 2, targets.len());
    case.claim_pass(
        "install.targets.claude_local.absent_outside_project",
        false,
        target_detected(targets, "claude-local"),
    );
    case.finish();
}

fn installs_skill_into_all_sandbox_roots(report: &mut HarnessLabReportBuilder) {
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

    let command_report = json_stdout(&output);
    assert_eq!(command_report["skill_name"], "example-skill");
    assert_eq!(command_report["dry_run"], false);
    assert_eq!(command_report["installs"].as_array().unwrap().len(), 3);

    for root in [lab.agents_root(), lab.codex_root(), lab.claude_root()] {
        assert!(root.join("example-skill/SKILL.md").is_file());
        assert!(root.join("example-skill/skill.spec.yml").is_file());
    }
    lab.assert_no_real_home_writes();

    let mut case = report.case("installs_skill_into_all_sandbox_roots");
    case.claim_pass(
        "install.skill.name",
        "example-skill",
        command_report["skill_name"].clone(),
    );
    case.claim_pass(
        "install.skill.dry_run",
        false,
        command_report["dry_run"].clone(),
    );
    case.claim_pass(
        "install.skill.target_count",
        3,
        command_report["installs"].as_array().unwrap().len(),
    );
    for (id, root) in [
        ("agents", lab.agents_root()),
        ("codex", lab.codex_root()),
        ("claude-local", lab.claude_root()),
    ] {
        case.claim_pass(
            format!("install.skill.{id}.skill_md"),
            true,
            root.join("example-skill/SKILL.md").is_file(),
        );
        case.claim_pass(
            format!("install.skill.{id}.spec_yml"),
            true,
            root.join("example-skill/skill.spec.yml").is_file(),
        );
    }
    case.finish();
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

fn target_detected(targets: &[serde_json::Value], id: &str) -> bool {
    targets
        .iter()
        .find(|target| target["id"] == id)
        .is_some_and(|target| target["detected"] == true)
}

fn target_path(targets: &[serde_json::Value], id: &str) -> std::path::PathBuf {
    let target = targets
        .iter()
        .find(|target| target["id"] == id)
        .unwrap_or_else(|| panic!("missing target {id} in {targets:#?}"));
    std::path::PathBuf::from(target["path"].as_str().unwrap())
}
