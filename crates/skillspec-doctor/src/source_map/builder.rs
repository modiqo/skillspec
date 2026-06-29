use super::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const CHUNKED_MARKDOWN_THRESHOLD_BYTES: usize = 256 * 1024;

pub(super) fn build(source: &Path) -> Result<SourceMap> {
    let source_root = source_root(source);
    let files = discover_files(source, &source_root)?;
    let mut nodes = Vec::new();
    let mut classifications = Vec::new();
    let mut references = Vec::new();
    let mut diagnostics = Vec::new();

    for file in files
        .iter()
        .filter(|file| file.kind == SourceFileKind::Markdown)
    {
        let path = source_root.join(&file.path);
        match parse_markdown_file(&source_root, file, &path) {
            Ok(parsed) => {
                nodes.extend(parsed.nodes);
                classifications.extend(parsed.classifications);
                references.extend(parsed.references);
            }
            Err(error) => diagnostics.push(SourceDiagnostic {
                level: "error".to_owned(),
                message: error.to_string(),
                path: Some(file.path.clone()),
            }),
        }
    }

    let coverage = coverage(&files, &nodes, &classifications, 0);
    Ok(SourceMap {
        schema: SOURCE_MAP_SCHEMA.to_owned(),
        source_root: source_root.display().to_string(),
        generator: SourceMapGenerator {
            name: "skillspec".to_owned(),
            markdown_engine: "markdown-rs".to_owned(),
        },
        files,
        nodes,
        classifications,
        references,
        coverage,
        diagnostics,
    })
}

fn discover_files(source: &Path, source_root: &Path) -> Result<Vec<SourceFileRecord>> {
    let mut paths = Vec::new();
    if source.is_file() {
        paths.push(source.to_path_buf());
    } else {
        collect_files(source, &mut paths)?;
    }
    paths.sort();

    let mut files = Vec::new();
    for path in paths {
        let relative = path.strip_prefix(source_root).unwrap_or(&path);
        let relative_string = path_to_spec_string(relative);
        let bytes = fs::read(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        let kind = file_kind(&path);
        let is_text = std::str::from_utf8(&bytes).ok();
        let load_status = if kind == SourceFileKind::Asset || is_text.is_none() {
            SourceFileLoadStatus::BinaryPreserved
        } else {
            SourceFileLoadStatus::Loaded
        };
        files.push(SourceFileRecord {
            id: format!("file:{}", file_slug(relative)),
            path: relative_string.clone(),
            kind,
            sha256: sha256_hex(&bytes),
            bytes: bytes.len(),
            lines: is_text.map(|text| text.lines().count()).unwrap_or(0),
            role_candidates: role_candidates(relative, &relative_string),
            load_status,
        });
    }
    Ok(files)
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if should_skip(&path) {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn should_skip(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with('.') || matches!(name, "target" | "node_modules")
}

struct ParsedMarkdown {
    nodes: Vec<SourceNodeRecord>,
    classifications: Vec<SourceClassificationRecord>,
    references: Vec<SourceReferenceRecord>,
}

fn parse_markdown_file(
    source_root: &Path,
    file: &SourceFileRecord,
    path: &Path,
) -> Result<ParsedMarkdown> {
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let frontmatter = extract_frontmatter(file, &content);
    let (body, byte_offset, line_offset) = match &frontmatter {
        Some(frontmatter) => {
            let end = frontmatter.byte_range.map(|range| range[1]).unwrap_or(0);
            (
                &content[end..],
                end,
                frontmatter.line_range.map(|range| range[1]).unwrap_or(0),
            )
        }
        None => (content.as_str(), 0, 0),
    };
    if body.len() > CHUNKED_MARKDOWN_THRESHOLD_BYTES {
        return parse_markdown_file_chunked(
            source_root,
            file,
            &content,
            body,
            byte_offset,
            line_offset,
            frontmatter,
        );
    }
    let mdast = markdown::to_mdast(body, &markdown::ParseOptions::gfm()).map_err(|message| {
        Error::InvalidInput {
            message: format!("failed to parse Markdown {}: {message:?}", file.path),
        }
    })?;
    let value = serde_json::to_value(mdast)?;
    let mut ctx = ParseContext {
        source_root,
        file,
        content: &content,
        byte_offset,
        line_offset,
        counters: BTreeMap::new(),
        heading_slugs: BTreeMap::new(),
        nodes: Vec::new(),
        classifications: Vec::new(),
        references: Vec::new(),
    };
    if let Some(frontmatter) = frontmatter {
        ctx.nodes.push(frontmatter);
    }
    visit_node(&mut ctx, &value, None);
    Ok(ParsedMarkdown {
        nodes: ctx.nodes,
        classifications: ctx.classifications,
        references: ctx.references,
    })
}

fn parse_markdown_file_chunked(
    source_root: &Path,
    file: &SourceFileRecord,
    content: &str,
    body: &str,
    byte_offset: usize,
    line_offset: usize,
    frontmatter: Option<SourceNodeRecord>,
) -> Result<ParsedMarkdown> {
    let file_slug = file.id.trim_start_matches("file:");
    let mut nodes = Vec::new();
    let mut classifications = Vec::new();
    let mut references = Vec::new();
    let mut children = Vec::new();
    let mut counters = BTreeMap::<String, usize>::new();
    let mut heading_slugs = BTreeMap::<String, usize>::new();
    if let Some(frontmatter) = frontmatter {
        children.push(frontmatter.id.clone());
        nodes.push(frontmatter);
    }

    let mut paragraph_start: Option<(usize, usize)> = None;
    let mut paragraph = String::new();
    let mut in_code: Option<(String, usize, usize, usize)> = None;
    let mut code = String::new();
    let mut cursor = byte_offset;

    for (index, line) in body.split_inclusive('\n').enumerate() {
        let line_number = line_offset + index + 1;
        let line_start = cursor;
        cursor += line.len();
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let fence = code_fence(trimmed);

        if let Some((language, start_line, start_byte, fence_len)) = in_code.as_ref().cloned() {
            if fence.is_some_and(|(_, current_fence_len)| current_fence_len >= fence_len) {
                let id = next_node_id(file_slug, "code", &mut counters);
                let end_byte = cursor;
                let record = SourceNodeRecord {
                    id: id.clone(),
                    file: file.id.clone(),
                    kind: "code".to_owned(),
                    depth: None,
                    title: None,
                    language: (!language.is_empty()).then_some(language.clone()),
                    parent: Some(format!("root:{file_slug}")),
                    children: Vec::new(),
                    byte_range: Some([start_byte, end_byte]),
                    line_range: Some([start_line, line_number]),
                    hash: Some(sha256_hex(
                        source_slice(content, Some([start_byte, end_byte])).as_bytes(),
                    )),
                    text_preview: preview(&code),
                    coverage_status: SourceCoverageStatus::Mapped,
                };
                classify_code_text(&mut classifications, &record, &language, &code);
                children.push(id);
                nodes.push(record);
                in_code = None;
                code.clear();
            } else {
                code.push_str(line);
            }
            continue;
        }

        if let Some((language, fence_len)) = fence {
            flush_paragraph_chunk(
                source_root,
                file,
                content,
                &mut nodes,
                &mut classifications,
                &mut references,
                &mut children,
                &mut counters,
                paragraph_start.take(),
                &mut paragraph,
                line_start,
                line_number,
            );
            in_code = Some((language, line_number, line_start, fence_len));
            continue;
        }

        if let Some(title) = heading_title(trimmed) {
            flush_paragraph_chunk(
                source_root,
                file,
                content,
                &mut nodes,
                &mut classifications,
                &mut references,
                &mut children,
                &mut counters,
                paragraph_start.take(),
                &mut paragraph,
                line_start,
                line_number,
            );
            let slug = slug(&title);
            let key = format!("{file_slug}:{slug}");
            let count = heading_slugs.entry(key).or_insert(0);
            *count += 1;
            let id = if *count == 1 {
                format!("heading:{file_slug}.{slug}")
            } else {
                format!("heading:{file_slug}.{slug}-{count}")
            };
            let depth = trimmed.chars().take_while(|char| *char == '#').count() as u8;
            let record = SourceNodeRecord {
                id: id.clone(),
                file: file.id.clone(),
                kind: "heading".to_owned(),
                depth: Some(depth),
                title: Some(title.clone()),
                language: None,
                parent: Some(format!("root:{file_slug}")),
                children: Vec::new(),
                byte_range: Some([line_start, cursor]),
                line_range: Some([line_number, line_number]),
                hash: Some(sha256_hex(line.as_bytes())),
                text_preview: preview(&title),
                coverage_status: SourceCoverageStatus::Mapped,
            };
            classify_text_record(&mut classifications, &record, &title);
            collect_text_references(source_root, file, &record.id, &title, &mut references);
            children.push(id);
            nodes.push(record);
            continue;
        }

        if trimmed.trim().is_empty() {
            flush_paragraph_chunk(
                source_root,
                file,
                content,
                &mut nodes,
                &mut classifications,
                &mut references,
                &mut children,
                &mut counters,
                paragraph_start.take(),
                &mut paragraph,
                line_start,
                line_number,
            );
            continue;
        }

        if paragraph_start.is_none() {
            paragraph_start = Some((line_number, line_start));
        }
        paragraph.push_str(line);
    }

    flush_paragraph_chunk(
        source_root,
        file,
        content,
        &mut nodes,
        &mut classifications,
        &mut references,
        &mut children,
        &mut counters,
        paragraph_start,
        &mut paragraph,
        cursor,
        line_offset + body.lines().count().max(1),
    );

    let root_id = format!("root:{file_slug}");
    nodes.insert(
        0,
        SourceNodeRecord {
            id: root_id,
            file: file.id.clone(),
            kind: "root".to_owned(),
            depth: None,
            title: Some("Markdown root (chunked)".to_owned()),
            language: None,
            parent: None,
            children,
            byte_range: Some([byte_offset, content.len()]),
            line_range: Some([line_offset + 1, content.lines().count()]),
            hash: Some(sha256_hex(body.as_bytes())),
            text_preview: Some("Large Markdown file mapped in chunked mode.".to_owned()),
            coverage_status: SourceCoverageStatus::Mapped,
        },
    );

    Ok(ParsedMarkdown {
        nodes,
        classifications,
        references,
    })
}

struct ParseContext<'a> {
    source_root: &'a Path,
    file: &'a SourceFileRecord,
    content: &'a str,
    byte_offset: usize,
    line_offset: usize,
    counters: BTreeMap<String, usize>,
    heading_slugs: BTreeMap<String, usize>,
    nodes: Vec<SourceNodeRecord>,
    classifications: Vec<SourceClassificationRecord>,
    references: Vec<SourceReferenceRecord>,
}

fn visit_node(ctx: &mut ParseContext<'_>, node: &Value, parent: Option<String>) -> Option<String> {
    let kind = node_type(node)?;
    let sequence = next_counter(&mut ctx.counters, &kind);
    let file_id = ctx.file.id.clone();
    let handle = node_handle(ctx, &file_id, &kind, sequence, node);
    let children_values = node
        .get("children")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut child_handles = Vec::new();
    for child in &children_values {
        if let Some(child_handle) = visit_node(ctx, child, Some(handle.clone())) {
            child_handles.push(child_handle);
        }
    }

    let byte_range = byte_range(ctx, node);
    let line_range = line_range(ctx, node);
    let text = node_text(node);
    let text_preview = preview(
        text.as_deref()
            .unwrap_or_else(|| source_slice(ctx.content, byte_range)),
    );
    let hash =
        byte_range.map(|range| sha256_hex(source_slice(ctx.content, Some(range)).as_bytes()));
    let language = node.get("lang").and_then(Value::as_str).map(str::to_owned);
    let title = (kind == "heading")
        .then(|| collect_plain_text(node))
        .flatten();
    let depth = node
        .get("depth")
        .and_then(Value::as_u64)
        .and_then(|depth| u8::try_from(depth).ok());

    let record = SourceNodeRecord {
        id: handle.clone(),
        file: ctx.file.id.clone(),
        kind: kind.clone(),
        depth,
        title,
        language,
        parent,
        children: child_handles,
        byte_range,
        line_range,
        hash,
        text_preview,
        coverage_status: SourceCoverageStatus::Mapped,
    };
    classify_node(ctx, &record, node);
    collect_references(ctx, &record, node);
    ctx.nodes.push(record);
    Some(handle)
}

fn classify_node(ctx: &mut ParseContext<'_>, record: &SourceNodeRecord, node: &Value) {
    if record.kind == "code" {
        ctx.classifications.push(SourceClassificationRecord {
            id: format!("class:{}:code", record.id),
            target: record.id.clone(),
            kind: SourceClassificationKind::CodeBlock,
            signals: record.language.clone().into_iter().collect(),
            suggested_constructs: vec!["code".to_owned(), "resource".to_owned()],
            confidence: "high".to_owned(),
            reason: "Fenced code block parsed from Markdown AST.".to_owned(),
            coverage_status: SourceCoverageStatus::ReviewRequired,
        });
        classify_code_dependencies(ctx, record, node);
    }

    if !matches!(
        record.kind.as_str(),
        "text" | "inlinecode" | "code" | "heading"
    ) {
        return;
    }

    let text = collect_plain_text(node).unwrap_or_default();
    let lowered = text.to_ascii_lowercase();
    let modal_signals = signals(
        &lowered,
        &["must", "never", "only", "always", "avoid", "forbid"],
    );
    if !modal_signals.is_empty() {
        ctx.classifications.push(SourceClassificationRecord {
            id: format!("class:{}:modal", record.id),
            target: record.id.clone(),
            kind: SourceClassificationKind::ModalObligation,
            signals: modal_signals,
            suggested_constructs: vec!["rule".to_owned()],
            confidence: "medium".to_owned(),
            reason: "Text contains modal obligation language.".to_owned(),
            coverage_status: SourceCoverageStatus::ReviewRequired,
        });
    }

    let dependency_signals = signals(
        &lowered,
        &[
            "requires",
            "dependency",
            "dependencies",
            "pip install",
            "npm install",
            "cargo install",
            "brew install",
        ],
    );
    if !dependency_signals.is_empty() {
        ctx.classifications.push(SourceClassificationRecord {
            id: format!("class:{}:dependency", record.id),
            target: record.id.clone(),
            kind: SourceClassificationKind::DependencyMention,
            signals: dependency_signals,
            suggested_constructs: vec!["dependency".to_owned(), "deps.toml".to_owned()],
            confidence: "medium".to_owned(),
            reason: "Text mentions dependency or install language.".to_owned(),
            coverage_status: SourceCoverageStatus::ReviewRequired,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn flush_paragraph_chunk(
    source_root: &Path,
    file: &SourceFileRecord,
    content: &str,
    nodes: &mut Vec<SourceNodeRecord>,
    classifications: &mut Vec<SourceClassificationRecord>,
    references: &mut Vec<SourceReferenceRecord>,
    children: &mut Vec<String>,
    counters: &mut BTreeMap<String, usize>,
    start: Option<(usize, usize)>,
    paragraph: &mut String,
    end_byte: usize,
    end_line: usize,
) {
    let Some((start_line, start_byte)) = start else {
        paragraph.clear();
        return;
    };
    let text = paragraph.trim();
    if text.is_empty() {
        paragraph.clear();
        return;
    }
    let file_slug = file.id.trim_start_matches("file:");
    let id = next_node_id(file_slug, "paragraph_chunk", counters);
    let end_byte = end_byte.min(content.len());
    let record = SourceNodeRecord {
        id: id.clone(),
        file: file.id.clone(),
        kind: "paragraph_chunk".to_owned(),
        depth: None,
        title: None,
        language: None,
        parent: Some(format!("root:{file_slug}")),
        children: Vec::new(),
        byte_range: Some([start_byte, end_byte]),
        line_range: Some([start_line, end_line.saturating_sub(1).max(start_line)]),
        hash: Some(sha256_hex(text.as_bytes())),
        text_preview: preview(text),
        coverage_status: SourceCoverageStatus::Mapped,
    };
    classify_text_record(classifications, &record, text);
    collect_text_references(source_root, file, &record.id, text, references);
    children.push(id);
    nodes.push(record);
    paragraph.clear();
}

fn classify_text_record(
    classifications: &mut Vec<SourceClassificationRecord>,
    record: &SourceNodeRecord,
    text: &str,
) {
    let lowered = text.to_ascii_lowercase();
    let modal_signals = signals(
        &lowered,
        &["must", "never", "only", "always", "avoid", "forbid"],
    );
    if !modal_signals.is_empty() {
        classifications.push(SourceClassificationRecord {
            id: format!("class:{}:modal", record.id),
            target: record.id.clone(),
            kind: SourceClassificationKind::ModalObligation,
            signals: modal_signals,
            suggested_constructs: vec!["rule".to_owned()],
            confidence: "medium".to_owned(),
            reason: "Text contains modal obligation language.".to_owned(),
            coverage_status: SourceCoverageStatus::ReviewRequired,
        });
    }

    let dependency_signals = signals(
        &lowered,
        &[
            "requires",
            "dependency",
            "dependencies",
            "pip install",
            "npm install",
            "cargo install",
            "brew install",
        ],
    );
    if !dependency_signals.is_empty() {
        classifications.push(SourceClassificationRecord {
            id: format!("class:{}:dependency", record.id),
            target: record.id.clone(),
            kind: SourceClassificationKind::DependencyMention,
            signals: dependency_signals,
            suggested_constructs: vec!["dependency".to_owned(), "deps.toml".to_owned()],
            confidence: "medium".to_owned(),
            reason: "Text mentions dependency or install language.".to_owned(),
            coverage_status: SourceCoverageStatus::ReviewRequired,
        });
    }
}

fn classify_code_text(
    classifications: &mut Vec<SourceClassificationRecord>,
    record: &SourceNodeRecord,
    language: &str,
    text: &str,
) {
    classifications.push(SourceClassificationRecord {
        id: format!("class:{}:code", record.id),
        target: record.id.clone(),
        kind: SourceClassificationKind::CodeBlock,
        signals: (!language.is_empty())
            .then(|| language.to_owned())
            .into_iter()
            .collect(),
        suggested_constructs: vec!["code".to_owned(), "resource".to_owned()],
        confidence: "high".to_owned(),
        reason: "Fenced code block parsed from a large Markdown chunk.".to_owned(),
        coverage_status: SourceCoverageStatus::ReviewRequired,
    });

    let lang = language.to_ascii_lowercase();
    let packages = if matches!(lang.as_str(), "python" | "py") {
        python_imports(text)
    } else if matches!(lang.as_str(), "javascript" | "js" | "typescript" | "ts") {
        javascript_imports(text)
    } else {
        BTreeSet::new()
    };
    for package in packages {
        classifications.push(SourceClassificationRecord {
            id: format!("class:{}:dependency:{package}", record.id),
            target: record.id.clone(),
            kind: SourceClassificationKind::DependencyMention,
            signals: vec![package],
            suggested_constructs: vec!["dependency".to_owned(), "deps.toml".to_owned()],
            confidence: "high".to_owned(),
            reason: "Package import parsed from fenced code chunk.".to_owned(),
            coverage_status: SourceCoverageStatus::ReviewRequired,
        });
    }
}

fn collect_text_references(
    source_root: &Path,
    file: &SourceFileRecord,
    source_id: &str,
    text: &str,
    references: &mut Vec<SourceReferenceRecord>,
) {
    for target in markdown_link_targets(text) {
        let target_kind = if target.starts_with("http://") || target.starts_with("https://") {
            SourceReferenceKind::ExternalUri
        } else if target.starts_with('#') {
            SourceReferenceKind::Anchor
        } else if target.trim().is_empty() {
            SourceReferenceKind::Unknown
        } else {
            SourceReferenceKind::LocalFile
        };
        let resolved_file = (target_kind == SourceReferenceKind::LocalFile)
            .then(|| resolve_local_reference(source_root, &file.path, &target))
            .flatten();
        references.push(SourceReferenceRecord {
            id: format!("ref:{}:{}", source_id, references.len() + 1),
            source: source_id.to_owned(),
            target,
            target_kind,
            resolved_file,
        });
    }
}

fn markdown_link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        targets.push(after[..end].trim().to_owned());
        rest = &after[end + 1..];
    }
    targets
}

fn code_fence(line: &str) -> Option<(String, usize)> {
    let trimmed = line.trim_start();
    for marker in ["```", "~~~"] {
        if trimmed.starts_with(marker) {
            let count = trimmed
                .chars()
                .take_while(|char| *char == marker.chars().next().unwrap())
                .count();
            if count >= 3 {
                let language = trimmed[count..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_owned();
                return Some((language, count));
            }
        }
    }
    None
}

fn heading_title(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let depth = trimmed.chars().take_while(|char| *char == '#').count();
    if !(1..=6).contains(&depth) {
        return None;
    }
    let rest = trimmed[depth..].trim();
    (!rest.is_empty()).then(|| rest.to_owned())
}

fn next_node_id(file_slug: &str, kind: &str, counters: &mut BTreeMap<String, usize>) -> String {
    let counter = counters.entry(kind.to_owned()).or_insert(0);
    *counter += 1;
    format!("{kind}:{file_slug}.{counter}")
}

fn classify_code_dependencies(ctx: &mut ParseContext<'_>, record: &SourceNodeRecord, node: &Value) {
    let value = node
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let lang = node
        .get("lang")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let packages = if matches!(lang.as_str(), "python" | "py") {
        python_imports(value)
    } else if matches!(lang.as_str(), "javascript" | "js" | "typescript" | "ts") {
        javascript_imports(value)
    } else {
        BTreeSet::new()
    };
    for package in packages {
        ctx.classifications.push(SourceClassificationRecord {
            id: format!("class:{}:dependency:{package}", record.id),
            target: record.id.clone(),
            kind: SourceClassificationKind::DependencyMention,
            signals: vec![package],
            suggested_constructs: vec!["dependency".to_owned(), "deps.toml".to_owned()],
            confidence: "high".to_owned(),
            reason: "Package import parsed from fenced code block.".to_owned(),
            coverage_status: SourceCoverageStatus::ReviewRequired,
        });
    }
}

fn collect_references(ctx: &mut ParseContext<'_>, record: &SourceNodeRecord, node: &Value) {
    let Some(url) = node.get("url").and_then(Value::as_str) else {
        return;
    };
    let target_kind = if url.starts_with("http://") || url.starts_with("https://") {
        SourceReferenceKind::ExternalUri
    } else if url.starts_with('#') {
        SourceReferenceKind::Anchor
    } else if url.trim().is_empty() {
        SourceReferenceKind::Unknown
    } else {
        SourceReferenceKind::LocalFile
    };
    let resolved_file = (target_kind == SourceReferenceKind::LocalFile)
        .then(|| resolve_local_reference(ctx.source_root, &ctx.file.path, url))
        .flatten();
    ctx.references.push(SourceReferenceRecord {
        id: format!("ref:{}:{}", record.id, ctx.references.len() + 1),
        source: record.id.clone(),
        target: url.to_owned(),
        target_kind,
        resolved_file,
    });
}

pub(super) fn source_root(source: &Path) -> PathBuf {
    if source.is_file() {
        source
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        source.to_path_buf()
    }
}

fn extract_frontmatter(file: &SourceFileRecord, content: &str) -> Option<SourceNodeRecord> {
    let mut offset = 0usize;
    let mut lines = content.split_inclusive('\n');
    let first = lines.next()?;
    offset += first.len();
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return None;
    }
    for line in lines {
        offset += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let line_count = content[..offset].lines().count();
            let text = source_slice(content, Some([0, offset]));
            return Some(SourceNodeRecord {
                id: format!("frontmatter:{}", file.id.trim_start_matches("file:")),
                file: file.id.clone(),
                kind: "frontmatter".to_owned(),
                depth: None,
                title: Some("YAML frontmatter".to_owned()),
                language: Some("yaml".to_owned()),
                parent: None,
                children: Vec::new(),
                byte_range: Some([0, offset]),
                line_range: Some([1, line_count]),
                hash: Some(sha256_hex(text.as_bytes())),
                text_preview: preview(text),
                coverage_status: SourceCoverageStatus::Mapped,
            });
        }
    }
    None
}

fn file_kind(path: &Path) -> SourceFileKind {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "md" | "markdown") || name == "skill.md" {
        SourceFileKind::Markdown
    } else if matches!(
        extension.as_str(),
        "py" | "js" | "ts" | "tsx" | "jsx" | "rs" | "sh" | "bash" | "zsh"
    ) {
        SourceFileKind::Code
    } else if matches!(
        name.as_str(),
        "package.json" | "pyproject.toml" | "requirements.txt" | "cargo.toml"
    ) {
        SourceFileKind::Manifest
    } else if matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "pdf"
    ) {
        SourceFileKind::Asset
    } else {
        SourceFileKind::Other
    }
}

fn role_candidates(path: &Path, path_string: &str) -> Vec<String> {
    let mut roles = Vec::new();
    if path_string.eq_ignore_ascii_case("SKILL.md") {
        roles.push("source_skill".to_owned());
    }
    if matches!(file_kind(path), SourceFileKind::Markdown) {
        roles.push("runtime_guidance_candidate".to_owned());
    }
    if matches!(file_kind(path), SourceFileKind::Code) {
        roles.push("script_or_code_candidate".to_owned());
    }
    if matches!(file_kind(path), SourceFileKind::Manifest) {
        roles.push("dependency_manifest_candidate".to_owned());
    }
    roles
}

fn node_type(node: &Value) -> Option<String> {
    node.get("type")
        .and_then(Value::as_str)
        .map(|kind| kind.to_ascii_lowercase())
}

fn next_counter(counters: &mut BTreeMap<String, usize>, kind: &str) -> usize {
    let counter = counters.entry(kind.to_owned()).or_insert(0);
    *counter += 1;
    *counter
}

fn node_handle(
    ctx: &mut ParseContext<'_>,
    file_id: &str,
    kind: &str,
    sequence: usize,
    node: &Value,
) -> String {
    let file = file_id.trim_start_matches("file:");
    if kind == "root" {
        return format!("root:{file}");
    }
    if kind == "heading" {
        if let Some(title) = collect_plain_text(node) {
            let slug = slug(&title);
            let key = format!("{file}:{slug}");
            let count = ctx.heading_slugs.entry(key).or_insert(0);
            *count += 1;
            if *count == 1 {
                return format!("heading:{file}.{slug}");
            }
            return format!("heading:{file}.{slug}-{count}");
        }
    }
    format!("{kind}:{file}.{sequence}")
}

fn collect_plain_text(node: &Value) -> Option<String> {
    let mut parts = Vec::new();
    collect_text_parts(node, &mut parts);
    let text = parts.join(" ").trim().to_owned();
    (!text.is_empty()).then_some(text)
}

fn collect_text_parts(node: &Value, parts: &mut Vec<String>) {
    if let Some(value) = node.get("value").and_then(Value::as_str) {
        if !value.trim().is_empty() {
            parts.push(value.trim().to_owned());
        }
    }
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            collect_text_parts(child, parts);
        }
    }
}

fn byte_range(ctx: &ParseContext<'_>, node: &Value) -> Option<[usize; 2]> {
    let position = node.get("position")?;
    let start = position
        .get("start")?
        .get("offset")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let end = position
        .get("end")?
        .get("offset")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    Some([start + ctx.byte_offset, end + ctx.byte_offset])
}

fn line_range(ctx: &ParseContext<'_>, node: &Value) -> Option<[usize; 2]> {
    let position = node.get("position")?;
    let start = position
        .get("start")?
        .get("line")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let end = position
        .get("end")?
        .get("line")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    Some([start + ctx.line_offset, end + ctx.line_offset])
}

fn node_text(node: &Value) -> Option<String> {
    node.get("value")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| collect_plain_text(node))
}

pub(super) fn source_slice(source: &str, range: Option<[usize; 2]>) -> &str {
    let Some([start, end]) = range else {
        return "";
    };
    source.get(start..end).unwrap_or("")
}

fn preview(text: &str) -> Option<String> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        None
    } else if text.chars().count() > 180 {
        let preview = text.chars().take(180).collect::<String>();
        Some(format!("{preview}..."))
    } else {
        Some(text)
    }
}

fn signals(text: &str, candidates: &[&str]) -> Vec<String> {
    candidates
        .iter()
        .filter(|candidate| text.contains(**candidate))
        .map(|candidate| (*candidate).to_owned())
        .collect()
}

fn python_imports(source: &str) -> BTreeSet<String> {
    let mut packages = BTreeSet::new();
    for line in source.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            for part in rest.split(',') {
                if let Some(package) = package_base(part) {
                    packages.insert(package);
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("from ") {
            if let Some(package) = rest.split_whitespace().next().and_then(package_base) {
                packages.insert(package);
            }
        }
    }
    packages
}

fn javascript_imports(source: &str) -> BTreeSet<String> {
    let mut packages = BTreeSet::new();
    for line in source.lines() {
        for marker in [" from ", "require(", "import("] {
            if let Some(package) = quoted_package_after(line, marker) {
                packages.insert(package);
            }
        }
    }
    packages
}

fn quoted_package_after(line: &str, marker: &str) -> Option<String> {
    let start = line.find(marker)? + marker.len();
    let rest = line[start..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &rest[quote.len_utf8()..];
    let end = rest.find(quote)?;
    normalize_js_package(&rest[..end])
}

fn normalize_js_package(specifier: &str) -> Option<String> {
    if specifier.starts_with('.') || specifier.starts_with('/') || specifier.starts_with("node:") {
        return None;
    }
    if specifier.starts_with('@') {
        let mut parts = specifier.split('/');
        Some(format!("{}/{}", parts.next()?, parts.next()?))
    } else {
        package_base(specifier)
    }
}

fn package_base(import: &str) -> Option<String> {
    let base = import
        .split_whitespace()
        .next()
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("")
        .trim();
    (!base.is_empty() && !is_stdlib_or_builtin(base)).then(|| base.to_owned())
}

fn is_stdlib_or_builtin(name: &str) -> bool {
    matches!(
        name,
        "json"
            | "os"
            | "sys"
            | "pathlib"
            | "typing"
            | "subprocess"
            | "re"
            | "fs"
            | "path"
            | "url"
            | "crypto"
            | "process"
    )
}

fn resolve_local_reference(source_root: &Path, file_path: &str, target: &str) -> Option<String> {
    let target = target.split('#').next().unwrap_or(target);
    if target.is_empty() {
        return None;
    }
    let base = Path::new(file_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let candidate = source_root.join(base).join(target);
    if !candidate.exists() {
        return None;
    }
    candidate
        .strip_prefix(source_root)
        .ok()
        .map(path_to_spec_string)
}

fn coverage(
    files: &[SourceFileRecord],
    nodes: &[SourceNodeRecord],
    classifications: &[SourceClassificationRecord],
    stale_files: usize,
) -> SourceCoverage {
    let mut statuses = BTreeMap::<String, usize>::new();
    for node in nodes {
        *statuses
            .entry(node.coverage_status.as_str().to_owned())
            .or_default() += 1;
    }
    for classification in classifications {
        *statuses
            .entry(classification.coverage_status.as_str().to_owned())
            .or_default() += 1;
    }
    let review_required = statuses.get("review_required").copied().unwrap_or(0);
    SourceCoverage {
        total_files: files.len(),
        total_nodes: nodes.len(),
        total_classifications: classifications.len(),
        statuses,
        stale_files,
        review_required,
    }
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "untitled".to_owned()
    } else {
        slug
    }
}

fn file_slug(path: &Path) -> String {
    slug(&path_to_spec_string(path))
}

pub(super) fn path_to_spec_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
