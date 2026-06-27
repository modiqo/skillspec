use super::types::RiskLevel;
use super::{
    ContractMitigationReport, DoctorIssue, DoctorPackageRiskReport, DoctorReport, SurfaceReport,
};

pub fn render(report: &DoctorReport) -> String {
    let mut output = String::new();

    output.push_str("SkillSpec Doctor\n");
    output.push_str("================\n\n");
    output.push_str(&format!("Target: {}\n", report.target));
    output.push_str(&format!(
        "Shape: {} - {}\n",
        report.shape.kind, report.shape.summary
    ));
    output.push_str(&format!("Status: {}\n", report.analysis_status));
    if let Some(staged_from) = &report.staged_from {
        output.push_str(&format!("Remote: {staged_from}\n"));
    }
    if let Some(primary) = &report.shape.primary_skill {
        output.push_str(&format!("Primary skill: {primary}\n"));
    }

    output.push_str("\nWhat This Measures\n");
    output.push_str("------------------\n");
    output.push_str(&format!("{}\n", report.score_model.plain_language_summary));
    output.push_str("This is a baseline of the current skill shape at doctor time. It is not a grade of domain knowledge, human usefulness, author effort, or legal/medical correctness.\n");
    output.push_str("Higher risk means a higher chance the agent skips, reorders, or improvises load-bearing instructions.\n");

    output.push_str("\nCurrent Skill Baseline\n");
    output.push_str("----------------------\n");
    output.push_str(&format!("Verdict: {}\n", report.verdict));
    output.push_str(&format!(
        "Follow-through readiness: {}\n",
        humanize_snake(&report.score_model.readiness_label)
    ));
    if let Some((label, score, level)) = primary_risk(report) {
        output.push_str(&format!(
            "Agent follow-through risk: {} ({}/100, {})\n",
            level.as_str(),
            score,
            label
        ));
    } else {
        output.push_str(&format!(
            "Agent follow-through risk: not evaluated ({})\n",
            report.analysis_status
        ));
    }
    if let Some(frontmatter) = &report.frontmatter_discovery_risk {
        output.push_str(&format!(
            "Discovery risk: {} ({}/100)\n",
            frontmatter.level.as_str(),
            frontmatter.score
        ));
    }
    if let Some(mitigation) = &report.contract_mitigation {
        output.push_str(&render_contract_line(mitigation));
    }
    if let Some(risk) = &report.raw_activation_risk {
        output.push_str(&format!(
            "Raw activation load risk: {} ({}/100 before contract mitigation)\n",
            risk.level.as_str(),
            risk.score
        ));
    }

    output.push_str("\nSurface\n");
    output.push_str("-------\n");
    output.push_str(&render_surface(
        &report.surface,
        report.large_surface_percentage,
    ));
    output.push_str(&format!(
        "Instruction signals: {} modal obligation(s), {} late obligation(s), {} numbered step(s), {} dependency mention(s)\n",
        report.counts.modal_obligations,
        report.counts.late_modal_obligations,
        report.counts.numbered_steps,
        report.counts.dependency_mentions
    ));
    output.push_str(&format!(
        "Code in active skill: {} block(s), {} unlabeled\n",
        report.counts.code_blocks_in_skill, report.counts.unlabeled_code_blocks_in_skill
    ));

    if !report.shape.plugin_roots.is_empty()
        || !report.shape.referenced_skill_paths.is_empty()
        || !report.shape.negative_signals.is_empty()
    {
        output.push_str("\nShape Details\n");
        output.push_str("-------------\n");
        if !report.shape.plugin_roots.is_empty() {
            output.push_str("Plugin roots:\n");
            for plugin in &report.shape.plugin_roots {
                output.push_str(&format!(
                    "- {} at {} ({} skill file(s))\n",
                    plugin.namespace,
                    plugin.path,
                    plugin.skill_files.len()
                ));
            }
        }
        if !report.shape.referenced_skill_paths.is_empty() {
            output.push_str("Referenced skills:\n");
            for path in &report.shape.referenced_skill_paths {
                output.push_str(&format!("- {path}\n"));
            }
        }
        if !report.shape.negative_signals.is_empty() {
            output.push_str("Negative signals:\n");
            for signal in &report.shape.negative_signals {
                output.push_str(&format!("- {signal}\n"));
            }
        }
    }

    if !report.packages.is_empty() {
        output.push_str("\nPackages\n");
        output.push_str("--------\n");
        for package in report.packages.iter().take(12) {
            output.push_str(&render_package_line(package));
        }
        if report.packages.len() > 12 {
            output.push_str(&format!(
                "... {} more package(s). Use --json for the full list.\n",
                report.packages.len() - 12
            ));
        }
    }

    output.push_str("\nFindings\n");
    output.push_str("--------\n");
    if report.issues.is_empty() {
        output.push_str("No static structure issues detected.\n");
    } else {
        for (index, issue) in report.issues.iter().take(8).enumerate() {
            output.push_str(&render_issue_text(index + 1, issue));
        }
        if report.issues.len() > 8 {
            output.push_str(&format!(
                "... {} more finding(s). Use --json or --html for the full report.\n",
                report.issues.len() - 8
            ));
        }
    }

    output.push_str("\nNext Actions\n");
    output.push_str("------------\n");
    output.push_str(&format!(
        "Recommended next action: {}\n",
        report.shape.recommended_command
    ));
    for step in &report.suggested_next_steps {
        output.push_str(&format!("- {step}\n"));
    }

    output.push_str("\nBasis\n");
    output.push_str("-----\n");
    output.push_str(&format!(
        "{} cited basis item(s) are attached to this report.\n",
        report.basis.len()
    ));
    for basis in report.basis.iter().take(6) {
        output.push_str(&format!(
            "- {}: {} ({})\n",
            basis.id, basis.claim, basis.source
        ));
    }
    if report.basis.len() > 6 {
        output.push_str(&format!(
            "... {} more basis item(s). Use --json for the full basis registry.\n",
            report.basis.len() - 6
        ));
    }
    output.push_str("Use --html for a shareable review page or --json for machine evidence.\n");

    trim_trailing_newline(&mut output);
    output
}

pub fn render_markdown(report: &DoctorReport) -> String {
    let mut output = String::new();

    output.push_str("# SkillSpec Doctor report\n\n");
    output.push_str(&format!(
        "**Target:** {}\n\n",
        markdown_location(&report.target)
    ));
    output.push_str(&format!(
        "**Shape:** {} - {}\n\n",
        inline_code(&report.shape.kind),
        markdown_text(&report.shape.summary)
    ));
    output.push_str(&format!(
        "**Status:** {}\n\n",
        inline_code(&report.analysis_status)
    ));
    if let Some(staged_from) = &report.staged_from {
        output.push_str(&format!(
            "**Remote:** {}\n\n",
            markdown_location(staged_from)
        ));
    }
    if let Some(primary) = &report.shape.primary_skill {
        output.push_str(&format!("**Primary skill:** {}\n\n", inline_code(primary)));
    }

    output.push_str("## What This Measures\n\n");
    output.push_str(&format!(
        "{}\n\n",
        markdown_text(&report.score_model.plain_language_summary)
    ));
    output.push_str("This is a baseline of the current skill shape at doctor time. It is not a grade of domain knowledge, human usefulness, author effort, or legal/medical correctness.\n\n");
    output.push_str("Higher risk means a higher chance the agent skips, reorders, or improvises load-bearing instructions.\n\n");

    output.push_str("## Current Skill Baseline\n\n");
    output.push_str(&format!(
        "- **Verdict:** {}\n",
        markdown_text(&report.verdict)
    ));
    output.push_str(&format!(
        "- **Follow-through readiness:** **{}**\n",
        markdown_text(&humanize_snake(&report.score_model.readiness_label))
    ));
    if let Some((label, score, level)) = primary_risk(report) {
        output.push_str(&format!(
            "- **Agent follow-through risk:** **{}** (`{}/100`, {})\n",
            level.as_str(),
            score,
            markdown_text(label)
        ));
    } else {
        output.push_str(&format!(
            "- **Agent follow-through risk:** not evaluated (`{}`)\n",
            markdown_text(&report.analysis_status)
        ));
    }
    if let Some(frontmatter) = &report.frontmatter_discovery_risk {
        output.push_str(&format!(
            "- **Discovery risk:** **{}** (`{}/100`)\n",
            frontmatter.level.as_str(),
            frontmatter.score
        ));
    }
    if let Some(mitigation) = &report.contract_mitigation {
        output.push_str(&format!(
            "- **Contract mitigation:** **{}**; residual risk is **{}** (`{}/100`)\n",
            mitigation.level.as_str(),
            mitigation.residual_risk_level.as_str(),
            mitigation.residual_risk_score
        ));
        output.push_str(&format!(
            "- **Contract surface:** `{}` route(s), `{}` rule(s), `{}` command(s), `{}` dependency item(s), `{}` test(s)\n",
            mitigation.routes,
            mitigation.rules,
            mitigation.commands,
            mitigation.dependencies,
            mitigation.tests
        ));
    }
    if let Some(risk) = &report.raw_activation_risk {
        output.push_str(&format!(
            "- **Raw activation load risk:** **{}** (`{}/100` before contract mitigation)\n",
            risk.level.as_str(),
            risk.score
        ));
    }

    output.push_str("\n## Surface\n\n");
    output.push_str(&format!(
        "- **Activation body:** `{}` line(s), `{}` byte(s), approximately `{}` token(s)\n",
        report.surface.activation_lines,
        report.surface.activation_bytes,
        report.surface.activation_estimated_tokens
    ));
    output.push_str(&format!(
        "- **Deferred resources:** `{}` file(s), `{}` byte(s)\n",
        report.surface.deferred_files, report.surface.deferred_bytes
    ));
    output.push_str(&format!(
        "- **Frontmatter:** `{}` line(s), `{}` byte(s)\n",
        report.surface.frontmatter_lines, report.surface.frontmatter_bytes
    ));
    output.push_str(&format!(
        "- **Activation-loaded surface:** `{}%`\n",
        report.large_surface_percentage
    ));
    output.push_str(&format!(
        "- **Unmapped package files:** `{}`\n",
        report.surface.unmapped_files
    ));
    output.push_str(&format!(
        "- **Instruction signals:** `{}` modal obligation(s), `{}` late obligation(s), `{}` numbered step(s), `{}` dependency mention(s)\n",
        report.counts.modal_obligations,
        report.counts.late_modal_obligations,
        report.counts.numbered_steps,
        report.counts.dependency_mentions
    ));
    output.push_str(&format!(
        "- **Code in active skill:** `{}` block(s), `{}` unlabeled\n",
        report.counts.code_blocks_in_skill, report.counts.unlabeled_code_blocks_in_skill
    ));

    if !report.shape.plugin_roots.is_empty()
        || !report.shape.referenced_skill_paths.is_empty()
        || !report.shape.negative_signals.is_empty()
    {
        output.push_str("\n## Shape Details\n\n");
        if !report.shape.plugin_roots.is_empty() {
            output.push_str("### Plugin Roots\n\n");
            for plugin in &report.shape.plugin_roots {
                output.push_str(&format!(
                    "- **{}** at {} with `{}` skill file(s)\n",
                    markdown_text(&plugin.namespace),
                    inline_code(&plugin.path),
                    plugin.skill_files.len()
                ));
            }
            output.push('\n');
        }
        if !report.shape.referenced_skill_paths.is_empty() {
            output.push_str("### Referenced Skills\n\n");
            for path in &report.shape.referenced_skill_paths {
                output.push_str(&format!("- {}\n", inline_code(path)));
            }
            output.push('\n');
        }
        if !report.shape.negative_signals.is_empty() {
            output.push_str("### Negative Signals\n\n");
            for signal in &report.shape.negative_signals {
                output.push_str(&format!("- {}\n", markdown_text(signal)));
            }
        }
    }

    if !report.packages.is_empty() {
        output.push_str("\n## Packages\n\n");
        for package in report.packages.iter().take(12) {
            output.push_str(&render_package_markdown(package));
        }
        if report.packages.len() > 12 {
            output.push_str(&format!(
                "\n{} more package(s) omitted. Use `--json` for the full list.\n",
                report.packages.len() - 12
            ));
        }
    }

    output.push_str("\n## Findings\n\n");
    if report.issues.is_empty() {
        output.push_str("No static structure issues detected.\n");
    } else {
        for (index, issue) in report.issues.iter().take(8).enumerate() {
            output.push_str(&render_issue_markdown(index + 1, issue));
        }
        if report.issues.len() > 8 {
            output.push_str(&format!(
                "\n{} more finding(s) omitted. Use `--json` or `--html` for the full report.\n",
                report.issues.len() - 8
            ));
        }
    }

    output.push_str("\n## Next Actions\n\n");
    output.push_str("**Recommended next action**\n\n");
    output.push_str("```text\n");
    output.push_str(&report.shape.recommended_command);
    output.push_str("\n```\n\n");
    for step in &report.suggested_next_steps {
        output.push_str(&format!("- {}\n", markdown_text_preserve_code(step)));
    }

    output.push_str("\n## Research Basis\n\n");
    output.push_str(&format!(
        "{} cited basis item(s) are attached to this report.\n\n",
        report.basis.len()
    ));
    for basis in report.basis.iter().take(6) {
        output.push_str(&format!(
            "- **{}** - {} ([{}]({}))\n",
            markdown_text(&basis.id),
            markdown_text(&basis.claim),
            markdown_text(&basis.citation),
            markdown_link_url(&basis_href(&basis.source))
        ));
    }
    if report.basis.len() > 6 {
        output.push_str(&format!(
            "- {} more basis item(s) omitted. Use `--json` for the full basis registry.\n",
            report.basis.len() - 6
        ));
    }
    output
        .push_str("\nUse `--html` for a shareable review page or `--json` for machine evidence.\n");

    trim_trailing_newline(&mut output);
    output
}

pub fn render_html(report: &DoctorReport) -> String {
    let mut output = String::new();
    let (risk_label, risk_score, risk_level) = primary_risk(report)
        .map(|(label, score, level)| (label.to_owned(), score, level))
        .unwrap_or_else(|| ("not evaluated".to_owned(), 0, RiskLevel::Low));
    let readiness_label = humanize_snake(&report.score_model.readiness_label);

    output.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    output.push_str("<meta charset=\"utf-8\">\n");
    output.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    output.push_str("<title>SkillSpec Doctor Report</title>\n");
    output.push_str(STYLE);
    output.push_str("</head>\n<body>\n<main>\n");

    output.push_str("<section class=\"hero\">\n");
    output.push_str("<p class=\"eyebrow\">SkillSpec Doctor</p>\n");
    output.push_str(&format!(
        "<h1>{}</h1>\n",
        escape_html(&report.shape.kind.replace('_', " "))
    ));
    output.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape_html(&report.shape.summary)
    ));
    output.push_str("<div class=\"hero-meta\">\n");
    output.push_str(&format!(
        "<span>Target <strong>{}</strong></span>",
        escape_html(&report.target)
    ));
    output.push_str(&format!(
        "<span>Status <strong>{}</strong></span>",
        escape_html(&report.analysis_status)
    ));
    if let Some(primary) = &report.shape.primary_skill {
        output.push_str(&format!(
            "<span>Primary <strong>{}</strong></span>",
            escape_html(primary)
        ));
    }
    output.push_str("</div>\n</section>\n");

    output.push_str("<section class=\"cards\">\n");
    output.push_str(&metric_card(
        "Follow-through readiness",
        &readiness_label,
        "current skill baseline",
        "neutral",
    ));
    output.push_str(&metric_card(
        "Agent follow-through risk",
        &format!("{} / 100", risk_score),
        &format!("{} risk, {}", risk_level.as_str(), risk_label),
        risk_level.as_str(),
    ));
    if let Some(frontmatter) = &report.frontmatter_discovery_risk {
        output.push_str(&metric_card(
            "Discovery risk",
            &format!("{} / 100", frontmatter.score),
            frontmatter.level.as_str(),
            frontmatter.level.as_str(),
        ));
    }
    output.push_str(&metric_card(
        "Activation load",
        &format!("~{} tokens", report.surface.activation_estimated_tokens),
        &format!(
            "{} lines, {}% activation-loaded",
            report.surface.activation_lines, report.large_surface_percentage
        ),
        "neutral",
    ));
    output.push_str("</section>\n");

    output.push_str("<section class=\"panel explainer\"><h2>What This Measures</h2>");
    output.push_str(&format!(
        "<p>{}</p>",
        escape_html(&report.score_model.plain_language_summary)
    ));
    output.push_str("<p class=\"muted\">This is a baseline of the current skill shape at doctor time. It is not a grade of domain knowledge, human usefulness, author effort, or legal/medical correctness. Higher risk means a higher chance the agent skips, reorders, or improvises load-bearing instructions.</p>");
    output.push_str("</section>\n");

    output.push_str("<section class=\"panel-grid\">\n");
    output.push_str("<article class=\"panel\"><h2>Surface</h2>");
    output.push_str("<dl class=\"pairs\">");
    output.push_str(&pair(
        "Frontmatter",
        &format!("{} lines", report.surface.frontmatter_lines),
    ));
    output.push_str(&pair(
        "Activation",
        &format!(
            "{} lines, {} bytes",
            report.surface.activation_lines, report.surface.activation_bytes
        ),
    ));
    output.push_str(&pair(
        "Deferred",
        &format!(
            "{} files, {} bytes",
            report.surface.deferred_files, report.surface.deferred_bytes
        ),
    ));
    output.push_str(&pair(
        "Unmapped files",
        &report.surface.unmapped_files.to_string(),
    ));
    output.push_str("</dl></article>\n");

    output.push_str("<article class=\"panel\"><h2>Instruction Signals</h2>");
    output.push_str("<dl class=\"pairs\">");
    output.push_str(&pair(
        "Modal obligations",
        &format!(
            "{} total, {} late",
            report.counts.modal_obligations, report.counts.late_modal_obligations
        ),
    ));
    output.push_str(&pair(
        "Numbered steps",
        &report.counts.numbered_steps.to_string(),
    ));
    output.push_str(&pair(
        "Code blocks",
        &format!(
            "{} total, {} unlabeled",
            report.counts.code_blocks_in_skill, report.counts.unlabeled_code_blocks_in_skill
        ),
    ));
    output.push_str(&pair(
        "Dependency mentions",
        &report.counts.dependency_mentions.to_string(),
    ));
    output.push_str("</dl></article>\n");
    output.push_str("</section>\n");

    if let Some(mitigation) = &report.contract_mitigation {
        output.push_str("<section class=\"panel\"><h2>Contract Mitigation</h2>");
        output.push_str(&format!(
            "<p class=\"callout\">{} mitigation. Residual risk is {} ({}/100).</p>",
            escape_html(mitigation.level.as_str()),
            escape_html(mitigation.residual_risk_level.as_str()),
            mitigation.residual_risk_score
        ));
        output.push_str("<dl class=\"pairs compact\">");
        output.push_str(&pair("Routes", &mitigation.routes.to_string()));
        output.push_str(&pair("Rules", &mitigation.rules.to_string()));
        output.push_str(&pair("Commands", &mitigation.commands.to_string()));
        output.push_str(&pair("Dependencies", &mitigation.dependencies.to_string()));
        output.push_str(&pair("Tests", &mitigation.tests.to_string()));
        output.push_str("</dl></section>\n");
    }

    if !report.packages.is_empty() {
        output.push_str("<section class=\"panel\"><h2>Packages</h2>");
        output.push_str("<div class=\"table-wrap\"><table><thead><tr><th>Package</th><th>Path</th><th>Role</th><th>Drift</th><th>Discovery</th></tr></thead><tbody>");
        for package in report.packages.iter().take(32) {
            output.push_str("<tr>");
            output.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td><span class=\"badge {}\">{}</span></td><td><span class=\"badge {}\">{}</span></td>",
                escape_html(&package.package_id),
                escape_html(&package.path),
                escape_html(&package.shape_role),
                escape_html(package.agent_drift_risk.level.as_str()),
                escape_html(package.agent_drift_risk.level.as_str()),
                escape_html(package.frontmatter_discovery_risk.level.as_str()),
                escape_html(package.frontmatter_discovery_risk.level.as_str())
            ));
            output.push_str("</tr>");
        }
        output.push_str("</tbody></table></div>");
        if report.packages.len() > 32 {
            output.push_str(&format!(
                "<p class=\"muted\">{} more package(s) omitted from this HTML preview. Use JSON for the full list.</p>",
                report.packages.len() - 32
            ));
        }
        output.push_str("</section>\n");
    }

    output.push_str("<section class=\"panel\"><h2>Findings</h2>");
    if report.issues.is_empty() {
        output.push_str("<p class=\"callout\">No static structure issues detected.</p>");
    } else {
        output.push_str("<div class=\"findings\">");
        for issue in &report.issues {
            output.push_str(&render_issue_html(issue));
        }
        output.push_str("</div>");
    }
    output.push_str("</section>\n");

    output.push_str("<section class=\"panel\"><h2>Next Actions</h2>");
    output.push_str(&format!(
        "<p class=\"command\">{}</p>",
        escape_html(&report.shape.recommended_command)
    ));
    output.push_str("<ol class=\"next\">");
    for step in &report.suggested_next_steps {
        output.push_str(&format!("<li>{}</li>", escape_html(step)));
    }
    output.push_str("</ol></section>\n");

    output.push_str("<section class=\"panel\"><h2>Research Basis</h2>");
    output.push_str("<div class=\"basis-grid\">");
    for basis in &report.basis {
        output.push_str("<article class=\"basis\">");
        output.push_str(&format!("<h3>{}</h3>", escape_html(&basis.id)));
        output.push_str(&format!("<p>{}</p>", escape_html(&basis.claim)));
        output.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            escape_html(&basis_href(&basis.source)),
            escape_html(&basis.citation)
        ));
        output.push_str("</article>");
    }
    output.push_str("</div></section>\n");

    output.push_str("</main>\n</body>\n</html>\n");
    output
}

fn render_surface(surface: &SurfaceReport, large_surface_percentage: u8) -> String {
    format!(
        "Activation body: {} line(s), {} byte(s), ~{} token(s)\nDeferred resources: {} file(s), {} byte(s)\nFrontmatter: {} line(s), {} byte(s)\nActivation-loaded surface: {}%\nUnmapped package files: {}\n",
        surface.activation_lines,
        surface.activation_bytes,
        surface.activation_estimated_tokens,
        surface.deferred_files,
        surface.deferred_bytes,
        surface.frontmatter_lines,
        surface.frontmatter_bytes,
        large_surface_percentage,
        surface.unmapped_files
    )
}

fn render_contract_line(mitigation: &ContractMitigationReport) -> String {
    format!(
        "Contract mitigation: {} (routes {}, rules {}, commands {}, dependencies {}, tests {})\nResidual risk: {} ({}/100)\n",
        mitigation.level.as_str(),
        mitigation.routes,
        mitigation.rules,
        mitigation.commands,
        mitigation.dependencies,
        mitigation.tests,
        mitigation.residual_risk_level.as_str(),
        mitigation.residual_risk_score
    )
}

fn render_package_line(package: &DoctorPackageRiskReport) -> String {
    format!(
        "- {}: {} | role={} | drift={} | discovery={}\n",
        package.package_id,
        package.path,
        package.shape_role,
        package.agent_drift_risk.level.as_str(),
        package.frontmatter_discovery_risk.level.as_str()
    )
}

fn render_issue_text(index: usize, issue: &DoctorIssue) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "{}. [{}] {}\n",
        index,
        issue.severity.to_uppercase(),
        issue.title
    ));
    if let Some(location) = &issue.location {
        output.push_str(&format!("   Location: {location}\n"));
    }
    output.push_str(&format!("   Evidence: {}\n", issue.evidence));
    if !issue.basis.is_empty() {
        output.push_str(&format!("   Basis: {}\n", issue.basis.join(", ")));
    }
    output.push_str(&format!("   Fix: {}\n", issue.remediation));
    output
}

fn render_issue_html(issue: &DoctorIssue) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "<article class=\"finding\"><div><span class=\"badge {}\">{}</span><h3>{}</h3></div>",
        escape_html(&issue.severity),
        escape_html(&issue.severity),
        escape_html(&issue.title)
    ));
    if let Some(location) = &issue.location {
        output.push_str(&format!(
            "<p class=\"location\">{}</p>",
            escape_html(location)
        ));
    }
    output.push_str(&format!(
        "<p><strong>Evidence:</strong> {}</p>",
        escape_html(&issue.evidence)
    ));
    if !issue.basis.is_empty() {
        output.push_str(&format!(
            "<p><strong>Basis:</strong> {}</p>",
            escape_html(&issue.basis.join(", "))
        ));
    }
    output.push_str(&format!(
        "<p><strong>Fix:</strong> {}</p></article>",
        escape_html(&issue.remediation)
    ));
    output
}

fn render_package_markdown(package: &DoctorPackageRiskReport) -> String {
    let mut output = String::new();
    output.push_str(&format!("### {}\n\n", markdown_text(&package.package_id)));
    output.push_str(&format!("- **Path:** {}\n", inline_code(&package.path)));
    output.push_str(&format!(
        "- **Role:** {}\n",
        markdown_text(&package.shape_role)
    ));
    output.push_str(&format!(
        "- **Drift risk:** **{}**\n",
        package.agent_drift_risk.level.as_str()
    ));
    output.push_str(&format!(
        "- **Discovery risk:** **{}**\n\n",
        package.frontmatter_discovery_risk.level.as_str()
    ));
    output
}

fn render_issue_markdown(index: usize, issue: &DoctorIssue) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "### {}. {}: {}\n\n",
        index,
        issue.severity.to_uppercase(),
        markdown_text(&issue.title)
    ));
    if let Some(location) = &issue.location {
        output.push_str(&format!(
            "**Location:** {}\n\n",
            markdown_location(location)
        ));
    }
    output.push_str(&format!(
        "**Evidence:** {}\n\n",
        markdown_text(&issue.evidence)
    ));
    if !issue.basis.is_empty() {
        let basis = issue
            .basis
            .iter()
            .map(|id| inline_code(id))
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("**Basis:** {basis}\n\n"));
    }
    output.push_str(&format!(
        "**Fix:** {}\n\n",
        markdown_text(&issue.remediation)
    ));
    output
}

fn markdown_location(value: &str) -> String {
    if value.starts_with("http://") || value.starts_with("https://") {
        format!("[{}]({})", markdown_text(value), markdown_link_url(value))
    } else {
        inline_code(value)
    }
}

fn markdown_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn markdown_text_preserve_code(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_code = false;
    for ch in value.chars() {
        if ch == '`' {
            in_code = !in_code;
            output.push(ch);
        } else if in_code {
            output.push(ch);
        } else {
            match ch {
                '&' => output.push_str("&amp;"),
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                _ => output.push(ch),
            }
        }
    }
    output
}

fn markdown_link_url(value: &str) -> String {
    value
        .replace(' ', "%20")
        .replace(')', "%29")
        .replace('(', "%28")
}

fn inline_code(value: &str) -> String {
    let max_tick_run = value.split(|ch| ch != '`').map(str::len).max().unwrap_or(0);
    let ticks = "`".repeat(max_tick_run + 1);
    if max_tick_run == 0 {
        format!("{ticks}{value}{ticks}")
    } else {
        format!("{ticks} {value} {ticks}")
    }
}

fn humanize_snake(value: &str) -> String {
    value.replace('_', " ")
}

fn metric_card(label: &str, value: &str, detail: &str, level: &str) -> String {
    format!(
        "<article class=\"card\"><p>{}</p><strong>{}</strong><span class=\"badge {}\">{}</span></article>",
        escape_html(label),
        escape_html(value),
        escape_html(level),
        escape_html(detail)
    )
}

fn pair(label: &str, value: &str) -> String {
    format!(
        "<dt>{}</dt><dd>{}</dd>",
        escape_html(label),
        escape_html(value)
    )
}

fn primary_risk(report: &DoctorReport) -> Option<(&'static str, u8, RiskLevel)> {
    if let Some(mitigation) = &report.contract_mitigation {
        return Some((
            "residual after contract mitigation",
            mitigation.residual_risk_score,
            mitigation.residual_risk_level,
        ));
    }
    if let Some(risk) = &report.agent_drift_risk {
        return Some(("single-skill drift", risk.score, risk.level));
    }
    if let Some(risk) = &report.workspace_agent_drift_risk {
        return Some(("workspace aggregate", risk.score, risk.level));
    }
    None
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn basis_href(source: &str) -> String {
    let source = source.trim();
    if source.starts_with("http://") || source.starts_with("https://") {
        return source.to_owned();
    }

    let path = source.split_whitespace().next().unwrap_or(source);
    if path.starts_with("docs/") {
        return format!("https://github.com/modiqo/skillspec/blob/main/{path}");
    }
    if path.starts_with("spec/") {
        return format!("https://github.com/modiqo/skillspec/blob/main/{path}");
    }
    source.to_owned()
}

fn trim_trailing_newline(output: &mut String) {
    while output.ends_with('\n') {
        output.pop();
    }
}

const STYLE: &str = r#"<style>
:root {
  color-scheme: light;
  --ink: #14161a;
  --muted: #636a74;
  --line: #dfe3e8;
  --panel: #ffffff;
  --canvas: #f6f7f9;
  --accent: #166d68;
  --low: #157f4f;
  --medium: #9a6500;
  --high: #b23b21;
  --critical: #8a1538;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background: var(--canvas);
  color: var(--ink);
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  line-height: 1.5;
}
main {
  width: min(1120px, calc(100vw - 40px));
  margin: 0 auto;
  padding: 48px 0 72px;
}
.hero {
  border-bottom: 1px solid var(--line);
  padding-bottom: 28px;
  margin-bottom: 24px;
}
.eyebrow {
  margin: 0 0 10px;
  color: var(--accent);
  font-size: 13px;
  font-weight: 760;
  letter-spacing: .08em;
  text-transform: uppercase;
}
h1 {
  margin: 0;
  font-size: clamp(36px, 6vw, 72px);
  line-height: .95;
  letter-spacing: 0;
  text-transform: capitalize;
}
h2 {
  margin: 0 0 18px;
  font-size: 20px;
  letter-spacing: 0;
}
h3 {
  margin: 8px 0 8px;
  font-size: 16px;
  letter-spacing: 0;
}
.summary {
  max-width: 780px;
  margin: 18px 0 0;
  color: var(--muted);
  font-size: 19px;
}
.hero-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 22px;
}
.hero-meta span,
.badge {
  border: 1px solid var(--line);
  border-radius: 999px;
  background: #fff;
  color: var(--muted);
  padding: 6px 10px;
  font-size: 13px;
}
.hero-meta strong { color: var(--ink); font-weight: 680; }
.cards {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 14px;
  margin-bottom: 14px;
}
.card,
.panel {
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 8px;
  box-shadow: 0 1px 2px rgba(20, 22, 26, .04);
}
.card {
  min-height: 150px;
  padding: 18px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}
.card p {
  margin: 0;
  color: var(--muted);
  font-size: 14px;
}
.card strong {
  display: block;
  margin: 18px 0;
  font-size: 30px;
  line-height: 1;
}
.panel {
  padding: 22px;
  margin-top: 14px;
}
.panel-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}
.pairs {
  display: grid;
  grid-template-columns: minmax(130px, 220px) 1fr;
  gap: 10px 18px;
  margin: 0;
}
.pairs.compact { grid-template-columns: repeat(5, minmax(0, 1fr)); }
.pairs dt {
  color: var(--muted);
  font-size: 13px;
}
.pairs dd {
  margin: 0;
  font-weight: 650;
}
.callout {
  margin: 0 0 16px;
  padding: 14px 16px;
  border-left: 4px solid var(--accent);
  background: #eef7f5;
}
.table-wrap { overflow-x: auto; }
table {
  width: 100%;
  border-collapse: collapse;
}
th, td {
  border-bottom: 1px solid var(--line);
  padding: 11px 8px;
  text-align: left;
  vertical-align: top;
}
th {
  color: var(--muted);
  font-size: 12px;
  text-transform: uppercase;
}
.findings {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
.finding {
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 16px;
  background: #fcfcfd;
}
.finding p { margin: 10px 0 0; }
.location,
.muted {
  color: var(--muted);
  font-size: 13px;
}
.command {
  margin: 0 0 16px;
  padding: 14px 16px;
  border-radius: 8px;
  background: #101317;
  color: #f4f7fb;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  overflow-x: auto;
}
.next { margin: 0; padding-left: 22px; }
.next li + li { margin-top: 8px; }
.basis-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
.basis {
  border-top: 1px solid var(--line);
  padding-top: 12px;
}
.basis p { margin: 0 0 8px; color: var(--muted); }
.basis a { color: var(--accent); overflow-wrap: anywhere; }
.badge.low { border-color: rgba(21, 127, 79, .25); color: var(--low); background: #eef9f3; }
.badge.medium { border-color: rgba(154, 101, 0, .25); color: var(--medium); background: #fff6df; }
.badge.high { border-color: rgba(178, 59, 33, .25); color: var(--high); background: #fff0eb; }
.badge.critical { border-color: rgba(138, 21, 56, .25); color: var(--critical); background: #ffedf3; }
.badge.neutral { border-color: var(--line); color: var(--muted); background: #fff; }
@media (max-width: 860px) {
  main { width: min(100vw - 28px, 1120px); padding-top: 28px; }
  .cards,
  .panel-grid,
  .findings,
  .basis-grid {
    grid-template-columns: 1fr;
  }
  .pairs,
  .pairs.compact {
    grid-template-columns: 1fr;
  }
}
</style>
"#;
