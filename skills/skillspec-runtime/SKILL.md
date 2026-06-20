---
name: skillspec-runtime
description: Use when an agent should use an existing skill.spec.yml to route a task before acting. Loads a SkillSpec, runs `skillspec decide` or `skillspec explain` with a trace directory, obeys routes/rules/forbids/elicitations, executes with the appropriate harness tools, and reports the decision trace plus evidence.
---

# skillspec-runtime

Use this skill when the user provides a `skill.spec.yml`, invokes a
SkillSpec-backed skill, or asks to run a task through a structured SkillSpec
contract.

SkillSpec is not the execution engine. It is the steering contract. Use it to
decide what should happen, what must not happen, what must be asked first, and
what evidence must be reported after the task is done.

## Runtime Stance

Treat `skill.spec.yml` as the decision source of truth.

The prose skill can explain the personality and product feel. The spec decides:

- which route to try first
- which fallback routes are allowed
- which substitutions are forbidden
- which user question must be asked before acting
- which commands or tool families are appropriate
- which completion obligations must happen after success
- where the decision trace is written

Do not bypass the spec because the prose feels obvious. If the spec and prose
conflict, follow the spec and report the mismatch.

## Resolve The Spec

Use this order:

1. If the user gives an explicit `skill.spec.yml` path, use that file.
2. If this skill was invoked from a generated skill folder, use the sibling
   `skill.spec.yml` next to that `SKILL.md`.
3. If a folder was provided, use `<folder>/skill.spec.yml`.
4. If no spec can be resolved, ask for the spec path. Do not search the whole
   home directory.

Pass only the user task text to `--input`. Strip activation prefixes such as
`/rote-shell-spec`, `$rote-shell-spec`, `/my-skill`, or `$my-skill`.

## Decide First

Validate before using a spec:

```bash
skillspec validate path/to/skill.spec.yml
skillspec imports check path/to/skill.spec.yml
skillspec deps check path/to/skill.spec.yml
skillspec deps check path/to/skill.spec.yml --command '<command-id>'
```

Then run a traced decision:

```bash
skillspec decide path/to/skill.spec.yml --input '<task text>' --trace-dir "${PWD}/.skillspec/traces"
```

Use `explain` when the user asks why a route was chosen, when the route is
surprising, or when debugging the spec:

```bash
skillspec explain path/to/skill.spec.yml --input '<task text>' --trace-dir "${PWD}/.skillspec/traces"
```

Preserve the emitted trace `run_dir`. Report it at completion.

## Interpret The Decision

Read the decision as a contract:

- `route` is the chosen strategy.
- `route_order` is the fallback order.
- `matched_rules` are the rules that justify the decision.
- `forbid` entries are hard no-go substitutions.
- `allow` entries are narrow exceptions, not blanket permission.
- `elicit` means ask a bounded user question before executing.
- `commands` are named templates or command families the harness may use.
- `dependencies` are the tools, files, env vars, services, adapters, browsers,
  and packages that commands may require.
- `imports` are runtime-loadable guidance. Load `always` imports before task
  actions. Load `on_demand` imports only when their route, rule, recipe, code
  path, or parent import is active.
- `closures` and `after_success` are completion obligations.

If a decision forbids a tempting shortcut, do not take the shortcut. Example:
if browser work forbids native web search as the answer, use the browser route
or ask for permission to change route.

## Elicitations

When the decision asks for elicitation:

1. Ask the bounded question from the spec.
2. Present the listed choices in plain language.
3. Do not invent a fourth choice unless the spec permits free-form input.
4. After the user answers, continue on the selected route and preserve the
   same trace path in the final report.

Use elicitation for auth choices, browser attach/headless/headed choices,
install decisions, destructive actions, and unclear target systems.

## Execute With The Harness

After route selection, use the appropriate harness tools or local commands.
Check declared dependencies before executing commands that reference them. When
the command id is known, prefer `skillspec deps check <spec> --command <id>` so
unrelated optional dependencies do not block the task. If a dependency is
missing, use the declared provision elicitation or ask the user; do not silently
install a global tool.

Examples:

- adapter/API route: use the selected adapter or flow and persist response ids.
- CLI/process route: run through the project's required command wrapper and
  preserve stdout, stderr, files, leases, and dataflow evidence.
- browser route: use the browser skill/tooling named by the spec; attach or
  spawn according to the selected elicitation.
- mixed route: keep all artifacts in one task workspace when the system
  supports it.

For rote-backed specs, initialize or enter a rote workspace before running
workspace-scoped commands:

```bash
rote init <task-name> --seq --force
eval $(rote cd <task-name>)
```

Then use rote commands so the canvas, command log, query refs, traces, and
dependency graph can be reported at the end.

## Completion Report

End with a compact evidence report:

```text
SkillSpec route: <route>
SkillSpec decision trace: <run_dir>
Evidence: <response ids, files, browser snapshots, logs, or workspace>
Completion obligations: <closures completed or still pending>
```

If the underlying system exposes cost or replay metrics, include them only when
they were actually queried. Do not invent token savings.

## Failure Handling

If validation fails, stop and report the spec error.

If the selected route is unavailable, try only the next allowed route in
`route_order`. If no allowed route remains, ask the user how to proceed.

If a command or tool produces evidence in a workspace, report the exact
workspace and ids. If the evidence is not captured, say so plainly.

## Done Definition

The runtime use is complete when:

- the spec was validated
- dependencies were checked before command use
- a traced decision was recorded
- the chosen route or allowed fallback was followed
- required elicitations were honored
- forbidden substitutions were avoided
- task evidence was captured
- the final answer includes the trace path and evidence references
