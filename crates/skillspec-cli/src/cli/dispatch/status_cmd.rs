use skillspec::{domain::harness, error::Result, report};
use std::path::PathBuf;

pub(super) fn run(roots: Vec<PathBuf>, json: bool) -> Result<()> {
    let status_report = harness::status_report(harness::StatusOptions { roots })?;
    if json {
        report::json(&status_report)
    } else {
        report::text(&harness::render_status(&status_report))
    }
}
