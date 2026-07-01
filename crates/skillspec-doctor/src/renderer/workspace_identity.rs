use super::{escape_html, inline_code, markdown_text, pair, WorkspaceIdentityReport};

pub(super) fn render_workspace_identity(identity: &WorkspaceIdentityReport) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "Source files: {} total\n",
        identity.source_file_count
    ));
    output.push_str(&format!(
        "Namespaced packages: {} package(s) across {} namespace(s)\n",
        identity.namespaced_package_count, identity.namespace_count
    ));
    output.push_str(&format!(
        "Skill content: {} SKILL.md file(s), {} unique byte content item(s), {} repeated occurrence(s) in {} referentiable group(s)\n",
        identity.skill_file_count,
        identity.unique_skill_content_count,
        identity.repeated_skill_content_occurrences,
        identity.repeated_skill_content_groups
    ));
    output.push_str(&format!(
        "Estimated source tokens: ~{} total if copied byte-for-byte, ~{} unique if referenced, ~{} repeated\n",
        identity.total_skill_content_estimated_tokens,
        identity.unique_skill_content_estimated_tokens,
        identity.repeated_skill_content_estimated_tokens
    ));
    output.push_str(&format!(
        "Frontmatter names: {} repeated name group(s), {} repeated occurrence(s)\n",
        identity.same_frontmatter_name_groups, identity.same_frontmatter_name_occurrences
    ));

    if !identity.namespaces.is_empty() {
        output.push_str("Namespaces:\n");
        for namespace in identity.namespaces.iter().take(8) {
            output.push_str(&format!(
                "- {}: {} skill file(s)",
                namespace.namespace, namespace.skill_file_count
            ));
            if !namespace.sample_paths.is_empty() {
                output.push_str(&format!("; samples {}", namespace.sample_paths.join(", ")));
            }
            output.push('\n');
        }
        if identity.namespaces.len() > 8 {
            output.push_str(&format!(
                "... {} more namespace(s). Use --json for the full identity report.\n",
                identity.namespaces.len() - 8
            ));
        }
    }

    let repeated_refs = identity
        .source_content_refs
        .iter()
        .filter(|item| item.occurrence_count > 1)
        .collect::<Vec<_>>();
    if !repeated_refs.is_empty() {
        output.push_str("Referentiable repeated content:\n");
        for item in repeated_refs.iter().take(5) {
            output.push_str(&format!(
                "- {}: canonical {}; {} alias(es), ~{} repeated token(s)\n",
                item.sha256.chars().take(12).collect::<String>(),
                item.canonical_path,
                item.aliases.len(),
                item.repeated_estimated_tokens
            ));
        }
        if repeated_refs.len() > 5 {
            output.push_str(&format!(
                "... {} more repeated content group(s). Use --json for all source_content_refs.\n",
                repeated_refs.len() - 5
            ));
        }
    }

    if !identity.frontmatter_name_refs.is_empty() {
        output.push_str("Repeated frontmatter names:\n");
        for item in identity.frontmatter_name_refs.iter().take(5) {
            output.push_str(&format!(
                "- {}: {} occurrence(s) across {}\n",
                item.public_name,
                item.occurrence_count,
                item.paths.join(", ")
            ));
        }
        if identity.frontmatter_name_refs.len() > 5 {
            output.push_str(&format!(
                "... {} more repeated frontmatter name group(s). Use --json for all frontmatter_name_refs.\n",
                identity.frontmatter_name_refs.len() - 5
            ));
        }
    }

    output.push_str(&format!("Recommendation: {}\n", identity.recommendation));
    output
}

pub(super) fn render_workspace_identity_markdown(identity: &WorkspaceIdentityReport) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "- **Source files:** `{}` total\n",
        identity.source_file_count
    ));
    output.push_str(&format!(
        "- **Namespaced packages:** `{}` package(s) across `{}` namespace(s)\n",
        identity.namespaced_package_count, identity.namespace_count
    ));
    output.push_str(&format!(
        "- **Skill content:** `{}` `SKILL.md` file(s), `{}` unique byte content item(s), `{}` repeated occurrence(s) in `{}` referentiable group(s)\n",
        identity.skill_file_count,
        identity.unique_skill_content_count,
        identity.repeated_skill_content_occurrences,
        identity.repeated_skill_content_groups
    ));
    output.push_str(&format!(
        "- **Estimated source tokens:** approximately `{}` total if copied byte-for-byte, `{}` unique if referenced, `{}` repeated\n",
        identity.total_skill_content_estimated_tokens,
        identity.unique_skill_content_estimated_tokens,
        identity.repeated_skill_content_estimated_tokens
    ));
    output.push_str(&format!(
        "- **Frontmatter names:** `{}` repeated name group(s), `{}` repeated occurrence(s)\n",
        identity.same_frontmatter_name_groups, identity.same_frontmatter_name_occurrences
    ));

    let repeated_refs = identity
        .source_content_refs
        .iter()
        .filter(|item| item.occurrence_count > 1)
        .collect::<Vec<_>>();
    if !repeated_refs.is_empty() {
        output.push_str("\n### Referentiable Repeated Content\n\n");
        for item in repeated_refs.iter().take(5) {
            output.push_str(&format!(
                "- `{}`: canonical {}; `{}` alias(es), approximately `{}` repeated token(s)\n",
                item.sha256.chars().take(12).collect::<String>(),
                inline_code(&item.canonical_path),
                item.aliases.len(),
                item.repeated_estimated_tokens
            ));
        }
        if repeated_refs.len() > 5 {
            output.push_str(&format!(
                "- {} more repeated content group(s) omitted. Use `--json` for all `source_content_refs`.\n",
                repeated_refs.len() - 5
            ));
        }
    }

    if !identity.frontmatter_name_refs.is_empty() {
        output.push_str("\n### Repeated Frontmatter Names\n\n");
        for item in identity.frontmatter_name_refs.iter().take(5) {
            output.push_str(&format!(
                "- **{}:** `{}` occurrence(s) across {}\n",
                markdown_text(&item.public_name),
                item.occurrence_count,
                item.paths
                    .iter()
                    .map(|path| inline_code(path))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if identity.frontmatter_name_refs.len() > 5 {
            output.push_str(&format!(
                "- {} more repeated frontmatter name group(s) omitted. Use `--json` for all `frontmatter_name_refs`.\n",
                identity.frontmatter_name_refs.len() - 5
            ));
        }
    }

    output.push_str(&format!(
        "\n**Recommendation:** {}\n",
        markdown_text(&identity.recommendation)
    ));
    output
}

pub(super) fn render_workspace_identity_html(identity: &WorkspaceIdentityReport) -> String {
    let mut output = String::new();
    output.push_str("<section class=\"panel\"><h2>Workspace Identity</h2>");
    output.push_str("<dl class=\"pairs compact\">");
    output.push_str(&pair(
        "Source files",
        &identity.source_file_count.to_string(),
    ));
    output.push_str(&pair(
        "Namespaced packages",
        &format!(
            "{} across {} namespace(s)",
            identity.namespaced_package_count, identity.namespace_count
        ),
    ));
    output.push_str(&pair(
        "Unique skill content",
        &format!(
            "{} unique for {} SKILL.md file(s)",
            identity.unique_skill_content_count, identity.skill_file_count
        ),
    ));
    output.push_str(&pair(
        "Repeated content",
        &format!(
            "{} occurrence(s) in {} referentiable group(s)",
            identity.repeated_skill_content_occurrences, identity.repeated_skill_content_groups
        ),
    ));
    output.push_str(&pair(
        "Estimated source tokens",
        &format!(
            "~{} copied byte-for-byte, ~{} unique if referenced, ~{} repeated",
            identity.total_skill_content_estimated_tokens,
            identity.unique_skill_content_estimated_tokens,
            identity.repeated_skill_content_estimated_tokens
        ),
    ));
    output.push_str(&pair(
        "Repeated frontmatter names",
        &format!(
            "{} group(s), {} repeated occurrence(s)",
            identity.same_frontmatter_name_groups, identity.same_frontmatter_name_occurrences
        ),
    ));
    output.push_str("</dl>");

    let repeated_refs = identity
        .source_content_refs
        .iter()
        .filter(|item| item.occurrence_count > 1)
        .collect::<Vec<_>>();
    if !repeated_refs.is_empty() {
        output.push_str("<h3>Referentiable Repeated Content</h3><ul class=\"next\">");
        for item in repeated_refs.iter().take(8) {
            output.push_str(&format!(
                "<li><code>{}</code>: canonical <code>{}</code>; {} alias(es), ~{} repeated token(s)</li>",
                escape_html(&item.sha256.chars().take(12).collect::<String>()),
                escape_html(&item.canonical_path),
                item.aliases.len(),
                item.repeated_estimated_tokens
            ));
        }
        output.push_str("</ul>");
    }
    output.push_str(&format!(
        "<p class=\"callout\">{}</p>",
        escape_html(&identity.recommendation)
    ));
    output.push_str("</section>\n");
    output
}
