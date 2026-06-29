use crate::durable_lifecycle;
use crate::error::{Error, Result};
use crate::router::{self, IndexReport, IndexStatusReport};
use crate::visibility::{self, VisibilityApplyReport, VisibilityRestoreReport};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG_SCHEMA: &str = "skillspec/router-config/v1";
const ROUTER_MARKER: &str = ".skillspec-router-managed";
const DEFAULT_ROUTER_NAME: &str = "skill-router";

#[derive(Clone, Debug)]
pub struct RouterInstallOptions {
    pub roots: Vec<PathBuf>,
    pub index: PathBuf,
    pub manifest: Option<PathBuf>,
    pub router_name: Option<String>,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct RouterUninstallOptions {
    pub manifest: Option<PathBuf>,
    pub router_name: Option<String>,
    pub index: Option<PathBuf>,
    pub keep_index: bool,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct RouterUpdateOptions {
    pub backup_dir: Option<PathBuf>,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct RouterModeOptions {
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct RouterRefreshOptions {
    pub roots: Vec<PathBuf>,
    pub index: PathBuf,
    pub visibility_manifest: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct RouterGuardOptions {
    pub config: Option<PathBuf>,
    pub hook: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterInstallReport {
    pub router_name: String,
    pub router_skill_dir: PathBuf,
    pub router_skill_dirs: Vec<PathBuf>,
    pub index: PathBuf,
    pub manifest: PathBuf,
    pub config: PathBuf,
    pub dry_run: bool,
    pub router_skill_status: RouterFileStatus,
    pub router_skill_reports: Vec<RouterSkillReport>,
    pub durable_executor: DurableExecutorReport,
    pub visibility: VisibilityApplyReport,
    pub index_report: Option<IndexReport>,
    pub preparedness: RouterPreparednessReport,
    pub harness_hooks: Vec<RouterHarnessHookReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterUninstallReport {
    pub router_name: String,
    pub router_skill_dir: PathBuf,
    pub router_skill_dirs: Vec<PathBuf>,
    pub manifest: PathBuf,
    pub index: Option<PathBuf>,
    pub config: PathBuf,
    pub dry_run: bool,
    pub router_skill_status: RouterFileStatus,
    pub router_skill_reports: Vec<RouterSkillReport>,
    pub index_removed: bool,
    pub config_removed: bool,
    pub restore: VisibilityRestoreReport,
    pub harness_hooks: Vec<RouterHarnessHookReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterUpdateReport {
    pub router_name: String,
    pub router_skill_dirs: Vec<PathBuf>,
    pub index: PathBuf,
    pub manifest: PathBuf,
    pub config: PathBuf,
    pub dry_run: bool,
    pub backup: Option<RouterBackupReport>,
    pub router_skill_reports: Vec<RouterSkillReport>,
    pub durable_executor: DurableExecutorReport,
    pub visibility: VisibilityApplyReport,
    pub index_report: Option<IndexReport>,
    pub preparedness: Option<RouterPreparednessReport>,
    pub harness_hooks: Vec<RouterHarnessHookReport>,
    pub restart_warning: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterModeReport {
    pub router_name: String,
    pub router_skill_dirs: Vec<PathBuf>,
    pub index: PathBuf,
    pub manifest: PathBuf,
    pub config: PathBuf,
    pub enabled: bool,
    pub dry_run: bool,
    pub router_skill_reports: Vec<RouterSkillReport>,
    pub durable_executor: DurableExecutorReport,
    pub visibility: VisibilityApplyReport,
    pub index_report: Option<IndexReport>,
    pub preparedness: Option<RouterPreparednessReport>,
    pub harness_hooks: Vec<RouterHarnessHookReport>,
    pub restart_warning: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterStatusReport {
    pub installed: bool,
    pub enabled: bool,
    pub disabled: bool,
    pub config: PathBuf,
    pub router_name: Option<String>,
    pub roots: Vec<PathBuf>,
    pub router_skill_dirs: Vec<RouterSkillInstallStatus>,
    pub index: Option<PathBuf>,
    pub manifest: Option<PathBuf>,
    pub index_status: Option<IndexStatusReport>,
    pub harness_hooks: Vec<RouterHarnessHookReport>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterSkillInstallStatus {
    pub path: PathBuf,
    pub present: bool,
    pub managed: bool,
    pub has_skill_md: bool,
    pub has_skill_spec: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterBackupReport {
    pub path: PathBuf,
    pub items: Vec<RouterBackupItem>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterBackupItem {
    pub kind: &'static str,
    pub source: PathBuf,
    pub backup: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterRefreshReport {
    pub config: Option<PathBuf>,
    pub router_config_present: bool,
    pub roots: Vec<PathBuf>,
    pub index: PathBuf,
    pub visibility_manifest: Option<PathBuf>,
    pub status_before: IndexStatusReport,
    pub visibility: Option<VisibilityApplyReport>,
    pub index_report: IndexReport,
    pub preparedness: Option<RouterPreparednessReport>,
    pub advice: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterGuardReport {
    pub config: PathBuf,
    pub installed: bool,
    pub enabled: bool,
    pub repaired: bool,
    pub first_hop_ready: bool,
    pub router_skill_dirs: Vec<RouterSkillInstallStatus>,
    pub status_before: Option<IndexStatusReport>,
    pub status_after: Option<IndexStatusReport>,
    pub visibility: Option<VisibilityApplyReport>,
    pub index_report: Option<IndexReport>,
    pub preparedness: Option<RouterPreparednessReport>,
    pub repair_command: String,
    pub message: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterHookReport {
    pub config: PathBuf,
    pub router_skill_reports: Vec<RouterSkillReport>,
    pub visibility: VisibilityApplyReport,
    pub index_report: IndexReport,
    pub preparedness: RouterPreparednessReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterHarnessHookReport {
    pub harness: RouterHarness,
    pub path: PathBuf,
    pub status: RouterHarnessHookStatus,
    pub command: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouterHarness {
    Codex,
    Claude,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterHarnessHookStatus {
    Planned,
    Installed,
    Removed,
    Missing,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterSkillReport {
    pub path: PathBuf,
    pub status: RouterFileStatus,
}

#[derive(Clone, Debug, Serialize)]
pub struct DurableExecutorReport {
    pub present: bool,
    pub skill_dir: Option<PathBuf>,
    pub message: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterPreparednessReport {
    pub ready: bool,
    pub visibility_checked: bool,
    pub index_built: bool,
    pub status_checked: bool,
    pub index_exists: bool,
    pub index_stale: bool,
    pub indexed_skills: usize,
    pub discovered_skills: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterFileStatus {
    Planned,
    Installed,
    Removed,
    Missing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RouterConfig {
    schema: String,
    created_at_unix: u64,
    #[serde(default = "default_enabled")]
    enabled: bool,
    roots: Vec<PathBuf>,
    #[serde(default)]
    router_skill_dirs: Vec<PathBuf>,
    index: PathBuf,
    manifest: PathBuf,
    #[serde(default, rename = "router_root", skip_serializing)]
    legacy_router_root: Option<PathBuf>,
    router_name: String,
}

pub fn default_router_name() -> &'static str {
    DEFAULT_ROUTER_NAME
}

pub fn direct_index_warnings() -> Vec<String> {
    let mut warnings = vec![
        "`skillspec index` is router-specific: it builds the SQLite catalog used by `skillspec route` and the optional skill-router. It is not source analysis, workspace recon, or skill import; use `skillspec source map` for one prose skill and `skillspec workspace map` for multi-skill/plugin source roots.".to_owned(),
    ];

    let config_path = match config_path() {
        Ok(path) => path,
        Err(error) => {
            warnings.push(format!(
                "Could not locate router config while checking router state: {error}. The index was still written, but router activation state is unknown."
            ));
            return warnings;
        }
    };

    match read_config_optional() {
        Ok(Some(config)) if config.enabled => warnings.push(
            "Router mode is installed and enabled. For installed-router maintenance, prefer `skillspec router index refresh` because it reapplies router-managed visibility and checks preparedness; direct `skillspec index` only rewrites the catalog."
                .to_owned(),
        ),
        Ok(Some(_)) => warnings.push(format!(
            "Router mode is installed but disabled in {}. This index will not affect implicit skill selection until router mode is enabled. Use `skillspec route --index <index> --query <task>` for manual lookup, or run `skillspec router enable` to reactivate router mode.",
            config_path.display()
        )),
        Ok(None) => warnings.push(
            "No installed router config was found. This index is a standalone router catalog for manual `skillspec route`; it does not install or activate the skill-router. Run `skillspec router install` to activate router mode."
                .to_owned(),
        ),
        Err(error) => warnings.push(format!(
            "Could not inspect router config at {}: {error}. The index was still written, but router activation state is unknown.",
            config_path.display()
        )),
    }

    warnings
}

pub fn install(options: RouterInstallOptions) -> Result<RouterInstallReport> {
    if options.roots.is_empty() {
        return Err(Error::InvalidInput {
            message: "router install requires at least one --roots path".to_owned(),
        });
    }
    let index = router::normalize_index_path(options.index);
    let router_name = options
        .router_name
        .unwrap_or_else(|| DEFAULT_ROUTER_NAME.to_owned());
    validate_router_name(&router_name)?;
    let router_skill_dirs = router_skill_dirs_for_roots(&options.roots, &router_name);
    let router_skill_dir = router_skill_dirs[0].clone();
    let manifest = options
        .manifest
        .clone()
        .unwrap_or_else(|| default_manifest_for_index(&index));
    let config = config_path()?;
    let durable_executor = inspect_durable_executor(&options.roots)?;

    let router_skill_reports =
        install_router_skills(&router_skill_dirs, &router_name, &index, options.dry_run)?;
    let visibility = apply_router_visibility(
        &options.roots,
        &router_name,
        true,
        manifest.clone(),
        options.dry_run,
    )?;
    let (index_report, preparedness, harness_hooks) = if options.dry_run {
        let planned_config = build_config(
            &options.roots,
            &router_skill_dirs,
            &index,
            &manifest,
            &router_name,
            true,
        );
        (
            None,
            RouterPreparednessReport {
                ready: false,
                visibility_checked: true,
                index_built: false,
                status_checked: false,
                index_exists: false,
                index_stale: true,
                indexed_skills: 0,
                discovered_skills: 0,
                warnings: vec![
                    "dry run did not write files, build the index, or check status".to_owned(),
                ],
            },
            hooks::apply_harness_hooks_for_config(&config, &planned_config, true, true)?,
        )
    } else {
        let report = router::index(router::IndexOptions {
            roots: options.roots.clone(),
            out: index.clone(),
            visibility_manifest: Some(manifest.clone()),
        })?;
        let preparedness =
            check_preparedness(&options.roots, &index, &manifest, &visibility, &report)?;
        write_config(
            &config,
            &options.roots,
            &router_skill_dirs,
            &index,
            &manifest,
            &router_name,
            true,
        )?;
        let saved_config =
            read_config_optional_at(&config)?.ok_or_else(|| Error::InvalidInput {
                message: format!(
                    "router install wrote no readable config at {}",
                    config.display()
                ),
            })?;
        let harness_hooks =
            hooks::apply_harness_hooks_for_config(&config, &saved_config, true, false)?;
        (Some(report), preparedness, harness_hooks)
    };

    Ok(RouterInstallReport {
        router_name,
        router_skill_dir,
        router_skill_dirs,
        index,
        manifest,
        config,
        dry_run: options.dry_run,
        router_skill_status: if options.dry_run {
            RouterFileStatus::Planned
        } else {
            RouterFileStatus::Installed
        },
        router_skill_reports,
        durable_executor,
        visibility,
        index_report,
        preparedness,
        harness_hooks,
    })
}

pub fn uninstall(options: RouterUninstallOptions) -> Result<RouterUninstallReport> {
    let config = read_config_optional()?;
    let config_path = config_path()?;
    let router_name = options
        .router_name
        .or_else(|| config.as_ref().map(|config| config.router_name.clone()))
        .unwrap_or_else(|| DEFAULT_ROUTER_NAME.to_owned());
    validate_router_name(&router_name)?;
    let router_skill_dirs = config
        .as_ref()
        .map(|config| configured_router_skill_dirs(config, &router_name))
        .ok_or_else(|| Error::InvalidInput {
            message: "router uninstall requires router config to locate the managed router skill"
                .to_owned(),
        })?;
    if router_skill_dirs.is_empty() {
        return Err(Error::InvalidInput {
            message: "router uninstall requires router config to locate the managed router skill"
                .to_owned(),
        });
    }
    let manifest = options
        .manifest
        .or_else(|| config.as_ref().map(|config| config.manifest.clone()))
        .ok_or_else(|| Error::InvalidInput {
            message: "router uninstall requires --manifest when no router config exists".to_owned(),
        })?;
    let index = options
        .index
        .map(router::normalize_index_path)
        .or_else(|| config.as_ref().map(|config| config.index.clone()));
    let router_skill_dir = router_skill_dirs[0].clone();

    let restore = visibility::restore(visibility::VisibilityRestoreOptions {
        manifest: manifest.clone(),
        dry_run: options.dry_run,
    })?;
    let harness_hooks = hooks::apply_harness_hooks_for_config(
        &config_path,
        config
            .as_ref()
            .expect("router config was required before uninstall"),
        false,
        options.dry_run,
    )?;

    let router_skill_reports = remove_router_skills(&router_skill_dirs, options.dry_run)?;
    let router_skill_status = aggregate_router_file_status(&router_skill_reports);
    let mut index_removed = false;
    if !options.keep_index {
        if let Some(index) = &index {
            if index.exists() {
                index_removed = true;
                if !options.dry_run {
                    fs::remove_file(index).map_err(|source| Error::Write {
                        path: index.clone(),
                        source,
                    })?;
                }
            }
        }
    }
    let mut config_removed = false;
    if config_path.exists() {
        config_removed = true;
        if !options.dry_run {
            fs::remove_file(&config_path).map_err(|source| Error::Write {
                path: config_path.clone(),
                source,
            })?;
        }
    }

    Ok(RouterUninstallReport {
        router_name,
        router_skill_dir,
        router_skill_dirs,
        manifest,
        index,
        config: config_path,
        dry_run: options.dry_run,
        router_skill_status,
        router_skill_reports,
        index_removed,
        config_removed,
        restore,
        harness_hooks,
    })
}

pub fn update(options: RouterUpdateOptions) -> Result<RouterUpdateReport> {
    let config = read_config_optional()?.ok_or_else(|| Error::InvalidInput {
        message: "router update requires an existing router config; run router install first"
            .to_owned(),
    })?;
    let config_path = config_path()?;
    let router_skill_dirs = configured_router_skill_dirs(&config, &config.router_name);
    if router_skill_dirs.is_empty() {
        return Err(Error::InvalidInput {
            message: "router update requires router config to locate managed router skills"
                .to_owned(),
        });
    }
    let backup = if options.dry_run {
        None
    } else {
        Some(create_router_backup(
            options.backup_dir,
            &config_path,
            &config,
            &router_skill_dirs,
        )?)
    };
    let durable_executor = inspect_durable_executor(&config.roots)?;
    let router_skill_reports = install_router_skills(
        &router_skill_dirs,
        &config.router_name,
        &config.index,
        options.dry_run,
    )?;
    let visibility = apply_router_visibility(
        &config.roots,
        &config.router_name,
        config.enabled,
        config.manifest.clone(),
        options.dry_run,
    )?;
    let (index_report, preparedness) = if options.dry_run || !config.enabled {
        (None, None)
    } else {
        let report = router::index(router::IndexOptions {
            roots: config.roots.clone(),
            out: config.index.clone(),
            visibility_manifest: Some(config.manifest.clone()),
        })?;
        let preparedness = check_preparedness(
            &config.roots,
            &config.index,
            &config.manifest,
            &visibility,
            &report,
        )?;
        write_config(
            &config_path,
            &config.roots,
            &router_skill_dirs,
            &config.index,
            &config.manifest,
            &config.router_name,
            config.enabled,
        )?;
        (Some(report), Some(preparedness))
    };
    let harness_hooks = hooks::apply_harness_hooks_for_config(
        &config_path,
        &config,
        config.enabled,
        options.dry_run,
    )?;

    Ok(RouterUpdateReport {
        router_name: config.router_name,
        router_skill_dirs,
        index: config.index,
        manifest: config.manifest,
        config: config_path,
        dry_run: options.dry_run,
        backup,
        router_skill_reports,
        durable_executor,
        visibility,
        index_report,
        preparedness,
        harness_hooks,
        restart_warning: restart_warning(),
    })
}

pub fn enable(options: RouterModeOptions) -> Result<RouterModeReport> {
    set_enabled(true, options)
}

pub fn disable(options: RouterModeOptions) -> Result<RouterModeReport> {
    set_enabled(false, options)
}

pub fn status() -> Result<RouterStatusReport> {
    let config_path = config_path()?;
    let Some(config) = read_config_optional()? else {
        return Ok(RouterStatusReport {
            installed: false,
            enabled: false,
            disabled: false,
            config: config_path,
            router_name: None,
            roots: Vec::new(),
            router_skill_dirs: Vec::new(),
            index: None,
            manifest: None,
            index_status: None,
            harness_hooks: Vec::new(),
            warnings: Vec::new(),
        });
    };

    let router_skill_dirs = configured_router_skill_dirs(&config, &config.router_name)
        .into_iter()
        .map(|path| RouterSkillInstallStatus {
            present: path.is_dir(),
            managed: path.join(ROUTER_MARKER).is_file(),
            has_skill_md: path.join("SKILL.md").is_file(),
            has_skill_spec: path.join("skill.spec.yml").is_file(),
            path,
        })
        .collect::<Vec<_>>();
    let index_status = router::index_status(router::IndexStatusOptions {
        roots: config.roots.clone(),
        index: config.index.clone(),
        visibility_manifest: Some(config.manifest.clone()),
    })?;
    let warnings = index_status.warnings.clone();
    let harness_hooks = hooks::inspect_harness_hooks_for_config(&config_path, &config);

    Ok(RouterStatusReport {
        installed: true,
        enabled: config.enabled,
        disabled: !config.enabled,
        config: config_path,
        router_name: Some(config.router_name),
        roots: config.roots,
        router_skill_dirs,
        index: Some(config.index),
        manifest: Some(config.manifest),
        index_status: Some(index_status),
        harness_hooks,
        warnings,
    })
}

pub fn after_skill_install() -> Result<Option<RouterHookReport>> {
    let Some(config) = read_config_optional()? else {
        return Ok(None);
    };
    if !config.enabled {
        return Ok(None);
    }
    let router_skill_reports = install_router_skills(
        &configured_router_skill_dirs(&config, &config.router_name),
        &config.router_name,
        &config.index,
        false,
    )?;
    let visibility = apply_router_visibility(
        &config.roots,
        &config.router_name,
        true,
        config.manifest.clone(),
        false,
    )?;
    let index_report = router::index(router::IndexOptions {
        roots: config.roots.clone(),
        out: config.index.clone(),
        visibility_manifest: Some(config.manifest.clone()),
    })?;
    let preparedness = check_preparedness(
        &config.roots,
        &config.index,
        &config.manifest,
        &visibility,
        &index_report,
    )?;
    Ok(Some(RouterHookReport {
        config: config_path()?,
        router_skill_reports,
        visibility,
        index_report,
        preparedness,
    }))
}

pub fn refresh(options: RouterRefreshOptions) -> Result<RouterRefreshReport> {
    let config = read_config_optional()?;
    let router_config_present = config.is_some();
    let config_path = router_config_present.then(config_path).transpose()?;
    let manifest = options
        .visibility_manifest
        .clone()
        .or_else(|| config.as_ref().map(|config| config.manifest.clone()));
    let index = router::normalize_index_path(options.index);
    let status_before = router::index_status(router::IndexStatusOptions {
        roots: options.roots.clone(),
        index: index.clone(),
        visibility_manifest: manifest.clone(),
    })?;

    let visibility =
        if router_config_present && config.as_ref().is_some_and(|config| config.enabled) {
            let Some(manifest) = manifest.clone() else {
                return Err(Error::InvalidInput {
                    message: "router index refresh found router config but no visibility manifest"
                        .to_owned(),
                });
            };
            if let Some(config) = &config {
                install_router_skills(
                    &configured_router_skill_dirs(config, &config.router_name),
                    &config.router_name,
                    &index,
                    false,
                )?;
            }
            Some(apply_router_visibility(
                &options.roots,
                &config
                    .as_ref()
                    .map(|config| config.router_name.clone())
                    .unwrap_or_else(|| DEFAULT_ROUTER_NAME.to_owned()),
                true,
                manifest,
                false,
            )?)
        } else {
            None
        };

    let index_report = router::index(router::IndexOptions {
        roots: options.roots.clone(),
        out: index.clone(),
        visibility_manifest: manifest.clone(),
    })?;
    let preparedness = match (&visibility, &manifest) {
        (Some(visibility), Some(manifest)) => Some(check_preparedness(
            &options.roots,
            &index,
            manifest,
            visibility,
            &index_report,
        )?),
        _ => None,
    };

    Ok(RouterRefreshReport {
        config: config_path,
        router_config_present,
        roots: options.roots,
        index,
        visibility_manifest: manifest,
        advice: status_before.advice.clone(),
        status_before,
        visibility,
        index_report,
        preparedness,
    })
}

pub fn guard(options: RouterGuardOptions) -> Result<RouterGuardReport> {
    hooks::guard(options)
}

fn check_preparedness(
    roots: &[PathBuf],
    index: &Path,
    manifest: &Path,
    visibility: &VisibilityApplyReport,
    index_report: &IndexReport,
) -> Result<RouterPreparednessReport> {
    let status = router::index_status(router::IndexStatusOptions {
        roots: roots.to_vec(),
        index: index.to_path_buf(),
        visibility_manifest: Some(manifest.to_path_buf()),
    })?;
    let report = preparedness_from_status(visibility, index_report, &status);
    if !report.ready {
        return Err(Error::InvalidInput {
            message: format!(
                "router preparedness check failed after indexing: exists={}, stale={}, indexed={}, discovered={}",
                report.index_exists,
                report.index_stale,
                report.indexed_skills,
                report.discovered_skills
            ),
        });
    }
    Ok(report)
}

fn preparedness_from_status(
    visibility: &VisibilityApplyReport,
    index_report: &IndexReport,
    status: &IndexStatusReport,
) -> RouterPreparednessReport {
    let visibility_checked = visibility
        .changes
        .iter()
        .all(|change| matches!(change.status, visibility::VisibilityChangeStatus::Applied));
    let index_built = index_report.skills_indexed == status.indexed_skills;
    let mut warnings = status.warnings.clone();
    warnings.extend(index_report.warnings.iter().cloned());
    warnings.extend(visibility.warnings.iter().cloned());
    let ready = visibility_checked
        && index_built
        && status.exists
        && !status.stale
        && status.indexed_skills == status.discovered_skills;
    RouterPreparednessReport {
        ready,
        visibility_checked,
        index_built,
        status_checked: true,
        index_exists: status.exists,
        index_stale: status.stale,
        indexed_skills: status.indexed_skills,
        discovered_skills: status.discovered_skills,
        warnings,
    }
}

mod hooks;
mod render;
mod template;

pub use render::{
    render_guard, render_guard_hook_json, render_install, render_mode, render_refresh,
    render_uninstall, render_update,
};

fn router_skill_dirs_for_roots(roots: &[PathBuf], router_name: &str) -> Vec<PathBuf> {
    roots.iter().map(|root| root.join(router_name)).collect()
}

fn install_router_skills(
    skill_dirs: &[PathBuf],
    router_name: &str,
    index: &Path,
    dry_run: bool,
) -> Result<Vec<RouterSkillReport>> {
    let mut reports = Vec::new();
    for skill_dir in skill_dirs {
        if !dry_run {
            write_router_skill(skill_dir, router_name, index)?;
        }
        reports.push(RouterSkillReport {
            path: skill_dir.clone(),
            status: if dry_run {
                RouterFileStatus::Planned
            } else {
                RouterFileStatus::Installed
            },
        });
    }
    Ok(reports)
}

fn create_router_backup(
    backup_dir: Option<PathBuf>,
    config_path: &Path,
    config: &RouterConfig,
    router_skill_dirs: &[PathBuf],
) -> Result<RouterBackupReport> {
    let backup_root = backup_dir.unwrap_or_else(|| default_update_backup_dir(config_path));
    if backup_root.exists() {
        return Err(Error::InvalidInput {
            message: format!(
                "router update backup directory already exists: {}",
                backup_root.display()
            ),
        });
    }
    fs::create_dir_all(&backup_root).map_err(|source| Error::Write {
        path: backup_root.clone(),
        source,
    })?;
    let mut items = Vec::new();

    backup_file_if_present(
        "config",
        config_path,
        &backup_root.join("config.json"),
        &mut items,
    )?;
    backup_file_if_present(
        "manifest",
        &config.manifest,
        &backup_root.join("visibility-manifest.json"),
        &mut items,
    )?;
    backup_file_if_present(
        "index",
        &config.index,
        &backup_root.join("skill-index.sqlite"),
        &mut items,
    )?;

    for (index, skill_dir) in router_skill_dirs.iter().enumerate() {
        if skill_dir.exists() {
            let destination = backup_root.join(format!("router-skill-{index}"));
            copy_dir_recursive(skill_dir, &destination)?;
            items.push(RouterBackupItem {
                kind: "router_skill_dir",
                source: skill_dir.clone(),
                backup: destination,
            });
        }
    }

    let report = RouterBackupReport {
        path: backup_root.clone(),
        items,
    };
    let backup_manifest = backup_root.join("backup.json");
    let json = serde_json::to_string_pretty(&report).map_err(Error::RenderJson)?;
    write_file(&backup_manifest, &format!("{json}\n"))?;
    Ok(report)
}

fn default_update_backup_dir(config_path: &Path) -> PathBuf {
    let base = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
        .join(format!("update-{}", now_unix()));
    if !base.exists() {
        return base;
    }
    for index in 1.. {
        let candidate = base.with_file_name(format!(
            "{}-{index}",
            base.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("update")
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
    base
}

fn restart_warning() -> String {
    "Restart active Codex, Claude, Agents, or vendor harness sessions so they reload updated router skill files, guard hooks, and native visibility metadata.".to_owned()
}

fn backup_file_if_present(
    kind: &'static str,
    source: &Path,
    destination: &Path,
    items: &mut Vec<RouterBackupItem>,
) -> Result<()> {
    if !source.is_file() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::copy(source, destination).map_err(|source_error| Error::Write {
        path: destination.to_path_buf(),
        source: source_error,
    })?;
    items.push(RouterBackupItem {
        kind,
        source: source.to_path_buf(),
        backup: destination.to_path_buf(),
    });
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(|source_error| Error::Write {
        path: destination.to_path_buf(),
        source: source_error,
    })?;
    for entry in fs::read_dir(source).map_err(|source_error| Error::Read {
        path: source.to_path_buf(),
        source: source_error,
    })? {
        let entry = entry.map_err(|source_error| Error::Read {
            path: source.to_path_buf(),
            source: source_error,
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|source_error| Error::Read {
            path: source_path.clone(),
            source: source_error,
        })?;
        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|source_error| Error::Write {
                path: destination_path,
                source: source_error,
            })?;
        }
    }
    Ok(())
}

fn write_router_skill(skill_dir: &Path, router_name: &str, index: &Path) -> Result<()> {
    fs::create_dir_all(skill_dir).map_err(|source| Error::Write {
        path: skill_dir.to_path_buf(),
        source,
    })?;
    let router_name_yaml = template::yaml_single_quote(router_name);
    let index_arg = template::shell_single_quote(&index.display().to_string());
    let skill = format!(
        r#"---
name: {router_name_yaml}
description: Use for every user request when SkillSpec router mode is enabled. First check the local SkillSpec router index, load the selected skill only when route decision is use_skill, and continue with normal agent behavior when the decision is bypass or ambiguous.
metadata:
  routing:
    tags: [skills, router, discovery, codex, claude]
    triggers:
      - every request
      - any request
      - user request
      - tell me
      - explain
      - enlighten
      - what is
      - help with
      - primary skill discovery
      - choose from installed skills
      - choose the right skill
      - route to a skill
      - find matching skill
      - select local skill
      - many skills installed
      - skill descriptions shortened
      - install skill router
      - disable implicit skill invocation
---

# Skill Router

This is a thin native harness loader generated by `skillspec router install`.
When router mode is enabled and the harness has been restarted, this router is
the first hop for every user request in the managed skill roots. All routed
skills are explicit-only/manual-only, so the router must decide before any
domain skill is read.

## Fast Path

For ordinary user requests, do not read `./skill.spec.yml` and do not run
`skillspec router index status`. The prompt hook already ran
`skillspec router guard`; when it reports `first_hop_ready=true`, run only:

```bash
skillspec route --index {index_arg} --query '<user task>' --top 5 --json
```

Load a domain skill only when route JSON returns `decision: "use_skill"` and a
non-null `selected` skill. If the decision is `bypass` or `ambiguous`, do not
load any candidate skill; continue with the normal agent path for the user
request.

Load and follow `./skill.spec.yml` only for router lifecycle, repair,
visibility, guard, index status, index refresh, or when the prompt hook is
missing or reports that router readiness failed.
`durable-executor` remains implicit only when its own lifecycle state is enabled.
"#
    );
    write_file(&skill_dir.join("SKILL.md"), &skill)?;
    write_file(
        &skill_dir.join("skill.spec.yml"),
        &template::render_router_spec(router_name, index),
    )?;
    write_file(
        &skill_dir.join(ROUTER_MARKER),
        &format!(
            "schema: skillspec/router-managed/v1\ncreated_at_unix: {}\n",
            now_unix()
        ),
    )
}

fn validate_router_name(router_name: &str) -> Result<()> {
    let mut chars = router_name.chars();
    let valid = chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|char| {
            char.is_ascii_lowercase()
                || char.is_ascii_digit()
                || char == '-'
                || char == '_'
                || char == '.'
        })
        && !router_name.contains("..");
    if valid {
        return Ok(());
    }

    Err(Error::InvalidInput {
        message: format!(
            "router name must start with a lowercase ASCII letter and contain only lowercase ASCII letters, digits, '-', '_', or '.': {router_name:?}"
        ),
    })
}

fn remove_router_skills(skill_dirs: &[PathBuf], dry_run: bool) -> Result<Vec<RouterSkillReport>> {
    let mut reports = Vec::new();
    for skill_dir in skill_dirs {
        let status = remove_router_skill(skill_dir, dry_run)?;
        reports.push(RouterSkillReport {
            path: skill_dir.clone(),
            status,
        });
    }
    Ok(reports)
}

fn remove_router_skill(skill_dir: &Path, dry_run: bool) -> Result<RouterFileStatus> {
    if !skill_dir.exists() {
        return Ok(RouterFileStatus::Missing);
    }
    let marker = skill_dir.join(ROUTER_MARKER);
    if !marker.is_file() {
        return Err(Error::InvalidInput {
            message: format!(
                "refusing to remove router skill without managed marker: {}",
                skill_dir.display()
            ),
        });
    }
    if !dry_run {
        fs::remove_dir_all(skill_dir).map_err(|source| Error::Write {
            path: skill_dir.to_path_buf(),
            source,
        })?;
    }
    Ok(RouterFileStatus::Removed)
}

fn aggregate_router_file_status(reports: &[RouterSkillReport]) -> RouterFileStatus {
    if reports
        .iter()
        .any(|report| matches!(report.status, RouterFileStatus::Removed))
    {
        return RouterFileStatus::Removed;
    }
    if reports
        .iter()
        .any(|report| matches!(report.status, RouterFileStatus::Installed))
    {
        return RouterFileStatus::Installed;
    }
    if reports
        .iter()
        .any(|report| matches!(report.status, RouterFileStatus::Planned))
    {
        return RouterFileStatus::Planned;
    }
    RouterFileStatus::Missing
}

fn inspect_durable_executor(roots: &[PathBuf]) -> Result<DurableExecutorReport> {
    let mut warnings = Vec::new();
    let entries = router::scan_roots(roots, &mut warnings)?;
    let durable_enabled = durable_lifecycle::is_enabled_for_router()?;
    if let Some(entry) = entries
        .iter()
        .find(|entry| entry.name == visibility::ROUTER_MANAGED_IMPLICIT_EXCEPTION)
    {
        let mode = if durable_enabled {
            "implicit"
        } else {
            "explicit-only"
        };
        return Ok(DurableExecutorReport {
            present: true,
            skill_dir: Some(entry.skill_dir.clone()),
            message: format!(
                "{} present in managed roots; durable lifecycle keeps it {mode}",
                visibility::ROUTER_MANAGED_IMPLICIT_EXCEPTION,
            ),
            warnings,
        });
    }

    warnings.push(format!(
        "{} is not installed in managed roots; durable first-hop is unavailable until installed separately",
        visibility::ROUTER_MANAGED_IMPLICIT_EXCEPTION
    ));
    Ok(DurableExecutorReport {
        present: false,
        skill_dir: None,
        message: format!(
            "{} missing from managed roots",
            visibility::ROUTER_MANAGED_IMPLICIT_EXCEPTION
        ),
        warnings,
    })
}

fn set_enabled(enabled: bool, options: RouterModeOptions) -> Result<RouterModeReport> {
    let mut config = read_config_optional()?.ok_or_else(|| Error::InvalidInput {
        message:
            "router enable/disable requires an existing router config; run router install first"
                .to_owned(),
    })?;
    let config_path = config_path()?;
    let router_skill_dirs = configured_router_skill_dirs(&config, &config.router_name);
    if router_skill_dirs.is_empty() {
        return Err(Error::InvalidInput {
            message: "router enable/disable requires router config to locate managed router skills"
                .to_owned(),
        });
    }
    let router_skill_reports = install_router_skills(
        &router_skill_dirs,
        &config.router_name,
        &config.index,
        options.dry_run,
    )?;
    let durable_executor = inspect_durable_executor(&config.roots)?;
    let visibility = apply_router_visibility(
        &config.roots,
        &config.router_name,
        enabled,
        config.manifest.clone(),
        options.dry_run,
    )?;
    let (index_report, preparedness) = if enabled && !options.dry_run {
        let report = router::index(router::IndexOptions {
            roots: config.roots.clone(),
            out: config.index.clone(),
            visibility_manifest: Some(config.manifest.clone()),
        })?;
        let preparedness = check_preparedness(
            &config.roots,
            &config.index,
            &config.manifest,
            &visibility,
            &report,
        )?;
        (Some(report), Some(preparedness))
    } else {
        (None, None)
    };
    if !options.dry_run {
        config.enabled = enabled;
        write_config(
            &config_path,
            &config.roots,
            &router_skill_dirs,
            &config.index,
            &config.manifest,
            &config.router_name,
            config.enabled,
        )?;
    }
    let harness_hooks =
        hooks::apply_harness_hooks_for_config(&config_path, &config, enabled, options.dry_run)?;
    Ok(RouterModeReport {
        router_name: config.router_name,
        router_skill_dirs,
        index: config.index,
        manifest: config.manifest,
        config: config_path,
        enabled,
        dry_run: options.dry_run,
        router_skill_reports,
        durable_executor,
        visibility,
        index_report,
        preparedness,
        harness_hooks,
        restart_warning: restart_warning(),
    })
}

fn apply_router_visibility(
    roots: &[PathBuf],
    router_name: &str,
    enabled: bool,
    manifest: PathBuf,
    dry_run: bool,
) -> Result<VisibilityApplyReport> {
    visibility::apply_router_mode(visibility::RouterModeVisibilityOptions {
        roots: roots.to_vec(),
        router_name: router_name.to_owned(),
        durable_enabled: durable_lifecycle::is_enabled_for_router()?,
        enabled,
        manifest,
        dry_run,
    })
}

fn configured_router_skill_dirs(config: &RouterConfig, router_name: &str) -> Vec<PathBuf> {
    if !config.router_skill_dirs.is_empty() {
        return config.router_skill_dirs.clone();
    }
    let mut dirs = Vec::new();
    if let Some(root) = &config.legacy_router_root {
        dirs.push(root.join(router_name));
    }
    for root in &config.roots {
        let dir = root.join(router_name);
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

fn write_config(
    path: &Path,
    roots: &[PathBuf],
    router_skill_dirs: &[PathBuf],
    index: &Path,
    manifest: &Path,
    router_name: &str,
    enabled: bool,
) -> Result<()> {
    let config = build_config(
        roots,
        router_skill_dirs,
        index,
        manifest,
        router_name,
        enabled,
    );
    let json = serde_json::to_string_pretty(&config).map_err(Error::RenderJson)?;
    write_file(path, &format!("{json}\n"))
}

fn build_config(
    roots: &[PathBuf],
    router_skill_dirs: &[PathBuf],
    index: &Path,
    manifest: &Path,
    router_name: &str,
    enabled: bool,
) -> RouterConfig {
    RouterConfig {
        schema: CONFIG_SCHEMA.to_owned(),
        created_at_unix: now_unix(),
        enabled,
        roots: roots.to_vec(),
        router_skill_dirs: router_skill_dirs.to_vec(),
        index: index.to_path_buf(),
        manifest: manifest.to_path_buf(),
        legacy_router_root: None,
        router_name: router_name.to_owned(),
    }
}

fn default_enabled() -> bool {
    true
}

fn read_config_optional() -> Result<Option<RouterConfig>> {
    let path = config_path()?;
    read_config_optional_at(&path)
}

fn read_config_optional_at(path: &Path) -> Result<Option<RouterConfig>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config: RouterConfig = serde_json::from_str(&text).map_err(|source| Error::ParseJson {
        path: path.to_path_buf(),
        source,
    })?;
    if config.schema != CONFIG_SCHEMA {
        return Err(Error::InvalidInput {
            message: format!(
                "unsupported router config schema {:?}; expected {CONFIG_SCHEMA}",
                config.schema
            ),
        });
    }
    Ok(Some(config))
}

fn config_path() -> Result<PathBuf> {
    Ok(skillspec_home()?.join("router/config.json"))
}

fn skillspec_home() -> Result<PathBuf> {
    if let Some(path) = env::var_os("SKILLSPEC_HOME") {
        return Ok(PathBuf::from(path));
    }
    let Some(home) = env::var_os("HOME") else {
        return Err(Error::InvalidInput {
            message: "HOME is not set; set SKILLSPEC_HOME or HOME".to_owned(),
        });
    };
    Ok(PathBuf::from(home).join(".skillspec"))
}

fn default_manifest_for_index(index: &Path) -> PathBuf {
    index
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("visibility-manifest.json")
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
