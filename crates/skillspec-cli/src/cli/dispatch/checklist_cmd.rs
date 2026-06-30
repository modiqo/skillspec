use crate::cli::args::{ChecklistStageArg, DoctorCommand, ImportCommand, RunCommand};
use skillspec::{domain::checklist, error::Result, report};

pub(super) fn doctor(command: DoctorCommand) -> Result<()> {
    match command {
        DoctorCommand::Checklist {
            source,
            stage,
            json,
        } => emit(checklist::doctor(&source, stage.into())?, json),
    }
}

pub(super) fn import(command: ImportCommand) -> Result<()> {
    match command {
        ImportCommand::Checklist {
            target,
            build_root,
            stage,
            json,
        } => emit(
            checklist::import(&target, build_root.as_deref(), stage.into())?,
            json,
        ),
    }
}

pub(super) fn run(command: RunCommand) -> Result<()> {
    match command {
        RunCommand::Checklist {
            target,
            stage,
            json,
        } => emit(checklist::run(&target, stage.into())?, json),
    }
}

fn emit(checklist_report: checklist::ChecklistReport, json: bool) -> Result<()> {
    let blocked = matches!(checklist_report.status, checklist::ChecklistStatus::Blocked);
    if json {
        report::json(&checklist_report)?;
    } else {
        report::text(&checklist::render(&checklist_report))?;
    }
    if blocked {
        std::process::exit(1);
    }
    Ok(())
}

impl From<ChecklistStageArg> for checklist::ChecklistStage {
    fn from(value: ChecklistStageArg) -> Self {
        match value {
            ChecklistStageArg::Entry => Self::Entry,
            ChecklistStageArg::Loop => Self::Loop,
            ChecklistStageArg::Exit => Self::Exit,
        }
    }
}
