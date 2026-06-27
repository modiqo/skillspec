use super::types::{
    DoctorPackageRiskReport, RiskCondition, RiskConditionKind, RiskConfidence, RiskEvidence,
    RiskLevel, WorkspaceAgentDriftRiskReport,
};
use super::{
    basis, display_list, frontmatter, issue, metrics, path_to_slash, shape_root, slugify,
    with_location, DoctorIssue, DoctorReport, DoctorShapeReport, Error, Result,
    ShapeClassification, SurfaceReport, LARGE_BODY_LINES, LARGE_BODY_TOKENS,
};
use super::{risk, severity_rank};
use crate::remote_source;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn inspect_local_target(
    path: &Path,
    classification: ShapeClassification,
) -> Result<DoctorReport> {
    let root = shape_root(path);
    let mut skill_texts = Vec::new();
    for skill_file in &classification.shape.skill_files {
        let path = root.join(skill_file);
        let content = fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        skill_texts.push((skill_file.clone(), content));
    }
    Ok(report_from_skill_texts(
        &path.display().to_string(),
        "local",
        None,
        classification,
        skill_texts,
    ))
}

pub(super) fn inspect_remote_target(
    target: &str,
    checkout_dir: &Path,
    classification: ShapeClassification,
) -> Result<DoctorReport> {
    let mut skill_texts = Vec::new();
    for skill_file in &classification.shape.skill_files {
        skill_texts.push((
            skill_file.clone(),
            remote_source::git_show_text(checkout_dir, skill_file)?,
        ));
    }
    Ok(report_from_skill_texts(
        target,
        "remote_github",
        None,
        classification,
        skill_texts,
    ))
}

fn report_from_skill_texts(
    target: &str,
    source_kind: &str,
    staged_from: Option<String>,
    classification: ShapeClassification,
    skill_texts: Vec<(String, String)>,
) -> DoctorReport {
    let shape = classification.shape;
    let counts = classification.counts;
    let mut package_reports = Vec::new();
    for (skill_path, content) in skill_texts {
        package_reports.push(package_report(&shape, &skill_path, &content));
    }

    let mut issues = super::shape_issues(&shape, &counts);
    issues.extend(workspace_issues(&shape, &package_reports));
    issues.sort_by_key(|issue| severity_rank(&issue.severity));
    let penalty = issues
        .iter()
        .map(|issue| usize::from(issue.score_penalty))
        .sum::<usize>()
        .min(100);
    let structural_score = u8::try_from(100usize.saturating_sub(penalty)).unwrap_or(0);
    let workspace_agent_drift_risk = workspace_agent_drift_risk(&shape, &package_reports, &issues);
    let surface = workspace_surface(&package_reports);
    let basis = basis();
    let suggested_next_steps = super::shape_next_steps(&shape);
    let score_model = super::score_model(
        "workspace_agent_drift_risk.score",
        Some(workspace_agent_drift_risk.score),
        Some(workspace_agent_drift_risk.level),
    );

    DoctorReport {
        target: target.to_owned(),
        source_kind: source_kind.to_owned(),
        analysis_status: "workspace".to_owned(),
        staged_from,
        shape,
        verdict: format!(
            "workspace risk: {}",
            workspace_agent_drift_risk.level.as_str()
        ),
        score_model,
        structural_score,
        large_surface_percentage: 0,
        surface,
        counts,
        issues,
        frontmatter_discovery_risk: None,
        agent_drift_risk: None,
        raw_activation_risk: None,
        contract_mitigation: None,
        workspace_agent_drift_risk: Some(workspace_agent_drift_risk),
        packages: package_reports,
        basis,
        suggested_next_steps,
    }
}

fn package_report(
    shape: &DoctorShapeReport,
    skill_path: &str,
    content: &str,
) -> DoctorPackageRiskReport {
    let sections = frontmatter::split_skill(content);
    let frontmatter_risk = frontmatter::analyze(Path::new(skill_path), &sections);
    let activation_estimated_tokens = metrics::estimate_tokens(&sections.body);
    let activation_lines = sections.body.lines().count();
    let package_issues = package_issues(skill_path, activation_estimated_tokens, activation_lines);
    let package_penalty = package_issues
        .iter()
        .map(|issue| usize::from(issue.score_penalty))
        .sum::<usize>()
        .saturating_add(usize::from(frontmatter_risk.score.min(30)))
        .min(100);
    let structural_score = u8::try_from(100usize.saturating_sub(package_penalty)).unwrap_or(0);
    let basis = basis();
    let agent_drift_risk = risk::agent_report(
        structural_score,
        &package_issues,
        Some(frontmatter_risk.clone()),
        &basis,
    );
    let plugin_name = plugin_for_skill_path(shape, skill_path);
    let public_name = frontmatter_risk
        .fields
        .name
        .clone()
        .unwrap_or_else(|| fallback_public_name(skill_path));
    let install_slug = install_slug(plugin_name.as_deref(), skill_path, &public_name);
    DoctorPackageRiskReport {
        package_id: package_id(plugin_name.as_deref(), skill_path, &public_name),
        public_name,
        plugin_name,
        install_slug,
        path: skill_path.to_owned(),
        shape_role: shape_role(shape, skill_path),
        entrypoint: "SKILL.md".to_owned(),
        structural_score,
        activation_estimated_tokens,
        activation_lines,
        frontmatter_discovery_risk: frontmatter_risk,
        agent_drift_risk,
    }
}

fn package_issues(
    skill_path: &str,
    activation_estimated_tokens: usize,
    activation_lines: usize,
) -> Vec<DoctorIssue> {
    let mut issues = Vec::new();
    if activation_estimated_tokens > LARGE_BODY_TOKENS || activation_lines > LARGE_BODY_LINES {
        issues.push(with_location(
            issue(
                "activation_token_load",
                "high",
                "Large activation-loaded instruction body",
                format!(
                    "Package activation body is {} lines / approximately {} tokens.",
                    activation_lines, activation_estimated_tokens
                ),
                vec![
                    "ruler_effective_context",
                    "tiktoken_token_accounting",
                    "skillsbench_focused_skills",
                ],
                "Move examples, references, and detailed procedures into deferred files or structured SkillSpec entries.",
                18,
            ),
            skill_path.to_owned(),
        ));
    }
    issues
}

fn workspace_surface(packages: &[DoctorPackageRiskReport]) -> SurfaceReport {
    SurfaceReport {
        activation_lines: packages
            .iter()
            .map(|package| package.activation_lines)
            .sum::<usize>(),
        activation_estimated_tokens: packages
            .iter()
            .map(|package| package.activation_estimated_tokens)
            .sum::<usize>(),
        deferred_files: packages.len(),
        ..SurfaceReport::default()
    }
}

fn workspace_issues(
    shape: &DoctorShapeReport,
    packages: &[DoctorPackageRiskReport],
) -> Vec<DoctorIssue> {
    let mut issues = Vec::new();
    if let Some(issue) = name_collision_issue(shape, packages) {
        issues.push(issue);
    }
    if !shape.referenced_skill_paths.is_empty() {
        issues.push(with_location(
            issue(
                "workspace_cross_skill_reference_risk",
                "high",
                "Skill packages reference other skill packages without explicit dependencies",
                format!(
                    "Referenced nested skill package(s): {}.",
                    display_list(&shape.referenced_skill_paths)
                ),
                vec!["skillspec_local_reliability_gap", "skillspec_local_contract_trace"],
                "Preserve package identity and connect packages with explicit SkillSpec dependencies.",
                16,
            ),
            shape.root.clone(),
        ));
    }
    issues
}

fn name_collision_issue(
    shape: &DoctorShapeReport,
    packages: &[DoctorPackageRiskReport],
) -> Option<DoctorIssue> {
    let mut by_slug = BTreeMap::<&str, Vec<&str>>::new();
    for package in packages {
        by_slug
            .entry(package.install_slug.as_str())
            .or_default()
            .push(package.path.as_str());
    }
    let collisions = by_slug
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(slug, paths)| format!("{slug}: {}", paths.join(", ")))
        .collect::<Vec<_>>();
    if collisions.is_empty() {
        return None;
    }
    Some(with_location(
        issue(
            "workspace_name_collision_risk",
            "high",
            "Workspace package names collide after normalization",
            format!("Install-slug collision(s): {}.", collisions.join("; ")),
            vec![
                "claude_skill_frontmatter_discovery",
                "skillspec_local_reliability_gap",
            ],
            "Use namespace-preserving package ids and collision-resistant install slugs.",
            14,
        ),
        shape.root.clone(),
    ))
}

fn workspace_agent_drift_risk(
    shape: &DoctorShapeReport,
    packages: &[DoctorPackageRiskReport],
    issues: &[DoctorIssue],
) -> WorkspaceAgentDriftRiskReport {
    let package_score = packages
        .iter()
        .map(|package| package.agent_drift_risk.score)
        .max()
        .unwrap_or(0);
    let issue_score = issues
        .iter()
        .map(|issue| usize::from(issue.score_penalty))
        .sum::<usize>()
        .min(100) as u8;
    let score = package_score.max(issue_score);
    let conditions = issues
        .iter()
        .map(|issue| RiskCondition {
            id: issue.id.clone(),
            kind: if issue.id.starts_with("workspace_") {
                RiskConditionKind::WorkspaceAggregateRisk
            } else {
                RiskConditionKind::ShapeRisk
            },
            level: RiskLevel::from_severity(&issue.severity),
            score_delta: issue.score_penalty,
            confidence: RiskConfidence::Medium,
            measurement: BTreeMap::new(),
            evidence: vec![RiskEvidence {
                path: issue.location.clone().unwrap_or_else(|| shape.root.clone()),
                line: None,
                text_preview: issue.evidence.clone(),
            }],
            basis_ids: issue.basis.clone(),
            claim_scope: "static_workspace_shape_risk".to_owned(),
            threshold_source: "skillspec_policy_v0".to_owned(),
            consequence: issue.title.clone(),
            recommended_action: issue.remediation.clone(),
        })
        .collect::<Vec<_>>();
    WorkspaceAgentDriftRiskReport {
        score,
        level: RiskLevel::from_score(score),
        summary: format!(
            "{} package(s) analyzed under {}.",
            packages.len(),
            shape.kind
        ),
        conditions,
    }
}

fn plugin_for_skill_path(shape: &DoctorShapeReport, skill_path: &str) -> Option<String> {
    let path = Path::new(skill_path);
    shape
        .plugin_roots
        .iter()
        .find(|plugin| path.starts_with(PathBuf::from(&plugin.path).join("skills")))
        .map(|plugin| plugin.namespace.clone())
}

fn fallback_public_name(skill_path: &str) -> String {
    Path::new(skill_path)
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("skill")
        .to_owned()
}

fn package_id(plugin_name: Option<&str>, skill_path: &str, public_name: &str) -> String {
    plugin_name
        .map(|plugin| format!("{plugin}:{}", path_package_id(skill_path)))
        .unwrap_or_else(|| {
            let package_id = path_package_id(skill_path);
            if package_id.is_empty() {
                public_name.to_owned()
            } else {
                package_id
            }
        })
}

fn path_package_id(skill_path: &str) -> String {
    Path::new(skill_path)
        .parent()
        .map(path_to_slash)
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| "root".to_owned())
}

fn install_slug(plugin_name: Option<&str>, skill_path: &str, public_name: &str) -> String {
    let raw = plugin_name
        .map(|plugin| format!("{plugin}__{}", path_package_id(skill_path)))
        .unwrap_or_else(|| public_name.to_owned());
    slugify(&raw).replace('-', "__")
}

fn shape_role(shape: &DoctorShapeReport, skill_path: &str) -> String {
    if shape.primary_skill.as_deref() == Some(skill_path) {
        return "entry_skill".to_owned();
    }
    if plugin_for_skill_path(shape, skill_path).is_some() {
        return "plugin_skill".to_owned();
    }
    "skill_package".to_owned()
}
