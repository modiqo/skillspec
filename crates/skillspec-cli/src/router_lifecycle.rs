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

#[derive(Clone, Debug, Serialize)]
pub struct RouterInstallReport {
    pub router_name: String,
    pub router_skill_dir: PathBuf,
    pub index: PathBuf,
    pub manifest: PathBuf,
    pub config: PathBuf,
    pub dry_run: bool,
    pub router_skill_status: RouterFileStatus,
    pub durable_executor: DurableExecutorReport,
    pub visibility: VisibilityApplyReport,
    pub index_report: Option<IndexReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterUninstallReport {
    pub router_name: String,
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

#[derive(Clone, Debug, Serialize)]
pub struct DurableExecutorReport {
    pub present: bool,
    pub skill_dir: Option<PathBuf>,
    pub message: String,
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
    let router_install_root = options.roots[0].clone();
    let router_skill_dir = router_install_root.join(&router_name);
    let manifest = options
        .manifest
        .clone()
        .unwrap_or_else(|| default_manifest_for_index(&index));
    let config = config_path()?;
    let durable_executor = inspect_durable_executor(&options.roots)?;

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
        write_config(&config, &options.roots, &index, &manifest, &router_name)?;
        Some(report)
    };

    Ok(RouterInstallReport {
        router_name,
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
        durable_executor,
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
    validate_router_name(&router_name)?;
    let router_install_root = config
        .as_ref()
        .and_then(configured_router_install_root)
        .ok_or_else(|| Error::InvalidInput {
            message: "router uninstall requires router config to locate the managed router skill"
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
    let router_skill_dir = router_install_root.join(&router_name);

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
          description: Check whether the router index exists and whether it is stale for the configured roots.
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
    label: Install, refresh, or uninstall router
    rank: 20
    description: Install the explicit-only router skill into the first managed root, manage its index and manifest, or uninstall and restore visibility from the manifest.
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
          description: Run router install, uninstall, index refresh, or index status commands.
          requires:
            - run_router_lifecycle_command
        - id: verify_lifecycle_change
          owner_skill: {router_skill}
          description: Verify router skill files, manifest, config, and index status after the lifecycle change.
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
    description: Compare the router index against current skill roots.
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
    description: Apply the requested router lifecycle command.
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

fn configured_router_install_root(config: &RouterConfig) -> Option<PathBuf> {
    config
        .legacy_router_root
        .clone()
        .or_else(|| config.roots.first().cloned())
}

fn write_config(
    path: &Path,
    roots: &[PathBuf],
    index: &Path,
    manifest: &Path,
    router_name: &str,
) -> Result<()> {
    let config = RouterConfig {
        schema: CONFIG_SCHEMA.to_owned(),
        created_at_unix: now_unix(),
        roots: roots.to_vec(),
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
