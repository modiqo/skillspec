use super::types::{CurrentGate, GuideMode, GuideReport, GuideWarningKind};

pub fn render_text(report: &GuideReport) -> String {
    let mut output = String::new();
    output.push_str("SkillSpec guide\n\n");
    render_start(&mut output, report);
    render_path(&mut output, report);
    render_current_gate(&mut output, &report.current_gate);
    render_end(&mut output, report);
    render_resume(&mut output, report);
    render_warnings(&mut output, report);
    if report.guide == GuideMode::Full {
        render_full_detail(&mut output, report);
    }
    output.trim_end().to_owned()
}

pub fn render_summary_markdown(report: &GuideReport) -> String {
    let mut output = String::new();
    output.push_str("# SkillSpec Guide Summary\n\n");
    output.push_str(&format!("- run_dir: `{}`\n", report.start.run_dir));
    output.push_str(&format!(
        "- selected_route: `{}`\n",
        report.start.selected_route.as_deref().unwrap_or("none")
    ));
    output.push_str(&format!(
        "- current_phase: `{}`\n",
        report.current_gate.phase.as_deref().unwrap_or("none")
    ));
    output.push_str(&format!(
        "- open_requirements: {}\n",
        join_or_none(&report.current_gate.open_requirements)
    ));
    output.push_str("\n## Next Commands\n\n");
    write_command_bullets(&mut output, &report.current_gate.allowed_commands);
    output.push_str("\n## End Anchor\n\n");
    write_bullets(&mut output, &report.end.done_when);
    output.push_str("\n## Resume\n\n");
    output.push_str(&format!("`{}`\n", report.resume.command));
    output
}

fn render_start(output: &mut String, report: &GuideReport) {
    output.push_str("START\n");
    output.push_str(&format!("- spec: {}\n", report.start.spec));
    output.push_str(&format!("- run_dir: {}\n", report.start.run_dir));
    output.push_str(&format!(
        "- selected_route: {}\n",
        report.start.selected_route.as_deref().unwrap_or("none")
    ));
    if let Some(selection) = &report.start.route_selection {
        output.push_str(&format!("- route_selection: {}", selection.basis));
        if let Some(rule) = &selection.rule_id {
            output.push_str(&format!(" via {rule}"));
        }
        if let Some(reason) = &selection.reason {
            output.push_str(&format!(" ({reason})"));
        }
        output.push('\n');
    }
    output.push_str(&format!(
        "- matched_rules: {}\n",
        join_or_none(&report.start.matched_rules)
    ));
    output.push_str(&format!(
        "- route_candidates_seen: {}\n",
        report.start.route_candidates_seen
    ));
    output.push_str(&format!(
        "- first_phase: {}\n",
        report.start.first_phase.as_deref().unwrap_or("none")
    ));
    output.push_str(&format!(
        "- current_phase: {}\n",
        report.start.current_phase.as_deref().unwrap_or("none")
    ));
    output.push('\n');
}

fn render_path(output: &mut String, report: &GuideReport) {
    output.push_str("PATH\n");
    if report.path.phase_order.is_empty() {
        output.push_str("- no execution plan; selected route is the active scope\n");
    } else {
        output.push_str(&format!("{}\n", report.path.phase_order.join(" -> ")));
    }
    if !report.path.completed_phases.is_empty() {
        output.push_str(&format!(
            "- completed: {}\n",
            join_or_none(&report.path.completed_phases)
        ));
    }
    if !report.path.blocked_phases.is_empty() {
        output.push_str(&format!(
            "- blocked: {}\n",
            join_or_none(&report.path.blocked_phases)
        ));
    }
    if !report.path.remaining_phases.is_empty() {
        output.push_str(&format!(
            "- remaining: {}\n",
            join_or_none(&report.path.remaining_phases)
        ));
    }
    output.push('\n');
}

fn render_current_gate(output: &mut String, gate: &CurrentGate) {
    output.push_str("CURRENT GATE\n");
    output.push_str(&format!(
        "- phase: {}\n",
        gate.phase.as_deref().unwrap_or("none")
    ));
    if let Some(owner) = &gate.owner_skill {
        output.push_str(&format!("- owner_skill: {owner}\n"));
    }
    if let Some(route) = &gate.route_scope {
        output.push_str(&format!("- route_scope: {route}\n"));
    }
    if let Some(description) = &gate.description {
        output.push_str(&format!("- purpose: {description}\n"));
    }
    output.push_str(&format!(
        "- open_requirements: {}\n",
        join_or_none(&gate.open_requirements)
    ));

    output.push_str("\nDO NOW\n");
    write_bullets(output, &gate.do_now);
    output.push_str("\nDO NOT\n");
    write_bullets(output, &gate.do_not);
    output.push_str("\nNEXT COMMANDS\n");
    write_command_bullets(output, &gate.allowed_commands);
    if !gate.recommended_queries.is_empty() {
        output.push_str("\nLOAD MORE ONLY IF BLOCKED\n");
        write_command_bullets(output, &gate.recommended_queries);
    }
    output.push('\n');
}

fn render_end(output: &mut String, report: &GuideReport) {
    output.push_str("END\n");
    output.push_str("- done_when:\n");
    write_indented_bullets(output, &report.end.done_when);
    output.push_str(&format!(
        "- route_fulfillment_event: {}\n",
        report.end.route_fulfillment_event
    ));
    output.push_str(&format!(
        "- token_stats: `{}`\n",
        report.end.token_stats_command
    ));
    output.push_str(&format!(
        "- final_progress: `{}`\n",
        report.end.final_progress_command
    ));
    output.push_str(&format!("- align: `{}`\n", report.end.alignment_command));
    output.push_str(&format!(
        "- final_response_must_include: {}\n",
        join_or_none(&report.end.final_response_must_include)
    ));
    output.push('\n');
}

fn render_resume(output: &mut String, report: &GuideReport) {
    output.push_str("RESUME\n");
    output.push_str(&format!("- `{}`\n", report.resume.command));
    output.push_str(&format!("- guide_state: {}\n", report.resume.guide_state));
    output.push_str(&format!(
        "- guide_summary: {}\n\n",
        report.resume.guide_summary
    ));
}

fn render_warnings(output: &mut String, report: &GuideReport) {
    if report.warnings.is_empty() {
        return;
    }
    output.push_str("WARNINGS\n");
    for warning in &report.warnings {
        let kind = match warning.kind {
            GuideWarningKind::SpecChangedDecisionStable => "spec_changed_decision_stable",
            GuideWarningKind::SpecChangedNoPriorGuide => "spec_changed_no_prior_guide",
        };
        output.push_str(&format!("- {kind}: {}\n", warning.message));
    }
    output.push('\n');
}

fn render_full_detail(output: &mut String, report: &GuideReport) {
    output.push_str("FULL DETAIL\n");
    output.push_str(&format!("- input_sha256: {}\n", report.start.input_sha256));
    output.push_str(&format!(
        "- spec_fingerprint: {}\n",
        report.start.spec_fingerprint
    ));
    output.push_str(&format!(
        "- decision_fingerprint: {}\n",
        report.start.decision_fingerprint
    ));
    if !report.current_gate.allowed_now.is_empty() {
        output.push_str("- allowed_now:\n");
        write_bullets(output, &report.current_gate.allowed_now);
    }
    if !report.current_gate.progress_to_record.is_empty() {
        output.push_str("- progress_to_record:\n");
        for hint in &report.current_gate.progress_to_record {
            output.push_str(&format!("  - `{}`\n", hint.command));
        }
    }
}

fn write_bullets(output: &mut String, items: &[String]) {
    if items.is_empty() {
        output.push_str("- none\n");
        return;
    }
    for item in items {
        output.push_str(&format!("- {item}\n"));
    }
}

fn write_indented_bullets(output: &mut String, items: &[String]) {
    if items.is_empty() {
        output.push_str("  - none\n");
        return;
    }
    for item in items {
        output.push_str(&format!("  - {item}\n"));
    }
}

fn write_command_bullets(output: &mut String, items: &[String]) {
    if items.is_empty() {
        output.push_str("- none\n");
        return;
    }
    for item in items {
        output.push_str(&format!("- `{item}`\n"));
    }
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}
