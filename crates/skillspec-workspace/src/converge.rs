use super::{
    dependency_edges, load_manifest, output_package_dir, path_to_string, topological_package_order,
    validate_workspace, write_text, WorkspaceDependencyEdge, WorkspacePackage, WorkspaceReference,
};
use serde::{Deserialize, Serialize};
use skillspec_core::error::{Error, Result};
use skillspec_core::parser;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceConvergeReport {
    pub ok: bool,
    pub manifest_path: String,
    pub build_root: String,
    pub report_path: String,
    pub package_count: usize,
    pub ready: Vec<String>,
    pub failed: Vec<String>,
    pub blocked: Vec<String>,
    pub missing: Vec<String>,
    pub validation_warnings: Vec<String>,
    pub cross_package_references: Vec<WorkspaceReference>,
    pub dependency_edges: Vec<WorkspaceDependencyEdge>,
    pub packages: Vec<WorkspaceConvergePackageReport>,
    pub next: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceConvergePackageReport {
    pub package_id: String,
    pub status: WorkspaceConvergeStatus,
    pub output_dir: String,
    pub spec_path: String,
    pub package_report_path: String,
    pub dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceConvergeStatus {
    Ready,
    Failed,
    Blocked,
    Missing,
}

impl WorkspaceConvergeStatus {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Deserialize)]
struct PackageEvidenceReport {
    status: String,
    message: Option<String>,
}

pub fn converge_workspace(
    manifest_path: &Path,
    build_root: &Path,
) -> Result<WorkspaceConvergeReport> {
    let validation = validate_workspace(manifest_path)?;
    if !validation.ok {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace converge requires a valid manifest; run `skillspec workspace validate {}` first. Errors: {}",
                manifest_path.display(),
                validation.errors.join("; ")
            ),
        });
    }

    let manifest = load_manifest(manifest_path)?;
    let validation_warnings = validation.warnings;
    let cross_package_references = manifest
        .references
        .iter()
        .filter(|reference| reference.target_package.is_some())
        .cloned()
        .collect::<Vec<_>>();
    if !build_root.is_dir() {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace converge build root is not a directory: {}",
                build_root.display()
            ),
        });
    }

    let mut package_reports = Vec::new();
    let mut statuses = BTreeMap::<String, WorkspaceConvergeStatus>::new();
    for package_id in topological_package_order(&manifest) {
        let package = manifest
            .packages
            .get(&package_id)
            .expect("topological order only includes known packages");
        let blocked_by = package
            .depends_on
            .iter()
            .filter(|dependency| statuses.get(*dependency) != Some(&WorkspaceConvergeStatus::Ready))
            .cloned()
            .collect::<Vec<_>>();

        let mut package_report = converge_one_package(package, build_root)?;
        if !blocked_by.is_empty() && package_report.status == WorkspaceConvergeStatus::Ready {
            package_report.status = WorkspaceConvergeStatus::Blocked;
            package_report.message = Some(format!(
                "blocked because dependencies are not ready: {}",
                blocked_by.join(", ")
            ));
        }
        statuses.insert(package.package_id.clone(), package_report.status.clone());
        package_reports.push(package_report);
    }

    let ready = package_ids_by_status(&package_reports, WorkspaceConvergeStatus::Ready);
    let failed = package_ids_by_status(&package_reports, WorkspaceConvergeStatus::Failed);
    let blocked = package_ids_by_status(&package_reports, WorkspaceConvergeStatus::Blocked);
    let missing = package_ids_by_status(&package_reports, WorkspaceConvergeStatus::Missing);
    let report_path = build_root.join("workspace-converge.report.md");

    let report = WorkspaceConvergeReport {
        ok: failed.is_empty() && blocked.is_empty() && missing.is_empty(),
        manifest_path: path_to_string(manifest_path),
        build_root: path_to_string(build_root),
        report_path: path_to_string(&report_path),
        package_count: manifest.packages.len(),
        ready,
        failed,
        blocked,
        missing,
        validation_warnings,
        cross_package_references,
        dependency_edges: dependency_edges(&manifest),
        packages: package_reports,
        next: vec![format!(
            "skillspec workspace compile {} --build-root {} --target <target>",
            manifest_path.display(),
            build_root.display()
        )],
    };
    write_text(&report_path, &render_converge_report(&report))?;
    Ok(report)
}

pub fn render_converge_report(report: &WorkspaceConvergeReport) -> String {
    let mut output = String::new();
    output.push_str("Workspace converge\n\n");
    output.push_str(&format!("- manifest: {}\n", report.manifest_path));
    output.push_str(&format!("- build_root: {}\n", report.build_root));
    output.push_str(&format!("- packages: {}\n", report.package_count));
    output.push_str(&format!(
        "- status: {}\n",
        if report.ok { "ok" } else { "failed" }
    ));
    output.push_str(&format!("- report: {}\n", report.report_path));
    output.push('\n');

    push_id_list(&mut output, "Ready", &report.ready);
    push_id_list(&mut output, "Failed", &report.failed);
    push_id_list(&mut output, "Blocked", &report.blocked);
    push_id_list(&mut output, "Missing", &report.missing);

    output.push_str("\n## Graph Proof\n\n");
    output.push_str("- manifest validation: passed\n");
    output.push_str("- cycles checked: passed\n");
    output.push_str("- dependency references checked: passed\n");
    output.push_str("- install slug collision check: passed\n");
    output.push_str(&format!(
        "- cross-package references: {}\n",
        report.cross_package_references.len()
    ));
    output.push_str(&format!(
        "- ready package drafts: {}/{}\n",
        report.ready.len(),
        report.package_count
    ));
    if !report.validation_warnings.is_empty() {
        output.push_str("- validation warnings:\n");
        for warning in &report.validation_warnings {
            output.push_str(&format!("  - {warning}\n"));
        }
    }

    output.push_str("\n## Packages\n\n");
    for package in &report.packages {
        output.push_str(&format!(
            "- {}: {} -> {}\n",
            package.package_id,
            package.status.as_str(),
            package.spec_path
        ));
        if let Some(message) = &package.message {
            output.push_str(&format!("  message: {message}\n"));
        }
    }

    output.push_str("\n## Dependency Graph\n\n");
    if report.dependency_edges.is_empty() {
        output.push_str("- none\n");
    } else {
        for edge in &report.dependency_edges {
            output.push_str(&format!("- {} -> {}\n", edge.from, edge.to));
        }
    }

    output.push_str("\n## Next\n\n");
    if report.ok {
        for next in &report.next {
            output.push_str(&format!("- {next}\n"));
        }
    } else {
        output.push_str("- fix failed or missing package drafts, then rerun workspace converge\n");
    }
    output
}

fn converge_one_package(
    package: &WorkspacePackage,
    build_root: &Path,
) -> Result<WorkspaceConvergePackageReport> {
    let output_dir = output_package_dir(package, build_root)?;
    let spec_path = output_dir.join("skill.spec.yml");
    let package_report_path = output_dir.join(".skillspec/workspace-import.json");
    let evidence = load_package_evidence(&package_report_path)?;

    let (status, message) = match evidence.as_ref().map(|evidence| evidence.status.as_str()) {
        Some("failed") => (
            WorkspaceConvergeStatus::Failed,
            evidence.and_then(|evidence| evidence.message.clone()),
        ),
        Some("blocked") => (
            WorkspaceConvergeStatus::Blocked,
            evidence.and_then(|evidence| evidence.message.clone()),
        ),
        Some("skipped") => (
            WorkspaceConvergeStatus::Missing,
            evidence
                .and_then(|evidence| evidence.message.clone())
                .or_else(|| Some("package was skipped during workspace import".to_owned())),
        ),
        Some("built") | Some("cached") | None => {
            if !spec_path.is_file() {
                (
                    WorkspaceConvergeStatus::Missing,
                    Some(format!("missing generated spec {}", spec_path.display())),
                )
            } else {
                match parser::load_spec(&spec_path) {
                    Ok(_) => (WorkspaceConvergeStatus::Ready, None),
                    Err(error) => (WorkspaceConvergeStatus::Failed, Some(error.to_string())),
                }
            }
        }
        Some(other) => (
            WorkspaceConvergeStatus::Failed,
            Some(format!("unknown workspace import status {other:?}")),
        ),
    };

    Ok(WorkspaceConvergePackageReport {
        package_id: package.package_id.clone(),
        status,
        output_dir: path_to_string(&output_dir),
        spec_path: path_to_string(&spec_path),
        package_report_path: path_to_string(&package_report_path),
        dependencies: package.depends_on.clone(),
        message,
    })
}

fn load_package_evidence(path: &Path) -> Result<Option<PackageEvidenceReport>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|source| Error::ParseJson {
            path: path.to_path_buf(),
            source,
        })
}

fn package_ids_by_status(
    packages: &[WorkspaceConvergePackageReport],
    status: WorkspaceConvergeStatus,
) -> Vec<String> {
    packages
        .iter()
        .filter_map(|package| (package.status == status).then_some(package.package_id.clone()))
        .collect()
}

fn push_id_list(output: &mut String, title: &str, ids: &[String]) {
    output.push_str(&format!("## {title}\n\n"));
    if ids.is_empty() {
        output.push_str("- none\n");
    } else {
        for id in ids {
            output.push_str(&format!("- {id}\n"));
        }
    }
    output.push('\n');
}
