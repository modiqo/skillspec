use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub(super) struct ObservedCommand {
    pub(super) template: String,
    pub(super) tool: Option<String>,
    pub(super) dependency_id: Option<String>,
}

pub(super) fn infer_observed_commands(log: &str) -> Vec<ObservedCommand> {
    let commands = serde_json::from_str::<serde_json::Value>(log)
        .ok()
        .map(|value| {
            let mut commands = Vec::new();
            collect_json_commands(&value, &mut commands);
            commands
        })
        .filter(|commands| !commands.is_empty())
        .unwrap_or_else(|| collect_text_commands(log));

    let mut seen = BTreeSet::new();
    commands
        .into_iter()
        .filter_map(|command| command_without_rote_provenance(&command))
        .filter(|command| seen.insert(command.clone()))
        .take(20)
        .map(|template| {
            let tool = command_tool(&template);
            let dependency_id = tool.as_deref().map(dependency_id_for_tool);
            ObservedCommand {
                template,
                tool,
                dependency_id,
            }
        })
        .collect()
}

pub(super) fn dependency_id_for_tool(tool: &str) -> String {
    sanitize_id(tool)
}

fn command_without_rote_provenance(command: &str) -> Option<String> {
    if is_rote_provenance_command(command) {
        return None;
    }
    if command.trim_start().starts_with("rote exec --") {
        return normalize_command(command.trim_start().trim_start_matches("rote exec --"));
    }
    if command.contains(" rote exec --") {
        let (_, tail) = command.split_once(" rote exec --")?;
        return normalize_command(tail);
    }
    let command = unwrap_shell_command(command);
    normalize_command(&command)
}

fn is_rote_provenance_command(command: &str) -> bool {
    let lower = command.trim().to_ascii_lowercase();
    lower.starts_with("rote workspace ")
        || lower.starts_with("rote progress ")
        || lower.starts_with("rote trace ")
        || lower == "rote workspace"
}

fn unwrap_shell_command(command: &str) -> String {
    let trimmed = command.trim();
    for prefix in ["sh -c ", "bash -lc ", "zsh -lc "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim_matches('"').trim_matches('\'').to_owned();
        }
    }
    trimmed.to_owned()
}

fn collect_json_commands(value: &serde_json::Value, output: &mut Vec<String>) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                collect_json_commands(value, output);
            }
        }
        serde_json::Value::Object(map) => {
            for key in [
                "command",
                "cmd",
                "argv",
                "invocation",
                "shell_command",
                "raw_command",
            ] {
                if let Some(value) = map.get(key) {
                    match value {
                        serde_json::Value::String(text) => output.push(text.clone()),
                        serde_json::Value::Array(parts) if key == "argv" => {
                            let text = parts
                                .iter()
                                .filter_map(|part| part.as_str())
                                .collect::<Vec<_>>()
                                .join(" ");
                            if !text.is_empty() {
                                output.push(text);
                            }
                        }
                        _ => {}
                    }
                }
            }
            for value in map.values() {
                collect_json_commands(value, output);
            }
        }
        serde_json::Value::String(text) if text.contains("rote exec --") => {
            output.push(text.clone());
        }
        _ => {}
    }
}

fn collect_text_commands(log: &str) -> Vec<String> {
    log.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            for marker in ["command:", "cmd:", "$ "] {
                if let Some((_, tail)) = trimmed.split_once(marker) {
                    let command = tail.trim();
                    if !command.is_empty() {
                        return Some(command.to_owned());
                    }
                }
            }
            if trimmed.contains("rote exec --") {
                Some(trimmed.to_owned())
            } else {
                None
            }
        })
        .collect()
}

fn normalize_command(command: &str) -> Option<String> {
    let cleaned = command
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(';')
        .trim();
    (!cleaned.is_empty()).then(|| cleaned.to_owned())
}

fn command_tool(command: &str) -> Option<String> {
    if let Some(captures) = cli_discovery_regex().captures(command) {
        if let Some(tool) = captures.name("tool").and_then(normalize_tool_name) {
            return Some(tool);
        }
    }
    let captures = cli_invocation_regex().captures(command)?;
    captures.name("tool").and_then(normalize_tool_name)
}

fn cli_discovery_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?x)
            ^\s*
            (?:\$\s*)?
            (?:env\s+(?:-[A-Za-z]+\s+)*)?
            (?:[A-Za-z_][A-Za-z0-9_]*=(?:"[^"]*"|'[^']*'|[^\s]+)\s+)*
            (?:command\s+-v|which|type\s+-P)
            \s+
            (?P<tool>(?:[./~\w-]+/)?[A-Za-z][A-Za-z0-9_.-]*)
            (?:\s|$)
            "#,
        )
        .expect("CLI discovery regex must compile")
    })
}

fn cli_invocation_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?x)
            ^\s*
            (?:\$\s*)?
            (?:env\s+(?:-[A-Za-z]+\s+)*)?
            (?:[A-Za-z_][A-Za-z0-9_]*=(?:"[^"]*"|'[^']*'|[^\s]+)\s+)*
            (?P<tool>(?:[./~\w-]+/)?[A-Za-z][A-Za-z0-9_.-]*)
            (?:\s|$)
            "#,
        )
        .expect("CLI invocation regex must compile")
    })
}

fn normalize_tool_name(raw: regex::Match<'_>) -> Option<String> {
    let value = raw
        .as_str()
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`');
    if value.is_empty() || value.contains('=') {
        return None;
    }
    let file_name = Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value);
    let id = sanitize_id(file_name);
    (!id.is_empty()).then(|| file_name.to_owned())
}

pub(super) fn dependency_ids(commands: &[ObservedCommand]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|command| command.dependency_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn skill_id(name: Option<&str>, task: &str) -> String {
    name.map(sanitize_id)
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| {
            if task.to_ascii_lowercase().contains("profile")
                && task.to_ascii_lowercase().contains("enrich")
            {
                "parallel_profile_enricher".to_owned()
            } else {
                "cli_workflow".to_owned()
            }
        })
}

fn sanitize_id(input: &str) -> String {
    let mut out = String::new();
    for char in input.chars() {
        if char.is_ascii_alphanumeric() {
            out.push(char.to_ascii_lowercase());
        } else if matches!(char, '-' | '_' | '.' | ' ') && !out.ends_with('_') {
            out.push('_');
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "cli_workflow".to_owned()
    } else {
        out
    }
}

pub(super) fn title_from_id(id: &str) -> String {
    id.split(['_', '-', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
