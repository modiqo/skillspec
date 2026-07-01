use crate::remote_source::RemoteSkillSource;
use crate::source_map::{
    SourceClassificationKind, SourceFileKind, SourceFileLoadStatus, SourceMap, SourceReferenceKind,
};
mod frontmatter;
mod metrics;
pub mod remote_source;
mod renderer;
mod risk;
pub mod source_map;
mod types;
mod workspace_package_profile;
mod workspace_report;

pub use renderer::{render, render_html, render_markdown};

use serde::Serialize;
use skillspec_core::error::{Error, Result};
use skillspec_core::{model::SkillSpec, parser};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use types::{
    AgentDriftRiskReport, ContractMitigationLevel, ContractMitigationReport,
    DoctorPackageRiskReport, FrontmatterDiscoveryRiskReport, RawActivationRiskReport, RiskLevel,
    WorkspaceAgentDriftRiskReport,
};

const LARGE_BODY_LINES: usize = 500;
const LARGE_BODY_TOKENS: usize = 5_000;
const PRIMACY_LINE_PERCENT: usize = 60;

#[derive(Clone, Debug, Serialize)]
pub struct DoctorReport {
    pub target: String,
    pub source_kind: String,
    pub analysis_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_from: Option<String>,
    pub shape: DoctorShapeReport,
    pub verdict: String,
    pub score_model: DoctorScoreModelReport,
    pub structural_score: u8,
    pub large_surface_percentage: u8,
    pub surface: SurfaceReport,
    pub counts: DoctorCounts,
    pub issues: Vec<DoctorIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter_discovery_risk: Option<FrontmatterDiscoveryRiskReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_drift_risk: Option<AgentDriftRiskReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_activation_risk: Option<RawActivationRiskReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_mitigation: Option<ContractMitigationReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_agent_drift_risk: Option<WorkspaceAgentDriftRiskReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_identity: Option<WorkspaceIdentityReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<DoctorPackageRiskReport>,
    pub basis: Vec<DoctorBasis>,
    pub suggested_next_steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorScoreModelReport {
    pub schema: String,
    pub primary_score_label: String,
    pub primary_score_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_score: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_level: Option<RiskLevel>,
    pub risk_direction: String,
    pub readiness_label: String,
    pub baseline_scope: String,
    pub plain_language_summary: String,
    pub not_measuring: Vec<String>,
    pub basis_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorShapeReport {
    pub kind: String,
    pub summary: String,
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_skill: Option<String>,
    pub skill_files: Vec<String>,
    pub plugin_roots: Vec<DoctorPluginRootReport>,
    pub referenced_skill_paths: Vec<String>,
    pub negative_signals: Vec<String>,
    pub recommended_command: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorPluginRootReport {
    pub namespace: String,
    pub path: String,
    pub skill_files: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceIdentityReport {
    pub source_file_count: usize,
    pub skill_file_count: usize,
    pub namespaced_package_count: usize,
    pub namespace_count: usize,
    pub namespaces: Vec<WorkspaceNamespaceIdentityReport>,
    pub unique_skill_content_count: usize,
    pub repeated_skill_content_groups: usize,
    pub repeated_skill_content_occurrences: usize,
    pub total_skill_content_estimated_tokens: usize,
    pub unique_skill_content_estimated_tokens: usize,
    pub repeated_skill_content_estimated_tokens: usize,
    pub source_content_refs: Vec<WorkspaceSourceContentRefReport>,
    pub same_frontmatter_name_groups: usize,
    pub same_frontmatter_name_occurrences: usize,
    pub frontmatter_name_refs: Vec<WorkspaceFrontmatterNameRefReport>,
    pub recommendation: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceNamespaceIdentityReport {
    pub namespace: String,
    pub skill_file_count: usize,
    pub sample_paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceSourceContentRefReport {
    pub sha256: String,
    pub canonical_path: String,
    pub aliases: Vec<String>,
    pub occurrence_count: usize,
    pub content_bytes: usize,
    pub estimated_tokens: usize,
    pub repeated_occurrence_count: usize,
    pub repeated_estimated_tokens: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceFrontmatterNameRefReport {
    pub public_name: String,
    pub paths: Vec<String>,
    pub occurrence_count: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SurfaceReport {
    pub frontmatter_bytes: usize,
    pub frontmatter_lines: usize,
    pub activation_bytes: usize,
    pub activation_lines: usize,
    pub activation_estimated_tokens: usize,
    pub deferred_bytes: usize,
    pub deferred_files: usize,
    pub unmapped_files: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DoctorCounts {
    pub total_files: usize,
    pub markdown_files: usize,
    pub code_files: usize,
    pub manifest_files: usize,
    pub code_blocks_in_skill: usize,
    pub unlabeled_code_blocks_in_skill: usize,
    pub modal_obligations: usize,
    pub late_modal_obligations: usize,
    pub numbered_steps: usize,
    pub dependency_mentions: usize,
    pub missing_local_references: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorIssue {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub evidence: String,
    pub basis: Vec<String>,
    pub remediation: String,
    pub score_penalty: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorBasis {
    pub id: String,
    pub kind: String,
    pub citation: String,
    pub source: String,
    pub claim: String,
}

struct SkillBody {
    path: PathBuf,
    file_id: String,
    body: String,
    frontmatter: String,
    body_start_line: usize,
}

pub fn inspect_target(target: &str) -> Result<DoctorReport> {
    let target_path = Path::new(target);
    if target_path.exists() {
        let mut report = inspect_local_target(target_path)?;
        report.target = target.to_owned();
        return Ok(report);
    }

    if looks_like_explicit_local_target(target) {
        let cwd = std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "<unknown>".to_owned());
        return Err(Error::InvalidInput {
            message: format!(
                "doctor target {target:?} does not exist locally from {cwd}; local paths must exist before doctor runs. Use ./<path>, ../<path>, or an absolute path for local folders. Use an explicit GitHub URL or owner/repo shorthand without a trailing slash for remote sources."
            ),
        });
    }

    let Some(remote) = remote_source::parse_target(target)? else {
        return Err(Error::InvalidInput {
            message: format!(
                "doctor target {target:?} does not exist locally; remote doctor supports public GitHub repo or skill-folder URLs such as https://github.com/<owner>/<repo> and https://github.com/<owner>/<repo>/tree/<branch>/<path>"
            ),
        });
    };
    let staged = remote_source::clone_remote_temp(&remote, "skillspec-doctor")?;
    let rewrite_root = remote
        .path
        .as_deref()
        .map(|path| staged.checkout_dir().join(path))
        .unwrap_or_else(|| staged.checkout_dir().to_path_buf());
    let mut report = match inspect_staged_remote(target, &remote, staged.checkout_dir()) {
        Ok(report) => report,
        Err(error) => return Err(rewrite_remote_error(error, &rewrite_root, target)),
    };
    rewrite_remote_locations(&mut report, &rewrite_root, target);
    report.target = target.to_owned();
    report.source_kind = "remote_github".to_owned();
    report.staged_from = Some(remote.repo_url);
    Ok(report)
}

pub fn inspect(path: &Path) -> Result<DoctorReport> {
    inspect_local_target(path)
}

fn looks_like_explicit_local_target(target: &str) -> bool {
    let trimmed = target.trim();
    trimmed.starts_with('.')
        || trimmed.starts_with('/')
        || trimmed.starts_with('~')
        || trimmed.ends_with('/')
        || trimmed.ends_with('\\')
}

fn inspect_local_target(path: &Path) -> Result<DoctorReport> {
    let classification = classify_local(path)?;
    if classification.shape.kind == "non_skill_repository" {
        return Ok(shape_only_report_from_classification(
            &path.display().to_string(),
            "local",
            None,
            classification,
        ));
    }
    if classification.shape.kind != "simple_skill" {
        return workspace_report::inspect_local_target(path, classification);
    }
    let shape = classification.shape;
    let package_root = shape
        .primary_skill
        .as_deref()
        .and_then(|skill_path| Path::new(skill_path).parent())
        .map(|relative_parent| shape_root(path).join(relative_parent))
        .unwrap_or_else(|| shape_root(path));
    let mut report = inspect_simple_skill(&package_root)?;
    report.shape = shape;
    apply_simple_recommended_action(&mut report, &package_root);
    Ok(report)
}

fn inspect_simple_skill(path: &Path) -> Result<DoctorReport> {
    let map = source_map::build(path)?;
    let source_root = PathBuf::from(&map.source_root);
    let skill = load_skill_body(&map, &source_root)?;
    let skill_sections = frontmatter::split_skill(&format!("{}{}", skill.frontmatter, skill.body));
    let frontmatter_discovery_risk = frontmatter::analyze(&skill.path, &skill_sections);
    let mut shape = simple_shape_for_source_root(&source_root)?;
    let surface = surface_report(&map, &skill);
    let counts = counts(&map, &skill);
    let skill_spec_path = source_root.join("skill.spec.yml");
    let has_skill_spec_file = skill_spec_path.exists();
    let (contract_source, invalid_contract_issue) = if has_skill_spec_file {
        match parser::load_spec(&skill_spec_path) {
            Ok(spec) => (Some((skill_spec_path.clone(), spec)), None),
            Err(error) => (
                None,
                Some(with_location(
                    issue(
                        "invalid_behavior_contract",
                        "high",
                        "SkillSpec contract is present but invalid",
                        format!("skill.spec.yml could not be loaded: {error}"),
                        vec![
                            "contract_trace_behavioral_contract",
                            "contract_trace_static_well_formedness",
                        ],
                        "Fix `skill.spec.yml` validation errors before relying on this skill; an invalid contract cannot mitigate prose drift.",
                        18,
                    ),
                    skill_spec_path.display().to_string(),
                )),
            ),
        }
    } else {
        (None, None)
    };
    let has_valid_skill_spec = contract_source.is_some();
    let has_structured_dependencies = contract_source
        .as_ref()
        .is_some_and(|(_, spec)| !spec.dependencies.is_empty());
    let has_deps_toml = source_root.join("deps.toml").exists();
    let has_tests = source_root.join(".skillspec").exists()
        || contract_source
            .as_ref()
            .is_some_and(|(_, spec)| !spec.tests.is_empty());
    shape.recommended_command = recommended_simple_next_action(&source_root, has_valid_skill_spec);
    let mut issues = Vec::new();
    if let Some(issue) = invalid_contract_issue {
        issues.push(issue);
    }
    issues.extend(frontmatter_issues(
        &frontmatter_discovery_risk,
        &skill.path.display().to_string(),
    ));

    let large_surface_percentage = metrics::percentage(
        surface.activation_bytes,
        surface.frontmatter_bytes + surface.activation_bytes + surface.deferred_bytes,
    );

    if surface.activation_lines > LARGE_BODY_LINES
        || surface.activation_estimated_tokens > LARGE_BODY_TOKENS
    {
        let penalty = if surface.activation_lines > LARGE_BODY_LINES * 2
            || surface.activation_estimated_tokens > LARGE_BODY_TOKENS * 2
        {
            24
        } else {
            16
        };
        issues.push(with_location(
            issue(
                "large_activation_body",
                "high",
                "Large activation-loaded instruction body",
                format!(
                    "SKILL.md activation body is {} lines / approximately {} tokens; this exceeds the {} line or {} token guidance used by skill authoring practice.",
                    surface.activation_lines,
                    surface.activation_estimated_tokens,
                    LARGE_BODY_LINES,
                    LARGE_BODY_TOKENS
                ),
                vec![
                    "reliability_gap_instruction_density",
                    "contract_trace_activation_adherence_enforcement",
                ],
                "Split detail into referenced files, then use `skillspec source map` and `skillspec import-skill` so load-bearing behavior can be reviewed progressively.",
                penalty,
            ),
            format!("{}:{}", skill.path.display(), skill.body_start_line),
        ));
    }

    if large_surface_percentage >= 75 {
        issues.push(with_location(
            issue(
                "large_activation_surface",
                "high",
                "Most package text loads at activation",
                format!(
                    "{}% of loaded text surface is in SKILL.md activation body; little is deferred behind task-specific references.",
                    large_surface_percentage
                ),
                vec![
                    "reliability_gap_metadata_context_pressure",
                    "reliability_gap_instruction_density",
                ],
                "Move long references, examples, and code into referenced files and keep the activation body as a compact router.",
                18,
            ),
            skill.path.display().to_string(),
        ));
    } else if large_surface_percentage >= 50 {
        issues.push(with_location(
            issue(
                "medium_activation_surface",
                "medium",
                "Activation surface is still broad",
                format!(
                    "{}% of loaded text surface is in SKILL.md activation body.",
                    large_surface_percentage
                ),
                vec!["reliability_gap_instruction_density"],
                "Review whether examples and procedural detail can be deferred into references.",
                8,
            ),
            skill.path.display().to_string(),
        ));
    }

    if counts.modal_obligations >= 12 || counts.numbered_steps >= 12 {
        issues.push(with_location(
            issue(
                "instruction_density",
                "high",
                "Dense load-bearing prose",
                format!(
                    "Found {} modal obligation spans and {} numbered steps in the activation body.",
                    counts.modal_obligations, counts.numbered_steps
                ),
                vec![
                    "reliability_gap_instruction_density",
                    "contract_trace_activation_adherence_enforcement",
                ],
                "Promote route choices, forbids, elicitations, dependencies, and tests into `skill.spec.yml` so they can be checked instead of remembered.",
                14,
            ),
            skill.path.display().to_string(),
        ));
    }

    if counts.late_modal_obligations > 0 {
        issues.push(with_location(
            issue(
                "primacy_bias_late_obligations",
                "medium",
                "Late load-bearing instructions are exposed to primacy bias",
                format!(
                    "{} modal obligation span(s) appear after the first {}% of the activation body.",
                    counts.late_modal_obligations, PRIMACY_LINE_PERCENT
                ),
                vec!["reliability_gap_instruction_density"],
                "Move late obligations into earlier route/rule summaries or structured checks; do not rely on a model remembering buried instructions.",
                10,
            ),
            skill.path.display().to_string(),
        ));
    }

    if counts.code_blocks_in_skill > 0 {
        let code_bytes = skill_code_bytes(&map, &skill.file_id);
        let code_percent = metrics::percentage(code_bytes, surface.activation_bytes);
        issues.push(with_location(
            issue(
                "code_mixed_with_activation_instructions",
                "medium",
                "Code is mixed into the activation instruction body",
                format!(
                    "Found {} fenced code block(s) in SKILL.md, accounting for about {}% of activation bytes.",
                    counts.code_blocks_in_skill, code_percent
                ),
                vec![
                    "reliability_gap_no_execution_guarantees",
                    "contract_trace_static_well_formedness",
                ],
                "Move executable code into scripts/resources or structured `code` entries and state whether snippets are executable, examples, or reference material.",
                12,
            ),
            skill.path.display().to_string(),
        ));
    }

    if counts.unlabeled_code_blocks_in_skill > 0 {
        issues.push(with_location(
            issue(
                "unlabeled_code_fences",
                "medium",
                "Code fence language is ambiguous",
                format!(
                    "{} code fence(s) in SKILL.md omit a language label.",
                    counts.unlabeled_code_blocks_in_skill
                ),
                vec!["reliability_gap_no_execution_guarantees"],
                "Label code fences and classify each one as executable code, command example, or non-executable reference.",
                6,
            ),
            skill.path.display().to_string(),
        ));
    }

    if operational_prose(&skill.body) && !has_valid_skill_spec {
        issues.push(with_location(
            issue(
                "ambiguous_execution_substrate",
                "high",
                "Operational prose lacks a structured execution contract",
                "The activation body tells the model to use/run/create/fetch/click/install or similar, but there is no SkillSpec route, tool boundary, command template, or trace vocabulary.".to_owned(),
                vec![
                    "contract_trace_activation_adherence_enforcement",
                    "contract_trace_unproven_verdict",
                ],
                "Add a SkillSpec contract with routes, phase tool boundaries, commands, scenario tests, and trace requirements.",
                18,
            ),
            skill.path.display().to_string(),
        ));
    }

    if (counts.dependency_mentions > 0
        || counts.code_files > 0
        || counts.manifest_files > 0
        || counts.code_blocks_in_skill > 0)
        && !has_deps_toml
        && !has_structured_dependencies
    {
        issues.push(with_location(
            issue(
                "implicit_dependency_contract",
                "high",
                "Dependencies are implicit",
                format!(
                    "Detected {} dependency mention(s), {} code file(s), {} manifest file(s), and {} code block(s), but no deps.toml ledger.",
                    counts.dependency_mentions,
                    counts.code_files,
                    counts.manifest_files,
                    counts.code_blocks_in_skill
                ),
                vec!["reliability_gap_implicit_environment_contract"],
                "Create `deps.toml` and preserve dependency authority, local status, install risk, and degraded proof impact before proof or install.",
                16,
            ),
            source_root.display().to_string(),
        ));
    }

    if counts.missing_local_references > 0 {
        issues.push(with_location(
            issue(
                "missing_referenced_files",
                "medium",
                "Referenced local files are missing",
                format!(
                    "{} local Markdown reference(s) did not resolve to files in the skill package.",
                    counts.missing_local_references
                ),
                vec!["contract_trace_static_well_formedness"],
                "Fix broken links or preserve the missing files before import, install, or release.",
                8,
            ),
            skill.path.display().to_string(),
        ));
    }

    if surface.unmapped_files > 0 {
        issues.push(with_location(
            issue(
                "unmapped_package_surface",
                "medium",
                "Package files are present but not clearly reachable",
                format!(
                    "{} non-SKILL file(s) are present without an explicit local reference from Markdown.",
                    surface.unmapped_files
                ),
                vec!["contract_trace_static_well_formedness"],
                "Declare package-local files as imports, resources, code sources, artifacts, or dependency ledgers during SkillSpec porting.",
                8,
            ),
            source_root.display().to_string(),
        ));
    }

    if !has_valid_skill_spec {
        issues.push(with_location(
            issue(
                "missing_behavior_contract",
                "high",
                "No machine-checkable behavior contract",
                "No skill.spec.yml was found, so route choices, forbids, tool boundaries, dependency checks, scenario tests, and trace expectations are not falsifiable.".to_owned(),
                vec![
                    "reliability_gap_unfilled_requirement",
                    "contract_trace_behavioral_contract",
                ],
                "Run `skillspec source map`, then `skillspec import-skill`, and complete the generated contract before install or proof.",
                20,
            ),
            source_root.display().to_string(),
        ));
    }

    if !has_tests {
        issues.push(with_location(
            issue(
                "missing_trace_proof_surface",
                "medium",
                "Runtime success would be unproven",
                "No trace/progress/test surface was found, so a successful run would not prove which obligations actually executed.".to_owned(),
                vec!["contract_trace_unproven_verdict"],
                "Add scenario tests and trace/progress requirements; report `unproven` when evidence is absent instead of treating no error as success.",
                10,
            ),
            source_root.display().to_string(),
        ));
    }

    issues.sort_by_key(|issue| severity_rank(&issue.severity));
    let penalty = issues
        .iter()
        .map(|issue| usize::from(issue.score_penalty))
        .sum::<usize>()
        .min(100);
    let structural_score = u8::try_from(100usize.saturating_sub(penalty)).unwrap_or(0);
    let verdict = verdict(structural_score);

    let basis = basis();
    let raw_activation_risk = raw_activation_risk(&surface, &counts, structural_score);
    let contract_mitigation = contract_source
        .as_ref()
        .map(|(path, spec)| contract_mitigation(path, spec, raw_activation_risk.score));
    let mut agent_drift_risk = risk::agent_report(
        structural_score,
        &issues,
        Some(frontmatter_discovery_risk.clone()),
        &basis,
    );
    if let Some(mitigation) = &contract_mitigation {
        agent_drift_risk.summary = format!(
            "{} raw activation risk; SkillSpec contract mitigation is {}. Residual risk is {} because activation-loaded prose still has to stay small.",
            raw_activation_risk.level.as_str(),
            mitigation.level.as_str(),
            mitigation.residual_risk_level.as_str()
        );
        agent_drift_risk.recommended_mode = "thin_trampoline_and_use_guided_cli".to_owned();
    }
    let suggested_next_steps = next_steps(
        &source_root,
        has_valid_skill_spec,
        has_deps_toml,
        has_structured_dependencies,
        raw_activation_risk.score,
    );
    let score_model = if let Some(mitigation) = &contract_mitigation {
        score_model(
            "contract_mitigation.residual_risk_score",
            Some(mitigation.residual_risk_score),
            Some(mitigation.residual_risk_level),
        )
    } else {
        score_model(
            "agent_drift_risk.score",
            Some(agent_drift_risk.score),
            Some(agent_drift_risk.level),
        )
    };

    Ok(DoctorReport {
        target: source_root.display().to_string(),
        source_kind: "local".to_owned(),
        analysis_status: "full".to_owned(),
        staged_from: None,
        shape,
        verdict,
        score_model,
        structural_score,
        large_surface_percentage,
        surface,
        counts,
        issues,
        frontmatter_discovery_risk: Some(frontmatter_discovery_risk),
        agent_drift_risk: Some(agent_drift_risk),
        raw_activation_risk: Some(raw_activation_risk),
        contract_mitigation,
        workspace_agent_drift_risk: None,
        workspace_identity: None,
        packages: Vec::new(),
        basis,
        suggested_next_steps,
    })
}

#[derive(Clone, Debug)]
struct ShapeClassification {
    shape: DoctorShapeReport,
    counts: DoctorCounts,
}

fn inspect_staged_remote(
    target: &str,
    remote: &RemoteSkillSource,
    checkout_dir: &Path,
) -> Result<DoctorReport> {
    if let Some(path) = &remote.path {
        remote_source::set_sparse_path(checkout_dir, path)?;
        let scope_path = checkout_dir.join(path);
        if !scope_path.exists() {
            return Err(Error::InvalidInput {
                message: format!(
                    "remote path {path} did not materialize from {}",
                    remote.repo_url
                ),
            });
        }
        return inspect_local_target(&scope_path);
    }

    let tree_files = remote_source::git_tree_files(checkout_dir)?;
    let root_skill_content = if tree_files
        .iter()
        .any(|path| path.to_string_lossy().eq_ignore_ascii_case("SKILL.md"))
    {
        remote_source::git_show_text(checkout_dir, "SKILL.md").ok()
    } else {
        None
    };
    let classification = classify_shape_from_files(
        Path::new(""),
        target,
        tree_files,
        root_skill_content.as_deref(),
    )?;
    if classification.shape.kind != "simple_skill" {
        if classification.shape.kind == "non_skill_repository" {
            return Ok(shape_only_report_from_classification(
                target,
                "remote_github",
                None,
                classification,
            ));
        }
        return workspace_report::inspect_remote_target(target, checkout_dir, classification);
    }

    let primary = classification
        .shape
        .primary_skill
        .clone()
        .unwrap_or_else(|| "SKILL.md".to_owned());
    let package_path = Path::new(&primary)
        .parent()
        .map(path_to_slash)
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| ".".to_owned());
    remote_source::set_sparse_path(checkout_dir, &package_path)?;
    let scope_path = if package_path == "." {
        checkout_dir.to_path_buf()
    } else {
        checkout_dir.join(&package_path)
    };
    let mut report = inspect_local_target(&scope_path)?;
    report.shape = classification.shape;
    apply_simple_recommended_action(&mut report, &scope_path);
    Ok(report)
}

fn classify_local(path: &Path) -> Result<ShapeClassification> {
    let root = shape_root(path);
    let files = collect_inventory_files(&root)?;
    let root_skill_content = fs::read_to_string(root.join("SKILL.md")).ok();
    classify_shape_from_files(
        &root,
        &root.display().to_string(),
        files,
        root_skill_content.as_deref(),
    )
}

pub(crate) fn classify_source_shape(path: &Path) -> Result<DoctorShapeReport> {
    Ok(classify_local(path)?.shape)
}

fn classify_shape_from_files(
    root: &Path,
    root_label: &str,
    mut files: Vec<PathBuf>,
    root_skill_content: Option<&str>,
) -> Result<ShapeClassification> {
    files.retain(|path| !path_has_skipped_dir(path));
    files.sort();
    let skill_files = files
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let root_skill = skill_files
        .iter()
        .find(|path| path.components().count() == 1)
        .map(|path| path_to_slash(path));
    let plugin_roots = plugin_roots_from_files(root, &files);
    let referenced_skill_paths = root_skill_content
        .map(|content| referenced_nested_skills(content, &skill_files))
        .unwrap_or_default();
    let counts = inventory_counts(&files);

    let mut negative_signals = Vec::new();
    let kind = if !plugin_roots.is_empty() {
        negative_signals
            .push("plugin markers require namespace-preserving workspace processing".to_owned());
        "plugin_workspace"
    } else if skill_files.is_empty() {
        negative_signals.push("no SKILL.md discovered under the target".to_owned());
        if counts.code_files > 0 || counts.manifest_files > 0 {
            negative_signals.push(format!(
                "code-like repository surface detected: {} code file(s), {} manifest file(s)",
                counts.code_files, counts.manifest_files
            ));
        }
        "non_skill_repository"
    } else if skill_files.len() == 1 {
        "simple_skill"
    } else if root_skill.is_some() {
        negative_signals
            .push("root SKILL.md plus nested SKILL.md packages is not one atomic skill".to_owned());
        "entry_skill_with_subskills"
    } else {
        negative_signals.push("multiple SKILL.md files require workspace mapping".to_owned());
        "multi_skill_workspace"
    }
    .to_owned();

    let primary_skill = match kind.as_str() {
        "simple_skill" => skill_files.first().map(|path| path_to_slash(path)),
        "entry_skill_with_subskills" => root_skill.clone(),
        _ => None,
    };
    let summary = shape_summary(&kind, skill_files.len(), plugin_roots.len());
    let recommended_command =
        recommended_shape_command(&kind, root_label, primary_skill.as_deref());

    Ok(ShapeClassification {
        shape: DoctorShapeReport {
            kind,
            summary,
            root: root_label.to_owned(),
            primary_skill,
            skill_files: skill_files.iter().map(|path| path_to_slash(path)).collect(),
            plugin_roots,
            referenced_skill_paths,
            negative_signals,
            recommended_command,
        },
        counts,
    })
}

fn shape_only_report_from_classification(
    target: &str,
    source_kind: &str,
    staged_from: Option<String>,
    classification: ShapeClassification,
) -> DoctorReport {
    let shape = classification.shape;
    let counts = classification.counts;
    let issues = shape_issues(&shape, &counts);
    let basis = basis();
    let suggested_next_steps = shape_next_steps(&shape);
    DoctorReport {
        target: target.to_owned(),
        source_kind: source_kind.to_owned(),
        analysis_status: "shape_only".to_owned(),
        staged_from,
        shape,
        verdict: "shape-only: full single-skill doctor not run".to_owned(),
        score_model: score_model("not_evaluated", None, None),
        structural_score: 0,
        large_surface_percentage: 0,
        surface: SurfaceReport::default(),
        counts,
        issues,
        frontmatter_discovery_risk: None,
        agent_drift_risk: None,
        raw_activation_risk: None,
        contract_mitigation: None,
        workspace_agent_drift_risk: None,
        workspace_identity: None,
        packages: Vec::new(),
        basis,
        suggested_next_steps,
    }
}

fn frontmatter_issues(report: &FrontmatterDiscoveryRiskReport, location: &str) -> Vec<DoctorIssue> {
    report
        .conditions
        .iter()
        .filter(|condition| condition.score_delta > 0)
        .map(|condition| {
            with_location(
                issue(
                    &condition.id,
                    condition.level.as_str(),
                    frontmatter_issue_title(&condition.id),
                    condition
                        .evidence
                        .first()
                        .map(|evidence| evidence.text_preview.clone())
                        .unwrap_or_else(|| condition.consequence.clone()),
                    condition.basis_ids.iter().map(String::as_str).collect(),
                    &condition.recommended_action,
                    condition.score_delta.min(20),
                ),
                location.to_owned(),
            )
        })
        .collect()
}

fn frontmatter_issue_title(id: &str) -> &str {
    match id {
        "missing_or_malformed_frontmatter" => "Frontmatter is missing or malformed",
        "ambiguous_short_description" => "Frontmatter description is too ambiguous",
        "overbroad_description" => "Frontmatter description is overbroad",
        "description_listing_budget_risk" => "Frontmatter discovery text risks truncation",
        "manual_only_visibility" => "Skill is manual-only for automatic discovery",
        _ => "Frontmatter discovery risk",
    }
}

fn simple_shape_for_source_root(source_root: &Path) -> Result<DoctorShapeReport> {
    Ok(classify_local(source_root)?.shape)
}

fn collect_inventory_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_inventory_files_inner(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_inventory_files_inner(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if should_skip_inventory_dir(file_name) {
                continue;
            }
            collect_inventory_files_inner(root, &path, files)?;
        } else {
            files.push(strip_prefix(&path, root));
        }
    }
    Ok(())
}

fn inventory_counts(files: &[PathBuf]) -> DoctorCounts {
    DoctorCounts {
        total_files: files.len(),
        markdown_files: files.iter().filter(|path| extension_is(path, "md")).count(),
        code_files: files
            .iter()
            .filter(|path| looks_like_code_file(path))
            .count(),
        manifest_files: files
            .iter()
            .filter(|path| looks_like_manifest_file(path))
            .count(),
        ..DoctorCounts::default()
    }
}

fn plugin_roots_from_files(root: &Path, files: &[PathBuf]) -> Vec<DoctorPluginRootReport> {
    let mut roots = Vec::<PathBuf>::new();
    for file in files {
        if let Some(plugin_root) = plugin_root_from_marker_file(file) {
            roots.push(plugin_root);
        }
    }
    roots.sort();
    roots.dedup();
    roots
        .into_iter()
        .filter_map(|plugin_root| {
            let skill_files = files
                .iter()
                .filter(|file| path_is_prefix(&plugin_root.join("skills"), file))
                .filter(|file| {
                    file.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
                })
                .map(|path| path_to_slash(path))
                .collect::<Vec<_>>();
            if skill_files.is_empty() {
                return None;
            }
            Some(DoctorPluginRootReport {
                namespace: plugin_namespace(root, &plugin_root),
                path: path_to_slash(&plugin_root),
                skill_files,
            })
        })
        .collect()
}

fn plugin_root_from_marker_file(file: &Path) -> Option<PathBuf> {
    let file_name = file.file_name().and_then(|name| name.to_str())?;
    if file_name == ".mcp.json" || file_name == "CLAUDE.md" {
        return file.parent().map(Path::to_path_buf);
    }
    if !is_plugin_manifest_file_name(file_name) {
        return None;
    }
    let metadata_dir = file.parent()?;
    let metadata_dir_name = metadata_dir.file_name().and_then(|name| name.to_str())?;
    if is_plugin_metadata_dir_name(metadata_dir_name) {
        return metadata_dir.parent().map(Path::to_path_buf);
    }
    None
}

fn is_plugin_metadata_dir_name(name: &str) -> bool {
    name.trim_start_matches('.')
        .to_ascii_lowercase()
        .contains("plugin")
}

fn is_plugin_manifest_file_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "plugin.json"
            | "marketplace.json"
            | "manifest.json"
            | "package.json"
            | "plugin.yml"
            | "plugin.yaml"
            | "marketplace.yml"
            | "marketplace.yaml"
            | "manifest.yml"
            | "manifest.yaml"
            | "plugin.toml"
            | "manifest.toml"
    )
}

fn referenced_nested_skills(root_skill_content: &str, skill_files: &[PathBuf]) -> Vec<String> {
    let lowered = root_skill_content.to_ascii_lowercase();
    let mut referenced = Vec::new();
    for skill_file in skill_files {
        if skill_file.components().count() == 1 {
            continue;
        }
        let package = skill_file.parent().unwrap_or_else(|| Path::new(""));
        let package_slash = path_to_slash(package);
        let package_name = package
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let candidates = [
            package_slash.to_ascii_lowercase(),
            format!("{}/skill.md", package_slash.to_ascii_lowercase()),
            format!("/{package_name}").to_ascii_lowercase(),
            format!("`{package_name}`").to_ascii_lowercase(),
        ];
        if candidates
            .iter()
            .filter(|candidate| !candidate.is_empty())
            .any(|candidate| lowered.contains(candidate))
        {
            referenced.push(package_slash);
        }
    }
    referenced.sort();
    referenced.dedup();
    referenced
}

fn shape_summary(kind: &str, skill_count: usize, plugin_count: usize) -> String {
    match kind {
        "simple_skill" => "one atomic SKILL.md package; detailed doctor analysis can run".to_owned(),
        "entry_skill_with_subskills" => format!(
            "root SKILL.md plus {} nested skill package(s); this needs workspace mapping before per-skill doctor analysis",
            skill_count.saturating_sub(1)
        ),
        "plugin_workspace" => format!(
            "plugin-shaped workspace with {plugin_count} plugin namespace(s); preserve namespaces before import"
        ),
        "multi_skill_workspace" => {
            format!("{skill_count} SKILL.md packages discovered; map as a workspace")
        }
        "non_skill_repository" => "no SKILL.md entrypoint was discovered; doctor stopped before source-map parsing".to_owned(),
        _ => "unknown shape".to_owned(),
    }
}

fn recommended_shape_command(kind: &str, root: &str, primary_skill: Option<&str>) -> String {
    match kind {
        "simple_skill" => primary_skill
            .and_then(|skill| Path::new(skill).parent())
            .map(path_to_slash)
            .filter(|path| !path.is_empty())
            .map(|path| format!("/skillspec import {root}/{path}, compile it for <target>, install it, and prove it"))
            .unwrap_or_else(|| format!("/skillspec import {root}, compile it for <target>, install it, and prove it")),
        "entry_skill_with_subskills" | "plugin_workspace" | "multi_skill_workspace" => {
            format!("skillspec workspace map {root} --out <build-dir>/skillspec.workspace.yml")
        }
        "non_skill_repository" => {
            "Pass a skill folder or add SKILL.md before running skillspec doctor".to_owned()
        }
        _ => "Inspect the target manually".to_owned(),
    }
}

fn score_model(
    primary_score_field: &str,
    primary_score: Option<u8>,
    primary_level: Option<RiskLevel>,
) -> DoctorScoreModelReport {
    DoctorScoreModelReport {
        schema: "skillspec.doctor.score_model.v0".to_owned(),
        primary_score_label: "agent_follow_through_risk".to_owned(),
        primary_score_field: primary_score_field.to_owned(),
        primary_score,
        primary_level,
        risk_direction: "higher_score_means_higher_risk".to_owned(),
        readiness_label: readiness_label(primary_level).to_owned(),
        baseline_scope: "current_skill_shape_at_doctor_time".to_owned(),
        plain_language_summary: "Doctor estimates how likely an agent is to miss, reorder, improvise, use the wrong surface, or finish without proof because of the current skill shape.".to_owned(),
        not_measuring: vec![
            "domain expertise".to_owned(),
            "legal, medical, or factual correctness".to_owned(),
            "human usefulness".to_owned(),
            "author effort".to_owned(),
        ],
        basis_ids: vec![
            "reliability_gap_instruction_density".to_owned(),
            "reliability_gap_metadata_context_pressure".to_owned(),
            "contract_trace_activation_adherence_enforcement".to_owned(),
            "contract_trace_unproven_verdict".to_owned(),
        ],
    }
}

fn readiness_label(level: Option<RiskLevel>) -> &'static str {
    match level {
        Some(RiskLevel::Low) => "strong",
        Some(RiskLevel::Medium) => "moderate",
        Some(RiskLevel::High) => "low",
        Some(RiskLevel::Critical) => "very_low",
        None => "not_evaluated",
    }
}

fn shape_issues(shape: &DoctorShapeReport, counts: &DoctorCounts) -> Vec<DoctorIssue> {
    let mut issues = Vec::new();
    match shape.kind.as_str() {
        "entry_skill_with_subskills" => issues.push(with_location(
            issue(
                "workspace_shape_entry_with_subskills",
                "medium",
                "Root skill references a multi-skill workspace",
                format!(
                    "{} SKILL.md file(s) were discovered; referenced nested packages: {}.",
                    shape.skill_files.len(),
                    display_list(&shape.referenced_skill_paths)
                ),
                vec!["contract_trace_static_well_formedness"],
                "Run `skillspec workspace map` so each SKILL.md is treated as an atomic package and dependencies are explicit.",
                0,
            ),
            shape.root.clone(),
        )),
        "plugin_workspace" => issues.push(with_location(
            issue(
                "plugin_workspace_shape",
                "medium",
                "Plugin-shaped workspace requires namespace preservation",
                format!(
                    "Detected {} plugin root(s) and {} SKILL.md package(s).",
                    shape.plugin_roots.len(),
                    shape.skill_files.len()
                ),
                vec!["contract_trace_static_well_formedness"],
                "Run `skillspec workspace map`; do not flatten skill names across plugin namespaces.",
                0,
            ),
            shape.root.clone(),
        )),
        "multi_skill_workspace" => issues.push(with_location(
            issue(
                "multi_skill_workspace_shape",
                "medium",
                "Multiple atomic skill packages found",
                format!("Detected {} SKILL.md file(s).", shape.skill_files.len()),
                vec!["contract_trace_static_well_formedness"],
                "Run `skillspec workspace map` before fanout import or per-package doctor analysis.",
                0,
            ),
            shape.root.clone(),
        )),
        "non_skill_repository" => issues.push(with_location(
            issue(
                "no_skill_entrypoint",
                "high",
                "Target is not shaped like an agent skill",
                format!(
                    "No SKILL.md was found. Static inventory saw {} Markdown file(s), {} code file(s), and {} manifest file(s).",
                    counts.markdown_files, counts.code_files, counts.manifest_files
                ),
                vec!["contract_trace_static_well_formedness"],
                "Pass a folder containing SKILL.md, a GitHub skill folder URL, or add a SKILL.md entrypoint before running doctor.",
                0,
            ),
            shape.root.clone(),
        )),
        _ => {}
    }
    issues
}

fn recommended_simple_next_action(source_root: &Path, has_valid_skill_spec: bool) -> String {
    let source = source_root.display();
    if has_valid_skill_spec {
        format!("skillspec install skill {source} --target <target> --retire-existing")
    } else {
        format!("/skillspec import {source}, compile it for <target>, install it, and prove it")
    }
}

fn apply_simple_recommended_action(report: &mut DoctorReport, source_root: &Path) {
    let has_valid_skill_spec = report.contract_mitigation.is_some();
    report.shape.recommended_command =
        recommended_simple_next_action(source_root, has_valid_skill_spec);
}

fn shape_next_steps(shape: &DoctorShapeReport) -> Vec<String> {
    match shape.kind.as_str() {
        "entry_skill_with_subskills" | "plugin_workspace" | "multi_skill_workspace" => vec![
            format!(
                "Map the workspace before import: `skillspec workspace map {} --out <build-dir>/skillspec.workspace.yml`.",
                shape.root
            ),
            "From the harness, ask `/skillspec map this repo and import the packages safely` when you want the agent to preserve package shape, plugin namespaces, and dependencies.".to_owned(),
            "After fanout import, read the converge/alignment reports: decision replay should pass, required package steps should be proven, and missing proof should be explicit.".to_owned(),
            "Optionally publish the doctor baseline, generated `skill.spec.yml` files, and alignment reports with the source repo so reviewers can see what changed.".to_owned(),
            "Restart the harness after install, then use the SkillSpec-backed skills normally rather than invoking internal CLI steps by hand.".to_owned(),
            "Do not flatten the repo into one skill; process each discovered SKILL.md as an atomic package and converge dependencies before install.".to_owned(),
        ],
        "non_skill_repository" => vec![
            "Stop here: doctor did not find a SKILL.md entrypoint.".to_owned(),
            "Select a folder that contains SKILL.md, pass a GitHub skill-folder URL, or add a SKILL.md before importing.".to_owned(),
            "Do not run import or workspace fanout against an ordinary code repository until a skill entrypoint exists.".to_owned(),
        ],
        _ => vec![
            "Use the recommended action above instead of rerunning doctor on the same target."
                .to_owned(),
        ],
    }
}

fn shape_root(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

fn should_skip_inventory_dir(name: &str) -> bool {
    if is_plugin_metadata_dir_name(name) {
        return false;
    }
    if name.starts_with('.') {
        return true;
    }
    matches!(
        name,
        "__pycache__" | "target" | "node_modules" | "vendor" | "dist" | "build"
    )
}

fn path_has_skipped_dir(path: &Path) -> bool {
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        if components.peek().is_none() {
            break;
        }
        let std::path::Component::Normal(value) = component else {
            continue;
        };
        let Some(name) = value.to_str() else {
            continue;
        };
        if should_skip_inventory_dir(name) {
            return true;
        }
    }
    false
}

fn looks_like_code_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "rs" | "py"
                    | "js"
                    | "ts"
                    | "tsx"
                    | "jsx"
                    | "go"
                    | "java"
                    | "kt"
                    | "swift"
                    | "c"
                    | "cc"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "rb"
                    | "php"
                    | "sh"
                    | "bash"
                    | "zsh"
            )
        })
        .unwrap_or(false)
}

fn looks_like_manifest_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "package.json"
            | "cargo.toml"
            | "pyproject.toml"
            | "requirements.txt"
            | "go.mod"
            | "pom.xml"
            | "build.gradle"
            | "gemfile"
            | ".mcp.json"
            | "plugin.json"
    )
}

fn extension_is(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn plugin_namespace(root: &Path, plugin_root: &Path) -> String {
    plugin_manifest_namespace(root, plugin_root)
        .or_else(|| {
            plugin_root
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .map(|value| slugify(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "plugin".to_owned())
}

fn plugin_manifest_namespace(root: &Path, plugin_root: &Path) -> Option<String> {
    let plugin_root = root.join(plugin_root);
    let mut candidates = Vec::new();
    let entries = fs::read_dir(plugin_root).ok()?;
    for entry in entries.filter_map(|entry| entry.ok()) {
        let child = entry.path();
        if !child.is_dir()
            || !entry
                .file_name()
                .to_str()
                .is_some_and(is_plugin_metadata_dir_name)
        {
            continue;
        }
        let Ok(files) = fs::read_dir(child) else {
            continue;
        };
        for file in files.filter_map(|file| file.ok()) {
            if file.path().is_file()
                && file
                    .file_name()
                    .to_str()
                    .is_some_and(is_plugin_manifest_file_name)
            {
                candidates.push(file.path());
            }
        }
    }
    candidates.sort_by_key(|path| plugin_manifest_priority(path));
    candidates
        .into_iter()
        .find_map(|path| namespace_from_manifest_file(&path))
}

fn plugin_manifest_priority(path: &Path) -> usize {
    match path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
        .as_deref()
    {
        Some("plugin.json") => 0,
        Some("marketplace.json") => 1,
        Some("manifest.json") => 2,
        Some("package.json") => 3,
        _ => 4,
    }
}

fn namespace_from_manifest_file(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let parsed = if matches!(extension.as_str(), "yaml" | "yml") {
        serde_yaml::from_str::<serde_yaml::Value>(&content).ok()
    } else if extension == "json" {
        serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .and_then(|value| serde_yaml::to_value(value).ok())
    } else {
        None
    }?;
    parsed
        .get("name")
        .or_else(|| parsed.get("id"))
        .or_else(|| parsed.get("title"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_owned()
}

fn strip_prefix(path: &Path, base: &Path) -> PathBuf {
    path.strip_prefix(base).unwrap_or(path).to_path_buf()
}

fn path_is_prefix(prefix: &Path, path: &Path) -> bool {
    path == prefix || path.starts_with(prefix)
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str().map(str::to_owned),
            std::path::Component::CurDir => Some(".".to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn display_list(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

fn rewrite_remote_locations(report: &mut DoctorReport, staged_skill_path: &Path, target: &str) {
    let Some(prefix) = staged_skill_path.to_str() else {
        return;
    };
    let target = target.trim_end_matches('/');
    rewrite_prefixed_string(&mut report.shape.root, prefix, target);
    rewrite_prefixed_string(&mut report.shape.recommended_command, prefix, target);
    for step in &mut report.suggested_next_steps {
        rewrite_prefixed_string(step, prefix, target);
    }
    for issue in &mut report.issues {
        if let Some(location) = &mut issue.location {
            rewrite_prefixed_string(location, prefix, target);
        }
    }
}

fn rewrite_prefixed_string(value: &mut String, prefix: &str, target: &str) {
    if let Some(suffix) = value.strip_prefix(prefix) {
        *value = format!("{target}{suffix}");
    } else if value.contains(prefix) {
        *value = value.replace(prefix, target);
    }
}

fn rewrite_remote_error(error: Error, staged_skill_path: &Path, target: &str) -> Error {
    let Error::InvalidInput { message } = error else {
        return error;
    };
    let Some(prefix) = staged_skill_path.to_str() else {
        return Error::InvalidInput { message };
    };
    Error::InvalidInput {
        message: message.replace(prefix, target.trim_end_matches('/')),
    }
}

fn load_skill_body(map: &SourceMap, source_root: &Path) -> Result<SkillBody> {
    let skill_files = map
        .files
        .iter()
        .filter(|file| {
            Path::new(&file.path)
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        })
        .collect::<Vec<_>>();
    if skill_files.is_empty() {
        return Err(Error::InvalidInput {
            message: format!(
                "skillspec doctor expected a prose skill folder or SKILL.md file under {}",
                source_root.display()
            ),
        });
    }
    if skill_files.len() > 1 {
        let paths = skill_files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(Error::InvalidInput {
            message: format!(
                "skillspec doctor requires exactly one SKILL.md; found {} under {}: {}",
                skill_files.len(),
                source_root.display(),
                paths
            ),
        });
    }
    let file = skill_files[0];
    let path = source_root.join(&file.path);
    let content = fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })?;
    let frontmatter_node = map
        .nodes
        .iter()
        .find(|node| node.file == file.id && node.kind == "frontmatter");
    let (frontmatter, body, body_start_line) = match frontmatter_node {
        Some(node) => {
            let end = node.byte_range.map(|range| range[1]).unwrap_or(0);
            let line = node.line_range.map(|range| range[1] + 1).unwrap_or(1);
            (
                content.get(..end).unwrap_or("").to_owned(),
                content.get(end..).unwrap_or("").to_owned(),
                line,
            )
        }
        None => (String::new(), content.clone(), 1),
    };
    Ok(SkillBody {
        path,
        file_id: file.id.clone(),
        body,
        frontmatter,
        body_start_line,
    })
}

fn surface_report(map: &SourceMap, skill: &SkillBody) -> SurfaceReport {
    let referenced = map
        .references
        .iter()
        .filter_map(|reference| reference.resolved_file.as_deref())
        .collect::<BTreeSet<_>>();
    let frontmatter_bytes = skill.frontmatter.len();
    let frontmatter_lines = skill.frontmatter.lines().count();
    let activation_bytes = skill.body.len();
    let activation_lines = skill.body.lines().count();
    let activation_estimated_tokens = metrics::estimate_tokens(&skill.body);
    let deferred_files = map
        .files
        .iter()
        .filter(|file| file.id != skill.file_id && file.load_status == SourceFileLoadStatus::Loaded)
        .count();
    let deferred_bytes = map
        .files
        .iter()
        .filter(|file| file.id != skill.file_id && file.load_status == SourceFileLoadStatus::Loaded)
        .map(|file| file.bytes)
        .sum::<usize>();
    let unmapped_files = map
        .files
        .iter()
        .filter(|file| file.id != skill.file_id)
        .filter(|file| {
            file.kind == SourceFileKind::Code
                || file.kind == SourceFileKind::Manifest
                || file.kind == SourceFileKind::Other
        })
        .filter(|file| !referenced.contains(file.path.as_str()))
        .count();
    SurfaceReport {
        frontmatter_bytes,
        frontmatter_lines,
        activation_bytes,
        activation_lines,
        activation_estimated_tokens,
        deferred_bytes,
        deferred_files,
        unmapped_files,
    }
}

fn counts(map: &SourceMap, skill: &SkillBody) -> DoctorCounts {
    let markdown_files = map
        .files
        .iter()
        .filter(|file| file.kind == SourceFileKind::Markdown)
        .count();
    let code_files = map
        .files
        .iter()
        .filter(|file| file.kind == SourceFileKind::Code)
        .count();
    let manifest_files = map
        .files
        .iter()
        .filter(|file| file.kind == SourceFileKind::Manifest)
        .count();
    let code_nodes = map
        .nodes
        .iter()
        .filter(|node| node.file == skill.file_id && node.kind == "code")
        .collect::<Vec<_>>();
    let code_blocks_in_skill = code_nodes.len();
    let unlabeled_code_blocks_in_skill = code_nodes
        .iter()
        .filter(|node| {
            node.language
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        })
        .count();
    let modal_nodes = map
        .classifications
        .iter()
        .filter(|class| class.kind == SourceClassificationKind::ModalObligation)
        .filter_map(|class| map.nodes.iter().find(|node| node.id == class.target))
        .filter(|node| node.file == skill.file_id)
        .collect::<Vec<_>>();
    let modal_obligations = modal_nodes.len();
    let late_line =
        skill.body_start_line + (skill.body.lines().count() * PRIMACY_LINE_PERCENT / 100);
    let late_modal_obligations = modal_nodes
        .iter()
        .filter(|node| {
            node.line_range
                .map(|range| range[0] >= late_line)
                .unwrap_or(false)
        })
        .count();
    let numbered_steps = skill
        .body
        .lines()
        .filter(|line| looks_like_numbered_step(line))
        .count();
    let dependency_mentions = map
        .classifications
        .iter()
        .filter(|class| class.kind == SourceClassificationKind::DependencyMention)
        .count();
    let missing_local_references = map
        .references
        .iter()
        .filter(|reference| reference.target_kind == SourceReferenceKind::LocalFile)
        .filter(|reference| reference.resolved_file.is_none())
        .count();
    DoctorCounts {
        total_files: map.files.len(),
        markdown_files,
        code_files,
        manifest_files,
        code_blocks_in_skill,
        unlabeled_code_blocks_in_skill,
        modal_obligations,
        late_modal_obligations,
        numbered_steps,
        dependency_mentions,
        missing_local_references,
    }
}

fn skill_code_bytes(map: &SourceMap, skill_file_id: &str) -> usize {
    map.nodes
        .iter()
        .filter(|node| node.file == skill_file_id && node.kind == "code")
        .filter_map(|node| {
            node.byte_range
                .map(|range| range[1].saturating_sub(range[0]))
        })
        .sum()
}

fn looks_like_numbered_step(line: &str) -> bool {
    let trimmed = line.trim_start();
    let digits = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();
    digits > 0 && trimmed.chars().nth(digits) == Some('.')
}

fn operational_prose(body: &str) -> bool {
    let lowered = body.to_ascii_lowercase();
    [
        " run ",
        " execute ",
        " use ",
        " call ",
        " fetch ",
        " open ",
        " click ",
        " install ",
        " create ",
        " generate ",
        " browse ",
        " shell ",
        " python ",
        " bash ",
        " api ",
    ]
    .iter()
    .any(|signal| lowered.contains(signal))
}

fn issue(
    id: &str,
    severity: &str,
    title: &str,
    evidence: String,
    basis: Vec<&str>,
    remediation: &str,
    score_penalty: u8,
) -> DoctorIssue {
    DoctorIssue {
        id: id.to_owned(),
        severity: severity.to_owned(),
        title: title.to_owned(),
        evidence,
        basis: basis.into_iter().map(str::to_owned).collect(),
        remediation: remediation.to_owned(),
        score_penalty,
        location: None,
    }
}

fn with_location(mut issue: DoctorIssue, location: String) -> DoctorIssue {
    issue.location = Some(location);
    issue
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

fn verdict(score: u8) -> String {
    match score {
        80..=100 => "low reliability debt".to_owned(),
        60..=79 => "medium reliability debt".to_owned(),
        40..=59 => "high reliability debt".to_owned(),
        _ => "critical reliability debt".to_owned(),
    }
}

fn basis() -> Vec<DoctorBasis> {
    vec![
        DoctorBasis {
            id: "reliability_gap_instruction_density".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Skills Reliability Gap".to_owned(),
            source: "docs/overview/00-skills-reliability-gap.md §3.1".to_owned(),
            claim: "Large, dense activation bodies increase dropped-step risk and later instructions suffer primacy bias.".to_owned(),
        },
        DoctorBasis {
            id: "reliability_gap_metadata_context_pressure".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Skills Reliability Gap".to_owned(),
            source: "docs/overview/00-skills-reliability-gap.md §3.2".to_owned(),
            claim: "Skill frontmatter metadata and activated bodies consume context; broad surfaces degrade routing and adherence.".to_owned(),
        },
        DoctorBasis {
            id: "reliability_gap_implicit_environment_contract".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Skills Reliability Gap".to_owned(),
            source: "docs/overview/00-skills-reliability-gap.md §3.3".to_owned(),
            claim: "Scripts and snippets carry dependency contracts whether or not the skill declares them.".to_owned(),
        },
        DoctorBasis {
            id: "reliability_gap_no_execution_guarantees".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Skills Reliability Gap".to_owned(),
            source: "docs/overview/00-skills-reliability-gap.md §3.5".to_owned(),
            claim: "Prose instructions and embedded snippets are guidance unless moved into checkable or enforced surfaces.".to_owned(),
        },
        DoctorBasis {
            id: "reliability_gap_unfilled_requirement".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Skills Reliability Gap".to_owned(),
            source: "docs/overview/00-skills-reliability-gap.md §5".to_owned(),
            claim: "Reliable skills need a portable, machine-checkable account of intended behavior, dependencies, and proof.".to_owned(),
        },
        DoctorBasis {
            id: "contract_trace_activation_adherence_enforcement".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Contract Trace Methodology".to_owned(),
            source: "docs/overview/08-contract-trace-methodology.md §3.1".to_owned(),
            claim: "Activation, adherence, and enforcement are separate gates; static prose mostly influences adherence, not enforcement.".to_owned(),
        },
        DoctorBasis {
            id: "contract_trace_behavioral_contract".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Contract Trace Methodology".to_owned(),
            source: "docs/overview/08-contract-trace-methodology.md §4.1".to_owned(),
            claim: "A behavioral contract makes steering, dependencies, forbids, and tests statically checkable.".to_owned(),
        },
        DoctorBasis {
            id: "contract_trace_static_well_formedness".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Contract Trace Methodology".to_owned(),
            source: "docs/overview/08-contract-trace-methodology.md §4.1".to_owned(),
            claim: "Reference closure, reachability, and typed structure are static pre-filters before execution.".to_owned(),
        },
        DoctorBasis {
            id: "contract_trace_unproven_verdict".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Contract Trace Methodology".to_owned(),
            source: "docs/overview/08-contract-trace-methodology.md §4.3".to_owned(),
            claim: "Missing trace evidence should be reported as unproven, not inferred as success.".to_owned(),
        },
        DoctorBasis {
            id: "claude_skill_frontmatter_discovery".to_owned(),
            kind: "harness_documentation".to_owned(),
            citation: "Claude Code skills documentation".to_owned(),
            source: "https://code.claude.com/docs/en/skills".to_owned(),
            claim: "Skill frontmatter, especially description and when_to_use, is used for automatic discovery and can be shortened under listing budget pressure.".to_owned(),
        },
        DoctorBasis {
            id: "skilldex_format_conformance".to_owned(),
            kind: "research_paper".to_owned(),
            citation: "Skilldex: A Package Manager and Registry for Agent Skill Packages with Hierarchical Scope-Based Distribution".to_owned(),
            source: "https://arxiv.org/abs/2604.16911".to_owned(),
            claim: "Skill package tooling can score frontmatter validity and description specificity as static package-quality diagnostics.".to_owned(),
        },
        DoctorBasis {
            id: "skill_metadata_supply_chain".to_owned(),
            kind: "research_paper".to_owned(),
            citation: "Under the Hood of SKILL.md: Semantic Supply-chain Attacks on AI Agent Skill Registry".to_owned(),
            source: "https://arxiv.org/abs/2605.11418".to_owned(),
            claim: "Natural-language skill metadata and instructions affect which skills are surfaced, selected, and loaded.".to_owned(),
        },
        DoctorBasis {
            id: "ruler_effective_context".to_owned(),
            kind: "research_paper".to_owned(),
            citation: "RULER: What's the Real Context Size of Your Long-Context Language Models?".to_owned(),
            source: "https://arxiv.org/abs/2404.06654".to_owned(),
            claim: "Reliable usable context can be smaller than advertised context length and varies by task.".to_owned(),
        },
        DoctorBasis {
            id: "tiktoken_token_accounting".to_owned(),
            kind: "tooling_documentation".to_owned(),
            citation: "OpenAI tiktoken and token counting guide".to_owned(),
            source: "https://github.com/openai/tiktoken".to_owned(),
            claim: "Model/encoding-aware tokenization is the correct measurement surface for token load.".to_owned(),
        },
        DoctorBasis {
            id: "skillsbench_focused_skills".to_owned(),
            kind: "research_paper".to_owned(),
            citation: "Can Skills Make AI Agents Competent?".to_owned(),
            source: "https://arxiv.org/abs/2602.12670".to_owned(),
            claim: "Focused, checkable skills are a better risk target than broad comprehensive documentation.".to_owned(),
        },
        DoctorBasis {
            id: "skillspec_local_reliability_gap".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Skills Reliability Gap".to_owned(),
            source: "docs/overview/00-skills-reliability-gap.md".to_owned(),
            claim: "Large activation bodies, implicit dependencies, mixed code/instructions, missing contracts, and missing proof surfaces create reliability debt.".to_owned(),
        },
        DoctorBasis {
            id: "skillspec_local_contract_trace".to_owned(),
            kind: "local_methodology".to_owned(),
            citation: "SkillSpec Contract Trace Methodology".to_owned(),
            source: "docs/overview/08-contract-trace-methodology.md".to_owned(),
            claim: "Route choice, forbids, dependencies, tool boundaries, tests, and trace/progress proof are the checkable surfaces that reduce drift.".to_owned(),
        },
    ]
}

fn raw_activation_risk(
    surface: &SurfaceReport,
    counts: &DoctorCounts,
    structural_score: u8,
) -> RawActivationRiskReport {
    let score = 100u8.saturating_sub(structural_score);
    let level = RiskLevel::from_score(score);
    RawActivationRiskReport {
        score,
        level,
        activation_estimated_tokens: surface.activation_estimated_tokens,
        activation_lines: surface.activation_lines,
        modal_obligations: counts.modal_obligations,
        late_modal_obligations: counts.late_modal_obligations,
        summary: format!(
            "{} static activation risk before considering structured contract mitigation",
            level.as_str()
        ),
    }
}

fn contract_mitigation(
    spec_path: &Path,
    spec: &SkillSpec,
    raw_risk_score: u8,
) -> ContractMitigationReport {
    let level = contract_mitigation_level(spec);
    let reduction = match level {
        ContractMitigationLevel::Strong => 30,
        ContractMitigationLevel::Partial => 18,
        ContractMitigationLevel::Weak => 8,
    };
    let residual_risk_score = raw_risk_score.saturating_sub(reduction);
    let residual_risk_level = RiskLevel::from_score(residual_risk_score);
    ContractMitigationReport {
        present: true,
        spec_path: spec_path.display().to_string(),
        routes: spec.routes.len(),
        rules: spec.rules.len(),
        commands: spec.commands.len(),
        dependencies: spec.dependencies.len(),
        tests: spec.tests.len(),
        level,
        residual_risk_score,
        residual_risk_level,
        summary: format!(
            "Valid skill.spec.yml provides {} contract mitigation; residual risk stays {} until the activated trampoline is thin.",
            level.as_str(),
            residual_risk_level.as_str()
        ),
    }
}

fn contract_mitigation_level(spec: &SkillSpec) -> ContractMitigationLevel {
    let core = usize::from(!spec.routes.is_empty())
        + usize::from(!spec.rules.is_empty())
        + usize::from(!spec.commands.is_empty())
        + usize::from(!spec.dependencies.is_empty())
        + usize::from(!spec.tests.is_empty());
    match core {
        5 => ContractMitigationLevel::Strong,
        3..=4 => ContractMitigationLevel::Partial,
        _ => ContractMitigationLevel::Weak,
    }
}

fn next_steps(
    source_root: &Path,
    has_valid_skill_spec: bool,
    has_deps_toml: bool,
    has_structured_dependencies: bool,
    raw_activation_score: u8,
) -> Vec<String> {
    let mut steps = Vec::new();
    let source = source_root.display();
    if has_valid_skill_spec {
        steps.push(format!(
            "Install the SkillSpec-backed skill into the harness you use: `skillspec install skill {source} --target <codex|agents|claude-local> --retire-existing`."
        ));
        steps.push("Restart the harness, then invoke the skill normally; the generated loader should ask the CLI for route guidance instead of loading the full contract into context.".to_owned());
        steps.push("Read the final alignment summary after important runs: decision replay, requirements proven, missing proof, forbidden actions, and token/wall-clock metrics when available.".to_owned());
        if raw_activation_score <= 24 {
            steps.push("Keep the activated SKILL.md trampoline thin and let `skillspec run-loop --guide agent` drive route, gate, resume, and proof navigation.".to_owned());
        } else {
            steps.push("Thin the activated SKILL.md trampoline and let `skillspec run-loop --guide agent` drive route, gate, resume, and proof navigation.".to_owned());
        }
        steps.push("Use `skillspec run-loop <skill.spec.yml> --input '<task>' --trace-dir <dir> --guide agent` instead of loading the full spec or duplicating policy in prose.".to_owned());
        steps.push("Optionally publish the doctor baseline and alignment report with the skill repo so reviewers can compare current-shape risk with proven execution.".to_owned());
    } else {
        steps.push(format!(
            "Capture this baseline before changing the skill: `skillspec doctor {source} --markdown > .skillspec/reports/skillspec-doctor-baseline.md`."
        ));
        steps.push(
            "Install the `skillspec` skill into your harness if it is not already installed."
                .to_owned(),
        );
        steps.push(format!(
            "From the harness, ask `/skillspec import {source}, compile it for <target>, verify it, test it, and prove it. Print the alignment summary.`"
        ));
        steps.push("Read the alignment summary before trusting the port: decision replay should pass, required steps should be proven, missing proof should be explicit, and forbidden actions should show no violations.".to_owned());
        steps.push("Optionally publish the baseline doctor report, generated `skill.spec.yml`, compiled loader, and alignment report with the source repo or PR.".to_owned());
        steps.push("Restart the harness after install, then try the SkillSpec-backed skill normally on a real task.".to_owned());
        steps.push(format!(
            "For a CLI-only port, run `skillspec source map {source} --out <draft-dir>/.skillspec/source-map`, then `skillspec import-skill {source} --out <draft-dir>/skill.spec.yml --source-map <draft-dir>/.skillspec/source-map/source-map.json`."
        ));
    }
    if !has_deps_toml && !has_structured_dependencies {
        steps.push("Create or complete `deps.toml`; preserve dependency authority, local status, install risk, and degraded proof impact.".to_owned());
    }
    if has_valid_skill_spec {
        steps.push("Keep operational precision in routes, rules, forbids, command templates, tests, progress, and alignment proof, not in the trampoline prose.".to_owned());
    } else {
        steps.push("Promote operational prose into routes, rules, forbids, command templates, scenario tests, and trace/progress proof obligations.".to_owned());
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::{
        inspect_target, rewrite_remote_error, rewrite_remote_locations, DoctorCounts, DoctorReport,
        DoctorShapeReport, SurfaceReport,
    };
    use skillspec_core::error::Error;
    use std::path::Path;

    #[test]
    fn doctor_does_not_treat_trailing_slash_local_path_as_github_shorthand() {
        let error = inspect_target("definitely-missing-local-skill-dir/skillspec/")
            .unwrap_err()
            .to_string();

        assert!(error.contains("does not exist locally"));
        assert!(error.contains("without a trailing slash"));
        assert!(!error.contains("git failed during clone remote skill repository"));
        assert!(!error.contains("github.com/definitely-missing-local-skill-dir/skillspec"));
    }

    #[test]
    fn rewrites_remote_error_locations() {
        let error = rewrite_remote_error(
            Error::InvalidInput {
                message: "found multiple SKILL.md under /tmp/skillspec-doctor/repo/skills"
                    .to_owned(),
            },
            Path::new("/tmp/skillspec-doctor/repo/skills"),
            "https://github.com/owner/repo/tree/main/skills",
        )
        .to_string();
        assert!(error.contains("https://github.com/owner/repo/tree/main/skills"));
        assert!(!error.contains("/tmp/skillspec-doctor"));
    }

    #[test]
    fn rewrites_remote_folder_locations_without_duplicating_requested_path() {
        let mut report = DoctorReport {
            target: "/tmp/skillspec-doctor/repo/skills/pdf".to_owned(),
            source_kind: "local".to_owned(),
            analysis_status: "full".to_owned(),
            staged_from: None,
            shape: DoctorShapeReport {
                kind: "simple_skill".to_owned(),
                summary: "one atomic SKILL.md package".to_owned(),
                root: "/tmp/skillspec-doctor/repo/skills/pdf".to_owned(),
                primary_skill: Some("SKILL.md".to_owned()),
                skill_files: vec!["SKILL.md".to_owned()],
                plugin_roots: Vec::new(),
                referenced_skill_paths: Vec::new(),
                negative_signals: Vec::new(),
                recommended_command:
                    "skillspec install skill /tmp/skillspec-doctor/repo/skills/pdf --target <target> --retire-existing"
                        .to_owned(),
            },
            verdict: "low reliability debt".to_owned(),
            score_model: super::score_model("agent_drift_risk.score", Some(0), Some(super::RiskLevel::Low)),
            structural_score: 100,
            large_surface_percentage: 0,
            surface: SurfaceReport::default(),
            counts: DoctorCounts::default(),
            issues: Vec::new(),
            frontmatter_discovery_risk: None,
            agent_drift_risk: None,
            raw_activation_risk: None,
            contract_mitigation: None,
            workspace_agent_drift_risk: None,
            workspace_identity: None,
            packages: Vec::new(),
            basis: Vec::new(),
            suggested_next_steps: vec![
                "Install /tmp/skillspec-doctor/repo/skills/pdf into the harness".to_owned(),
            ],
        };

        rewrite_remote_locations(
            &mut report,
            Path::new("/tmp/skillspec-doctor/repo/skills/pdf"),
            "https://github.com/owner/repo/tree/main/skills/pdf",
        );

        assert_eq!(
            report.shape.root,
            "https://github.com/owner/repo/tree/main/skills/pdf"
        );
        assert!(report
            .shape
            .recommended_command
            .contains("https://github.com/owner/repo/tree/main/skills/pdf"));
        assert!(!report
            .shape
            .recommended_command
            .contains("skills/pdf/skills/pdf"));
        assert!(report.suggested_next_steps[0]
            .contains("https://github.com/owner/repo/tree/main/skills/pdf"));
    }
}
