# Skill Router

The skill router is an optional discovery layer for large local skill libraries.
It solves one narrow problem:

```text
many installed skills
-> native name/description inventory consumes context budget
-> descriptions may be shortened
-> implicit skill selection becomes less reliable
```

The router does not replace native skills and does not execute task work. It
builds a local SQLite catalog, ranks candidate skills for a user request, and
returns a route decision: `use_skill`, `bypass`, or `ambiguous`. Only
`use_skill` authorizes loading the returned `selected` `SKILL.md`; `bypass` and
`ambiguous` preserve normal agent behavior without silently loading a candidate.

The top-level `skillspec index` command is part of this router surface. It is
not a general repository search command, source map, workspace map, or import
planner. When run directly, it prints router-state warnings so operators and
agents know whether the catalog is standalone, blocked behind disabled router
mode, or better maintained through `skillspec router index refresh`.

## Current Implementation

The implemented CLI surface is:

```bash
skillspec index --roots <skill-root>... --out <index-file-or-router-dir> [--visibility-manifest <manifest>]
skillspec route --index <index-file-or-router-dir> --query '<user task>' [--current-harness agents|codex|claude-local] [--current-root <skill-root>] --top 5 --json
skillspec skills audit --roots <skill-root>... --json
skillspec visibility plan --roots <skill-root>... --json
skillspec visibility apply --roots <skill-root>... --manifest <manifest> --json
skillspec visibility restore --manifest <manifest> --json
skillspec skills set-visibility <skill> manual-only --roots <skill-root>... --manifest <manifest>
skillspec skills disable <skill> --roots <skill-root>... --manifest <manifest>
skillspec skills enable <skill> --roots <skill-root>... --manifest <manifest>
skillspec status [--roots <skill-root>...] [--json]
skillspec router install --roots <skill-root>... --index <index-file-or-router-dir> [--force]
skillspec router enable
skillspec router disable
skillspec router update [--backup-dir <backup-dir>]
skillspec router guard [--config <router-config>] [--hook] [--harness agents|codex|claude-local] [--json]
skillspec router index status --roots <skill-root>... --index <index-file-or-router-dir> --visibility-manifest <manifest>
skillspec router index refresh --roots <skill-root>... --index <index-file-or-router-dir> --visibility-manifest <manifest>
skillspec router uninstall # alias: delete
skillspec durable-executor install <source-folder> --target <target>
skillspec durable-executor enable
skillspec durable-executor disable
skillspec durable-executor update [--source <source-folder>] [--backup-dir <backup-dir>]
skillspec durable-executor delete # alias: uninstall
```

`skillspec router install` writes:

- a SkillSpec-backed `skill-router` skill with a thin `SKILL.md`, a
  `skill.spec.yml` contract, and a managed marker in every configured
  `--roots` path;
- a SQLite index;
- a visibility manifest;
- managed Codex/Claude prompt guard hook entries where supported;
- a router config under `SKILLSPEC_HOME/router/config.json`, or
  `~/.skillspec/router/config.json` when `SKILLSPEC_HOME` is not set.

Any index argument can be either the SQLite file itself or the router directory;
directory paths resolve to `skill-index.sqlite`.

If an older local install used `~/.skillspec/router` as a SQLite index file,
`router install --index ~/.skillspec/router` cannot also create
`~/.skillspec/router/config.json`. In that case install reports a legacy-index
collision and asks for `--force`; the forced path moves the old file to
`~/.skillspec/router/skill-index.sqlite` before creating router config.

Router mode is the managed state created by `skillspec router install`:

- after install/enable and when managed hooks are loaded by the active harness,
  the generated `skill-router` skill is the first hop for every user request in
  managed roots and is directly invocable;
- when enabled, indexed routed skills are made explicit-only/manual-only unless
  they are already `off`;
- `durable-executor` is implicit only when it is present in the managed roots
  and its own lifecycle state is enabled;
- when disabled, managed router guard hook entries are removed, the generated
  `skill-router` skill is explicit-only, and routed skills are restored to
  implicit/default native visibility;
- the index remains searchable by `skillspec route`;
- install and install-hook refreshes run an immediate status check after
  indexing; preparedness requires a present, non-stale index whose indexed
  skill count matches the discovered skill count;
- `skillspec router guard` can be run by users or prompt hooks to verify
  `first_hop_ready`; when enabled and index/visibility drift is detected it
  reapplies router-managed visibility and rebuilds the index before reporting;
- ordinary first-hop routing does not run index status or repair checks when the
  prompt hook has already reported `first_hop_ready=true`; it runs a single
  `skillspec route` decision and obeys that result;
- route first collapses duplicate physical installs of the same logical skill,
  then uses optional harness/root context only to choose which physical copy to
  load;
- the manifest is the only rollback authority.

Router install does not install or copy `durable-executor`. If
`durable-executor` is found in the managed roots and durable lifecycle is
enabled, router mode preserves it as the implicit first-hop. If it is missing,
router install still succeeds and reports that durable first-hop execution is
unavailable until durable-executor is installed separately.

The router guarantee is visibility-backed, not prose-only. Within configured
roots, router install/enable writes native metadata and managed prompt guard
hooks so the router is the first hop for every request while routed skills stop
competing for implicit selection. The prompt hook owns freshness and repair; the
ordinary router first-hop owns only dispatch. When hook context says
`first_hop_ready=true`, the router must query the local index once and obey the
route decision before loading any domain skill. It must not run
`skillspec router index status` or read the full router SkillSpec for a normal
prompt. If the decision is `bypass`, the agent continues with the normal path
for the request, including ordinary workspace or web search when those are
otherwise allowed. If the decision is `ambiguous`, the agent must not silently
load a candidate; it asks only when the user explicitly requested skill
selection, otherwise it continues normally. A harness restart is required before
active sessions reliably observe the metadata and hook changes. Skills outside
the managed roots, stale harness sessions, disabled hooks, or harness-specific
selection bugs are outside the guarantee; `skillspec status`,
`skillspec router guard`, and `skillspec router index status` are the
verification gates.

`durable-executor` has its own managed lifecycle. `skillspec durable-executor
install <source-folder>` first checks that `rote` is available on `PATH`, then
installs from an explicit local source, records the source and every managed
install directory under
`SKILLSPEC_HOME/durable-executor/config.json`, and writes a managed marker into
each installed folder. `skillspec durable-executor update` backs up that config
and every recorded folder before rewriting marker-protected folders from the
recorded source or `--source`; it runs the same `rote` preflight and refuses an
existing unmarked folder.
`skillspec durable-executor delete` removes only recorded folders that contain
the durable managed marker. If router mode is configured, durable install,
update, and delete refresh router-managed visibility and the index.

`skillspec durable-executor disable` is a switch, not an uninstall: it keeps
recorded durable installs but makes them explicit-only across Codex and Claude
visibility metadata. `skillspec durable-executor enable` checks that `rote` is
on `PATH` and makes those recorded installs implicit again. Router mode reads
this durable lifecycle state so a later router update does not silently undo a
durable disable.

`skillspec router disable` is a switch, not an uninstall: it keeps config,
manifest, index, and router skill files, removes managed router guard hook
entries, makes the router explicit-only, and restores routed skills to
implicit/default visibility. `skillspec router enable` switches router mode back
on, refreshes router skill files, reinstalls managed guard hooks, reapplies
explicit-only routed-skill controls, rebuilds the index from current roots, and
checks preparedness.

`skillspec status` is the read-only overview for humans and agents. It reports
whether router and durable-executor are installed and enabled, which roots are
supported and scanned, the last router index state, and the names/details of
SkillSpec-backed versus legacy prose skills. It does not rebuild the index,
repair visibility, install durable-executor, or mutate harness roots.

When that config exists, `skillspec install skill` automatically reapplies the
router-managed visibility profile and refreshes the configured index after a
successful install, then performs the same preparedness check.

Ordinary `skillspec install skill` is for domain skills. It is not the cleanup
surface for SkillSpec-owned router or durable-executor installs; use the
specific lifecycle commands so recorded roots, managed markers, backups, router
refresh, and restart warnings stay consistent.

`skillspec router update` is for maintenance of an existing router install. It
starts from `SKILLSPEC_HOME/router/config.json`, backs up the config, manifest,
index, and every managed router skill directory, rewrites the generated
SkillSpec-backed router package in each recorded root, reapplies visibility,
refreshes managed guard hooks to match enabled state, preserves the current
enabled/disabled mode, rebuilds the index only when enabled, and reports
preparedness when enabled. Because Codex, Claude, Agents, and vendor harnesses
load skill metadata and hooks at session start, the command warns the operator
to restart active harness sessions after a successful update. This is the right
repair path for stale generated router text, missing router `skill.spec.yml`
files, stale managed hook commands, or symlinked `.agents`/`.codex` roots that
need every logical install path refreshed.

If a skill is added outside `skillspec install skill`, the router cannot observe
that filesystem change until a router command runs. `skillspec router index
status` is the read-only detector: it reports new, changed, and missing skills,
marks each changed entry as prose-only or SkillSpec-backed, and gives conversion
advice for prose-only `SKILL.md` packages. `skillspec router index refresh` is
the repair step: when router config is present it reapplies router-managed
explicit invocation controls, preserves `durable-executor` as the implicit
exception, rebuilds the index, and checks preparedness. SkillSpec-backed
out-of-band additions are indexed directly; prose-only additions are still made
explicit-only and indexed, but the report advises converting them with
`skillspec import-skill`.

Direct `skillspec index` still exists for manual catalog creation and
`skillspec route --index ...` lookup. It should not be used as an authoring
recon substitute. If router config exists but is disabled, direct indexing does
not make the router implicit or change routed-skill visibility; run
`skillspec router enable` to reactivate router mode, or keep using
`skillspec route` manually against the standalone index.

## Duplicate Logical Skills

Multi-harness setups often install the same skill into `.agents`, `.codex`, and
project-local `.claude` roots. Those copies are physical duplicates, not three
different routing choices. The router therefore collapses candidates by logical
identity before it applies the `use_skill`/`bypass`/`ambiguous` match gate.

Logical identity is:

- the resolved `skill.spec.yml` `id` when the skill is SkillSpec-backed;
- otherwise the skill name plus a normalized prose checksum that strips
  visibility-only `disable-model-invocation` frontmatter before hashing.

After collapse, the representative candidate keeps the strongest duplicate
score so the match gate is still based on the user's intent, not on filesystem
placement. Filesystem preference only chooses which copy to load:

1. `--current-root`, when the caller knows the active skill root.
2. `--current-harness`, when the caller knows the active harness.
3. Project-local `.claude/skills` when the current working directory is inside
   that project.
4. A SkillSpec-backed copy.
5. Configured root order from the index, then stable path order.

This keeps the router harness-agnostic for logical selection while still letting
a Codex session load the Codex copy and a Claude project session load the local
Claude copy. It also preserves ambiguity for real conflicts: two skills with
the same display name but different `skill.spec.yml` ids or different prose
normalized prose checksums remain separate candidates and can still return
`ambiguous`. Router-managed visibility metadata alone is not enough to split a
logical prose skill.

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

Shared `.agents/skills` roots also get Claude-compatible SKILL.md frontmatter,
because those roots may be symlinked into more than one harness:

```yaml
disable-model-invocation: true
```

That header can represent implicit versus manual-only. For `name-only` and
`off`, SkillSpec writes the closest native manual-only control and relies on the
visibility manifest to preserve the exact router state.

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
filters out `off` skills. The route result includes the decision, optional
selected candidate, scores, confidence, visibility, whether the skill is
SkillSpec-backed, bypass reason when no skill should load, and an execution-mode
elicitation hint when the caller has not already supplied direct or durable
execution mode.

The match gate is intentionally conservative:

- `use_skill`: the top candidate has high confidence and clears the separation
  threshold from the next candidate.
- `bypass`: no positive candidate exists, the best candidate is only low or
  medium confidence, or the top candidate lacks an activation anchor.
- `ambiguous`: strong top candidates are too close for automatic skill
  selection.

The activation-anchor gate prevents broad skills from winning on generic prose
overlap. A candidate must match its name or a non-generic name anchor such as
`linear`, `pdf`, `adapter`, or `browse`; generic words such as `docs`,
`document`, `create`, `skill`, and `router` are not enough by themselves.

## Execution Substrate Boundary

The router is provider-neutral. It answers only whether a local skill should be
loaded, and which installed copy should be loaded. It must not hardcode a vendor,
adapter system, browser system, shell runner, or durable execution substrate.

Execution substrate policy belongs in one of these places:

- the selected skill's own `skill.spec.yml`, when a domain skill declares its
  allowed tools, forbids, phases, checks, and evidence;
- the optional durable-executor skill, when the user or installation policy
  chooses durable execution;
- a future declarative capability contract, if SkillSpec grows a portable way
  for skills to advertise execution substrates without binding the router to a
  particular provider.

This boundary matters for open source adoption. A project should be able to use
SkillSpec router with its own skills, tools, adapters, browser automation, and
execution recorder without inheriting another vendor's runtime policy from the
harness.

## Durable Execution

The router and durable-executor are separate layers.

Router mode answers:

```text
Should this request load a local skill, and if so which one?
```

Durable-executor mode answers:

```text
How should tool-backed work execute and prove itself?
```

Durability is a global execution envelope, not a per-skill or per-tool index
flag. The index should not accumulate flags such as `git_requires_durable`,
`browser_requires_durable`, or `npm_requires_durable`. `skillspec route`
returns a `use_skill` decision plus an execution-mode hint when appropriate.

Normal routing:

```text
user task
-> implicit skill-router first-hop for every request after router install/enable and harness restart
-> prompt hook has already verified first_hop_ready=true
-> skillspec route once
-> if decision is use_skill and selected is non-null, load the selected domain skill
-> if decision is bypass, continue normal agent behavior
-> if decision is ambiguous, do not silently load a candidate
```

Durable routing:

```text
tool-backed user task
-> skillspec route returns decision use_skill for a domain skill
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
For example, if a selected domain skill asks for a local process, the active
durable-executor contract decides how that process is recorded and proved.

## Safety Boundaries

- Loading a hidden skill's markdown does not grant additional tools or bypass
  approvals.
- `off` skills are excluded from route results.
- Router uninstall removes only a generated router skill that has the managed
  marker file.
- Visibility restore is manifest-driven.
- Raw command details remain preserved in traces and evidence, even when UI
  progress prefers human descriptions.
