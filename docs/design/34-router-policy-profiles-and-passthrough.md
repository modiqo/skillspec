# Router Policy Profiles And Passthrough

Status: implemented first cut. This document describes the SQLite-backed router
preference and passthrough layer now implemented under `skillspec router policy`
and `skillspec router profile`. YAML import/export and native visibility
mutation for `native-passthrough` remain future work.

## Problem

Router mode is useful when many skills are installed because it keeps native
skill discovery small and makes skill loading explicit. But operators need more
control than lexical matching alone:

- prefer one installed skill over another for certain words or phrases;
- keep the router universal without hardcoding any provider or tool;
- switch into high-throughput work modes where almost everything bypasses;
- allow future schedulers to change routing profiles dynamically;
- explain why policy changed a route decision.

The policy layer must be a local, auditable control plane. It must not turn the
open-source router into a vendor-specific runtime.

## Non-Goals

- Do not hardcode providers, tools, SaaS names, browser systems, shell runners,
  or durable recorders in `router.rs`.
- Do not hide policy effects from route output.
- Do not use policy to select skills that are disabled or missing.
- Do not replace each skill's own `skill.spec.yml` tool boundaries, forbids,
  checks, or proof requirements.
- Do not overload `skillspec router disable`; disable still means restoring
  router-managed visibility and turning router mode off.

## Files

Router behavior should stay split across generated contract, runtime database,
local config, and optional import/export files:

```text
~/.skillspec/router/
  config.json
  skill-index.sqlite
  visibility-manifest.json
  profile-manifest.json
```

`skill-index.sqlite` should become the runtime source of truth for both the
skill index and compiled router policy. `skillspec route` should open one
database and read candidates, active profile, and matching policy rules from
that database.

`config.json` remains the router lifecycle state. It may later record active
profile metadata, not the policy content. In the first implementation, active
profile state lives directly in `router_policy_profiles.active` inside
`skill-index.sqlite` so `skillspec route` has one hot-path read source:

```json
{
  "schema": "skillspec/router-config/v1",
  "enabled": true,
  "policy": {
    "active_profile": "default",
    "strict": false,
    "epoch": 42
  }
}
```

YAML remains useful for review, backup, examples, and sharing, but it should be
imported into SQLite before runtime. YAML import/export is not part of the first
implementation:

```text
policy.yml -> skillspec router policy import -> skill-index.sqlite policy tables
skill-index.sqlite policy tables -> skillspec router policy export -> policy.yml
```

The generated `skill-router/skill.spec.yml` must not embed policy. It only
teaches the agent to run `skillspec route` and obey the returned JSON decision.

`profile-manifest.json` is reserved for future reversible visibility and hook
changes made by a native passthrough profile. The first implementation records
`native-passthrough` as a profile mode, but route-time behavior remains soft
passthrough and no native visibility mutation is performed.

## Runtime SQLite Schema

Policy should live in normalized SQLite tables beside the existing indexed
skills. The exact schema can evolve, but the first implementation should be easy
to query and explain:

```sql
CREATE TABLE router_policy_profiles (
  name TEXT PRIMARY KEY,
  mode TEXT NOT NULL,              -- route, soft-passthrough, native-passthrough
  strict INTEGER NOT NULL DEFAULT 0,
  default_decision TEXT,           -- bypass for soft-passthrough
  active INTEGER NOT NULL DEFAULT 0,
  description TEXT,
  updated_at_unix INTEGER
);

CREATE TABLE router_policy_rules (
  id TEXT PRIMARY KEY,
  profile TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  mode TEXT NOT NULL DEFAULT 'soft',
  anchor TEXT NOT NULL DEFAULT 'none',
  ordinal INTEGER NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  FOREIGN KEY(profile) REFERENCES router_policy_profiles(name)
);

CREATE TABLE router_policy_predicates (
  rule_id TEXT NOT NULL,
  kind TEXT NOT NULL,              -- any_keywords, all_keywords, none_keywords
  phrase TEXT NOT NULL,
  ordinal INTEGER NOT NULL,
  FOREIGN KEY(rule_id) REFERENCES router_policy_rules(id)
);

CREATE TABLE router_policy_preferences (
  rule_id TEXT NOT NULL,
  ordinal INTEGER NOT NULL,
  effect TEXT NOT NULL,            -- prefer, suppress, forbid, allow
  target_kind TEXT NOT NULL,       -- skill, tag, source, has_skill_spec
  target_value TEXT NOT NULL,
  weight REAL,
  FOREIGN KEY(rule_id) REFERENCES router_policy_rules(id)
);

CREATE TABLE router_policy_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  at_unix INTEGER NOT NULL,
  actor TEXT,
  action TEXT NOT NULL,            -- set, import, activate, clear, export
  profile TEXT,
  summary TEXT
);
```

Useful indexes:

```sql
CREATE INDEX router_policy_rules_profile_priority
ON router_policy_rules(profile, enabled, priority DESC, ordinal ASC);

CREATE INDEX router_policy_predicates_rule_kind
ON router_policy_predicates(rule_id, kind);

CREATE INDEX router_policy_preferences_rule_order
ON router_policy_preferences(rule_id, effect, ordinal);
```

## Import/Export YAML Schema

YAML is an interchange format, not the hot-path store.

Example:

```yaml
schema: skillspec/router-policy/v1

defaults:
  mode: route
  strict: false
  max_rules_warning: 100
  max_preferences_warning: 10

profiles:
  default:
    mode: route
    rules:
      - browser_work
      - api_work

  code:
    mode: soft-passthrough
    allow:
      - skill: coding-guidelines
      - tag: repo-readiness
    rules:
      - code_review_support

  focus:
    mode: native-passthrough
    implicit_allow:
      - skill: coding-guidelines
    explicit_all_others: true
    disable_router_first_hop: true

rules:
  - id: browser_work
    priority: 100
    when:
      any_keywords: ["browse", "browser", "dashboard", "authenticated", "click", "navigate"]
    prefer:
      - skill: rote-browse
        weight: 100
      - skill: browser
        weight: 60
      - tag: browser-automation
        weight: 40
    mode: soft
    anchor: policy

  - id: api_work
    priority: 90
    when:
      any_keywords: ["api", "connect to", "fetch from"]
      none_keywords: ["create adapter", "build adapter"]
    prefer:
      - tag: recorded-api
      - skill: durable-executor
    mode: soft

  - id: code_review_support
    priority: 50
    when:
      any_keywords: ["review", "diff", "clippy", "test", "preflight"]
    prefer:
      - skill: coding-guidelines
    mode: soft
```

The schema is intentionally provider-neutral. It can name concrete skills, but
those names live in user policy tables, not in router code.

## Number And Ordering

The SQLite store should not impose a small hard limit. Operators may define many
rules and many preferences. The CLI should still warn when policy becomes hard
to audit:

- more than 100 rules: warn;
- more than 10 preferences in one rule: warn;
- duplicate preferences in one rule: reject in strict mode, warn otherwise;
- unknown skill names: reject in strict mode, warn otherwise;
- unknown tags: warn because tags may appear after future installs.

Ordering is explicit:

1. Profile selection chooses the active rule set.
2. Matching rules are sorted by `priority` descending.
3. Ties use `router_policy_rules.ordinal`, lower ordinal first.
4. Inside a rule, `router_policy_preferences.ordinal` is meaningful.
5. Optional `weight` overrides order-derived weight.

If no weights are supplied, import and `policy set` should derive weights from
order. For example:

```yaml
prefer:
  - skill: rote-browse
  - skill: browser
  - tag: browser-automation
```

means first preference is strongest. A simple derived scheme is:

```text
first preference  -> +100
second preference -> +80
third preference  -> +60
```

The exact values are less important than the invariant: stored ordinal is stable
and visible in route explanations.

## Match Predicates

Initial predicates should stay simple and explainable:

```yaml
when:
  any_keywords: ["browser", "dashboard"]
  all_keywords: ["github", "issue"]
  none_keywords: ["create adapter"]
```

`keywords` means case-insensitive phrase matching after query normalization. It
should support single tokens and multi-word phrases. Regex can be added later,
but should not be part of the first implementation because it makes policy less
auditable and harder to query.

Future predicates can include:

- current harness;
- current workspace path;
- time window;
- active git repository;
- installed skill tags;
- user-supplied execution mode;
- scheduler-provided context.

## Preference Targets

`prefer`, `allow`, `suppress`, and `forbid` targets should support:

```yaml
- skill: exact-skill-name
- tag: tag-name
- has_skill_spec: true
- source: codex
- source: claude-local
```

Skill targets are precise. Tag targets are broader and should only apply to
skills already present in the route index.

`forbid` means the matched target cannot be selected by route while the policy
is active. `suppress` means score penalty but not absolute exclusion.

Disabled/off skills remain excluded regardless of policy.

## Route Semantics

Normal route flow should become:

```text
read index
score candidates from skill front matter and optional skill.spec.yml metadata
collapse duplicate physical roots into logical skills
load active router policy profile
apply policy effects
run match gate
return decision, selected, candidates, and policy explanation
```

Policy must be visible in JSON:

```json
{
  "policy": {
    "profile": "default",
    "mode": "route",
    "epoch": 42,
    "matches": [
      {
        "rule": "browser_work",
        "effect": "boost",
        "target": "rote-browse",
        "weight": 100
      }
    ]
  }
}
```

Policy should never silently erase the base route score. Candidate output should
include both:

```json
{
  "name": "browser",
  "base_score": 8.2,
  "policy_score": 60.0,
  "score": 68.2,
  "policy_reason": "matched rule browser_work preference #2"
}
```

## Activation Anchor Interaction

The activation-anchor gate prevents broad false positives. Policy introduces an
operator-authored anchor, so the interaction must be explicit.

Recommended behavior:

- `anchor: none`: policy can boost score but cannot satisfy activation anchor.
- `anchor: policy`: an exact `skill` preference may satisfy the activation gate
  when the rule predicate matches.
- tag preferences should not satisfy anchor by default because they are broad.

Example:

```yaml
rules:
  - id: browser_work
    when:
      any_keywords: ["browse", "dashboard"]
    prefer:
      - skill: browser
    mode: soft
    anchor: policy
```

This lets an operator say, "In my environment, browse/dashboard is enough reason
to consider the `browser` skill anchored." The route output must report that the
anchor came from policy, not lexical matching.

## Rule Modes

### `soft`

Boost or suppress candidates, then run the normal match gate.

Use this for most policies.

### `hard`

If the predicate matches, choose the first installed allowed preference unless a
higher-priority forbid blocks it. This mode can bypass ordinary lexical ranking,
but it must still:

- refuse missing skills;
- refuse off/disabled skills;
- report the exact rule and preference item used;
- be opt-in per rule.

Use this for tightly controlled environments.

### `soft-passthrough`

Router remains first-hop, but the active profile defaults to bypass. Only
allowlisted skills or matching rules can route.

This preserves the router guard/hook path while reducing accidental skill loads.

### `native-passthrough`

Router steps out of the hot path for maximum throughput:

- managed prompt guard hooks are disabled or removed;
- generated `skill-router` is set explicit-only;
- all non-allowlisted skills are set explicit/manual-only;
- allowlisted skills can remain implicit;
- original state is recorded in `profile-manifest.json`.

This is a temporary profile, not router uninstall. Clearing the profile restores
the router-managed state from the profile manifest.

## Passthrough Profiles

Code-focused example:

```yaml
profiles:
  code:
    mode: native-passthrough
    description: "High-throughput coding mode"
    implicit_allow:
      - skill: coding-guidelines
    explicit_all_others: true
    disable_router_first_hop: true
```

Expected operator commands:

```sh
skillspec router profile apply code --dry-run --json
skillspec router profile apply code
skillspec router profile status --json
skillspec router profile clear
```

`router disable` should not be used for this because disable means "restore
pre-router visibility." Profile clear means "return to the router state that was
active before this temporary profile."

## Guard Behavior

Future work: `skillspec router guard --hook --json` should report profile
state. The first implementation applies active policy in `skillspec route`, but
does not yet surface profile state through guard output.

Normal route profile:

```json
{
  "first_hop_ready": true,
  "active_profile": "default",
  "profile_mode": "route"
}
```

Soft passthrough:

```json
{
  "first_hop_ready": true,
  "active_profile": "code",
  "profile_mode": "soft-passthrough",
  "default_decision": "bypass"
}
```

Native passthrough:

```json
{
  "first_hop_ready": false,
  "passthrough_ready": true,
  "active_profile": "code",
  "profile_mode": "native-passthrough",
  "router_first_hop": false
}
```

This keeps the guarantee honest: router-first is not claimed during native
passthrough.

## CLI Surface

Implemented commands:

```sh
skillspec router policy init --index <router-index> [--json]
skillspec router policy list --index <router-index> [--json]
skillspec router policy show --index <router-index> [--profile <name>] [--json]
skillspec router policy get <profile-or-rule-id> --index <router-index> [--json]
skillspec router policy set-profile <name> \
  --index <router-index> \
  --mode route|soft-passthrough|native-passthrough \
  [--active] [--strict] [--description <text>] [--json]
skillspec router policy set-rule <id> \
  --index <router-index> \
  --profile <name> \
  [--priority <n>] \
  [--mode soft|hard] \
  [--anchor none|policy] \
  [--when-any <phrase>]... \
  [--when-all <phrase>]... \
  [--when-none <phrase>]... \
  [--prefer <target>]... \
  [--allow <target>]... \
  [--suppress <target>]... \
  [--forbid <target>]... \
  [--json]
skillspec router policy remove-rule <id> --index <router-index> [--json]
skillspec router policy explain --index <router-index> --query <text> [--profile <name>] [--json]

skillspec router profile status --index <router-index> [--json]
skillspec router profile apply <name> --index <router-index> [--dry-run] [--json]
skillspec router profile clear --index <router-index> [--dry-run] [--json]
```

Concrete policy management should be possible without writing YAML:

```sh
skillspec router policy set-profile code \
  --index ~/.skillspec/router \
  --mode soft-passthrough \
  --active \
  --json

skillspec router policy set-rule browser_work \
  --index ~/.skillspec/router \
  --profile default \
  --priority 100 \
  --when-any "browse" \
  --when-any "browser" \
  --when-any "dashboard" \
  --when-any "authenticated" \
  --prefer skill:rote-browse \
  --prefer skill:browser \
  --anchor policy \
  --json

skillspec router policy list --index ~/.skillspec/router --json
skillspec router policy get browser_work --index ~/.skillspec/router --json
```

`skillspec route` should stay the hot-path leaf command:

```sh
skillspec route \
  --index ~/.skillspec/router \
  --profile code \
  --query "browse the dashboard" \
  --json
```

The route command reads policy from `skill-index.sqlite`. `--profile` is an
override for that invocation; when omitted, route uses the active profile row in
SQLite.

`skillspec route policy ...` should not be added. `skillspec route` already
requires `--index` and `--query`; policy management belongs under
`skillspec router policy` because it mutates router state rather than performing
one routing decision.

Future CLI surface:

- `skillspec router policy validate`
- `skillspec router policy import`
- `skillspec router policy export`
- `skillspec router profile list`

## Policy Queries

The implementation should own SQL. Users define structured policy through CLI or
imported YAML; they do not provide arbitrary SQL.

Representative internal queries:

Active profile:

```sql
SELECT name, mode, strict, default_decision
FROM router_policy_profiles
WHERE active = 1
LIMIT 1;
```

Rules for a profile:

```sql
SELECT id, priority, mode, anchor
FROM router_policy_rules
WHERE profile = ? AND enabled = 1
ORDER BY priority DESC, ordinal ASC;
```

Predicates for a rule:

```sql
SELECT kind, phrase
FROM router_policy_predicates
WHERE rule_id = ?
ORDER BY ordinal ASC;
```

Preferences for a rule:

```sql
SELECT effect, target_kind, target_value, weight, ordinal
FROM router_policy_preferences
WHERE rule_id = ?
ORDER BY ordinal ASC;
```

The first implementation can evaluate phrase predicates in Rust after fetching
rules, because query text normalization and phrase matching are small. Later,
common phrase indexes can be added if policy grows large.

## Scheduler Integration

A scheduler should change policy through `skillspec router policy` or by
atomically replacing policy rows in SQLite through a future supported API. It
should not edit generated router skill files.

Recommended scheduler-safe state:

```json
{
  "policy": {
    "active_profile": "code",
    "epoch": 42,
    "updated_by": "scheduler",
    "updated_at_unix": 1782780000
  }
}
```

Route can cache the active profile and rules by SQLite `PRAGMA data_version` plus
policy epoch. If policy rows are inconsistent, route should fail closed in
strict mode and bypass with an explicit policy error in non-strict mode.

## Safety And Audit

Every policy-influenced route must explain:

- active profile;
- matched rules;
- preference item used;
- whether policy supplied the activation anchor;
- score delta;
- missing or ignored policy targets;
- strict versus non-strict behavior.

Native passthrough must explain:

- which hooks were disabled;
- which skills remained implicit;
- which skills were made explicit/manual-only;
- where the profile manifest was written;
- how to restore.

## Implementation Plan

1. Add policy data model and parser in the harness crate.
2. Extend the router SQLite schema with policy tables and migrations.
3. Add `skillspec router policy init/validate/show/list/get/set/remove/import/export/explain`.
4. Extend `skillspec route` with optional `--profile`.
5. Apply soft policy scoring after duplicate collapse and before match gate.
6. Add policy explanation fields to route JSON.
7. Add profile state to router config and SQLite active profile rows.
8. Add soft-passthrough route semantics.
9. Add native-passthrough lifecycle commands with reversible profile manifest.
10. Update generated `skill-router/skill.spec.yml` and `skills/skillspec` self
   skill to teach policy/profile commands.
11. Add controlled harness-lab tests and baselines.

## Test Matrix

Required tests:

- policy SQLite schema migrates existing router indexes safely;
- policy import accepts valid rules, profiles, preferences, and predicates;
- `policy set/get/list/remove` round-trip policy rows without YAML;
- strict mode rejects duplicate preferences and unknown exact skills;
- non-strict mode warns but continues for unknown optional targets;
- preference ordinal changes candidate ordering predictably;
- explicit weights override array-derived weights;
- soft mode cannot select an unrelated skill without activation anchor;
- `anchor: policy` allows an exact skill preference to satisfy anchor;
- hard mode selects the first installed allowed preference and reports why;
- disabled/off skills cannot be selected by policy;
- soft-passthrough defaults to bypass except allowlisted matches;
- native-passthrough disables router first-hop and preserves restore manifest;
- profile clear restores router-managed state;
- route JSON includes policy explanation for every policy effect;
- YAML export/import round-trips a SQLite policy without losing order;
- no provider-specific skill names appear in router code or default policy.

## Open Questions

- Should `anchor: policy` default to false for maximum safety, or true for exact
  skill preferences to make policy useful with fewer fields?
- Should native passthrough disable managed hooks or keep a minimal hook that
  reports passthrough state?
- Should profile state live primarily in router config, SQLite active rows, or
  both? The implementation should avoid split-brain state.
- Should active profile be overrideable by environment variable for short-lived
  sessions?
- Should scheduler updates require a signed policy file in managed environments?
