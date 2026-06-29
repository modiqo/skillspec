//! Test-only controlled harness lab for SkillSpec.
//!
//! This crate intentionally stays outside the published CLI crate. It provides
//! sandbox homes, fake harness roots, command construction, and assertions for
//! integration tests that need to exercise harness-facing behavior without
//! touching a developer's real home directory.

mod assertions;
mod command;
mod fixtures;
mod lab;
mod paths;
mod report;
mod temp;

pub use assertions::{assert_failure, assert_success, json_stdout, stderr, stdout};
pub use fixtures::{basic_skill_md, basic_skill_spec};
pub use lab::HarnessLab;
pub use report::{
    compare_reports, CaseStatus, HarnessLabReport, HarnessLabReportBuilder, ReportCase,
    ReportClaim, ReportComparison, ReportRegression,
};
