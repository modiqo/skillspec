use crate::error::{Error, Result};
use crate::model::SkillSpec;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 1;
pub(crate) const DEFAULT_INDEX_FILE: &str = "skill-index.sqlite";
const ROUTER_MANAGED_MARKER: &str = ".skillspec-router-managed";

#[derive(Clone, Debug)]
pub struct IndexOptions {
    pub roots: Vec<PathBuf>,
    pub out: PathBuf,
    pub visibility_manifest: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct RouteOptions {
    pub index: PathBuf,
    pub query: String,
    pub top: usize,
    pub execution_mode: Option<ExecutionMode>,
}

#[derive(Clone, Debug)]
pub struct IndexStatusOptions {
    pub roots: Vec<PathBuf>,
    pub index: PathBuf,
    pub visibility_manifest: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionMode {
    Direct,
    Durable,
}

#[derive(Clone, Debug, Serialize)]
pub struct IndexReport {
    pub index: PathBuf,
    pub roots: Vec<PathBuf>,
    pub skills_indexed: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IndexStatusReport {
    pub index: PathBuf,
    pub exists: bool,
    pub stale: bool,
    pub roots: Vec<PathBuf>,
    pub indexed_skills: usize,
    pub discovered_skills: usize,
    pub new_skills: Vec<IndexStatusEntry>,
    pub changed_skills: Vec<IndexStatusEntry>,
    pub missing_skills: Vec<IndexStatusEntry>,
    pub updated_at_unix: Option<u64>,
    pub warnings: Vec<String>,
    pub advice: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IndexStatusEntry {
    pub name: String,
    pub path: PathBuf,
    pub has_skill_spec: bool,
    pub advice: Option<String>,
}

#[derive(Clone, Copy, Debug)]
enum IndexStatusKind {
    New,
    Changed,
    Missing,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuditReport {
    pub roots: Vec<PathBuf>,
    pub skills: usize,
    pub overlong_descriptions: usize,
    pub vague_descriptions: usize,
    pub missing_negative_boundaries: usize,
    pub duplicate_names: Vec<String>,
    pub warnings: Vec<String>,
    pub entries: Vec<AuditEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuditEntry {
    pub name: String,
    pub path: PathBuf,
    pub description_chars: usize,
    pub visibility: Visibility,
    pub has_skill_spec: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouteReport {
    pub query: String,
    pub decision: RouteDecision,
    pub selected: Option<RouteCandidate>,
    pub candidates: Vec<RouteCandidate>,
    pub bypass_reason: Option<RouteBypassReason>,
    pub decision_reason: String,
    pub elicitation: Option<String>,
    pub execution_mode: Option<ExecutionMode>,
    pub index: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouteCandidate {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub score: f64,
    pub confidence: Confidence,
    pub reason: String,
    pub visibility: Visibility,
    pub has_skill_spec: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteDecision {
    UseSkill,
    Bypass,
    Ambiguous,
}

impl RouteDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::UseSkill => "use_skill",
            Self::Bypass => "bypass",
            Self::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteBypassReason {
    NoCandidates,
    LowConfidence,
    AmbiguousMatch,
}

impl RouteBypassReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::NoCandidates => "no_candidates",
            Self::LowConfidence => "low_confidence",
            Self::AmbiguousMatch => "ambiguous_match",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Implicit,
    ManualOnly,
    NameOnly,
    Off,
}

impl Visibility {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Implicit => "implicit",
            Self::ManualOnly => "manual-only",
            Self::NameOnly => "name-only",
            Self::Off => "off",
        }
    }

    pub(crate) fn from_str(value: &str) -> Self {
        match value {
            "manual-only" => Self::ManualOnly,
            "name-only" => Self::NameOnly,
            "off" => Self::Off,
            _ => Self::Implicit,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SkillEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) skill_dir: PathBuf,
    pub(crate) description: String,
    pub(crate) short_description: Option<String>,
    pub(crate) source: String,
    pub(crate) visibility: Visibility,
    pub(crate) has_skill_spec: bool,
    pub(crate) checksum: String,
    pub(crate) tags: Vec<String>,
    pub(crate) triggers: Vec<String>,
    pub(crate) negative_triggers: Vec<String>,
    pub(crate) text: String,
}

#[derive(Debug)]
struct SkillEntryRow {
    id: String,
    name: String,
    path: String,
    skill_dir: String,
    description: String,
    short_description: Option<String>,
    source: String,
    visibility: String,
    has_skill_spec: bool,
    checksum: String,
    tags_json: String,
    triggers_json: String,
    negative_triggers_json: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    metadata: BTreeMap<String, serde_yaml::Value>,
    #[serde(default, rename = "disable-model-invocation")]
    disable_model_invocation: bool,
}

pub fn index(options: IndexOptions) -> Result<IndexReport> {
    let out = normalize_index_path(options.out);
    let mut warnings = Vec::new();
    let mut entries = scan_roots(&options.roots, &mut warnings)?;
    if let Some(manifest) = &options.visibility_manifest {
        apply_visibility_manifest_overrides(&mut entries, manifest, &mut warnings)?;
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut conn = Connection::open(&out)?;
    create_schema(&conn)?;
    write_entries(&mut conn, &entries)?;
    Ok(IndexReport {
        index: out,
        roots: options.roots,
        skills_indexed: entries.len(),
        warnings,
    })
}

pub fn audit(roots: &[PathBuf]) -> Result<AuditReport> {
    let mut warnings = Vec::new();
    let entries = scan_roots(roots, &mut warnings)?;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &entries {
        *counts.entry(entry.name.clone()).or_default() += 1;
    }
    let duplicate_names = counts
        .into_iter()
        .filter_map(|(name, count)| (count > 1).then_some(name))
        .collect::<Vec<_>>();

    let mut audit_entries = Vec::new();
    let mut overlong_descriptions = 0;
    let mut vague_descriptions = 0;
    let mut missing_negative_boundaries = 0;
    for entry in &entries {
        let mut entry_warnings = Vec::new();
        let description_chars = entry.description.chars().count();
        if description_chars > 800 {
            overlong_descriptions += 1;
            entry_warnings.push("description is over 800 characters".to_owned());
        }
        if is_vague_description(&entry.description) {
            vague_descriptions += 1;
            entry_warnings.push("description is vague for implicit routing".to_owned());
        }
        if entry.negative_triggers.is_empty()
            && !entry.description.to_lowercase().contains("do not use")
        {
            missing_negative_boundaries += 1;
            entry_warnings.push("no negative routing boundary found".to_owned());
        }
        audit_entries.push(AuditEntry {
            name: entry.name.clone(),
            path: entry.path.clone(),
            description_chars,
            visibility: entry.visibility,
            has_skill_spec: entry.has_skill_spec,
            warnings: entry_warnings,
        });
    }

    Ok(AuditReport {
        roots: roots.to_vec(),
        skills: entries.len(),
        overlong_descriptions,
        vague_descriptions,
        missing_negative_boundaries,
        duplicate_names,
        warnings,
        entries: audit_entries,
    })
}

pub fn index_status(options: IndexStatusOptions) -> Result<IndexStatusReport> {
    let index = normalize_index_path(options.index);
    let mut warnings = Vec::new();
    let mut discovered = scan_roots(&options.roots, &mut warnings)?;
    if let Some(manifest) = &options.visibility_manifest {
        apply_visibility_manifest_overrides(&mut discovered, manifest, &mut warnings)?;
    }
    if !index.is_file() {
        let new_skills: Vec<_> = discovered
            .iter()
            .map(|entry| status_entry(entry, IndexStatusKind::New))
            .collect();
        let mut advice = vec![
            "router index is missing; run `skillspec router index refresh` to build it".to_owned(),
        ];
        advice.extend(status_advice(&new_skills, &[], &[]));
        return Ok(IndexStatusReport {
            index,
            exists: false,
            stale: true,
            roots: options.roots,
            indexed_skills: 0,
            discovered_skills: discovered.len(),
            new_skills,
            changed_skills: Vec::new(),
            missing_skills: Vec::new(),
            updated_at_unix: None,
            warnings,
            advice,
        });
    }

    let conn = Connection::open(&index)?;
    let indexed = read_entries(&conn, &index)?;
    let metadata = read_metadata(&conn)?;
    let updated_at_unix = metadata
        .get("updated_at_unix")
        .and_then(|value| value.parse::<u64>().ok());

    let mut new_skills = Vec::new();
    let mut changed_skills = Vec::new();
    for discovered_entry in &discovered {
        match indexed
            .iter()
            .find(|indexed_entry| same_path(&indexed_entry.path, &discovered_entry.path))
        {
            Some(indexed_entry)
                if indexed_entry.checksum != discovered_entry.checksum
                    || indexed_entry.visibility != discovered_entry.visibility =>
            {
                changed_skills.push(status_entry(discovered_entry, IndexStatusKind::Changed));
            }
            Some(_) => {}
            None => new_skills.push(status_entry(discovered_entry, IndexStatusKind::New)),
        }
    }

    let mut missing_skills = Vec::new();
    for indexed_entry in &indexed {
        if !discovered
            .iter()
            .any(|discovered_entry| same_path(&indexed_entry.path, &discovered_entry.path))
        {
            missing_skills.push(status_entry(indexed_entry, IndexStatusKind::Missing));
        }
    }

    let stale = !new_skills.is_empty() || !changed_skills.is_empty() || !missing_skills.is_empty();
    let advice = status_advice(&new_skills, &changed_skills, &missing_skills);
    Ok(IndexStatusReport {
        index,
        exists: true,
        stale,
        roots: options.roots,
        indexed_skills: indexed.len(),
        discovered_skills: discovered.len(),
        new_skills,
        changed_skills,
        missing_skills,
        updated_at_unix,
        warnings,
        advice,
    })
}

pub fn route(options: RouteOptions) -> Result<RouteReport> {
    let index = normalize_index_path(options.index);
    let conn = Connection::open(&index)?;
    let entries = read_entries(&conn, &index)?;
    let candidates = score_candidates(&entries, &options.query, options.top);
    let match_decision = decide_candidate_match(&candidates);
    let selected = match_decision.selected;
    let elicitation = if options.execution_mode.is_none() && selected.is_some() {
        Some("execution_mode_direct_or_durable".to_owned())
    } else {
        None
    };
    Ok(RouteReport {
        query: options.query,
        decision: match_decision.decision,
        selected,
        candidates,
        bypass_reason: match_decision.bypass_reason,
        decision_reason: match_decision.decision_reason,
        elicitation,
        execution_mode: options.execution_mode,
        index,
    })
}

pub(crate) fn normalize_index_path(path: PathBuf) -> PathBuf {
    let path_text = path.as_os_str().to_string_lossy();
    if path.is_dir() || path_text.ends_with(std::path::MAIN_SEPARATOR) {
        path.join(DEFAULT_INDEX_FILE)
    } else {
        path
    }
}

pub fn render_index(report: &IndexReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router index\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Skills indexed: {}\n", report.skills_indexed));
    if !report.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output
}

pub fn render_index_status(report: &IndexStatusReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router index status\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Exists: {}\n", report.exists));
    output.push_str(&format!("Stale: {}\n", report.stale));
    output.push_str(&format!("Indexed skills: {}\n", report.indexed_skills));
    output.push_str(&format!(
        "Discovered skills: {}\n",
        report.discovered_skills
    ));
    if let Some(updated_at_unix) = report.updated_at_unix {
        output.push_str(&format!("Updated at unix: {updated_at_unix}\n"));
    }
    render_status_entries(&mut output, "New skills", &report.new_skills);
    render_status_entries(&mut output, "Changed skills", &report.changed_skills);
    render_status_entries(&mut output, "Missing skills", &report.missing_skills);
    if !report.advice.is_empty() {
        output.push_str("\nAdvice:\n");
        for advice in &report.advice {
            output.push_str(&format!("- {advice}\n"));
        }
    }
    if !report.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output
}

fn render_status_entries(output: &mut String, label: &str, entries: &[IndexStatusEntry]) {
    if entries.is_empty() {
        return;
    }
    output.push_str(&format!("\n{label}:\n"));
    for entry in entries {
        let kind = if entry.has_skill_spec {
            "skillspec"
        } else {
            "prose"
        };
        output.push_str(&format!(
            "- {} [{}]: {}\n",
            entry.name,
            kind,
            entry.path.display()
        ));
        if let Some(advice) = &entry.advice {
            output.push_str(&format!("  advice: {advice}\n"));
        }
    }
}

pub fn render_audit(report: &AuditReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router audit\n\n");
    output.push_str(&format!("Skills: {}\n", report.skills));
    output.push_str(&format!(
        "Overlong descriptions: {}\n",
        report.overlong_descriptions
    ));
    output.push_str(&format!(
        "Vague descriptions: {}\n",
        report.vague_descriptions
    ));
    output.push_str(&format!(
        "Missing negative boundaries: {}\n",
        report.missing_negative_boundaries
    ));
    if !report.duplicate_names.is_empty() {
        output.push_str("\nDuplicate names:\n");
        for name in &report.duplicate_names {
            output.push_str(&format!("- {name}\n"));
        }
    }
    if !report.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output
}

fn status_entry(entry: &SkillEntry, kind: IndexStatusKind) -> IndexStatusEntry {
    IndexStatusEntry {
        name: entry.name.clone(),
        path: entry.path.clone(),
        has_skill_spec: entry.has_skill_spec,
        advice: Some(status_entry_advice(entry, kind)),
    }
}

fn status_advice(
    new_skills: &[IndexStatusEntry],
    changed_skills: &[IndexStatusEntry],
    missing_skills: &[IndexStatusEntry],
) -> Vec<String> {
    let mut advice = Vec::new();
    if new_skills
        .iter()
        .chain(changed_skills.iter())
        .any(|entry| !entry.has_skill_spec)
    {
        advice.push(
            "prose skill detected outside SkillSpec; run `skillspec import-skill <skill-folder> --out <skill-folder>/skill.spec.yml`, review, validate, and test it"
                .to_owned(),
        );
    }
    if !new_skills.is_empty() || !changed_skills.is_empty() {
        advice.push(
            "run `skillspec router index refresh` to apply router-managed explicit invocation controls and rebuild the routing index"
                .to_owned(),
        );
    }
    if !missing_skills.is_empty() {
        advice.push(
            "run `skillspec router index refresh` to remove missing skills from the routing index, or restore the missing skill folders"
                .to_owned(),
        );
    }
    advice
}

fn status_entry_advice(entry: &SkillEntry, kind: IndexStatusKind) -> String {
    match kind {
        IndexStatusKind::New | IndexStatusKind::Changed if entry.has_skill_spec => {
            "SkillSpec-backed skill detected outside the router workflow; refresh will apply explicit invocation controls and index it".to_owned()
        }
        IndexStatusKind::New | IndexStatusKind::Changed => {
            "prose skill detected outside SkillSpec; convert with `skillspec import-skill` and refresh will still apply explicit invocation controls and index it".to_owned()
        }
        IndexStatusKind::Missing => {
            "skill is indexed but missing from current roots; refresh will remove it from the routing index".to_owned()
        }
    }
}

pub fn render_route(report: &RouteReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router route\n\n");
    output.push_str(&format!("Query: {}\n", report.query));
    output.push_str(&format!("Decision: {}\n", report.decision.as_str()));
    output.push_str(&format!("Reason: {}\n", report.decision_reason));
    if let Some(reason) = report.bypass_reason {
        output.push_str(&format!("Bypass reason: {}\n", reason.as_str()));
    }
    if let Some(selected) = &report.selected {
        output.push_str(&format!(
            "Selected: {} ({:.3})\n",
            selected.name, selected.score
        ));
        output.push_str(&format!("Path: {}\n", selected.path.display()));
        output.push_str(&format!("Confidence: {:?}\n", selected.confidence).to_lowercase());
        output.push_str(&format!("Reason: {}\n", selected.reason));
    } else {
        output.push_str("Selected: none\n");
    }
    if let Some(elicitation) = &report.elicitation {
        output.push_str(&format!("Elicitation: {elicitation}\n"));
    }
    if !report.candidates.is_empty() {
        output.push_str("\nCandidates:\n");
        for candidate in &report.candidates {
            output.push_str(&format!(
                "- {} {:.3}: {}\n",
                candidate.name,
                candidate.score,
                candidate.path.display()
            ));
        }
    }
    output
}

struct MatchDecision {
    decision: RouteDecision,
    selected: Option<RouteCandidate>,
    bypass_reason: Option<RouteBypassReason>,
    decision_reason: String,
}

fn decide_candidate_match(candidates: &[RouteCandidate]) -> MatchDecision {
    let Some(top) = candidates.first() else {
        return MatchDecision {
            decision: RouteDecision::Bypass,
            selected: None,
            bypass_reason: Some(RouteBypassReason::NoCandidates),
            decision_reason: "no indexed skill produced a positive route score".to_owned(),
        };
    };
    if top.confidence == Confidence::High {
        return MatchDecision {
            decision: RouteDecision::UseSkill,
            selected: Some(top.clone()),
            bypass_reason: None,
            decision_reason: "top candidate passed the high-confidence match gate".to_owned(),
        };
    }
    if let Some(second) = candidates.get(1) {
        if top.score >= 8.0 && second.score > 0.0 && top.score <= second.score * 1.10 {
            return MatchDecision {
                decision: RouteDecision::Ambiguous,
                selected: None,
                bypass_reason: Some(RouteBypassReason::AmbiguousMatch),
                decision_reason: "top candidates are too close for automatic skill selection"
                    .to_owned(),
            };
        }
    }
    MatchDecision {
        decision: RouteDecision::Bypass,
        selected: None,
        bypass_reason: Some(RouteBypassReason::LowConfidence),
        decision_reason: "best candidate did not pass the automatic match gate".to_owned(),
    }
}

pub(crate) fn scan_roots(roots: &[PathBuf], warnings: &mut Vec<String>) -> Result<Vec<SkillEntry>> {
    let mut entries = Vec::new();
    let mut seen_paths = BTreeSet::new();
    for (root_index, root) in roots.iter().enumerate() {
        if !root.exists() {
            warnings.push(format!("root does not exist: {}", root.display()));
            continue;
        }
        collect_skills(
            root,
            root,
            root_index,
            &mut seen_paths,
            &mut entries,
            warnings,
        )?;
    }
    disambiguate_duplicate_ids(&mut entries);
    entries.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    Ok(entries)
}

fn collect_skills(
    root: &Path,
    dir: &Path,
    root_index: usize,
    seen_paths: &mut BTreeSet<PathBuf>,
    entries: &mut Vec<SkillEntry>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let skill_path = dir.join("SKILL.md");
    if skill_path.is_file() {
        let canonical = fs::canonicalize(&skill_path).unwrap_or_else(|_| skill_path.clone());
        if seen_paths.insert(canonical) {
            match read_skill_entry(root, root_index, &skill_path) {
                Ok(entry) => entries.push(entry),
                Err(error) => warnings.push(format!("skipped {}: {error}", skill_path.display())),
            }
        }
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })? {
        let path = entry
            .map_err(|source| Error::Read {
                path: dir.to_path_buf(),
                source,
            })?
            .path();
        if path.is_dir() {
            collect_skills(root, &path, root_index, seen_paths, entries, warnings)?;
        }
    }
    Ok(())
}

fn read_skill_entry(root: &Path, root_index: usize, skill_path: &Path) -> Result<SkillEntry> {
    let body = fs::read_to_string(skill_path).map_err(|source| Error::Read {
        path: skill_path.to_path_buf(),
        source,
    })?;
    let frontmatter = parse_frontmatter(skill_path, &body)?;
    let skill_dir = skill_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let spec_path = skill_dir.join("skill.spec.yml");
    let spec = if spec_path.is_file() {
        crate::parser::load_spec(&spec_path).ok()
    } else {
        None
    };
    let openai_visibility = read_openai_visibility(&skill_dir)?;
    let claude_visibility = read_claude_visibility(&skill_dir, &frontmatter.name)?;
    let visibility = if let Some(visibility) = claude_visibility {
        visibility
    } else if frontmatter.disable_model_invocation {
        Visibility::ManualOnly
    } else {
        openai_visibility.unwrap_or(Visibility::Implicit)
    };
    let short_description = frontmatter
        .metadata
        .get("short-description")
        .and_then(serde_yaml::Value::as_str)
        .map(ToOwned::to_owned);
    let mut tags = Vec::new();
    let mut triggers = Vec::new();
    let mut negative_triggers = Vec::new();
    extract_routing_metadata(
        &frontmatter.metadata,
        &mut tags,
        &mut triggers,
        &mut negative_triggers,
    );
    if let Some(spec) = &spec {
        enrich_from_spec(spec, &mut tags, &mut triggers, &mut negative_triggers);
    }
    let mut text_parts = vec![
        frontmatter.name.clone(),
        frontmatter.name.replace('-', " "),
        frontmatter.description.clone(),
    ];
    if let Some(short) = &short_description {
        text_parts.push(short.clone());
    }
    text_parts.extend(tags.iter().cloned());
    text_parts.extend(triggers.iter().cloned());
    if let Some(spec) = &spec {
        text_parts.push(spec.title.clone());
        text_parts.push(spec.description.clone());
    }
    let entry_id = entry_id(root, root_index, skill_path, &frontmatter.name);
    let checksum = checksum(&body);
    Ok(SkillEntry {
        id: entry_id,
        name: frontmatter.name,
        path: skill_path.to_path_buf(),
        skill_dir,
        description: frontmatter.description,
        short_description,
        source: format!("root-{root_index}:{}", root.display()),
        visibility,
        has_skill_spec: spec.is_some(),
        checksum,
        tags,
        triggers,
        negative_triggers,
        text: text_parts.join("\n"),
    })
}

fn parse_frontmatter(path: &Path, body: &str) -> Result<SkillFrontmatter> {
    let Some(rest) = body.strip_prefix("---") else {
        return Err(Error::InvalidInput {
            message: format!("missing YAML frontmatter in {}", path.display()),
        });
    };
    let Some((frontmatter, _)) = rest.split_once("\n---") else {
        return Err(Error::InvalidInput {
            message: format!("unterminated YAML frontmatter in {}", path.display()),
        });
    };
    serde_yaml::from_str(frontmatter).map_err(|source| Error::ParseYaml {
        path: path.to_path_buf(),
        source,
    })
}

fn read_openai_visibility(skill_dir: &Path) -> Result<Option<Visibility>> {
    let path = skill_dir.join("agents/openai.yaml");
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })?;
    let value: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|source| Error::ParseYaml {
            path: path.clone(),
            source,
        })?;
    let allow_implicit = value
        .get("policy")
        .and_then(|policy| policy.get("allow_implicit_invocation"))
        .and_then(serde_yaml::Value::as_bool);
    Ok(match allow_implicit {
        Some(false) => Some(Visibility::ManualOnly),
        Some(true) => Some(Visibility::Implicit),
        None => None,
    })
}

fn apply_visibility_manifest_overrides(
    entries: &mut [SkillEntry],
    manifest_path: &Path,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let text = fs::read_to_string(manifest_path).map_err(|source| Error::Read {
        path: manifest_path.to_path_buf(),
        source,
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|source| Error::ParseJson {
            path: manifest_path.to_path_buf(),
            source,
        })?;
    let Some(changes) = value.get("changes").and_then(serde_json::Value::as_array) else {
        return Err(Error::InvalidInput {
            message: format!(
                "visibility manifest {} is missing changes[]",
                manifest_path.display()
            ),
        });
    };

    for change in changes {
        let Some(visibility_text) = change
            .get("after_visibility")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                change
                    .get("after")
                    .and_then(|after| after.get("visibility"))
                    .and_then(serde_json::Value::as_str)
            })
        else {
            warnings.push("visibility manifest change is missing after_visibility".to_owned());
            continue;
        };
        let visibility = Visibility::from_str(visibility_text);
        let skill_dir = change
            .get("skill_dir")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from);
        let skill_file = change
            .get("skill_file")
            .or_else(|| change.get("path"))
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from);
        let skill_name = change
            .get("skill")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);

        let mut matched = false;
        for entry in entries.iter_mut() {
            let path_match = skill_dir
                .as_ref()
                .is_some_and(|path| same_path(path, &entry.skill_dir))
                || skill_file
                    .as_ref()
                    .is_some_and(|path| same_path(path, &entry.path));
            let name_match = skill_dir.is_none()
                && skill_file.is_none()
                && skill_name.as_ref() == Some(&entry.name);
            if path_match || name_match {
                entry.visibility = visibility;
                matched = true;
            }
        }
        if !matched {
            let label = skill_name.unwrap_or_else(|| "<unknown>".to_owned());
            warnings.push(format!(
                "visibility manifest references missing skill: {label}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn read_claude_visibility(skill_dir: &Path, skill_name: &str) -> Result<Option<Visibility>> {
    let Some(settings_path) = claude_settings_path(skill_dir) else {
        return Ok(None);
    };
    if !settings_path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&settings_path).map_err(|source| Error::Read {
        path: settings_path.clone(),
        source,
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|source| Error::ParseJson {
            path: settings_path.clone(),
            source,
        })?;
    let Some(state) = value
        .get("skillOverrides")
        .and_then(|overrides| overrides.get(skill_name))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };
    Ok(match state {
        "on" => Some(Visibility::Implicit),
        "name-only" => Some(Visibility::NameOnly),
        "user-invocable-only" => Some(Visibility::ManualOnly),
        "off" => Some(Visibility::Off),
        _ => None,
    })
}

pub(crate) fn claude_settings_path(skill_dir: &Path) -> Option<PathBuf> {
    for ancestor in skill_dir.ancestors() {
        if ancestor.file_name().and_then(|name| name.to_str()) == Some(".claude") {
            return Some(ancestor.join("settings.json"));
        }
    }
    None
}

fn extract_routing_metadata(
    metadata: &BTreeMap<String, serde_yaml::Value>,
    tags: &mut Vec<String>,
    triggers: &mut Vec<String>,
    negative_triggers: &mut Vec<String>,
) {
    let Some(routing) = metadata.get("routing") else {
        return;
    };
    extend_string_list(tags, routing.get("tags"));
    extend_string_list(triggers, routing.get("triggers"));
    extend_string_list(negative_triggers, routing.get("negative_triggers"));
}

fn enrich_from_spec(
    spec: &SkillSpec,
    tags: &mut Vec<String>,
    triggers: &mut Vec<String>,
    negative_triggers: &mut Vec<String>,
) {
    tags.push(spec.id.clone());
    if let Some(activation) = &spec.activation {
        triggers.push(activation.summary.clone());
        triggers.extend(activation.keywords.iter().cloned());
        if let Some(priority) = &activation.priority {
            tags.push(priority.clone());
        }
    }
    for route in &spec.routes {
        tags.push(route.id.0.clone());
        triggers.push(route.label.clone());
        if let Some(description) = &route.description {
            triggers.push(description.clone());
        }
    }
    for rule in &spec.rules {
        if let Some(reason) = &rule.reason {
            triggers.push(reason.clone());
        }
        negative_triggers.extend(rule.forbid.iter().cloned());
    }
}

fn extend_string_list(target: &mut Vec<String>, value: Option<&serde_yaml::Value>) {
    match value {
        Some(serde_yaml::Value::Sequence(values)) => {
            target.extend(
                values
                    .iter()
                    .filter_map(serde_yaml::Value::as_str)
                    .map(ToOwned::to_owned),
            );
        }
        Some(serde_yaml::Value::String(value)) => target.push(value.clone()),
        _ => {}
    }
}

fn checksum(text: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(text.as_bytes()))
}

fn entry_id(root: &Path, root_index: usize, skill_path: &Path, name: &str) -> String {
    let relative = skill_path
        .strip_prefix(root)
        .unwrap_or(skill_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let slug = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .flat_map(|component| component.split(|char: char| !char.is_ascii_alphanumeric()))
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() || slug == name {
        name.to_owned()
    } else {
        format!("root{root_index}-{slug}")
    }
}

fn disambiguate_duplicate_ids(entries: &mut [SkillEntry]) {
    let mut counts = BTreeMap::<String, usize>::new();
    for entry in entries.iter() {
        *counts.entry(entry.id.clone()).or_default() += 1;
    }
    for entry in entries {
        if counts.get(&entry.id).copied().unwrap_or_default() > 1 {
            entry.id = format!("{}-{}", entry.id, path_fingerprint(&entry.path));
        }
    }
}

fn path_fingerprint(path: &Path) -> String {
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let hex = format!("{:x}", Sha256::digest(path.to_string_lossy().as_bytes()));
    hex.chars().take(12).collect()
}

fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            skill_dir TEXT NOT NULL,
            description TEXT NOT NULL,
            short_description TEXT,
            source TEXT NOT NULL,
            visibility TEXT NOT NULL,
            has_skill_spec INTEGER NOT NULL,
            checksum TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            triggers_json TEXT NOT NULL,
            negative_triggers_json TEXT NOT NULL,
            text TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS skills_name_idx ON skills(name);
        ",
    )?;
    Ok(())
}

fn write_entries(conn: &mut Connection, entries: &[SkillEntry]) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM skills", [])?;
    tx.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', ?1)",
        [SCHEMA_VERSION.to_string()],
    )?;
    tx.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('updated_at_unix', ?1)",
        [now_unix().to_string()],
    )?;
    for entry in entries {
        tx.execute(
            "INSERT OR REPLACE INTO skills
            (id, name, path, skill_dir, description, short_description, source, visibility, has_skill_spec, checksum, tags_json, triggers_json, negative_triggers_json, text)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                entry.id,
                entry.name,
                entry.path.to_string_lossy(),
                entry.skill_dir.to_string_lossy(),
                entry.description,
                entry.short_description,
                entry.source,
                entry.visibility.as_str(),
                if entry.has_skill_spec { 1_i64 } else { 0_i64 },
                entry.checksum,
                serde_json::to_string(&entry.tags)?,
                serde_json::to_string(&entry.triggers)?,
                serde_json::to_string(&entry.negative_triggers)?,
                entry.text,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn read_entries(conn: &Connection, index_path: &Path) -> Result<Vec<SkillEntry>> {
    let mut statement = conn.prepare(
        "SELECT id, name, path, skill_dir, description, short_description, source, visibility,
                has_skill_spec, checksum, tags_json, triggers_json, negative_triggers_json, text
         FROM skills",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(SkillEntryRow {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            skill_dir: row.get(3)?,
            description: row.get(4)?,
            short_description: row.get(5)?,
            source: row.get(6)?,
            visibility: row.get(7)?,
            has_skill_spec: row.get::<_, i64>(8)? != 0,
            checksum: row.get(9)?,
            tags_json: row.get(10)?,
            triggers_json: row.get(11)?,
            negative_triggers_json: row.get(12)?,
            text: row.get(13)?,
        })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        let row = row?;
        entries.push(SkillEntry {
            id: row.id,
            name: row.name,
            path: PathBuf::from(row.path),
            skill_dir: PathBuf::from(row.skill_dir),
            description: row.description,
            short_description: row.short_description,
            source: row.source,
            visibility: Visibility::from_str(&row.visibility),
            has_skill_spec: row.has_skill_spec,
            checksum: row.checksum,
            tags: parse_index_json(index_path, "tags_json", &row.tags_json)?,
            triggers: parse_index_json(index_path, "triggers_json", &row.triggers_json)?,
            negative_triggers: parse_index_json(
                index_path,
                "negative_triggers_json",
                &row.negative_triggers_json,
            )?,
            text: row.text,
        });
    }
    Ok(entries)
}

fn read_metadata(conn: &Connection) -> Result<BTreeMap<String, String>> {
    let mut statement = conn.prepare("SELECT key, value FROM metadata")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut metadata = BTreeMap::new();
    for row in rows {
        let (key, value) = row?;
        metadata.insert(key, value);
    }
    Ok(metadata)
}

fn score_candidates(entries: &[SkillEntry], query: &str, top: usize) -> Vec<RouteCandidate> {
    if top == 0 {
        return Vec::new();
    }
    let query_terms = tokenize(query);
    if query_terms.is_empty() {
        return Vec::new();
    }
    let routable_entries = entries
        .iter()
        .filter(|entry| !is_managed_router_entry(entry))
        .collect::<Vec<_>>();
    if routable_entries.is_empty() {
        return Vec::new();
    }
    let docs = routable_entries
        .iter()
        .map(|entry| tokenize(&entry.text))
        .collect::<Vec<_>>();
    let avg_len = docs.iter().map(Vec::len).sum::<usize>().max(1) as f64 / docs.len().max(1) as f64;
    let mut document_frequency: HashMap<&str, usize> = HashMap::new();
    for term in &query_terms {
        let count = docs
            .iter()
            .filter(|doc| doc.iter().any(|doc_term| doc_term == term))
            .count();
        document_frequency.insert(term, count);
    }
    let mut candidates = routable_entries
        .iter()
        .zip(docs.iter())
        .filter(|(entry, _)| entry.visibility != Visibility::Off)
        .map(|(entry, doc)| {
            let mut score = bm25_score(
                doc,
                &query_terms,
                &document_frequency,
                routable_entries.len(),
                avg_len,
            );
            let query_lower = query.to_lowercase();
            if query_lower.contains(&entry.name.to_lowercase()) {
                score += 8.0;
            }
            for trigger in &entry.triggers {
                if !trigger.is_empty() && query_lower.contains(&trigger.to_lowercase()) {
                    score += 3.0;
                }
            }
            for negative in &entry.negative_triggers {
                if !negative.is_empty() && query_lower.contains(&negative.to_lowercase()) {
                    score -= 5.0;
                }
            }
            let reason = reason(entry, score);
            RouteCandidate {
                id: entry.id.clone(),
                name: entry.name.clone(),
                path: entry.path.clone(),
                score,
                confidence: Confidence::Low,
                reason,
                visibility: entry.visibility,
                has_skill_spec: entry.has_skill_spec,
            }
        })
        .filter(|candidate| candidate.score > 0.0)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.name.cmp(&right.name))
    });
    let top_score = candidates
        .first()
        .map(|candidate| candidate.score)
        .unwrap_or(0.0);
    let second_score = candidates
        .get(1)
        .map(|candidate| candidate.score)
        .unwrap_or(0.0);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.confidence = if index == 0 && top_score >= 8.0 && top_score >= second_score * 1.25
        {
            Confidence::High
        } else if candidate.score >= 3.0 {
            Confidence::Medium
        } else {
            Confidence::Low
        };
    }
    candidates.truncate(top);
    candidates
}

fn is_managed_router_entry(entry: &SkillEntry) -> bool {
    entry.skill_dir.join(ROUTER_MANAGED_MARKER).is_file()
}

fn parse_index_json(index_path: &Path, column: &str, json: &str) -> Result<Vec<String>> {
    serde_json::from_str(json).map_err(|source| Error::InvalidInput {
        message: format!(
            "invalid {column} in skill router index {}: {source}",
            index_path.display()
        ),
    })
}

fn bm25_score(
    doc: &[String],
    query_terms: &[String],
    document_frequency: &HashMap<&str, usize>,
    document_count: usize,
    avg_len: f64,
) -> f64 {
    let mut term_counts: HashMap<&str, usize> = HashMap::new();
    for term in doc {
        *term_counts.entry(term.as_str()).or_default() += 1;
    }
    let k1 = 1.2;
    let b = 0.75;
    let doc_len = doc.len().max(1) as f64;
    let mut score = 0.0;
    for term in query_terms {
        let tf = *term_counts.get(term.as_str()).unwrap_or(&0) as f64;
        if tf == 0.0 {
            continue;
        }
        let df = *document_frequency.get(term.as_str()).unwrap_or(&0) as f64;
        let idf = (((document_count as f64 - df + 0.5) / (df + 0.5)) + 1.0).ln();
        score += idf * ((tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * doc_len / avg_len)));
    }
    score
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|char: char| !char.is_ascii_alphanumeric())
        .filter(|term| term.len() > 1)
        .map(|term| term.to_ascii_lowercase())
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .collect()
}

const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "with", "when", "use", "this", "that", "from", "into", "skill", "skills",
    "task", "user", "asks", "needs", "should", "will", "can", "are", "was", "were", "has", "have",
];

fn reason(entry: &SkillEntry, score: f64) -> String {
    let mut parts = Vec::new();
    if entry.has_skill_spec {
        parts.push("SkillSpec-backed");
    }
    if entry.visibility == Visibility::ManualOnly {
        parts.push("manual-only");
    }
    if score >= 8.0 {
        parts.push("strong lexical match");
    } else if score >= 3.0 {
        parts.push("lexical match");
    }
    if parts.is_empty() {
        "weak lexical match".to_owned()
    } else {
        parts.join(", ")
    }
}

fn is_vague_description(description: &str) -> bool {
    let normalized = description.trim().to_ascii_lowercase();
    normalized.len() < 40
        || matches!(
            normalized.as_str(),
            "helps with skills" | "use this skill" | "general purpose skill"
        )
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_prefers_exact_skill_name() {
        let entries = vec![
            SkillEntry {
                id: "pdf".to_owned(),
                name: "pdf".to_owned(),
                path: PathBuf::from("/tmp/pdf/SKILL.md"),
                skill_dir: PathBuf::from("/tmp/pdf"),
                description: "Use for PDF documents.".to_owned(),
                short_description: None,
                source: "test".to_owned(),
                visibility: Visibility::Implicit,
                has_skill_spec: false,
                checksum: "sha256:test".to_owned(),
                tags: Vec::new(),
                triggers: vec!["extract PDF text".to_owned()],
                negative_triggers: Vec::new(),
                text: "pdf PDF documents extract text".to_owned(),
            },
            SkillEntry {
                id: "deploy".to_owned(),
                name: "deploy".to_owned(),
                path: PathBuf::from("/tmp/deploy/SKILL.md"),
                skill_dir: PathBuf::from("/tmp/deploy"),
                description: "Use for deployment.".to_owned(),
                short_description: None,
                source: "test".to_owned(),
                visibility: Visibility::Implicit,
                has_skill_spec: false,
                checksum: "sha256:test".to_owned(),
                tags: Vec::new(),
                triggers: Vec::new(),
                negative_triggers: Vec::new(),
                text: "deploy application production".to_owned(),
            },
        ];
        let candidates = score_candidates(&entries, "use pdf to extract text", 3);
        assert_eq!(candidates[0].name, "pdf");
    }

    #[test]
    fn route_ignores_managed_router_skill() {
        let router_dir = std::env::temp_dir().join(format!(
            "skillspec-router-test-{}-{}",
            std::process::id(),
            now_unix()
        ));
        fs::create_dir_all(&router_dir).unwrap();
        fs::write(router_dir.join(ROUTER_MANAGED_MARKER), "managed").unwrap();
        let entries = vec![
            SkillEntry {
                id: "skill-router".to_owned(),
                name: "skill-router".to_owned(),
                path: router_dir.join("SKILL.md"),
                skill_dir: router_dir.clone(),
                description: "Use for every request, tell me, explain, what is, help with."
                    .to_owned(),
                short_description: None,
                source: "test".to_owned(),
                visibility: Visibility::Implicit,
                has_skill_spec: true,
                checksum: "sha256:test".to_owned(),
                tags: Vec::new(),
                triggers: vec!["what is".to_owned()],
                negative_triggers: Vec::new(),
                text: "skill router every request tell me explain what is help with".to_owned(),
            },
            SkillEntry {
                id: "notes".to_owned(),
                name: "notes".to_owned(),
                path: PathBuf::from("/tmp/notes/SKILL.md"),
                skill_dir: PathBuf::from("/tmp/notes"),
                description: "Use for notes.".to_owned(),
                short_description: None,
                source: "test".to_owned(),
                visibility: Visibility::Implicit,
                has_skill_spec: false,
                checksum: "sha256:test".to_owned(),
                tags: Vec::new(),
                triggers: Vec::new(),
                negative_triggers: Vec::new(),
                text: "notes meeting action items".to_owned(),
            },
        ];

        let candidates = score_candidates(&entries, "what is the time today", 5);

        assert!(candidates
            .iter()
            .all(|candidate| candidate.name != "skill-router"));
        let _ = fs::remove_dir_all(router_dir);
    }

    #[test]
    fn match_gate_uses_only_high_confidence_skill() {
        let candidate = route_candidate("pdf", 9.0, Confidence::High);

        let decision = decide_candidate_match(&[candidate]);

        assert_eq!(decision.decision, RouteDecision::UseSkill);
        assert_eq!(decision.selected.unwrap().name, "pdf");
        assert_eq!(decision.bypass_reason, None);
    }

    #[test]
    fn match_gate_bypasses_without_positive_candidates() {
        let decision = decide_candidate_match(&[]);

        assert_eq!(decision.decision, RouteDecision::Bypass);
        assert!(decision.selected.is_none());
        assert_eq!(
            decision.bypass_reason,
            Some(RouteBypassReason::NoCandidates)
        );
    }

    #[test]
    fn match_gate_bypasses_low_or_medium_confidence() {
        let candidate = route_candidate("notes", 3.5, Confidence::Medium);

        let decision = decide_candidate_match(&[candidate]);

        assert_eq!(decision.decision, RouteDecision::Bypass);
        assert!(decision.selected.is_none());
        assert_eq!(
            decision.bypass_reason,
            Some(RouteBypassReason::LowConfidence)
        );
    }

    #[test]
    fn match_gate_marks_close_candidates_ambiguous() {
        let candidates = vec![
            route_candidate("notes", 9.0, Confidence::Medium),
            route_candidate("docs", 8.6, Confidence::Medium),
        ];

        let decision = decide_candidate_match(&candidates);

        assert_eq!(decision.decision, RouteDecision::Ambiguous);
        assert!(decision.selected.is_none());
        assert_eq!(
            decision.bypass_reason,
            Some(RouteBypassReason::AmbiguousMatch)
        );
    }

    fn route_candidate(name: &str, score: f64, confidence: Confidence) -> RouteCandidate {
        RouteCandidate {
            id: name.to_owned(),
            name: name.to_owned(),
            path: PathBuf::from(format!("/tmp/{name}/SKILL.md")),
            score,
            confidence,
            reason: "test".to_owned(),
            visibility: Visibility::Implicit,
            has_skill_spec: false,
        }
    }
}
