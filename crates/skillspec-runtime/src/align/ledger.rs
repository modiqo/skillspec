use super::AlignTokenSummary;
use skillspec_core::error::{Error, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub(super) struct ExecutionLedger {
    pub(super) paths: Vec<String>,
    pub(super) events: Vec<ExecutionEvent>,
}

#[derive(Clone, Debug)]
pub(super) struct ExecutionEvent {
    pub(super) event: String,
    pub(super) phase: Option<String>,
    pub(super) requirement: Option<String>,
    pub(super) command: Option<String>,
    pub(super) executor: Option<String>,
    pub(super) through_rote: Option<bool>,
    pub(super) operation_kind: Option<String>,
    pub(super) execution_mode: Option<String>,
    pub(super) workspace: Option<String>,
    pub(super) response_id: Option<String>,
    pub(super) lease_id: Option<String>,
    pub(super) exit_code: Option<i64>,
    pub(super) timed_out: Option<bool>,
    pub(super) stdout_captured: Option<bool>,
    pub(super) stderr_captured: Option<bool>,
    pub(super) ready: Option<bool>,
    pub(super) anonymous: Option<bool>,
    pub(super) included_result: Option<bool>,
    pub(super) included_alignment: Option<bool>,
    pub(super) included_evidence: Option<bool>,
    pub(super) included_token_savings: Option<bool>,
    pub(super) fallback_needed: Option<bool>,
    pub(super) matches_len: Option<usize>,
    pub(super) id: Option<String>,
    pub(super) total_tokens: Option<u64>,
    pub(super) input_tokens: Option<u64>,
    pub(super) output_tokens: Option<u64>,
    pub(super) prompt_tokens: Option<u64>,
    pub(super) completion_tokens: Option<u64>,
    pub(super) context_tokens: Option<u64>,
    pub(super) query_result_tokens: Option<u64>,
    pub(super) cached_tokens: Option<u64>,
    pub(super) response_tokens_cached: Option<u64>,
    pub(super) saved_tokens: Option<u64>,
    pub(super) reduction_percent: Option<f64>,
    pub(super) agent_visible_tokens: Option<u64>,
    pub(super) artifact_tokens_preserved: Option<u64>,
    pub(super) avoided_tokens: Option<u64>,
    pub(super) metrics_source: Option<String>,
}

impl ExecutionLedger {
    pub(super) fn read(paths: &[PathBuf]) -> Result<Self> {
        let mut ledger = Self::default();
        for path in paths {
            ledger.paths.push(path.display().to_string());
            let content = fs::read_to_string(path).map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;
            let trimmed = content.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('[') {
                let values =
                    serde_json::from_str::<Vec<serde_json::Value>>(trimmed).map_err(|source| {
                        Error::ParseJson {
                            path: path.clone(),
                            source,
                        }
                    })?;
                ledger
                    .events
                    .extend(values.iter().map(ExecutionEvent::from_value));
                continue;
            }
            for line in content.lines().filter(|line| !line.trim().is_empty()) {
                let value = serde_json::from_str::<serde_json::Value>(line).map_err(|source| {
                    Error::ParseJson {
                        path: path.clone(),
                        source,
                    }
                })?;
                ledger.events.push(ExecutionEvent::from_value(&value));
            }
        }
        Ok(ledger)
    }

    pub(super) fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    pub(super) fn process_starts(&self) -> Vec<&ExecutionEvent> {
        self.events
            .iter()
            .filter(|event| {
                matches!(
                    event.event.as_str(),
                    "process_started" | "background_process_started"
                )
            })
            .collect()
    }

    pub(super) fn has_process_start(&self) -> bool {
        !self.process_starts().is_empty()
    }

    pub(super) fn has_background_start(&self) -> bool {
        self.events
            .iter()
            .any(|event| event.event == "background_process_started")
    }

    pub(super) fn has_background_terminal_event(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.event.as_str(),
                "process_wait_finished" | "process_status_checked"
            )
        })
    }

    pub(super) fn background_lease_summary(&self) -> Option<String> {
        let leases = self
            .events
            .iter()
            .filter(|event| event.event == "background_process_started")
            .filter_map(|event| event.lease_id.as_deref())
            .collect::<BTreeSet<_>>();
        (!leases.is_empty()).then(|| {
            let timed_out = self
                .events
                .iter()
                .any(|event| event.timed_out == Some(true));
            format!(
                "background lease(s) {} were started and {}{}",
                leases.into_iter().collect::<Vec<_>>().join(", "),
                if self.has_background_terminal_event() {
                    "status-checked or waited"
                } else {
                    "not status-checked or waited"
                },
                if timed_out { "; timeout observed" } else { "" }
            )
        })
    }

    pub(super) fn has_adapter_discovery(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.event.as_str(),
                "adapter_discovery_started"
                    | "adapter_discovery_finished"
                    | "adapter_discovery_ran"
            )
        })
    }

    pub(super) fn adapter_discovery_summary(&self) -> Option<String> {
        self.events
            .iter()
            .find(|event| {
                matches!(
                    event.event.as_str(),
                    "adapter_discovery_finished" | "adapter_discovery_ran"
                )
            })
            .map(|event| {
                let matches = event
                    .matches_len
                    .map(|count| format!("{count} match(es)"))
                    .unwrap_or_else(|| "unknown matches".to_owned());
                let fallback = match event.fallback_needed {
                    Some(true) => "; fallback needed",
                    Some(false) => "; fallback not needed",
                    None => "",
                };
                format!("adapter discovery ran with {matches}{fallback}")
            })
    }

    pub(super) fn has_cli_readiness(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.event.as_str(),
                "cli_readiness_check_finished" | "dependency_check_finished"
            )
        })
    }

    pub(super) fn cli_readiness_ready(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.event.as_str(),
                "cli_readiness_check_finished" | "dependency_check_finished"
            ) && event.ready != Some(false)
                && event.exit_code.unwrap_or(0) == 0
        })
    }

    pub(super) fn process_commands(&self) -> Vec<String> {
        self.process_starts()
            .into_iter()
            .filter_map(|event| event.command.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub(super) fn command_summary(&self) -> String {
        let starts = self.process_starts();
        let commands = self.process_commands();
        if commands.is_empty() {
            return "no process command evidence".to_owned();
        }
        let operations = starts
            .iter()
            .filter_map(|event| event.operation_kind.as_deref())
            .collect::<BTreeSet<_>>();
        let modes = starts
            .iter()
            .filter_map(|event| event.execution_mode.as_deref())
            .collect::<BTreeSet<_>>();
        let refs = starts
            .iter()
            .filter_map(|event| event.short_ref())
            .collect::<BTreeSet<_>>();
        let mut parts = vec![format!(
            "command(s) {} ran with arguments redacted",
            commands.join(", ")
        )];
        if !operations.is_empty() {
            parts.push(format!(
                "operation(s) {}",
                operations.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        if !modes.is_empty() {
            parts.push(format!(
                "mode(s) {}",
                modes.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        if !refs.is_empty() {
            parts.push(format!(
                "evidence {}",
                refs.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        parts.join("; ")
    }

    pub(super) fn all_processes_use_rote_exec(&self) -> Option<bool> {
        let starts = self.process_starts();
        if starts.is_empty() {
            return None;
        }
        Some(starts.iter().all(|event| event.uses_rote_exec()))
    }

    pub(super) fn any_direct_process(&self) -> bool {
        self.process_starts()
            .iter()
            .any(|event| event.is_direct_process())
    }

    pub(super) fn all_processes_have_workspace(&self) -> Option<bool> {
        let starts = self.process_starts();
        if starts.is_empty() {
            return None;
        }
        Some(starts.iter().all(|event| event.workspace.is_some()))
    }

    pub(super) fn has_named_workspace(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.event.as_str(),
                "workspace_created" | "workspace_selected"
            ) && event.anonymous != Some(true)
                && event.workspace.is_some()
        }) || self
            .process_starts()
            .iter()
            .any(|event| event.workspace.is_some())
    }

    pub(super) fn any_anonymous_workspace(&self) -> bool {
        self.events
            .iter()
            .any(|event| event.anonymous == Some(true))
    }

    pub(super) fn all_process_output_captured(&self) -> Option<bool> {
        let starts = self.process_starts();
        if starts.is_empty() {
            return None;
        }
        Some(starts.iter().all(|event| {
            event.stdout_captured == Some(true) || event.stderr_captured == Some(true)
        }))
    }

    pub(super) fn has_stats_collected(&self) -> bool {
        self.events
            .iter()
            .any(|event| event.event == "stats_collected")
    }

    pub(super) fn has_workspace_trace_collected(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.event.as_str(),
                "workspace_trace_collected" | "trace_collected"
            )
        })
    }

    pub(super) fn has_final_response(&self) -> bool {
        self.events
            .iter()
            .any(|event| event.event == "final_response_sent")
    }

    pub(super) fn final_response_included_evidence(&self) -> bool {
        self.events.iter().any(|event| {
            event.event == "final_response_sent"
                && event.included_result == Some(true)
                && event.included_evidence == Some(true)
                && event.included_token_savings == Some(true)
        })
    }

    pub(super) fn final_response_included_alignment(&self) -> bool {
        self.events.iter().any(|event| {
            event.event == "final_response_sent" && event.included_alignment == Some(true)
        })
    }

    pub(super) fn has_negative_event(&self, names: &[&str]) -> bool {
        self.events
            .iter()
            .any(|event| names.iter().any(|name| event.event == *name))
    }

    pub(super) fn has_event_for_id(&self, names: &[&str], id: &str) -> bool {
        self.events.iter().any(|event| {
            names.iter().any(|name| event.event == *name)
                && event.id.as_deref().is_some_and(|value| value == id)
        })
    }

    pub(super) fn phase_events(&self) -> Vec<&ExecutionEvent> {
        self.events
            .iter()
            .filter(|event| {
                matches!(
                    event.event.as_str(),
                    "phase_started" | "phase_completed" | "phase_blocked"
                ) && event.phase.is_some()
            })
            .collect()
    }

    pub(super) fn has_requirement_satisfied(&self, phase: &str, requirement: &str) -> bool {
        self.events.iter().any(|event| {
            event.event == "requirement_satisfied"
                && event.phase.as_deref() == Some(phase)
                && event.requirement.as_deref() == Some(requirement)
        })
    }

    pub(super) fn has_requirement_failed(&self, phase: &str, requirement: &str) -> bool {
        self.events.iter().any(|event| {
            event.event == "requirement_failed"
                && event.phase.as_deref() == Some(phase)
                && event.requirement.as_deref() == Some(requirement)
        })
    }

    pub(super) fn forbidden_violation_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| {
                matches!(
                    event.event.as_str(),
                    "forbidden_action" | "forbidden_action_observed" | "forbid_violated"
                )
            })
            .count()
    }

    pub(super) fn token_summary(&self) -> AlignTokenSummary {
        let token_events = self
            .events
            .iter()
            .filter(|event| event.has_token_fields())
            .collect::<Vec<_>>();
        if token_events.is_empty() {
            return AlignTokenSummary {
                consumption: "not recorded".to_owned(),
                savings: "not recorded".to_owned(),
                evidence: vec![
                    "add a stats_collected event with token usage and savings fields".to_owned(),
                ],
            };
        }

        let total_tokens = sum_token(&token_events, |event| event.total_tokens).or_else(|| {
            let input = sum_token(&token_events, |event| {
                event.input_tokens.or(event.prompt_tokens)
            });
            let output = sum_token(&token_events, |event| {
                event.output_tokens.or(event.completion_tokens)
            });
            input.zip(output).map(|(input, output)| input + output)
        });
        let input_tokens = sum_token(&token_events, |event| {
            event.input_tokens.or(event.prompt_tokens)
        });
        let output_tokens = sum_token(&token_events, |event| {
            event.output_tokens.or(event.completion_tokens)
        });
        let context_tokens = sum_token(&token_events, |event| event.context_tokens);
        let query_result_tokens = sum_token(&token_events, |event| event.query_result_tokens);
        let agent_visible_tokens = sum_token(&token_events, |event| event.agent_visible_tokens);
        let artifact_tokens_preserved =
            sum_token(&token_events, |event| event.artifact_tokens_preserved);
        let avoided_output_tokens =
            sum_token(&token_events, |event| event.avoided_tokens).or_else(|| {
                artifact_tokens_preserved
                    .zip(agent_visible_tokens)
                    .map(|(artifact, visible)| artifact.saturating_sub(visible))
            });
        let metrics_source = token_events
            .iter()
            .find_map(|event| event.metrics_source.as_deref())
            .unwrap_or("estimated");
        let saved_tokens = sum_token(&token_events, |event| {
            event
                .saved_tokens
                .or(event.cached_tokens)
                .or(event.response_tokens_cached)
        });
        let reduction_percent = token_events
            .iter()
            .find_map(|event| event.reduction_percent);

        let consumption = if let Some(total) = total_tokens {
            let mut parts = vec![format!("total {total} tokens")];
            if let Some(input) = input_tokens {
                parts.push(format!("input {input}"));
            }
            if let Some(output) = output_tokens {
                parts.push(format!("output {output}"));
            }
            parts.join("; ")
        } else if let Some(context) = context_tokens {
            format!("retrieved workspace context {context} tokens")
        } else if let Some(query_result) = query_result_tokens {
            format!("query-result data {query_result} tokens recorded")
        } else if let Some(visible) = agent_visible_tokens {
            format!(
                "estimated agent-visible output {visible} tokens ({metrics_source}; not measured model usage)"
            )
        } else if let Some(artifact) = artifact_tokens_preserved {
            format!(
                "estimated artifact footprint {artifact} tokens preserved outside chat ({metrics_source}; not measured model usage)"
            )
        } else {
            "recorded, but no total/input/output token fields were present".to_owned()
        };

        let savings = if let (Some(cached), Some(result)) = (
            sum_token(&token_events, |event| event.response_tokens_cached),
            query_result_tokens,
        ) {
            let saved = cached.saturating_sub(result);
            match reduction_percent {
                Some(percent) => format!(
                    "{saved} tokens saved by query reduction ({cached} cached response tokens reduced to {result} query-result tokens, {percent:.1}% reduction)"
                ),
                None => format!(
                    "{saved} tokens saved by query reduction ({cached} cached response tokens reduced to {result} query-result tokens)"
                ),
            }
        } else {
            match (saved_tokens, reduction_percent) {
                (Some(saved), Some(percent)) => {
                    format!("{saved} tokens saved or cached; {percent:.1}% reduction")
                }
                (Some(saved), None) => format!("{saved} tokens saved or cached"),
                (None, Some(percent)) => format!("{percent:.1}% reduction recorded"),
                (None, None) => {
                    if let Some(avoided) = avoided_output_tokens {
                        match (artifact_tokens_preserved, agent_visible_tokens) {
                            (Some(artifact), Some(visible)) => format!(
                                "estimated {avoided} tokens kept out of chat ({artifact} artifact tokens preserved; {visible} agent-visible tokens; source: {metrics_source})"
                            ),
                            _ => format!(
                                "estimated {avoided} tokens kept out of chat (source: {metrics_source})"
                            ),
                        }
                    } else if let Some(artifact) = artifact_tokens_preserved {
                        format!(
                            "estimated {artifact} artifact tokens preserved outside chat (source: {metrics_source})"
                        )
                    } else {
                        "not recorded".to_owned()
                    }
                }
            }
        };

        let evidence = token_events
            .iter()
            .map(|event| {
                let mut label = event.event.clone();
                if let Some(workspace) = &event.workspace {
                    label.push_str(&format!(" in workspace {workspace}"));
                }
                if let Some(reference) = event.short_ref() {
                    label.push_str(&format!(" ({reference})"));
                }
                label
            })
            .collect();

        AlignTokenSummary {
            consumption,
            savings,
            evidence,
        }
    }
}

impl ExecutionEvent {
    pub(super) fn from_value(value: &serde_json::Value) -> Self {
        Self {
            event: string_field(value, "event").unwrap_or_else(|| "unknown".to_owned()),
            phase: string_field(value, "phase"),
            requirement: string_field(value, "requirement"),
            command: command_field(value),
            executor: string_field(value, "executor"),
            through_rote: bool_field(value, "through_rote"),
            operation_kind: string_field(value, "operation_kind"),
            execution_mode: string_field(value, "execution_mode"),
            workspace: string_field(value, "workspace"),
            response_id: string_field(value, "response_id"),
            lease_id: string_field(value, "lease_id"),
            exit_code: i64_field(value, "exit_code"),
            timed_out: bool_field(value, "timed_out"),
            stdout_captured: bool_field(value, "stdout_captured"),
            stderr_captured: bool_field(value, "stderr_captured"),
            ready: bool_field(value, "ready"),
            anonymous: bool_field(value, "anonymous"),
            included_result: bool_field(value, "included_result"),
            included_alignment: bool_field(value, "included_alignment"),
            included_evidence: bool_field(value, "included_evidence"),
            included_token_savings: bool_field(value, "included_token_savings"),
            fallback_needed: bool_field(value, "fallback_needed"),
            matches_len: value
                .get("matches")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            id: string_field(value, "id").or_else(|| string_field(value, "obligation_id")),
            total_tokens: u64_field(value, "total_tokens"),
            input_tokens: u64_field(value, "input_tokens"),
            output_tokens: u64_field(value, "output_tokens"),
            prompt_tokens: u64_field(value, "prompt_tokens"),
            completion_tokens: u64_field(value, "completion_tokens"),
            context_tokens: u64_field(value, "context_tokens"),
            query_result_tokens: u64_field(value, "query_result_tokens"),
            cached_tokens: u64_field(value, "cached_tokens"),
            response_tokens_cached: u64_field(value, "response_tokens_cached"),
            saved_tokens: u64_field(value, "saved_tokens"),
            reduction_percent: f64_field(value, "reduction_percent"),
            agent_visible_tokens: u64_field(value, "agent_visible_tokens"),
            artifact_tokens_preserved: u64_field(value, "artifact_tokens_preserved"),
            avoided_tokens: u64_field(value, "avoided_tokens"),
            metrics_source: string_field(value, "metrics_source"),
        }
    }

    pub(super) fn uses_rote_exec(&self) -> bool {
        self.through_rote == Some(true)
            || self
                .executor
                .as_deref()
                .is_some_and(|executor| matches!(executor, "rote_exec" | "rote"))
    }

    pub(super) fn is_direct_process(&self) -> bool {
        self.through_rote == Some(false)
            || self.executor.as_deref().is_some_and(|executor| {
                matches!(executor, "direct_harness" | "direct_cli" | "direct_shell")
            })
    }

    pub(super) fn short_ref(&self) -> Option<String> {
        self.response_id
            .as_ref()
            .map(|id| format!("response {id}"))
            .or_else(|| self.lease_id.as_ref().map(|id| format!("lease {id}")))
    }

    pub(super) fn has_token_fields(&self) -> bool {
        self.total_tokens.is_some()
            || self.input_tokens.is_some()
            || self.output_tokens.is_some()
            || self.prompt_tokens.is_some()
            || self.completion_tokens.is_some()
            || self.context_tokens.is_some()
            || self.query_result_tokens.is_some()
            || self.cached_tokens.is_some()
            || self.response_tokens_cached.is_some()
            || self.saved_tokens.is_some()
            || self.reduction_percent.is_some()
            || self.agent_visible_tokens.is_some()
            || self.artifact_tokens_preserved.is_some()
            || self.avoided_tokens.is_some()
    }
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(serde_json::Value::as_bool)
}

fn i64_field(value: &serde_json::Value, key: &str) -> Option<i64> {
    value.get(key).and_then(serde_json::Value::as_i64)
}

fn u64_field(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
    })
}

fn f64_field(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(serde_json::Value::as_f64)
}

fn sum_token(
    events: &[&ExecutionEvent],
    field: impl Fn(&ExecutionEvent) -> Option<u64>,
) -> Option<u64> {
    let mut saw = false;
    let mut total = 0_u64;
    for event in events {
        if let Some(value) = field(event) {
            saw = true;
            total = total.saturating_add(value);
        }
    }
    saw.then_some(total)
}

fn command_field(value: &serde_json::Value) -> Option<String> {
    string_field(value, "command")
        .or_else(|| string_field(value, "program"))
        .or_else(|| {
            value
                .get("invocation")
                .and_then(|invocation| string_field(invocation, "program"))
        })
        .and_then(|command| sanitize_command(&command))
}

fn sanitize_command(command: &str) -> Option<String> {
    let token = command.split_whitespace().next()?.trim();
    if token.is_empty() {
        return None;
    }
    Path::new(token)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .or_else(|| Some(token.to_owned()))
}
