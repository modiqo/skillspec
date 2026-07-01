use super::types::{AgentDriftRiskReport, DoctorPackageRiskReport};
use super::{
    basis, frontmatter, issue, metrics, path_to_slash, risk, severity_rank, slugify, with_location,
    DoctorIssue, DoctorReport, DoctorShapeReport, Error, Result, LARGE_BODY_LINES,
    LARGE_BODY_TOKENS,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct RiskProfile {
    structural_score: u8,
    activation_estimated_tokens: usize,
    activation_lines: usize,
    frontmatter_discovery_risk: super::FrontmatterDiscoveryRiskReport,
    agent_drift_risk: AgentDriftRiskReport,
    source: String,
    canonical_package_root: Option<PathBuf>,
    canonical_path: String,
}

pub(super) fn package_reports(
    shape: &DoctorShapeReport,
    source_root: Option<&Path>,
    skill_texts: &[(String, String)],
) -> Result<Vec<DoctorPackageRiskReport>> {
    let mut cache = BTreeMap::<String, RiskProfile>::new();
    let mut reports = Vec::new();

    for (skill_path, content) in skill_texts {
        let source_content_sha256 = sha256_hex(content.as_bytes());
        let package_root = source_root.and_then(|root| package_root(root, skill_path));
        let fingerprint = match &package_root {
            Some(root) => package_fingerprint(root, &source_content_sha256)?,
            None => format!("skill-text:{source_content_sha256}"),
        };

        let (profile, canonical_risk_profile_path) = if let Some(cached) = cache.get(&fingerprint) {
            let mut reused = cached.clone();
            reused.source = "reused_identical_package_profile".to_owned();
            (
                reused,
                (cached.canonical_path != *skill_path).then(|| cached.canonical_path.clone()),
            )
        } else {
            let profile = match &package_root {
                Some(root) => profile_from_package_root(root, skill_path, content)?,
                None => fallback_profile_from_text(skill_path, content),
            };
            cache.insert(fingerprint, profile.clone());
            (profile, None)
        };

        reports.push(package_report_from_profile(
            shape,
            skill_path,
            source_content_sha256,
            profile,
            canonical_risk_profile_path,
        ));
    }

    Ok(reports)
}

fn package_report_from_profile(
    shape: &DoctorShapeReport,
    skill_path: &str,
    source_content_sha256: String,
    mut profile: RiskProfile,
    canonical_risk_profile_path: Option<String>,
) -> DoctorPackageRiskReport {
    if let Some(canonical_root) = &profile.canonical_package_root {
        let current_relative_root = package_relative_root(skill_path);
        rewrite_agent_evidence(
            &mut profile.agent_drift_risk,
            canonical_root,
            &current_relative_root,
        );
    }

    let plugin_name = plugin_for_skill_path(shape, skill_path);
    let public_name = profile
        .frontmatter_discovery_risk
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
        source_content_sha256,
        risk_profile_source: profile.source,
        canonical_risk_profile_path,
        shape_role: shape_role(shape, skill_path),
        entrypoint: "SKILL.md".to_owned(),
        structural_score: profile.structural_score,
        activation_estimated_tokens: profile.activation_estimated_tokens,
        activation_lines: profile.activation_lines,
        frontmatter_discovery_risk: profile.frontmatter_discovery_risk,
        agent_drift_risk: profile.agent_drift_risk,
    }
}

fn profile_from_package_root(
    package_root: &Path,
    skill_path: &str,
    content: &str,
) -> Result<RiskProfile> {
    match super::inspect_simple_skill(package_root) {
        Ok(report) => Ok(profile_from_simple_report(
            report,
            "full_package_analysis",
            Some(package_root.to_path_buf()),
            skill_path,
        )),
        Err(_) => Ok(fallback_profile_from_text(skill_path, content)),
    }
}

fn profile_from_simple_report(
    report: DoctorReport,
    source: &str,
    canonical_package_root: Option<PathBuf>,
    skill_path: &str,
) -> RiskProfile {
    let frontmatter_discovery_risk = report
        .frontmatter_discovery_risk
        .expect("simple skill doctor always produces frontmatter risk");
    let agent_drift_risk = report
        .agent_drift_risk
        .expect("simple skill doctor always produces agent drift risk");
    RiskProfile {
        structural_score: report.structural_score,
        activation_estimated_tokens: report.surface.activation_estimated_tokens,
        activation_lines: report.surface.activation_lines,
        frontmatter_discovery_risk,
        agent_drift_risk,
        source: source.to_owned(),
        canonical_package_root,
        canonical_path: skill_path.to_owned(),
    }
}

fn fallback_profile_from_text(skill_path: &str, content: &str) -> RiskProfile {
    let sections = frontmatter::split_skill(content);
    let frontmatter_risk = frontmatter::analyze(Path::new(skill_path), &sections);
    let activation_estimated_tokens = metrics::estimate_tokens(&sections.body);
    let activation_lines = sections.body.lines().count();
    let package_issues =
        fallback_package_issues(skill_path, activation_estimated_tokens, activation_lines);
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
    RiskProfile {
        structural_score,
        activation_estimated_tokens,
        activation_lines,
        frontmatter_discovery_risk: frontmatter_risk,
        agent_drift_risk,
        source: "fallback_skill_text_analysis".to_owned(),
        canonical_package_root: None,
        canonical_path: skill_path.to_owned(),
    }
}

fn fallback_package_issues(
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
    issues.sort_by_key(|issue| severity_rank(&issue.severity));
    issues
}

fn package_fingerprint(package_root: &Path, source_content_sha256: &str) -> Result<String> {
    let mut files = super::collect_inventory_files(package_root)?;
    files.sort();
    let mut hasher = Sha256::new();
    hasher.update(source_content_sha256.as_bytes());
    for relative in files {
        if relative
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        {
            continue;
        }
        let path = package_root.join(&relative);
        let bytes = fs::read(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        hasher.update(path_to_slash(&relative).as_bytes());
        hasher.update(b"\0");
        hasher.update(sha256_hex(&bytes).as_bytes());
        hasher.update(b"\0");
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn rewrite_agent_evidence(
    report: &mut AgentDriftRiskReport,
    canonical_root: &Path,
    current_relative_root: &str,
) {
    for condition in &mut report.conditions {
        for evidence in &mut condition.evidence {
            let path = Path::new(&evidence.path);
            if let Ok(relative) = path.strip_prefix(canonical_root) {
                evidence.path = if relative.as_os_str().is_empty() {
                    current_relative_root.to_owned()
                } else if current_relative_root.is_empty() {
                    path_to_slash(relative)
                } else {
                    path_to_slash(&PathBuf::from(current_relative_root).join(relative))
                };
            }
        }
    }
}

fn package_root(root: &Path, skill_path: &str) -> Option<PathBuf> {
    Path::new(skill_path)
        .parent()
        .map(|parent| root.join(parent))
        .or_else(|| Some(root.to_path_buf()))
}

fn package_relative_root(skill_path: &str) -> String {
    Path::new(skill_path)
        .parent()
        .map(path_to_slash)
        .filter(|path| !path.is_empty())
        .unwrap_or_default()
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
        .unwrap_or_else(|| fallback_public_name(skill_path))
}

fn install_slug(plugin_name: Option<&str>, skill_path: &str, public_name: &str) -> String {
    plugin_name
        .map(|plugin| {
            format!(
                "{}--{}",
                slugify(plugin),
                slugify(&path_package_id(skill_path))
            )
        })
        .unwrap_or_else(|| slugify(public_name))
}

fn shape_role(shape: &DoctorShapeReport, skill_path: &str) -> String {
    if shape.primary_skill.as_deref() == Some(skill_path) {
        return "entry".to_owned();
    }
    if plugin_for_skill_path(shape, skill_path).is_some() {
        return "plugin_skill".to_owned();
    }
    "workspace_skill".to_owned()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
