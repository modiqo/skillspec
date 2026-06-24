use crate::error::{Error, Result};
use crate::install::{self, HarnessTarget, InstallReport};
use crate::router_lifecycle::{self, RouterHookReport};
use crate::visibility;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG_SCHEMA: &str = "skillspec/durable-executor-config/v1";
const DURABLE_MARKER: &str = ".skillspec-durable-executor-managed";
const DURABLE_NAME: &str = visibility::ROUTER_MANAGED_IMPLICIT_EXCEPTION;

#[derive(Clone, Debug)]
pub struct DurableInstallOptions {
    pub source: PathBuf,
    pub targets: Vec<HarnessTarget>,
    pub all_detected: bool,
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Clone, Debug)]
pub struct DurableUpdateOptions {
    pub source: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct DurableDeleteOptions {
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DurableInstallReport {
    pub skill_name: String,
    pub source: PathBuf,
    pub config: PathBuf,
    pub dry_run: bool,
    pub install: InstallReport,
    pub managed_installs: Vec<DurableSkillReport>,
    pub router_hook: Option<RouterHookReport>,
    pub restart_warning: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DurableUpdateReport {
    pub skill_name: String,
    pub source: PathBuf,
    pub config: PathBuf,
    pub dry_run: bool,
    pub backup: Option<DurableBackupReport>,
    pub managed_installs: Vec<DurableSkillReport>,
    pub router_hook: Option<RouterHookReport>,
    pub restart_warning: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DurableDeleteReport {
    pub skill_name: String,
    pub config: PathBuf,
    pub dry_run: bool,
    pub managed_installs: Vec<DurableSkillReport>,
    pub config_removed: bool,
    pub router_hook: Option<RouterHookReport>,
    pub restart_warning: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DurableSkillReport {
    pub path: PathBuf,
    pub status: DurableFileStatus,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DurableFileStatus {
    Planned,
    Installed,
    Updated,
    Removed,
    Missing,
}

#[derive(Clone, Debug, Serialize)]
pub struct DurableBackupReport {
    pub path: PathBuf,
    pub items: Vec<DurableBackupItem>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DurableBackupItem {
    pub kind: &'static str,
    pub source: PathBuf,
    pub backup: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DurableConfig {
    schema: String,
    created_at_unix: u64,
    source: PathBuf,
    install_dirs: Vec<PathBuf>,
}

pub fn install(options: DurableInstallOptions) -> Result<DurableInstallReport> {
    let source = canonical_source(&options.source)?;
    let install = install::install_skill_without_router_hook(
        &source,
        &options.targets,
        options.all_detected,
        options.dry_run,
        options.force,
        Some(DURABLE_NAME),
    )?;
    let config = config_path()?;
    let install_dirs = install
        .installs
        .iter()
        .map(|target| target.path.clone())
        .collect::<Vec<_>>();
    let managed_installs = install_dirs
        .iter()
        .map(|path| DurableSkillReport {
            path: path.clone(),
            status: if options.dry_run {
                DurableFileStatus::Planned
            } else {
                DurableFileStatus::Installed
            },
        })
        .collect::<Vec<_>>();
    let router_hook = if options.dry_run {
        None
    } else {
        for install_dir in &install_dirs {
            write_marker(install_dir)?;
        }
        write_config(&config, &source, &install_dirs)?;
        router_lifecycle::after_skill_install()?
    };

    Ok(DurableInstallReport {
        skill_name: DURABLE_NAME.to_owned(),
        source,
        config,
        dry_run: options.dry_run,
        install,
        managed_installs,
        router_hook,
        restart_warning: restart_warning(),
    })
}

pub fn update(options: DurableUpdateOptions) -> Result<DurableUpdateReport> {
    let config_path = config_path()?;
    let config = read_config_optional()?.ok_or_else(|| Error::InvalidInput {
        message: "durable-executor update requires durable-executor config; run durable-executor install first".to_owned(),
    })?;
    if config.install_dirs.is_empty() {
        return Err(Error::InvalidInput {
            message:
                "durable-executor update requires durable-executor config with managed install dirs"
                    .to_owned(),
        });
    }
    let source = canonical_source(options.source.as_ref().unwrap_or(&config.source))?;
    if !options.dry_run {
        for install_dir in &config.install_dirs {
            ensure_update_allowed(install_dir)?;
        }
    }
    let backup = if options.dry_run {
        None
    } else {
        Some(create_backup(
            options.backup_dir,
            &config_path,
            &config.install_dirs,
        )?)
    };
    let mut managed_installs = Vec::new();
    for install_dir in &config.install_dirs {
        if !options.dry_run {
            install::sync_skill_package(&source, install_dir)?;
            write_marker(install_dir)?;
        }
        managed_installs.push(DurableSkillReport {
            path: install_dir.clone(),
            status: if options.dry_run {
                DurableFileStatus::Planned
            } else {
                DurableFileStatus::Updated
            },
        });
    }
    let router_hook = if options.dry_run {
        None
    } else {
        write_config(&config_path, &source, &config.install_dirs)?;
        router_lifecycle::after_skill_install()?
    };

    Ok(DurableUpdateReport {
        skill_name: DURABLE_NAME.to_owned(),
        source,
        config: config_path,
        dry_run: options.dry_run,
        backup,
        managed_installs,
        router_hook,
        restart_warning: restart_warning(),
    })
}

pub fn delete(options: DurableDeleteOptions) -> Result<DurableDeleteReport> {
    let config_path = config_path()?;
    let config = read_config_optional()?.ok_or_else(|| Error::InvalidInput {
        message:
            "durable-executor delete requires durable-executor config to locate managed installs"
                .to_owned(),
    })?;
    let mut managed_installs = Vec::new();
    for install_dir in &config.install_dirs {
        let status = remove_managed_install(install_dir, options.dry_run)?;
        managed_installs.push(DurableSkillReport {
            path: install_dir.clone(),
            status,
        });
    }
    let mut config_removed = false;
    let router_hook = if options.dry_run {
        None
    } else {
        let hook = router_lifecycle::after_skill_install()?;
        if config_path.exists() {
            fs::remove_file(&config_path).map_err(|source| Error::Write {
                path: config_path.clone(),
                source,
            })?;
            config_removed = true;
        }
        hook
    };

    Ok(DurableDeleteReport {
        skill_name: DURABLE_NAME.to_owned(),
        config: config_path,
        dry_run: options.dry_run,
        managed_installs,
        config_removed,
        router_hook,
        restart_warning: restart_warning(),
    })
}

pub fn render_install(report: &DurableInstallReport) -> String {
    let mut output = String::new();
    output.push_str("Durable-executor install\n\n");
    output.push_str(&format!("Source: {}\n", report.source.display()));
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str("Managed installs:\n");
    for install in &report.managed_installs {
        output.push_str(&format!(
            "- {}: {:?}\n",
            install.path.display(),
            install.status
        ));
    }
    if report.router_hook.is_some() {
        output.push_str("Router hook: refreshed visibility and index\n");
    }
    output.push_str(&format!("Restart warning: {}\n", report.restart_warning));
    output
}

pub fn render_update(report: &DurableUpdateReport) -> String {
    let mut output = String::new();
    output.push_str("Durable-executor update\n\n");
    output.push_str(&format!("Source: {}\n", report.source.display()));
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    if let Some(backup) = &report.backup {
        output.push_str(&format!("Backup: {}\n", backup.path.display()));
    }
    output.push_str("Managed installs:\n");
    for install in &report.managed_installs {
        output.push_str(&format!(
            "- {}: {:?}\n",
            install.path.display(),
            install.status
        ));
    }
    if report.router_hook.is_some() {
        output.push_str("Router hook: refreshed visibility and index\n");
    }
    output.push_str(&format!("Restart warning: {}\n", report.restart_warning));
    output
}

pub fn render_delete(report: &DurableDeleteReport) -> String {
    let mut output = String::new();
    output.push_str("Durable-executor delete\n\n");
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str("Managed installs:\n");
    for install in &report.managed_installs {
        output.push_str(&format!(
            "- {}: {:?}\n",
            install.path.display(),
            install.status
        ));
    }
    output.push_str(&format!("Config removed: {}\n", report.config_removed));
    if report.router_hook.is_some() {
        output.push_str("Router hook: refreshed visibility and index\n");
    }
    output.push_str(&format!("Restart warning: {}\n", report.restart_warning));
    output
}

fn canonical_source(source: &Path) -> Result<PathBuf> {
    if !source.is_dir() {
        return Err(Error::InvalidInput {
            message: format!(
                "{} is not a durable-executor source folder",
                source.display()
            ),
        });
    }
    let skill = source.join("SKILL.md");
    let spec = source.join("skill.spec.yml");
    if !skill.is_file() || !spec.is_file() {
        return Err(Error::InvalidInput {
            message: format!(
                "durable-executor source must contain SKILL.md and skill.spec.yml: {}",
                source.display()
            ),
        });
    }
    let skill_text = fs::read_to_string(&skill).map_err(|source| Error::Read {
        path: skill.clone(),
        source,
    })?;
    if !frontmatter_names_durable_executor(&skill_text) {
        return Err(Error::InvalidInput {
            message: format!(
                "durable-executor source SKILL.md frontmatter must declare name: {DURABLE_NAME}"
            ),
        });
    }
    source.canonicalize().map_err(|source_error| Error::Read {
        path: source.to_path_buf(),
        source: source_error,
    })
}

fn frontmatter_names_durable_executor(text: &str) -> bool {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return false;
    }
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            return false;
        }
        if trimmed == format!("name: {DURABLE_NAME}")
            || trimmed == format!("name: \"{DURABLE_NAME}\"")
        {
            return true;
        }
    }
    false
}

fn remove_managed_install(install_dir: &Path, dry_run: bool) -> Result<DurableFileStatus> {
    if !install_dir.exists() {
        return Ok(DurableFileStatus::Missing);
    }
    let marker = install_dir.join(DURABLE_MARKER);
    if !marker.is_file() {
        return Err(Error::InvalidInput {
            message: format!(
                "refusing to remove durable-executor without managed marker: {}",
                install_dir.display()
            ),
        });
    }
    if !dry_run {
        fs::remove_dir_all(install_dir).map_err(|source| Error::Write {
            path: install_dir.to_path_buf(),
            source,
        })?;
    }
    Ok(DurableFileStatus::Removed)
}

fn ensure_update_allowed(install_dir: &Path) -> Result<()> {
    if !install_dir.exists() {
        return Ok(());
    }
    let marker = install_dir.join(DURABLE_MARKER);
    if marker.is_file() {
        return Ok(());
    }
    Err(Error::InvalidInput {
        message: format!(
            "refusing to update durable-executor without managed marker: {}",
            install_dir.display()
        ),
    })
}

fn write_marker(install_dir: &Path) -> Result<()> {
    write_file(
        &install_dir.join(DURABLE_MARKER),
        &format!(
            "schema: skillspec/durable-executor-managed/v1\ncreated_at_unix: {}\n",
            now_unix()
        ),
    )
}

fn write_config(path: &Path, source: &Path, install_dirs: &[PathBuf]) -> Result<()> {
    let config = DurableConfig {
        schema: CONFIG_SCHEMA.to_owned(),
        created_at_unix: now_unix(),
        source: source.to_path_buf(),
        install_dirs: install_dirs.to_vec(),
    };
    let json = serde_json::to_string_pretty(&config).map_err(Error::RenderJson)?;
    write_file(path, &format!("{json}\n"))
}

fn read_config_optional() -> Result<Option<DurableConfig>> {
    let path = config_path()?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })?;
    let config: DurableConfig = serde_json::from_str(&text).map_err(|source| Error::ParseJson {
        path: path.clone(),
        source,
    })?;
    if config.schema != CONFIG_SCHEMA {
        return Err(Error::InvalidInput {
            message: format!(
                "unsupported durable-executor config schema {:?}; expected {CONFIG_SCHEMA}",
                config.schema
            ),
        });
    }
    Ok(Some(config))
}

fn create_backup(
    backup_dir: Option<PathBuf>,
    config_path: &Path,
    install_dirs: &[PathBuf],
) -> Result<DurableBackupReport> {
    let backup_root = backup_dir.unwrap_or_else(|| default_update_backup_dir(config_path));
    if backup_root.exists() {
        return Err(Error::InvalidInput {
            message: format!(
                "durable-executor update backup directory already exists: {}",
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
    for (index, install_dir) in install_dirs.iter().enumerate() {
        if install_dir.exists() {
            let destination = backup_root.join(format!("durable-executor-{index}"));
            copy_dir_recursive(install_dir, &destination)?;
            items.push(DurableBackupItem {
                kind: "durable_executor_dir",
                source: install_dir.clone(),
                backup: destination,
            });
        }
    }
    let report = DurableBackupReport {
        path: backup_root.clone(),
        items,
    };
    let backup_manifest = backup_root.join("backup.json");
    let json = serde_json::to_string_pretty(&report).map_err(Error::RenderJson)?;
    write_file(&backup_manifest, &format!("{json}\n"))?;
    Ok(report)
}

fn backup_file_if_present(
    kind: &'static str,
    source: &Path,
    backup: &Path,
    items: &mut Vec<DurableBackupItem>,
) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::copy(source, backup).map_err(|source_error| Error::Write {
        path: backup.to_path_buf(),
        source: source_error,
    })?;
    items.push(DurableBackupItem {
        kind,
        source: source.to_path_buf(),
        backup: backup.to_path_buf(),
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
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|source_error| Error::Write {
                path: destination_path,
                source: source_error,
            })?;
        }
    }
    Ok(())
}

fn config_path() -> Result<PathBuf> {
    Ok(skillspec_home()?.join("durable-executor/config.json"))
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

fn default_update_backup_dir(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
        .join(format!("update-{}", now_unix()))
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
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn restart_warning() -> String {
    "restart Codex, Claude, or other harness sessions that cache skill metadata before testing changes"
        .to_owned()
}
