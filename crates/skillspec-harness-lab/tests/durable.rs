mod durable_matrix;

use skillspec_harness_lab::{
    baseline_path, compare_or_update_baseline, compare_reports, CaseStatus, HarnessLabReport,
    HarnessLabReportBuilder,
};

const DURABLE_BASELINE: &str = include_str!("../baselines/14-durable-harness-lab.json");
const PHASE: &str = "14-durable-harness-lab";

#[test]
fn durable_phase_matches_baseline() {
    let report = run_durable_phase();
    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, PHASE);
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 5);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 40);
    assert_eq!(report.summary.claims_failed, 0);

    assert_matches_or_updates_baseline(&report);
}

#[test]
fn durable_report_comparison_detects_observed_regression() {
    let baseline = run_durable_phase();
    let mut candidate = baseline.clone();
    candidate.cases[0].claims[0].observed = Some(serde_json::json!("changed"));

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "claim_observed_changed"));
}

fn run_durable_phase() -> HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new(PHASE);
    durable_matrix::lifecycle::durable_install_update_delete_managed_dirs(&mut report);
    durable_matrix::lifecycle::durable_refuses_update_and_delete_without_marker(&mut report);
    durable_matrix::mode::durable_disable_enable_toggles_implicit_invocation(&mut report);
    durable_matrix::preflight::durable_install_and_enable_require_rote(&mut report);
    durable_matrix::router::durable_install_refreshes_router_and_remains_implicit(&mut report);
    report.build()
}

fn assert_matches_or_updates_baseline(report: &HarnessLabReport) {
    let baseline_path = baseline_path(PHASE);
    let comparison = compare_or_update_baseline(PHASE, DURABLE_BASELINE, report);
    assert_eq!(
        comparison.status,
        CaseStatus::Pass,
        "harness lab durable report regressed against {}\n{}",
        baseline_path.display(),
        serde_json::to_string_pretty(&comparison).unwrap()
    );
}
