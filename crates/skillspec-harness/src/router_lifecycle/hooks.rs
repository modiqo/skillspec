use super::*;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;

const MANAGED_HOOK_MATCH: &str = "skillspec router guard";

pub(super) fn guard(options: RouterGuardOptions) -> Result<RouterGuardReport> {
    let config_path = options.config.unwrap_or(config_path()?);
    let Some(config) = read_config_optional_at(&config_path)? else {
        return Ok(inactive_guard_report(
            config_path,
            false,
            options.current_harness,
            "router config is missing; run `skillspec router install` to enable router-first mode",
            "skillspec router install --roots <skill-roots> --index <router-index>",
        ));
    };

    if !config.enabled {
        return Ok(inactive_guard_report(
            config_path,
            true,
            options.current_harness,
            "router config is installed but disabled; run `skillspec router enable` to restore router-first mode",
            "skillspec router enable --json",
        ));
    }

    let router_skill_dirs = router_skill_statuses(&config);
    let mut warnings = router_skill_warnings(&router_skill_dirs);
    let mut repaired = false;
    let status_before = router::index_status(router::IndexStatusOptions {
        roots: config.roots.clone(),
        index: config.index.clone(),
        visibility_manifest: Some(config.manifest.clone()),
    })?;

    let (visibility, index_report) = if status_before.stale || !status_before.exists {
        repaired = true;
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
        (Some(visibility), Some(index_report))
    } else {
        (None, None)
    };

    let status_after = router::index_status(router::IndexStatusOptions {
        roots: config.roots.clone(),
        index: config.index.clone(),
        visibility_manifest: Some(config.manifest.clone()),
    })?;
    warnings.extend(status_after.warnings.iter().cloned());

    let preparedness = match (&visibility, &index_report) {
        (Some(visibility), Some(index_report)) => {
            let report = preparedness_from_status(visibility, index_report, &status_after);
            warnings.extend(report.warnings.iter().cloned());
            Some(report)
        }
        _ => None,
    };
    let router_skills_ready = router_skill_dirs.iter().all(|status| {
        status.present && status.managed && status.has_skill_md && status.has_skill_spec
    });
    let index_ready = status_after.exists
        && !status_after.stale
        && status_after.indexed_skills == status_after.discovered_skills;
    let first_hop_ready = router_skills_ready && index_ready;
    let repair_command = repair_command_for_config(&config);
    let message = if first_hop_ready {
        if repaired {
            "router guard repaired visibility/index drift; router-first is ready".to_owned()
        } else {
            "router guard verified router-first readiness".to_owned()
        }
    } else {
        format!(
            "router guard could not prove router-first readiness; repair with `{repair_command}`"
        )
    };

    Ok(RouterGuardReport {
        config: config_path,
        installed: true,
        enabled: true,
        current_harness: options.current_harness,
        repaired,
        first_hop_ready,
        router_skill_dirs,
        status_before: Some(status_before),
        status_after: Some(status_after),
        visibility,
        index_report,
        preparedness,
        repair_command,
        message,
        warnings: unique_strings(warnings),
    })
}

pub(super) fn apply_harness_hooks_for_config(
    config_path: &Path,
    config: &RouterConfig,
    enabled: bool,
    dry_run: bool,
) -> Result<Vec<RouterHarnessHookReport>> {
    let mut reports = Vec::new();
    for target in hook_targets_for_roots(&config.roots) {
        let command = managed_hook_command(config_path, target.harness);
        let report = if enabled {
            install_hook(&target, &command, dry_run)?
        } else {
            remove_hook(&target, &command, dry_run)?
        };
        reports.push(report);
    }
    Ok(reports)
}

pub(super) fn inspect_harness_hooks_for_config(
    config_path: &Path,
    config: &RouterConfig,
) -> Vec<RouterHarnessHookReport> {
    hook_targets_for_roots(&config.roots)
        .into_iter()
        .map(|target| {
            let command = managed_hook_command(config_path, target.harness);
            inspect_hook(&target, &command)
        })
        .collect()
}

pub(super) fn hook_json_for_guard(report: &RouterGuardReport) -> Value {
    if report.first_hop_ready {
        let mut context =
            "SkillSpec router guard: first_hop_ready=true. Use skill-router as the first hop before loading domain skills.".to_owned();
        if let Some(harness) = report.current_harness {
            context.push_str(&format!(
                " Current harness: {}; pass `--current-harness {}` to `skillspec route` for duplicate-root physical selection.",
                harness.as_str(),
                harness.as_str()
            ));
        }
        if report.repaired {
            context.push_str(" The guard repaired router visibility/index drift before this turn.");
        }
        return json!({
            "hookSpecificOutput": {
                "hookEventName": "UserPromptSubmit",
                "additionalContext": context
            }
        });
    }

    json!({
        "decision": "block",
        "reason": format!("SkillSpec router guard blocked this turn: {}. Repair: {}", report.message, report.repair_command)
    })
}

fn inactive_guard_report(
    config: PathBuf,
    installed: bool,
    current_harness: Option<router::RouteHarness>,
    message: &str,
    repair_command: &str,
) -> RouterGuardReport {
    RouterGuardReport {
        config,
        installed,
        enabled: false,
        current_harness,
        repaired: false,
        first_hop_ready: false,
        router_skill_dirs: Vec::new(),
        status_before: None,
        status_after: None,
        visibility: None,
        index_report: None,
        preparedness: None,
        repair_command: repair_command.to_owned(),
        message: message.to_owned(),
        warnings: Vec::new(),
    }
}

fn router_skill_statuses(config: &RouterConfig) -> Vec<RouterSkillInstallStatus> {
    configured_router_skill_dirs(config, &config.router_name)
        .into_iter()
        .map(|path| RouterSkillInstallStatus {
            present: path.is_dir(),
            managed: path.join(ROUTER_MARKER).is_file(),
            has_skill_md: path.join("SKILL.md").is_file(),
            has_skill_spec: path.join("skill.spec.yml").is_file(),
            path,
        })
        .collect()
}

fn router_skill_warnings(statuses: &[RouterSkillInstallStatus]) -> Vec<String> {
    statuses
        .iter()
        .filter(|status| {
            !status.present || !status.managed || !status.has_skill_md || !status.has_skill_spec
        })
        .map(|status| {
            format!(
                "managed router skill is not ready: {} present={} managed={} has_skill_md={} has_skill_spec={}",
                status.path.display(),
                status.present,
                status.managed,
                status.has_skill_md,
                status.has_skill_spec
            )
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct HookTarget {
    harness: RouterHarness,
    path: PathBuf,
}

fn hook_targets_for_roots(roots: &[PathBuf]) -> Vec<HookTarget> {
    let mut targets = BTreeSet::new();
    for root in roots {
        if let Some(base) = parent_named(root, ".codex") {
            targets.insert(HookTarget {
                harness: RouterHarness::Codex,
                path: base.join("hooks.json"),
            });
        }
        if let Some(base) = parent_named(root, ".claude") {
            targets.insert(HookTarget {
                harness: RouterHarness::Claude,
                path: base.join("settings.json"),
            });
        }
        if let Some(base) = shared_agents_base(root) {
            targets.insert(HookTarget {
                harness: RouterHarness::Codex,
                path: base.join(".codex/hooks.json"),
            });
            let claude_dir = base.join(".claude");
            if claude_dir.is_dir() {
                targets.insert(HookTarget {
                    harness: RouterHarness::Claude,
                    path: claude_dir.join("settings.json"),
                });
            }
        }
    }
    targets.into_iter().collect()
}

fn parent_named(root: &Path, name: &str) -> Option<PathBuf> {
    if root.file_name().and_then(|value| value.to_str()) != Some("skills") {
        return None;
    }
    let parent = root.parent()?;
    (parent.file_name().and_then(|value| value.to_str()) == Some(name))
        .then(|| parent.to_path_buf())
}

fn shared_agents_base(root: &Path) -> Option<PathBuf> {
    if root.file_name().and_then(|value| value.to_str()) != Some("skills") {
        return None;
    }
    let parent = root.parent()?;
    if parent.file_name().and_then(|value| value.to_str()) != Some(".agents") {
        return None;
    }
    parent.parent().map(Path::to_path_buf)
}

fn install_hook(
    target: &HookTarget,
    command: &str,
    dry_run: bool,
) -> Result<RouterHarnessHookReport> {
    if dry_run {
        return Ok(report_for_target(
            target,
            RouterHarnessHookStatus::Planned,
            command,
            "managed router guard hook would be installed",
        ));
    }
    let mut root = read_json_object(&target.path)?;
    remove_managed_hook_from_object(&mut root);
    add_managed_hook_to_object(target.harness, &mut root, command);
    write_json_object(&target.path, &root)?;
    Ok(report_for_target(
        target,
        RouterHarnessHookStatus::Installed,
        command,
        "managed router guard hook installed",
    ))
}

fn remove_hook(
    target: &HookTarget,
    command: &str,
    dry_run: bool,
) -> Result<RouterHarnessHookReport> {
    if dry_run {
        return Ok(report_for_target(
            target,
            RouterHarnessHookStatus::Planned,
            command,
            "managed router guard hook would be removed",
        ));
    }
    if !target.path.is_file() {
        return Ok(report_for_target(
            target,
            RouterHarnessHookStatus::Missing,
            command,
            "hook config file is missing",
        ));
    }
    let mut root = read_json_object(&target.path)?;
    let removed = remove_managed_hook_from_object(&mut root);
    write_json_object(&target.path, &root)?;
    Ok(report_for_target(
        target,
        if removed {
            RouterHarnessHookStatus::Removed
        } else {
            RouterHarnessHookStatus::Missing
        },
        command,
        if removed {
            "managed router guard hook removed"
        } else {
            "managed router guard hook was not present"
        },
    ))
}

fn inspect_hook(target: &HookTarget, command: &str) -> RouterHarnessHookReport {
    if !target.path.is_file() {
        return report_for_target(
            target,
            RouterHarnessHookStatus::Missing,
            command,
            "hook config file is missing",
        );
    }
    let status = match read_json_object(&target.path) {
        Ok(root) if has_expected_hook(&root, command) => RouterHarnessHookStatus::Installed,
        Ok(_) | Err(_) => RouterHarnessHookStatus::Missing,
    };
    report_for_target(
        target,
        status,
        command,
        if status == RouterHarnessHookStatus::Installed {
            "managed router guard hook is installed"
        } else {
            "managed router guard hook is missing"
        },
    )
}

fn report_for_target(
    target: &HookTarget,
    status: RouterHarnessHookStatus,
    command: &str,
    message: &str,
) -> RouterHarnessHookReport {
    RouterHarnessHookReport {
        harness: target.harness,
        path: target.path.clone(),
        status,
        command: command.to_owned(),
        message: message.to_owned(),
    }
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>> {
    if !path.is_file() {
        return Ok(Map::new());
    }
    let text = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let value: Value = serde_json::from_str(&text).map_err(|source| Error::ParseJson {
        path: path.to_path_buf(),
        source,
    })?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| Error::InvalidInput {
            message: format!("hook config must be a JSON object: {}", path.display()),
        })
}

fn write_json_object(path: &Path, root: &Map<String, Value>) -> Result<()> {
    let json = serde_json::to_string_pretty(root).map_err(Error::RenderJson)?;
    write_file(path, &format!("{json}\n"))
}

fn add_managed_hook_to_object(
    harness: RouterHarness,
    root: &mut Map<String, Value>,
    command: &str,
) {
    let hooks = object_field(root, "hooks");
    let event = array_field(hooks, "UserPromptSubmit");
    let hook = match harness {
        RouterHarness::Codex => json!({
            "hooks": [
                {
                    "type": "command",
                    "command": command,
                    "statusMessage": "Checking SkillSpec router guard"
                }
            ]
        }),
        RouterHarness::Claude => json!({
            "hooks": [
                {
                    "type": "command",
                    "command": command
                }
            ]
        }),
    };
    event.push(hook);
}

fn object_field<'a>(root: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).is_some_and(Value::is_object) {
        root.insert(key.to_owned(), Value::Object(Map::new()));
    }
    root.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object field was just inserted")
}

fn array_field<'a>(root: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    if !root.get(key).is_some_and(Value::is_array) {
        root.insert(key.to_owned(), Value::Array(Vec::new()));
    }
    root.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array field was just inserted")
}

fn remove_managed_hook_from_object(root: &mut Map<String, Value>) -> bool {
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return false;
    };
    let mut removed = false;
    for event in hooks.values_mut() {
        let Some(groups) = event.as_array_mut() else {
            continue;
        };
        let mut next_groups = Vec::new();
        for mut group in std::mem::take(groups) {
            let mut keep_group = true;
            if let Some(hook_list) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                let before = hook_list.len();
                hook_list.retain(|hook| !is_managed_hook(hook));
                removed |= hook_list.len() != before;
                keep_group = !hook_list.is_empty();
            }
            if keep_group {
                next_groups.push(group);
            }
        }
        *groups = next_groups;
    }
    removed
}

fn has_expected_hook(root: &Map<String, Value>, command: &str) -> bool {
    root.get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|hooks| hooks.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|group| group.get("hooks").and_then(Value::as_array))
        .flatten()
        .any(|hook| {
            hook.get("command")
                .and_then(Value::as_str)
                .is_some_and(|hook_command| hook_command == command)
        })
}

fn is_managed_hook(hook: &Value) -> bool {
    hook.get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| command.contains(MANAGED_HOOK_MATCH) && command.contains("--hook"))
}

fn managed_hook_command(config_path: &Path, harness: RouterHarness) -> String {
    let route_harness = match harness {
        RouterHarness::Codex => router::RouteHarness::Codex,
        RouterHarness::Claude => router::RouteHarness::ClaudeLocal,
    };
    format!(
        "skillspec router guard --config {} --hook --harness {}",
        shell_quote(&config_path.to_string_lossy()),
        route_harness.as_str()
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn repair_command_for_config(config: &RouterConfig) -> String {
    let roots = config
        .roots
        .iter()
        .map(|root| format!(" --roots {}", shell_quote(&root.to_string_lossy())))
        .collect::<String>();
    format!(
        "skillspec router index refresh{roots} --index {} --visibility-manifest {}",
        shell_quote(&config.index.to_string_lossy()),
        shell_quote(&config.manifest.to_string_lossy())
    )
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}
