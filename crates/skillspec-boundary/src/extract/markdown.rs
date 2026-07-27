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
fn is_shell_language(language: Option<&str>) -> bool {
    match language.map(str::trim).map(str::to_ascii_lowercase) {
        None => true,
        Some(language) => matches!(
            language.as_str(),
            "" | "sh" | "bash" | "zsh" | "shell" | "console" | "terminal" | "text"
        ),
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
        if !is_shell_language(node.language.as_deref()) {
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
        out.extend(shell::extract(
            body,
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
    use super::{is_shell_language, strip_fence};

    #[test]
    fn shell_fences_and_unlabeled_fences_are_extracted() {
        for language in [None, Some(""), Some("sh"), Some("bash"), Some("Console")] {
            assert!(is_shell_language(language), "{language:?}");
        }
    }

    #[test]
    fn other_languages_are_left_to_their_own_extractors() {
        for language in [Some("python"), Some("rust"), Some("json"), Some("yaml")] {
            assert!(!is_shell_language(language), "{language:?}");
        }
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
