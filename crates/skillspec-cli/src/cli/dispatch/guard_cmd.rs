//! The `skillspec boundary guard` lifecycle.
//!
//! The guard's policy store, decision logic, and decision log live in
//! `skillspec-boundary`. The managed `PreToolUse` hook mutation lives here,
//! because it edits the harness's own `settings.json` and keeping it out of the
//! analysis crate keeps that crate free of a harness dependency. The mutation
//! reuses the router guard's discipline: remove only handlers whose command is
//! the managed guard command, and leave every user hook in place.

use crate::cli::args::GuardCommand;
use serde_json::{json, Map, Value};
use skillspec::boundary::guard::{GuardConfig, GuardPolicy, GuardStore, Mode};
use skillspec::{boundary, error::Error, error::Result, report};
use std::io::Read;
use std::path::{Path, PathBuf};

/// The managed guard command written into the hook config.
const MANAGED_COMMAND: &str = "skillspec boundary guard hook";

pub(super) fn run(command: GuardCommand) -> Result<()> {
    match command {
        GuardCommand::Install => install(),
        GuardCommand::Add { path } => add(path),
        GuardCommand::Status => status(),
        GuardCommand::Mode { mode } => set_mode(mode),
        GuardCommand::Log { json } => log(json),
        GuardCommand::Uninstall => uninstall(),
        GuardCommand::Hook => hook(),
    }
}

fn store() -> Result<GuardStore> {
    Ok(GuardStore::at(&skillspec_home()?))
}

fn install() -> Result<()> {
    let store = store()?;
    store.save_config(&GuardConfig::default())?;
    let installed = install_hook()?;
    report::text(&format!(
        "Installed the boundary guard in observe mode.\n{installed}\nApprove a skill with `skillspec boundary guard add <skill>`, then read the log with `skillspec boundary guard log`.\n"
    ))
}

fn add(path: String) -> Result<()> {
    let surface = boundary::analyze_target(&path)?;
    let proposal = boundary::compile(&surface);
    let slug = Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_owned();
    let policy = GuardPolicy::from_proposal(&slug, &proposal);
    let stored = store()?.save_policy(&policy)?;

    let mut message = format!(
        "Approved {} grant(s) for {slug}.\nPolicy: {}\n",
        policy.allow.len(),
        stored.display()
    );
    if !policy.pending_review.is_empty() {
        message.push_str(&format!(
            "Held for review, not approved: {}\n",
            policy.pending_review.join(", ")
        ));
    }
    if !policy.complete {
        message.push_str("The skill's surface is incomplete, so this policy does not cover everything it can do.\n");
    }
    report::text(&message)
}

fn status() -> Result<()> {
    let store = store()?;
    let config = store.config()?;
    let policies = store.policies()?;
    let hooks = installed_hook_paths();

    let mut out = String::from("SkillSpec Boundary Guard\n");
    out.push_str(&format!("Mode: {}\n", config.mode.as_str()));
    out.push_str(&format!("Enabled: {}\n", config.enabled));
    out.push_str(&format!(
        "Managed hooks: {}\n",
        if hooks.is_empty() {
            "none installed".to_owned()
        } else {
            hooks.join(", ")
        }
    ));
    out.push_str(&format!("Policies: {}\n", policies.len()));
    for policy in &policies {
        out.push_str(&format!(
            "- {} ({} allowed{})\n",
            policy.skill_slug,
            policy.allow.len(),
            if policy.pending_review.is_empty() {
                String::new()
            } else {
                format!(", {} pending review", policy.pending_review.len())
            }
        ));
    }
    out.push_str(
        "\nThe guard is advisory: if the hook cannot run, the harness decides. SkillSpec does not sandbox.\n",
    );
    report::text(&out)
}

fn set_mode(mode: String) -> Result<()> {
    let parsed = Mode::parse(&mode).ok_or_else(|| Error::InvalidInput {
        message: format!("unknown guard mode {mode:?}; use observe, prompt, or enforce"),
    })?;
    let store = store()?;
    let mut config = store.config()?;
    config.mode = parsed;
    store.save_config(&config)?;
    report::text(&format!("Guard mode set to {}.\n", parsed.as_str()))
}

fn log(json: bool) -> Result<()> {
    let decisions = store()?.decisions()?;
    if json {
        return report::json(&decisions);
    }
    if decisions.is_empty() {
        return report::text("No decisions recorded yet.\n");
    }
    let mut out = String::new();
    let covered = decisions.iter().filter(|d| d.uncovered.is_empty()).count();
    out.push_str(&format!(
        "{} call(s) recorded; {} covered by policy, {} not.\n\n",
        decisions.len(),
        covered,
        decisions.len() - covered
    ));
    for entry in decisions.iter().rev().take(20) {
        out.push_str(&format!(
            "{}  {:<8} {:<7} {}\n",
            entry.ts,
            entry.tool,
            entry.decision,
            if entry.uncovered.is_empty() {
                "covered".to_owned()
            } else {
                format!("uncovered: {}", entry.uncovered.join(", "))
            }
        ));
    }
    report::text(&out)
}

fn uninstall() -> Result<()> {
    let removed = uninstall_hook()?;
    report::text(&format!(
        "Removed the managed guard hook.\n{removed}\nStored policies and the decision log are left in place; delete $SKILLSPEC_HOME/boundary to remove them.\n"
    ))
}

/// The `PreToolUse` entrypoint. Reads the payload on stdin and prints a
/// decision; a parse failure defers rather than blocking, so a guard bug never
/// wedges the user's session.
fn hook() -> Result<()> {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return print_value(&defer("could not read the hook payload"));
    }
    let payload: Value = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(_) => return print_value(&defer("could not parse the hook payload")),
    };
    // The pure crate does not read the clock, so the CLI supplies the time.
    // Unix seconds keeps it dependency-free and sortable.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_owned());
    match boundary::guard::hook_decision(&store()?, &payload, &now) {
        Ok(decision) => print_value(&decision),
        Err(_) => print_value(&defer("the guard could not evaluate this call")),
    }
}

fn defer(reason: &str) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "defer",
            "permissionDecisionReason": format!("SkillSpec boundary guard: {reason}"),
        }
    })
}

fn print_value(value: &Value) -> Result<()> {
    report::text(&format!("{value}\n"))
}

// ---- managed hook mutation (settings.json PreToolUse) ----

/// The guard installs into the user's global Claude settings only.
///
/// Enforcement is against the union of every approved policy, so the guard is
/// inherently global; a per-project hook would fragment that. Installing into an
/// arbitrary working directory's `.claude` would also be intrusive.
fn hook_targets() -> Vec<PathBuf> {
    home_dir()
        .map(|home| vec![home.join(".claude").join("settings.json")])
        .unwrap_or_default()
}

fn installed_hook_paths() -> Vec<String> {
    hook_targets()
        .into_iter()
        .filter(|path| has_managed_hook(path))
        .map(|path| path.display().to_string())
        .collect()
}

fn install_hook() -> Result<String> {
    let mut installed = Vec::new();
    for path in hook_targets() {
        let mut root = read_object(&path)?;
        add_managed_hook(&mut root);
        write_object(&path, &root)?;
        installed.push(path.display().to_string());
    }
    Ok(if installed.is_empty() {
        "No .claude settings location was found to install into.".to_owned()
    } else {
        format!("Hook installed in: {}", installed.join(", "))
    })
}

fn uninstall_hook() -> Result<String> {
    let mut removed = Vec::new();
    for path in hook_targets() {
        if !path.exists() {
            continue;
        }
        let mut root = read_object(&path)?;
        if remove_managed_hook(&mut root) {
            write_object(&path, &root)?;
            removed.push(path.display().to_string());
        }
    }
    Ok(if removed.is_empty() {
        "No managed guard hook was installed.".to_owned()
    } else {
        format!("Hook removed from: {}", removed.join(", "))
    })
}

fn read_object(path: &Path) -> Result<Map<String, Value>> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    serde_json::from_str::<Value>(&text)
        .map_err(|source| Error::ParseJson {
            path: path.to_path_buf(),
            source,
        })?
        .as_object()
        .cloned()
        .ok_or_else(|| Error::InvalidInput {
            message: format!("settings file is not a JSON object: {}", path.display()),
        })
}

fn write_object(path: &Path, root: &Map<String, Value>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(root).map_err(Error::RenderJson)?;
    std::fs::write(path, format!("{json}\n")).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn add_managed_hook(root: &mut Map<String, Value>) {
    let hooks = object_field(root, "hooks");
    let events = array_field(hooks, "PreToolUse");
    // Idempotent: do not add a second managed entry.
    if events.iter().any(managed_matcher_entry) {
        return;
    }
    events.push(json!({
        "matcher": "*",
        "hooks": [ { "type": "command", "command": MANAGED_COMMAND } ]
    }));
}

fn remove_managed_hook(root: &mut Map<String, Value>) -> bool {
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return false;
    };
    let Some(events) = hooks.get_mut("PreToolUse").and_then(Value::as_array_mut) else {
        return false;
    };
    let before = events.len();
    events.retain(|entry| !managed_matcher_entry(entry));
    events.len() != before
}

/// Whether a PreToolUse matcher entry contains the managed guard command.
fn managed_matcher_entry(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| {
            hooks
                .iter()
                .any(|hook| hook.get("command").and_then(Value::as_str) == Some(MANAGED_COMMAND))
        })
}

fn has_managed_hook(path: &Path) -> bool {
    read_object(path)
        .ok()
        .and_then(|root| root.get("hooks").and_then(Value::as_object).cloned())
        .and_then(|hooks| hooks.get("PreToolUse").and_then(Value::as_array).cloned())
        .is_some_and(|events| events.iter().any(managed_matcher_entry))
}

fn object_field<'a>(root: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).is_some_and(Value::is_object) {
        root.insert(key.to_owned(), Value::Object(Map::new()));
    }
    root.get_mut(key).and_then(Value::as_object_mut).unwrap()
}

fn array_field<'a>(root: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    if !root.get(key).is_some_and(Value::is_array) {
        root.insert(key.to_owned(), Value::Array(Vec::new()));
    }
    root.get_mut(key).and_then(Value::as_array_mut).unwrap()
}

fn skillspec_home() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SKILLSPEC_HOME") {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".skillspec"))
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::InvalidInput {
            message: "HOME is not set; set SKILLSPEC_HOME or HOME".to_owned(),
        })
}
