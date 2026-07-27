//! The boundary guard: applying a reviewed policy to installed skills.
//!
//! This module owns the policy store, the operating mode, the decision log, and
//! the `PreToolUse` hook decision. It does not install harness hooks - that
//! mutates another project's config file and lives in the CLI layer, so this
//! crate stays free of a harness dependency.
//!
//! The guard is an advisory gate at a lifecycle event a harness chooses to
//! expose. It is not a sandbox: no kernel involvement, no isolation, no network
//! interposition. A guard that cannot run does not silently permit; that is a
//! stated limit, surfaced in status, not a guarantee of coverage.
//!
//! See `docs/design/security/43-boundary-guard-hook.md`.

pub mod decide;
pub mod policy;

pub use decide::{evaluate, Decision, GuardDecision, Mode};
pub use policy::GuardPolicy;

use serde::{Deserialize, Serialize};
use skillspec_core::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// The guard's persistent configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardConfig {
    pub enabled: bool,
    pub mode: Mode,
}

impl Default for GuardConfig {
    fn default() -> Self {
        // Observe is the install default. A guard that blocks on day one against
        // a policy nobody has validated gets uninstalled within an hour, and a
        // control that is uninstalled protects nothing.
        Self {
            enabled: true,
            mode: Mode::Observe,
        }
    }
}

/// The guard's on-disk store, rooted at `$SKILLSPEC_HOME/boundary/`.
pub struct GuardStore {
    root: PathBuf,
}

impl GuardStore {
    /// Open the store under the given SkillSpec home directory.
    pub fn at(skillspec_home: &Path) -> Self {
        Self {
            root: skillspec_home.join("boundary"),
        }
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    fn policies_dir(&self) -> PathBuf {
        self.root.join("policies")
    }

    fn decisions_path(&self) -> PathBuf {
        self.root.join("decisions.jsonl")
    }

    /// Load the config, or the default if none is stored.
    pub fn config(&self) -> Result<GuardConfig> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(GuardConfig::default());
        }
        let text = fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| Error::ParseJson { path, source })
    }

    /// Persist the config.
    pub fn save_config(&self, config: &GuardConfig) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|source| Error::Write {
            path: self.root.clone(),
            source,
        })?;
        let json = serde_json::to_string_pretty(config).map_err(Error::RenderJson)?;
        write(&self.config_path(), &format!("{json}\n"))
    }

    /// Store a reviewed policy, keyed by its skill slug.
    pub fn save_policy(&self, policy: &GuardPolicy) -> Result<PathBuf> {
        let dir = self.policies_dir();
        fs::create_dir_all(&dir).map_err(|source| Error::Write {
            path: dir.clone(),
            source,
        })?;
        let path = dir.join(format!("{}.json", slugify(&policy.skill_slug)));
        let json = serde_json::to_string_pretty(policy).map_err(Error::RenderJson)?;
        write(&path, &format!("{json}\n"))?;
        Ok(path)
    }

    /// Every stored policy, in stable slug order.
    pub fn policies(&self) -> Result<Vec<GuardPolicy>> {
        let dir = self.policies_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = fs::read_dir(&dir)
            .map_err(|source| Error::Read {
                path: dir.clone(),
                source,
            })?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        entries.sort();

        let mut policies = Vec::new();
        for path in entries {
            let text = fs::read_to_string(&path).map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;
            let policy = serde_json::from_str(&text).map_err(|source| Error::ParseJson {
                path: path.clone(),
                source,
            })?;
            policies.push(policy);
        }
        Ok(policies)
    }

    /// The union of every stored policy's approved grants: the effective
    /// allow-set the guard enforces, since a `PreToolUse` call cannot be
    /// attributed to a single skill.
    pub fn allow_set(&self) -> Result<Vec<String>> {
        let mut allow = self
            .policies()?
            .into_iter()
            .flat_map(|policy| policy.allow)
            .collect::<Vec<_>>();
        allow.sort();
        allow.dedup();
        Ok(allow)
    }

    /// Append one decision to the local, never-transmitted decision log.
    pub fn record(&self, entry: &DecisionRecord) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|source| Error::Write {
            path: self.root.clone(),
            source,
        })?;
        let line = serde_json::to_string(entry).map_err(Error::RenderJson)?;
        let mut existing = if self.decisions_path().exists() {
            fs::read_to_string(self.decisions_path()).unwrap_or_default()
        } else {
            String::new()
        };
        existing.push_str(&line);
        existing.push('\n');
        write(&self.decisions_path(), &existing)
    }

    /// The recorded decisions, newest last.
    pub fn decisions(&self) -> Result<Vec<DecisionRecord>> {
        let path = self.decisions_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        Ok(text
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
    }
}

/// One appended decision. Local only, never transmitted.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Caller-supplied timestamp; this crate does not read the clock.
    pub ts: String,
    pub tool: String,
    pub decision: String,
    pub mode: String,
    pub effects: Vec<String>,
    pub uncovered: Vec<String>,
}

impl DecisionRecord {
    pub fn from_decision(ts: &str, mode: Mode, decision: &GuardDecision) -> Self {
        Self {
            ts: ts.to_owned(),
            tool: decision.tool.clone(),
            decision: decision.decision.permission_decision().to_owned(),
            mode: mode.as_str().to_owned(),
            effects: decision.effects.clone(),
            uncovered: decision.uncovered.clone(),
        }
    }
}

/// Evaluate a Claude Code `PreToolUse` payload against the store, record the
/// decision, and return the JSON the hook should print on stdout.
///
/// `now` is supplied by the caller; this crate does not read the clock, so the
/// decision log stays deterministic under test.
pub fn hook_decision(
    store: &GuardStore,
    payload: &serde_json::Value,
    now: &str,
) -> Result<serde_json::Value> {
    let config = store.config()?;
    if !config.enabled {
        return Ok(defer_output("guard disabled"));
    }
    let tool = payload
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let empty = serde_json::Value::Object(Default::default());
    let input = payload.get("tool_input").unwrap_or(&empty);

    let allow = store.allow_set()?;
    let decision = evaluate(tool, input, &allow, config.mode);
    store.record(&DecisionRecord::from_decision(now, config.mode, &decision))?;

    Ok(decision_output(&decision))
}

/// The `hookSpecificOutput` object for a `PreToolUse` decision.
///
/// Format verified against the Claude Code hooks documentation on 2026-07-27:
/// `permissionDecision` is one of allow/deny/ask/defer, with a reason.
fn decision_output(decision: &GuardDecision) -> serde_json::Value {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": decision.decision.permission_decision(),
            "permissionDecisionReason": format!("SkillSpec boundary guard: {}", decision.reason),
        }
    })
}

fn defer_output(reason: &str) -> serde_json::Value {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "defer",
            "permissionDecisionReason": format!("SkillSpec boundary guard: {reason}"),
        }
    })
}

fn write(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{DecisionRecord, GuardConfig, GuardStore, Mode};
    use crate::guard::policy::GuardPolicy;

    fn temp_home() -> std::path::PathBuf {
        // A unique directory under the crate's target dir, not the real home.
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/guard-store-tests");
        let unique = base.join(format!("h{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&unique);
        std::fs::create_dir_all(&unique).unwrap();
        unique
    }

    #[test]
    fn the_default_config_is_observe() {
        let config = GuardConfig::default();
        assert_eq!(config.mode, Mode::Observe);
        assert!(config.enabled);
    }

    #[test]
    fn config_round_trips_through_the_store() {
        let home = temp_home();
        let store = GuardStore::at(&home);
        assert_eq!(store.config().unwrap().mode, Mode::Observe);
        store
            .save_config(&GuardConfig {
                enabled: true,
                mode: Mode::Enforce,
            })
            .unwrap();
        assert_eq!(store.config().unwrap().mode, Mode::Enforce);
    }

    #[test]
    fn the_allow_set_is_the_union_of_stored_policies() {
        let home = temp_home();
        let store = GuardStore::at(&home);
        store
            .save_policy(&GuardPolicy {
                schema: "s".to_owned(),
                skill_slug: "a".to_owned(),
                allow: vec!["proc.exec:git".to_owned()],
                pending_review: vec![],
                complete: true,
            })
            .unwrap();
        store
            .save_policy(&GuardPolicy {
                schema: "s".to_owned(),
                skill_slug: "b".to_owned(),
                allow: vec![
                    "net.fetch:api.github.com".to_owned(),
                    "proc.exec:git".to_owned(),
                ],
                pending_review: vec![],
                complete: true,
            })
            .unwrap();
        let allow = store.allow_set().unwrap();
        assert_eq!(allow, ["net.fetch:api.github.com", "proc.exec:git"]);
    }

    #[test]
    fn decisions_append_and_read_back() {
        let home = temp_home();
        let store = GuardStore::at(&home);
        for id in ["a", "b"] {
            store
                .record(&DecisionRecord {
                    ts: "2026-01-01T00:00:00Z".to_owned(),
                    tool: id.to_owned(),
                    decision: "defer".to_owned(),
                    mode: "observe".to_owned(),
                    effects: vec![],
                    uncovered: vec![],
                })
                .unwrap();
        }
        let decisions = store.decisions().unwrap();
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[1].tool, "b");
    }
}
