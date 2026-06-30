use super::{escape_html, inline_code, markdown_text, pair, DoctorReport, WorkspaceIdentityReport};

pub(super) fn render_shape_contract(report: &DoctorReport) -> String {
    let mut output = String::new();
    output.push_str(&format!("Kind: {}\n", report.shape.kind));
    output.push_str(&format!(
        "Kind handling: {}\n",
        shape_kind_handling(&report.shape.kind)
    ));
    output.push_str(&format!("Analysis status: {}\n", report.analysis_status));
    output.push_str(&format!(
        "Skill files: {}\n",
        report.shape.skill_files.len()
    ));
    output.push_str(&format!("Packages: {}\n", shape_package_count(report)));
    output.push_str(&format!("Namespaces: {}\n", shape_namespace_count(report)));
    output.push_str(&format!(
        "Plugin roots: {}\n",
        report.shape.plugin_roots.len()
    ));
    if let Some(primary) = &report.shape.primary_skill {
        output.push_str(&format!("Primary skill: {primary}\n"));
    }
    if !report.shape.plugin_roots.is_empty() {
        output.push_str("Plugin root details:\n");
        for plugin in report.shape.plugin_roots.iter().take(8) {
            output.push_str(&format!(
                "- {} at {} ({} skill file(s))\n",
                plugin.namespace,
                plugin.path,
                plugin.skill_files.len()
            ));
        }
        if report.shape.plugin_roots.len() > 8 {
            output.push_str(&format!(
                "... {} more plugin root(s). Use --json for the full shape report.\n",
                report.shape.plugin_roots.len() - 8
            ));
        }
    }
    if !report.shape.referenced_skill_paths.is_empty() {
        output.push_str(&format!(
            "Referenced skill paths: {}\n",
            report.shape.referenced_skill_paths.join(", ")
        ));
    }
    output.push_str(&format!(
        "Next command: {}\n",
        report.shape.recommended_command
    ));
    output
}

pub(super) fn render_shape_contract_markdown(report: &DoctorReport) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "- **Kind:** {}\n",
        inline_code(&report.shape.kind)
    ));
    output.push_str(&format!(
        "- **Kind handling:** {}\n",
        markdown_text(shape_kind_handling(&report.shape.kind))
    ));
    output.push_str(&format!(
        "- **Analysis status:** {}\n",
        inline_code(&report.analysis_status)
    ));
    output.push_str(&format!(
        "- **Skill files:** `{}`\n",
        report.shape.skill_files.len()
    ));
    output.push_str(&format!(
        "- **Packages:** `{}`\n",
        shape_package_count(report)
    ));
    output.push_str(&format!(
        "- **Namespaces:** `{}`\n",
        shape_namespace_count(report)
    ));
    output.push_str(&format!(
        "- **Plugin roots:** `{}`\n",
        report.shape.plugin_roots.len()
    ));
    if let Some(primary) = &report.shape.primary_skill {
        output.push_str(&format!("- **Primary skill:** {}\n", inline_code(primary)));
    }
    if !report.shape.plugin_roots.is_empty() {
        output.push_str("\n### Plugin Root Details\n\n");
        for plugin in report.shape.plugin_roots.iter().take(8) {
            output.push_str(&format!(
                "- **{}** at {} with `{}` skill file(s)\n",
                markdown_text(&plugin.namespace),
                inline_code(&plugin.path),
                plugin.skill_files.len()
            ));
        }
        if report.shape.plugin_roots.len() > 8 {
            output.push_str(&format!(
                "- {} more plugin root(s) omitted. Use `--json` for the full shape report.\n",
                report.shape.plugin_roots.len() - 8
            ));
        }
    }
    if !report.shape.referenced_skill_paths.is_empty() {
        output.push_str("\n### Referenced Skill Paths\n\n");
        for path in &report.shape.referenced_skill_paths {
            output.push_str(&format!("- {}\n", inline_code(path)));
        }
    }
    output.push_str("\n**Next command**\n\n");
    output.push_str("```text\n");
    output.push_str(&report.shape.recommended_command);
    output.push_str("\n```\n");
    output
}

pub(super) fn render_shape_contract_html(report: &DoctorReport) -> String {
    let mut output = String::new();
    output.push_str("<section class=\"panel\"><h2>Shape Contract</h2>");
    output.push_str("<dl class=\"pairs compact\">");
    output.push_str(&pair("Kind", &report.shape.kind));
    output.push_str(&pair(
        "Kind handling",
        shape_kind_handling(&report.shape.kind),
    ));
    output.push_str(&pair("Analysis status", &report.analysis_status));
    output.push_str(&pair(
        "Skill files",
        &report.shape.skill_files.len().to_string(),
    ));
    output.push_str(&pair("Packages", &shape_package_count(report).to_string()));
    output.push_str(&pair(
        "Namespaces",
        &shape_namespace_count(report).to_string(),
    ));
    output.push_str(&pair(
        "Plugin roots",
        &report.shape.plugin_roots.len().to_string(),
    ));
    if let Some(primary) = &report.shape.primary_skill {
        output.push_str(&pair("Primary skill", primary));
    }
    output.push_str(&pair("Next command", &report.shape.recommended_command));
    output.push_str("</dl>");
    if !report.shape.plugin_roots.is_empty() {
        output.push_str("<h3>Plugin Root Details</h3><ul class=\"next\">");
        for plugin in report.shape.plugin_roots.iter().take(8) {
            output.push_str(&format!(
                "<li><strong>{}</strong> at <code>{}</code> with {} skill file(s)</li>",
                escape_html(&plugin.namespace),
                escape_html(&plugin.path),
                plugin.skill_files.len()
            ));
        }
        output.push_str("</ul>");
    }
    output.push_str("</section>\n");
    output
}

fn shape_package_count(report: &DoctorReport) -> usize {
    report
        .workspace_identity
        .as_ref()
        .map(|identity: &WorkspaceIdentityReport| identity.namespaced_package_count)
        .unwrap_or_else(|| {
            if !report.packages.is_empty() {
                report.packages.len()
            } else {
                report.shape.skill_files.len()
            }
        })
}

fn shape_namespace_count(report: &DoctorReport) -> usize {
    report
        .workspace_identity
        .as_ref()
        .map(|identity| identity.namespace_count)
        .unwrap_or_else(|| report.shape.plugin_roots.len())
}

fn shape_kind_handling(kind: &str) -> &'static str {
    match kind {
        "simple_skill" => {
            "one atomic package; use the single-skill import, review, compile, dry-run, and install path"
        }
        "entry_skill_with_subskills" => {
            "entry skill plus nested packages; map as a workspace and preserve cross-skill references"
        }
        "multi_skill_workspace" => {
            "multiple package identities; map as a workspace and preserve relative package paths"
        }
        "plugin_workspace" => {
            "plugin-shaped workspace; preserve plugin parent, namespace, shared files, and skills/ folder shape"
        }
        "non_skill_repository" => {
            "not a skill source; stop before import until a SKILL.md entrypoint exists"
        }
        _ => "unknown shape; inspect before importing or installing",
    }
}
