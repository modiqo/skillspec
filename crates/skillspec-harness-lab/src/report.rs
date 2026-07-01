use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const REPORT_SCHEMA: &str = "skillspec/harness-lab-report/v0";
pub const UPDATE_BASELINES_ENV: &str = "UPDATE_HARNESS_LAB_BASELINES";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HarnessLabReport {
    pub schema: String,
    pub phase: String,
    pub summary: ReportSummary,
    pub cases: Vec<ReportCase>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportSummary {
    pub status: CaseStatus,
    pub cases_total: usize,
    pub cases_passed: usize,
    pub cases_failed: usize,
    pub claims_total: usize,
    pub claims_passed: usize,
    pub claims_failed: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportCase {
    pub id: String,
    pub status: CaseStatus,
    pub claims: Vec<ReportClaim>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportClaim {
    pub id: String,
    pub status: CaseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed: Option<Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Pass,
    Fail,
}

#[derive(Clone, Debug, Default)]
pub struct HarnessLabReportBuilder {
    phase: String,
    cases: Vec<ReportCase>,
}

impl HarnessLabReportBuilder {
    pub fn new(phase: impl Into<String>) -> Self {
        Self {
            phase: phase.into(),
            cases: Vec::new(),
        }
    }

    pub fn case(&mut self, id: impl Into<String>) -> ReportCaseBuilder<'_> {
        ReportCaseBuilder {
            report: self,
            id: id.into(),
            claims: Vec::new(),
        }
    }

    pub fn build(self) -> HarnessLabReport {
        let cases_total = self.cases.len();
        let cases_passed = self
            .cases
            .iter()
            .filter(|case| case.status == CaseStatus::Pass)
            .count();
        let claims_total = self.cases.iter().map(|case| case.claims.len()).sum();
        let claims_passed = self
            .cases
            .iter()
            .flat_map(|case| &case.claims)
            .filter(|claim| claim.status == CaseStatus::Pass)
            .count();
        let cases_failed = cases_total - cases_passed;
        let claims_failed = claims_total - claims_passed;
        let status = if cases_failed == 0 && claims_failed == 0 {
            CaseStatus::Pass
        } else {
            CaseStatus::Fail
        };

        HarnessLabReport {
            schema: REPORT_SCHEMA.to_owned(),
            phase: self.phase,
            summary: ReportSummary {
                status,
                cases_total,
                cases_passed,
                cases_failed,
                claims_total,
                claims_passed,
                claims_failed,
            },
            cases: self.cases,
        }
    }
}

pub struct ReportCaseBuilder<'a> {
    report: &'a mut HarnessLabReportBuilder,
    id: String,
    claims: Vec<ReportClaim>,
}

impl ReportCaseBuilder<'_> {
    pub fn claim_pass(
        &mut self,
        id: impl Into<String>,
        expected: impl Serialize,
        observed: impl Serialize,
    ) -> &mut Self {
        self.claims.push(ReportClaim {
            id: id.into(),
            status: CaseStatus::Pass,
            expected: Some(to_value(expected)),
            observed: Some(to_value(observed)),
        });
        self
    }

    pub fn claim_fail(
        &mut self,
        id: impl Into<String>,
        expected: impl Serialize,
        observed: impl Serialize,
    ) -> &mut Self {
        self.claims.push(ReportClaim {
            id: id.into(),
            status: CaseStatus::Fail,
            expected: Some(to_value(expected)),
            observed: Some(to_value(observed)),
        });
        self
    }

    pub fn finish(self) {
        let status = if self
            .claims
            .iter()
            .all(|claim| claim.status == CaseStatus::Pass)
        {
            CaseStatus::Pass
        } else {
            CaseStatus::Fail
        };
        self.report.cases.push(ReportCase {
            id: self.id,
            status,
            claims: self.claims,
        });
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReportComparison {
    pub status: CaseStatus,
    pub regressions: Vec<ReportRegression>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReportRegression {
    pub kind: String,
    pub case_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed: Option<Value>,
}

pub fn compare_reports(
    baseline: &HarnessLabReport,
    candidate: &HarnessLabReport,
) -> ReportComparison {
    let mut regressions = Vec::new();
    if baseline.schema != candidate.schema {
        regressions.push(ReportRegression {
            kind: "schema_changed".to_owned(),
            case_id: String::new(),
            claim_id: None,
            expected: Some(Value::String(baseline.schema.clone())),
            observed: Some(Value::String(candidate.schema.clone())),
        });
    }
    if baseline.phase != candidate.phase {
        regressions.push(ReportRegression {
            kind: "phase_changed".to_owned(),
            case_id: String::new(),
            claim_id: None,
            expected: Some(Value::String(baseline.phase.clone())),
            observed: Some(Value::String(candidate.phase.clone())),
        });
    }

    let baseline_cases = cases_by_id(baseline);
    let candidate_cases = cases_by_id(candidate);
    for case_id in baseline_cases.keys() {
        let Some(candidate_case) = candidate_cases.get(case_id) else {
            regressions.push(ReportRegression {
                kind: "case_missing".to_owned(),
                case_id: case_id.clone(),
                claim_id: None,
                expected: None,
                observed: None,
            });
            continue;
        };
        let baseline_case = baseline_cases[case_id];
        if baseline_case.status == CaseStatus::Pass && candidate_case.status != CaseStatus::Pass {
            regressions.push(ReportRegression {
                kind: "case_status_regressed".to_owned(),
                case_id: case_id.clone(),
                claim_id: None,
                expected: Some(to_value(baseline_case.status)),
                observed: Some(to_value(candidate_case.status)),
            });
        }
        compare_claims(case_id, baseline_case, candidate_case, &mut regressions);
    }

    ReportComparison {
        status: if regressions.is_empty() {
            CaseStatus::Pass
        } else {
            CaseStatus::Fail
        },
        regressions,
    }
}

pub fn baseline_path(phase: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("baselines")
        .join(format!("{phase}.json"))
}

pub fn compare_or_update_baseline(
    phase: &str,
    embedded_baseline: &str,
    report: &HarnessLabReport,
) -> ReportComparison {
    let path = baseline_path(phase);
    let baseline_text = if std::env::var(UPDATE_BASELINES_ENV).as_deref() == Ok("1") {
        write_report(&path, report);
        std::fs::read_to_string(&path).unwrap()
    } else {
        embedded_baseline.to_owned()
    };
    let baseline = serde_json::from_str(&baseline_text).unwrap();
    compare_reports(&baseline, report)
}

pub(crate) fn write_report(path: &Path, report: &HarnessLabReport) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let json = serde_json::to_string_pretty(report).unwrap();
    std::fs::write(path, format!("{json}\n")).unwrap();
}

fn compare_claims(
    case_id: &str,
    baseline_case: &ReportCase,
    candidate_case: &ReportCase,
    regressions: &mut Vec<ReportRegression>,
) {
    let baseline_claims = claims_by_id(baseline_case);
    let candidate_claims = claims_by_id(candidate_case);
    for claim_id in baseline_claims.keys() {
        let Some(candidate_claim) = candidate_claims.get(claim_id) else {
            regressions.push(ReportRegression {
                kind: "claim_missing".to_owned(),
                case_id: case_id.to_owned(),
                claim_id: Some(claim_id.clone()),
                expected: None,
                observed: None,
            });
            continue;
        };
        let baseline_claim = baseline_claims[claim_id];
        if baseline_claim.status == CaseStatus::Pass && candidate_claim.status != CaseStatus::Pass {
            regressions.push(ReportRegression {
                kind: "claim_status_regressed".to_owned(),
                case_id: case_id.to_owned(),
                claim_id: Some(claim_id.clone()),
                expected: Some(to_value(baseline_claim.status)),
                observed: Some(to_value(candidate_claim.status)),
            });
        }
        if baseline_claim.observed != candidate_claim.observed {
            regressions.push(ReportRegression {
                kind: "claim_observed_changed".to_owned(),
                case_id: case_id.to_owned(),
                claim_id: Some(claim_id.clone()),
                expected: baseline_claim.observed.clone(),
                observed: candidate_claim.observed.clone(),
            });
        }
    }
}

fn cases_by_id(report: &HarnessLabReport) -> BTreeMap<String, &ReportCase> {
    assert_unique(report.cases.iter().map(|case| case.id.as_str()), "case");
    report
        .cases
        .iter()
        .map(|case| (case.id.clone(), case))
        .collect()
}

fn claims_by_id(case: &ReportCase) -> BTreeMap<String, &ReportClaim> {
    assert_unique(case.claims.iter().map(|claim| claim.id.as_str()), "claim");
    case.claims
        .iter()
        .map(|claim| (claim.id.clone(), claim))
        .collect()
}

fn assert_unique<'a>(ids: impl Iterator<Item = &'a str>, kind: &str) {
    let mut seen = BTreeSet::new();
    for id in ids {
        assert!(seen.insert(id), "duplicate {kind} id in report: {id}");
    }
}

fn to_value(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}
