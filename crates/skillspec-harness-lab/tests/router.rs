mod router_matrix;

use skillspec_harness_lab::{
    baseline_path, compare_or_update_baseline, compare_reports, CaseStatus, HarnessLabReport,
    HarnessLabReportBuilder,
};

const ROUTER_BASELINE: &str = include_str!("../baselines/13-router-harness-lab.json");
const PHASE: &str = "13-router-harness-lab";

#[test]
fn router_phase_matches_baseline() {
    let report = run_router_phase();
    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, PHASE);
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 5);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 55);
    assert_eq!(report.summary.claims_failed, 0);

    assert_matches_or_updates_baseline(&report);
}

#[test]
fn router_report_comparison_detects_missing_case_regression() {
    let baseline = run_router_phase();
    let mut candidate = baseline.clone();
    candidate.cases.pop();

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "case_missing"));
}

fn run_router_phase() -> HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new(PHASE);
    router_matrix::install::router_install_writes_hooks_visibility_and_index(&mut report);
    router_matrix::guard::router_guard_repairs_out_of_band_skill_and_hook_context(&mut report);
    router_matrix::route::router_routes_clear_intent_and_bypasses_ordinary_tasks(&mut report);
    router_matrix::mode::router_disable_enable_restores_and_reapplies_visibility(&mut report);
    router_matrix::uninstall::router_uninstall_removes_router_and_restores_visibility(&mut report);
    report.build()
}

fn assert_matches_or_updates_baseline(report: &HarnessLabReport) {
    let baseline_path = baseline_path(PHASE);
    let comparison = compare_or_update_baseline(PHASE, ROUTER_BASELINE, report);
    assert_eq!(
        comparison.status,
        CaseStatus::Pass,
        "harness lab router report regressed against {}\n{}",
        baseline_path.display(),
        serde_json::to_string_pretty(&comparison).unwrap()
    );
}
