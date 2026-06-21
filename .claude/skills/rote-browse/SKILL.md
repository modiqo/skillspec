---
name: rote-browse
description: "Use when the task needs to browse a website with rote, attach to an active browser, inspect logged-in web app state, snapshot or slice a page, click or type in a page, recover a stale browser ref or page lease and crystallize browser exploration into a reusable flow. Handles browser handoff and page evidence and dependency checks. Preserves evidence with SkillSpec routes, forbids, dependencies, traces, and token-savings reports"
---

# rote browse

Browser automation through rote with page leases, snapshots, slices, attach-existing sessions, auth gates, and cleanup.

This skill is a thin loader for the colocated `skill.spec.yml`. The spec is the source of truth for routes, rules, dependencies, imports, resources, recipes, tests, and trace requirements. Do not treat the spec as background prose; treat it as the execution contract for this task.

## Runtime Contract

1. Load `./skill.spec.yml` from this skill folder before taking task actions.
2. When the `skillspec` CLI is available and the spec shape is unfamiliar, run `skillspec sensemake ./skill.spec.yml --view index` to learn the section roles, counts, query handles, and navigation grammar without dumping the full YAML.
3. Then run:

   ```bash
   skillspec decide ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
   ```

4. Strip skill invocation prefixes such as `/my-skill`, `$my-skill`, or `/rote-shell-spec` before passing `--input`.
5. Preserve the emitted trace `run_dir`.
6. Read the decision JSON before using tools. Do not act from route labels alone.
7. Pull active details with `skillspec query ./skill.spec.yml <handle> --view summary` and relationship edges with `skillspec refs ./skill.spec.yml <handle> --view summary`. Prefer precise handles such as `rule:<id>`, `rule:<id>.forbid`, `command:<id>.requires`, and `state:<id>.next` over reading the whole spec.
8. Materialize the active contract described below, then execute only actions that satisfy it.
9. When the CLI is available after a trace exists, run `skillspec trace align ./skill.spec.yml --decision-trace <run_dir>` and, when structured action evidence exists, add `--execution-trace <jsonl>`. Report the alignment status, meaning, model layers, evidence gaps, user-facing proof rows, summary, and trace path.
10. If the CLI is unavailable, read `skill.spec.yml` directly and apply the same contract manually. Do not expand this loader into a second source of truth.

## How To Execute The Structure

Before the first task action, convert the decision output and relevant spec sections into a checklist:

- `route`: the selected route is the strategy to use. If no route is selected, stop and ask for the missing task shape instead of inventing a fallback.
- route handoff: if the selected route has `handoff`, treat it as a hard execution boundary. Follow the handoff target and boundary before using tools from the current skill; `stop_current_skill` means do not continue current-skill execution except to pass the declared context.
- `matched_rules`: these are active obligations, not explanatory decoration. Use each rule's `reason`, `prefer`, `forbid`, `elicit`, and `after_success` fields to constrain the next action.
- `forbid`: forbids are hard negative constraints on behavior. They block substitutions even when a convenient tool is available. If a forbidden action seems necessary, stop and ask for explicit user approval or a different route; do not silently do it.
- user constraints: carry explicit user instructions such as "do not search the web" into the same checklist. The spec adds structure; it does not erase the user's constraints.
- `elicit`: ask the required question before irreversible work, side effects, installs, auth steps, or broad exploration.
- `dependencies`: prove readiness for the active route, command, recipe, or code block before using it. Prefer command-scoped checks such as `skillspec deps check ./skill.spec.yml --command <id>` when a command id is known.
- dependency evidence: a missing environment variable only proves that variable is absent; it does not prove that auth, API keys, browser sessions, keychains, vaults, or CLI-native credentials are absent. When auth can live outside env, prove readiness with the declared command, adapter, browser, or dependency check instead of grepping env.
- `imports` and `resources`: load only the items required by the active route/rule/recipe/code, plus anything marked `always`.
- `commands`, `recipes`, and `code`: use declared templates and ordered steps as the allowed execution surface. Check their `requires` fields first, preserve outputs as evidence, and do not replace them with unrelated tools unless the active contract allows that substitution.
- `after_success` and closures: these are completion obligations. Do them before the final response, or report why they remain unproven.

If every allowed route is blocked by missing dependencies, auth, permissions, or a forbid, report the blocker and ask how to proceed. Do not switch to native search, raw shell, browser automation, direct API calls, or installs just because they are available in the harness.

## Quick Commands

```bash
skillspec sensemake ./skill.spec.yml --view index
skillspec validate ./skill.spec.yml
skillspec imports check ./skill.spec.yml
skillspec test ./skill.spec.yml
skillspec deps check ./skill.spec.yml
skillspec query ./skill.spec.yml rule:<id> --view summary
skillspec refs ./skill.spec.yml rule:<id> --view summary
skillspec query ./skill.spec.yml command:<id>.requires
skillspec explain ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec trace align ./skill.spec.yml --decision-trace "${PWD}/.skillspec/traces/<run-id>" --execution-trace <execution-ledger.jsonl>
```

## Completion Report

When reporting completion, include the selected route, the SkillSpec trace `run_dir`, the `skillspec trace align` status (`pass`, `fail`, or `unproven`), status meaning, decision-replay and execution-proof layer results, evidence gaps, align summary/conclusion, and the user-facing alignment proof rows. Command proof must name only the command basename, never raw args. When rote workspace evidence or stats exist, include a visible `Token savings` section: name the workspace and response ids/files the user can retrieve later, state measured context-window/API tokens only if queried, explain that the workspace keeps full evidence outside the prompt, and explain that crystallized or remembered reuse can avoid reloading full evidence into the model window. Do not reduce this to a bare token count or invent replay savings.

Minimum final response shape when workspace evidence exists:

- `Result`: answer the user's task directly.
- `Evidence`: workspace name plus important response ids/files the user can query later.
- `Token savings`: state measured context-window/API tokens when available; otherwise say savings are structurally available but not measured. Explain that full evidence is outside the prompt in the rote workspace and can be retrieved by id/file instead of reloaded into context.
- `SkillSpec`: selected route, trace run directory, alignment status, evidence gaps, and proof rows that map request/spec obligations to observed evidence. Never let this replace the Result, Evidence, or Token savings sections.

## Route Hints

- `attach_existing_session`: Attach to an active browser
- `new_headed_session`: Start a new visible browser
- `new_headless_session`: Start a new headless browser
- `saved_auth_replay`: Use saved browser auth state
- `page_recovery`: Recover browser page or ref state
- `stateful_page_flow`: Work through a stateful page
- `browser_flow_crystallization`: Crystallize browser exploration
