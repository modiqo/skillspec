use super::{
    dependency_edges, load_manifest, manifest_relative_path, output_package_dir, path_to_string,
    validate_workspace, write_text, WorkspaceDependencyEdge, WorkspaceManifest, WorkspacePackage,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skillspec_authoring::importer;
use skillspec_core::error::{Error, Result};
use skillspec_core::parser;
use skillspec_doctor::{self as doctor, source_map};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

const WORKSPACE_CACHE_SCHEMA: &str = "skillspec/workspace-cache/v2";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const IMPORT_COMMAND_OPTIONS: &str = "workspace-import/default";

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceImportReport {
    pub ok: bool,
    pub manifest_path: String,
    pub build_root: String,
    pub manifest_copy_path: String,
    pub report_path: String,
    pub package_count: usize,
    pub built: Vec<String>,
    pub cached: Vec<String>,
    pub failed: Vec<String>,
    pub skipped: Vec<String>,
    pub blocked: Vec<String>,
    pub cache_hits: u64,
    pub cache_misses: u64,
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
    Cached,
    Failed,
    Skipped,
    Blocked,
}

impl WorkspaceImportStatus {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Built => "built",
            Self::Cached => "cached",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Blocked => "blocked",
        }
    }

    fn is_ready(&self) -> bool {
        matches!(self, Self::Built | Self::Cached)
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct WorkspaceCacheFile {
    schema: String,
    cli_version: String,
    manifest_hash: String,
    command_options: String,
    packages: BTreeMap<String, WorkspaceCachePackage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorkspaceCachePackage {
    package_id: String,
    source_hash: String,
    spec_hash: String,
    artifacts: Vec<String>,
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
    let manifest_hash = file_hash(manifest_path)?;
    fs::create_dir_all(build_root).map_err(|source| Error::Write {
        path: build_root.to_path_buf(),
        source,
    })?;
    let manifest_copy_path = build_root.join("skillspec.workspace.yml");
    write_yaml(&manifest_copy_path, &manifest)?;
    let mut cache = load_workspace_cache(build_root, &manifest_hash);

    let mut package_reports = Vec::new();
    let mut statuses = BTreeMap::<String, WorkspaceImportStatus>::new();
    let mut remaining = manifest.packages.keys().cloned().collect::<BTreeSet<_>>();

    while !remaining.is_empty() {
        let blocked = remaining
            .iter()
            .filter_map(|package_id| {
                let package = manifest.packages.get(package_id)?;
                let blocked_by = package
                    .depends_on
                    .iter()
                    .filter(|dependency| {
                        statuses
                            .get(*dependency)
                            .is_some_and(|status| !status.is_ready())
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                (!blocked_by.is_empty()).then(|| (package_id.clone(), blocked_by))
            })
            .collect::<Vec<_>>();

        for (package_id, blocked_by) in blocked {
            let package = manifest
                .packages
                .get(&package_id)
                .expect("remaining package id is known");
            let package_report =
                blocked_package_report(&manifest, package, build_root, &blocked_by);
            write_package_report(&package_report)?;
            statuses.insert(package.package_id.clone(), package_report.status.clone());
            remaining.remove(&package_id);
            package_reports.push(package_report);
        }

        let ready = remaining
            .iter()
            .filter(|package_id| {
                let package = manifest
                    .packages
                    .get(*package_id)
                    .expect("remaining package id is known");
                package.depends_on.iter().all(|dependency| {
                    statuses
                        .get(dependency)
                        .is_some_and(|status| status.is_ready())
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        if ready.is_empty() {
            let stuck = remaining.iter().cloned().collect::<Vec<_>>();
            for package_id in stuck {
                let package = manifest
                    .packages
                    .get(&package_id)
                    .expect("remaining package id is known");
                let package_report = failed_package_report(
                    &manifest,
                    package,
                    build_root,
                    Error::InvalidInput {
                        message: "workspace import could not make dependency progress".to_owned(),
                    },
                );
                write_package_report(&package_report)?;
                statuses.insert(package.package_id.clone(), package_report.status.clone());
                remaining.remove(&package_id);
                package_reports.push(package_report);
            }
            continue;
        }

        let mut batch_reports = import_ready_batch(&manifest, build_root, &cache, &ready);
        batch_reports.sort_by(|left, right| left.package_id.cmp(&right.package_id));
        for package_report in batch_reports {
            write_package_report(&package_report)?;
            if package_report.status == WorkspaceImportStatus::Built {
                update_workspace_cache(&mut cache, &package_report)?;
            }
            statuses.insert(
                package_report.package_id.clone(),
                package_report.status.clone(),
            );
            remaining.remove(&package_report.package_id);
            package_reports.push(package_report);
        }
    }

    package_reports.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    let built = package_ids_by_status(&package_reports, WorkspaceImportStatus::Built);
    let cached = package_ids_by_status(&package_reports, WorkspaceImportStatus::Cached);
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
        cached,
        failed,
        skipped,
        blocked,
        cache_hits: package_reports
            .iter()
            .filter(|package| package.status == WorkspaceImportStatus::Cached)
            .count() as u64,
        cache_misses: package_reports
            .iter()
            .filter(|package| package.status == WorkspaceImportStatus::Built)
            .count() as u64,
        dependency_edges: dependency_edges(&manifest),
        packages: package_reports,
        next: vec![format!(
            "skillspec workspace converge {} --build-root {}",
            manifest_path.display(),
            build_root.display()
        )],
    };
    store_workspace_cache(build_root, &cache);
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
    push_id_list(&mut output, "Cached", &report.cached);
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

fn import_ready_batch(
    manifest: &WorkspaceManifest,
    build_root: &Path,
    cache: &WorkspaceCacheFile,
    ready: &[String],
) -> Vec<WorkspaceImportPackageReport> {
    let workers = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1);
    let mut reports = Vec::new();
    for chunk in ready.chunks(workers) {
        let mut chunk_reports = thread::scope(|scope| {
            let mut handles = Vec::new();
            for package_id in chunk {
                let package = manifest
                    .packages
                    .get(package_id)
                    .expect("ready package id is known");
                handles.push(scope.spawn(move || {
                    let report = import_one_package(manifest, package, build_root, cache);
                    match report {
                        Ok(report) => report,
                        Err(error) => failed_package_report(manifest, package, build_root, error),
                    }
                }));
            }
            handles
                .into_iter()
                .map(|handle| handle.join().expect("workspace import worker panicked"))
                .collect::<Vec<_>>()
        });
        reports.append(&mut chunk_reports);
    }
    reports
}

fn import_one_package(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
    build_root: &Path,
    cache: &WorkspaceCacheFile,
) -> Result<WorkspaceImportPackageReport> {
    let source_package = source_package_path(manifest, package)?;
    let output_dir = output_package_dir(package, build_root)?;
    fs::create_dir_all(&output_dir).map_err(|source| Error::Write {
        path: output_dir.clone(),
        source,
    })?;
    let source_hash = package_source_hash(&source_package)?;
    if let Some(report) = cached_package_report(manifest, package, build_root, cache, &source_hash)?
    {
        return Ok(report);
    }

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

fn cached_package_report(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
    build_root: &Path,
    cache: &WorkspaceCacheFile,
    source_hash: &str,
) -> Result<Option<WorkspaceImportPackageReport>> {
    let Some(entry) = cache.packages.get(&package.package_id) else {
        return Ok(None);
    };
    if entry.source_hash != source_hash {
        return Ok(None);
    }
    let output_dir = output_package_dir(package, build_root)?;
    let spec_path = output_dir.join("skill.spec.yml");
    if !spec_path.is_file() || file_hash(&spec_path)? != entry.spec_hash {
        return Ok(None);
    }
    if entry
        .artifacts
        .iter()
        .map(PathBuf::from)
        .any(|path| !path.is_file())
    {
        return Ok(None);
    }
    Ok(Some(package_report(
        manifest,
        package,
        build_root,
        WorkspaceImportStatus::Cached,
        Some("source_hash_unchanged".to_owned()),
        Some(output_dir.join(".skillspec/source-map/source-map.json")),
    )))
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

fn load_workspace_cache(build_root: &Path, manifest_hash: &str) -> WorkspaceCacheFile {
    let path = workspace_cache_path(build_root);
    let Some(cache) = fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<WorkspaceCacheFile>(&content).ok())
    else {
        return fresh_workspace_cache(manifest_hash);
    };
    if cache.schema == WORKSPACE_CACHE_SCHEMA
        && cache.cli_version == CLI_VERSION
        && cache.manifest_hash == manifest_hash
        && cache.command_options == IMPORT_COMMAND_OPTIONS
    {
        cache
    } else {
        fresh_workspace_cache(manifest_hash)
    }
}

fn fresh_workspace_cache(manifest_hash: &str) -> WorkspaceCacheFile {
    WorkspaceCacheFile {
        schema: WORKSPACE_CACHE_SCHEMA.to_owned(),
        cli_version: CLI_VERSION.to_owned(),
        manifest_hash: manifest_hash.to_owned(),
        command_options: IMPORT_COMMAND_OPTIONS.to_owned(),
        packages: BTreeMap::new(),
    }
}

fn store_workspace_cache(build_root: &Path, cache: &WorkspaceCacheFile) {
    let path = workspace_cache_path(build_root);
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(content) = serde_json::to_string_pretty(cache) else {
        return;
    };
    let _ = fs::write(path, format!("{content}\n"));
}

fn update_workspace_cache(
    cache: &mut WorkspaceCacheFile,
    report: &WorkspaceImportPackageReport,
) -> Result<()> {
    let source_hash = package_source_hash(&PathBuf::from(&report.source_path))?;
    let spec_hash = file_hash(&PathBuf::from(&report.spec_path))?;
    let artifacts = [
        report.spec_path.clone(),
        report.source_map_path.clone(),
        report.doctor_report_path.clone(),
        report.package_report_path.clone(),
    ]
    .into_iter()
    .filter(|path| PathBuf::from(path).is_file())
    .collect::<Vec<_>>();
    cache.packages.insert(
        report.package_id.clone(),
        WorkspaceCachePackage {
            package_id: report.package_id.clone(),
            source_hash,
            spec_hash,
            artifacts,
        },
    );
    Ok(())
}

fn workspace_cache_path(build_root: &Path) -> PathBuf {
    build_root.join(".skillspec/workspace-cache.json")
}

fn package_source_hash(source: &Path) -> Result<String> {
    let mut paths = Vec::new();
    collect_hashable_files(source, &mut paths)?;
    paths.sort();
    let mut hasher = Sha256::new();
    for path in paths {
        hasher.update(path_to_string(&path).as_bytes());
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
            namespace: None,
            local_name: None,
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
            install_slug_policy: super::super::WorkspaceInstallSlugPolicy::WorkspacePath,
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
