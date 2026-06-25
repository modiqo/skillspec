use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_MAP_SCHEMA: &str = "skillspec-source-map/v0";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceMap {
    pub schema: String,
    pub source_root: String,
    pub generator: SourceMapGenerator,
    pub files: Vec<SourceFileRecord>,
    pub nodes: Vec<SourceNodeRecord>,
    pub classifications: Vec<SourceClassificationRecord>,
    pub references: Vec<SourceReferenceRecord>,
    pub coverage: SourceCoverage,
    pub diagnostics: Vec<SourceDiagnostic>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceMapGenerator {
    pub name: String,
    pub markdown_engine: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceFileRecord {
    pub id: String,
    pub path: String,
    pub kind: SourceFileKind,
    pub sha256: String,
    pub bytes: usize,
    pub lines: usize,
    pub role_candidates: Vec<String>,
    pub load_status: SourceFileLoadStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFileKind {
    Markdown,
    Code,
    Asset,
    Manifest,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFileLoadStatus {
    Loaded,
    BinaryPreserved,
    IgnoredByPolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceNodeRecord {
    pub id: String,
    pub file: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub children: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_range: Option<[usize; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<[usize; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_preview: Option<String>,
    pub coverage_status: SourceCoverageStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoverageStatus {
    Unread,
    Mapped,
    Preserved,
    Promoted,
    NotApplicable,
    ReviewRequired,
    Blocked,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceClassificationRecord {
    pub id: String,
    pub target: String,
    pub kind: SourceClassificationKind,
    pub signals: Vec<String>,
    pub suggested_constructs: Vec<String>,
    pub confidence: String,
    pub reason: String,
    pub coverage_status: SourceCoverageStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClassificationKind {
    ActivationSignal,
    RouteCandidate,
    ModalObligation,
    ForbidCandidate,
    ElicitationCandidate,
    DependencyMention,
    CommandExample,
    CodeBlock,
    ImportCandidate,
    ResourceCandidate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceReferenceRecord {
    pub id: String,
    pub source: String,
    pub target: String,
    pub target_kind: SourceReferenceKind,
    pub resolved_file: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceReferenceKind {
    LocalFile,
    ExternalUri,
    Anchor,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceCoverage {
    pub total_files: usize,
    pub total_nodes: usize,
    pub total_classifications: usize,
    pub statuses: BTreeMap<String, usize>,
    pub stale_files: usize,
    pub review_required: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceDiagnostic {
    pub level: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceMapWriteReport {
    pub source_map: String,
    pub markdown_view: String,
    pub files: usize,
    pub nodes: usize,
    pub classifications: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceStaleReport {
    pub ok: bool,
    pub files: Vec<SourceStaleFile>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceStaleFile {
    pub path: String,
    pub status: String,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceView {
    Index,
    Summary,
    Full,
}

pub fn create_source_map(source: &Path, out_dir: &Path) -> Result<SourceMapWriteReport> {
    let map = build(source)?;
    fs::create_dir_all(out_dir).map_err(|source| Error::Write {
        path: out_dir.to_path_buf(),
        source,
    })?;
    let json_path = out_dir.join("source-map.json");
    let markdown_path = out_dir.join("source-map.md");
    write_json(&json_path, &map)?;
    write_markdown_view(&markdown_path, &map)?;
    Ok(SourceMapWriteReport {
        source_map: json_path.display().to_string(),
        markdown_view: markdown_path.display().to_string(),
        files: map.files.len(),
        nodes: map.nodes.len(),
        classifications: map.classifications.len(),
    })
}

pub fn build(source: &Path) -> Result<SourceMap> {
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

pub fn load(path: &Path) -> Result<SourceMap> {
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&content).map_err(|source| Error::ParseJson {
        path: path.to_path_buf(),
        source,
    })
}

pub fn source_root_for(source: &Path) -> PathBuf {
    source_root(source)
}

pub fn query(path: &Path, handle: &str, view: SourceView) -> Result<Value> {
    let map = load(path)?;
    match handle {
        "files" => Ok(json!(map.files)),
        "nodes" => Ok(nodes_for_view(&map, view)),
        "classifications" => Ok(classifications_for_view(&map, view)),
        "references" => Ok(json!(map.references)),
        "dependencies" => Ok(classifications_by_kind(
            &map,
            SourceClassificationKind::DependencyMention,
            view,
        )),
        "code" => Ok(classifications_by_kind(
            &map,
            SourceClassificationKind::CodeBlock,
            view,
        )),
        "coverage" => Ok(json!(map.coverage)),
        "coverage:review_required" => Ok(review_required(&map)),
        _ => {
            if let Some(file) = map.files.iter().find(|file| file.id == handle) {
                return Ok(json!(file));
            }
            if let Some(node) = map.nodes.iter().find(|node| node.id == handle) {
                return node_for_view(&map, node, view);
            }
            if let Some(classification) = map
                .classifications
                .iter()
                .find(|classification| classification.id == handle)
            {
                return Ok(json!(classification));
            }
            Err(Error::InvalidInput {
                message: format!("unknown source-map handle {handle:?}"),
            })
        }
    }
}

pub fn stale(path: &Path, root: Option<&Path>) -> Result<SourceStaleReport> {
    let map = load(path)?;
    let root = root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(&map.source_root));
    let mut ok = true;
    let mut files = Vec::new();
    for file in &map.files {
        let file_path = root.join(&file.path);
        let actual_sha256 = fs::read(&file_path).ok().map(|bytes| sha256_hex(&bytes));
        let status = match &actual_sha256 {
            None => {
                ok = false;
                "missing"
            }
            Some(actual) if actual == &file.sha256 => "fresh",
            Some(_) => {
                ok = false;
                "stale"
            }
        };
        files.push(SourceStaleFile {
            path: file.path.clone(),
            status: status.to_owned(),
            expected_sha256: file.sha256.clone(),
            actual_sha256,
        });
    }
    Ok(SourceStaleReport { ok, files })
}

pub fn render_query(value: &Value) -> String {
    match serde_json::to_string_pretty(value) {
        Ok(json) => json,
        Err(_) => "<unrenderable source-map query>".to_owned(),
    }
}

pub fn render_write_report(report: &SourceMapWriteReport) -> String {
    format!(
        "ok: wrote source map {}\nview: {}\nfiles: {}\nnodes: {}\nclassifications: {}\nnext: run `skillspec source coverage {}` before import\nnext: query exact handles with `skillspec source query {} <handle> --view full`",
        report.source_map,
        report.markdown_view,
        report.files,
        report.nodes,
        report.classifications,
        report.source_map,
        report.source_map
    )
}

pub fn render_stale(report: &SourceStaleReport) -> String {
    let mut output = String::new();
    output.push_str(if report.ok {
        "source map: fresh\n"
    } else {
        "source map: stale\n"
    });
    for file in &report.files {
        output.push_str(&format!("- {}: {}\n", file.path, file.status));
    }
    output
}

pub fn render_coverage(coverage: &SourceCoverage) -> String {
    let mut output = String::new();
    output.push_str("source-map coverage:\n");
    output.push_str(&format!("- files: {}\n", coverage.total_files));
    output.push_str(&format!("- nodes: {}\n", coverage.total_nodes));
    output.push_str(&format!(
        "- classifications: {}\n",
        coverage.total_classifications
    ));
    output.push_str(&format!(
        "- review_required: {}\n",
        coverage.review_required
    ));
    output.push_str(&format!("- stale_files: {}\n", coverage.stale_files));
    if !coverage.statuses.is_empty() {
        output.push_str("statuses:\n");
        for (status, count) in &coverage.statuses {
            output.push_str(&format!("- {status}: {count}\n"));
        }
    }
    output
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

fn source_root(source: &Path) -> PathBuf {
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

fn source_slice(source: &str, range: Option<[usize; 2]>) -> &str {
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

fn nodes_for_view(map: &SourceMap, view: SourceView) -> Value {
    match view {
        SourceView::Full => json!(map.nodes),
        SourceView::Index => json!(map
            .nodes
            .iter()
            .filter(|node| matches!(
                node.kind.as_str(),
                "frontmatter" | "root" | "heading" | "code" | "html"
            ))
            .map(node_summary)
            .collect::<Vec<_>>()),
        SourceView::Summary => json!(map.nodes.iter().map(node_summary).collect::<Vec<_>>()),
    }
}

fn node_summary(node: &SourceNodeRecord) -> Value {
    json!({
        "id": node.id,
        "kind": node.kind,
        "file": node.file,
        "line_range": node.line_range,
        "title": node.title,
        "language": node.language,
        "coverage_status": node.coverage_status,
        "text_preview": node.text_preview,
    })
}

fn node_for_view(map: &SourceMap, node: &SourceNodeRecord, view: SourceView) -> Result<Value> {
    if view != SourceView::Full {
        return Ok(json!(node));
    }
    let file = map
        .files
        .iter()
        .find(|file| file.id == node.file)
        .ok_or_else(|| Error::InvalidInput {
            message: format!("node {} references unknown file {}", node.id, node.file),
        })?;
    let path = Path::new(&map.source_root).join(&file.path);
    let content = fs::read_to_string(&path).map_err(|source| Error::Read { path, source })?;
    Ok(json!({
        "node": node,
        "source": source_slice(&content, node.byte_range).to_owned(),
    }))
}

fn classifications_for_view(map: &SourceMap, view: SourceView) -> Value {
    match view {
        SourceView::Full => json!(map.classifications),
        SourceView::Index | SourceView::Summary => json!(map
            .classifications
            .iter()
            .map(|classification| classification_summary(map, classification))
            .collect::<Vec<_>>()),
    }
}

fn classifications_by_kind(
    map: &SourceMap,
    kind: SourceClassificationKind,
    view: SourceView,
) -> Value {
    let classifications = map
        .classifications
        .iter()
        .filter(|classification| classification.kind == kind)
        .collect::<Vec<_>>();
    match view {
        SourceView::Full => json!(classifications),
        SourceView::Index | SourceView::Summary => json!(classifications
            .iter()
            .map(|classification| classification_summary(map, classification))
            .collect::<Vec<_>>()),
    }
}

fn classification_summary(map: &SourceMap, classification: &SourceClassificationRecord) -> Value {
    let target = map
        .nodes
        .iter()
        .find(|node| node.id == classification.target);
    json!(
        {
            "id": classification.id,
            "kind": classification.kind,
            "target": classification.target,
            "signals": classification.signals,
            "confidence": classification.confidence,
            "reason": classification.reason,
            "coverage_status": classification.coverage_status,
            "target_line_range": target.and_then(|node| node.line_range),
            "target_preview": target.and_then(|node| node.text_preview.as_deref()),
        }
    )
}

fn review_required(map: &SourceMap) -> Value {
    json!({
        "nodes": map.nodes.iter().filter(|node| node.coverage_status == SourceCoverageStatus::ReviewRequired).collect::<Vec<_>>(),
        "classifications": map.classifications.iter().filter(|classification| classification.coverage_status == SourceCoverageStatus::ReviewRequired).collect::<Vec<_>>(),
    })
}

fn write_json(path: &Path, map: &SourceMap) -> Result<()> {
    let content = serde_json::to_vec_pretty(map)?;
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn write_markdown_view(path: &Path, map: &SourceMap) -> Result<()> {
    let mut output = String::new();
    output.push_str("# SkillSpec Source Map\n\n");
    output.push_str(&format!("- source_root: `{}`\n", map.source_root));
    output.push_str(&format!("- files: {}\n", map.files.len()));
    output.push_str(&format!("- nodes: {}\n", map.nodes.len()));
    output.push_str(&format!(
        "- classifications: {}\n\n",
        map.classifications.len()
    ));
    output.push_str("## Files\n\n");
    for file in &map.files {
        output.push_str(&format!(
            "- `{}` {} lines={} bytes={}\n",
            file.id, file.path, file.lines, file.bytes
        ));
    }
    output.push_str("\n## Review Required\n\n");
    for classification in &map.classifications {
        if classification.coverage_status == SourceCoverageStatus::ReviewRequired {
            output.push_str(&format!(
                "- `{}` {} -> {}\n",
                classification.id,
                classification.kind_string(),
                classification.target
            ));
        }
    }
    fs::write(path, output).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

impl SourceClassificationRecord {
    fn kind_string(&self) -> &'static str {
        self.kind.as_str()
    }
}

impl SourceClassificationKind {
    fn as_str(&self) -> &'static str {
        match self {
            SourceClassificationKind::ActivationSignal => "activation_signal",
            SourceClassificationKind::RouteCandidate => "route_candidate",
            SourceClassificationKind::ModalObligation => "modal_obligation",
            SourceClassificationKind::ForbidCandidate => "forbid_candidate",
            SourceClassificationKind::ElicitationCandidate => "elicitation_candidate",
            SourceClassificationKind::DependencyMention => "dependency_mention",
            SourceClassificationKind::CommandExample => "command_example",
            SourceClassificationKind::CodeBlock => "code_block",
            SourceClassificationKind::ImportCandidate => "import_candidate",
            SourceClassificationKind::ResourceCandidate => "resource_candidate",
        }
    }
}

impl SourceCoverageStatus {
    fn as_str(&self) -> &'static str {
        match self {
            SourceCoverageStatus::Unread => "unread",
            SourceCoverageStatus::Mapped => "mapped",
            SourceCoverageStatus::Preserved => "preserved",
            SourceCoverageStatus::Promoted => "promoted",
            SourceCoverageStatus::NotApplicable => "not_applicable",
            SourceCoverageStatus::ReviewRequired => "review_required",
            SourceCoverageStatus::Blocked => "blocked",
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
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

fn path_to_spec_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
