use super::{
    dependency_edges, load_manifest, manifest_relative_path, output_package_dir, path_to_string,
    topological_package_order, validate_workspace, write_text, WorkspaceDependencyEdge,
    WorkspaceManifest, WorkspacePackage,
};
use crate::error::{Error, Result};
use crate::{doctor, importer, parser, source_map};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceImportReport {
    pub ok: bool,
    pub manifest_path: String,
    pub build_root: String,
    pub manifest_copy_path: String,
    pub report_path: String,
    pub package_count: usize,
    pub built: Vec<String>,
    pub failed: Vec<String>,
    pub skipped: Vec<String>,
    pub blocked: Vec<String>,
    pub dependency_edges: Vec<WorkspaceDependencyEdge>,
    pub packages: Vec<WorkspaceImportPackageReport>,
    pub next: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceImportPackageReport {
    pub package_id: String,
    pub status: WorkspaceImportStatus,
    pub source_path: String,
    pub output_dir: String,
    pub spec_path: String,
    pub source_map_path: String,
    pub doctor_report_path: String,
    pub package_report_path: String,
    pub dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceImportStatus {
    Built,
    Failed,
    Skipped,
    Blocked,
}

impl WorkspaceImportStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Built => "built",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct PackageEvidenceReport {
    package_id: String,
    status: WorkspaceImportStatus,
    source_path: String,
    output_dir: String,
    spec_path: String,
    source_map_path: String,
    doctor_report_path: String,
    dependencies: Vec<String>,
    message: Option<String>,
}

pub fn import_workspace(manifest_path: &Path, build_root: &Path) -> Result<WorkspaceImportReport> {
    let validation = validate_workspace(manifest_path)?;
    if !validation.ok {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace import requires a valid manifest; run `skillspec workspace validate {}` first. Errors: {}",
                manifest_path.display(),
                validation.errors.join("; ")
            ),
        });
    }

    let manifest = load_manifest(manifest_path)?;
    fs::create_dir_all(build_root).map_err(|source| Error::Write {
        path: build_root.to_path_buf(),
        source,
    })?;
    let manifest_copy_path = build_root.join("skillspec.workspace.yml");
    write_yaml(&manifest_copy_path, &manifest)?;

    let mut package_reports = Vec::new();
    let mut statuses = BTreeMap::<String, WorkspaceImportStatus>::new();
    for package_id in topological_package_order(&manifest) {
        let package = manifest
            .packages
            .get(&package_id)
            .expect("topological order only includes known packages");
        let blocked_by = package
            .depends_on
            .iter()
            .filter(|dependency| statuses.get(*dependency) != Some(&WorkspaceImportStatus::Built))
            .cloned()
            .collect::<Vec<_>>();
        let report = if blocked_by.is_empty() {
            import_one_package(&manifest, package, build_root)
        } else {
            Ok(blocked_package_report(
                &manifest,
                package,
                build_root,
                &blocked_by,
            ))
        };

        let package_report = match report {
            Ok(report) => report,
            Err(error) => failed_package_report(&manifest, package, build_root, error),
        };
        write_package_report(&package_report)?;
        statuses.insert(package.package_id.clone(), package_report.status.clone());
        package_reports.push(package_report);
    }

    let built = package_ids_by_status(&package_reports, WorkspaceImportStatus::Built);
    let failed = package_ids_by_status(&package_reports, WorkspaceImportStatus::Failed);
    let skipped = package_ids_by_status(&package_reports, WorkspaceImportStatus::Skipped);
    let blocked = package_ids_by_status(&package_reports, WorkspaceImportStatus::Blocked);
    let report_path = build_root.join("workspace-import.report.md");

    let report = WorkspaceImportReport {
        ok: failed.is_empty() && blocked.is_empty(),
        manifest_path: path_to_string(manifest_path),
        build_root: path_to_string(build_root),
        manifest_copy_path: path_to_string(&manifest_copy_path),
        report_path: path_to_string(&report_path),
        package_count: manifest.packages.len(),
        built,
        failed,
        skipped,
        blocked,
        dependency_edges: dependency_edges(&manifest),
        packages: package_reports,
        next: vec![format!(
            "skillspec workspace converge {} --build-root {}",
            manifest_path.display(),
            build_root.display()
        )],
    };
    write_text(&report_path, &render_import_report(&report))?;
    Ok(report)
}

pub fn render_import_report(report: &WorkspaceImportReport) -> String {
    let mut output = String::new();
    output.push_str("Workspace import\n\n");
    output.push_str(&format!("- manifest: {}\n", report.manifest_path));
    output.push_str(&format!("- build_root: {}\n", report.build_root));
    output.push_str(&format!("- packages: {}\n", report.package_count));
    output.push_str(&format!(
        "- status: {}\n",
        if report.ok { "ok" } else { "failed" }
    ));
    output.push_str(&format!("- report: {}\n", report.report_path));
    output.push('\n');

    push_id_list(&mut output, "Built", &report.built);
    push_id_list(&mut output, "Failed", &report.failed);
    push_id_list(&mut output, "Skipped", &report.skipped);
    push_id_list(&mut output, "Blocked", &report.blocked);

    output.push_str("\n## Packages\n\n");
    for package in &report.packages {
        output.push_str(&format!(
            "- {}: {} -> {}\n",
            package.package_id,
            package.status.as_str(),
            package.output_dir
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
    for next in &report.next {
        output.push_str(&format!("- {next}\n"));
    }
    output
}

fn import_one_package(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
    build_root: &Path,
) -> Result<WorkspaceImportPackageReport> {
    let source_package = source_package_path(manifest, package)?;
    let output_dir = output_package_dir(package, build_root)?;
    fs::create_dir_all(&output_dir).map_err(|source| Error::Write {
        path: output_dir.clone(),
        source,
    })?;

    let doctor_report_path = output_dir.join(".skillspec/reports/doctor.json");
    let doctor_report = doctor::inspect_target(&source_package.display().to_string())?;
    write_json(&doctor_report_path, &doctor_report)?;

    let source_map_dir = output_dir.join(".skillspec/source-map");
    let source_map_report = source_map::create_source_map(&source_package, &source_map_dir)?;
    let source_map_path = PathBuf::from(&source_map_report.source_map);

    let spec_path = output_dir.join("skill.spec.yml");
    let imported = importer::import_skill_for_output(&source_package, &spec_path)?;
    parser::validate_spec(&imported)?;
    parser::write_spec(&spec_path, &imported)?;

    Ok(package_report(
        manifest,
        package,
        build_root,
        WorkspaceImportStatus::Built,
        None,
        Some(source_map_path),
    ))
}

fn blocked_package_report(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
    build_root: &Path,
    blocked_by: &[String],
) -> WorkspaceImportPackageReport {
    package_report(
        manifest,
        package,
        build_root,
        WorkspaceImportStatus::Blocked,
        Some(format!(
            "blocked because dependencies did not build: {}",
            blocked_by.join(", ")
        )),
        None,
    )
}

fn failed_package_report(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
    build_root: &Path,
    error: Error,
) -> WorkspaceImportPackageReport {
    package_report(
        manifest,
        package,
        build_root,
        WorkspaceImportStatus::Failed,
        Some(error.to_string()),
        None,
    )
}

fn package_report(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
    build_root: &Path,
    status: WorkspaceImportStatus,
    message: Option<String>,
    source_map_path: Option<PathBuf>,
) -> WorkspaceImportPackageReport {
    let source_path = source_package_path(manifest, package).unwrap_or_else(|_| PathBuf::new());
    let output_dir =
        output_package_dir(package, build_root).unwrap_or_else(|_| build_root.to_path_buf());
    let spec_path = output_dir.join("skill.spec.yml");
    let doctor_report_path = output_dir.join(".skillspec/reports/doctor.json");
    let source_map_path =
        source_map_path.unwrap_or_else(|| output_dir.join(".skillspec/source-map/source-map.json"));
    let package_report_path = output_dir.join(".skillspec/workspace-import.json");

    WorkspaceImportPackageReport {
        package_id: package.package_id.clone(),
        status,
        source_path: path_to_string(&source_path),
        output_dir: path_to_string(&output_dir),
        spec_path: path_to_string(&spec_path),
        source_map_path: path_to_string(&source_map_path),
        doctor_report_path: path_to_string(&doctor_report_path),
        package_report_path: path_to_string(&package_report_path),
        dependencies: package.depends_on.clone(),
        message,
    }
}

fn write_package_report(report: &WorkspaceImportPackageReport) -> Result<()> {
    let path = PathBuf::from(&report.package_report_path);
    let evidence = PackageEvidenceReport {
        package_id: report.package_id.clone(),
        status: report.status.clone(),
        source_path: report.source_path.clone(),
        output_dir: report.output_dir.clone(),
        spec_path: report.spec_path.clone(),
        source_map_path: report.source_map_path.clone(),
        doctor_report_path: report.doctor_report_path.clone(),
        dependencies: report.dependencies.clone(),
        message: report.message.clone(),
    };
    write_json(&path, &evidence)
}

fn package_ids_by_status(
    packages: &[WorkspaceImportPackageReport],
    status: WorkspaceImportStatus,
) -> Vec<String> {
    packages
        .iter()
        .filter_map(|package| (package.status == status).then_some(package.package_id.clone()))
        .collect()
}

fn source_package_path(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
) -> Result<PathBuf> {
    let relative = manifest_relative_path(&package.path).ok_or_else(|| Error::InvalidInput {
        message: format!(
            "package {} path must be a relative workspace path without parent components: {}",
            package.package_id, package.path
        ),
    })?;
    Ok(PathBuf::from(&manifest.source_root).join(relative))
}

fn write_yaml(path: &Path, value: &impl Serialize) -> Result<()> {
    let content = serde_yaml::to_string(value).map_err(|source| Error::RenderYaml {
        path: path.to_path_buf(),
        source,
    })?;
    write_text(path, &content)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let content = serde_json::to_string_pretty(value)?;
    write_text(path, &format!("{content}\n"))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn package(id: &str, deps: &[&str]) -> WorkspacePackage {
        WorkspacePackage {
            package_id: id.to_owned(),
            path: id.to_owned(),
            kind: super::super::WorkspacePackageKind::Helper,
            entrypoint: "SKILL.md".to_owned(),
            public_name: id.to_owned(),
            install_slug: format!("skills--{id}"),
            depends_on: deps.iter().map(|dep| (*dep).to_owned()).collect(),
        }
    }

    #[test]
    fn topological_order_places_dependencies_first() {
        let manifest = WorkspaceManifest {
            schema: super::super::WORKSPACE_SCHEMA.to_owned(),
            source_root: "/tmp/skills".to_owned(),
            workspace_slug: "skills".to_owned(),
            output_root: "/tmp/skills/.skillspec/workspace-build".to_owned(),
            packages: BTreeMap::from([
                ("app".to_owned(), package("app", &["shared"])),
                ("shared".to_owned(), package("shared", &[])),
                ("wrapper".to_owned(), package("wrapper", &["app"])),
            ]),
            references: Vec::new(),
        };

        assert_eq!(
            super::super::topological_package_order(&manifest),
            vec!["shared", "app", "wrapper"]
        );
    }
}
