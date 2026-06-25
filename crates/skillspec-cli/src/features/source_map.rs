use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;

mod builder;
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
    builder::build(source)
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
    builder::source_root(source)
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
        let actual_sha256 = fs::read(&file_path)
            .ok()
            .map(|bytes| builder::sha256_hex(&bytes));
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
        "source": builder::source_slice(&content, node.byte_range).to_owned(),
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
