use serde::Deserialize;
use sha2::{Digest, Sha256};
use skillspec_core::error::{Error, Result};
use skillspec_core::model::{ResourceRole, SkillSpec};
use std::fs;
use std::path::{Path, PathBuf};

const IMPORTED_SCAFFOLD_DESCRIPTION: &str = "Imported SkillSpec scaffold from SKILL.md";
const WORKSPACE_PROMOTION_SCHEMA: &str = "skillspec/workspace-promotion/v0";

#[derive(Clone, Debug)]
pub(crate) struct ReviewGate {
    blockers: Vec<String>,
}

impl ReviewGate {
    pub(crate) fn is_blocked(&self) -> bool {
        !self.blockers.is_empty()
    }

    pub(crate) fn message(&self) -> String {
        format!(
            "generated mechanical scaffold requires semantic promotion before converge, compile, or install: {}. Promote this package from source/SKILL_md.old and its source map by adding real activation/routes/rules/checks/tests/dependency decisions, then rerun workspace converge.",
            self.blockers.join("; ")
        )
    }
}

pub(crate) fn review_gate(output_dir: &Path, spec: &SkillSpec) -> Result<ReviewGate> {
    let mut blockers = Vec::new();
    if spec.id == "imported.skill" {
        blockers.push("generic imported.skill id".to_owned());
    }
    if spec.description.trim() == IMPORTED_SCAFFOLD_DESCRIPTION {
        blockers.push("importer scaffold description".to_owned());
    }
    if spec.routes.is_empty() {
        blockers.push("no executable routes".to_owned());
    }
    if spec.activation.is_none() && spec.applies_when.is_empty() && spec.rules.is_empty() {
        blockers.push("no activation, applies_when, or routing rules".to_owned());
    }
    if !spec.review_required.is_empty() {
        blockers.push("importer review_required notes remain".to_owned());
    }
    if spec
        .resources
        .values()
        .any(|resource| resource.path == "source/SKILL_md.old")
    {
        blockers.push("preserved prose source has not been promoted".to_owned());
    }

    if deps_toml_marks_import_scaffold(output_dir)? {
        blockers.push("deps.toml marks generated import scaffold as review_required".to_owned());
    }

    if let Some(evidence) = load_workspace_import_evidence(output_dir)? {
        if evidence.requires_promotion_proof() {
            blockers.extend(workspace_promotion_blockers(output_dir, spec, &evidence)?);
        }
    }

    Ok(ReviewGate { blockers })
}

fn deps_toml_marks_import_scaffold(output_dir: &Path) -> Result<bool> {
    let path = output_dir.join("deps.toml");
    if !path.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(&path).map_err(|source| Error::Read { path, source })?;
    Ok(content
        .lines()
        .map(str::trim)
        .any(|line| line == "generated_by = \"skillspec import-skill\"")
        && content
            .lines()
            .map(str::trim)
            .any(|line| line == "review_required = true"))
}

#[derive(Debug, Deserialize)]
struct WorkspaceImportEvidence {
    package_id: String,
    status: String,
    source_path: String,
    spec_path: String,
    source_map_path: String,
}

impl WorkspaceImportEvidence {
    fn requires_promotion_proof(&self) -> bool {
        matches!(self.status.as_str(), "built" | "cached")
    }
}

#[derive(Debug, Deserialize)]
struct WorkspacePromotionProof {
    schema: String,
    package_id: String,
    status: String,
    source_sha256: String,
    spec_sha256: String,
    #[serde(default)]
    source_map_sha256: Option<String>,
    #[serde(default)]
    review: WorkspacePromotionReview,
}

#[derive(Debug, Default, Deserialize)]
struct WorkspacePromotionReview {
    #[serde(default)]
    activation_reviewed: bool,
    #[serde(default)]
    routes_reviewed: bool,
    #[serde(default)]
    rules_reviewed: bool,
    #[serde(default)]
    dependencies_reviewed: bool,
    #[serde(default)]
    checks_or_tests_reviewed: bool,
    #[serde(default)]
    proof_reviewed: bool,
}

fn load_workspace_import_evidence(output_dir: &Path) -> Result<Option<WorkspaceImportEvidence>> {
    let path = output_dir.join(".skillspec/workspace-import.json");
    if !path.exists() {
        return Ok(None);
    }
    read_json(&path).map(Some)
}

fn workspace_promotion_blockers(
    output_dir: &Path,
    spec: &SkillSpec,
    evidence: &WorkspaceImportEvidence,
) -> Result<Vec<String>> {
    let mut blockers = runtime_prose_wrapper_blockers(spec);
    let proof_path = output_dir.join(".skillspec/workspace-promotion.json");
    if !proof_path.exists() {
        blockers.push(format!(
            "missing workspace promotion proof {}",
            proof_path.display()
        ));
        return Ok(blockers);
    }

    let proof: WorkspacePromotionProof = read_json(&proof_path)?;
    if proof.schema != WORKSPACE_PROMOTION_SCHEMA {
        blockers.push(format!(
            "workspace promotion proof schema {:?} is not {WORKSPACE_PROMOTION_SCHEMA:?}",
            proof.schema
        ));
    }
    if proof.package_id != evidence.package_id {
        blockers.push(format!(
            "workspace promotion proof package_id {:?} does not match imported package {:?}",
            proof.package_id, evidence.package_id
        ));
    }
    if proof.status != "reviewed" {
        blockers.push(format!(
            "workspace promotion proof status {:?} is not reviewed",
            proof.status
        ));
    }

    let source_path = PathBuf::from(&evidence.source_path);
    if !source_path.exists() {
        blockers.push(format!(
            "workspace promotion proof source path is missing: {}",
            source_path.display()
        ));
    } else {
        let source_hash = package_source_hash(&source_path)?;
        if proof.source_sha256 != source_hash {
            blockers.push("workspace promotion proof source_sha256 is stale".to_owned());
        }
    }

    let spec_path = PathBuf::from(&evidence.spec_path);
    let current_spec_path = output_dir.join("skill.spec.yml");
    let spec_path = if spec_path.is_file() {
        spec_path
    } else {
        current_spec_path
    };
    let spec_hash = file_hash(&spec_path)?;
    if proof.spec_sha256 != spec_hash {
        blockers.push("workspace promotion proof spec_sha256 is stale".to_owned());
    }

    if let Some(expected_source_map_hash) = proof.source_map_sha256.as_deref() {
        let source_map_path = PathBuf::from(&evidence.source_map_path);
        if !source_map_path.is_file() {
            blockers.push(format!(
                "workspace promotion proof source map is missing: {}",
                source_map_path.display()
            ));
        } else if file_hash(&source_map_path)? != expected_source_map_hash {
            blockers.push("workspace promotion proof source_map_sha256 is stale".to_owned());
        }
    }

    let missing_review = missing_review_fields(&proof.review);
    if !missing_review.is_empty() {
        blockers.push(format!(
            "workspace promotion proof is incomplete: {}",
            missing_review.join(", ")
        ));
    }

    Ok(blockers)
}

fn missing_review_fields(review: &WorkspacePromotionReview) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !review.activation_reviewed {
        missing.push("activation_reviewed");
    }
    if !review.routes_reviewed {
        missing.push("routes_reviewed");
    }
    if !review.rules_reviewed {
        missing.push("rules_reviewed");
    }
    if !review.dependencies_reviewed {
        missing.push("dependencies_reviewed");
    }
    if !review.checks_or_tests_reviewed {
        missing.push("checks_or_tests_reviewed");
    }
    if !review.proof_reviewed {
        missing.push("proof_reviewed");
    }
    missing
}

fn runtime_prose_wrapper_blockers(spec: &SkillSpec) -> Vec<String> {
    let mut blockers = Vec::new();
    if spec
        .entry
        .as_ref()
        .is_some_and(|entry| is_runtime_prose_wrapper_text(&entry.prompt))
    {
        blockers.push(
            "entry prompt still delegates execution to original prose instructions".to_owned(),
        );
    }
    if spec.applies_when.iter().any(value_mentions_original_prose) {
        blockers.push(
            "applies_when still delegates selection to original prose instructions".to_owned(),
        );
    }
    if spec.routes.iter().any(|route| {
        route.execution_plan.as_ref().is_some_and(|plan| {
            plan.phases.iter().any(|phase| {
                phase
                    .description
                    .as_deref()
                    .is_some_and(is_runtime_prose_wrapper_text)
            })
        })
    }) {
        blockers.push(
            "route phases still delegate execution to original prose instructions".to_owned(),
        );
    }
    if spec.resources.values().any(|resource| {
        resource.role == ResourceRole::SourceMaterial
            && (resource.path.contains("SKILL_source")
                || resource
                    .description
                    .as_deref()
                    .is_some_and(is_runtime_prose_wrapper_text))
    }) {
        blockers.push("original SKILL.md remains a runtime source material dependency".to_owned());
    }
    blockers
}

fn value_mentions_original_prose(value: &serde_yaml::Value) -> bool {
    match value {
        serde_yaml::Value::String(value) => is_runtime_prose_wrapper_text(value),
        serde_yaml::Value::Sequence(values) => values.iter().any(value_mentions_original_prose),
        serde_yaml::Value::Mapping(values) => values.iter().any(|(key, value)| {
            value_mentions_original_prose(key) || value_mentions_original_prose(value)
        }),
        _ => false,
    }
}

fn is_runtime_prose_wrapper_text(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("load the promoted original")
        || value.contains("load the original")
        || value.contains("original instructions")
        || value.contains("source instructions")
        || value.contains("skill_source")
        || (value.contains("authoritative runtime") && value.contains("instructions"))
}

fn read_json<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&content).map_err(|source| Error::ParseJson {
        path: path.to_path_buf(),
        source,
    })
}

fn package_source_hash(source: &Path) -> Result<String> {
    let mut paths = Vec::new();
    collect_hashable_files(source, &mut paths)?;
    paths.sort();
    let mut hasher = Sha256::new();
    for path in paths {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update([0]);
        let bytes = fs::read(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        hasher.update(sha256_hex(&bytes).as_bytes());
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_hashable_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if should_skip_path(path) {
        return Ok(());
    }
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_hashable_files(&path, files)?;
        } else if !should_skip_path(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn should_skip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.') || matches!(name, "target" | "node_modules"))
}

fn file_hash(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
