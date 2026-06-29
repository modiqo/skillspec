use crate::durable_lifecycle::{self, DurableStatusReport};
use crate::install;
use crate::router::{self, Visibility};
use crate::router_lifecycle::{self, RouterStatusReport};
use serde::Serialize;
use skillspec_core::error::Result;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct StatusOptions {
    pub roots: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusReport {
    pub router: RouterStatusReport,
    pub durable_executor: DurableStatusReport,
    pub roots: RootsStatusReport,
    pub skills: SkillInventoryReport,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RootsStatusReport {
    pub supported_count: usize,
    pub detected_count: usize,
    pub scanned_count: usize,
    pub scan_source: ScanRootSource,
    pub supported: Vec<SupportedRootReport>,
    pub scanned: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SupportedRootReport {
    pub id: &'static str,
    pub label: &'static str,
    pub path: PathBuf,
    pub detected: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanRootSource {
    Provided,
    RouterConfig,
    Detected,
    None,
}

#[derive(Clone, Debug, Serialize)]
pub struct SkillInventoryReport {
    pub total: usize,
    pub skillspec_backed_count: usize,
    pub legacy_count: usize,
    pub disabled_count: usize,
    pub skillspec_backed: Vec<SkillStatusEntry>,
    pub legacy: Vec<SkillStatusEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SkillStatusEntry {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub skill_dir: PathBuf,
    pub visibility: Visibility,
    pub has_skill_spec: bool,
    pub description_chars: usize,
    pub short_description: Option<String>,
    pub source: String,
}

pub fn status(options: StatusOptions) -> Result<StatusReport> {
    let router = router_lifecycle::status()?;
    let durable_executor = durable_lifecycle::status()?;
    let supported_roots = install::detect_targets()?
        .into_iter()
        .map(|root| SupportedRootReport {
            id: root.id,
            label: root.label,
            path: root.path,
            detected: root.detected,
        })
        .collect::<Vec<_>>();
    let (scan_source, scanned_roots) = select_scan_roots(&options.roots, &router, &supported_roots);
    let mut warnings = Vec::new();
    let entries = if scanned_roots.is_empty() {
        Vec::new()
    } else {
        router::scan_roots(&scanned_roots, &mut warnings)?
    };

    let mut skillspec_backed = Vec::new();
    let mut legacy = Vec::new();
    let mut disabled_count = 0;
    for entry in entries {
        if entry.visibility == Visibility::Off {
            disabled_count += 1;
        }
        let status = SkillStatusEntry {
            id: entry.id,
            name: entry.name,
            path: entry.path,
            skill_dir: entry.skill_dir,
            visibility: entry.visibility,
            has_skill_spec: entry.has_skill_spec,
            description_chars: entry.description.chars().count(),
            short_description: entry.short_description,
            source: entry.source,
        };
        if status.has_skill_spec {
            skillspec_backed.push(status);
        } else {
            legacy.push(status);
        }
    }

    warnings.extend(router.warnings.iter().cloned());
    warnings.extend(durable_executor.warnings.iter().cloned());

    Ok(StatusReport {
        roots: RootsStatusReport {
            supported_count: supported_roots.len(),
            detected_count: supported_roots.iter().filter(|root| root.detected).count(),
            scanned_count: scanned_roots.len(),
            scan_source,
            supported: supported_roots,
            scanned: scanned_roots,
        },
        skills: SkillInventoryReport {
            total: skillspec_backed.len() + legacy.len(),
            skillspec_backed_count: skillspec_backed.len(),
            legacy_count: legacy.len(),
            disabled_count,
            skillspec_backed,
            legacy,
        },
        router,
        durable_executor,
        warnings: unique_strings(warnings),
    })
}

pub fn render(report: &StatusReport) -> String {
    let mut output = String::new();
    output.push_str("SkillSpec status\n\n");
    output.push_str(&format!(
        "Supported roots: {} (detected {})\n",
        report.roots.supported_count, report.roots.detected_count
    ));
    output.push_str(&format!(
        "Scanned roots: {} ({:?})\n",
        report.roots.scanned_count, report.roots.scan_source
    ));
    for root in &report.roots.supported {
        output.push_str(&format!(
            "- {}: {} detected={} ({})\n",
            root.id,
            root.path.display(),
            root.detected,
            root.label
        ));
    }

    output.push('\n');
    output.push_str(&format!(
        "Router: {}\n",
        lifecycle_label(report.router.installed, report.router.enabled)
    ));
    output.push_str(&format!(
        "Router config: {}\n",
        report.router.config.display()
    ));
    if let Some(index) = &report.router.index {
        output.push_str(&format!("Router index: {}\n", index.display()));
    }
    if let Some(index_status) = &report.router.index_status {
        output.push_str(&format!(
            "Router index status: exists={}, stale={}, indexed={}, discovered={}\n",
            index_status.exists,
            index_status.stale,
            index_status.indexed_skills,
            index_status.discovered_skills
        ));
        if let Some(updated_at) = index_status.updated_at_unix {
            output.push_str(&format!("Router index updated_at_unix: {updated_at}\n"));
        }
    }

    output.push('\n');
    output.push_str(&format!(
        "Durable executor: {}\n",
        lifecycle_label(
            report.durable_executor.installed,
            report.durable_executor.enabled
        )
    ));
    output.push_str(&format!(
        "Durable config: {}\n",
        report.durable_executor.config.display()
    ));
    if let Some(source) = &report.durable_executor.source {
        output.push_str(&format!("Durable source: {}\n", source.display()));
    }

    output.push('\n');
    output.push_str(&format!("Skills total: {}\n", report.skills.total));
    output.push_str(&format!(
        "SkillSpec-backed: {}\n",
        report.skills.skillspec_backed_count
    ));
    output.push_str(&format!("Legacy prose: {}\n", report.skills.legacy_count));
    output.push_str(&format!("Disabled/off: {}\n", report.skills.disabled_count));
    render_skill_entries(
        &mut output,
        "SkillSpec-backed skills",
        &report.skills.skillspec_backed,
    );
    render_skill_entries(&mut output, "Legacy prose skills", &report.skills.legacy);

    if !report.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output
}

fn select_scan_roots(
    provided_roots: &[PathBuf],
    router: &RouterStatusReport,
    supported_roots: &[SupportedRootReport],
) -> (ScanRootSource, Vec<PathBuf>) {
    if !provided_roots.is_empty() {
        return (
            ScanRootSource::Provided,
            unique_paths(provided_roots.to_vec()),
        );
    }
    if router.installed && !router.roots.is_empty() {
        return (
            ScanRootSource::RouterConfig,
            unique_paths(router.roots.clone()),
        );
    }
    let detected = supported_roots
        .iter()
        .filter(|root| root.detected)
        .map(|root| root.path.clone())
        .collect::<Vec<_>>();
    if detected.is_empty() {
        (ScanRootSource::None, Vec::new())
    } else {
        (ScanRootSource::Detected, unique_paths(detected))
    }
}

fn render_skill_entries(output: &mut String, label: &str, entries: &[SkillStatusEntry]) {
    if entries.is_empty() {
        return;
    }
    output.push_str(&format!("\n{label}:\n"));
    for entry in entries {
        output.push_str(&format!(
            "- {} [{}]: {}\n",
            entry.name,
            entry.visibility.as_str(),
            entry.path.display()
        ));
    }
}

fn lifecycle_label(installed: bool, enabled: bool) -> &'static str {
    match (installed, enabled) {
        (true, true) => "installed/enabled",
        (true, false) => "installed/disabled",
        (false, _) => "not installed",
    }
}

fn unique_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for path in paths {
        if seen.insert(path.clone()) {
            unique.push(path);
        }
    }
    unique
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            unique.push(value);
        }
    }
    unique
}
