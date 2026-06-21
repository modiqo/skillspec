# repo readiness

Check a local repository against remote state, CI, contributors, and public profile context.

This document is a complete Markdown rendering of the SkillSpec behavioral contract.

## Runtime Contract

- Read this generated skill for orientation and immediate rules.
- Treat routes, rules, states, commands, tests, and review notes below as authoritative.
- Route `handoff` entries are hard execution boundaries, not prose. If a selected route has `handoff.boundary: stop_current_skill`, stop current-skill execution except to pass the declared context to the target skill.
- Rules beat prose when there is tension.
- `forbid` entries are hard negative steering, not suggestions.
- `elicit` entries require bounded user questions before guessing.
- Use the scenario tests as examples of expected behavior.
- When unfamiliar with the spec shape, run `skillspec sensemake <skill-folder>/skill.spec.yml --view index` to get section roles, counts, ids, and query commands without consuming the whole spec.
- When the `skillspec` CLI is available, prefer `skillspec decide` or `skillspec explain` over manual interpretation.
- After `skillspec decide`, inspect matched rules and active execution surfaces with `skillspec query <skill-folder>/skill.spec.yml <handle> --view summary` and `skillspec refs <skill-folder>/skill.spec.yml <handle> --view summary` instead of ad hoc YAML queries.
- Escalate query detail from `--view index` to `--view summary` to `--view full` only when the smaller view cannot answer the decision.
- When invoking `skillspec decide`, pass only the user's task text. Strip skill invocation prefixes such as `/rote-shell-spec`, `$rote-shell-spec`, or `/my-skill` before setting `--input`.
- Prefer `--input='<task text>'` in shell examples so `$skill-name` text is not expanded by the shell.
- Resolve `skill.spec.yml` relative to this `SKILL.md` folder, not the process working directory.
- Always pass `--trace-dir`; use `${PWD}/.skillspec/traces` unless the user or harness provides a run-specific trace directory.
- After `skillspec decide` prints trace lines, keep the emitted `run_dir` and mention it when reporting how the decision was made.
- When the CLI is available, run `skillspec trace align <skill-folder>/skill.spec.yml --decision-trace <run_dir>` and add `--execution-trace <jsonl>` when structured action evidence exists. Include the alignment status, status meaning, decision-replay and execution-proof layer results, evidence gaps, user-facing proof rows, summary, and any failed/unproven checks in the completion report.
- When rote workspace evidence or stats exist, make the completion report user-facing with a visible `Token savings` section: name the workspace and response ids/files, describe the workspace as a retrievable context file system, report measured context-window/API tokens only when queried, and explain crystallized/remembered reuse as avoiding full evidence reloads. Do not reduce this to a bare token count.
- Alignment proof rows may mention command basenames such as `gh` or `git`, but must not include raw command arguments because args may contain private data.

Minimum final response shape when workspace evidence exists:

- `Result`: answer the user's task directly.
- `Evidence`: workspace name plus important response ids/files the user can query later.
- `Token savings`: state measured context-window/API tokens when available; otherwise say savings are structurally available but not measured. Explain that full evidence is outside the prompt in the rote workspace and can be retrieved by id/file instead of reloaded into context.
- `SkillSpec`: selected route, trace run directory, alignment status, evidence gaps, and proof rows that map request/spec obligations to observed evidence. Never let this replace the Result, Evidence, or Token savings sections.

## Routes

Try lower-rank routes first unless matching rules override the route or route order.

### `remembered_route`
- label: Use saved repo readiness route
- rank: 10

### `local_cli`
- label: Use local git and gh CLI
- rank: 20

### `tracked_background`
- label: Run long local checks as a tracked background job
- rank: 25

### `connected_service`
- label: Use GitHub service connection
- rank: 30

### `browser`
- label: Use browser for public profile extraction
- rank: 40

## Rules

Evaluate rules in order. A matching rule may choose a route, replace route order, forbid substitutions, allow narrow fallbacks, request bounded elicitation, and schedule post-success actions.

### `local_repo_state_uses_cli`
- when:
  - user_says_any: "branch", "in sync", "local repo"
- prefer: `local_cli`
- reason: Local branch state lives on this computer.

### `browse_profiles_uses_browser`
- when:
  - user_says_any: "browse profiles", "social profile", "public profile"
- prefer: `browser`
- forbid: `native_search_as_answer`, `raw_playwright`
- allow:
  - `native_search`: "url_discovery_only"
- reason: Profile extraction must observe pages rather than summarize search results.

### `repeated_readiness_can_be_saved`
- when:
  - task_recurrence_likely: true
- after_success: `collect_trace_cost`, `ask_to_remember`

### `long_checks_use_tracked_background`
- when:
  - command_likely_long_running: true
- prefer: `tracked_background`
- after_success: `wait_for_background_job`, `summarize_streams`
- reason: Long-running checks should be leased, observable, and joined instead of blocking the agent blindly.

### `auth_prompts_use_terminal_or_browser`
- when:
  - interactive_prompt_likely: true
- route_order: `browser` -> `local_cli` -> `connected_service`
- forbid: `background_without_human_confirm`, `hidden_credential_prompt`
- reason: Interactive auth should stay visible and bounded.

## Dependencies

Check declared dependencies before using commands that require them. Missing dependencies must be handled through the declared provision or elicitation path; do not silently install global tools.

### `cargo`
- kind: `cli`
- description: Cargo is required for Rust repository checks and tests.
- command: `cargo`
- check:
  - command: `cargo`
- permission:
  - required: true
  - reason: Cargo commands may execute project build scripts and tests.
  - safety: `local_read`

### `gh`
- kind: `cli`
- description: GitHub CLI is required for authenticated remote repository inspection.
- command: `gh`
- check:
  - command: `gh`
- permission:
  - required: true
  - reason: GitHub CLI may use authenticated account state and network access.
  - safety: `network_read`
- provision:
  - options:
    - `user_global`: Install GitHub CLI with a user package manager
      command: `brew install gh`
      safety: `local_write`

### `git`
- kind: `cli`
- description: Git is required for local repository branch and sync state.
- command: `git`
- check:
  - command: `git`

### `rote`
- kind: `cli`
- description: Rote CLI is required for tracked background jobs, process streams, and dependency traces.
- command: `rote`
- check:
  - command: `rote`

## State Machine

Use states as lifecycle guidance. State actions must reference commands or closures; snippets supply user-facing copy.

### `inspect_repo`
- do: `git_status`, `gh_repo`
- next: `maybe_long_checks`

### `maybe_long_checks`
- do: `cargo_tests_background`
- next: `report`

### `report`
- do: `collect_trace_cost`
- say: `readiness_report`

### `start`
- do: `choose_route`
- next: `inspect_repo`

## Command Templates

Command templates are examples and contracts, not automatic permission. Apply the safety class and the harness approval policy before executing.

### `cargo_tests_background`
- description: Run cargo tests as a tracked background job and observe stderr/stdout through process streams.
- safety: `local_read`
- template:

```bash
rote exec --background --stdout-file logs/cargo-test.stdout.log --stderr-file logs/cargo-test.stderr.log -- cargo test --features test-helpers
```
- requires:
  - dependencies: `cargo`, `rote`

### `collect_trace_cost`
- description: Collect work cost after completion.
- safety: `read_only`
- template:

```bash
rote trace --deps --format json
```
- requires:
  - dependencies: `rote`

### `gh_repo`
- description: Inspect remote repository state through GitHub CLI.
- safety: `network_read`
- template:

```bash
gh repo view --json nameWithOwner,defaultBranchRef,pushedAt
```
- requires:
  - dependencies: `gh`

### `git_status`
- description: Check local branch state.
- safety: `local_read`
- template:

```bash
git status --short --branch
```
- requires:
  - dependencies: `git`

### `summarize_streams`
- description: Read tracked stdout and stderr artifacts rather than terminal scrollback.
- safety: `read_only`
- template:

```bash
rote stream follow-process <lease> --stream stderr --from-start --max-bytes 65536
```
- requires:
  - dependencies: `rote`

### `wait_for_background_job`
- description: Join a tracked finite job before making completion claims.
- safety: `read_only`
- template:

```bash
rote exec wait <lease> --timeout-ms 600000 --poll-ms 1000
```
- requires:
  - dependencies: `rote`

## Snippets

### `readiness_report`
Show the outcome, cost, recurrence estimate, and ask whether to remember the route.

## Closures

Closures are post-task obligations or named lifecycle actions. Run them when referenced by states or `after_success`.

### `ask_to_remember`
```yaml
description: Ask whether to save this readiness route for repeated use.
```

### `choose_route`
```yaml
description: Pick the route after evaluating local CLI, GitHub connection, browser need, and saved route availability.
```

## Scenario Tests

Use these as behavioral examples. The agent should make the same routing and guardrail choices for equivalent tasks.

### local sync check prefers CLI
- input: "check whether this branch is in sync with remote"
- expect route: `local_cli`

### public profile browsing uses browser
- input: "browse recent committers social profiles"
- expect route: `browser`
- expect forbid: `native_search_as_answer`

### long cargo checks use tracked background
- input: "run cargo test and check gh status while it runs"
- expect route: `tracked_background`
- expect after_success: `wait_for_background_job`, `summarize_streams`

### auth prompts stay visible
- input: "login to gh with browser auth"
- expect route_order: `browser` -> `local_cli` -> `connected_service`
- expect forbid: `hidden_credential_prompt`

## SkillSpec CLI Commands

Use these commands when the `skillspec` CLI is available. Replace `<skill-folder>` with the folder containing this generated `SKILL.md`. The default trace location is `${PWD}/.skillspec/traces`, where `PWD` is the task working directory.

```bash
skillspec sensemake <skill-folder>/skill.spec.yml --view index
skillspec validate <skill-folder>/skill.spec.yml
skillspec imports check <skill-folder>/skill.spec.yml
skillspec test <skill-folder>/skill.spec.yml
skillspec deps check <skill-folder>/skill.spec.yml
skillspec deps check <skill-folder>/skill.spec.yml --command <command-id>
skillspec query <skill-folder>/skill.spec.yml rule:<rule-id> --view summary
skillspec refs <skill-folder>/skill.spec.yml rule:<rule-id> --view summary
skillspec query <skill-folder>/skill.spec.yml command:<command-id>.requires
skillspec decide <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec explain <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec trace compact "${PWD}/.skillspec/traces/<run-id>"
skillspec trace align <skill-folder>/skill.spec.yml --decision-trace "${PWD}/.skillspec/traces/<run-id>" --execution-trace <execution-ledger.jsonl>
```
