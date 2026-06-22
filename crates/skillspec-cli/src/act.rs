use crate::decision::{Decision, RouteSelectionBasis};
use crate::model::{ExecutionPhase, HandoffBoundary, RouteHandoff, RouteId, SkillSpec};
use crate::trace::TraceWriteResult;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
pub struct ActReport {
    pub input: String,
    pub selected_route: Option<String>,
    pub route_selection: Option<ActRouteSelection>,
    pub matched_rules: Vec<ActMatchedRule>,
    pub current_phase: Option<ActPhase>,
    pub phases: Vec<ActPhase>,
    pub forbidden: Vec<String>,
    pub elicitations: Vec<String>,
    pub after_success: Vec<String>,
    pub allowed_now: Vec<String>,
    pub required_transitions: Vec<String>,
    pub before_tool_call: Vec<String>,
    pub ooda_loop: Vec<String>,
    pub authority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<ActTrace>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActRouteSelection {
    pub route: String,
    pub basis: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActMatchedRule {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActPhase {
    pub id: String,
    pub owner_skill: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub requires: Vec<String>,
    pub checks: Vec<String>,
    pub forbid: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff: Option<ActHandoff>,
    pub jumps: Vec<ActJump>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActHandoff {
    pub to_skill: String,
    pub boundary: String,
    pub pass_context: Vec<String>,
    pub forbid: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActJump {
    pub when: String,
    pub to_phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActTrace {
    pub run_id: String,
    pub run_dir: String,
    pub trace_jsonl: String,
    pub summary_json: String,
}

pub fn build_report(
    spec: &SkillSpec,
    decision: &Decision,
    trace: Option<&TraceWriteResult>,
) -> ActReport {
    let phases = decision
        .execution_plan
        .as_ref()
        .map(|plan| plan.phases.iter().map(phase_report).collect::<Vec<_>>())
        .unwrap_or_default();
    let current_phase = phases.first().cloned();
    let selected_route = decision.route.as_ref().map(|route| route.0.clone());
    let selected_route_model = selected_route_ref(spec, decision.route.as_ref());
    let forbidden = forbidden_items(decision, current_phase.as_ref(), selected_route_model);

    let required_transitions = required_transitions(&phases, selected_route_model);
    let allowed_now = allowed_now(
        decision,
        current_phase.as_ref(),
        selected_route_model,
        &forbidden,
    );
    let before_tool_call = before_tool_call(&forbidden, !decision.elicit.is_empty());

    ActReport {
        input: decision.input.clone(),
        selected_route,
        route_selection: decision
            .route_selection
            .as_ref()
            .map(|selection| ActRouteSelection {
                route: selection.route.0.clone(),
                basis: route_selection_basis_name(&selection.basis).to_owned(),
                rule_id: selection.rule_id.as_ref().map(|rule| rule.0.clone()),
                reason: selection.reason.clone(),
            }),
        matched_rules: decision
            .matched_rules
            .iter()
            .map(|rule| ActMatchedRule {
                id: rule.id.0.clone(),
                reason: rule.reason.clone(),
            })
            .collect(),
        current_phase,
        phases,
        forbidden,
        elicitations: decision.elicit.clone(),
        after_success: decision.after_success.clone(),
        allowed_now,
        required_transitions,
        before_tool_call,
        ooda_loop: vec![
            "Observe: use the user task and this SkillSpec decision trace as the current facts."
                .to_owned(),
            "Orient: apply the selected route, matched rules, current phase, forbids, handoffs, dependencies, and closures.".to_owned(),
            "Decide: choose only an action allowed in the current phase or selected route.".to_owned(),
            "Act: execute the allowed action, capture evidence, then repeat this checklist before the next tool call.".to_owned(),
        ],
        authority:
            "The selected route and matched rules override lower-level skill defaults and generic tool preferences."
                .to_owned(),
        trace: trace.map(|trace| ActTrace {
            run_id: trace.run_id.clone(),
            run_dir: trace.run_dir.display().to_string(),
            trace_jsonl: trace.trace_jsonl.display().to_string(),
            summary_json: trace.summary_json.display().to_string(),
        }),
    }
}

pub fn render(report: &ActReport) -> String {
    let mut output = String::new();
    output.push_str("SkillSpec action checklist\n\n");
    output.push_str(&format!("Input: {}\n", report.input));
    match &report.selected_route {
        Some(route) => output.push_str(&format!("Selected route: {route}\n")),
        None => output.push_str("Selected route: none\n"),
    }
    if let Some(selection) = &report.route_selection {
        output.push_str(&format!("Route selection: {}", selection.basis));
        if let Some(rule_id) = &selection.rule_id {
            output.push_str(&format!(" via {rule_id}"));
        }
        if let Some(reason) = &selection.reason {
            output.push_str(&format!(" ({reason})"));
        }
        output.push('\n');
    }
    output.push_str(&format!("Route authority: {}\n", report.authority));
    if let Some(trace) = &report.trace {
        output.push_str(&format!("Trace: {}\n", trace.run_dir));
    }

    output.push_str("\nOODA loop:\n");
    write_bullets(&mut output, &report.ooda_loop);

    if let Some(phase) = &report.current_phase {
        output.push_str("\nCurrent phase:\n");
        output.push_str(&format!("- {} owned by {}\n", phase.id, phase.owner_skill));
        if let Some(route) = &phase.route {
            output.push_str(&format!("- route scope: {route}\n"));
        }
        if let Some(description) = &phase.description {
            output.push_str(&format!("- description: {description}\n"));
        }
        if !phase.requires.is_empty() {
            output.push_str(&format!("- requires: {}\n", phase.requires.join(", ")));
        }
        if !phase.checks.is_empty() {
            output.push_str(&format!("- checks: {}\n", phase.checks.join(", ")));
        }
        if !phase.forbid.is_empty() {
            output.push_str(&format!("- phase forbids: {}\n", phase.forbid.join(", ")));
        }
    } else {
        output.push_str(
            "\nCurrent phase:\n- no execution plan; selected route is the active scope\n",
        );
    }

    if !report.matched_rules.is_empty() {
        output.push_str("\nMatched rules:\n");
        for rule in &report.matched_rules {
            match &rule.reason {
                Some(reason) => output.push_str(&format!("- {}: {reason}\n", rule.id)),
                None => output.push_str(&format!("- {}\n", rule.id)),
            }
        }
    }

    output.push_str("\nAllowed now:\n");
    write_bullets(&mut output, &report.allowed_now);

    output.push_str("\nForbidden:\n");
    if report.forbidden.is_empty() {
        output.push_str("- none declared\n");
    } else {
        write_bullets(&mut output, &report.forbidden);
    }

    if !report.elicitations.is_empty() {
        output.push_str("\nRequired elicitations:\n");
        write_bullets(&mut output, &report.elicitations);
    }

    if !report.required_transitions.is_empty() {
        output.push_str("\nRequired transitions:\n");
        write_bullets(&mut output, &report.required_transitions);
    }

    if !report.after_success.is_empty() {
        output.push_str("\nAfter success:\n");
        write_bullets(&mut output, &report.after_success);
    }

    output.push_str("\nBefore each tool call:\n");
    for item in &report.before_tool_call {
        output.push_str(&format!("[ ] {item}\n"));
    }

    output.trim_end().to_owned()
}

fn phase_report(phase: &ExecutionPhase) -> ActPhase {
    ActPhase {
        id: phase.id.clone(),
        owner_skill: phase.owner_skill.clone(),
        route: phase.route.as_ref().map(|route| route.0.clone()),
        description: phase.description.clone(),
        requires: phase.requires.clone(),
        checks: phase.checks.clone(),
        forbid: phase.forbid.clone(),
        handoff: phase.handoff.as_ref().map(handoff_report),
        jumps: phase
            .jumps
            .iter()
            .map(|jump| ActJump {
                when: jump.when.clone(),
                to_phase: jump.to_phase.clone(),
                reason: jump.reason.clone(),
            })
            .collect(),
    }
}

fn handoff_report(handoff: &RouteHandoff) -> ActHandoff {
    ActHandoff {
        to_skill: handoff.to_skill.clone(),
        boundary: handoff_boundary_name(&handoff.boundary).to_owned(),
        pass_context: handoff.pass_context.clone(),
        forbid: handoff.forbid.clone(),
        reason: handoff.reason.clone(),
    }
}

fn selected_route_ref<'a>(
    spec: &'a SkillSpec,
    selected_route: Option<&RouteId>,
) -> Option<&'a crate::model::Route> {
    let selected_route = selected_route?;
    spec.routes
        .iter()
        .find(|route| route.id.0 == selected_route.0)
}

fn forbidden_items(
    decision: &Decision,
    current_phase: Option<&ActPhase>,
    selected_route: Option<&crate::model::Route>,
) -> Vec<String> {
    let mut items = decision.forbid.clone();
    if let Some(phase) = current_phase {
        items.extend(phase.forbid.clone());
        if let Some(handoff) = &phase.handoff {
            items.extend(handoff.forbid.clone());
        }
    }
    if let Some(route) = selected_route {
        if let Some(handoff) = &route.handoff {
            items.extend(handoff.forbid.clone());
        }
    }
    unique(items)
}

fn allowed_now(
    decision: &Decision,
    current_phase: Option<&ActPhase>,
    selected_route: Option<&crate::model::Route>,
    forbidden: &[String],
) -> Vec<String> {
    let mut allowed = Vec::new();
    if let Some(phase) = current_phase {
        allowed.push(format!(
            "execute current phase `{}` requirements before later phases",
            phase.id
        ));
        allowed.push(format!(
            "stay within owner skill `{}` for this phase",
            phase.owner_skill
        ));
        if !phase.requires.is_empty() {
            allowed.push(format!(
                "satisfy phase requirements: {}",
                phase.requires.join(", ")
            ));
        }
    } else if let Some(route) = &decision.route {
        allowed.push(format!(
            "use selected route `{}` as the active action scope",
            route.0
        ));
    }

    if has_cli_rote_constraint(forbidden) {
        allowed.push(
            "for CLI/shell/process work, use rote flow search, a named rote workspace, and `rote exec --`"
                .to_owned(),
        );
    }
    if selected_route
        .and_then(|route| route.handoff.as_ref())
        .is_some()
    {
        allowed.push(
            "follow the selected route handoff before using the target skill's tools".to_owned(),
        );
    }
    if current_phase
        .and_then(|phase| phase.handoff.as_ref())
        .is_some()
    {
        allowed.push("follow the current phase handoff boundary before continuing".to_owned());
    }
    allowed.push("inspect active details with `skillspec query` or `skillspec refs` when the checklist is not specific enough".to_owned());
    allowed.push(
        "capture command output, files, traces, or response ids as evidence for later alignment"
            .to_owned(),
    );
    unique(allowed)
}

fn required_transitions(
    phases: &[ActPhase],
    selected_route: Option<&crate::model::Route>,
) -> Vec<String> {
    let mut transitions = Vec::new();
    if phases.len() > 1 {
        for window in phases.windows(2) {
            transitions.push(format!(
                "complete phase `{}` before starting phase `{}`",
                window[0].id, window[1].id
            ));
        }
    }
    for phase in phases {
        if let Some(handoff) = &phase.handoff {
            transitions.push(format!(
                "phase `{}` hands off to `{}` with boundary `{}`",
                phase.id, handoff.to_skill, handoff.boundary
            ));
        }
        for jump in &phase.jumps {
            transitions.push(format!(
                "if `{}`, jump from phase `{}` to `{}`",
                jump.when, phase.id, jump.to_phase
            ));
        }
    }
    if let Some(route) = selected_route {
        if let Some(handoff) = &route.handoff {
            transitions.push(format!(
                "selected route hands off to `{}` with boundary `{}`",
                handoff.to_skill,
                handoff_boundary_name(&handoff.boundary)
            ));
        }
    }
    unique(transitions)
}

fn before_tool_call(forbidden: &[String], has_elicitations: bool) -> Vec<String> {
    let mut checks = vec![
        "Is this tool or substrate allowed by the current phase or selected route?".to_owned(),
        "Does this action violate any listed forbid?".to_owned(),
        "Do the selected route and matched rules override any lower-level default I am about to follow?".to_owned(),
        "Are required dependencies, checks, or command-specific requirements satisfied?".to_owned(),
        "If a handoff boundary applies, has the handoff happened exactly as specified?".to_owned(),
        "Will the result be captured as evidence for trace alignment or final reporting?".to_owned(),
    ];
    if has_elicitations {
        checks.insert(
            3,
            "Have required elicitations been answered or explicitly waived?".to_owned(),
        );
    }
    if forbidden.iter().any(|item| item.contains("native_search")) {
        checks.insert(
            1,
            "If this is search or browser work, did the checklist explicitly allow native web tools?".to_owned(),
        );
    }
    checks
}

fn has_cli_rote_constraint(forbidden: &[String]) -> bool {
    forbidden.iter().any(|item| {
        item.contains("direct_cli_without_rote_exec")
            || item.contains("direct_shell_command_without_rote_exec")
            || item.contains("direct_harness_cli_call_without_rote_exec")
            || item.contains("rote_exec_outside_workspace")
    })
}

fn route_selection_basis_name(basis: &RouteSelectionBasis) -> &'static str {
    match basis {
        RouteSelectionBasis::RulePrefer => "rule_prefer",
        RouteSelectionBasis::RouteOrderDefault => "route_order_default",
        RouteSelectionBasis::DefaultRouteOrder => "default_route_order",
    }
}

fn handoff_boundary_name(boundary: &HandoffBoundary) -> &'static str {
    match boundary {
        HandoffBoundary::StopCurrentSkill => "stop_current_skill",
        HandoffBoundary::ResumeAfterHandoff => "resume_after_handoff",
    }
}

fn write_bullets(output: &mut String, items: &[String]) {
    for item in items {
        output.push_str(&format!("- {item}\n"));
    }
}

fn unique(items: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut unique_items = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            unique_items.push(item);
        }
    }
    unique_items
}
