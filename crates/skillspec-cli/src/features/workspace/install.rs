use super::{
    dependency_edges, load_manifest, output_package_dir, path_to_string, topological_package_order,
    validate_workspace, write_text, WorkspaceDependencyEdge, WorkspaceManifest, WorkspacePackage,
};
use crate::error::{Error, Result};
use crate::git_context;
use crate::install::{self, HarnessRoot, HarnessTarget, InstallStatus};
use crate::router::Visibility;
use crate::visibility;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const INSTALL_MANIFEST_SCHEMA: &str = "skillspec/workspace-install/v0";

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceInstallReport {
    pub ok: bool,
    pub dry_run: bool,
    pub manifest_path: String,
    pub build_root: String,
    pub report_path: String,
    pub install_manifest_path: String,
    pub package_count: usize,
    pub targets: Vec<String>,
    pub installed: Vec<String>,
    pub planned: Vec<String>,
    pub failed: Vec<String>,
    pub blocked: Vec<String>,
    pub missing: Vec<String>,
    pub visibility_policy: WorkspaceVisibilityPolicy,
    pub apply_visibility: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility_manifest_path: Option<String>,
    pub visibility: Vec<WorkspaceVisibilityReport>,
    pub visibility_changes: Vec<visibility::VisibilityChangeReport>,
    pub visibility_warnings: Vec<String>,
    pub router_refresh_recommended: bool,
    pub router_refresh_advice: Vec<String>,
    pub dependency_edges: Vec<WorkspaceDependencyEdge>,
    pub packages: Vec<WorkspaceInstallPackageReport>,
    pub next: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceInstallPackageReport {
    pub package_id: String,
    pub public_name: String,
    pub install_slug: String,
    pub kind: super::WorkspacePackageKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<WorkspaceVisibilityAssignment>,
    pub status: WorkspaceInstallStatus,
    pub source_dir: String,
    pub dependencies: Vec<String>,
    pub targets: Vec<WorkspaceInstallTargetReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceVisibilityPolicy {
    EntryImplicit,
    AllImplicit,
    AllManual,
    None,
}

impl WorkspaceVisibilityPolicy {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::EntryImplicit => "entry-implicit",
            Self::AllImplicit => "all-implicit",
            Self::AllManual => "all-manual",
            Self::None => "none",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct WorkspaceVisibilityAssignment {
    pub target: Visibility,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceVisibilityReport {
    pub package_id: String,
    pub public_name: String,
    pub install_slug: String,
    pub kind: super::WorkspacePackageKind,
    pub target_visibility: Visibility,
    pub applied: bool,
    pub target_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceInstallStatus {
    Planned,
    Installed,
    Failed,
    Blocked,
    Missing,
}

impl WorkspaceInstallStatus {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Installed => "installed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Missing => "missing",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceInstallTargetReport {
    pub target: HarnessTarget,
    pub id: String,
    pub path: String,
    pub existed: bool,
    pub retired_existing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    pub status: WorkspaceInstallTargetStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceInstallTargetStatus {
    Planned,
    Installed,
    Blocked,
    Failed,
}

impl WorkspaceInstallTargetStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Installed => "installed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct WorkspaceInstalledManifest {
    schema: &'static str,
    manifest_path: String,
    build_root: String,
    targets: Vec<String>,
    packages: Vec<WorkspaceInstalledPackage>,
}

#[derive(Clone, Debug, Serialize)]
struct WorkspaceInstalledPackage {
    package_id: String,
    public_name: String,
    install_slug: String,
    kind: super::WorkspacePackageKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    visibility: Option<WorkspaceVisibilityAssignment>,
    dependencies: Vec<String>,
    installs: Vec<WorkspaceInstalledTarget>,
}

#[derive(Clone, Debug, Serialize)]
struct WorkspaceInstalledTarget {
    target: HarnessTarget,
    id: String,
    path: String,
    retired_existing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_path: Option<String>,
}

pub struct WorkspaceInstallRequest<'a> {
    pub manifest_path: &'a Path,
    pub build_root: &'a Path,
    pub targets: &'a [HarnessTarget],
    pub all_detected: bool,
    pub dry_run: bool,
    pub retire_existing: bool,
    pub visibility_policy: WorkspaceVisibilityPolicy,
    pub apply_visibility: bool,
    pub visibility_manifest: Option<&'a Path>,
}

struct PreflightContext<'a> {
    build_root: &'a Path,
    targets: &'a [HarnessTarget],
    all_detected: bool,
    retire_existing: bool,
    roots: &'a [HarnessRoot],
    duplicate_public_names: &'a BTreeMap<String, Vec<String>>,
    visibility_policy: WorkspaceVisibilityPolicy,
}

struct InstallReportContext<'a> {
    manifest_path: &'a Path,
    build_root: &'a Path,
    dry_run: bool,
    roots: &'a [HarnessRoot],
    manifest: &'a WorkspaceManifest,
    visibility_policy: WorkspaceVisibilityPolicy,
    apply_visibility: bool,
    visibility_manifest: Option<&'a Path>,
}

pub fn install_workspace(request: WorkspaceInstallRequest<'_>) -> Result<WorkspaceInstallReport> {
    let validation = validate_workspace(request.manifest_path)?;
    if !validation.ok {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace install requires a valid manifest; run `skillspec workspace validate {}` first. Errors: {}",
                request.manifest_path.display(),
                validation.errors.join("; ")
            ),
        });
    }
    if !request.build_root.is_dir() {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace install build root is not a directory: {}",
                request.build_root.display()
            ),
        });
    }

    let roots = install::selected_roots(request.targets, request.all_detected)?;
    if roots.is_empty() {
        return Err(Error::InvalidInput {
            message: "no install targets selected; use --target or --all-detected".to_owned(),
        });
    }

    let manifest = load_manifest(request.manifest_path)?;
    let duplicate_public_names = duplicate_public_names(&manifest);
    let preflight_context = PreflightContext {
        build_root: request.build_root,
        targets: request.targets,
        all_detected: request.all_detected,
        retire_existing: request.retire_existing,
        roots: &roots,
        duplicate_public_names: &duplicate_public_names,
        visibility_policy: request.visibility_policy,
    };
    let report_context = InstallReportContext {
        manifest_path: request.manifest_path,
        build_root: request.build_root,
        dry_run: request.dry_run,
        roots: &roots,
        manifest: &manifest,
        visibility_policy: request.visibility_policy,
        apply_visibility: request.apply_visibility,
        visibility_manifest: request.visibility_manifest,
    };
    let mut package_reports = preflight_packages(&manifest, &preflight_context)?;

    let mut report = install_report(&report_context, package_reports, Vec::new(), Vec::new());

    if report.ok && !request.dry_run {
        package_reports = install_packages(
            &manifest,
            request.build_root,
            request.targets,
            request.all_detected,
            request.retire_existing,
            report.packages,
        )?;
        report = install_report(&report_context, package_reports, Vec::new(), Vec::new());
        if request.apply_visibility && request.visibility_policy != WorkspaceVisibilityPolicy::None
        {
            let (changes, warnings) = apply_workspace_visibility(
                &report,
                &roots,
                request.visibility_policy,
                visibility_manifest_path(request.build_root, request.visibility_manifest),
            )?;
            let packages = report.packages;
            report = install_report(&report_context, packages, changes, warnings);
        }
        if !report.installed.is_empty() {
            write_install_manifest(&report, &manifest)?;
        }
    }

    write_text(
        &PathBuf::from(&report.report_path),
        &render_install_report(&report),
    )?;
    Ok(report)
}

pub fn render_install_report(report: &WorkspaceInstallReport) -> String {
    let mut output = String::new();
    output.push_str("Workspace install\n\n");
    output.push_str(&format!("- manifest: {}\n", report.manifest_path));
    output.push_str(&format!("- build_root: {}\n", report.build_root));
    output.push_str(&format!(
        "- mode: {}\n",
        if report.dry_run { "dry-run" } else { "install" }
    ));
    output.push_str(&format!("- targets: {}\n", report.targets.join(", ")));
    output.push_str(&format!("- packages: {}\n", report.package_count));
    output.push_str(&format!(
        "- visibility_policy: {}\n",
        report.visibility_policy.as_str()
    ));
    output.push_str(&format!(
        "- apply_visibility: {}\n",
        report.apply_visibility
    ));
    if let Some(path) = &report.visibility_manifest_path {
        output.push_str(&format!("- visibility_manifest: {path}\n"));
    }
    output.push_str(&format!(
        "- status: {}\n",
        if report.ok { "ok" } else { "failed" }
    ));
    output.push_str(&format!("- report: {}\n", report.report_path));
    output.push_str(&format!(
        "- install_manifest: {}\n",
        report.install_manifest_path
    ));
    output.push('\n');

    push_id_list(&mut output, "Installed", &report.installed);
    push_id_list(&mut output, "Planned", &report.planned);
    push_id_list(&mut output, "Failed", &report.failed);
    push_id_list(&mut output, "Blocked", &report.blocked);
    push_id_list(&mut output, "Missing", &report.missing);

    output.push_str("\n## Packages\n\n");
    for package in &report.packages {
        output.push_str(&format!(
            "- {}: {} kind={} public_name={} install_slug={} source={}\n",
            package.package_id,
            package.status.as_str(),
            package.kind.as_str(),
            package.public_name,
            package.install_slug,
            package.source_dir
        ));
        if let Some(visibility) = &package.visibility {
            output.push_str(&format!("  visibility: {}\n", visibility.target.as_str()));
        }
        if !package.dependencies.is_empty() {
            output.push_str(&format!(
                "  depends_on: {}\n",
                package.dependencies.join(", ")
            ));
        }
        if let Some(message) = &package.message {
            output.push_str(&format!("  message: {message}\n"));
        }
        for target in &package.targets {
            output.push_str(&format!(
                "  - {}: {} -> {}",
                target.id,
                target.status.as_str(),
                target.path
            ));
            if target.existed {
                output.push_str(" existed=true");
            }
            if target.retired_existing {
                output.push_str(" retired_existing=true");
            }
            if let Some(backup_path) = &target.backup_path {
                output.push_str(&format!(" backup={backup_path}"));
            }
            output.push('\n');
            if let Some(message) = &target.message {
                output.push_str(&format!("    message: {message}\n"));
            }
        }
    }

    output.push_str("\n## Visibility\n\n");
    if report.visibility.is_empty() {
        output.push_str("- none\n");
    } else {
        for visibility in &report.visibility {
            output.push_str(&format!(
                "- {}: {} ({}) -> {} applied={}\n",
                visibility.package_id,
                visibility.public_name,
                visibility.kind.as_str(),
                visibility.target_visibility.as_str(),
                visibility.applied
            ));
            if let Some(message) = &visibility.message {
                output.push_str(&format!("  message: {message}\n"));
            }
        }
    }
    if !report.visibility_changes.is_empty() {
        output.push_str("\n## Native Visibility Changes\n\n");
        for change in &report.visibility_changes {
            output.push_str(&format!(
                "- {}: {} -> {} ({:?})\n",
                change.skill,
                change.before_visibility.as_str(),
                change.after_visibility.as_str(),
                change.status
            ));
        }
    }
    if !report.visibility_warnings.is_empty() {
        output.push_str("\n## Visibility Warnings\n\n");
        for warning in &report.visibility_warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }

    output.push_str("\n## Router Refresh\n\n");
    if report.router_refresh_recommended {
        for advice in &report.router_refresh_advice {
            output.push_str(&format!("- {advice}\n"));
        }
    } else {
        output.push_str("- not needed for dry-run or no-op install\n");
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
        output.push_str("- fix missing compiled loaders, folder collisions, or public-name collisions, then rerun workspace install with --dry-run before installing\n");
    }
    output
}

fn preflight_packages(
    manifest: &WorkspaceManifest,
    context: &PreflightContext<'_>,
) -> Result<Vec<WorkspaceInstallPackageReport>> {
    let mut statuses = BTreeMap::<String, WorkspaceInstallStatus>::new();
    let mut reports = Vec::new();
    for package_id in topological_package_order(manifest) {
        let package = manifest
            .packages
            .get(&package_id)
            .expect("topological order only includes known packages");
        let blocked_by = package
            .depends_on
            .iter()
            .filter(|dependency| {
                statuses.get(*dependency) != Some(&WorkspaceInstallStatus::Planned)
            })
            .cloned()
            .collect::<Vec<_>>();
        let report = if blocked_by.is_empty() {
            preflight_one_package(package, context)?
        } else {
            blocked_package_report(
                package,
                context.build_root,
                context.roots,
                format!(
                    "dependency packages are not install-ready: {}",
                    blocked_by.join(", ")
                ),
            )?
        };
        statuses.insert(package.package_id.clone(), report.status.clone());
        reports.push(report);
    }
    Ok(reports)
}

fn preflight_one_package(
    package: &WorkspacePackage,
    context: &PreflightContext<'_>,
) -> Result<WorkspaceInstallPackageReport> {
    let source_dir = output_package_dir(package, context.build_root)?;
    let loader_path = source_dir.join("SKILL.md");
    let spec_path = source_dir.join("skill.spec.yml");
    if !loader_path.is_file() || !spec_path.is_file() {
        return missing_package_report(
            package,
            &source_dir,
            context.roots,
            format!(
                "compiled package is missing {}; run workspace compile before install",
                if !loader_path.is_file() {
                    loader_path.display().to_string()
                } else {
                    spec_path.display().to_string()
                }
            ),
        );
    }

    let dry_run = install::install_skill_without_router_hook(
        &source_dir,
        context.targets,
        context.all_detected,
        true,
        false,
        context.retire_existing,
        Some(&package.install_slug),
    );
    let mut target_reports = match dry_run {
        Ok(report) => report
            .installs
            .into_iter()
            .map(|target| WorkspaceInstallTargetReport {
                target: target.target,
                id: target.id.to_owned(),
                path: path_to_string(&target.path),
                existed: target.existed,
                retired_existing: target.retired_existing,
                backup_path: target.backup_path.as_ref().map(|path| path_to_string(path)),
                status: WorkspaceInstallTargetStatus::Planned,
                message: None,
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            return failed_package_report(package, &source_dir, context.roots, error.to_string());
        }
    };

    let mut blockers = Vec::new();
    if let Some(packages) = context.duplicate_public_names.get(&package.public_name) {
        blockers.push(format!(
            "public_name {:?} is used by multiple workspace packages: {}",
            package.public_name,
            packages.join(", ")
        ));
    }

    for target in &mut target_reports {
        let install_dir = PathBuf::from(&target.path);
        if target.existed && !context.retire_existing {
            let message = format!(
                "install folder already exists; rerun with --retire-existing only after reviewing backup behavior: {}",
                install_dir.display()
            );
            target.status = WorkspaceInstallTargetStatus::Blocked;
            target.message = Some(message.clone());
            blockers.push(message);
            continue;
        }

        let Some(root) = context
            .roots
            .iter()
            .find(|root| root.target == target.target)
        else {
            continue;
        };
        match public_name_collision(&root.path, &install_dir, &package.public_name) {
            Ok(Some(collision_path)) => {
                let message = format!(
                    "public_name {:?} already exists at {}",
                    package.public_name,
                    collision_path.display()
                );
                target.status = WorkspaceInstallTargetStatus::Blocked;
                target.message = Some(message.clone());
                blockers.push(message);
            }
            Ok(None) => {}
            Err(error) => {
                let message = format!(
                    "could not inspect existing skills for public_name collisions under {}: {}",
                    root.path.display(),
                    error
                );
                target.status = WorkspaceInstallTargetStatus::Blocked;
                target.message = Some(message.clone());
                blockers.push(message);
            }
        }
    }

    let message = unique_messages(&blockers).join("; ");
    Ok(WorkspaceInstallPackageReport {
        package_id: package.package_id.clone(),
        public_name: package.public_name.clone(),
        install_slug: package.install_slug.clone(),
        kind: package.kind.clone(),
        visibility: visibility_assignment(package, context.visibility_policy),
        status: if blockers.is_empty() {
            WorkspaceInstallStatus::Planned
        } else {
            WorkspaceInstallStatus::Blocked
        },
        source_dir: path_to_string(&source_dir),
        dependencies: package.depends_on.clone(),
        targets: target_reports,
        message: (!message.is_empty()).then_some(message),
    })
}

fn install_packages(
    manifest: &WorkspaceManifest,
    build_root: &Path,
    targets: &[HarnessTarget],
    all_detected: bool,
    retire_existing: bool,
    mut package_reports: Vec<WorkspaceInstallPackageReport>,
) -> Result<Vec<WorkspaceInstallPackageReport>> {
    let mut statuses = BTreeMap::<String, WorkspaceInstallStatus>::new();
    for package_report in &mut package_reports {
        let package = manifest
            .packages
            .get(&package_report.package_id)
            .expect("report only includes known packages");
        let blocked_by = package
            .depends_on
            .iter()
            .filter(|dependency| {
                statuses.get(*dependency) != Some(&WorkspaceInstallStatus::Installed)
            })
            .cloned()
            .collect::<Vec<_>>();
        if !blocked_by.is_empty() {
            package_report.status = WorkspaceInstallStatus::Blocked;
            package_report.message = Some(format!(
                "dependency packages did not install: {}",
                blocked_by.join(", ")
            ));
            for target in &mut package_report.targets {
                target.status = WorkspaceInstallTargetStatus::Blocked;
                target.message = package_report.message.clone();
            }
            statuses.insert(package.package_id.clone(), package_report.status.clone());
            continue;
        }

        let source_dir = output_package_dir(package, build_root)?;
        match install::install_skill_without_router_hook(
            &source_dir,
            targets,
            all_detected,
            false,
            false,
            retire_existing,
            Some(&package.install_slug),
        ) {
            Ok(report) => {
                package_report.status = WorkspaceInstallStatus::Installed;
                package_report.targets = report
                    .installs
                    .into_iter()
                    .map(|target| WorkspaceInstallTargetReport {
                        target: target.target,
                        id: target.id.to_owned(),
                        path: path_to_string(&target.path),
                        existed: target.existed,
                        retired_existing: target.retired_existing,
                        backup_path: target.backup_path.as_ref().map(|path| path_to_string(path)),
                        status: match target.status {
                            InstallStatus::Planned => WorkspaceInstallTargetStatus::Planned,
                            InstallStatus::Installed => WorkspaceInstallTargetStatus::Installed,
                        },
                        message: None,
                    })
                    .collect();
                package_report.message = None;
            }
            Err(error) => {
                package_report.status = WorkspaceInstallStatus::Failed;
                package_report.message = Some(error.to_string());
                for target in &mut package_report.targets {
                    target.status = WorkspaceInstallTargetStatus::Failed;
                    target.message = package_report.message.clone();
                }
            }
        }
        statuses.insert(package.package_id.clone(), package_report.status.clone());
    }
    Ok(package_reports)
}

fn install_report(
    context: &InstallReportContext<'_>,
    packages: Vec<WorkspaceInstallPackageReport>,
    visibility_changes: Vec<visibility::VisibilityChangeReport>,
    visibility_warnings: Vec<String>,
) -> WorkspaceInstallReport {
    let report_path = context.build_root.join("workspace-install.report.md");
    let install_manifest_path = context.build_root.join("workspace-install.manifest.json");
    let installed = package_ids_by_status(&packages, WorkspaceInstallStatus::Installed);
    let planned = package_ids_by_status(&packages, WorkspaceInstallStatus::Planned);
    let failed = package_ids_by_status(&packages, WorkspaceInstallStatus::Failed);
    let blocked = package_ids_by_status(&packages, WorkspaceInstallStatus::Blocked);
    let missing = package_ids_by_status(&packages, WorkspaceInstallStatus::Missing);
    let visibility = visibility_reports(
        &packages,
        context.visibility_policy,
        context.apply_visibility,
    );
    let visibility_manifest_path = (context.visibility_policy != WorkspaceVisibilityPolicy::None)
        .then(|| visibility_manifest_path(context.build_root, context.visibility_manifest))
        .flatten();
    let router_refresh_recommended = !context.dry_run && !installed.is_empty();
    WorkspaceInstallReport {
        ok: failed.is_empty() && blocked.is_empty() && missing.is_empty(),
        dry_run: context.dry_run,
        manifest_path: path_to_string(context.manifest_path),
        build_root: path_to_string(context.build_root),
        report_path: path_to_string(&report_path),
        install_manifest_path: path_to_string(&install_manifest_path),
        package_count: context.manifest.packages.len(),
        targets: context
            .roots
            .iter()
            .map(|root| root.id.to_owned())
            .collect(),
        installed,
        planned,
        failed,
        blocked,
        missing,
        visibility_policy: context.visibility_policy,
        apply_visibility: context.apply_visibility,
        visibility_manifest_path: visibility_manifest_path
            .as_ref()
            .map(|path| path_to_string(path)),
        visibility,
        visibility_changes,
        visibility_warnings,
        router_refresh_recommended,
        router_refresh_advice: router_refresh_advice(
            context.roots,
            visibility_manifest_path.as_deref(),
        ),
        dependency_edges: dependency_edges(context.manifest),
        packages,
        next: install_next_steps(
            context.dry_run,
            context.visibility_policy,
            context.apply_visibility,
            &context.manifest.source_root,
            context.build_root,
        ),
    }
}

fn install_next_steps(
    dry_run: bool,
    visibility_policy: WorkspaceVisibilityPolicy,
    apply_visibility: bool,
    source_root: &str,
    build_root: &Path,
) -> Vec<String> {
    if dry_run {
        return vec![
            "rerun the same command without --dry-run after reviewing planned writes and visibility targets".to_owned(),
        ];
    }

    let mut next = vec![
        "inspect workspace-install.manifest.json for installed package ids, slugs, public names, visibility targets, and target paths".to_owned(),
    ];
    if visibility_policy != WorkspaceVisibilityPolicy::None {
        if apply_visibility {
            next.push(
                "inspect workspace-visibility.manifest.json for reversible native visibility changes"
                    .to_owned(),
            );
        } else {
            next.push(
                "inspect visibility targets in the install report; rerun with --apply-visibility to write native visibility metadata when ready".to_owned(),
            );
        }
    }
    next.push("refresh router indexes separately if this harness uses router mode".to_owned());
    next.extend(git_context::workspace_pull_request_next_steps(
        Path::new(source_root),
        build_root,
    ));
    next
}

fn write_install_manifest(
    report: &WorkspaceInstallReport,
    manifest: &WorkspaceManifest,
) -> Result<()> {
    let installed_manifest = WorkspaceInstalledManifest {
        schema: INSTALL_MANIFEST_SCHEMA,
        manifest_path: report.manifest_path.clone(),
        build_root: report.build_root.clone(),
        targets: report.targets.clone(),
        packages: report
            .packages
            .iter()
            .filter(|package| package.status == WorkspaceInstallStatus::Installed)
            .map(|package| WorkspaceInstalledPackage {
                package_id: package.package_id.clone(),
                public_name: package.public_name.clone(),
                install_slug: package.install_slug.clone(),
                kind: package.kind.clone(),
                visibility: package.visibility,
                dependencies: manifest
                    .packages
                    .get(&package.package_id)
                    .map(|source| source.depends_on.clone())
                    .unwrap_or_default(),
                installs: package
                    .targets
                    .iter()
                    .filter(|target| target.status == WorkspaceInstallTargetStatus::Installed)
                    .map(|target| WorkspaceInstalledTarget {
                        target: target.target,
                        id: target.id.clone(),
                        path: target.path.clone(),
                        retired_existing: target.retired_existing,
                        backup_path: target.backup_path.clone(),
                    })
                    .collect(),
            })
            .collect(),
    };
    write_json(
        &PathBuf::from(&report.install_manifest_path),
        &installed_manifest,
    )
}

fn visibility_assignment(
    package: &WorkspacePackage,
    policy: WorkspaceVisibilityPolicy,
) -> Option<WorkspaceVisibilityAssignment> {
    visibility_target(&package.kind, policy).map(|target| WorkspaceVisibilityAssignment { target })
}

fn visibility_target(
    kind: &super::WorkspacePackageKind,
    policy: WorkspaceVisibilityPolicy,
) -> Option<Visibility> {
    match policy {
        WorkspaceVisibilityPolicy::None => None,
        WorkspaceVisibilityPolicy::AllImplicit => Some(Visibility::Implicit),
        WorkspaceVisibilityPolicy::AllManual => Some(Visibility::ManualOnly),
        WorkspaceVisibilityPolicy::EntryImplicit => match kind {
            super::WorkspacePackageKind::Entry => Some(Visibility::Implicit),
            super::WorkspacePackageKind::Shared
            | super::WorkspacePackageKind::Helper
            | super::WorkspacePackageKind::Wrapper => Some(Visibility::ManualOnly),
        },
    }
}

fn visibility_reports(
    packages: &[WorkspaceInstallPackageReport],
    policy: WorkspaceVisibilityPolicy,
    apply_visibility: bool,
) -> Vec<WorkspaceVisibilityReport> {
    if policy == WorkspaceVisibilityPolicy::None {
        return Vec::new();
    }
    packages
        .iter()
        .filter_map(|package| {
            let visibility = package.visibility?;
            Some(WorkspaceVisibilityReport {
                package_id: package.package_id.clone(),
                public_name: package.public_name.clone(),
                install_slug: package.install_slug.clone(),
                kind: package.kind.clone(),
                target_visibility: visibility.target,
                applied: apply_visibility && package.status == WorkspaceInstallStatus::Installed,
                target_paths: package
                    .targets
                    .iter()
                    .map(|target| target.path.clone())
                    .collect(),
                message: Some(match visibility.target {
                    Visibility::Implicit => {
                        "user-facing package remains visible for native selection".to_owned()
                    }
                    Visibility::ManualOnly => {
                        "support package is installed but not implicitly selected".to_owned()
                    }
                    Visibility::NameOnly => "package is name-only for router selection".to_owned(),
                    Visibility::Off => "package is hidden from router selection".to_owned(),
                }),
            })
        })
        .collect()
}

fn apply_workspace_visibility(
    report: &WorkspaceInstallReport,
    roots: &[HarnessRoot],
    policy: WorkspaceVisibilityPolicy,
    manifest_path: Option<PathBuf>,
) -> Result<(Vec<visibility::VisibilityChangeReport>, Vec<String>)> {
    let Some(manifest_path) = manifest_path else {
        return Ok((Vec::new(), Vec::new()));
    };
    let roots = roots
        .iter()
        .map(|root| root.path.clone())
        .collect::<Vec<_>>();
    let skills = report
        .packages
        .iter()
        .filter(|package| package.status == WorkspaceInstallStatus::Installed)
        .filter_map(|package| {
            let target = package.visibility.map(|assignment| assignment.target)?;
            Some(visibility::SkillVisibilityTarget {
                skill: package.public_name.clone(),
                visibility: target,
            })
        })
        .collect::<Vec<_>>();
    let apply_report = visibility::set_visibilities(visibility::SetVisibilitiesOptions {
        roots,
        skills,
        manifest: manifest_path,
        dry_run: false,
    })?;
    let changes = apply_report.changes;
    let mut warnings = apply_report.warnings;
    if policy == WorkspaceVisibilityPolicy::EntryImplicit
        && !changes
            .iter()
            .any(|change| change.after_visibility == Visibility::ManualOnly)
    {
        warnings.push(
            "entry-implicit policy applied, but no support package required native visibility files"
                .to_owned(),
        );
    }
    Ok((changes, warnings))
}

fn visibility_manifest_path(build_root: &Path, override_path: Option<&Path>) -> Option<PathBuf> {
    Some(
        override_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| build_root.join("workspace-visibility.manifest.json")),
    )
}

fn router_refresh_advice(roots: &[HarnessRoot], visibility_manifest: Option<&Path>) -> Vec<String> {
    let roots_arg = roots
        .iter()
        .map(|root| format!("--roots {}", root.path.display()))
        .collect::<Vec<_>>()
        .join(" ");
    let visibility_arg = visibility_manifest
        .map(|path| format!(" --visibility-manifest {}", path.display()))
        .unwrap_or_default();
    vec![
        "workspace install does not refresh router indexes".to_owned(),
        format!(
            "if router mode is enabled, run `skillspec router index refresh {roots_arg} --index <router-index>{visibility_arg}`"
        ),
    ]
}

fn missing_package_report(
    package: &WorkspacePackage,
    source_dir: &Path,
    roots: &[HarnessRoot],
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    Ok(WorkspaceInstallPackageReport {
        package_id: package.package_id.clone(),
        public_name: package.public_name.clone(),
        install_slug: package.install_slug.clone(),
        kind: package.kind.clone(),
        visibility: None,
        status: WorkspaceInstallStatus::Missing,
        source_dir: path_to_string(source_dir),
        dependencies: package.depends_on.clone(),
        targets: target_reports_for_roots(package, roots, WorkspaceInstallTargetStatus::Blocked),
        message: Some(message),
    })
}

fn failed_package_report(
    package: &WorkspacePackage,
    source_dir: &Path,
    roots: &[HarnessRoot],
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    Ok(WorkspaceInstallPackageReport {
        package_id: package.package_id.clone(),
        public_name: package.public_name.clone(),
        install_slug: package.install_slug.clone(),
        kind: package.kind.clone(),
        visibility: None,
        status: WorkspaceInstallStatus::Failed,
        source_dir: path_to_string(source_dir),
        dependencies: package.depends_on.clone(),
        targets: target_reports_for_roots(package, roots, WorkspaceInstallTargetStatus::Failed),
        message: Some(message),
    })
}

fn blocked_package_report(
    package: &WorkspacePackage,
    build_root: &Path,
    roots: &[HarnessRoot],
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    let source_dir = output_package_dir(package, build_root)?;
    Ok(WorkspaceInstallPackageReport {
        package_id: package.package_id.clone(),
        public_name: package.public_name.clone(),
        install_slug: package.install_slug.clone(),
        kind: package.kind.clone(),
        visibility: None,
        status: WorkspaceInstallStatus::Blocked,
        source_dir: path_to_string(&source_dir),
        dependencies: package.depends_on.clone(),
        targets: target_reports_for_roots(package, roots, WorkspaceInstallTargetStatus::Blocked),
        message: Some(message),
    })
}

fn target_reports_for_roots(
    package: &WorkspacePackage,
    roots: &[HarnessRoot],
    status: WorkspaceInstallTargetStatus,
) -> Vec<WorkspaceInstallTargetReport> {
    roots
        .iter()
        .map(|root| {
            let path = root.path.join(&package.install_slug);
            WorkspaceInstallTargetReport {
                target: root.target,
                id: root.id.to_owned(),
                existed: path.exists(),
                path: path_to_string(&path),
                retired_existing: false,
                backup_path: None,
                status: status.clone(),
                message: None,
            }
        })
        .collect()
}

fn public_name_collision(
    root: &Path,
    install_dir: &Path,
    public_name: &str,
) -> Result<Option<PathBuf>> {
    if !root.is_dir() {
        return Ok(None);
    }
    for entry in fs::read_dir(root).map_err(|source| Error::Read {
        path: root.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if same_path(&path, install_dir) || !path.is_dir() {
            continue;
        }
        if installed_public_name(&path)?.as_deref() == Some(public_name) {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn installed_public_name(skill_dir: &Path) -> Result<Option<String>> {
    let skill_path = skill_dir.join("SKILL.md");
    if !skill_path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(&skill_path).map_err(|source| Error::Read {
        path: skill_path.clone(),
        source,
    })?;
    let frontmatter = parse_frontmatter(&skill_path, &content)?;
    Ok(frontmatter
        .get("name")
        .and_then(|value| value.as_str())
        .map(str::to_owned))
}

fn parse_frontmatter(path: &Path, content: &str) -> Result<BTreeMap<String, serde_yaml::Value>> {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(rest) = normalized.strip_prefix("---") else {
        return Ok(BTreeMap::new());
    };
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"));
    let Some(rest) = rest else {
        return Ok(BTreeMap::new());
    };

    let mut yaml = String::new();
    for line in rest.lines() {
        if line.trim() == "---" {
            if yaml.trim().is_empty() {
                return Ok(BTreeMap::new());
            }
            return serde_yaml::from_str(&yaml).map_err(|source| Error::ParseYaml {
                path: path.to_path_buf(),
                source,
            });
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    Ok(BTreeMap::new())
}

fn duplicate_public_names(manifest: &WorkspaceManifest) -> BTreeMap<String, Vec<String>> {
    let mut by_name = BTreeMap::<String, Vec<String>>::new();
    for package in manifest.packages.values() {
        by_name
            .entry(package.public_name.clone())
            .or_default()
            .push(package.package_id.clone());
    }
    by_name
        .into_iter()
        .filter_map(|(name, packages)| (packages.len() > 1).then_some((name, packages)))
        .collect()
}

fn package_ids_by_status(
    packages: &[WorkspaceInstallPackageReport],
    status: WorkspaceInstallStatus,
) -> Vec<String> {
    packages
        .iter()
        .filter_map(|package| (package.status == status).then_some(package.package_id.clone()))
        .collect()
}

fn unique_messages(messages: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    messages
        .iter()
        .filter_map(|message| {
            if seen.insert(message.clone()) {
                Some(message.clone())
            } else {
                None
            }
        })
        .collect()
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
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
