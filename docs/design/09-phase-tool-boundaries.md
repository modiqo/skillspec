# Phase Tool Boundaries

Phase tool boundaries are the runtime permission contract rendered by
`skillspec act`.

They answer this question:

```text
What tools, data sources, substrates, providers, adapters, APIs, browser modes,
CLIs, or skills may the agent use for the next action?
```

The answer is deliberately conservative. If the next action is not allowed by
the active boundary, the agent should stop and ask for explicit permission.

## Why Boundaries Are Needed

Routes and rules can forbid broad categories of behavior, but agents also need a
phase-local execution surface.

For example, a route might require:

- shell facts through one substrate;
- browser work through a different skill;
- no native web search;
- no raw shell;
- no install until dependencies are reviewed.

If the agent only remembers the route label, it may still reach for a convenient
generic tool. A phase boundary makes the allowed surface visible immediately
before action.

The boundary is rendered inside the action checklist as:

```text
PHASE TOOL BOUNDARY - HARD
- default: deny
- allowed: ...
- forbidden: ...
- permission required for: ...
- instruction: ...
```

This is not explanatory decoration. It is the permission boundary for the next
action.

## Grammar Surface

`tool_boundary` may be declared on:

- `entry`;
- a route;
- an execution phase.

The model includes:

- `default`;
- `allow`;
- `forbid`;
- `permission_required_for`.

`default` is currently `allow` or `deny`.

`allow`, `forbid`, and `permission_required_for` are string lists. They are
contract labels, not executable policy code. Harnesses can map them to concrete
tool ids, adapter names, product permissions, or approval prompts.

## Effective Boundary

`skillspec act` computes an effective boundary by merging:

1. runtime defaults;
2. `entry.tool_boundary`;
3. selected route `tool_boundary`;
4. current phase `tool_boundary`;
5. active forbids from decision rules, phases, and handoffs.

The runtime default is:

```text
default: deny
allowed:
  - skillspec_cli
  - current_phase_owner_skill
  - declared_commands_dependencies_imports_resources
  - local_files_referenced_by_active_spec
permission_required_for:
  - any_unlisted_tool
  - any_forbidden_action
  - any_new_data_source
  - any_new_execution_substrate
  - any_new_provider_or_adapter
  - any_external_side_effect
```

This default means the agent can navigate the active SkillSpec and use material
declared by the active spec. It cannot silently use unrelated harness tools or
new data sources.

## Generic Permission Language

The boundary must be generic. It should not say only "web data" or name one
provider. The hard rule is:

```text
Any unlisted tool, data source, execution substrate, provider, adapter, CLI,
browser mode, API, or skill requires explicit user permission before use.
```

This covers:

- native web search;
- direct browser automation;
- raw shell;
- package managers;
- external APIs;
- local CLIs;
- MCP tools;
- adapters;
- background processes;
- code execution substrates;
- another installed skill;
- any new data source not already allowed by the current phase.

The point is not to ban useful tools. The point is to prevent silent boundary
switches.

## Relationship To Forbids

`forbid` and `tool_boundary.forbid` both appear in the action checklist.

Decision forbids usually come from rules. Phase forbids usually come from the
execution plan. Handoff forbids can apply at a route or phase boundary.

The effective boundary combines them so the before-tool-call checklist can ask:

- Is this tool explicitly allowed?
- Is this tool forbidden?
- Is this data source new?
- Is this execution substrate new?
- Does this action violate the selected route or matched rules?

If something is forbidden, permission is required to override it. The agent
should not choose an alternate tool just because it is available.

## Relationship To Handoffs

A phase can declare a handoff. `skillspec act` renders the handoff and required
transition.

If a handoff has boundary `stop_current_skill`, the current skill should stop
domain execution after passing the declared context. It should not keep acting
with its own tools while also asking the target skill to act.

The effective boundary still applies during handoff setup. If the handoff target
is not allowed, the spec should declare it or the agent should ask permission.

## Relationship To Lower-Level Skill Defaults

The selected route and matched rules override lower-level defaults.

If a lower-level skill says a public page can use a generic web tool, but the
selected SkillSpec route says browser work must use an attached browser session,
the selected route controls.

If a lower-level skill says shell work can use raw commands, but the active
SkillSpec route says shell work must use a durable substrate, the selected route
controls.

The action checklist includes route authority for this reason:

```text
The selected route and matched rules override lower-level skill defaults and
generic tool preferences.
```

## Before-Tool-Call Checklist

`skillspec act` renders these questions for every phase:

- Is this tool, data source, execution substrate, provider, adapter, CLI,
  browser mode, API, or skill explicitly allowed by the phase tool boundary?
- If the next action is unlisted or forbidden, should the agent stop and ask for
  permission?
- Does this action violate any listed forbid?
- Do the selected route and matched rules override a lower-level default?
- Are required dependencies, checks, or command-specific requirements satisfied?
- If a handoff boundary applies, has the handoff happened exactly as specified?
- Will the result be captured as evidence for trace alignment or final
  reporting?

When elicitations are active, `act` also asks whether required elicitations have
been answered or explicitly waived.

The agent should repeat this checklist before every tool call, not only at the
start of the run.

## Authoring Guidance

Use `entry.tool_boundary` for skill-wide defaults.

Use route `tool_boundary` for route-specific substrate choices.

Use phase `tool_boundary` for the current action surface.

Prefer concrete contract labels that the harness can map:

```yaml
tool_boundary:
  default: deny
  allow:
    - skillspec_cli
    - rote_exec
    - local_repo_files
  forbid:
    - native_web_search
    - raw_browser
  permission_required_for:
    - any_unlisted_tool
    - any_external_side_effect
```

Avoid labels that only make sense to one model prompt and cannot be checked by a
harness.

## Runtime Behavior

When the next action is allowed, the agent may proceed and record evidence after
the action.

When the next action is unlisted, the agent must ask permission before using it.

When the next action is forbidden, the agent must stop and ask permission or
choose a different allowed route.

When the needed substrate is missing, the agent should record a blocked phase or
ask for permission to install, authenticate, or use a fallback.

When a user explicitly grants permission, the harness should capture that
permission as execution evidence.

## What Boundaries Are Not

Tool boundaries are not a security sandbox by themselves.

They do not prevent a tool call unless the harness enforces them.

They do not grant product permissions.

They do not execute commands.

They do not replace dependency checks, elicitations, review, or user approval.

They are the structured contract that lets the agent, harness, and reviewer
agree on whether a tool was in scope.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-core/src/spec/model.rs`, which defines `ToolBoundary` and
  `ToolBoundaryDefault`;
- `crates/skillspec-runtime/src/act.rs`, which merges entry, route, phase, and
  active forbids into the effective boundary;
- `spec/grammar.md`, which documents `permission_required_for`;
- `spec/skill.spec.schema.json`, which defines the schema surface;
- generated loader output in `crates/skillspec-authoring/src/compiler.rs`.
