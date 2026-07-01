use super::{DoctorReport, RiskLevel};

pub(super) fn scope_label(report: &DoctorReport) -> &'static str {
    match report.shape.kind.as_str() {
        "simple_skill" => "Single skill",
        "multi_skill_workspace" => "Multi-skill workspace",
        "plugin_workspace" => "Plugin workspace",
        "entry_skill_with_subskills" => "Entry + subskills",
        "non_skill_repository" => "Shape only",
        _ => "Unknown shape",
    }
}

pub(super) fn assessment_scope(report: &DoctorReport) -> &'static str {
    match report.shape.kind.as_str() {
        "simple_skill" => {
            "one atomic SKILL.md package; the score describes this package directly"
        }
        "multi_skill_workspace" => {
            "workspace container plus full per-package raw skill profiles; package paths remain part of identity"
        }
        "plugin_workspace" => {
            "plugin workspace plus full per-package raw skill profiles; plugin namespace and skill path remain part of identity"
        }
        "entry_skill_with_subskills" => {
            "entry SKILL.md plus nested skill packages; root entry and subskills are reported as separate package identities"
        }
        "non_skill_repository" => {
            "repository shape only; no package trust or runtime behavior score was computed"
        }
        _ => "unknown shape; inspect the JSON shape contract before import or install",
    }
}

pub(super) fn risk_interpretation(report: &DoctorReport) -> String {
    match report.shape.kind.as_str() {
        "simple_skill" => {
            "A high score means this raw package should be ported before install or runtime reliance."
                .to_owned()
        }
        "multi_skill_workspace" | "entry_skill_with_subskills" => {
            "Workspace risk is the maximum of workspace-shape risk and per-package raw skill risk. A healthy workspace map does not certify each package; inspect package drift rows and port in risk order.".to_owned()
        }
        "plugin_workspace" => {
            "Plugin workspace risk is the maximum of plugin-shape risk and per-package raw skill risk. Preserve plugin namespace/path identity and inspect package drift rows before import, compile, or install.".to_owned()
        }
        "non_skill_repository" => {
            "Doctor did not find an importable skill package, so it reports only source shape and next selection guidance.".to_owned()
        }
        _ => "Shape is not recognized; do not infer install readiness from this report.".to_owned(),
    }
}

pub(super) fn package_risk_rollup(report: &DoctorReport) -> Option<String> {
    let mut by_score = report.packages.iter().collect::<Vec<_>>();
    if by_score.is_empty() {
        return None;
    }
    by_score.sort_by(|left, right| {
        right
            .agent_drift_risk
            .score
            .cmp(&left.agent_drift_risk.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    let top = by_score[0];
    let critical = count_packages_at_level(report, RiskLevel::Critical);
    let high = count_packages_at_level(report, RiskLevel::High);
    let medium = count_packages_at_level(report, RiskLevel::Medium);
    let low = count_packages_at_level(report, RiskLevel::Low);
    Some(format!(
        "max {} ({}/100) at {}; critical={}, high={}, medium={}, low={}",
        top.agent_drift_risk.level.as_str(),
        top.agent_drift_risk.score,
        top.path,
        critical,
        high,
        medium,
        low
    ))
}

fn count_packages_at_level(report: &DoctorReport, level: RiskLevel) -> usize {
    report
        .packages
        .iter()
        .filter(|package| package.agent_drift_risk.level == level)
        .count()
}
