mod fingerprint;
mod render;
mod state;
mod types;

use crate::act::{self, ActPhase, ActReport};
use crate::decision::{self, DecisionWithEvents};
use crate::error::{Error, Result};
use crate::model::SkillSpec;
use crate::progress::{self, ProgressReport};
use crate::trace::{self, TraceEnvelope};
use std::path::Path;

pub use render::render_text;
pub use types::{GuideMode, GuideReport};

use types::{
    CurrentGate, EndAnchor, GuidePath, GuideStartMode, GuideStatePaths, GuideWarning,
    GuideWarningKind, ProgressRecordHint, ResumeAnchor, RouteSelectionAnchor, StartAnchor,
    GUIDE_SCHEMA,
};

pub struct BuildOptions<'a> {
    pub spec: &'a SkillSpec,
    pub spec_path: &'a Path,
    pub input: Option<&'a str>,
    pub resume_run_dir: Option<&'a Path>,
    pub trace_dir: Option<&'a Path>,
    pub phase_override: Option<&'a str>,
    pub guide_mode: GuideMode,
}

pub fn build_report(options: BuildOptions<'_>) -> Result<GuideReport> {
    match options.resume_run_dir {
        Some(run_dir) => build_resume(options, run_dir),
        None => build_start(options),
    }
}

fn build_start(options: BuildOptions<'_>) -> Result<GuideReport> {
    let input = options.input.ok_or_else(|| Error::InvalidInput {
        message: "run-loop --guide requires --input or --resume".to_owned(),
    })?;
    let trace_dir = options.trace_dir.ok_or_else(|| Error::InvalidInput {
        message:
            "run-loop --guide with --input requires --trace-dir so guide state survives compaction"
                .to_owned(),
    })?;
    let decision_with_events = decision::decide_with_events(options.spec, input);
    let trace = trace::write_decision_trace(
        trace_dir,
        options.spec_path,
        options.spec,
        &decision_with_events,
    )?;
    let progress = progress::show(options.spec, &trace.run_dir)?;
    let current_phase = options.phase_override.or(progress.current_phase.as_deref());
    let act = act::build_report_for_phase(
        options.spec,
        &decision_with_events.decision,
        Some(&trace),
        current_phase,
    )?;
    let report = assemble_report(AssembleInputs {
        mode: GuideStartMode::Start,
        guide_mode: options.guide_mode,
        spec: options.spec,
        spec_path: options.spec_path,
        input,
        decision_with_events: &decision_with_events,
        act: &act,
        progress: &progress,
        run_dir: &trace.run_dir,
        current_phase,
        warnings: Vec::new(),
    })?;
    state::write(&trace.run_dir, &report)?;
    Ok(report)
}

fn build_resume(options: BuildOptions<'_>, run_dir: &Path) -> Result<GuideReport> {
    if options.input.is_some() {
        return Err(Error::InvalidInput {
            message: "--input and --resume cannot be used together".to_owned(),
        });
    }
    let trace = trace::compact(run_dir)?;
    let envelopes = trace::read_envelopes(run_dir)?;
    if envelopes.is_empty() {
        return Err(Error::InvalidInput {
            message: format!(
                "cannot resume {}; decision trace has no events",
                run_dir.display()
            ),
        });
    }
    let input = trace_input(&envelopes)?;
    let expected_input_sha = first_input_sha(&envelopes).ok_or_else(|| Error::InvalidInput {
        message: "cannot resume; decision trace has no input_sha256".to_owned(),
    })?;
    let actual_input_sha = trace::input_sha256(&input);
    if expected_input_sha != actual_input_sha {
        return Err(Error::InvalidInput {
            message: "cannot resume; decision trace input hash does not match recovered input"
                .to_owned(),
        });
    }

    let prior = state::read_prior(run_dir)?;
    if let Some(prior) = &prior {
        if prior.start.input_sha256 != actual_input_sha {
            return Err(Error::InvalidInput {
                message: "cannot resume; prior guide state was created for a different input hash"
                    .to_owned(),
            });
        }
    }

    let decision_with_events = decision::decide_with_events(options.spec, &input);
    let progress = progress::show(options.spec, run_dir)?;
    let current_phase = options.phase_override.or(progress.current_phase.as_deref());
    let act = act::build_report_for_phase(
        options.spec,
        &decision_with_events.decision,
        Some(&trace),
        current_phase,
    )?;
    let spec_fingerprint = trace::spec_fingerprint(options.spec, options.spec_path)?;
    let decision_fingerprint =
        fingerprint::decision_fingerprint(&decision_with_events.decision, &act, &actual_input_sha)?;
    let mut warnings = Vec::new();
    if let Some(trace_spec_fingerprint) = first_spec_fingerprint(&envelopes) {
        if trace_spec_fingerprint != spec_fingerprint {
            match &prior {
                Some(prior) if prior.start.decision_fingerprint != decision_fingerprint => {
                    return Err(Error::InvalidInput {
                        message: "cannot resume; spec changed and the active route/gate decision fingerprint changed. Re-plan with --input instead."
                            .to_owned(),
                    });
                }
                Some(prior) => warnings.push(GuideWarning {
                    kind: GuideWarningKind::SpecChangedDecisionStable,
                    message: format!(
                        "spec changed since run start ({} -> {}), but prior guide decision fingerprint is stable",
                        prior.start.spec_fingerprint, spec_fingerprint
                    ),
                }),
                None => {
                    let traced_route = selected_route_from_trace(&envelopes);
                    let current_route = decision_with_events
                        .decision
                        .route
                        .as_ref()
                        .map(|route| route.0.clone());
                    if traced_route.is_some() && traced_route != current_route {
                        return Err(Error::InvalidInput {
                            message: "cannot resume; spec changed and selected route changed. Re-plan with --input instead."
                                .to_owned(),
                        });
                    }
                    warnings.push(GuideWarning {
                        kind: GuideWarningKind::SpecChangedNoPriorGuide,
                        message: "spec changed since run start and no prior guide-state decision fingerprint exists; selected route still matches trace"
                            .to_owned(),
                    });
                }
            }
        }
    }

    let report = assemble_report(AssembleInputs {
        mode: GuideStartMode::Resume,
        guide_mode: options.guide_mode,
        spec: options.spec,
        spec_path: options.spec_path,
        input: &input,
        decision_with_events: &decision_with_events,
        act: &act,
        progress: &progress,
        run_dir,
        current_phase,
        warnings,
    })?;
    state::write(run_dir, &report)?;
    Ok(report)
}

struct AssembleInputs<'a> {
    mode: GuideStartMode,
    guide_mode: GuideMode,
    spec: &'a SkillSpec,
    spec_path: &'a Path,
    input: &'a str,
    decision_with_events: &'a DecisionWithEvents,
    act: &'a ActReport,
    progress: &'a ProgressReport,
    run_dir: &'a Path,
    current_phase: Option<&'a str>,
    warnings: Vec<GuideWarning>,
}

fn assemble_report(inputs: AssembleInputs<'_>) -> Result<GuideReport> {
    let input_sha256 = trace::input_sha256(inputs.input);
    let spec_fingerprint = trace::spec_fingerprint(inputs.spec, inputs.spec_path)?;
    let decision_fingerprint = fingerprint::decision_fingerprint(
        &inputs.decision_with_events.decision,
        inputs.act,
        &input_sha256,
    )?;
    let spec_path = inputs.spec_path.display().to_string();
    let run_dir = inputs.run_dir.display().to_string();
    let guide_state = state::guide_state_path(inputs.run_dir)
        .display()
        .to_string();
    let guide_summary = state::guide_summary_path(inputs.run_dir)
        .display()
        .to_string();
    let current_phase = inputs
        .current_phase
        .or(inputs.progress.current_phase.as_deref())
        .map(str::to_owned);
    let first_phase = inputs.act.phases.first().map(|phase| phase.id.clone());
    let current_phase_model = current_phase
        .as_ref()
        .and_then(|phase| phase_by_id(inputs.act, phase));
    let selected_route = inputs
        .decision_with_events
        .decision
        .route
        .as_ref()
        .map(|route| route.0.clone());
    let matched_rules = inputs
        .act
        .matched_rules
        .iter()
        .map(|rule| rule.id.clone())
        .collect::<Vec<_>>();
    let start = StartAnchor {
        spec: spec_path.clone(),
        spec_id: inputs.spec.id.clone(),
        run_dir: run_dir.clone(),
        input_sha256,
        spec_fingerprint,
        decision_fingerprint,
        selected_route: selected_route.clone(),
        route_selection: inputs.act.route_selection.as_ref().map(|selection| {
            RouteSelectionAnchor {
                basis: selection.basis.clone(),
                rule_id: selection.rule_id.clone(),
                reason: selection.reason.clone(),
            }
        }),
        matched_rules: matched_rules.clone(),
        route_candidates_seen: inputs.spec.routes.len(),
        first_phase,
        current_phase: current_phase.clone(),
    };
    let path = GuidePath {
        phase_order: inputs
            .act
            .phases
            .iter()
            .map(|phase| phase.id.clone())
            .collect(),
        completed_phases: inputs.progress.completed_phases.clone(),
        blocked_phases: inputs.progress.blocked_phases.clone(),
        remaining_phases: inputs.progress.remaining_phases.clone(),
        required_transitions: inputs.act.required_transitions.clone(),
    };
    let current_gate = build_current_gate(CurrentGateInputs {
        spec: inputs.spec,
        spec_path: &spec_path,
        run_dir: &run_dir,
        selected_route: selected_route.as_deref(),
        matched_rules: &matched_rules,
        act: inputs.act,
        phase: current_phase_model,
        progress: inputs.progress,
    });
    let end = build_end_anchor(&spec_path, &run_dir, selected_route.as_deref());
    let resume = ResumeAnchor {
        command: format!(
            "skillspec run-loop {} --resume {} --guide agent",
            shell_arg(&spec_path),
            shell_arg(&run_dir)
        ),
        guide_state: guide_state.clone(),
        guide_summary: guide_summary.clone(),
    };
    let state_paths = GuideStatePaths {
        guide_state,
        guide_summary,
    };
    Ok(GuideReport {
        schema: GUIDE_SCHEMA.to_owned(),
        mode: inputs.mode,
        guide: inputs.guide_mode,
        start,
        path,
        current_gate,
        end,
        resume,
        warnings: inputs.warnings,
        state_paths,
    })
}

struct CurrentGateInputs<'a> {
    spec: &'a SkillSpec,
    spec_path: &'a str,
    run_dir: &'a str,
    selected_route: Option<&'a str>,
    matched_rules: &'a [String],
    act: &'a ActReport,
    phase: Option<&'a ActPhase>,
    progress: &'a ProgressReport,
}

fn build_current_gate(inputs: CurrentGateInputs<'_>) -> CurrentGate {
    let mut do_now = Vec::new();
    let mut do_not = inputs.act.forbidden.clone();
    let mut allowed_commands = Vec::new();
    let mut recommended_queries = Vec::new();
    let mut progress_to_record = Vec::new();
    let mut when_to_advance = Vec::new();

    if let Some(route) = inputs.selected_route {
        recommended_queries.push(format!(
            "skillspec query {} route:{} --view summary",
            shell_arg(inputs.spec_path),
            route
        ));
        recommended_queries.push(format!(
            "skillspec refs {} route:{} --view summary",
            shell_arg(inputs.spec_path),
            route
        ));
    }
    for rule in inputs.matched_rules.iter().take(3) {
        recommended_queries.push(format!(
            "skillspec query {} rule:{} --view summary",
            shell_arg(inputs.spec_path),
            rule
        ));
    }

    if let Some(phase) = inputs.phase {
        if !inputs.progress.open_requirements.is_empty() {
            do_now.push(format!(
                "satisfy or explicitly block current open requirements: {}",
                inputs.progress.open_requirements.join(", ")
            ));
        } else if !phase.requires.is_empty() {
            do_now.push(format!(
                "satisfy phase requirements: {}",
                phase.requires.join(", ")
            ));
        } else {
            do_now.push(format!("complete current phase `{}`", phase.id));
        }
        if !phase.checks.is_empty() {
            do_now.push(format!(
                "run or verify phase checks: {}",
                phase.checks.join(", ")
            ));
        }
        for command_id in phase.requires.iter().chain(phase.checks.iter()) {
            if let Some(command) = inputs.spec.commands.get(command_id) {
                allowed_commands.push(format!(
                    "declared command {}: {}",
                    command_id, command.template
                ));
                allowed_commands.push(format!(
                    "skillspec deps check {} --command {}",
                    shell_arg(inputs.spec_path),
                    command_id
                ));
                recommended_queries.push(format!(
                    "skillspec query {} command:{} --view summary",
                    shell_arg(inputs.spec_path),
                    command_id
                ));
            }
        }
        if !inputs.act.elicitations.is_empty() {
            do_now.push(format!(
                "answer or explicitly waive required elicitations: {}",
                inputs.act.elicitations.join(", ")
            ));
        }
        allowed_commands.push(format!(
            "skillspec act {} --input '<same-task>' --run {} --phase {}",
            shell_arg(inputs.spec_path),
            shell_arg(inputs.run_dir),
            phase.id
        ));
        for requirement in &inputs.progress.open_requirements {
            let command = format!(
                "skillspec progress record {} requirement-satisfied {} {} --evidence-kind <kind> --evidence-ref <ref>",
                shell_arg(inputs.run_dir),
                phase.id,
                requirement
            );
            progress_to_record.push(ProgressRecordHint {
                event: "requirement_satisfied".to_owned(),
                phase: Some(phase.id.clone()),
                requirement: Some(requirement.clone()),
                command: command.clone(),
            });
            allowed_commands.push(command);
        }
        let phase_completed = format!(
            "skillspec progress record {} phase-completed {} --evidence-kind <kind> --evidence-ref <ref>",
            shell_arg(inputs.run_dir),
            phase.id
        );
        progress_to_record.push(ProgressRecordHint {
            event: "phase_completed".to_owned(),
            phase: Some(phase.id.clone()),
            requirement: None,
            command: phase_completed.clone(),
        });
        allowed_commands.push(phase_completed);
        when_to_advance
            .push("record required evidence, then mark the phase completed or blocked".to_owned());
    } else if inputs.progress.current_phase.is_none()
        && !inputs.progress.completed_phases.is_empty()
    {
        do_now.push("all phases are completed or blocked; move to the end anchor".to_owned());
        when_to_advance.push("run final proof and alignment from the end anchor".to_owned());
    } else {
        do_now.push("use the selected route as the active scope".to_owned());
        when_to_advance
            .push("record route fulfillment or a blocker before final response".to_owned());
    }

    if do_not.is_empty() {
        do_not.push("do not use unlisted tools, data sources, or execution substrates without explicit permission".to_owned());
    }
    allowed_commands.push(format!(
        "skillspec progress show {} --run {}",
        shell_arg(inputs.spec_path),
        shell_arg(inputs.run_dir)
    ));

    CurrentGate {
        phase: inputs.phase.map(|phase| phase.id.clone()),
        owner_skill: inputs.phase.map(|phase| phase.owner_skill.clone()),
        route_scope: inputs.phase.and_then(|phase| phase.route.clone()),
        description: inputs.phase.and_then(|phase| phase.description.clone()),
        open_requirements: inputs.progress.open_requirements.clone(),
        checks: inputs
            .phase
            .map(|phase| phase.checks.clone())
            .unwrap_or_default(),
        do_now,
        do_not,
        allowed_now: inputs.act.allowed_now.clone(),
        allowed_commands,
        recommended_queries,
        progress_to_record,
        when_to_advance,
    }
}

fn build_end_anchor(spec_path: &str, run_dir: &str, selected_route: Option<&str>) -> EndAnchor {
    let route_id = selected_route.unwrap_or("<route-id>");
    EndAnchor {
        done_when: vec![
            "selected route is fulfilled or intentionally partial".to_owned(),
            "required checks passed or proof gaps are named".to_owned(),
            "progress evidence is recorded in execution.jsonl".to_owned(),
            "final-response evidence is recorded".to_owned(),
            "compact alignment summary is generated".to_owned(),
        ],
        route_fulfillment_event: "route-fulfilled".to_owned(),
        final_progress_command: format!(
            "skillspec progress final-response {} --phase <phase-id> --requirement <requirement-id> --result --evidence --alignment --token-savings",
            shell_arg(run_dir)
        ),
        alignment_command: format!(
            "skillspec trace align {} --decision-trace {} --execution-trace {}/execution.jsonl --summary --proof-digest {}/proof-digest.json",
            shell_arg(spec_path),
            shell_arg(run_dir),
            shell_arg(run_dir),
            shell_arg(run_dir)
        ),
        final_response_must_include: vec![
            "result".to_owned(),
            "evidence paths".to_owned(),
            "alignment summary".to_owned(),
            "token usage or not recorded".to_owned(),
            "selected route".to_owned(),
            "run directory".to_owned(),
        ],
        proof_paths: vec![
            format!("{run_dir}/execution.jsonl"),
            format!("{run_dir}/alignment.json"),
            format!("{run_dir}/proof-digest.json"),
            format!("route fulfillment event for `{route_id}`"),
        ],
    }
}

fn phase_by_id<'a>(act: &'a ActReport, phase_id: &str) -> Option<&'a ActPhase> {
    act.phases.iter().find(|phase| phase.id == phase_id)
}

fn trace_input(envelopes: &[TraceEnvelope]) -> Result<String> {
    envelopes
        .iter()
        .rev()
        .find(|envelope| envelope.event_name == "input_received")
        .and_then(|envelope| envelope.data.get("input"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::InvalidInput {
            message: "decision trace does not contain an input_received event with input"
                .to_owned(),
        })
}

fn first_input_sha(envelopes: &[TraceEnvelope]) -> Option<String> {
    envelopes
        .iter()
        .find_map(|envelope| envelope.input_sha256.clone())
}

fn first_spec_fingerprint(envelopes: &[TraceEnvelope]) -> Option<String> {
    envelopes
        .iter()
        .find_map(|envelope| envelope.spec_fingerprint.clone())
}

fn selected_route_from_trace(envelopes: &[TraceEnvelope]) -> Option<String> {
    envelopes
        .iter()
        .find(|envelope| envelope.event_name == "route_selected")
        .and_then(|envelope| envelope.data.get("route"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn shell_arg(value: &str) -> String {
    if value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-' | b':')
    }) {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
