use serde_json::Value;
use skillspec_harness_lab::{
    assert_failure, assert_success, baseline_path, compare_or_update_baseline, compare_reports,
    json_stdout, stderr, CaseStatus, HarnessLab, HarnessLabReport, HarnessLabReportBuilder,
};
use std::path::Path;
use std::process::Output;

const DOCTOR_BASELINE: &str = include_str!("../baselines/10-doctor-matrix.json");
const PHASE: &str = "10-doctor-matrix";

#[test]
fn doctor_phase_matches_baseline() {
    let report = run_doctor_phase();
    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, PHASE);
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 9);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 30);
    assert_eq!(report.summary.claims_failed, 0);

    assert_matches_or_updates_baseline(&report);
}

#[test]
fn doctor_report_comparison_detects_observed_regression() {
    let baseline = run_doctor_phase();
    let mut candidate = baseline.clone();
    candidate.cases[0].claims[0].observed = Some(serde_json::json!("changed"));

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "claim_observed_changed"));
}

fn run_doctor_phase() -> HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new(PHASE);
    doctor_reports_non_skill_file_path_as_shape_only(&mut report);
    doctor_rejects_nonexistent_path(&mut report);
    doctor_reports_empty_skill_without_panic(&mut report);
    doctor_reports_simple_skill_folder(&mut report);
    doctor_reports_direct_skill_md_path(&mut report);
    doctor_reports_malformed_frontmatter(&mut report);
    doctor_reports_entry_skill_with_cross_referenced_subskills(&mut report);
    doctor_reports_plugin_workspace(&mut report);
    doctor_reports_non_skill_repository_shape(&mut report);
    report.build()
}

fn assert_matches_or_updates_baseline(report: &HarnessLabReport) {
    let baseline_path = baseline_path(PHASE);
    let comparison = compare_or_update_baseline(PHASE, DOCTOR_BASELINE, report);
    assert_eq!(
        comparison.status,
        CaseStatus::Pass,
        "harness lab doctor report regressed against {}\n{}",
        baseline_path.display(),
        serde_json::to_string_pretty(&comparison).unwrap()
    );
}

fn doctor_reports_non_skill_file_path_as_shape_only(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("doctor-non-skill-file");
    let target = lab.root().join("NOT_SKILL.txt");
    lab.write_file(&target, "This is not a SkillSpec skill package.\n");

    let output = doctor_output(&lab, &target);
    assert_success(&output);
    let doctor = json_stdout(&output);
    assert_eq!(doctor["analysis_status"], "shape_only");
    assert_eq!(doctor["shape"]["kind"], "non_skill_repository");
    let issues = json_string(&doctor["issues"]);
    assert!(issues.contains("no_skill_entrypoint"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_reports_non_skill_file_path_as_shape_only");
    case.claim_pass("doctor.exit_success", true, output.status.success());
    case.claim_pass(
        "doctor.analysis_status",
        "shape_only",
        &doctor["analysis_status"],
    );
    case.claim_pass(
        "doctor.shape.kind",
        "non_skill_repository",
        &doctor["shape"]["kind"],
    );
    case.claim_pass(
        "doctor.issues.no_skill_entrypoint",
        true,
        issues.contains("no_skill_entrypoint"),
    );
    case.finish();
}

fn doctor_rejects_nonexistent_path(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("doctor-missing-path");
    let target = lab.root().join("missing-skill");

    let output = doctor_output(&lab, &target);
    assert_failure(&output);
    let stderr = stderr(&output);
    assert!(stderr.contains("does not exist locally"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_rejects_nonexistent_path");
    case.claim_pass("doctor.exit_success", false, output.status.success());
    case.claim_pass(
        "doctor.stderr.missing_path",
        true,
        stderr.contains("does not exist locally"),
    );
    case.finish();
}

fn doctor_reports_empty_skill_without_panic(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("doctor-empty-skill");
    let root = lab.root().join("empty-skill");
    lab.write_file(&root.join("SKILL.md"), "");

    let output = doctor_output(&lab, &root);
    assert_success(&output);
    let doctor = json_stdout(&output);
    assert_eq!(doctor["analysis_status"], "full");
    assert_eq!(doctor["shape"]["kind"], "simple_skill");
    let issues = json_string(&doctor["issues"]);
    assert!(issues.contains("missing_or_malformed_frontmatter"));
    assert!(issues.contains("missing_behavior_contract"));
    assert!(issues.contains("missing_trace_proof_surface"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_reports_empty_skill_without_panic");
    case.claim_pass("doctor.exit_success", true, output.status.success());
    case.claim_pass("doctor.analysis_status", "full", &doctor["analysis_status"]);
    case.claim_pass(
        "doctor.shape.kind",
        "simple_skill",
        &doctor["shape"]["kind"],
    );
    case.claim_pass(
        "doctor.issues.missing_or_malformed_frontmatter",
        true,
        issues.contains("missing_or_malformed_frontmatter"),
    );
    case.claim_pass(
        "doctor.issues.missing_behavior_contract",
        true,
        issues.contains("missing_behavior_contract"),
    );
    case.claim_pass(
        "doctor.issues.missing_trace_proof_surface",
        true,
        issues.contains("missing_trace_proof_surface"),
    );
    case.finish();
}

fn doctor_reports_simple_skill_folder(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("doctor-simple-folder");
    let root = lab.root().join("fixtures");
    let skill_dir = lab.write_skill(&root, "review-skill", &review_skill_md(), None);

    let output = doctor_output(&lab, &skill_dir);
    assert_success(&output);
    let doctor = json_stdout(&output);
    assert_eq!(doctor["analysis_status"], "full");
    assert_eq!(doctor["shape"]["kind"], "simple_skill");
    assert_eq!(
        doctor["score_model"]["primary_score_label"],
        "agent_follow_through_risk"
    );
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_reports_simple_skill_folder");
    case.claim_pass("doctor.exit_success", true, output.status.success());
    case.claim_pass("doctor.analysis_status", "full", &doctor["analysis_status"]);
    case.claim_pass(
        "doctor.shape.kind",
        "simple_skill",
        &doctor["shape"]["kind"],
    );
    case.claim_pass(
        "doctor.score_model.primary_score_label",
        "agent_follow_through_risk",
        &doctor["score_model"]["primary_score_label"],
    );
    case.claim_pass(
        "doctor.frontmatter.name",
        "review-skill",
        &doctor["frontmatter_discovery_risk"]["fields"]["name"],
    );
    case.finish();
}

fn doctor_reports_direct_skill_md_path(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("doctor-direct-skill-md");
    let root = lab.root().join("fixtures");
    let skill_dir = lab.write_skill(&root, "direct-skill", &review_skill_md(), None);
    let target = skill_dir.join("SKILL.md");

    let output = doctor_output(&lab, &target);
    assert_success(&output);
    let doctor = json_stdout(&output);
    assert_eq!(doctor["analysis_status"], "full");
    assert_eq!(doctor["shape"]["kind"], "simple_skill");
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_reports_direct_skill_md_path");
    case.claim_pass("doctor.exit_success", true, output.status.success());
    case.claim_pass("doctor.analysis_status", "full", &doctor["analysis_status"]);
    case.claim_pass(
        "doctor.shape.kind",
        "simple_skill",
        &doctor["shape"]["kind"],
    );
    case.claim_pass(
        "doctor.target.normalized",
        "<LAB_ROOT>/fixtures/direct-skill/SKILL.md",
        lab.normalize_path(&target),
    );
    case.finish();
}

fn doctor_reports_malformed_frontmatter(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("doctor-malformed-frontmatter");
    let root = lab.root().join("bad-frontmatter");
    lab.write_file(
        &root.join("SKILL.md"),
        "---\nname: bad\ndescription: Bad: unquoted colon\n---\n# Bad\n",
    );

    let output = doctor_output(&lab, &root);
    assert_success(&output);
    let doctor = json_stdout(&output);
    assert_eq!(
        doctor["frontmatter_discovery_risk"]["fields"]["parse_status"],
        "invalid_yaml"
    );
    let issues = json_string(&doctor["issues"]);
    assert!(issues.contains("missing_or_malformed_frontmatter"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_reports_malformed_frontmatter");
    case.claim_pass("doctor.exit_success", true, output.status.success());
    case.claim_pass(
        "doctor.frontmatter.parse_status",
        "invalid_yaml",
        &doctor["frontmatter_discovery_risk"]["fields"]["parse_status"],
    );
    case.claim_pass(
        "doctor.issues.missing_or_malformed_frontmatter",
        true,
        issues.contains("missing_or_malformed_frontmatter"),
    );
    case.finish();
}

fn doctor_reports_entry_skill_with_cross_referenced_subskills(
    report: &mut HarnessLabReportBuilder,
) {
    let lab = HarnessLab::new("doctor-entry-subskills");
    let root = lab.root().join("skills-repo");
    lab.write_file(
        &root.join("SKILL.md"),
        r#"---
name: parent
description: Parent skill.
---
# Parent

Use `./legal-review/SKILL.md` and `/contract-review` when those workflows apply.
"#,
    );
    lab.write_file(
        &root.join("legal-review").join("SKILL.md"),
        "---\nname: legal-review\ndescription: Legal review.\n---\n# Legal\n",
    );
    lab.write_file(
        &root.join("contract-review").join("SKILL.md"),
        "---\nname: contract-review\ndescription: Contract review.\n---\n# Contract\n",
    );

    let output = doctor_output(&lab, &root);
    assert_success(&output);
    let doctor = json_stdout(&output);
    assert_eq!(doctor["analysis_status"], "workspace");
    assert_eq!(doctor["shape"]["kind"], "entry_skill_with_subskills");
    assert_eq!(doctor["packages"].as_array().unwrap().len(), 3);
    let referenced = doctor["shape"]["referenced_skill_paths"]
        .as_array()
        .unwrap();
    assert!(referenced
        .iter()
        .any(|path| path.as_str() == Some("legal-review")));
    assert!(referenced
        .iter()
        .any(|path| path.as_str() == Some("contract-review")));
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_reports_entry_skill_with_cross_referenced_subskills");
    case.claim_pass("doctor.exit_success", true, output.status.success());
    case.claim_pass(
        "doctor.analysis_status",
        "workspace",
        &doctor["analysis_status"],
    );
    case.claim_pass(
        "doctor.shape.kind",
        "entry_skill_with_subskills",
        &doctor["shape"]["kind"],
    );
    case.claim_pass(
        "doctor.package_count",
        3,
        doctor["packages"].as_array().unwrap().len(),
    );
    case.claim_pass(
        "doctor.referenced.legal_review",
        true,
        referenced
            .iter()
            .any(|path| path.as_str() == Some("legal-review")),
    );
    case.claim_pass(
        "doctor.referenced.contract_review",
        true,
        referenced
            .iter()
            .any(|path| path.as_str() == Some("contract-review")),
    );
    case.finish();
}

fn doctor_reports_plugin_workspace(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("doctor-plugin-workspace");
    let root = lab.root().join("claude-for-legal");
    lab.write_file(
        &root
            .join("commercial")
            .join(".claude-plugin")
            .join("plugin.json"),
        r#"{"name":"commercial-legal","version":"1.0.0"}"#,
    );
    lab.write_file(
        &root
            .join("commercial")
            .join("skills")
            .join("review")
            .join("SKILL.md"),
        "---\nname: review\ndescription: Review.\n---\n# Review\n",
    );

    let output = doctor_output(&lab, &root);
    assert_success(&output);
    let doctor = json_stdout(&output);
    assert_eq!(doctor["analysis_status"], "workspace");
    assert_eq!(doctor["shape"]["kind"], "plugin_workspace");
    assert_eq!(
        doctor["shape"]["plugin_roots"][0]["namespace"],
        "commercial-legal"
    );
    assert_eq!(doctor["packages"][0]["shape_role"], "plugin_skill");
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_reports_plugin_workspace");
    case.claim_pass("doctor.exit_success", true, output.status.success());
    case.claim_pass(
        "doctor.analysis_status",
        "workspace",
        &doctor["analysis_status"],
    );
    case.claim_pass(
        "doctor.shape.kind",
        "plugin_workspace",
        &doctor["shape"]["kind"],
    );
    case.claim_pass(
        "doctor.plugin.namespace",
        "commercial-legal",
        &doctor["shape"]["plugin_roots"][0]["namespace"],
    );
    case.claim_pass(
        "doctor.package.shape_role",
        "plugin_skill",
        &doctor["packages"][0]["shape_role"],
    );
    case.finish();
}

fn doctor_reports_non_skill_repository_shape(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("doctor-non-skill-repo");
    let root = lab.root().join("code-repo");
    lab.write_file(
        &root.join("Cargo.toml"),
        "[package]\nname = \"not-a-skill\"\n",
    );
    lab.write_file(&root.join("src").join("main.rs"), "fn main() {}\n");

    let output = doctor_output(&lab, &root);
    assert_success(&output);
    let doctor = json_stdout(&output);
    assert_eq!(doctor["analysis_status"], "shape_only");
    assert_eq!(doctor["shape"]["kind"], "non_skill_repository");
    let issues = json_string(&doctor["issues"]);
    assert!(issues.contains("no_skill_entrypoint"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("doctor_reports_non_skill_repository_shape");
    case.claim_pass("doctor.exit_success", true, output.status.success());
    case.claim_pass(
        "doctor.analysis_status",
        "shape_only",
        &doctor["analysis_status"],
    );
    case.claim_pass(
        "doctor.shape.kind",
        "non_skill_repository",
        &doctor["shape"]["kind"],
    );
    case.claim_pass(
        "doctor.counts.code_files",
        1,
        &doctor["counts"]["code_files"],
    );
    case.claim_pass(
        "doctor.issues.no_skill_entrypoint",
        true,
        issues.contains("no_skill_entrypoint"),
    );
    case.finish();
}

fn doctor_output(lab: &HarnessLab, target: &Path) -> Output {
    lab.command()
        .arg("doctor")
        .arg(target)
        .arg("--json")
        .output()
        .unwrap()
}

fn review_skill_md() -> String {
    r#"---
name: review-skill
description: Review one file and report risks.
---
# Review Skill

1. Read the requested file.
2. Report risks.
"#
    .to_owned()
}

fn json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}
