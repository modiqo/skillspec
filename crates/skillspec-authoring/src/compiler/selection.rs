use super::Target;
use skillspec_core::model::{DependencyKind, SkillSpec};
use std::fmt::Write;

pub(super) fn write_frontmatter(
    output: &mut String,
    spec: &SkillSpec,
    target: Target,
    name: Option<&str>,
) {
    match target {
        Target::CodexSkill | Target::ClaudeSkill => {
            let _ = writeln!(output, "---");
            let name = name.unwrap_or(spec.id.as_str());
            let _ = writeln!(output, "name: {}", skill_name(name));
            let _ = writeln!(output, "description: {:?}", selection_description(spec));
            let _ = writeln!(output, "---");
            output.push('\n');
        }
        Target::Markdown => {}
    }
}

pub(super) fn selection_description(spec: &SkillSpec) -> String {
    let mut intents = Vec::new();
    for applies_when in &spec.applies_when {
        collect_user_intents(applies_when, &mut intents);
    }

    let activation_summary = spec
        .activation
        .as_ref()
        .map(|activation| activation.summary.trim().trim_end_matches('.'))
        .filter(|summary| !summary.is_empty());

    let mut capabilities = Vec::new();
    let cli_intent_surface = intents.iter().any(|intent| {
        let intent = intent.to_ascii_lowercase();
        intent.contains("cli")
            || intent.contains("shell")
            || intent.contains("command")
            || intent.contains("process")
            || intent.contains("terminal")
    }) || spec.routes.iter().any(|route| {
        let route = format!(
            "{} {}",
            route.label,
            route.description.as_deref().unwrap_or_default()
        )
        .to_ascii_lowercase();
        route.contains("cli")
            || route.contains("shell")
            || route.contains("command")
            || route.contains("process")
            || route.contains("terminal")
    });
    let cli_command_surface = spec
        .commands
        .keys()
        .any(|id| id.contains("exec") || id.contains("process") || id.contains("pty"));
    if cli_intent_surface && cli_command_surface {
        push_unique(&mut capabilities, "CLI and shell commands");
    }
    if spec
        .dependencies
        .values()
        .any(|dependency| dependency.kind == DependencyKind::Adapter)
    {
        push_unique(
            &mut capabilities,
            "APIs, MCP/rote adapters, and service connectors",
        );
    }
    if spec
        .dependencies
        .values()
        .any(|dependency| dependency.kind == DependencyKind::Browser)
    {
        push_unique(&mut capabilities, "browser handoff and page evidence");
    }
    if spec
        .dependencies
        .values()
        .any(|dependency| dependency.kind == DependencyKind::Service)
    {
        push_unique(&mut capabilities, "external services");
    }
    if spec
        .commands
        .keys()
        .any(|id| id.contains("exec") || id.contains("process"))
    {
        push_unique(&mut capabilities, "process capture");
    }
    if spec
        .commands
        .keys()
        .any(|id| id.contains("stream") || id.contains("follow"))
    {
        push_unique(&mut capabilities, "logs and streams");
    }
    if spec.commands.keys().any(|id| id.contains("pty")) {
        push_unique(&mut capabilities, "PTY and terminal-sensitive prompts");
    }
    if spec
        .commands
        .keys()
        .any(|id| id.contains("deps") || id.contains("dependency"))
    {
        push_unique(&mut capabilities, "dependency checks");
    }
    if spec
        .closures
        .keys()
        .any(|id| id.contains("crystallize") || id.contains("crystallization"))
    {
        push_unique(&mut capabilities, "flow crystallization and replay");
    }

    let mut parts = Vec::new();
    if let Some(summary) = activation_summary {
        parts.push(summary.to_owned());
    }
    if let Some(activation) = &spec.activation {
        if !activation.keywords.is_empty() {
            parts.push(format!(
                "Use for {}",
                sentence_list(
                    &activation
                        .keywords
                        .iter()
                        .take(12)
                        .cloned()
                        .collect::<Vec<_>>()
                )
            ));
        }
    }
    if !intents.is_empty() {
        parts.push(format!(
            "Use when the task needs to {}",
            sentence_list(&intents.into_iter().take(7).collect::<Vec<_>>())
        ));
    } else {
        parts.push(format!(
            "Use for {}",
            lower_first(spec.description.trim_end_matches('.'))
        ));
    }
    if !capabilities.is_empty() {
        parts.push(format!(
            "Handles {}",
            sentence_list(&capabilities.into_iter().take(8).collect::<Vec<_>>())
        ));
    }
    if spec
        .entry
        .as_ref()
        .is_some_and(|entry| entry.decision_required)
    {
        parts.push(
            "Requires `skillspec decide` before substrate tools or overlapping low-level skills"
                .to_owned(),
        );
    }
    parts.push(
        "Preserves evidence with SkillSpec routes, forbids, dependencies, traces, and token-savings reports"
            .to_owned(),
    );
    shorten_description(&parts.join(". "))
}

pub(super) fn collect_user_intents(value: &serde_yaml::Value, intents: &mut Vec<String>) {
    let serde_yaml::Value::Mapping(mapping) = value else {
        return;
    };
    let Some(user_intent) = mapping.get(serde_yaml::Value::String("user_intent".to_owned())) else {
        return;
    };
    match user_intent {
        serde_yaml::Value::Sequence(values) => {
            for value in values {
                if let Some(value) = value.as_str() {
                    push_unique(intents, value);
                }
            }
        }
        serde_yaml::Value::String(value) => push_unique(intents, value),
        _ => {}
    }
}

pub(super) fn push_unique(values: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if value.is_empty() || values.iter().any(|existing| existing == value) {
        return;
    }
    values.push(value.to_owned());
}

pub(super) fn sentence_list(values: &[String]) -> String {
    match values {
        [] => String::new(),
        [only] => only.clone(),
        [head @ .., last] => format!("{} and {}", head.join(", "), last),
    }
}

pub(super) fn shorten_description(value: &str) -> String {
    const LIMIT: usize = 900;
    if value.len() <= LIMIT {
        return value.to_owned();
    }
    let mut shortened = value[..LIMIT].trim_end().to_owned();
    if let Some(index) = shortened.rfind(['.', ',', ';']) {
        shortened.truncate(index);
    }
    shortened.trim_end().trim_end_matches('.').to_owned()
}

pub(super) fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut lowered = first.to_lowercase().collect::<String>();
    lowered.push_str(chars.as_str());
    lowered
}

fn skill_name(id: &str) -> String {
    let mut name = String::new();
    let mut last_was_dash = false;
    for char in id.chars() {
        let next = if char.is_ascii_alphanumeric() {
            last_was_dash = false;
            Some(char.to_ascii_lowercase())
        } else if !last_was_dash {
            last_was_dash = true;
            Some('-')
        } else {
            None
        };
        if let Some(char) = next {
            name.push(char);
        }
    }
    name.trim_matches('-').to_owned()
}
