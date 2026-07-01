use crate::{compiler, error, install, workspace};
use std::path::Path;
use std::time::Duration;

pub use install::HarnessTarget;
pub use workspace::{WorkspaceInstallSlugPolicy, WorkspaceVisibilityPolicy};

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

pub fn map_workspace(
    source_root: &Path,
    out: &Path,
    install_slug_policy: WorkspaceInstallSlugPolicy,
) -> error::Result<workspace::WorkspaceMapReport> {
    workspace::map_workspace(source_root, out, install_slug_policy)
}

pub fn load_manifest(path: &Path) -> error::Result<workspace::WorkspaceManifest> {
    workspace::load_manifest(path)
}

pub fn validate_workspace(path: &Path) -> error::Result<workspace::WorkspaceValidationReport> {
    workspace::validate_workspace(path)
}

pub fn import_workspace(
    manifest_path: &Path,
    build_root: &Path,
) -> error::Result<workspace::WorkspaceImportReport> {
    workspace::import_workspace(manifest_path, build_root)
}

pub fn converge_workspace(
    manifest_path: &Path,
    build_root: &Path,
) -> error::Result<workspace::WorkspaceConvergeReport> {
    workspace::converge_workspace(manifest_path, build_root)
}

pub fn compile_workspace(
    manifest_path: &Path,
    build_root: &Path,
    target: compiler::Target,
) -> error::Result<workspace::WorkspaceCompileReport> {
    workspace::compile_workspace(manifest_path, build_root, target)
}

pub fn install_workspace(
    request: WorkspaceInstallRequest<'_>,
) -> error::Result<workspace::WorkspaceInstallReport> {
    workspace::install_workspace(workspace::WorkspaceInstallRequest {
        manifest_path: request.manifest_path,
        build_root: request.build_root,
        targets: request.targets,
        all_detected: request.all_detected,
        dry_run: request.dry_run,
        retire_existing: request.retire_existing,
        install_slug_policy: request.install_slug_policy,
        visibility_policy: request.visibility_policy,
        apply_visibility: request.apply_visibility,
        visibility_manifest: request.visibility_manifest,
    })
}

pub fn render_map_report(
    report: &workspace::WorkspaceMapReport,
    manifest: &workspace::WorkspaceManifest,
) -> String {
    workspace::render_map_report(report, manifest)
}

pub fn render_map_summary(report: &workspace::WorkspaceMapReport, elapsed: Duration) -> String {
    workspace::render_map_summary(report, elapsed)
}

pub fn render_validation_report(report: &workspace::WorkspaceValidationReport) -> String {
    workspace::render_validation_report(report)
}

pub fn render_validation_summary(
    report: &workspace::WorkspaceValidationReport,
    elapsed: Duration,
) -> String {
    workspace::render_validation_summary(report, elapsed)
}

pub fn render_import_report(report: &workspace::WorkspaceImportReport) -> String {
    workspace::render_import_report(report)
}

pub fn render_import_summary(
    report: &workspace::WorkspaceImportReport,
    elapsed: Duration,
) -> String {
    workspace::render_import_summary(report, elapsed)
}

pub fn render_converge_report(report: &workspace::WorkspaceConvergeReport) -> String {
    workspace::render_converge_report(report)
}

pub fn render_converge_summary(
    report: &workspace::WorkspaceConvergeReport,
    elapsed: Duration,
) -> String {
    workspace::render_converge_summary(report, elapsed)
}

pub fn render_compile_report(report: &workspace::WorkspaceCompileReport) -> String {
    workspace::render_compile_report(report)
}

pub fn render_compile_summary(
    report: &workspace::WorkspaceCompileReport,
    elapsed: Duration,
) -> String {
    workspace::render_compile_summary(report, elapsed)
}

pub fn render_install_report(report: &workspace::WorkspaceInstallReport) -> String {
    workspace::render_install_report(report)
}

pub fn render_install_summary(
    report: &workspace::WorkspaceInstallReport,
    elapsed: Duration,
) -> String {
    workspace::render_install_summary(report, elapsed)
}
