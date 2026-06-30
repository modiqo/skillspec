use super::types::{
    DoctorPackageRiskReport, RiskCondition, RiskConditionKind, RiskConfidence, RiskEvidence,
    RiskLevel, WorkspaceAgentDriftRiskReport,
};
use super::{
    basis, display_list, frontmatter, issue, metrics, path_to_slash, shape_root, slugify,
    with_location, DoctorIssue, DoctorReport, DoctorShapeReport, Error, Result,
    ShapeClassification, SurfaceReport, WorkspaceFrontmatterNameRefReport, WorkspaceIdentityReport,
    WorkspaceNamespaceIdentityReport, WorkspaceSourceContentRefReport, LARGE_BODY_LINES,
    LARGE_BODY_TOKENS,
};
use super::{risk, severity_rank};
use crate::remote_source;
use sha2::{Digest, Sha256};
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
    let package_reports = skill_texts
        .iter()
        .map(|(skill_path, content)| package_report(&shape, skill_path, content))
        .collect::<Vec<_>>();
    let workspace_identity = workspace_identity(&shape, &counts, &package_reports, &skill_texts);

    let mut issues = super::shape_issues(&shape, &counts);
    issues.extend(workspace_issues(
        &shape,
        &package_reports,
        &workspace_identity,
    ));
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
        workspace_identity: Some(workspace_identity),
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
    identity: &WorkspaceIdentityReport,
) -> Vec<DoctorIssue> {
    let mut issues = Vec::new();
    if let Some(issue) = name_collision_issue(shape, packages) {
        issues.push(issue);
    }
    if identity.repeated_skill_content_groups > 0 {
        issues.push(with_location(
            issue(
                "workspace_repeated_skill_content",
                "medium",
                "Repeated skill content should be referentiable",
                format!(
                    "{} namespaced skill package file(s) contain {} unique byte-identical SKILL.md content item(s); {} repeated occurrence(s) across {} group(s) account for approximately {} repeated source token(s).",
                    identity.skill_file_count,
                    identity.unique_skill_content_count,
                    identity.repeated_skill_content_occurrences,
                    identity.repeated_skill_content_groups,
                    identity.repeated_skill_content_estimated_tokens
                ),
                vec!["skillspec_local_reliability_gap", "skillspec_local_contract_trace"],
                "Preserve every namespace/path package identity, but store one canonical source-content artifact per SHA and make each repeated package refer to that content instead of copying bytes as independent source.",
                8,
            ),
            shape.root.clone(),
        ));
    }
    if identity.same_frontmatter_name_groups > 0 {
        issues.push(with_location(
            issue(
                "workspace_reused_frontmatter_names",
                "medium",
                "Frontmatter names repeat across distinct package identities",
                format!(
                    "{} frontmatter name group(s) repeat across {} extra package occurrence(s). This is valid when namespace/path is the identity, but it is load-bearing for agents unless displayed explicitly.",
                    identity.same_frontmatter_name_groups,
                    identity.same_frontmatter_name_occurrences
                ),
                vec!["claude_skill_frontmatter_discovery", "skillspec_local_reliability_gap"],
                "Use namespace/path identity in workspace maps and show repeated names as referentiable aliases rather than duplicate package errors.",
                6,
            ),
            shape.root.clone(),
        ));
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

fn workspace_identity(
    shape: &DoctorShapeReport,
    counts: &super::DoctorCounts,
    packages: &[DoctorPackageRiskReport],
    skill_texts: &[(String, String)],
) -> WorkspaceIdentityReport {
    let namespaces = namespace_identity(packages);
    let source_content_refs = source_content_refs(skill_texts);
    let frontmatter_name_refs = frontmatter_name_refs(packages);
    let unique_skill_content_estimated_tokens = source_content_refs
        .iter()
        .map(|content| content.estimated_tokens)
        .sum::<usize>();
    let total_skill_content_estimated_tokens = skill_texts
        .iter()
        .map(|(_, content)| metrics::estimate_tokens(content))
        .sum::<usize>();
    let repeated_skill_content_estimated_tokens = source_content_refs
        .iter()
        .map(|content| content.repeated_estimated_tokens)
        .sum::<usize>();
    let repeated_skill_content_groups = source_content_refs
        .iter()
        .filter(|content| content.occurrence_count > 1)
        .count();
    let repeated_skill_content_occurrences = source_content_refs
        .iter()
        .map(|content| content.repeated_occurrence_count)
        .sum::<usize>();
    let same_frontmatter_name_groups = frontmatter_name_refs.len();
    let same_frontmatter_name_occurrences = frontmatter_name_refs
        .iter()
        .map(|item| item.occurrence_count.saturating_sub(1))
        .sum::<usize>();
    WorkspaceIdentityReport {
        source_file_count: counts.total_files,
        skill_file_count: shape.skill_files.len(),
        namespaced_package_count: packages.len(),
        namespace_count: namespaces.len(),
        namespaces,
        unique_skill_content_count: source_content_refs.len(),
        repeated_skill_content_groups,
        repeated_skill_content_occurrences,
        total_skill_content_estimated_tokens,
        unique_skill_content_estimated_tokens,
        repeated_skill_content_estimated_tokens,
        source_content_refs,
        same_frontmatter_name_groups,
        same_frontmatter_name_occurrences,
        frontmatter_name_refs,
        recommendation: "Keep namespace/path as package identity and make repeated byte-identical SKILL.md content referentiable through source_content_ref aliases instead of copying the same bytes as independent source material.".to_owned(),
    }
}

fn namespace_identity(
    packages: &[DoctorPackageRiskReport],
) -> Vec<WorkspaceNamespaceIdentityReport> {
    let mut by_namespace = BTreeMap::<String, Vec<String>>::new();
    for package in packages {
        by_namespace
            .entry(
                package
                    .plugin_name
                    .clone()
                    .unwrap_or_else(|| "workspace".to_owned()),
            )
            .or_default()
            .push(package.path.clone());
    }
    by_namespace
        .into_iter()
        .map(|(namespace, mut paths)| {
            paths.sort();
            let skill_file_count = paths.len();
            WorkspaceNamespaceIdentityReport {
                namespace,
                skill_file_count,
                sample_paths: paths.into_iter().take(8).collect(),
            }
        })
        .collect()
}

fn source_content_refs(skill_texts: &[(String, String)]) -> Vec<WorkspaceSourceContentRefReport> {
    let mut by_sha = BTreeMap::<String, Vec<(String, usize, usize)>>::new();
    for (path, content) in skill_texts {
        let sha = format!("{:x}", Sha256::digest(content.as_bytes()));
        by_sha.entry(sha).or_default().push((
            path.clone(),
            content.len(),
            metrics::estimate_tokens(content),
        ));
    }
    by_sha
        .into_iter()
        .map(|(sha256, mut items)| {
            items.sort_by(|left, right| left.0.cmp(&right.0));
            let occurrence_count = items.len();
            let canonical_path = items[0].0.clone();
            let content_bytes = items[0].1;
            let estimated_tokens = items[0].2;
            let aliases = items
                .iter()
                .skip(1)
                .map(|(path, _, _)| path.clone())
                .collect::<Vec<_>>();
            WorkspaceSourceContentRefReport {
                sha256,
                canonical_path,
                aliases,
                occurrence_count,
                content_bytes,
                estimated_tokens,
                repeated_occurrence_count: occurrence_count.saturating_sub(1),
                repeated_estimated_tokens: estimated_tokens
                    .saturating_mul(occurrence_count.saturating_sub(1)),
            }
        })
        .collect()
}

fn frontmatter_name_refs(
    packages: &[DoctorPackageRiskReport],
) -> Vec<WorkspaceFrontmatterNameRefReport> {
    let mut by_name = BTreeMap::<String, Vec<String>>::new();
    for package in packages {
        by_name
            .entry(package.public_name.clone())
            .or_default()
            .push(package.path.clone());
    }
    by_name
        .into_iter()
        .filter_map(|(public_name, mut paths)| {
            if paths.len() < 2 {
                return None;
            }
            paths.sort();
            Some(WorkspaceFrontmatterNameRefReport {
                public_name,
                occurrence_count: paths.len(),
                paths,
            })
        })
        .collect()
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
