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
skillspec index --roots <skill-root>... --out <index> [--visibility-manifest <manifest>]
skillspec route --index <index> --query '<user task>' --top 5 --json
skillspec skills audit --roots <skill-root>... --json
skillspec visibility plan --roots <skill-root>... --json
skillspec visibility apply --roots <skill-root>... --manifest <manifest> --json
skillspec visibility restore --manifest <manifest> --json
skillspec skills set-visibility <skill> manual-only --roots <skill-root>... --manifest <manifest>
skillspec skills disable <skill> --roots <skill-root>... --manifest <manifest>
skillspec skills enable <skill> --roots <skill-root>... --manifest <manifest>
skillspec router install --roots <skill-root>... --router-root <skill-root> --index <index>
skillspec router index status --roots <skill-root>... --index <index> --visibility-manifest <manifest>
skillspec router index refresh --roots <skill-root>... --index <index> --visibility-manifest <manifest>
skillspec router uninstall
```

`skillspec router install` writes:

- a visible `skill-router` skill with a managed marker;
- a SQLite index;
- a visibility manifest;
- a router config under `SKILLSPEC_HOME/router/config.json`, or
  `~/.skillspec/router/config.json` when `SKILLSPEC_HOME` is not set.

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

Durability is not a per-skill or per-tool index flag. The router returns the best
domain skill. If the task is tool-backed and the user has not already chosen an
execution mode, the agent asks whether to run direct or durable. If the user
chooses durable execution, the selected skill and user task are handed to
`durable-executor`, which owns workspace evidence, rote execution policy,
alignment, token stats, and final closure.

## Safety Boundaries

- Loading a hidden skill's markdown does not grant additional tools or bypass
  approvals.
- `off` skills are excluded from route results.
- Router uninstall removes only a generated router skill that has the managed
  marker file.
- Visibility restore is manifest-driven.
- Raw command details remain preserved in traces and evidence, even when UI
  progress prefers human descriptions.
