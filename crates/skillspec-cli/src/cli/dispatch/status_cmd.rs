use skillspec::{error::Result, report, status};
use std::path::PathBuf;

pub(super) fn run(roots: Vec<PathBuf>, json: bool) -> Result<()> {
    let status_report = status::status(status::StatusOptions { roots })?;
    if json {
        report::json(&status_report)
    } else {
        report::text(&status::render(&status_report))
    }
}
