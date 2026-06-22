# Capability Bootstrap

Capability bootstrap is the durable-executor escape hatch for this case:

1. The user asks for a domain outcome.
2. No reviewed domain SkillSpec owns that capability yet.
3. A local evidence-backed seed, adapter, CLI, script, or prior trace can satisfy
   the immediate task.

The implementation keeps the mutable capability inventory outside
`durable-executor`:

```text
~/.skillspec/capabilities/<domain>/<seed-id>.yml
```

`SKILLSPEC_HOME` can override the parent directory for tests or isolated runs.

## Implemented CLI Surface

The source of truth is `crates/skillspec-cli/src/capability.rs`, wired through
`crates/skillspec-cli/src/main.rs`.

Implemented commands:

```sh
skillspec capability store
skillspec capability add <id> --domain <domain> --kind <kind> --provides <capability>
skillspec capability update <id> --domain <domain> [patch options]
skillspec capability list [--domain <domain>]
skillspec capability search <capability> --domain <domain> --explain --json
skillspec capability inspect <id> --domain <domain> --json
skillspec capability verify <id> --domain <domain> --json
skillspec capability prefer <id> --domain <domain> --for <capability> --priority <0-100>
skillspec capability remove <id> --domain <domain>
skillspec capability scan
```

All capability commands emit JSON. `scan` is intentionally conservative in the
current implementation: it reports that no scan providers are configured rather
than guessing vendor capabilities from the environment.

`add` writes a complete seed from the supplied flags. `update` patches an
existing seed and preserves unspecified fields. This distinction matters for
long-lived local capability records: agents can add a new provided capability,
remove one stale alias, lower priority, or mark a seed failed without
accidentally deleting evidence, auth references, or promotion metadata.

## Seed Shape

A seed is strict YAML. Unknown fields are rejected by serde.

```yaml
id: preferred-voice-cli
domain: voice
kind: cli
command: voice-cli
provides:
  - text_to_speech
aliases:
  - voice message
rank:
  default_priority: 80
  preferred_for:
    - text_to_speech
  tie_breakers:
    quality: high
auth:
  env:
    - VOICE_PROVIDER_API_KEY
risk:
  external_service: true
  may_cost_money: true
evidence:
  - source: cli_help
    command: voice-cli --help
promotion:
  suggested_skill_id: voice.provider
```

The seed is not a SkillSpec and not a handoff target. It is evidence that a
tool may satisfy a capability before a domain skill exists.

## Ranking Behavior

`skillspec capability search` computes deterministic scores and returns reasons.
The current implementation uses:

- direct `provides` match: +40
- alias or evidence text match: +15
- verified evidence: +25
- missing verification: -20
- `preferred_for` match: +10
- `avoid_for` match: -30
- `rank.default_priority`: normalized 0-100 into 0-10
- explicit `--preferred-seed`: +100
- `--local-only`: excludes external-service candidates and adds +20 to local
  candidates
- external or paid candidate without `--local-only`: -10 and required gates

Failed verification excludes a candidate. If the top two candidates are within
10 points, `selected` is `null` and `ask_policy.reason` is
`top_candidates_within_10_points`.

## Updating Broken Or Degraded Seeds

When a seed stops working for a capability, do not delete it as the first
response. Preserve the historical metadata and patch the ranking/verification
state:

```sh
skillspec capability update preferred-voice-cli \
  --domain voice \
  --remove-preferred-for text_to_speech \
  --add-avoid-for text_to_speech \
  --priority 0 \
  --mark-failed
```

This removes the positive ranking signal, adds an avoid signal, and marks the
verification state failed while keeping the command, auth references, aliases,
evidence commands, and promotion metadata intact. A later `verify` can restore
the seed to `verified` if the underlying tool recovers.

## Empty Search Behavior

An empty first search is not permission to use an unseeded local tool. The
agent must preserve the empty result as evidence, broaden through normalized
capability and domain equivalents, and search again before falling back.

For example, a voice request may be normalized as `text_to_speech` by one
agent and `voice_generation` by another. The durable-executor pattern requires
checking related terms such as `voice`, `text_to_speech`, `voice_generation`,
`speech_synthesis`, `audio_generation`, and `voice_message` across plausible
domains such as `voice` and `audio`.

If no seed is found after related searches, the agent must ask before using an
unseeded local fallback or create and verify a local seed for that fallback
first. This prevents a machine-local command from bypassing seed ranking,
risk gates, and future skill-draft evidence.

## Verification Behavior

`skillspec capability verify` updates the seed with verification status and
outcomes. Current checks:

- path lookup for `command`;
- execution of declared evidence commands such as `<tool> --help` without a
  shell.

At least one successful outcome marks the seed `verified`; otherwise it is
`failed`.

## SkillSpec Integration

`examples/durable-executor/skill.spec.yml` now includes:

- route `capability_bootstrap`;
- resource `local_capability_seed_store`;
- commands `search_capability_seed_store`, `inspect_capability_seed`, and
  `verify_capability_seed`;
- closures for ranking, ask policy, risk gates, durable substrate execution,
  evidence capture, draft SkillSpec generation, and QA;
- scenario test `missing voice skill uses capability bootstrap`.

The core grammar now allows:

```yaml
commands:
  search_capability_seed_store:
    template: skillspec capability search <capability_id> --domain <domain_id> --explain --json
    requires:
      dependencies:
        - skillspec
      resources:
        - local_capability_seed_store
```

`commands.requires.resources` is modeled in Rust, validated by the parser,
included in `refs`, and documented in the JSON Schema and formal grammar.

## Sensemaking

When a spec contains `capability_bootstrap`,
`local_capability_seed_store`, or capability seed commands, `sensemake` adds
navigation hints for:

- inspecting the bootstrap route;
- ranked seed search;
- seed verification.

This keeps the feature discoverable without teaching capability commands for
unrelated specs.

## Measured Implementation Outcome

Focused implementation checks passed during this change:

- `cargo test --locked -p skillspec --test cli -- --nocapture`
- `cargo run --locked -p skillspec -- validate examples/durable-executor/skill.spec.yml`
- `cargo run --locked -p skillspec -- test examples/durable-executor/skill.spec.yml`

The durable-executor example passed `24/24` scenario tests after the bootstrap
route was added.

## Boundary

Capability bootstrap is not a runtime engine and not a vendor registry.
Commands manage the local seed store. durable-executor points at the store,
queries ranked evidence, applies user/risk gates, executes through a rote
adapter or `rote exec --`, and uses successful traces to draft reviewed domain
SkillSpecs.
