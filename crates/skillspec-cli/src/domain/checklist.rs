use crate::{checklist, error};
use std::path::Path;

pub use checklist::{ChecklistKind, ChecklistReport, ChecklistStage, ChecklistStatus};

pub fn doctor(target: &str, stage: ChecklistStage) -> error::Result<ChecklistReport> {
    checklist::doctor_checklist(target, stage)
}

pub fn import(
    target: &str,
    build_root: Option<&Path>,
    stage: ChecklistStage,
) -> error::Result<ChecklistReport> {
    checklist::import_checklist(target, build_root, stage)
}

pub fn run(target: &Path, stage: ChecklistStage) -> error::Result<ChecklistReport> {
    checklist::run_checklist(target, stage)
}

pub fn render(report: &ChecklistReport) -> String {
    checklist::render(report)
}
