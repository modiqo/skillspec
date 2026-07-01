mod import_matrix;

use skillspec_harness_lab::{
    baseline_path, compare_or_update_baseline, compare_reports, CaseStatus, HarnessLabReport,
    HarnessLabReportBuilder,
};

const IMPORT_BASELINE: &str = include_str!("../baselines/11-import-matrix.json");
const PHASE: &str = "11-import-matrix";

#[test]
fn import_phase_matches_baseline() {
    let report = run_import_phase();
    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, PHASE);
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 10);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 45);
    assert_eq!(report.summary.claims_failed, 0);

    assert_matches_or_updates_baseline(&report);
}

#[test]
fn import_report_comparison_detects_missing_claim_regression() {
    let baseline = run_import_phase();
    let mut candidate = baseline.clone();
    candidate.cases[0].claims.pop();

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "claim_missing"));
}

fn run_import_phase() -> HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new(PHASE);
    import_matrix::single::import_skill_rejects_missing_path(&mut report);
    import_matrix::single::import_skill_imports_single_skill_folder(&mut report);
    import_matrix::single::import_skill_imports_direct_skill_md_file(&mut report);
    import_matrix::single::import_skill_imports_direct_markdown_file_as_file_source(&mut report);
    import_matrix::single::import_skill_scaffolds_empty_skill_for_review(&mut report);
    import_matrix::single::import_skill_scaffolds_malformed_frontmatter_for_review(&mut report);
    import_matrix::single::import_skill_rejects_parent_folder_with_multiple_skills(&mut report);
    import_matrix::single::import_skill_rejects_stale_source_map(&mut report);
    import_matrix::workspace::workspace_import_fans_out_multiple_skills(&mut report);
    import_matrix::workspace::workspace_import_preserves_plugin_namespace(&mut report);
    report.build()
}

fn assert_matches_or_updates_baseline(report: &HarnessLabReport) {
    let baseline_path = baseline_path(PHASE);
    let comparison = compare_or_update_baseline(PHASE, IMPORT_BASELINE, report);
    assert_eq!(
        comparison.status,
        CaseStatus::Pass,
        "harness lab import report regressed against {}\n{}",
        baseline_path.display(),
        serde_json::to_string_pretty(&comparison).unwrap()
    );
}
