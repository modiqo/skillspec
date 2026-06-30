mod package_reports;
mod shape_preservation;
mod visibility_plan;

use super::{
    apply_install_slug_policy, dependency_edges, load_manifest, output_package_dir, path_to_string,
    topological_package_order, validate_manifest, write_text, WorkspaceDependencyEdge,
    WorkspaceInstallSlugPolicy, WorkspaceManifest, WorkspacePackage,
};
use package_reports::{
    blocked_multi_skill_package_report, blocked_package_report, blocked_plugin_package_report,
    failed_package_report, missing_package_report, multi_skill_package_report,
    multi_skill_target_reports, planned_public_name_collision_retirements, plugin_package_report,
    plugin_target_reports, public_name_collisions,
};
use serde::Serialize;
use shape_preservation::{
    copy_multi_skill_parent_without_skill_packages, copy_plugin_parent_without_skills,
    multi_skill_package_plan, plugin_install_identity, plugin_package_plan,
    workspace_parent_install_identity, MultiSkillPackageInstallPlan, PluginPackageInstallPlan,
};
use skillspec_authoring::git_context;
use skillspec_core::error::{Error, Result};
use skillspec_core::parser;
use skillspec_harness::install::{self, HarnessRoot, HarnessTarget, InstallStatus};
use skillspec_harness::router::Visibility;
use skillspec_harness::visibility;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use visibility_plan::{
    apply_workspace_visibility, router_refresh_advice, visibility_assignment,
    visibility_manifest_path, visibility_reports,
};

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
    pub install_slug_policy: WorkspaceInstallSlugPolicy,
    pub source_shape: super::WorkspaceSourceShape,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_skill_path: Option<String>,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub public_name_collisions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub retired_public_name_collisions: Vec<WorkspacePublicNameCollisionRetirement>,
    pub status: WorkspaceInstallTargetStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkspacePublicNameCollisionRetirement {
    pub path: String,
    pub backup_path: String,
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
    install_slug_policy: WorkspaceInstallSlugPolicy,
    source_shape: super::WorkspaceSourceShape,
    targets: Vec<String>,
    packages: Vec<WorkspaceInstalledPackage>,
}

#[derive(Clone, Debug, Serialize)]
struct WorkspaceInstalledPackage {
    package_id: String,
    public_name: String,
    install_slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugin_parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugin_skill_path: Option<String>,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    retired_public_name_collisions: Vec<WorkspacePublicNameCollisionRetirement>,
}

pub struct WorkspaceInstallRequest<'a> {
    pub manifest_path: &'a Path,
    pub build_root: &'a Path,
    pub targets: &'a [HarnessTarget],
    pub all_detected: bool,
    pub dry_run: bool,
    pub retire_existing: bool,
    pub install_slug_policy: Option<WorkspaceInstallSlugPolicy>,
    pub visibility_policy: WorkspaceVisibilityPolicy,
    pub apply_visibility: bool,
    pub visibility_manifest: Option<&'a Path>,
}

struct PreflightContext<'a> {
    build_root: &'a Path,
    targets: &'a [HarnessTarget],
    all_detected: bool,
    retire_existing: bool,
    collision_backup_root: Option<PathBuf>,
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
    let mut manifest = load_manifest(request.manifest_path)?;
    if let Some(policy) = request.install_slug_policy {
        apply_install_slug_policy(&mut manifest, policy);
    }

    let validation = validate_manifest(request.manifest_path, &manifest);
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

    let duplicate_public_names = duplicate_public_names(&manifest);
    let collision_backup_root = if request.retire_existing {
        Some(install::retired_skill_backup_root()?)
    } else {
        None
    };
    let preflight_context = PreflightContext {
        build_root: request.build_root,
        targets: request.targets,
        all_detected: request.all_detected,
        retire_existing: request.retire_existing,
        collision_backup_root,
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
        package_reports = match manifest.source_shape.kind {
            super::WorkspaceSourceShapeKind::PluginWorkspace => {
                install_plugin_packages(&manifest, request.build_root, &roots, report.packages)?
            }
            super::WorkspaceSourceShapeKind::MultiSkillWorkspace => install_multi_skill_packages(
                &manifest,
                request.build_root,
                &roots,
                report.packages,
            )?,
            _ => install_packages(
                &manifest,
                request.build_root,
                request.targets,
                request.all_detected,
                request.retire_existing,
                report.packages,
            )?,
        };
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
        "- install_slug_policy: {}\n",
        report.install_slug_policy.as_str()
    ));
    output.push_str(&format!(
        "- source_shape: {}\n",
        report.source_shape.kind.as_str()
    ));
    output.push_str(&format!(
        "- source_skill_files: {}\n",
        report.source_shape.skill_files
    ));
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
        if let Some(plugin_parent) = &package.plugin_parent {
            output.push_str(&format!("  plugin_parent: {plugin_parent}\n"));
        }
        if let Some(plugin_skill_path) = &package.plugin_skill_path {
            output.push_str(&format!("  plugin_skill_path: {plugin_skill_path}\n"));
        }
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
            for collision in &target.public_name_collisions {
                output.push_str(&format!("    public_name_collision: {collision}\n"));
            }
            for retirement in &target.retired_public_name_collisions {
                output.push_str(&format!(
                    "    retired_public_name_collision: {} backup={}\n",
                    retirement.path, retirement.backup_path
                ));
            }
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
        output.push_str("- fix missing compiled loaders, folder collisions, or public-name collisions, or rerun with --retire-existing after reviewing backup behavior; then rerun workspace install with --dry-run before installing\n");
    }
    output
}

fn preflight_packages(
    manifest: &WorkspaceManifest,
    context: &PreflightContext<'_>,
) -> Result<Vec<WorkspaceInstallPackageReport>> {
    if manifest.source_shape.kind == super::WorkspaceSourceShapeKind::PluginWorkspace {
        return preflight_plugin_packages(manifest, context);
    }
    if manifest.source_shape.kind == super::WorkspaceSourceShapeKind::MultiSkillWorkspace {
        return preflight_multi_skill_packages(manifest, context);
    }

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

fn preflight_plugin_packages(
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
        let plan = match plugin_package_plan(manifest, package) {
            Some(plan) => plan,
            None => {
                let report = blocked_package_report(
                    package,
                    context.build_root,
                    context.roots,
                    "plugin_parent_shape_not_preserved: plugin workspace package is not under a mapped plugin skills/ directory".to_owned(),
                )?;
                statuses.insert(package.package_id.clone(), report.status.clone());
                reports.push(report);
                continue;
            }
        };
        let blocked_by = package
            .depends_on
            .iter()
            .filter(|dependency| {
                statuses.get(*dependency) != Some(&WorkspaceInstallStatus::Planned)
            })
            .cloned()
            .collect::<Vec<_>>();
        let report = if blocked_by.is_empty() {
            preflight_one_plugin_package(package, &plan, context)?
        } else {
            blocked_plugin_package_report(
                package,
                &plan,
                context,
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

fn preflight_multi_skill_packages(
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
        let plan = match multi_skill_package_plan(manifest, package) {
            Some(plan) => plan,
            None => {
                let report = blocked_package_report(
                    package,
                    context.build_root,
                    context.roots,
                    "workspace_parent_shape_not_preserved: multi-skill workspace package has an invalid mapped path".to_owned(),
                )?;
                statuses.insert(package.package_id.clone(), report.status.clone());
                reports.push(report);
                continue;
            }
        };
        let blocked_by = package
            .depends_on
            .iter()
            .filter(|dependency| {
                statuses.get(*dependency) != Some(&WorkspaceInstallStatus::Planned)
            })
            .cloned()
            .collect::<Vec<_>>();
        let report = if blocked_by.is_empty() {
            preflight_one_multi_skill_package(package, &plan, context)?
        } else {
            blocked_multi_skill_package_report(
                package,
                &plan,
                context,
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

    let review_gate = match parser::load_spec(&spec_path) {
        Ok(spec) => Some(super::readiness::review_gate(&source_dir, &spec)?),
        Err(error) => {
            return failed_package_report(
                package,
                &source_dir,
                context.roots,
                format!("compiled package spec is invalid: {error}"),
            );
        }
    };
    if let Some(gate) = review_gate.filter(|gate| gate.is_blocked()) {
        return blocked_package_report(package, context.build_root, context.roots, gate.message());
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
                public_name_collisions: Vec::new(),
                retired_public_name_collisions: Vec::new(),
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

    let mut collision_backup_paths_by_identity = BTreeMap::new();
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
        match public_name_collisions(&root.path, &install_dir, &package.public_name) {
            Ok(collision_paths) if !collision_paths.is_empty() => {
                target.public_name_collisions = collision_paths
                    .iter()
                    .map(|path| path_to_string(path))
                    .collect();
                if context.retire_existing {
                    let backup_root = context
                        .collision_backup_root
                        .as_ref()
                        .expect("retire_existing preflight has backup root");
                    target.retired_public_name_collisions =
                        planned_public_name_collision_retirements(
                            &collision_paths,
                            backup_root,
                            target.target,
                            &package.public_name,
                            &mut collision_backup_paths_by_identity,
                        );
                    continue;
                }
                let message = format!(
                    "public_name {:?} already exists at {}; rerun with --retire-existing only after reviewing backup behavior",
                    package.public_name,
                    target.public_name_collisions.join(", ")
                );
                target.status = WorkspaceInstallTargetStatus::Blocked;
                target.message = Some(message.clone());
                blockers.push(message);
            }
            Ok(_) => {}
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
        plugin_parent: None,
        plugin_skill_path: None,
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

fn preflight_one_plugin_package(
    package: &WorkspacePackage,
    plan: &PluginPackageInstallPlan,
    context: &PreflightContext<'_>,
) -> Result<WorkspaceInstallPackageReport> {
    let source_dir = output_package_dir(package, context.build_root)?;
    let loader_path = source_dir.join("SKILL.md");
    let spec_path = source_dir.join("skill.spec.yml");
    if !loader_path.is_file() || !spec_path.is_file() {
        return plugin_package_report(
            package,
            plan,
            context,
            &source_dir,
            WorkspaceInstallStatus::Missing,
            WorkspaceInstallTargetStatus::Blocked,
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

    let review_gate = match parser::load_spec(&spec_path) {
        Ok(spec) => Some(super::readiness::review_gate(&source_dir, &spec)?),
        Err(error) => {
            return plugin_package_report(
                package,
                plan,
                context,
                &source_dir,
                WorkspaceInstallStatus::Failed,
                WorkspaceInstallTargetStatus::Failed,
                format!("compiled package spec is invalid: {error}"),
            );
        }
    };
    if let Some(gate) = review_gate.filter(|gate| gate.is_blocked()) {
        return blocked_plugin_package_report(package, plan, context, gate.message());
    }

    let mut target_reports = plugin_target_reports(
        package,
        plan,
        context,
        WorkspaceInstallTargetStatus::Planned,
    );
    let mut blockers = Vec::new();
    if let Some(packages) = context.duplicate_public_names.get(&package.public_name) {
        blockers.push(format!(
            "public_name {:?} is used by multiple workspace packages: {}",
            package.public_name,
            packages.join(", ")
        ));
    }

    let mut collision_backup_paths_by_identity = BTreeMap::new();
    for target in &mut target_reports {
        let Some(root) = context
            .roots
            .iter()
            .find(|root| root.target == target.target)
        else {
            continue;
        };
        let plugin_dir = root.path.join(&plan.plugin_install_slug);
        if plugin_dir.exists() && !plugin_dir.is_dir() {
            let message = format!(
                "plugin parent install target already exists and is not a directory: {}",
                plugin_dir.display()
            );
            target.status = WorkspaceInstallTargetStatus::Blocked;
            target.message = Some(message.clone());
            blockers.push(message);
            continue;
        }
        if plugin_dir.exists() && !context.retire_existing {
            let message = format!(
                "plugin_parent_shape_not_preserved: plugin parent install folder already exists; rerun with --retire-existing only after reviewing backup behavior: {}",
                plugin_dir.display()
            );
            target.status = WorkspaceInstallTargetStatus::Blocked;
            target.message = Some(message.clone());
            blockers.push(message);
            continue;
        }

        let install_dir = PathBuf::from(&target.path);
        match public_name_collisions(&root.path, &install_dir, &package.public_name) {
            Ok(collision_paths) if !collision_paths.is_empty() => {
                target.public_name_collisions = collision_paths
                    .iter()
                    .map(|path| path_to_string(path))
                    .collect();
                if context.retire_existing {
                    let backup_root = context
                        .collision_backup_root
                        .as_ref()
                        .expect("retire_existing preflight has backup root");
                    target.retired_public_name_collisions =
                        planned_public_name_collision_retirements(
                            &collision_paths,
                            backup_root,
                            target.target,
                            &package.public_name,
                            &mut collision_backup_paths_by_identity,
                        );
                    continue;
                }
                let message = format!(
                    "public_name {:?} already exists at {}; rerun with --retire-existing only after reviewing backup behavior",
                    package.public_name,
                    target.public_name_collisions.join(", ")
                );
                target.status = WorkspaceInstallTargetStatus::Blocked;
                target.message = Some(message.clone());
                blockers.push(message);
            }
            Ok(_) => {}
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
        plugin_parent: Some(plan.plugin_install_slug.clone()),
        plugin_skill_path: Some(path_to_string(&plan.skill_relative_path)),
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

fn preflight_one_multi_skill_package(
    package: &WorkspacePackage,
    plan: &MultiSkillPackageInstallPlan,
    context: &PreflightContext<'_>,
) -> Result<WorkspaceInstallPackageReport> {
    let source_dir = output_package_dir(package, context.build_root)?;
    let loader_path = source_dir.join("SKILL.md");
    let spec_path = source_dir.join("skill.spec.yml");
    if !loader_path.is_file() || !spec_path.is_file() {
        return multi_skill_package_report(
            package,
            plan,
            context,
            &source_dir,
            WorkspaceInstallStatus::Missing,
            WorkspaceInstallTargetStatus::Blocked,
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

    let review_gate = match parser::load_spec(&spec_path) {
        Ok(spec) => Some(super::readiness::review_gate(&source_dir, &spec)?),
        Err(error) => {
            return multi_skill_package_report(
                package,
                plan,
                context,
                &source_dir,
                WorkspaceInstallStatus::Failed,
                WorkspaceInstallTargetStatus::Failed,
                format!("compiled package spec is invalid: {error}"),
            );
        }
    };
    if let Some(gate) = review_gate.filter(|gate| gate.is_blocked()) {
        return blocked_multi_skill_package_report(package, plan, context, gate.message());
    }

    let mut target_reports = multi_skill_target_reports(
        package,
        plan,
        context,
        WorkspaceInstallTargetStatus::Planned,
    );
    let mut blockers = Vec::new();
    if let Some(packages) = context.duplicate_public_names.get(&package.public_name) {
        blockers.push(format!(
            "public_name {:?} is used by multiple workspace packages: {}",
            package.public_name,
            packages.join(", ")
        ));
    }

    let mut collision_backup_paths_by_identity = BTreeMap::new();
    for target in &mut target_reports {
        let Some(root) = context
            .roots
            .iter()
            .find(|root| root.target == target.target)
        else {
            continue;
        };
        let workspace_dir = root.path.join(&plan.workspace_install_slug);
        if workspace_dir.exists() && !workspace_dir.is_dir() {
            let message = format!(
                "workspace parent install target already exists and is not a directory: {}",
                workspace_dir.display()
            );
            target.status = WorkspaceInstallTargetStatus::Blocked;
            target.message = Some(message.clone());
            blockers.push(message);
            continue;
        }
        if workspace_dir.exists() && !context.retire_existing {
            let message = format!(
                "workspace_parent_shape_not_preserved: multi-skill parent install folder already exists; rerun with --retire-existing only after reviewing backup behavior: {}",
                workspace_dir.display()
            );
            target.status = WorkspaceInstallTargetStatus::Blocked;
            target.message = Some(message.clone());
            blockers.push(message);
            continue;
        }

        let install_dir = PathBuf::from(&target.path);
        match public_name_collisions(&root.path, &install_dir, &package.public_name) {
            Ok(collision_paths) if !collision_paths.is_empty() => {
                target.public_name_collisions = collision_paths
                    .iter()
                    .map(|path| path_to_string(path))
                    .collect();
                if context.retire_existing {
                    let backup_root = context
                        .collision_backup_root
                        .as_ref()
                        .expect("retire_existing preflight has backup root");
                    target.retired_public_name_collisions =
                        planned_public_name_collision_retirements(
                            &collision_paths,
                            backup_root,
                            target.target,
                            &package.public_name,
                            &mut collision_backup_paths_by_identity,
                        );
                    continue;
                }
                let message = format!(
                    "public_name {:?} already exists at {}; rerun with --retire-existing only after reviewing backup behavior",
                    package.public_name,
                    target.public_name_collisions.join(", ")
                );
                target.status = WorkspaceInstallTargetStatus::Blocked;
                target.message = Some(message.clone());
                blockers.push(message);
            }
            Ok(_) => {}
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
        plugin_parent: None,
        plugin_skill_path: None,
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
    let mut retired_public_name_collision_identities = BTreeSet::new();
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
        retire_planned_public_name_collisions(
            package_report,
            &mut retired_public_name_collision_identities,
        )?;
        let public_name_collision_fields =
            planned_public_name_collision_fields(&package_report.targets);
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
                    .map(|target| {
                        let path = path_to_string(&target.path);
                        let (public_name_collisions, retired_public_name_collisions) =
                            public_name_collision_fields
                                .get(&(target.id.to_owned(), path.clone()))
                                .cloned()
                                .unwrap_or_default();
                        WorkspaceInstallTargetReport {
                            target: target.target,
                            id: target.id.to_owned(),
                            path,
                            existed: target.existed,
                            retired_existing: target.retired_existing,
                            backup_path: target
                                .backup_path
                                .as_ref()
                                .map(|path| path_to_string(path)),
                            public_name_collisions,
                            retired_public_name_collisions,
                            status: match target.status {
                                InstallStatus::Planned => WorkspaceInstallTargetStatus::Planned,
                                InstallStatus::Installed => WorkspaceInstallTargetStatus::Installed,
                            },
                            message: None,
                        }
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

fn install_plugin_packages(
    manifest: &WorkspaceManifest,
    build_root: &Path,
    roots: &[HarnessRoot],
    mut package_reports: Vec<WorkspaceInstallPackageReport>,
) -> Result<Vec<WorkspaceInstallPackageReport>> {
    let mut statuses = BTreeMap::<String, WorkspaceInstallStatus>::new();
    let mut prepared_plugin_dirs = BTreeSet::<PathBuf>::new();
    let mut retired_plugin_identities = BTreeSet::<PathBuf>::new();
    let mut retired_public_name_collision_identities = BTreeSet::new();
    for package_report in &mut package_reports {
        let package = manifest
            .packages
            .get(&package_report.package_id)
            .expect("report only includes known packages");
        let Some(plan) = plugin_package_plan(manifest, package) else {
            package_report.status = WorkspaceInstallStatus::Blocked;
            package_report.message = Some(
                "plugin_parent_shape_not_preserved: plugin workspace package is not under a mapped plugin skills/ directory".to_owned(),
            );
            statuses.insert(package.package_id.clone(), package_report.status.clone());
            continue;
        };
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
        retire_planned_public_name_collisions(
            package_report,
            &mut retired_public_name_collision_identities,
        )?;
        let mut package_failed = None;
        for target in &mut package_report.targets {
            let Some(root) = roots.iter().find(|root| root.target == target.target) else {
                continue;
            };
            let plugin_dir = root.path.join(&plan.plugin_install_slug);
            let plugin_identity = plugin_install_identity(root, &plan.plugin_install_slug);
            if prepared_plugin_dirs.insert(plugin_identity.clone()) {
                if let Some(backup_path) = &target.backup_path {
                    if plugin_dir.exists() && retired_plugin_identities.insert(plugin_identity) {
                        install::retire_existing_skill_dir(
                            &plugin_dir,
                            &PathBuf::from(backup_path),
                        )?;
                    }
                } else if plugin_dir.exists() {
                    package_failed = Some(format!(
                        "plugin_parent_shape_not_preserved: plugin parent existed without a planned backup path: {}",
                        plugin_dir.display()
                    ));
                    break;
                }
                if let Err(error) =
                    copy_plugin_parent_without_skills(&plan.plugin_source_dir, &plugin_dir)
                {
                    package_failed = Some(error.to_string());
                    break;
                }
            }

            let install_dir = PathBuf::from(&target.path);
            match install::sync_skill_package(&source_dir, &install_dir) {
                Ok(()) => {
                    target.status = WorkspaceInstallTargetStatus::Installed;
                    target.message = None;
                }
                Err(error) => {
                    let message = error.to_string();
                    target.status = WorkspaceInstallTargetStatus::Failed;
                    target.message = Some(message.clone());
                    package_failed = Some(message);
                    break;
                }
            }
        }

        if let Some(message) = package_failed {
            package_report.status = WorkspaceInstallStatus::Failed;
            package_report.message = Some(message.clone());
            for target in &mut package_report.targets {
                if target.status != WorkspaceInstallTargetStatus::Installed {
                    target.status = WorkspaceInstallTargetStatus::Failed;
                    target.message = Some(message.clone());
                }
            }
        } else {
            package_report.status = WorkspaceInstallStatus::Installed;
            package_report.message = None;
        }
        statuses.insert(package.package_id.clone(), package_report.status.clone());
    }
    Ok(package_reports)
}

fn install_multi_skill_packages(
    manifest: &WorkspaceManifest,
    build_root: &Path,
    roots: &[HarnessRoot],
    mut package_reports: Vec<WorkspaceInstallPackageReport>,
) -> Result<Vec<WorkspaceInstallPackageReport>> {
    let mut statuses = BTreeMap::<String, WorkspaceInstallStatus>::new();
    let mut prepared_workspace_dirs = BTreeSet::<PathBuf>::new();
    let mut retired_workspace_identities = BTreeSet::<PathBuf>::new();
    let mut retired_public_name_collision_identities = BTreeSet::new();
    for package_report in &mut package_reports {
        let package = manifest
            .packages
            .get(&package_report.package_id)
            .expect("report only includes known packages");
        let Some(plan) = multi_skill_package_plan(manifest, package) else {
            package_report.status = WorkspaceInstallStatus::Blocked;
            package_report.message = Some(
                "workspace_parent_shape_not_preserved: multi-skill workspace package has an invalid mapped path".to_owned(),
            );
            statuses.insert(package.package_id.clone(), package_report.status.clone());
            continue;
        };
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
        retire_planned_public_name_collisions(
            package_report,
            &mut retired_public_name_collision_identities,
        )?;
        let mut package_failed = None;
        for target in &mut package_report.targets {
            let Some(root) = roots.iter().find(|root| root.target == target.target) else {
                continue;
            };
            let workspace_dir = root.path.join(&plan.workspace_install_slug);
            let workspace_identity =
                workspace_parent_install_identity(root, &plan.workspace_install_slug);
            if prepared_workspace_dirs.insert(workspace_identity.clone()) {
                if let Some(backup_path) = &target.backup_path {
                    if workspace_dir.exists()
                        && retired_workspace_identities.insert(workspace_identity)
                    {
                        install::retire_existing_skill_dir(
                            &workspace_dir,
                            &PathBuf::from(backup_path),
                        )?;
                    }
                } else if workspace_dir.exists() {
                    package_failed = Some(format!(
                        "workspace_parent_shape_not_preserved: multi-skill parent existed without a planned backup path: {}",
                        workspace_dir.display()
                    ));
                    break;
                }
                if let Err(error) = copy_multi_skill_parent_without_skill_packages(
                    manifest,
                    &plan.workspace_source_dir,
                    &workspace_dir,
                ) {
                    package_failed = Some(error.to_string());
                    break;
                }
            }

            let install_dir = PathBuf::from(&target.path);
            match install::sync_skill_package(&source_dir, &install_dir) {
                Ok(()) => {
                    target.status = WorkspaceInstallTargetStatus::Installed;
                    target.message = None;
                }
                Err(error) => {
                    let message = error.to_string();
                    target.status = WorkspaceInstallTargetStatus::Failed;
                    target.message = Some(message.clone());
                    package_failed = Some(message);
                    break;
                }
            }
        }

        if let Some(message) = package_failed {
            package_report.status = WorkspaceInstallStatus::Failed;
            package_report.message = Some(message.clone());
            for target in &mut package_report.targets {
                if target.status != WorkspaceInstallTargetStatus::Installed {
                    target.status = WorkspaceInstallTargetStatus::Failed;
                    target.message = Some(message.clone());
                }
            }
        } else {
            package_report.status = WorkspaceInstallStatus::Installed;
            package_report.message = None;
        }
        statuses.insert(package.package_id.clone(), package_report.status.clone());
    }
    Ok(package_reports)
}

type PublicNameCollisionFields = (Vec<String>, Vec<WorkspacePublicNameCollisionRetirement>);

fn planned_public_name_collision_fields(
    targets: &[WorkspaceInstallTargetReport],
) -> BTreeMap<(String, String), PublicNameCollisionFields> {
    targets
        .iter()
        .map(|target| {
            (
                (target.id.clone(), target.path.clone()),
                (
                    target.public_name_collisions.clone(),
                    target.retired_public_name_collisions.clone(),
                ),
            )
        })
        .collect()
}

fn retire_planned_public_name_collisions(
    package_report: &WorkspaceInstallPackageReport,
    retired_identities: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for target in &package_report.targets {
        for retirement in &target.retired_public_name_collisions {
            let collision_path = PathBuf::from(&retirement.path);
            let collision_identity = install::install_dir_identity(&collision_path);
            if retired_identities.insert(collision_identity) {
                install::retire_existing_skill_dir(
                    &collision_path,
                    &PathBuf::from(&retirement.backup_path),
                )?;
            }
        }
    }
    Ok(())
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
    let ok = failed.is_empty() && blocked.is_empty() && missing.is_empty();
    let visibility = visibility_reports(
        &packages,
        context.visibility_policy,
        context.apply_visibility,
    );
    let visibility_manifest_path = (context.visibility_policy != WorkspaceVisibilityPolicy::None)
        .then(|| visibility_manifest_path(context.build_root, context.visibility_manifest))
        .flatten();
    let router_refresh_recommended = !context.dry_run && !installed.is_empty();
    let next = install_next_steps(
        ok,
        context.dry_run,
        context.visibility_policy,
        context.apply_visibility,
        context.manifest_path,
        &context.manifest.source_root,
        context.build_root,
        context.manifest.packages.len(),
        blocked.len(),
        missing.len(),
    );
    WorkspaceInstallReport {
        ok,
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
        install_slug_policy: context.manifest.install_slug_policy,
        source_shape: context.manifest.source_shape.clone(),
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
        next,
    }
}

fn install_next_steps(
    ok: bool,
    dry_run: bool,
    visibility_policy: WorkspaceVisibilityPolicy,
    apply_visibility: bool,
    manifest_path: &Path,
    source_root: &str,
    build_root: &Path,
    package_count: usize,
    blocked_count: usize,
    missing_count: usize,
) -> Vec<String> {
    if !ok {
        if package_count > 0 && missing_count == package_count {
            return vec![
                format!(
                    "skillspec workspace import {} --out {} --summary",
                    manifest_path.display(),
                    build_root.display()
                ),
                format!(
                    "skillspec import checklist {} --build-root {} --stage loop --json",
                    manifest_path.display(),
                    build_root.display()
                ),
            ];
        }
        if blocked_count > 0 {
            return vec![
                format!(
                    "skillspec import checklist {} --build-root {} --stage loop --json",
                    manifest_path.display(),
                    build_root.display()
                ),
                "continue package promotion until the checklist reports complete; scaffold blockers are recoverable work, not terminal final-response blockers".to_owned(),
            ];
        }
        return vec![format!(
            "rerun `skillspec workspace converge {} --build-root {} --summary`, then compile and dry-run install after the reported package issues are fixed",
            manifest_path.display(),
            build_root.display()
        )];
    }
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
        install_slug_policy: report.install_slug_policy,
        source_shape: report.source_shape.clone(),
        targets: report.targets.clone(),
        packages: report
            .packages
            .iter()
            .filter(|package| package.status == WorkspaceInstallStatus::Installed)
            .map(|package| WorkspaceInstalledPackage {
                package_id: package.package_id.clone(),
                public_name: package.public_name.clone(),
                install_slug: package.install_slug.clone(),
                plugin_parent: package.plugin_parent.clone(),
                plugin_skill_path: package.plugin_skill_path.clone(),
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
                        retired_public_name_collisions: target
                            .retired_public_name_collisions
                            .clone(),
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
