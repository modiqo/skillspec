use crate::{durable_lifecycle, error, install, router, router_lifecycle, status, visibility};
use std::path::{Path, PathBuf};

pub use durable_lifecycle::{
    DurableDeleteOptions, DurableInstallOptions, DurableModeOptions, DurableUpdateOptions,
};
pub use install::HarnessTarget;
pub use router::{IndexStatusOptions, Visibility};
pub use router_lifecycle::{
    RouterGuardOptions, RouterInstallOptions, RouterModeOptions, RouterRefreshOptions,
    RouterUninstallOptions, RouterUpdateOptions,
};
pub use status::StatusOptions;
pub use visibility::{
    SetVisibilityOptions, VisibilityApplyOptions, VisibilityPlanOptions, VisibilityRestoreOptions,
};

pub fn detect_targets() -> error::Result<Vec<install::HarnessRoot>> {
    install::detect_targets()
}

#[allow(clippy::too_many_arguments)]
pub fn install_skill(
    folder: &Path,
    targets: &[HarnessTarget],
    all_detected: bool,
    dry_run: bool,
    force: bool,
    retire_existing: bool,
    name: Option<&str>,
) -> error::Result<install::InstallReport> {
    install::install_skill(
        folder,
        targets,
        all_detected,
        dry_run,
        force,
        retire_existing,
        name,
    )
}

pub fn status_report(options: StatusOptions) -> error::Result<status::StatusReport> {
    status::status(options)
}

pub fn render_status(report: &status::StatusReport) -> String {
    status::render(report)
}

pub fn audit_skills(roots: &[PathBuf]) -> error::Result<router::AuditReport> {
    router::audit(roots)
}

pub fn render_skill_audit(report: &router::AuditReport) -> String {
    router::render_audit(report)
}

pub fn router_index_status(
    options: router::IndexStatusOptions,
) -> error::Result<router::IndexStatusReport> {
    router::index_status(options)
}

pub fn render_router_index_status(report: &router::IndexStatusReport) -> String {
    router::render_index_status(report)
}

pub fn install_router(
    options: RouterInstallOptions,
) -> error::Result<router_lifecycle::RouterInstallReport> {
    router_lifecycle::install(options)
}

pub fn uninstall_router(
    options: RouterUninstallOptions,
) -> error::Result<router_lifecycle::RouterUninstallReport> {
    router_lifecycle::uninstall(options)
}

pub fn update_router(
    options: RouterUpdateOptions,
) -> error::Result<router_lifecycle::RouterUpdateReport> {
    router_lifecycle::update(options)
}

pub fn enable_router(
    options: RouterModeOptions,
) -> error::Result<router_lifecycle::RouterModeReport> {
    router_lifecycle::enable(options)
}

pub fn disable_router(
    options: RouterModeOptions,
) -> error::Result<router_lifecycle::RouterModeReport> {
    router_lifecycle::disable(options)
}

pub fn guard_router(
    options: RouterGuardOptions,
) -> error::Result<router_lifecycle::RouterGuardReport> {
    router_lifecycle::guard(options)
}

pub fn refresh_router_index(
    options: RouterRefreshOptions,
) -> error::Result<router_lifecycle::RouterRefreshReport> {
    router_lifecycle::refresh(options)
}

pub fn render_router_install(report: &router_lifecycle::RouterInstallReport) -> String {
    router_lifecycle::render_install(report)
}

pub fn render_router_uninstall(report: &router_lifecycle::RouterUninstallReport) -> String {
    router_lifecycle::render_uninstall(report)
}

pub fn render_router_update(report: &router_lifecycle::RouterUpdateReport) -> String {
    router_lifecycle::render_update(report)
}

pub fn render_router_mode(report: &router_lifecycle::RouterModeReport) -> String {
    router_lifecycle::render_mode(report)
}

pub fn render_router_guard(report: &router_lifecycle::RouterGuardReport) -> String {
    router_lifecycle::render_guard(report)
}

pub fn render_router_guard_hook_json(
    report: &router_lifecycle::RouterGuardReport,
) -> error::Result<String> {
    router_lifecycle::render_guard_hook_json(report)
}

pub fn render_router_refresh(report: &router_lifecycle::RouterRefreshReport) -> String {
    router_lifecycle::render_refresh(report)
}

pub fn plan_visibility(
    options: VisibilityPlanOptions,
) -> error::Result<visibility::VisibilityPlanReport> {
    visibility::plan(options)
}

pub fn apply_visibility(
    options: VisibilityApplyOptions,
) -> error::Result<visibility::VisibilityApplyReport> {
    visibility::apply(options)
}

pub fn restore_visibility(
    options: VisibilityRestoreOptions,
) -> error::Result<visibility::VisibilityRestoreReport> {
    visibility::restore(options)
}

pub fn set_visibility(
    options: SetVisibilityOptions,
) -> error::Result<visibility::VisibilityApplyReport> {
    visibility::set_visibility(options)
}

pub fn render_visibility_plan(report: &visibility::VisibilityPlanReport) -> String {
    visibility::render_plan(report)
}

pub fn render_visibility_apply(report: &visibility::VisibilityApplyReport) -> String {
    visibility::render_apply(report)
}

pub fn render_visibility_restore(report: &visibility::VisibilityRestoreReport) -> String {
    visibility::render_restore(report)
}

pub fn install_durable(
    options: DurableInstallOptions,
) -> error::Result<durable_lifecycle::DurableInstallReport> {
    durable_lifecycle::install(options)
}

pub fn update_durable(
    options: DurableUpdateOptions,
) -> error::Result<durable_lifecycle::DurableUpdateReport> {
    durable_lifecycle::update(options)
}

pub fn delete_durable(
    options: DurableDeleteOptions,
) -> error::Result<durable_lifecycle::DurableDeleteReport> {
    durable_lifecycle::delete(options)
}

pub fn enable_durable(
    options: DurableModeOptions,
) -> error::Result<durable_lifecycle::DurableModeReport> {
    durable_lifecycle::enable(options)
}

pub fn disable_durable(
    options: DurableModeOptions,
) -> error::Result<durable_lifecycle::DurableModeReport> {
    durable_lifecycle::disable(options)
}

pub fn render_durable_install(report: &durable_lifecycle::DurableInstallReport) -> String {
    durable_lifecycle::render_install(report)
}

pub fn render_durable_update(report: &durable_lifecycle::DurableUpdateReport) -> String {
    durable_lifecycle::render_update(report)
}

pub fn render_durable_delete(report: &durable_lifecycle::DurableDeleteReport) -> String {
    durable_lifecycle::render_delete(report)
}

pub fn render_durable_mode(report: &durable_lifecycle::DurableModeReport) -> String {
    durable_lifecycle::render_mode(report)
}
