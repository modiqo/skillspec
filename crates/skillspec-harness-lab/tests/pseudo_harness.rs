mod pseudo_harness_matrix;

use skillspec_harness_lab::{
    baseline_path, compare_or_update_baseline, compare_reports, CaseStatus, HarnessLabReport,
    HarnessLabReportBuilder,
};

const PSEUDO_HARNESS_BASELINE: &str = include_str!("../baselines/17-pseudo-harness-simulator.json");
const PHASE: &str = "17-pseudo-harness-simulator";

#[test]
fn pseudo_harness_phase_matches_baseline() {
    let report = run_pseudo_harness_phase();
    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, PHASE);
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 6);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 40);
    assert_eq!(report.summary.claims_failed, 0);

    assert_matches_or_updates_baseline(&report);
}

#[test]
fn pseudo_harness_report_comparison_detects_event_regression() {
    let baseline = run_pseudo_harness_phase();
    let mut candidate = baseline.clone();
    candidate.cases[0].claims.pop();

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "claim_missing"));
}

fn run_pseudo_harness_phase() -> HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new(PHASE);
    pseudo_harness_matrix::router::router_bypass_does_not_load_domain_skill(&mut report);
    pseudo_harness_matrix::router::router_selected_domain_loads_one_skill(&mut report);
    pseudo_harness_matrix::router::router_guard_repairs_before_catalog_build(&mut report);
    pseudo_harness_matrix::router::duplicate_root_candidates_collapse_to_one_logical_selection(
        &mut report,
    );
    pseudo_harness_matrix::durable::durable_observer_remains_implicit_with_router(&mut report);
    pseudo_harness_matrix::imported::imported_trampoline_handoff_is_visible(&mut report);
    report.build()
}

fn assert_matches_or_updates_baseline(report: &HarnessLabReport) {
    let baseline_path = baseline_path(PHASE);
    let comparison = compare_or_update_baseline(PHASE, PSEUDO_HARNESS_BASELINE, report);
    assert_eq!(
        comparison.status,
        CaseStatus::Pass,
        "harness lab pseudo-harness report regressed against {}\n{}",
        baseline_path.display(),
        serde_json::to_string_pretty(&comparison).unwrap()
    );
}
