use crate::error::{Error, Result};
use crate::router::{self, IndexReport};
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
    pub router_root: Option<PathBuf>,
    pub router_name: Option<String>,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct RouterUninstallOptions {
    pub manifest: Option<PathBuf>,
    pub router_root: Option<PathBuf>,
    pub router_name: Option<String>,
    pub index: Option<PathBuf>,
    pub keep_index: bool,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterInstallReport {
    pub router_name: String,
    pub router_root: PathBuf,
    pub router_skill_dir: PathBuf,
    pub index: PathBuf,
    pub manifest: PathBuf,
    pub config: PathBuf,
    pub dry_run: bool,
    pub router_skill_status: RouterFileStatus,
    pub visibility: VisibilityApplyReport,
    pub index_report: Option<IndexReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterUninstallReport {
    pub router_name: String,
    pub router_root: PathBuf,
    pub router_skill_dir: PathBuf,
    pub manifest: PathBuf,
    pub index: Option<PathBuf>,
    pub config: PathBuf,
    pub dry_run: bool,
    pub router_skill_status: RouterFileStatus,
    pub index_removed: bool,
    pub config_removed: bool,
    pub restore: VisibilityRestoreReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterHookReport {
    pub config: PathBuf,
    pub visibility: VisibilityApplyReport,
    pub index_report: IndexReport,
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
    index: PathBuf,
    manifest: PathBuf,
    router_root: PathBuf,
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
    let router_root = options
        .router_root
        .clone()
        .unwrap_or_else(|| options.roots[0].clone());
    let router_skill_dir = router_root.join(&router_name);
    let manifest = options
        .manifest
        .clone()
        .unwrap_or_else(|| default_manifest_for_index(&index));
    let config = config_path()?;

    if !options.dry_run {
        write_router_skill(&router_skill_dir, &router_name, &index)?;
    }
    let visibility = visibility::apply(visibility::VisibilityApplyOptions {
        roots: options.roots.clone(),
        profile: visibility::VisibilityProfile::RouterManaged,
        manifest: manifest.clone(),
        dry_run: options.dry_run,
    })?;
    let index_report = if options.dry_run {
        None
    } else {
        let report = router::index(router::IndexOptions {
            roots: options.roots.clone(),
            out: index.clone(),
            visibility_manifest: Some(manifest.clone()),
        })?;
        write_config(
            &config,
            &options.roots,
            &index,
            &manifest,
            &router_root,
            &router_name,
        )?;
        Some(report)
    };

    Ok(RouterInstallReport {
        router_name,
        router_root,
        router_skill_dir,
        index,
        manifest,
        config,
        dry_run: options.dry_run,
        router_skill_status: if options.dry_run {
            RouterFileStatus::Planned
        } else {
            RouterFileStatus::Installed
        },
        visibility,
        index_report,
    })
}

pub fn uninstall(options: RouterUninstallOptions) -> Result<RouterUninstallReport> {
    let config = read_config_optional()?;
    let config_path = config_path()?;
    let router_name = options
        .router_name
        .or_else(|| config.as_ref().map(|config| config.router_name.clone()))
        .unwrap_or_else(|| DEFAULT_ROUTER_NAME.to_owned());
    let router_root = options
        .router_root
        .or_else(|| config.as_ref().map(|config| config.router_root.clone()))
        .ok_or_else(|| Error::InvalidInput {
            message: "router uninstall requires --router-root when no router config exists"
                .to_owned(),
        })?;
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
    let router_skill_dir = router_root.join(&router_name);

    let restore = visibility::restore(visibility::VisibilityRestoreOptions {
        manifest: manifest.clone(),
        dry_run: options.dry_run,
    })?;

    let router_skill_status = remove_router_skill(&router_skill_dir, options.dry_run)?;
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
        router_root,
        router_skill_dir,
        manifest,
        index,
        config: config_path,
        dry_run: options.dry_run,
        router_skill_status,
        index_removed,
        config_removed,
        restore,
    })
}

pub fn after_skill_install() -> Result<Option<RouterHookReport>> {
    let Some(config) = read_config_optional()? else {
        return Ok(None);
    };
    let visibility = visibility::apply(visibility::VisibilityApplyOptions {
        roots: config.roots.clone(),
        profile: visibility::VisibilityProfile::RouterManaged,
        manifest: config.manifest.clone(),
        dry_run: false,
    })?;
    let index_report = router::index(router::IndexOptions {
        roots: config.roots,
        out: config.index,
        visibility_manifest: Some(config.manifest),
    })?;
    Ok(Some(RouterHookReport {
        config: config_path()?,
        visibility,
        index_report,
    }))
}

pub fn render_install(report: &RouterInstallReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router install\n\n");
    output.push_str(&format!("Router: {}\n", report.router_skill_dir.display()));
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
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
    output
}

pub fn render_uninstall(report: &RouterUninstallReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router uninstall\n\n");
    output.push_str(&format!("Router: {}\n", report.router_skill_dir.display()));
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

fn write_router_skill(skill_dir: &Path, router_name: &str, index: &Path) -> Result<()> {
    fs::create_dir_all(skill_dir).map_err(|source| Error::Write {
        path: skill_dir.to_path_buf(),
        source,
    })?;
    let skill = format!(
        r#"---
name: {router_name}
description: Use when selecting the right local skill from a large SkillSpec-indexed skill library, especially when many skills are installed or descriptions may be truncated.
---

# Skill Router

Use this skill as the visible discovery surface for large local skill libraries.

1. Run `skillspec route --index {index} --query "<user task>" --top 5 --json`.
2. If the selected candidate has high confidence, read that skill's `SKILL.md` and follow it.
3. If confidence is medium, compare the top candidates briefly and choose the best fit.
4. If confidence is low, ask the user to choose a skill or continue without a skill.
5. If route output includes `execution_mode_direct_or_durable`, ask the user whether to run direct or durable before tool-backed execution.

Index arguments may use either the SQLite file or the router directory; directory paths resolve to `skill-index.sqlite`.
"#,
        index = index.display()
    );
    write_file(&skill_dir.join("SKILL.md"), &skill)?;
    write_file(
        &skill_dir.join(ROUTER_MARKER),
        &format!(
            "schema: skillspec/router-managed/v1\ncreated_at_unix: {}\n",
            now_unix()
        ),
    )
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

fn write_config(
    path: &Path,
    roots: &[PathBuf],
    index: &Path,
    manifest: &Path,
    router_root: &Path,
    router_name: &str,
) -> Result<()> {
    let config = RouterConfig {
        schema: CONFIG_SCHEMA.to_owned(),
        created_at_unix: now_unix(),
        roots: roots.to_vec(),
        index: index.to_path_buf(),
        manifest: manifest.to_path_buf(),
        router_root: router_root.to_path_buf(),
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
