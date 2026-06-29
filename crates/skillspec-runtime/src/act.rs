use crate::decision::{Decision, RouteSelectionBasis};
use crate::trace::TraceWriteResult;
use serde::Serialize;
use skillspec_core::error::{Error, Result};
use skillspec_core::model::{
    ExecutionPhase, HandoffBoundary, RouteHandoff, RouteId, SkillSpec, ToolBoundary,
    ToolBoundaryDefault,
};
use std::collections::BTreeSet;
use std::path::Path;

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
    pub tool_boundary: ActToolBoundary,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_boundary: Option<ActDeclaredToolBoundary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActToolBoundary {
    pub default: String,
    pub allow: Vec<String>,
    pub forbid: Vec<String>,
    pub permission_required_for: Vec<String>,
    pub instruction: String,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActDeclaredToolBoundary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub allow: Vec<String>,
    pub forbid: Vec<String>,
    pub permission_required_for: Vec<String>,
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
    build_report_for_phase(spec, decision, trace, None).expect("phase is not specified")
}

pub fn build_report_for_phase(
    spec: &SkillSpec,
    decision: &Decision,
    trace: Option<&TraceWriteResult>,
    phase_id: Option<&str>,
) -> Result<ActReport> {
    let phases = decision
        .execution_plan
        .as_ref()
        .map(|plan| plan.phases.iter().map(phase_report).collect::<Vec<_>>())
        .unwrap_or_default();
    let current_phase = match phase_id {
        Some(phase_id) => Some(
            phases
                .iter()
                .find(|phase| phase.id == phase_id)
                .cloned()
                .ok_or_else(|| Error::InvalidInput {
                    message: format!("unknown execution phase {phase_id:?}"),
                })?,
        ),
        None => phases.first().cloned(),
    };
    let selected_route = decision.route.as_ref().map(|route| route.0.clone());
    let selected_route_model = selected_route_ref(spec, decision.route.as_ref());
    let forbidden = forbidden_items(decision, current_phase.as_ref(), selected_route_model);
    let tool_boundary = effective_tool_boundary(
        spec,
        selected_route_model,
        current_phase.as_ref(),
        &forbidden,
    );

    let required_transitions = required_transitions(&phases, selected_route_model);
    let allowed_now = allowed_now(
        decision,
        current_phase.as_ref(),
        selected_route_model,
        &forbidden,
    );
    let before_tool_call = before_tool_call(!decision.elicit.is_empty());

    Ok(ActReport {
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
            "Orient: apply the selected route, matched rules, current phase, phase tool boundary, forbids, handoffs, dependencies, and closures.".to_owned(),
            "Decide: choose only an action allowed in the current phase or selected route.".to_owned(),
            "Act: execute the allowed action, capture evidence, then repeat this checklist before the next tool call.".to_owned(),
        ],
        tool_boundary,
        authority:
            "The selected route and matched rules override lower-level skill defaults and generic tool preferences."
                .to_owned(),
        trace: trace.map(|trace| ActTrace {
            run_id: trace.run_id.clone(),
            run_dir: trace.run_dir.display().to_string(),
            trace_jsonl: trace.trace_jsonl.display().to_string(),
            summary_json: trace.summary_json.display().to_string(),
        }),
    })
}

pub fn trace_for_run(run_dir: &Path) -> ActTrace {
    let run_id = run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();
    ActTrace {
        run_id,
        run_dir: run_dir.display().to_string(),
        trace_jsonl: run_dir.join("trace.jsonl").display().to_string(),
        summary_json: run_dir.join("summary.json").display().to_string(),
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

    output.push_str("\nPHASE TOOL BOUNDARY - HARD\n");
    output.push_str(&format!("- default: {}\n", report.tool_boundary.default));
    output.push_str(&format!(
        "- allowed: {}\n",
        join_or_none(&report.tool_boundary.allow)
    ));
    output.push_str(&format!(
        "- forbidden: {}\n",
        join_or_none(&report.tool_boundary.forbid)
    ));
    output.push_str(&format!(
        "- permission required for: {}\n",
        join_or_none(&report.tool_boundary.permission_required_for)
    ));
    output.push_str(&format!(
        "- instruction: {}\n",
        report.tool_boundary.instruction
    ));

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

pub fn render_plan(report: &ActReport) -> String {
    let mut output = String::new();
    output.push_str("SkillSpec phase plan\n\n");
    output.push_str(&format!("Input: {}\n", report.input));
    match &report.selected_route {
        Some(route) => output.push_str(&format!("Selected route: {route}\n")),
        None => output.push_str("Selected route: none\n"),
    }
    if let Some(selection) = &report.route_selection {
        output.push_str(&format!("Route selection: {}\n", selection.basis));
    }
    if let Some(trace) = &report.trace {
        output.push_str(&format!("Run: {}\n", trace.run_dir));
    }

    output.push_str("\nPhases:\n");
    if report.phases.is_empty() {
        output.push_str("- no execution plan; selected route is the active scope\n");
    } else {
        for (index, phase) in report.phases.iter().enumerate() {
            output.push_str(&format!(
                "{}. {} owned by {}\n",
                index + 1,
                phase.id,
                phase.owner_skill
            ));
            if let Some(description) = &phase.description {
                output.push_str(&format!("   description: {description}\n"));
            }
            if !phase.requires.is_empty() {
                output.push_str(&format!("   requires: {}\n", phase.requires.join(", ")));
            }
            if !phase.forbid.is_empty() {
                output.push_str(&format!("   forbids: {}\n", phase.forbid.join(", ")));
            }
        }
    }

    if let Some(phase) = &report.current_phase {
        output.push_str(&format!("\nCurrent phase: {}\n", phase.id));
        output.push_str(&format!(
            "Next: skillspec act <skill.spec.yml> --input '<task>' --phase {}\n",
            phase.id
        ));
    }
    if !report.required_transitions.is_empty() {
        output.push_str("\nTransitions:\n");
        write_bullets(&mut output, &report.required_transitions);
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
        tool_boundary: phase.tool_boundary.as_ref().map(declared_tool_boundary),
    }
}

fn declared_tool_boundary(boundary: &ToolBoundary) -> ActDeclaredToolBoundary {
    ActDeclaredToolBoundary {
        default: boundary.default.as_ref().map(tool_boundary_default_name),
        allow: boundary.allow.clone(),
        forbid: boundary.forbid.clone(),
        permission_required_for: boundary.permission_required_for.clone(),
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
) -> Option<&'a skillspec_core::model::Route> {
    let selected_route = selected_route?;
    spec.routes
        .iter()
        .find(|route| route.id.0 == selected_route.0)
}

fn forbidden_items(
    decision: &Decision,
    current_phase: Option<&ActPhase>,
    selected_route: Option<&skillspec_core::model::Route>,
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

fn effective_tool_boundary(
    spec: &SkillSpec,
    selected_route: Option<&skillspec_core::model::Route>,
    current_phase: Option<&ActPhase>,
    active_forbids: &[String],
) -> ActToolBoundary {
    let mut default = ToolBoundaryDefault::Deny;
    let mut allow = vec![
        "skillspec_cli".to_owned(),
        "current_phase_owner_skill".to_owned(),
        "declared_commands_dependencies_imports_resources".to_owned(),
        "local_files_referenced_by_active_spec".to_owned(),
    ];
    let mut forbid = active_forbids.to_vec();
    let mut permission_required_for = vec![
        "any_unlisted_tool".to_owned(),
        "any_forbidden_action".to_owned(),
        "any_new_data_source".to_owned(),
        "any_new_execution_substrate".to_owned(),
        "any_new_provider_or_adapter".to_owned(),
        "any_external_side_effect".to_owned(),
    ];
    let mut sources = vec!["runtime_default".to_owned()];

    if let Some(entry) = &spec.entry {
        if let Some(boundary) = &entry.tool_boundary {
            merge_tool_boundary(
                "entry.tool_boundary",
                boundary,
                &mut default,
                &mut allow,
                &mut forbid,
                &mut permission_required_for,
                &mut sources,
            );
        }
    }

    if let Some(route) = selected_route {
        if let Some(boundary) = &route.tool_boundary {
            merge_tool_boundary(
                &format!("route:{}.tool_boundary", route.id.0),
                boundary,
                &mut default,
                &mut allow,
                &mut forbid,
                &mut permission_required_for,
                &mut sources,
            );
        }
    }

    if let Some(phase) = current_phase {
        if let Some(boundary) = &phase.tool_boundary {
            merge_declared_tool_boundary(
                &format!("phase:{}.tool_boundary", phase.id),
                boundary,
                &mut default,
                &mut allow,
                &mut forbid,
                &mut permission_required_for,
                &mut sources,
            );
        }
    }

    ActToolBoundary {
        default: tool_boundary_default_name(&default),
        allow: unique(allow),
        forbid: unique(forbid),
        permission_required_for: unique(permission_required_for),
        instruction:
            "Use only the allowed tools and substrates for this phase. If the next action needs any unlisted tool, forbidden action, new data source, new execution substrate, provider, adapter, CLI, browser mode, API, or skill, stop and ask for explicit permission before using it."
                .to_owned(),
        sources: unique(sources),
    }
}

fn merge_tool_boundary(
    source: &str,
    boundary: &ToolBoundary,
    default: &mut ToolBoundaryDefault,
    allow: &mut Vec<String>,
    forbid: &mut Vec<String>,
    permission_required_for: &mut Vec<String>,
    sources: &mut Vec<String>,
) {
    if let Some(boundary_default) = &boundary.default {
        *default = boundary_default.clone();
    }
    allow.extend(boundary.allow.clone());
    forbid.extend(boundary.forbid.clone());
    permission_required_for.extend(boundary.permission_required_for.clone());
    sources.push(source.to_owned());
}

fn merge_declared_tool_boundary(
    source: &str,
    boundary: &ActDeclaredToolBoundary,
    default: &mut ToolBoundaryDefault,
    allow: &mut Vec<String>,
    forbid: &mut Vec<String>,
    permission_required_for: &mut Vec<String>,
    sources: &mut Vec<String>,
) {
    if let Some(boundary_default) = &boundary.default {
        *default = match boundary_default.as_str() {
            "allow" => ToolBoundaryDefault::Allow,
            _ => ToolBoundaryDefault::Deny,
        };
    }
    allow.extend(boundary.allow.clone());
    forbid.extend(boundary.forbid.clone());
    permission_required_for.extend(boundary.permission_required_for.clone());
    sources.push(source.to_owned());
}

fn allowed_now(
    decision: &Decision,
    current_phase: Option<&ActPhase>,
    selected_route: Option<&skillspec_core::model::Route>,
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
    selected_route: Option<&skillspec_core::model::Route>,
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

fn before_tool_call(has_elicitations: bool) -> Vec<String> {
    let mut checks = vec![
        "Is this tool, data source, execution substrate, provider, adapter, CLI, browser mode, API, or skill explicitly allowed by the phase tool boundary?".to_owned(),
        "If the next action is unlisted or forbidden, stop and ask for permission before using it.".to_owned(),
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

fn tool_boundary_default_name(boundary: &ToolBoundaryDefault) -> String {
    match boundary {
        ToolBoundaryDefault::Allow => "allow".to_owned(),
        ToolBoundaryDefault::Deny => "deny".to_owned(),
    }
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
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
