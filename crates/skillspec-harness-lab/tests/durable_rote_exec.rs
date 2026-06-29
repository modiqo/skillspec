mod durable_rote_exec_matrix;

use skillspec_harness_lab::{
    baseline_path, compare_or_update_baseline, compare_reports, CaseStatus, HarnessLabReport,
    HarnessLabReportBuilder,
};

const CONTRACT_BASELINE: &str = include_str!("../baselines/15-durable-rote-exec-proof.json");
const LIVE_BASELINE: &str = include_str!("../baselines/15-durable-rote-exec-live.json");
const CONTRACT_PHASE: &str = "15-durable-rote-exec-proof";
const LIVE_PHASE: &str = "15-durable-rote-exec-live";

#[test]
fn durable_rote_exec_contract_phase_matches_baseline() {
    let report = run_contract_phase();
    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, CONTRACT_PHASE);
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 2);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 15);
    assert_eq!(report.summary.claims_failed, 0);

    assert_matches_or_updates_baseline(CONTRACT_PHASE, CONTRACT_BASELINE, &report);
}

#[test]
fn durable_rote_exec_contract_report_detects_observed_regression() {
    let baseline = run_contract_phase();
    let mut candidate = baseline.clone();
    candidate.cases[0].claims[0].observed = Some(serde_json::json!("changed"));

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "claim_observed_changed"));
}

#[test]
#[ignore = "requires authenticated local rote; run `just harness-lab-live-durable-rote-exec`"]
fn live_copied_local_rote_exec_phase_matches_baseline() {
    let report = run_live_phase();
    assert_eq!(report.schema, "skillspec/harness-lab-report/v0");
    assert_eq!(report.phase, LIVE_PHASE);
    assert_eq!(report.summary.status, CaseStatus::Pass);
    assert_eq!(report.summary.cases_total, 1);
    assert_eq!(report.summary.cases_failed, 0);
    assert!(report.summary.claims_total >= 12);
    assert_eq!(report.summary.claims_failed, 0);

    assert_matches_or_updates_baseline(LIVE_PHASE, LIVE_BASELINE, &report);
}

#[test]
#[ignore = "requires authenticated local rote; run `just harness-lab-live-durable-rote-exec`"]
fn live_copied_local_rote_exec_report_detects_observed_regression() {
    let baseline = run_live_phase();
    let mut candidate = baseline.clone();
    candidate.cases[0].claims[0].observed = Some(serde_json::json!("changed"));

    let comparison = compare_reports(&baseline, &candidate);
    assert_eq!(comparison.status, CaseStatus::Fail);
    assert!(comparison
        .regressions
        .iter()
        .any(|regression| regression.kind == "claim_observed_changed"));
}

fn run_contract_phase() -> HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new(CONTRACT_PHASE);
    durable_rote_exec_matrix::contract::durable_executor_selects_rote_exec_contract(&mut report);
    durable_rote_exec_matrix::contract::alignment_accepts_rote_exec_process_evidence(&mut report);
    report.build()
}

fn run_live_phase() -> HarnessLabReport {
    let mut report = HarnessLabReportBuilder::new(LIVE_PHASE);
    durable_rote_exec_matrix::live::copied_local_rote_runs_one_shot_process(&mut report);
    report.build()
}

fn assert_matches_or_updates_baseline(
    phase: &str,
    embedded_baseline: &str,
    report: &HarnessLabReport,
) {
    let baseline_path = baseline_path(phase);
    let comparison = compare_or_update_baseline(phase, embedded_baseline, report);
    assert_eq!(
        comparison.status,
        CaseStatus::Pass,
        "harness lab durable rote-exec report regressed against {}\n{}",
        baseline_path.display(),
        serde_json::to_string_pretty(&comparison).unwrap()
    );
}
