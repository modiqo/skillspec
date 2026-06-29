mod imported_runtime_matrix;

use skillspec_harness_lab::{
    baseline_path, compare_or_update_baseline, compare_reports, CaseStatus, HarnessLabReport,
    HarnessLabReportBuilder,
};

const IMPORTED_RUNTIME_BASELINE: &str = include_str!("../baselines/12-imported-skill-runtime.json");
const PHASE: &str = "12-imported-skill-runtime";

#[test]
fn imported_runtime_phase_matches_baseline() {
    let report = run_imported_runtime_phase();
    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, PHASE);
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 4);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 30);
    assert_eq!(report.summary.claims_failed, 0);

    assert_matches_or_updates_baseline(&report);
}

#[test]
fn imported_runtime_report_comparison_detects_observed_regression() {
    let baseline = run_imported_runtime_phase();
    let mut candidate = baseline.clone();
    candidate.cases[0].claims[0].observed = Some(serde_json::json!("changed"));

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "claim_observed_changed"));
}

fn run_imported_runtime_phase() -> HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new(PHASE);
    imported_runtime_matrix::compile_install::compiled_loader_installs_into_all_detected_roots(
        &mut report,
    );
    imported_runtime_matrix::compile_install::retire_existing_replaces_prose_skill(&mut report);
    imported_runtime_matrix::proof::decision_trace_without_execution_is_unproven(&mut report);
    imported_runtime_matrix::proof::progress_batch_and_alignment_prove_reviewed_import(&mut report);
    report.build()
}

fn assert_matches_or_updates_baseline(report: &HarnessLabReport) {
    let baseline_path = baseline_path(PHASE);
    let comparison = compare_or_update_baseline(PHASE, IMPORTED_RUNTIME_BASELINE, report);
    assert_eq!(
        comparison.status,
        CaseStatus::Pass,
        "harness lab imported runtime report regressed against {}\n{}",
        baseline_path.display(),
        serde_json::to_string_pretty(&comparison).unwrap()
    );
}
