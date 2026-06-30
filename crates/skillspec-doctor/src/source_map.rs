use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use skillspec_core::error::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

mod builder;
use std::path::{Path, PathBuf};

const SOURCE_MAP_SCHEMA: &str = "skillspec-source-map/v0";
const SOURCE_LENS_SCHEMA: &str = "skillspec/source-review-lens/v0";
const SOURCE_QUERY_SUMMARY_LIMIT: usize = 12;

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
    ConditionalRuleCandidate,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_checkout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
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

#[derive(Clone, Debug, Serialize)]
pub struct SourceLensReport {
    pub schema: String,
    pub source_map: String,
    pub cursor: usize,
    pub limit: usize,
    pub total: usize,
    pub shown: usize,
    pub omitted: usize,
    pub next_cursor: Option<usize>,
    pub units: Vec<SourceLensUnit>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceLensUnit {
    pub id: String,
    pub index: usize,
    pub count: usize,
    pub remaining_after: usize,
    pub source: String,
    pub source_kind: String,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<[usize; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    pub suggested_constructs: Vec<String>,
    pub required_target_kinds: Vec<String>,
    pub classifications: Vec<SourceLensClassification>,
    pub references: Vec<SourceLensReference>,
    pub next: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceLensClassification {
    pub id: String,
    pub kind: String,
    pub signals: Vec<String>,
    pub suggested_constructs: Vec<String>,
    pub confidence: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceLensReference {
    pub id: String,
    pub target: String,
    pub target_kind: String,
    pub resolved_file: Option<String>,
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
        staged_from: None,
        staged_checkout: None,
        source_path: None,
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

pub fn lens(path: &Path, cursor: usize, limit: usize) -> Result<SourceLensReport> {
    if cursor == 0 {
        return Err(Error::InvalidInput {
            message: "source lens cursor is 1-based; pass --cursor 1 or greater".to_owned(),
        });
    }
    if limit == 0 {
        return Err(Error::InvalidInput {
            message: "source lens limit must be greater than zero".to_owned(),
        });
    }

    let map = load(path)?;
    let mut review_nodes = map
        .nodes
        .iter()
        .filter(|node| node_requires_lens_review(node))
        .collect::<Vec<_>>();
    review_nodes.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then_with(|| left.line_range.cmp(&right.line_range))
            .then_with(|| left.id.cmp(&right.id))
    });

    let total = review_nodes.len();
    let start = cursor.saturating_sub(1).min(total);
    let end = start.saturating_add(limit).min(total);
    let units = review_nodes[start..end]
        .iter()
        .enumerate()
        .map(|(offset, node)| lens_unit(&map, node, start + offset + 1, total))
        .collect::<Vec<_>>();
    let shown = units.len();
    Ok(SourceLensReport {
        schema: SOURCE_LENS_SCHEMA.to_owned(),
        source_map: path.display().to_string(),
        cursor,
        limit,
        total,
        shown,
        omitted: total.saturating_sub(end),
        next_cursor: (end < total).then_some(end + 1),
        units,
    })
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
    let mut output = format!(
        "ok: wrote source map {}\nview: {}\nfiles: {}\nnodes: {}\nclassifications: {}\nnext: run `skillspec source coverage {}` before import\nnext: review one block at a time with `skillspec source lens {} --cursor 1`\nnext: query exact handles with `skillspec source query {} <handle> --view full`",
        report.source_map,
        report.markdown_view,
        report.files,
        report.nodes,
        report.classifications,
        report.source_map,
        report.source_map,
        report.source_map
    );
    if let Some(staged_from) = &report.staged_from {
        output.push_str(&format!("\nstaged_from: {staged_from}"));
    }
    if let Some(source_path) = &report.source_path {
        output.push_str(&format!("\nsource_path: {source_path}"));
        output.push_str(&format!(
            "\nnext: run `skillspec import-skill {source_path} --out <draft-dir>/skill.spec.yml --source-map {}`",
            report.source_map
        ));
    }
    output
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

pub fn render_lens(report: &SourceLensReport) -> String {
    let mut output = String::new();
    output.push_str("source review lens:\n");
    output.push_str(&format!("- source_map: {}\n", report.source_map));
    output.push_str(&format!("- cursor: {}\n", report.cursor));
    output.push_str(&format!("- total: {}\n", report.total));
    output.push_str(&format!("- shown: {}\n", report.shown));
    for unit in &report.units {
        output.push_str(&format!(
            "\n## {}/{} {}\n",
            unit.index, unit.count, unit.source
        ));
        output.push_str(&format!("- remaining_after: {}\n", unit.remaining_after));
        output.push_str(&format!("- file: {}\n", unit.file));
        if let Some(line_range) = unit.line_range {
            output.push_str(&format!("- lines: {}-{}\n", line_range[0], line_range[1]));
        }
        if let Some(hash) = &unit.hash {
            output.push_str(&format!("- hash: {hash}\n"));
        }
        if !unit.required_target_kinds.is_empty() {
            output.push_str(&format!(
                "- required_target_kinds: {}\n",
                unit.required_target_kinds.join(", ")
            ));
        }
        if let Some(preview) = &unit.preview {
            output.push_str(&format!("- preview: {}\n", compact_one_line(preview)));
        }
        if !unit.classifications.is_empty() {
            output.push_str("- classifications:\n");
            for classification in &unit.classifications {
                output.push_str(&format!(
                    "  - {} {} -> {}\n",
                    classification.id,
                    classification.kind,
                    classification.suggested_constructs.join(", ")
                ));
            }
        }
        if !unit.references.is_empty() {
            output.push_str("- references:\n");
            for reference in &unit.references {
                output.push_str(&format!(
                    "  - {} {} {}\n",
                    reference.id, reference.target_kind, reference.target
                ));
            }
        }
        output.push_str(&format!("- next: {}\n", unit.next));
    }
    if let Some(next_cursor) = report.next_cursor {
        output.push_str(&format!("\nnext_cursor: {next_cursor}\n"));
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

fn node_requires_lens_review(node: &SourceNodeRecord) -> bool {
    if matches!(node.kind.as_str(), "root" | "frontmatter") {
        return false;
    }
    if node.coverage_status == SourceCoverageStatus::NotApplicable {
        return false;
    }
    node.title
        .as_deref()
        .or(node.text_preview.as_deref())
        .is_some_and(|text| !text.trim().is_empty())
        || node.hash.is_some()
}

fn lens_unit(
    map: &SourceMap,
    node: &SourceNodeRecord,
    index: usize,
    count: usize,
) -> SourceLensUnit {
    let classifications = map
        .classifications
        .iter()
        .filter(|classification| classification.target == node.id)
        .map(|classification| SourceLensClassification {
            id: classification.id.clone(),
            kind: classification.kind.as_str().to_owned(),
            signals: classification.signals.clone(),
            suggested_constructs: classification.suggested_constructs.clone(),
            confidence: classification.confidence.clone(),
            reason: classification.reason.clone(),
        })
        .collect::<Vec<_>>();
    let references = map
        .references
        .iter()
        .filter(|reference| reference.source == node.id)
        .map(|reference| SourceLensReference {
            id: reference.id.clone(),
            target: reference.target.clone(),
            target_kind: reference.target_kind.as_str().to_owned(),
            resolved_file: reference.resolved_file.clone(),
        })
        .collect::<Vec<_>>();
    let mut suggested_constructs = BTreeSet::new();
    let mut required_target_kinds = BTreeSet::new();
    for classification in map
        .classifications
        .iter()
        .filter(|classification| classification.target == node.id)
    {
        for construct in &classification.suggested_constructs {
            suggested_constructs.insert(construct.clone());
            if let Some(kind) = construct_target_kind(construct) {
                required_target_kinds.insert(kind.to_owned());
            }
        }
    }
    for reference in map
        .references
        .iter()
        .filter(|reference| reference.source == node.id)
    {
        if matches!(
            reference.target_kind,
            SourceReferenceKind::LocalFile | SourceReferenceKind::ExternalUri
        ) {
            suggested_constructs.insert("resource".to_owned());
            required_target_kinds.insert("resource".to_owned());
        }
    }

    SourceLensUnit {
        id: format!("lens:{}", node.id),
        index,
        count,
        remaining_after: count.saturating_sub(index),
        source: node.id.clone(),
        source_kind: node.kind.clone(),
        file: node.file.clone(),
        line_range: node.line_range,
        hash: node.hash.clone(),
        title: node.title.clone(),
        preview: node.title.clone().or_else(|| node.text_preview.clone()),
        suggested_constructs: suggested_constructs.into_iter().collect(),
        required_target_kinds: required_target_kinds.into_iter().collect(),
        classifications,
        references,
        next: "port this block into matching SkillSpec constructs, validate, then advance with source lens --cursor <next_cursor>".to_owned(),
    }
}

fn construct_target_kind(construct: &str) -> Option<&'static str> {
    match construct {
        "activation" => Some("activation"),
        "route" => Some("route"),
        "rule" => Some("rule"),
        "dependency" => Some("dependency"),
        "code" => Some("code"),
        "resource" => Some("resource"),
        "import" => Some("import"),
        "command" => Some("command"),
        "elicitation" => Some("elicitation"),
        "closure" => Some("closure"),
        "test" => Some("test"),
        _ => None,
    }
}

fn compact_one_line(value: &str) -> String {
    let mut compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() > 240 {
        compact.truncate(237);
        compact.push_str("...");
    }
    compact
}

fn nodes_for_view(map: &SourceMap, view: SourceView) -> Value {
    match view {
        SourceView::Full => json!(map.nodes),
        SourceView::Index => limited_items(
            map.nodes
                .iter()
                .filter(|node| {
                    matches!(
                        node.kind.as_str(),
                        "frontmatter" | "root" | "heading" | "code" | "html"
                    )
                })
                .map(node_summary)
                .collect::<Vec<_>>(),
        ),
        SourceView::Summary => {
            limited_items(map.nodes.iter().map(node_summary).collect::<Vec<_>>())
        }
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
        SourceView::Index | SourceView::Summary => limited_items(
            map.classifications
                .iter()
                .map(|classification| classification_summary(map, classification))
                .collect::<Vec<_>>(),
        ),
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
        SourceView::Index | SourceView::Summary => limited_items(
            classifications
                .iter()
                .map(|classification| classification_summary(map, classification))
                .collect::<Vec<_>>(),
        ),
    }
}

fn limited_items(items: Vec<Value>) -> Value {
    let total = items.len();
    let shown_items = items
        .into_iter()
        .take(SOURCE_QUERY_SUMMARY_LIMIT)
        .collect::<Vec<_>>();
    let shown = shown_items.len();
    json!({
        "total": total,
        "shown": shown,
        "omitted": total.saturating_sub(shown),
        "items": shown_items,
        "next": "query an exact handle with --view full for source text"
    })
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
            SourceClassificationKind::ConditionalRuleCandidate => "conditional_rule_candidate",
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

impl SourceReferenceKind {
    fn as_str(&self) -> &'static str {
        match self {
            SourceReferenceKind::LocalFile => "local_file",
            SourceReferenceKind::ExternalUri => "external_uri",
            SourceReferenceKind::Anchor => "anchor",
            SourceReferenceKind::Unknown => "unknown",
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
