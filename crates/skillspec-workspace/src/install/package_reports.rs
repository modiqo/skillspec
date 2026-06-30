use super::{
    output_package_dir, path_to_string, PreflightContext, WorkspaceInstallPackageReport,
    WorkspaceInstallStatus, WorkspaceInstallTargetReport, WorkspaceInstallTargetStatus,
    WorkspacePublicNameCollisionRetirement,
};
use crate::WorkspacePackage;
use skillspec_core::error::{Error, Result};
use skillspec_harness::install::{self, HarnessRoot, HarnessTarget};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn missing_package_report(
    package: &WorkspacePackage,
    source_dir: &Path,
    roots: &[HarnessRoot],
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    Ok(WorkspaceInstallPackageReport {
        package_id: package.package_id.clone(),
        public_name: package.public_name.clone(),
        install_slug: package.install_slug.clone(),
        plugin_parent: None,
        plugin_skill_path: None,
        kind: package.kind.clone(),
        visibility: None,
        status: WorkspaceInstallStatus::Missing,
        source_dir: path_to_string(source_dir),
        dependencies: package.depends_on.clone(),
        targets: target_reports_for_roots(package, roots, WorkspaceInstallTargetStatus::Blocked),
        message: Some(message),
    })
}

pub(super) fn plugin_package_report(
    package: &WorkspacePackage,
    plan: &super::PluginPackageInstallPlan,
    context: &PreflightContext<'_>,
    source_dir: &Path,
    status: WorkspaceInstallStatus,
    target_status: WorkspaceInstallTargetStatus,
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    Ok(WorkspaceInstallPackageReport {
        package_id: package.package_id.clone(),
        public_name: package.public_name.clone(),
        install_slug: package.install_slug.clone(),
        plugin_parent: Some(plan.plugin_install_slug.clone()),
        plugin_skill_path: Some(path_to_string(&plan.skill_relative_path)),
        kind: package.kind.clone(),
        visibility: None,
        status,
        source_dir: path_to_string(source_dir),
        dependencies: package.depends_on.clone(),
        targets: plugin_target_reports(package, plan, context, target_status),
        message: Some(message),
    })
}

pub(super) fn blocked_plugin_package_report(
    package: &WorkspacePackage,
    plan: &super::PluginPackageInstallPlan,
    context: &PreflightContext<'_>,
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    let source_dir = output_package_dir(package, context.build_root)?;
    plugin_package_report(
        package,
        plan,
        context,
        &source_dir,
        WorkspaceInstallStatus::Blocked,
        WorkspaceInstallTargetStatus::Blocked,
        message,
    )
}

pub(super) fn multi_skill_package_report(
    package: &WorkspacePackage,
    plan: &super::MultiSkillPackageInstallPlan,
    context: &PreflightContext<'_>,
    source_dir: &Path,
    status: WorkspaceInstallStatus,
    target_status: WorkspaceInstallTargetStatus,
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    Ok(WorkspaceInstallPackageReport {
        package_id: package.package_id.clone(),
        public_name: package.public_name.clone(),
        install_slug: package.install_slug.clone(),
        plugin_parent: None,
        plugin_skill_path: None,
        kind: package.kind.clone(),
        visibility: None,
        status,
        source_dir: path_to_string(source_dir),
        dependencies: package.depends_on.clone(),
        targets: multi_skill_target_reports(package, plan, context, target_status),
        message: Some(message),
    })
}

pub(super) fn blocked_multi_skill_package_report(
    package: &WorkspacePackage,
    plan: &super::MultiSkillPackageInstallPlan,
    context: &PreflightContext<'_>,
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    let source_dir = output_package_dir(package, context.build_root)?;
    multi_skill_package_report(
        package,
        plan,
        context,
        &source_dir,
        WorkspaceInstallStatus::Blocked,
        WorkspaceInstallTargetStatus::Blocked,
        message,
    )
}

pub(super) fn failed_package_report(
    package: &WorkspacePackage,
    source_dir: &Path,
    roots: &[HarnessRoot],
    message: String,
) -> Result<WorkspaceInstallPackageReport> {
    Ok(WorkspaceInstallPackageReport {
        package_id: package.package_id.clone(),
        public_name: package.public_name.clone(),
        install_slug: package.install_slug.clone(),
        plugin_parent: None,
        plugin_skill_path: None,
        kind: package.kind.clone(),
        visibility: None,
        status: WorkspaceInstallStatus::Failed,
        source_dir: path_to_string(source_dir),
        dependencies: package.depends_on.clone(),
        targets: target_reports_for_roots(package, roots, WorkspaceInstallTargetStatus::Failed),
        message: Some(message),
    })
}

pub(super) fn blocked_package_report(
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
        plugin_parent: None,
        plugin_skill_path: None,
        kind: package.kind.clone(),
        visibility: None,
        status: WorkspaceInstallStatus::Blocked,
        source_dir: path_to_string(&source_dir),
        dependencies: package.depends_on.clone(),
        targets: target_reports_for_roots(package, roots, WorkspaceInstallTargetStatus::Blocked),
        message: Some(message),
    })
}

pub(super) fn public_name_collisions(
    root: &Path,
    install_dir: &Path,
    public_name: &str,
) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut collisions = Vec::new();
    for entry in fs::read_dir(root).map_err(|source| Error::Read {
        path: root.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if super::same_path(&path, install_dir) || !path.is_dir() {
            continue;
        }
        if installed_public_name(&path)?.as_deref() == Some(public_name) {
            collisions.push(path);
        }
    }
    collisions.sort();
    Ok(collisions)
}

pub(super) fn planned_public_name_collision_retirements(
    collisions: &[PathBuf],
    backup_root: &Path,
    target: HarnessTarget,
    public_name: &str,
    backup_paths_by_identity: &mut BTreeMap<PathBuf, PathBuf>,
) -> Vec<WorkspacePublicNameCollisionRetirement> {
    collisions
        .iter()
        .map(|collision_path| {
            let identity = install::install_dir_identity(collision_path);
            let backup_path = backup_paths_by_identity
                .entry(identity)
                .or_insert_with(|| {
                    install::retired_skill_backup_path(
                        backup_root,
                        target,
                        collision_backup_name(collision_path, public_name).as_ref(),
                    )
                })
                .clone();
            WorkspacePublicNameCollisionRetirement {
                path: path_to_string(collision_path),
                backup_path: path_to_string(&backup_path),
            }
        })
        .collect()
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
                public_name_collisions: Vec::new(),
                retired_public_name_collisions: Vec::new(),
                status: status.clone(),
                message: None,
            }
        })
        .collect()
}

pub(super) fn plugin_target_reports(
    package: &WorkspacePackage,
    plan: &super::PluginPackageInstallPlan,
    context: &PreflightContext<'_>,
    status: WorkspaceInstallTargetStatus,
) -> Vec<WorkspaceInstallTargetReport> {
    context
        .roots
        .iter()
        .map(|root| {
            let plugin_dir = root.path.join(&plan.plugin_install_slug);
            let path = plugin_dir.join("skills").join(&plan.skill_relative_path);
            let existed = plugin_dir.exists() || path.exists();
            let backup_path = (plugin_dir.exists() && context.retire_existing)
                .then(|| {
                    context.collision_backup_root.as_ref().map(|backup_root| {
                        install::retired_skill_backup_path(
                            backup_root,
                            root.target,
                            &plan.plugin_install_slug,
                        )
                    })
                })
                .flatten();
            WorkspaceInstallTargetReport {
                target: root.target,
                id: root.id.to_owned(),
                existed,
                path: path_to_string(&path),
                retired_existing: plugin_dir.exists() && context.retire_existing,
                backup_path: backup_path.as_ref().map(|path| path_to_string(path)),
                public_name_collisions: Vec::new(),
                retired_public_name_collisions: Vec::new(),
                status: status.clone(),
                message: Some(format!(
                    "plugin_parent={} plugin_skill_path={} package_slug={} shape=preserve-plugin-parent",
                    plan.plugin_install_slug,
                    path_to_string(&plan.skill_relative_path),
                    package.install_slug
                )),
            }
        })
        .collect()
}

pub(super) fn multi_skill_target_reports(
    package: &WorkspacePackage,
    plan: &super::MultiSkillPackageInstallPlan,
    context: &PreflightContext<'_>,
    status: WorkspaceInstallTargetStatus,
) -> Vec<WorkspaceInstallTargetReport> {
    context
        .roots
        .iter()
        .map(|root| {
            let workspace_dir = root.path.join(&plan.workspace_install_slug);
            let path = if plan.package_relative_path.as_os_str().is_empty() {
                workspace_dir.clone()
            } else {
                workspace_dir.join(&plan.package_relative_path)
            };
            let existed = workspace_dir.exists() || path.exists();
            let backup_path = (workspace_dir.exists() && context.retire_existing)
                .then(|| {
                    context.collision_backup_root.as_ref().map(|backup_root| {
                        install::retired_skill_backup_path(
                            backup_root,
                            root.target,
                            &plan.workspace_install_slug,
                        )
                    })
                })
                .flatten();
            WorkspaceInstallTargetReport {
                target: root.target,
                id: root.id.to_owned(),
                existed,
                path: path_to_string(&path),
                retired_existing: workspace_dir.exists() && context.retire_existing,
                backup_path: backup_path.as_ref().map(|path| path_to_string(path)),
                public_name_collisions: Vec::new(),
                retired_public_name_collisions: Vec::new(),
                status: status.clone(),
                message: Some(format!(
                    "workspace_parent={} workspace_skill_path={} package_slug={} shape=preserve-workspace-parent",
                    plan.workspace_install_slug,
                    path_to_string(&plan.package_relative_path),
                    package.install_slug
                )),
            }
        })
        .collect()
}

fn collision_backup_name(collision_path: &Path, public_name: &str) -> String {
    collision_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(public_name)
        .to_owned()
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
            return serde_yaml::from_str::<BTreeMap<String, serde_yaml::Value>>(&yaml).map_err(
                |source| Error::InvalidInput {
                    message: format!("invalid frontmatter in {}: {source}", path.display()),
                },
            );
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    Ok(BTreeMap::new())
}
