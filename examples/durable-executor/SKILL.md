---
name: durable-executor
description: "Universal first-hop for any tool-backed request that may execute, inspect, mutate, automate, browse, call, fetch, generate, install, test, run, or create artifacts through CLI, shell, API, adapter, browser, provider, local process, or external tool, with trace, alignment, evidence capture, and future recall. Use for any tool-backed request, any and all tool execution, all tool execution starts here, universal cli api shell browser provider router, universal tool-backed execution router, execute inspect mutate automate browse call fetch generate install test run create artifacts, create an artifact with a tool, generate a file with a tool, create audio, create a voice message, generate audio and text to speech. Use when the task needs to remember this workflow for future recall, preserve evidence with trace and alignment, route durable work through a meta-router"
---

# durable executor

Universal durable-work meta-router and first-hop execution substrate for any tool-backed CLI/API/shell/browser/provider/local-process/artifact work that should be traced, aligned, gated, or reusable.

## Entry Gate

- Before any task action, run `skillspec plan ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"`, preserve the printed `run_dir`, then run `skillspec act ./skill.spec.yml --input='<user task>' --run <run_dir> --phase <phase-id>`, and read the ordered phase plan plus current-route action checklist.
- Until that plan and checklist are read, the only allowed actions are loading this `SKILL.md`, loading the colocated `skill.spec.yml`, and running SkillSpec navigation or decision commands for this spec.
- The selected route and matched rules in the checklist override lower-level skill defaults. If a tool is forbidden, stop and report that the SkillSpec blocks it.
- After each phase action, record structured progress in `<run_dir>/execution.jsonl` and run `skillspec progress show ./skill.spec.yml --run <run_dir>` before moving to the next phase.
- This SkillSpec supersedes overlapping lower-level skill instructions: browser:browser.
- Forbidden before the decision: raw_repo_search, direct_file_read, direct_cli_without_rote_exec, direct_shell_command_without_rote_exec, direct_harness_cli_call_without_rote_exec, browser_browser_skill, node_repl.

This skill is a thin loader for the colocated `skill.spec.yml`. The spec is the source of truth for routes, rules, dependencies, imports, resources, recipes, tests, and trace requirements. Do not treat the spec as background prose; treat it as the execution contract for this task.

## Runtime Contract

1. Load `./skill.spec.yml` from this skill folder before taking task actions.
2. When the `skillspec` CLI is available and the spec shape is unfamiliar, run `skillspec sensemake ./skill.spec.yml --view index` to learn the section roles, counts, query handles, and navigation grammar without dumping the full YAML.
3. Then create the ordered phase plan and current-route action checklist:

   ```bash
   skillspec plan ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
   skillspec act ./skill.spec.yml --input='<user task>' --run <run_dir> --phase <phase-id>
   ```

4. Strip skill invocation prefixes such as `/my-skill`, `$my-skill`, or `/durable-executor-spec` before passing `--input`.
5. Preserve the emitted trace `run_dir`.
6. Read the full phase plan and action checklist before using tools. Treat them as the active execution SOP, not as advice. The `PHASE TOOL BOUNDARY - HARD` section is the permission boundary for the next action.
7. For each execution phase, run `skillspec act ./skill.spec.yml --input='<user task>' --run <run_dir> --phase <phase-id>` before acting, record phase progress in `<run_dir>/execution.jsonl`, then run `skillspec progress show ./skill.spec.yml --run <run_dir>` to see completed, current, blocked, and remaining phases.
8. Pull active details with `skillspec query ./skill.spec.yml <handle> --view summary` and relationship edges with `skillspec refs ./skill.spec.yml <handle> --view summary`. Prefer precise handles such as `rule:<id>`, `rule:<id>.forbid`, `command:<id>.requires`, and `state:<id>.next` over reading the whole spec.
9. Before every substrate/tool call, apply the phase tool boundary and checklist allow/deny questions. Any unlisted tool, data source, execution substrate, provider, adapter, CLI, browser mode, API, or skill requires explicit user permission before use. The selected route and matched rules override lower-level skill defaults and generic tool preferences.
10. When the CLI is available after a trace exists, run `skillspec trace align ./skill.spec.yml --decision-trace <run_dir>` and, when structured action evidence exists, add `--execution-trace <run_dir>/execution.jsonl`. The command writes `<run_dir>/alignment.json`; report the alignment status, meaning, model layers, evidence gaps, user-facing proof rows, summary, and trace path.
11. If `skillspec plan`, `skillspec act`, or `skillspec progress` is unavailable, fall back to `skillspec decide`, then manually construct the same ordered phase checklist and progress notes before using tools. If the CLI is unavailable, read `skill.spec.yml` directly and apply the same contract manually. Do not expand this loader into a second source of truth.

## Harness Presentation Contract

- When presenting plan, action, progress, command, recipe, or closure steps to a user, show the step `description` as the default visible text. If no description is present, show a humanized id.
- Keep raw command templates, concrete argv, provider payloads, and low-level tool details collapsed by default in normal progress UI. Reveal them only when the user explicitly expands details, approval is required, a command fails, debug/verbose mode is active, or no usable description exists.
- For approval prompts, destructive or externally mutating actions, and failure reports, show both the human description and the raw command or payload summary needed for informed approval/debugging.
- This is presentation-only. Always preserve raw command templates, concrete executed commands, stdout/stderr handles, response ids, and files in trace/evidence/alignment data exactly as required by the active SkillSpec.

## Authoring And Revision Contract

When importing, creating, revising, or extending this SkillSpec-backed skill, use the embedded grammar teacher before editing `skill.spec.yml`:

```bash
skillspec grammar sensemake --view index
skillspec grammar sensemake --view porting
skillspec grammar checklist --for import-skill
```

- Treat the checklist as the review gate for semantic edits: activation, routes, rules, elicitations, imports/resources, commands/deps, procedures, tests, proof, and contract quality.
- Fill or update a coverage matrix with `prose_span | obligation | skillspec_construct | confidence | status | review_note` before installing or releasing a changed skill.
- Use `skillspec grammar schema --json` when a harness needs the exact embedded JSON schema.
- Do not patch YAML by memory when the binary can teach the current grammar. Run the grammar commands again after CLI upgrades or when a spec shape is unfamiliar.

## Durable Handoff Contract

This skill participates in agent-mediated durable execution. There is no runtime handoff engine: the agent reads the active SkillSpec contracts, carries the handoff packet in context, and preserves the declared evidence.

- If a durable handoff packet is present, preserve its `workspace`, `trace_dir`, `return_to`, `branch_id`, and `execution_policy` fields.
- If no durable handoff packet is present and the task asks for remembered evidence, future recall, reuse, trace, alignment, or durable execution, route through `durable-executor` first unless the user explicitly requests direct/no-rote execution.
- If `durable_context.active` is true, do not route the whole task back to `durable-executor`; use `durable-executor` only as the execution substrate and then return to `return_to`.
- This skill owns its domain interpretation and validation. `durable-executor` owns workspace, trace, evidence, command substrate, final alignment, token-savings, and recall/crystallization closure when it initiated the handoff.
- Any CLI, shell command, local process, package command, API fallback, or provider command must use the durable execution substrate, normally a rote adapter or `rote exec --`, unless the active spec or user explicitly allows direct execution.
- On completion, emit a return packet with status, selected route, skill metadata, artifacts, evidence handles, blockers, and trace paths, then hand back to `return_to` for final closure.
- For parallel work, keep one top-level workspace but use branch-scoped `branch_id`, trace paths, evidence labels, and artifact directories.

## How To Execute The Structure

Before the first task action, use `skillspec plan` and `skillspec act` to convert the decision output and relevant spec sections into an ordered phase plan plus a current-route OODA checklist:

- `route`: the selected route is the strategy to use. If no route is selected, stop and ask for the missing task shape instead of inventing a fallback.
- execution plan: if the selected route has `execution_plan`, execute its phases in order before using any tool outside the current phase. A later handoff phase does not license skipping an earlier shell or adapter phase. If a phase declares `jumps`, take the first matching jump condition and continue at the named phase.
- phase tool boundary: `skillspec act` renders the effective `tool_boundary` inherited from entry, route, and phase. Treat it as hard. If a needed tool or substrate is not listed, stop and ask permission before using it.
- route handoff: if the selected route has `handoff`, treat it as a hard execution boundary. Follow the handoff target and boundary before using tools from the current skill; `stop_current_skill` means do not continue current-skill execution except to pass the declared context.
- `matched_rules`: these are active obligations, not explanatory decoration. Use each rule's `reason`, `prefer`, `forbid`, `elicit`, and `after_success` fields to constrain the next action.
- `forbid`: forbids are hard negative constraints on behavior. They block substitutions even when a convenient tool is available. If a forbidden action seems necessary, stop and ask for explicit user approval or a different route; do not silently do it.
- user constraints: carry explicit user instructions into the same checklist. The spec adds structure; it does not erase the user's constraints.
- `elicit`: ask the required question before irreversible work, side effects, installs, auth steps, or broad exploration.
- `dependencies`: prove readiness for the active route, command, recipe, or code block before using it. Prefer command-scoped checks such as `skillspec deps check ./skill.spec.yml --command <id>` when a command id is known.
- dependency evidence: a missing environment variable only proves that variable is absent; it does not prove that auth, API keys, browser sessions, keychains, vaults, or CLI-native credentials are absent. When auth can live outside env, prove readiness with the declared command, adapter, browser, or dependency check instead of grepping env.
- `imports` and `resources`: load only the items required by the active route/rule/recipe/code, plus anything marked `always`.
- `commands`, `recipes`, and `code`: use declared templates and ordered steps as the allowed execution surface. Check their `requires` fields first, preserve outputs as evidence, and do not replace them with unrelated tools unless the active contract allows that substitution.
- `after_success` and closures: these are completion obligations. Do them before the final response, or report why they remain unproven.

Repeat the checklist before every tool call. If a lower-level skill or generic tool default conflicts with the selected route, follow the selected route. If the next tool is forbidden, stop and report that the SkillSpec blocks it.

If every allowed route is blocked by missing dependencies, auth, permissions, or a forbid, report the blocker and ask how to proceed. Do not switch to native search, raw shell, browser automation, direct API calls, or installs just because they are available in the harness.

## Quick Commands

```bash
skillspec sensemake ./skill.spec.yml --view index
skillspec plan ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec act ./skill.spec.yml --input='<user task>' --run "${PWD}/.skillspec/traces/<run-id>" --phase <phase-id>
skillspec progress record "${PWD}/.skillspec/traces/<run-id>" phase-completed <phase-id> --evidence-kind rote_response --evidence-ref <ref>
skillspec progress stats "${PWD}/.skillspec/traces/<run-id>" --workspace <rote-workspace> --workspace-stats-report "${PWD}/.skillspec/traces/<run-id>/workspace-stats.txt" --phase <phase-id> --requirement <stats-requirement-id>
skillspec progress final-response "${PWD}/.skillspec/traces/<run-id>" --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings
skillspec progress show ./skill.spec.yml --run "${PWD}/.skillspec/traces/<run-id>"
skillspec validate ./skill.spec.yml
skillspec imports check ./skill.spec.yml
skillspec test ./skill.spec.yml
skillspec deps check ./skill.spec.yml
skillspec query ./skill.spec.yml rule:<id> --view summary
skillspec refs ./skill.spec.yml rule:<id> --view summary
skillspec query ./skill.spec.yml command:<id>.requires
skillspec decide ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec explain ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec trace align ./skill.spec.yml --decision-trace "${PWD}/.skillspec/traces/<run-id>" --execution-trace "${PWD}/.skillspec/traces/<run-id>/execution.jsonl"
```

## Completion Report

When reporting completion, always include the selected route, the SkillSpec trace `run_dir`, the persisted `<run_dir>/alignment.json`, and the compact `skillspec trace align` completion summary. Do not report a bare `unproven`; if alignment is incomplete, use `Alignment: partial` plus specific `Missing proof` rows from the align output. Command proof must name only the command basename, never raw args.

Always include token usage. For successful rote-backed runs, collect `rote workspace stats <workspace>` into a report file and run `skillspec progress stats <run_dir> --workspace <workspace> --workspace-stats-report <file> --phase <phase-id> --requirement <stats-requirement-id>` before alignment; missing `stats_collected` evidence is a workflow bug, not a normal omission. Draft the final response with Result, Evidence, Alignment summary, Token usage, and SkillSpec sections, run `skillspec progress final-response <run_dir> --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings`, then rerun `skillspec trace align` and report that final alignment. If stats truly cannot be collected, write `Token consumption: not recorded` and `Token savings: not recorded`; do not invent savings. When query-reduction stats exist, state the cached response tokens, extracted query-result tokens, saved-token delta, and reduction percentage. When rote workspace stats exist, include measured context-window/API tokens and explain that full evidence is outside the prompt in the workspace and can be retrieved by id/file instead of reloaded into context.

Minimum final response shape:

- `Result`: answer the user's task directly.
- `Evidence`: workspace name plus important response ids/files the user can query later.
- `Alignment summary`: include `Decision replay`, `Phase order`, `Requirements`, one or more `Missing proof` rows, `Forbidden actions`, and `Alignment` exactly as reported by `skillspec trace align`.
- `Token usage`: include `Token consumption` and `Token savings` exactly as reported by `skillspec trace align`; say `not recorded` when absent.
- `SkillSpec`: selected route, trace run directory, align status, status meaning, and proof rows that map request/spec obligations to observed evidence. Never let this replace the Result, Evidence, Alignment summary, or Token usage sections.

## Route Hints

- `durable_domain_handoff`: Hand off domain work and return for durable closure
- `capability_bootstrap`: Bootstrap capability when no domain SkillSpec exists
- `shell_then_browser_handoff`: Run shell evidence, then hand off browser work
- `adapter_first_cli_fallback`: Use rote adapters, then rote exec CLI fallback
- `browser_handoff`: Hand off to rote-browse for browser state
- `one_shot_process`: Capture a one-shot process
- `declared_file_io`: Capture declared file inputs or outputs
- `stream_follow`: Follow a moving file or process stream
- `background_process`: Start and track a background process lease
- `pty_transcript`: Capture a one-shot PTY transcript
- `dependency_preflight`: Check dependencies before replay or release
- `crystallized_flow`: Crystallize or replay a shell flow
- `raw_shell`: Use raw shell for disposable inspection only
