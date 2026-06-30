use super::{
    WorkspaceInstallPackageReport, WorkspaceInstallStatus, WorkspaceVisibilityAssignment,
    WorkspaceVisibilityPolicy, WorkspaceVisibilityReport,
};
use crate::WorkspacePackage;
use skillspec_core::error::Result;
use skillspec_harness::install::HarnessRoot;
use skillspec_harness::router::Visibility;
use skillspec_harness::visibility;
use std::path::{Path, PathBuf};

pub(super) fn visibility_assignment(
    package: &WorkspacePackage,
    policy: WorkspaceVisibilityPolicy,
) -> Option<WorkspaceVisibilityAssignment> {
    visibility_target(&package.kind, policy).map(|target| WorkspaceVisibilityAssignment { target })
}

pub(super) fn visibility_reports(
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

pub(super) fn apply_workspace_visibility(
    report: &super::WorkspaceInstallReport,
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

pub(super) fn visibility_manifest_path(
    build_root: &Path,
    override_path: Option<&Path>,
) -> Option<PathBuf> {
    Some(
        override_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| build_root.join("workspace-visibility.manifest.json")),
    )
}

pub(super) fn router_refresh_advice(
    roots: &[HarnessRoot],
    visibility_manifest: Option<&Path>,
) -> Vec<String> {
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

fn visibility_target(
    kind: &crate::WorkspacePackageKind,
    policy: WorkspaceVisibilityPolicy,
) -> Option<Visibility> {
    match policy {
        WorkspaceVisibilityPolicy::None => None,
        WorkspaceVisibilityPolicy::AllImplicit => Some(Visibility::Implicit),
        WorkspaceVisibilityPolicy::AllManual => Some(Visibility::ManualOnly),
        WorkspaceVisibilityPolicy::EntryImplicit => match kind {
            crate::WorkspacePackageKind::Entry => Some(Visibility::Implicit),
            crate::WorkspacePackageKind::Shared
            | crate::WorkspacePackageKind::Helper
            | crate::WorkspacePackageKind::Wrapper => Some(Visibility::ManualOnly),
        },
    }
}
