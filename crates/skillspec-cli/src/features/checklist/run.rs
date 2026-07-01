use super::*;

pub(super) fn run_spec_checklist(spec: &Path, stage: ChecklistStage) -> ChecklistReport {
    let spec_arg = shell_arg_path(spec);
    ChecklistReport {
        schema: CHECKLIST_SCHEMA,
        kind: ChecklistKind::Run,
        stage,
        status: ChecklistStatus::Ready,
        entity: ChecklistEntity {
            target: spec.display().to_string(),
            spec: Some(spec.display().to_string()),
            ..ChecklistEntity::default()
        },
        activation_policy: "selected_route_execution".to_owned(),
        position: ChecklistPosition::default(),
        steps: vec![ChecklistStep {
            id: "start_guided_run".to_owned(),
            description: "Start a guided SkillSpec run for a concrete task.".to_owned(),
            directive: "Use run-loop to select the route, persist guide state, and emit the current gate before executing task work.".to_owned(),
            commands: vec![format!(
                "skillspec run-loop {spec_arg} --input '<task>' --trace-dir .skillspec/traces --guide agent --json"
            )],
            repeat: ChecklistRepeat {
                until: Some("guide-state.json exists and current gate has concrete open requirements".to_owned()),
                ..ChecklistRepeat::default()
            },
            requires: vec!["task_input".to_owned(), "trace_dir".to_owned()],
            blocks: vec!["missing_task_input".to_owned()],
            forbid: vec!["execute_task_without_selected_route".to_owned()],
            evidence: vec!["<run-dir>/guide-state.json".to_owned()],
        }],
        forbid: vec!["claim_route_fulfilled_without_progress_evidence".to_owned()],
        next_command: Some(format!(
            "skillspec run-loop {spec_arg} --input '<task>' --trace-dir .skillspec/traces --guide agent --json"
        )),
        blockers: Vec::new(),
    }
}

pub(super) fn blocked_run_dir_checklist(run_dir: &Path, stage: ChecklistStage) -> ChecklistReport {
    ChecklistReport {
        schema: CHECKLIST_SCHEMA,
        kind: ChecklistKind::Run,
        stage,
        status: ChecklistStatus::Blocked,
        entity: ChecklistEntity {
            target: run_dir.display().to_string(),
            run_dir: Some(run_dir.display().to_string()),
            ..ChecklistEntity::default()
        },
        activation_policy: "selected_route_execution".to_owned(),
        position: ChecklistPosition::default(),
        steps: vec![ChecklistStep {
            id: "missing_guide_state".to_owned(),
            description: "The run directory does not contain guide-state.json.".to_owned(),
            directive: "Resume or restart through run-loop --guide agent so the checklist can read selected route, phase, requirements, forbids, and proof anchors.".to_owned(),
            commands: vec!["skillspec run-loop <skill.spec.yml> --resume <run-dir> --guide agent --json".to_owned()],
            repeat: ChecklistRepeat {
                until: Some("guide-state.json exists".to_owned()),
                ..ChecklistRepeat::default()
            },
            requires: vec!["guide-state.json".to_owned()],
            blocks: vec!["missing_guide_state".to_owned()],
            forbid: vec!["infer_route_from_memory".to_owned()],
            evidence: vec![run_dir.join("guide-state.json").display().to_string()],
        }],
        forbid: vec!["claim_route_fulfilled_without_progress_evidence".to_owned()],
        next_command: Some(
            "skillspec run-loop <skill.spec.yml> --resume <run-dir> --guide agent --json"
                .to_owned(),
        ),
        blockers: vec!["missing guide-state.json".to_owned()],
    }
}

pub(super) fn run_guide_checklist(
    run_dir: &Path,
    guide_report: &guide::GuideReport,
    stage: ChecklistStage,
) -> ChecklistReport {
    let current_phase = guide_report.current_gate.phase.clone();
    let open_requirements = guide_report.current_gate.open_requirements.clone();
    let status = if stage == ChecklistStage::Exit && !guide_report.path.remaining_phases.is_empty()
    {
        ChecklistStatus::Blocked
    } else if guide_report.path.remaining_phases.is_empty() && open_requirements.is_empty() {
        ChecklistStatus::Complete
    } else {
        ChecklistStatus::Ready
    };
    let commands = match stage {
        ChecklistStage::Entry => vec![guide_report.resume.command.clone()],
        ChecklistStage::Loop => {
            let mut commands = guide_report.current_gate.allowed_commands.clone();
            commands.extend(
                guide_report
                    .current_gate
                    .progress_to_record
                    .iter()
                    .map(|hint| hint.command.clone()),
            );
            commands
        }
        ChecklistStage::Exit => vec![
            guide_report.end.token_stats_command.clone(),
            guide_report.end.final_progress_command.clone(),
            guide_report.end.alignment_command.clone(),
        ],
    };
    let blockers = if status == ChecklistStatus::Blocked {
        vec![format!(
            "remaining phases before exit: {}",
            guide_report.path.remaining_phases.join(", ")
        )]
    } else {
        Vec::new()
    };
    ChecklistReport {
        schema: CHECKLIST_SCHEMA,
        kind: ChecklistKind::Run,
        stage,
        status,
        entity: ChecklistEntity {
            target: run_dir.display().to_string(),
            run_dir: Some(run_dir.display().to_string()),
            spec: Some(guide_report.start.spec.clone()),
            shape: guide_report.start.selected_route.clone(),
            ..ChecklistEntity::default()
        },
        activation_policy: "selected_route_execution".to_owned(),
        position: ChecklistPosition {
            current_phase,
            remaining_phases: Some(guide_report.path.remaining_phases.len()),
            ..ChecklistPosition::default()
        },
        steps: vec![ChecklistStep {
            id: match stage {
                ChecklistStage::Entry => "run_entry",
                ChecklistStage::Loop => "run_current_gate",
                ChecklistStage::Exit => "run_exit",
            }
            .to_owned(),
            description: match stage {
                ChecklistStage::Entry => "Orient on selected route and resume anchor.",
                ChecklistStage::Loop => {
                    "Fulfill the current route phase without violating active forbids."
                }
                ChecklistStage::Exit => {
                    "Align proof and produce the final response only after route evidence exists."
                }
            }
            .to_owned(),
            directive: match stage {
                ChecklistStage::Entry => "Use the persisted guide state as authority for selected route, matched rules, current phase, and allowed commands.",
                ChecklistStage::Loop => "Satisfy open requirements, run only allowed commands, checkpoint real evidence at natural boundaries, and repeat until no open requirements remain.",
                ChecklistStage::Exit => "Run final token/progress/alignment commands and report pass, partial, or fail with artifact paths.",
            }
            .to_owned(),
            commands,
            repeat: ChecklistRepeat {
                until: Some(match stage {
                    ChecklistStage::Entry => "current gate is loaded from guide state",
                    ChecklistStage::Loop => {
                        "current phase open requirements are satisfied or explicitly blocked"
                    }
                    ChecklistStage::Exit => {
                        "alignment is pass or intentionally partial with exact missing proof"
                    }
                }
                .to_owned()),
                ..ChecklistRepeat::default()
            },
            requires: open_requirements,
            blocks: guide_report.current_gate.checks.clone(),
            forbid: guide_report.current_gate.do_not.clone(),
            evidence: guide_report.end.proof_paths.clone(),
        }],
        forbid: guide_report.current_gate.do_not.clone(),
        next_command: guide_report
            .current_gate
            .allowed_commands
            .first()
            .cloned()
            .or_else(|| Some(guide_report.resume.command.clone())),
        blockers,
    }
}
