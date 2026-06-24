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
pub struct RouterRefreshOptions {
    pub roots: Vec<PathBuf>,
    pub index: PathBuf,
    pub visibility_manifest: Option<PathBuf>,
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
    pub restart_warning: String,
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
pub struct RouterHookReport {
    pub config: PathBuf,
    pub router_skill_reports: Vec<RouterSkillReport>,
    pub visibility: VisibilityApplyReport,
    pub index_report: IndexReport,
    pub preparedness: RouterPreparednessReport,
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
    let visibility = visibility::apply(visibility::VisibilityApplyOptions {
        roots: options.roots.clone(),
        profile: visibility::VisibilityProfile::RouterManaged,
        manifest: manifest.clone(),
        dry_run: options.dry_run,
    })?;
    let (index_report, preparedness) = if options.dry_run {
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
        )?;
        (Some(report), preparedness)
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
    let visibility = visibility::apply(visibility::VisibilityApplyOptions {
        roots: config.roots.clone(),
        profile: visibility::VisibilityProfile::RouterManaged,
        manifest: config.manifest.clone(),
        dry_run: options.dry_run,
    })?;
    let (index_report, preparedness) = if options.dry_run {
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
        )?;
        (Some(report), Some(preparedness))
    };

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
        restart_warning: restart_warning(),
    })
}

pub fn after_skill_install() -> Result<Option<RouterHookReport>> {
    let Some(config) = read_config_optional()? else {
        return Ok(None);
    };
    let router_skill_reports = install_router_skills(
        &configured_router_skill_dirs(&config, &config.router_name),
        &config.router_name,
        &config.index,
        false,
    )?;
    let visibility = visibility::apply(visibility::VisibilityApplyOptions {
        roots: config.roots.clone(),
        profile: visibility::VisibilityProfile::RouterManaged,
        manifest: config.manifest.clone(),
        dry_run: false,
    })?;
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

    let visibility = if router_config_present {
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
        Some(visibility::apply(visibility::VisibilityApplyOptions {
            roots: options.roots.clone(),
            profile: visibility::VisibilityProfile::RouterManaged,
            manifest,
            dry_run: false,
        })?)
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

pub fn render_install(report: &RouterInstallReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router install\n\n");
    output.push_str(&format!("Router: {}\n", report.router_skill_dir.display()));
    if report.router_skill_dirs.len() > 1 {
        output.push_str("Router roots:\n");
        for router_skill in &report.router_skill_reports {
            output.push_str(&format!(
                "- {} ({:?})\n",
                router_skill.path.display(),
                router_skill.status
            ));
        }
    }
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str(&format!(
        "Durable executor: {}\n",
        report.durable_executor.message
    ));
    if !report.durable_executor.warnings.is_empty() {
        output.push_str("Durable warnings:\n");
        for warning in &report.durable_executor.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output.push_str(&format!(
        "Visibility changes: {}\n",
        report.visibility.changes.len()
    ));
    if let Some(index_report) = &report.index_report {
        output.push_str(&format!(
            "Skills indexed: {}\n",
            index_report.skills_indexed
        ));
    }
    output.push_str(&format!("Prepared: {}\n", report.preparedness.ready));
    output.push_str(&format!(
        "Index stale after build: {}\n",
        report.preparedness.index_stale
    ));
    output
}

pub fn render_refresh(report: &RouterRefreshReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router index refresh\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!(
        "Router config present: {}\n",
        report.router_config_present
    ));
    if let Some(config) = &report.config {
        output.push_str(&format!("Config: {}\n", config.display()));
    }
    if let Some(manifest) = &report.visibility_manifest {
        output.push_str(&format!("Manifest: {}\n", manifest.display()));
    }
    output.push_str(&format!(
        "Detected stale before refresh: {}\n",
        report.status_before.stale
    ));
    output.push_str(&format!(
        "Visibility changes: {}\n",
        report
            .visibility
            .as_ref()
            .map(|visibility| visibility.changes.len())
            .unwrap_or(0)
    ));
    output.push_str(&format!(
        "Skills indexed: {}\n",
        report.index_report.skills_indexed
    ));
    if let Some(preparedness) = &report.preparedness {
        output.push_str(&format!("Prepared: {}\n", preparedness.ready));
        output.push_str(&format!(
            "Index stale after build: {}\n",
            preparedness.index_stale
        ));
    }
    if !report.advice.is_empty() {
        output.push_str("\nAdvice:\n");
        for advice in &report.advice {
            output.push_str(&format!("- {advice}\n"));
        }
    }
    output
}

pub fn render_update(report: &RouterUpdateReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router update\n\n");
    output.push_str("Router roots:\n");
    for router_skill in &report.router_skill_reports {
        output.push_str(&format!(
            "- {} ({:?})\n",
            router_skill.path.display(),
            router_skill.status
        ));
    }
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    if let Some(backup) = &report.backup {
        output.push_str(&format!("Backup: {}\n", backup.path.display()));
        output.push_str(&format!("Backup items: {}\n", backup.items.len()));
    }
    output.push_str(&format!(
        "Durable executor: {}\n",
        report.durable_executor.message
    ));
    if let Some(index_report) = &report.index_report {
        output.push_str(&format!(
            "Skills indexed: {}\n",
            index_report.skills_indexed
        ));
    }
    if let Some(preparedness) = &report.preparedness {
        output.push_str(&format!("Prepared: {}\n", preparedness.ready));
        output.push_str(&format!(
            "Index stale after update: {}\n",
            preparedness.index_stale
        ));
    }
    output.push_str(&format!("Restart warning: {}\n", report.restart_warning));
    output
}

pub fn render_uninstall(report: &RouterUninstallReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router uninstall\n\n");
    output.push_str(&format!("Router: {}\n", report.router_skill_dir.display()));
    if report.router_skill_dirs.len() > 1 {
        output.push_str("Router roots:\n");
        for router_skill in &report.router_skill_reports {
            output.push_str(&format!(
                "- {} ({:?})\n",
                router_skill.path.display(),
                router_skill.status
            ));
        }
    }
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    if let Some(index) = &report.index {
        output.push_str(&format!("Index: {}\n", index.display()));
    }
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str(&format!(
        "Visibility restores: {}\n",
        report.restore.changes.len()
    ));
    output.push_str(&format!("Index removed: {}\n", report.index_removed));
    output
}

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
    "Restart active Codex, Claude, Agents, or vendor harness sessions so they reload updated router skill files and native visibility metadata.".to_owned()
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
    let router_name_yaml = yaml_single_quote(router_name);
    let skill = format!(
        r#"---
name: {router_name_yaml}
description: Use when selecting the right local skill from a large SkillSpec-indexed skill library, especially when many skills are installed or descriptions may be truncated.
metadata:
  routing:
    tags: [skills, router, discovery, codex, claude]
    triggers:
      - choose the right skill
      - route to a skill
      - many skills installed
      - skill descriptions shortened
      - install skill router
      - disable implicit skill invocation
---

# Skill Router

This is a thin native harness loader generated by `skillspec router install`.
Load and follow `./skill.spec.yml`; that file is the router contract.
This router is explicit-only; `durable-executor` remains the implicit first-hop
when it is installed separately in the managed roots.
"#
    );
    write_file(&skill_dir.join("SKILL.md"), &skill)?;
    write_file(
        &skill_dir.join("skill.spec.yml"),
        &render_router_spec(router_name, index),
    )?;
    write_file(
        &skill_dir.join(ROUTER_MARKER),
        &format!(
            "schema: skillspec/router-managed/v1\ncreated_at_unix: {}\n",
            now_unix()
        ),
    )
}

fn render_router_spec(router_name: &str, index: &Path) -> String {
    let router_skill = yaml_single_quote(router_name);
    let index_path = shell_single_quote(&index.display().to_string());
    let route_command = yaml_single_quote(&format!(
        "skillspec route --index {index_path} --query \"<user task>\" --top 5 --json"
    ));
    let index_status_command = yaml_single_quote(&format!(
        "skillspec router index status --roots <skill-root>... --index {index_path} --visibility-manifest <manifest> --json"
    ));
    let lifecycle_plan_command = yaml_single_quote(&format!(
        "skillspec router install --roots <skill-root>... --index {index_path} --manifest <manifest> --dry-run --json"
    ));

    format!(
        r#"schema: skillspec/v0
id: skill.router
title: Skill Router
description: Route user requests to the best local skill from a large indexed library and manage native visibility controls for Codex and Claude. The installed SKILL.md is only the native loader; this SkillSpec is the router contract.

activation:
  summary: Use for skill discovery, large skill catalogs, shortened skill descriptions, router install/uninstall, index refresh/status, and Codex or Claude implicit invocation controls.
  keywords:
    - skill router
    - route to a skill
    - choose the right skill
    - many skills installed
    - skill descriptions shortened
    - install router
    - uninstall router
    - refresh skill index
    - out-of-band skill
    - prose skill added
    - skill visibility
    - disable implicit invocation
    - allow_implicit_invocation
    - disable-model-invocation
    - skillOverrides
  priority: broad_router

applies_when:
  - user_intent:
      - select a skill from a large local skill library
      - route a request to the best SkillSpec or SKILL.md package
      - reduce native skill discovery context pressure
      - install or uninstall the SkillSpec router
      - make skills explicit-only, manual-only, implicit, or off
      - refresh a skill index after skill additions or removals
      - detect or repair skills added outside the SkillSpec install flow

entry:
  prompt: Load this SkillSpec, route from the local index, then load only the selected skill or ask for direct versus durable execution.
  decision_required: true
  tool_boundary:
    default: deny
    allow:
      - skillspec_cli
      - local_skill_files
      - local_router_index
      - local_visibility_manifest
    permission_required_for:
      - any_unlisted_tool
      - mutating_visibility_files
      - installing_router_skill
      - deleting_router_skill
      - deleting_router_index

routes:
  - id: route_from_index
    label: Route from the skill index
    rank: 10
    description: Use the local SQLite index to choose the best candidate skill for the user request.
    execution_plan:
      mode: ordered
      phases:
        - id: check_index_status
          owner_skill: {router_skill}
          description: Check whether the router index exists, whether it is stale for the configured roots, and whether new or changed skills are prose-only or SkillSpec-backed.
          requires:
            - inspect_router_index_status
        - id: route_query
          owner_skill: {router_skill}
          description: Query the router index and select or present candidate skills.
          requires:
            - run_route_query
        - id: execution_mode_elicitation
          owner_skill: {router_skill}
          description: Ask direct versus durable execution only when route output requests it and the user has not already chosen.
          requires:
            - ask_direct_or_durable_when_needed

  - id: manage_router_lifecycle
    label: Install, update, refresh, or uninstall router
    rank: 20
    description: Install the explicit-only router skill into every managed root, back up and update recorded router installs, apply visibility, build and verify the index, refresh out-of-band additions, or uninstall and restore visibility from the manifest.
    execution_plan:
      mode: ordered
      phases:
        - id: plan_lifecycle_change
          owner_skill: {router_skill}
          description: Show the lifecycle operation, affected roots, index path, manifest path, and restore behavior before mutation.
          requires:
            - show_router_lifecycle_plan
        - id: apply_lifecycle_change
          owner_skill: {router_skill}
          description: Run router install, update, uninstall, index refresh, or index status commands. Install prepares the router; update backs up and rewrites recorded router packages; refresh repairs router-managed visibility and indexes out-of-band prose or SkillSpec-backed additions.
          requires:
            - run_router_lifecycle_command
        - id: verify_lifecycle_change
          owner_skill: {router_skill}
          description: Verify router skill files, manifest, config, preparedness.ready, and index status after the lifecycle change.
          requires:
            - verify_router_lifecycle_result

  - id: manage_visibility
    label: Manage native skill visibility
    rank: 30
    description: Plan, apply, restore, or explicitly set Codex and Claude native visibility controls.
    execution_plan:
      mode: ordered
      phases:
        - id: visibility_plan
          owner_skill: {router_skill}
          description: Preview native Codex and Claude visibility changes before editing files.
          requires:
            - run_visibility_plan
        - id: visibility_apply_or_restore
          owner_skill: {router_skill}
          description: Apply or restore visibility from a reversible manifest.
          requires:
            - run_visibility_apply_or_restore
        - id: visibility_verify
          owner_skill: {router_skill}
          description: Verify native metadata and router manifest state.

rules:
  - id: route_queries_use_index
    when:
      user_says_any:
        - route to a skill
        - choose the right skill
        - find matching skill
        - many skills installed
        - skill descriptions shortened
    prefer: route_from_index
    reason: Discovery should use the local index instead of loading every skill description into context.

  - id: lifecycle_requests_use_router_commands
    when:
      user_says_any:
        - install router
        - uninstall router
        - router index
        - refresh index
        - index status
        - out-of-band skill
        - prose skill added
    prefer: manage_router_lifecycle
    reason: Router lifecycle changes must write a manifest, config, and index through dedicated commands.

  - id: visibility_requests_use_native_controls
    when:
      user_says_any:
        - set visibility
        - explicit-only
        - manual-only
        - name-only
        - implicit invocation
        - disable implicit
        - skillOverrides
        - disable-model-invocation
        - allow_implicit_invocation
    prefer: manage_visibility
    reason: Visibility changes should use Codex and Claude native controls with a manifest-backed restore path.

elicitations:
  execution_mode_direct_or_durable:
    question: Do you want direct execution or durable execution for this selected skill?
    choices:
      - id: direct
        label: Direct
        sets:
          execution_mode: direct
      - id: durable
        label: Durable
        sets:
          execution_mode: durable
    default: direct

commands:
  inspect_router_index_status:
    description: Compare the router index against current skill roots and report advice for out-of-band prose or SkillSpec-backed skills.
    template: {index_status_command}
    safety: local_read

  run_route_query:
    description: Route the user request to candidate skills from the index.
    template: {route_command}
    safety: local_read

  show_router_lifecycle_plan:
    description: Preview router lifecycle changes before writing files.
    template: {lifecycle_plan_command}
    safety: local_read

  run_router_lifecycle_command:
    description: Apply the requested router lifecycle command. Use index refresh to apply explicit invocation controls and rebuild the index after out-of-band skill changes.
    template: 'skillspec router install|uninstall|index refresh|index status'
    safety: local_write

  run_visibility_plan:
    description: Preview native Codex and Claude visibility changes.
    template: 'skillspec visibility plan --roots <skill-root>... --json'
    safety: local_read

  run_visibility_apply_or_restore:
    description: Apply or restore native visibility controls using a manifest.
    template: 'skillspec visibility apply|restore --manifest <manifest>'
    safety: local_write

tests:
  - name: route query uses index
    input: choose the right skill from many skills installed
    expect:
      route: route_from_index
      matched_rules:
        - route_queries_use_index

  - name: lifecycle uses router commands
    input: install router and refresh index
    expect:
      route: manage_router_lifecycle
      matched_rules:
        - lifecycle_requests_use_router_commands

  - name: visibility uses native controls
    input: set a noisy skill to explicit-only with allow_implicit_invocation false
    expect:
      route: manage_visibility
      matched_rules:
        - visibility_requests_use_native_controls
"#
    )
}

fn yaml_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
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
    if let Some(entry) = entries
        .iter()
        .find(|entry| entry.name == visibility::ROUTER_MANAGED_IMPLICIT_EXCEPTION)
    {
        return Ok(DurableExecutorReport {
            present: true,
            skill_dir: Some(entry.skill_dir.clone()),
            message: format!(
                "{} present in managed roots; router-managed visibility keeps it implicit",
                visibility::ROUTER_MANAGED_IMPLICIT_EXCEPTION
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
) -> Result<()> {
    let config = RouterConfig {
        schema: CONFIG_SCHEMA.to_owned(),
        created_at_unix: now_unix(),
        roots: roots.to_vec(),
        router_skill_dirs: router_skill_dirs.to_vec(),
        index: index.to_path_buf(),
        manifest: manifest.to_path_buf(),
        legacy_router_root: None,
        router_name: router_name.to_owned(),
    };
    let json = serde_json::to_string_pretty(&config).map_err(Error::RenderJson)?;
    write_file(path, &format!("{json}\n"))
}

fn read_config_optional() -> Result<Option<RouterConfig>> {
    let path = config_path()?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })?;
    let config: RouterConfig = serde_json::from_str(&text).map_err(|source| Error::ParseJson {
        path: path.clone(),
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
