use serde::Deserialize;
use sha2::{Digest, Sha256};
use skillspec_core::error::{Error, Result};
use skillspec_core::model::{ResourceRole, SkillSpec};
use skillspec_doctor::source_map::{
    SourceClassificationKind, SourceCoverageStatus, SourceMap, SourceNodeRecord,
    SourceReferenceKind,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const IMPORTED_SCAFFOLD_DESCRIPTION: &str = "Imported SkillSpec scaffold from SKILL.md";
const WORKSPACE_PROMOTION_SCHEMA: &str = "skillspec/workspace-promotion/v0";
const SOURCE_OBLIGATION_COVERAGE_SCHEMA: &str = "skillspec/source-obligation-coverage/v0";

#[derive(Clone, Debug)]
pub(crate) struct ReviewGate {
    blockers: Vec<String>,
}

impl ReviewGate {
    pub(crate) fn is_blocked(&self) -> bool {
        !self.blockers.is_empty()
    }

    pub(crate) fn message(&self) -> String {
        let prefix = if self.blockers.iter().any(|blocker| {
            matches!(
                blocker.as_str(),
                "generic imported.skill id"
                    | "importer scaffold description"
                    | "no executable routes"
                    | "no activation, applies_when, or routing rules"
                    | "importer review_required notes remain"
                    | "preserved prose source has not been promoted"
            )
        }) {
            "generated mechanical scaffold requires semantic promotion before converge, compile, or install"
        } else {
            "workspace package promotion proof is incomplete before converge, compile, or install"
        };
        format!(
            "{prefix}: {}. Promote this package from source/SKILL_md.old and its source map by adding real activation/routes/rules/checks/tests/dependency decisions and complete source-obligation coverage, then rerun workspace converge.",
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
    #[serde(default)]
    package_index: usize,
    #[serde(default)]
    package_count: usize,
    #[serde(default)]
    remaining_after: usize,
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
    review_session: Option<WorkspacePromotionReviewSession>,
    #[serde(default)]
    review: WorkspacePromotionReview,
    #[serde(default)]
    source_obligation_coverage: Option<SourceObligationCoverageProof>,
}

#[derive(Debug, Deserialize)]
struct WorkspacePromotionReviewSession {
    package_id: String,
    package_index: usize,
    package_count: usize,
    remaining_after: usize,
    reviewed_source: String,
    source_sha256: String,
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

#[derive(Debug, Deserialize)]
struct SourceObligationCoverageProof {
    schema: String,
    total: usize,
    promoted: usize,
    not_applicable: usize,
    unresolved: usize,
    obligations: Vec<SourceObligationCoverageEntry>,
}

#[derive(Debug, Deserialize)]
struct SourceObligationCoverageEntry {
    source: String,
    #[serde(default)]
    source_hash: Option<String>,
    disposition: SourceObligationDisposition,
    #[serde(default)]
    targets: Vec<SourceObligationTarget>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum SourceObligationDisposition {
    Promoted,
    NotApplicable,
    Deferred,
    Unresolved,
    Waived,
}

#[derive(Debug, Deserialize)]
struct SourceObligationTarget {
    kind: String,
    id: String,
}

#[derive(Debug)]
struct SourceObligation {
    source: String,
    kind: String,
    hash: Option<String>,
    required_target_kinds: BTreeSet<String>,
    preview: Option<String>,
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
    blockers.extend(review_session_blockers(&proof, evidence));

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

    let source_map_path = PathBuf::from(&evidence.source_map_path);
    let mut source_map = None;
    if let Some(expected_source_map_hash) = proof.source_map_sha256.as_deref() {
        if !source_map_path.is_file() {
            blockers.push(format!(
                "workspace promotion proof source map is missing: {}",
                source_map_path.display()
            ));
        } else {
            let actual_source_map_hash = file_hash(&source_map_path)?;
            if actual_source_map_hash != expected_source_map_hash {
                blockers.push("workspace promotion proof source_map_sha256 is stale".to_owned());
            } else {
                source_map = Some(read_json::<SourceMap>(&source_map_path)?);
            }
        }
    } else if source_map_path.is_file() {
        source_map = Some(read_json::<SourceMap>(&source_map_path)?);
    }

    let missing_review = missing_review_fields(&proof.review);
    if !missing_review.is_empty() {
        blockers.push(format!(
            "workspace promotion proof is incomplete: {}",
            missing_review.join(", ")
        ));
    }

    if let Some(source_map) = source_map.as_ref() {
        blockers.extend(source_obligation_coverage_blockers(
            spec,
            source_map,
            proof.source_obligation_coverage.as_ref(),
        ));
    } else if proof.source_obligation_coverage.is_some() {
        blockers.push(
            "source obligation coverage proof is present but source map could not be loaded"
                .to_owned(),
        );
    }

    Ok(blockers)
}

fn review_session_blockers(
    proof: &WorkspacePromotionProof,
    evidence: &WorkspaceImportEvidence,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if evidence.package_index == 0 || evidence.package_count == 0 {
        blockers.push(
            "workspace import evidence is missing per-package review countdown metadata; rerun workspace import with the current CLI"
                .to_owned(),
        );
    }
    let Some(session) = &proof.review_session else {
        blockers.push(format!(
            "missing per-package review_session countdown for package {} of {}",
            evidence.package_index, evidence.package_count
        ));
        return blockers;
    };
    if session.package_id != evidence.package_id {
        blockers.push(format!(
            "review_session package_id {:?} does not match imported package {:?}",
            session.package_id, evidence.package_id
        ));
    }
    if session.package_index != evidence.package_index {
        blockers.push(format!(
            "review_session package_index {} does not match imported package index {}",
            session.package_index, evidence.package_index
        ));
    }
    if session.package_count != evidence.package_count {
        blockers.push(format!(
            "review_session package_count {} does not match workspace package count {}",
            session.package_count, evidence.package_count
        ));
    }
    if session.remaining_after != evidence.remaining_after {
        blockers.push(format!(
            "review_session remaining_after {} does not match expected countdown {}",
            session.remaining_after, evidence.remaining_after
        ));
    }
    if session.reviewed_source.trim() != "SKILL.md" {
        blockers.push(format!(
            "review_session reviewed_source {:?} must be SKILL.md for the package being promoted",
            session.reviewed_source
        ));
    }
    if session.source_sha256 != proof.source_sha256 {
        blockers.push(
            "review_session source_sha256 does not match workspace promotion source_sha256"
                .to_owned(),
        );
    }
    blockers
}

fn source_obligation_coverage_blockers(
    spec: &SkillSpec,
    source_map: &SourceMap,
    coverage: Option<&SourceObligationCoverageProof>,
) -> Vec<String> {
    let obligations = source_obligations(source_map);
    if obligations.is_empty() {
        return Vec::new();
    }

    let mut blockers = Vec::new();
    let Some(coverage) = coverage else {
        blockers.push(format!(
            "missing source obligation coverage proof: {} source obligation(s) require structural promotion or not-applicable disposition",
            obligations.len()
        ));
        return blockers;
    };

    if coverage.schema != SOURCE_OBLIGATION_COVERAGE_SCHEMA {
        blockers.push(format!(
            "source obligation coverage schema {:?} is not {SOURCE_OBLIGATION_COVERAGE_SCHEMA:?}",
            coverage.schema
        ));
    }

    let obligation_sources = obligations
        .iter()
        .map(|obligation| obligation.source.as_str())
        .collect::<BTreeSet<_>>();
    let mut entries = BTreeMap::<&str, &SourceObligationCoverageEntry>::new();
    let mut duplicate_sources = BTreeSet::new();
    for entry in &coverage.obligations {
        if entries.insert(entry.source.as_str(), entry).is_some() {
            duplicate_sources.insert(entry.source.as_str());
        }
    }

    if coverage.total != obligations.len() {
        blockers.push(format!(
            "source obligation coverage total {} does not match computed obligation count {}",
            coverage.total,
            obligations.len()
        ));
    }
    if !duplicate_sources.is_empty() {
        blockers.push(format!(
            "source obligation coverage has duplicate source entries: {}",
            duplicate_sources
                .into_iter()
                .take(10)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let stale_sources = entries
        .keys()
        .filter(|source| !obligation_sources.contains(**source))
        .copied()
        .collect::<Vec<_>>();
    if !stale_sources.is_empty() {
        blockers.push(format!(
            "source obligation coverage has stale source entries not present in current source map: {}",
            stale_sources.into_iter().take(10).collect::<Vec<_>>().join(", ")
        ));
    }

    let mut promoted = 0usize;
    let mut not_applicable = 0usize;
    let mut unresolved = 0usize;
    let mut missing = Vec::new();
    for obligation in &obligations {
        let Some(entry) = entries.get(obligation.source.as_str()) else {
            missing.push(obligation_summary(obligation));
            continue;
        };
        if let Some(expected_hash) = obligation.hash.as_deref() {
            match entry.source_hash.as_deref() {
                Some(actual_hash) if actual_hash == expected_hash => {}
                Some(_) => blockers.push(format!(
                    "source obligation {} source_hash is stale",
                    obligation.source
                )),
                None => blockers.push(format!(
                    "source obligation {} is missing source_hash from the source review lens",
                    obligation.source
                )),
            }
        }
        match entry.disposition {
            SourceObligationDisposition::Promoted => {
                promoted += 1;
                if entry.targets.is_empty() {
                    blockers.push(format!(
                        "source obligation {} is promoted but has no structural targets",
                        obligation.source
                    ));
                    continue;
                }
                if !obligation.required_target_kinds.is_empty()
                    && !entry
                        .targets
                        .iter()
                        .any(|target| obligation.required_target_kinds.contains(&target.kind))
                {
                    blockers.push(format!(
                        "source obligation {} requires one of target kind(s) [{}] from the source review lens, but proof targets [{}]",
                        obligation.source,
                        obligation
                            .required_target_kinds
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        entry
                            .targets
                            .iter()
                            .map(|target| format!("{}:{}", target.kind, target.id))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                for target in &entry.targets {
                    if !coverage_target_exists(spec, target) {
                        blockers.push(format!(
                            "source obligation {} target {}:{} does not exist in skill.spec.yml",
                            obligation.source, target.kind, target.id
                        ));
                    }
                }
            }
            SourceObligationDisposition::NotApplicable => {
                not_applicable += 1;
                if entry
                    .reason
                    .as_deref()
                    .map(str::trim)
                    .is_none_or(|reason| reason.len() < 10)
                {
                    blockers.push(format!(
                        "source obligation {} is not_applicable without a specific reason",
                        obligation.source
                    ));
                }
            }
            SourceObligationDisposition::Deferred
            | SourceObligationDisposition::Unresolved
            | SourceObligationDisposition::Waived => {
                unresolved += 1;
                blockers.push(format!(
                    "source obligation {} has non-installable disposition {:?}",
                    obligation.source, entry.disposition
                ));
            }
        }
    }

    if !missing.is_empty() {
        blockers.push(format!(
            "source obligation coverage is missing {} computed obligation(s): {}",
            missing.len(),
            missing.into_iter().take(10).collect::<Vec<_>>().join("; ")
        ));
    }
    if coverage.promoted != promoted {
        blockers.push(format!(
            "source obligation coverage promoted count {} does not match verified count {}",
            coverage.promoted, promoted
        ));
    }
    if coverage.not_applicable != not_applicable {
        blockers.push(format!(
            "source obligation coverage not_applicable count {} does not match verified count {}",
            coverage.not_applicable, not_applicable
        ));
    }
    if coverage.unresolved != unresolved {
        blockers.push(format!(
            "source obligation coverage unresolved count {} does not match verified count {}",
            coverage.unresolved, unresolved
        ));
    }

    blockers
}

fn source_obligations(source_map: &SourceMap) -> Vec<SourceObligation> {
    let mut obligations = Vec::new();
    let mut classified_targets = BTreeSet::new();
    for classification in &source_map.classifications {
        if classification.coverage_status == SourceCoverageStatus::ReviewRequired
            || classification.coverage_status == SourceCoverageStatus::Blocked
        {
            let target_node = source_map
                .nodes
                .iter()
                .find(|node| node.id == classification.target);
            obligations.push(SourceObligation {
                source: classification.id.clone(),
                kind: format!("classification:{:?}", classification.kind),
                hash: target_node.and_then(|node| node.hash.clone()),
                required_target_kinds: required_target_kinds_for_classification(
                    &classification.kind,
                    &classification.suggested_constructs,
                ),
                preview: Some(classification.reason.clone()),
            });
            classified_targets.insert(classification.target.as_str());
        }
    }

    for reference in &source_map.references {
        if matches!(
            reference.target_kind,
            SourceReferenceKind::LocalFile | SourceReferenceKind::ExternalUri
        ) {
            let mut required_target_kinds = BTreeSet::new();
            required_target_kinds.insert("resource".to_owned());
            let source_node = source_map
                .nodes
                .iter()
                .find(|node| node.id == reference.source);
            obligations.push(SourceObligation {
                source: reference.id.clone(),
                kind: format!("reference:{:?}", reference.target_kind),
                hash: source_node.and_then(|node| node.hash.clone()),
                required_target_kinds,
                preview: Some(reference.target.clone()),
            });
        }
    }

    for node in &source_map.nodes {
        if classified_targets.contains(node.id.as_str()) || !node_requires_coverage(node) {
            continue;
        }
        obligations.push(SourceObligation {
            source: node.id.clone(),
            kind: format!("node:{}", node.kind),
            hash: node.hash.clone(),
            required_target_kinds: BTreeSet::new(),
            preview: node.title.clone().or_else(|| node.text_preview.clone()),
        });
    }

    obligations.sort_by(|left, right| left.source.cmp(&right.source));
    obligations
}

fn required_target_kinds_for_classification(
    kind: &SourceClassificationKind,
    suggested_constructs: &[String],
) -> BTreeSet<String> {
    let mut required = BTreeSet::new();
    for construct in suggested_constructs {
        if let Some(target_kind) = construct_target_kind(construct) {
            required.insert(target_kind.to_owned());
        }
    }
    if required.is_empty() {
        match kind {
            SourceClassificationKind::ActivationSignal => {
                required.insert("activation".to_owned());
            }
            SourceClassificationKind::RouteCandidate => {
                required.insert("route".to_owned());
            }
            SourceClassificationKind::ModalObligation
            | SourceClassificationKind::ConditionalRuleCandidate
            | SourceClassificationKind::ForbidCandidate => {
                required.insert("rule".to_owned());
            }
            SourceClassificationKind::ElicitationCandidate => {
                required.insert("elicitation".to_owned());
            }
            SourceClassificationKind::DependencyMention => {
                required.insert("dependency".to_owned());
            }
            SourceClassificationKind::CommandExample => {
                required.insert("command".to_owned());
            }
            SourceClassificationKind::CodeBlock => {
                required.insert("code".to_owned());
                required.insert("resource".to_owned());
            }
            SourceClassificationKind::ImportCandidate => {
                required.insert("import".to_owned());
            }
            SourceClassificationKind::ResourceCandidate => {
                required.insert("resource".to_owned());
            }
        }
    }
    required
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

fn node_requires_coverage(node: &SourceNodeRecord) -> bool {
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
}

fn obligation_summary(obligation: &SourceObligation) -> String {
    match obligation.preview.as_deref() {
        Some(preview) if !preview.trim().is_empty() => {
            format!("{} [{}] {}", obligation.source, obligation.kind, preview)
        }
        _ => format!("{} [{}]", obligation.source, obligation.kind),
    }
}

fn coverage_target_exists(spec: &SkillSpec, target: &SourceObligationTarget) -> bool {
    match target.kind.as_str() {
        "activation" => spec.activation.is_some(),
        "applies_when" => !spec.applies_when.is_empty(),
        "entry" => spec.entry.is_some(),
        "route" => spec.routes.iter().any(|route| route.id.0 == target.id),
        "phase" | "route_phase" | "execution_phase" => spec.routes.iter().any(|route| {
            route.execution_plan.as_ref().is_some_and(|plan| {
                plan.phases.iter().any(|phase| {
                    phase.id == target.id
                        || target
                            .id
                            .rsplit_once('.')
                            .is_some_and(|(route_id, phase_id)| {
                                route.id.0 == route_id && phase.id == phase_id
                            })
                })
            })
        }),
        "rule" => spec.rules.iter().any(|rule| rule.id.0 == target.id),
        "state" => spec.states.contains_key(&target.id),
        "elicitation" => spec.elicitations.contains_key(&target.id),
        "dependency" => spec.dependencies.contains_key(&target.id),
        "import" => spec.imports.contains_key(&target.id),
        "resource" => spec.resources.contains_key(&target.id),
        "code" => spec.code.contains_key(&target.id),
        "artifact" => spec.artifacts.contains_key(&target.id),
        "recipe" => spec.recipes.contains_key(&target.id),
        "command" => spec.commands.contains_key(&target.id),
        "snippet" => spec.snippets.contains_key(&target.id),
        "closure" => spec.closures.contains_key(&target.id),
        "proof" => spec.proof.is_some(),
        "test" => spec.tests.iter().any(|test| test.name == target.id),
        "metadata" => spec.metadata.contains_key(&target.id),
        _ => false,
    }
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
