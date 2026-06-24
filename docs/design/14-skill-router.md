# Skill Router

The skill router is an optional discovery layer for large local skill libraries.
It solves one narrow problem:

```text
many installed skills
-> native name/description inventory consumes context budget
-> descriptions may be shortened
-> implicit skill selection becomes less reliable
```

The router does not replace native skills and does not execute the selected
skill. It builds a local SQLite catalog, ranks candidate skills for a user
request, and returns the selected `SKILL.md` path plus confidence and candidates.

## Current Implementation

The implemented CLI surface is:

```bash
skillspec index --roots <skill-root>... --out <index-file-or-router-dir> [--visibility-manifest <manifest>]
skillspec route --index <index-file-or-router-dir> --query '<user task>' --top 5 --json
skillspec skills audit --roots <skill-root>... --json
skillspec visibility plan --roots <skill-root>... --json
skillspec visibility apply --roots <skill-root>... --manifest <manifest> --json
skillspec visibility restore --manifest <manifest> --json
skillspec skills set-visibility <skill> manual-only --roots <skill-root>... --manifest <manifest>
skillspec skills disable <skill> --roots <skill-root>... --manifest <manifest>
skillspec skills enable <skill> --roots <skill-root>... --manifest <manifest>
skillspec router install --roots <skill-root>... --index <index-file-or-router-dir>
skillspec router index status --roots <skill-root>... --index <index-file-or-router-dir> --visibility-manifest <manifest>
skillspec router index refresh --roots <skill-root>... --index <index-file-or-router-dir> --visibility-manifest <manifest>
skillspec router uninstall
```

`skillspec router install` writes:

- a SkillSpec-backed `skill-router` skill with a thin `SKILL.md`, a
  `skill.spec.yml` contract, and a managed marker in the first `--roots` path;
- a SQLite index;
- a visibility manifest;
- a router config under `SKILLSPEC_HOME/router/config.json`, or
  `~/.skillspec/router/config.json` when `SKILLSPEC_HOME` is not set.

Any index argument can be either the SQLite file itself or the router directory;
directory paths resolve to `skill-index.sqlite`.

Router mode is the managed state created by `skillspec router install`:

- indexed skills are made explicit-only/manual-only unless they are already
  `off`;
- `durable-executor` is the only implicit exception when it is already present
  in the managed roots;
- the generated `skill-router` skill is explicit-only and still directly
  invocable;
- the index remains searchable by `skillspec route`;
- the manifest is the only rollback authority.

Router install does not install or copy `durable-executor`. If
`durable-executor` is found in the managed roots, router mode preserves it as the
implicit first-hop. If it is missing, router install still succeeds and reports
that durable first-hop execution is unavailable until durable-executor is
installed separately.

This differs from the original proposal where the router itself was the visible
implicit skill. In the implemented mode, `durable-executor` remains the implicit
first-hop for tool-backed work and can call `skillspec route` as a discovery
primitive before handing off domain work.

When that config exists, `skillspec install skill` automatically reapplies the
router-managed visibility profile and refreshes the configured index after a
successful install.

## Visibility Model

The router uses native harness controls where available.

Codex and Agents roots use:

```yaml
policy:
  allow_implicit_invocation: false
```

in `<skill>/agents/openai.yaml`.

Claude roots use `skillOverrides` in the nearest `.claude/settings.json`:

```json
{
  "skillOverrides": {
    "deploy": "user-invocable-only"
  }
}
```

The conceptual states are:

- `implicit`: native model routing can select the skill automatically.
- `manual-only`: explicit invocation and router selection are allowed, but
  native implicit invocation is disabled.
- `name-only`: native listing can keep the name while minimizing description
  budget where supported.
- `off`: router results exclude the skill. For Codex this uses native
  manual-only metadata plus the router visibility manifest as the exclusion
  source.

The manifest is the rollback boundary. Restore uses recorded file snapshots; it
does not infer previous state from current files.

## Route Algorithm

The first implementation uses deterministic BM25-style lexical scoring over:

- name;
- description;
- optional short description;
- tags and trigger phrases from `SKILL.md` frontmatter metadata;
- optional `skill.spec.yml` activation keywords, route labels, descriptions,
  and rule reasons.

It adds exact-name and trigger bonuses, subtracts negative-trigger penalties, and
filters out `off` skills. The route result includes the selected candidate,
scores, confidence, visibility, whether the skill is SkillSpec-backed, and an
execution-mode elicitation hint when the caller has not already supplied direct
or durable execution mode.

## Durable Execution

The router and durable-executor are separate layers.

Router mode answers:

```text
Which local skill best matches this request?
```

Durable-executor mode answers:

```text
How should tool-backed work execute and prove itself?
```

Durability is a global execution envelope, not a per-skill or per-tool index
flag. The index should not accumulate flags such as `git_requires_durable`,
`browser_requires_durable`, or `npm_requires_durable`. `skillspec route`
returns the best domain skill plus an execution-mode hint when appropriate.

Normal routing:

```text
user task
-> durable-executor implicit first-hop, or explicit skill-router invocation
-> skillspec route
-> selected domain skill
-> normal execution when direct mode is selected
```

Durable routing:

```text
tool-backed user task
-> skillspec route selects a domain skill
-> user has explicitly chosen durable mode, or durable-executor was invoked
-> durable-executor creates the execution envelope
-> selected skill supplies domain interpretation
-> durable-executor owns substrate, evidence, alignment, token stats, and closure
```

When the user has not already chosen direct or durable execution, route output
may include:

```json
{
  "elicitation": "execution_mode_direct_or_durable"
}
```

The harness then asks whether to run direct or durable. Skip that question when
the user already chose durable/direct mode, the task is pure discussion, or the
selected skill is `durable-executor` itself.

When durable mode is active, the durable envelope wins on execution substrate.
For example, if the selected domain skill says to run `git status`, the durable
envelope still requires the actual process to run through `rote exec`.

## Safety Boundaries

- Loading a hidden skill's markdown does not grant additional tools or bypass
  approvals.
- `off` skills are excluded from route results.
- Router uninstall removes only a generated router skill that has the managed
  marker file.
- Visibility restore is manifest-driven.
- Raw command details remain preserved in traces and evidence, even when UI
  progress prefers human descriptions.
