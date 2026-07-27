//! Markdown extraction.
//!
//! Two things in a Markdown file produce effects: fenced code blocks, which are
//! handed to the matching language extractor, and external links, which are at
//! minimum a fetch.
//!
//! Prose that merely *describes* an effect without a command or a link produces
//! nothing. That is a deliberate under-approximation: prose targets are not
//! determinable, and inventing them would put guesses into the grant set.

use super::shell;
use crate::effect::{
    Confidence, EffectClass, EffectEvidence, EffectObservation, EffectOrigin, EffectTarget, Reach,
};
use crate::normalize::url;
use skillspec_source::source_map::{SourceMap, SourceReferenceKind};

/// Fence languages handled by the shell extractor.
///
/// Unlabeled fences are included because shell is the dominant case in skills;
/// doctor separately reports unlabeled fences as a hygiene finding.
/// How confident the fence label is that the block is shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShellFence {
    /// Labeled `sh`/`bash`/etc.: extract every line.
    Explicit,
    /// Unlabeled: extract only lines that carry an effect signal, since these
    /// blocks are as likely to hold prose or output as commands.
    Ambiguous,
    /// Not shell: `python`, `json`, or `text`, which is declared non-code.
    No,
}

fn shell_fence(language: Option<&str>) -> ShellFence {
    match language.map(str::trim).map(str::to_ascii_lowercase) {
        None => ShellFence::Ambiguous,
        Some(language) => match language.as_str() {
            "" => ShellFence::Ambiguous,
            "sh" | "bash" | "zsh" | "shell" | "console" | "terminal" | "shell-session"
            | "shellsession" => ShellFence::Explicit,
            _ => ShellFence::No,
        },
    }
}

/// Extract effects from one Markdown file already present in the source map.
pub fn extract(
    map: &SourceMap,
    file_id: &str,
    file_path: &str,
    content: &str,
    reach: Reach,
) -> Vec<EffectObservation> {
    let mut observations = Vec::new();
    extract_code_blocks(map, file_id, file_path, content, reach, &mut observations);
    extract_external_links(map, file_id, file_path, reach, &mut observations);
    observations
}

fn extract_code_blocks(
    map: &SourceMap,
    file_id: &str,
    file_path: &str,
    content: &str,
    reach: Reach,
    out: &mut Vec<EffectObservation>,
) {
    for node in map
        .nodes
        .iter()
        .filter(|node| node.file == file_id && node.kind == "code")
    {
        let fence = shell_fence(node.language.as_deref());
        if fence == ShellFence::No {
            continue;
        }
        let Some([start, end]) = node.byte_range else {
            continue;
        };
        let Some(text) = content.get(start..end) else {
            continue;
        };
        let first_line = node.line_range.map(|range| range[0]).unwrap_or(1);
        let (body, body_line) = strip_fence(text, first_line);
        if looks_like_data(body) {
            continue;
        }
        let body = match fence {
            ShellFence::Explicit => body.to_owned(),
            // An unlabeled fence is as likely to be prose or program output as
            // commands, so keep only lines that carry an effect signal.
            _ => keep_command_lines(body),
        };
        out.extend(shell::extract(
            &body,
            shell::ShellContext {
                path: file_path,
                origin: EffectOrigin::MarkdownCodeBlock,
                reach,
                first_line: body_line,
            },
        ));
    }
}

fn extract_external_links(
    map: &SourceMap,
    file_id: &str,
    file_path: &str,
    reach: Reach,
    out: &mut Vec<EffectObservation>,
) {
    let node_lines = |node_id: &str| {
        map.nodes
            .iter()
            .find(|node| node.id == node_id)
            .and_then(|node| node.line_range)
            .map(|range| range[0])
    };

    for reference in map
        .references
        .iter()
        .filter(|reference| reference.target_kind == SourceReferenceKind::ExternalUri)
    {
        // References carry their source node, which belongs to exactly one file.
        let belongs_to_file = map
            .nodes
            .iter()
            .any(|node| node.id == reference.source && node.file == file_id);
        if !belongs_to_file {
            continue;
        }
        let Some(normalized) = url::normalize(&reference.target) else {
            continue;
        };
        out.push(EffectObservation {
            class: EffectClass::NetFetch,
            target: EffectTarget::Host {
                host: normalized.host,
                scheme: normalized.scheme,
                port: normalized.port,
                ip_literal: normalized.ip_literal,
                non_ascii: normalized.non_ascii,
            },
            resolution: normalized.resolution,
            origin: EffectOrigin::MarkdownProse,
            reach,
            // A documentation link is weaker evidence of a runtime fetch than a
            // command is, but omitting it would drop the host entirely.
            confidence: Confidence::Low,
            evidence: EffectEvidence::new(
                file_path,
                node_lines(&reference.source),
                &reference.target,
            )
            .with_node(reference.source.clone()),
        });
    }
}

/// Keep only lines from an unlabeled block that plausibly carry an effect.
///
/// Unlabeled fences hold as much English and program output as they do
/// commands. Rather than tokenize a sentence into imaginary binaries, keep a
/// line only when it shows a concrete signal: a shell operator, a URL, an
/// environment variable, or a first token that is a real path or a known
/// effect-bearing binary. Blank lines are preserved so reported line numbers
/// stay accurate.
fn keep_command_lines(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line_has_effect_signal(line) {
                line
            } else {
                ""
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_has_effect_signal(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let stripped = trimmed
        .trim_start_matches(['$', '#', '>', ' '])
        .trim_start();
    let first = stripped.split_whitespace().next().unwrap_or_default();
    // A prompt-prefixed or operator-bearing line, or a URL, is a command.
    if trimmed.contains("://")
        || trimmed.contains("${")
        || first.contains('/')
        || first.starts_with('~')
    {
        return true;
    }
    // Otherwise the first token must be a plausible effect-bearing program.
    let name = first
        .rsplit('/')
        .next()
        .unwrap_or(first)
        .to_ascii_lowercase();
    EFFECTFUL_BINARIES.contains(&name.as_str())
}

/// Binaries whose presence marks an unlabeled line as a real command.
///
/// Kept deliberately narrow: this list only rescues lines from unlabeled
/// fences, and a false negative there fails closed. Explicitly labeled shell
/// blocks are not filtered.
const EFFECTFUL_BINARIES: &[&str] = &[
    "curl", "wget", "nc", "ncat", "netcat", "ssh", "scp", "sftp", "rsync", "git", "cat", "tee",
    "cp", "mv", "rm", "ln", "chmod", "chown", "tar", "zip", "unzip", "gzip", "base64", "openssl",
    "dd", "mkdir", "touch", "npm", "pnpm", "yarn", "pip", "pip3", "cargo", "gem", "go", "brew",
    "apt", "apt-get", "python", "python3", "node", "sh", "bash", "zsh", "eval", "sudo", "docker",
    "kubectl", "aws", "gcloud", "gh", "jq", "yq", "sed", "awk", "grep", "rg", "find", "export",
    "source", "env", "printenv",
];

/// Whether an unlabeled block is structured data rather than commands.
///
/// Skills document output shapes in unlabeled fences constantly. Reading a JSON
/// sample as shell produces nothing but noise.
fn looks_like_data(body: &str) -> bool {
    let first = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    first.starts_with('{') || first.starts_with('[')
}

/// Drop the opening and closing fence lines from a code node's slice.
///
/// The source map's byte range covers the whole fenced block, delimiters
/// included. Handing those to the shell extractor would record ` ``` ` as a
/// command, so the fence comes off here and the reported line number moves with
/// it.
fn strip_fence(text: &str, first_line: usize) -> (&str, usize) {
    let is_fence = |line: &str| {
        let trimmed = line.trim_start();
        trimmed.starts_with("```") || trimmed.starts_with("~~~")
    };

    let mut body = text;
    let mut line = first_line;
    if let Some((head, rest)) = body.split_once('\n') {
        if is_fence(head) {
            body = rest;
            line += 1;
        }
    }
    if let Some(index) = body.rfind('\n') {
        if is_fence(&body[index + 1..]) {
            body = &body[..index];
        }
    } else if is_fence(body) {
        body = "";
    }
    (body, line)
}

#[cfg(test)]
mod tests {
    use super::strip_fence;

    #[test]
    fn labeled_shell_fences_are_explicit() {
        for language in [Some("sh"), Some("bash"), Some("Console")] {
            assert_eq!(
                super::shell_fence(language),
                super::ShellFence::Explicit,
                "{language:?}"
            );
        }
    }

    #[test]
    fn unlabeled_fences_are_ambiguous_and_text_is_not_shell() {
        assert_eq!(super::shell_fence(None), super::ShellFence::Ambiguous);
        assert_eq!(super::shell_fence(Some("")), super::ShellFence::Ambiguous);
        // `text` declares the block is not code.
        assert_eq!(super::shell_fence(Some("text")), super::ShellFence::No);
    }

    #[test]
    fn other_languages_are_left_to_their_own_extractors() {
        for language in [Some("python"), Some("rust"), Some("json"), Some("yaml")] {
            assert_eq!(
                super::shell_fence(language),
                super::ShellFence::No,
                "{language:?}"
            );
        }
    }

    #[test]
    fn prose_lines_are_dropped_from_unlabeled_blocks() {
        // The claude-api failure: documentation sentences became binaries.
        let kept = super::keep_command_lines(
            "The API returns a response object.\nYou can then read the result.\ncurl https://api.example.com",
        );
        assert!(kept.contains("curl https://api.example.com"));
        assert!(!kept.contains("The API"));
        assert!(!kept.contains("You can"));
    }

    #[test]
    fn command_lines_survive_the_prose_filter() {
        for line in [
            "cat ~/.aws/credentials",
            "$ git status",
            "curl -d @- https://x.test",
            "python3 build.py",
            "./scripts/run.sh",
        ] {
            assert!(super::line_has_effect_signal(line), "{line}");
        }
    }

    #[test]
    fn structured_data_blocks_are_not_read_as_commands() {
        assert!(super::looks_like_data("{\n  \"field\": 1\n}"));
        assert!(super::looks_like_data("[\n  1\n]"));
        assert!(!super::looks_like_data("git status"));
        assert!(!super::looks_like_data(""));
    }

    #[test]
    fn fence_delimiters_are_removed_and_the_line_number_follows() {
        let (body, line) = strip_fence("```sh\ngit status\n```", 4);
        assert_eq!(body, "git status");
        assert_eq!(line, 5);
    }

    #[test]
    fn an_unfenced_slice_is_returned_unchanged() {
        let (body, line) = strip_fence("git status", 4);
        assert_eq!(body, "git status");
        assert_eq!(line, 4);
    }

    #[test]
    fn a_multi_line_body_keeps_every_command() {
        let (body, _) = strip_fence("```\na\nb\nc\n```", 1);
        assert_eq!(body, "a\nb\nc");
    }
}
