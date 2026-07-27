# Boundary Guard Hook

Status: proposed. Nothing in this document is implemented.

## Purpose

Give a boundary proposal somewhere to be enforced, for skills the user already
has installed and does not intend to modify.

Documents 36 through 42 produce a report and a policy artifact. Without this
document, the last mile is a human copying a generated snippet into a
configuration file they may not have, in a grammar that may not support deny, and
then never learning whether it worked. That is a compiler with no consumer.

The guard hook closes that gap using machinery this repository already has.

## Why This Is Not "Adopt The Execution System"

The earlier position in this design was that enforcement required SkillSpec's
runtime, and that requiring it would kill adoption. Both halves were wrong.

`skillspec router guard` already installs a managed hook into Codex `hooks.json`
and Claude Code `settings.json`, returns a structured block decision, and removes
only handlers whose command is its own managed command. That lifecycle is
described in `docs/design/router/28-router-guard-hooks.md` and implemented in
`crates/skillspec-harness/src/router_lifecycle/hooks.rs`.

A boundary guard is the same shape at a different lifecycle event. It requires:

- no `skill.spec.yml`
- no compile
- no import
- no change to the skill being guarded
- no change to how the user invokes their skills

The user keeps using the prose skills they already have. The guard sits in front
of tool calls and refuses ones the reviewed policy does not cover.

## Guarantee

When a boundary policy is installed and the managed hook is present for the
active harness:

- the guard runs before a tool call is executed;
- a call matching an `allow` grant proceeds;
- a call matching a `permission_required_for` grant is surfaced for approval;
- a call matching nothing is refused, because the policy default is deny;
- every decision is appended to a local decision log.

The guarantee is bounded exactly as the router guard's is. It applies to
harnesses exposing a pre-tool lifecycle hook that SkillSpec can install and
manage. It does not constrain a harness that does not run hooks, a session
started before installation, a user who disables hooks, or anything a skill does
through a channel the harness does not route through a tool call.

That last clause matters and belongs in the CLI output, not only here. A guard
on tool calls does not constrain a subprocess that a permitted tool call spawns.
Enforcement stops at the boundary the harness itself draws.

## Harness Mapping

Follow the existing target resolution in `hook_targets_for_roots`:

| Harness | File | Event |
| --- | --- | --- |
| Claude Code | `settings.json` | `PreToolUse` |
| Codex | `hooks.json` | pre-tool equivalent, to be verified |

The router guard uses `UserPromptSubmit`. The event name, payload shape, and
decision grammar for the pre-tool event must be verified against current harness
documentation before implementation, under the same standing task that governs
the emitters in document 37. Do not infer the payload shape from the
`UserPromptSubmit` implementation.

## Policy Store

Policies live under `$SKILLSPEC_HOME/boundary/`, defaulting to
`~/.skillspec/boundary/`, matching how router and durable state already resolve.

```text
~/.skillspec/boundary/
  config.json                 enabled flag, mode, managed hook state
  policies/<install-slug>.json  one reviewed policy per guarded skill
  decisions.jsonl             append-only decision log
```

One policy per installed skill, keyed by install slug so the same identity the
router and installer already use is reused rather than invented.

A policy records the reviewed grant set, the `source_content_sha256` of the
package it was reviewed against, the reviewing user's decision on each
`permission_required_for` entry, and a timestamp. The content hash is what makes
re-review on update possible: a guarded skill whose bytes no longer match its
policy is a skill whose policy was approved for different content.

## Modes

| Mode | Behavior on a call matching no grant |
| --- | --- |
| `observe` | Allow, record. Nothing is blocked. |
| `prompt` | Surface for approval; remember the answer for the session. |
| `enforce` | Refuse, with the grant that would have been needed. |

`observe` is the default on install, and this is a deliberate product decision
rather than a weak one. A guard that blocks on day one against a policy nobody
has validated will be uninstalled within an hour, and a security control that is
uninstalled provides zero protection - strictly less than an observing one that
stays.

`observe` also produces the evidence that no other level of this design can:
`decisions.jsonl` is a record of what a skill actually did, which is exactly the
input validation level V2 in document 38 is blocked on. A week in observe mode
turns a static proposal into one checked against real behavior, and the
promotion prompt writes itself:

```text
14 days in observe mode. 61 calls, all covered by the current policy.
Promote to enforce?   skillspec boundary guard mode enforce
```

Mode is per-skill, so one skill can be enforced while others observe.

## Decision Log

`decisions.jsonl` is append-only, local, and never transmitted.

```json
{"ts":"2026-07-27T10:14:22Z","skill":"pdf-report","tool":"Bash",
 "effect":"net.egress:api.github.com","grant":"allow","decision":"allowed",
 "mode":"observe"}
```

The effect field is the guard's normalization of the intercepted call into the
same vocabulary the analysis uses. That shared vocabulary is what allows a
recorded call to be compared against a proposed grant at all, and it is the
reason the effect classes in document 36 were constrained to things a permission
system can express.

## Lifecycle

```text
skillspec boundary guard install     write config, install managed hook entries
skillspec boundary guard status      config, hook state, per-skill mode
skillspec boundary guard mode <m>    set mode globally or per skill
skillspec boundary guard review      walk pending permission_required_for items
skillspec boundary guard log         read decisions.jsonl
skillspec boundary guard uninstall   remove managed hook entries and config
```

Reuse the router's manifest-scoped mutation discipline exactly: SkillSpec removes
only hook handlers whose command is its own managed guard command, and leaves
every user hook in place. This is already implemented and tested for the router;
the boundary guard must not introduce a second, less careful mechanism.

## Failure Behavior

A guard that errors must not silently permit.

| Condition | Behavior |
| --- | --- |
| Policy file missing for a guarded skill | `observe` and warn; do not block a user over a missing file |
| Policy hash does not match package | Surface for re-review; in `enforce`, refuse until reviewed |
| Guard command fails to run | Harness-dependent. Record the exposure in `status` and state it plainly in the CLI |
| Call cannot be normalized into an effect | Treat as matching no grant, consistent with `PathClass::Unknown` |

The third row is an honest limitation rather than a design: if the hook process
cannot run, SkillSpec does not control what the harness does next. `status` must
report this rather than implying continuous coverage.

## What This Does Not Become

- **Not a sandbox.** It is an advisory gate at a lifecycle event a harness
  chooses to expose. It has no kernel involvement, no namespace isolation, no
  network interposition.
- **Not the execution system.** It does not select routes, render phases, plan,
  or record alignment. It observes tool calls against a policy.
- **Not a replacement for real isolation.** Where a container or egress proxy is
  available, that is a stronger substrate and the `egress-allowlist` target in
  document 37 exists to feed it.

## Sources

- `crates/skillspec-harness/src/router_lifecycle/hooks.rs` -
  `hook_targets_for_roots`, `install_hook`, `remove_hook`, `inspect_hook`, and
  the managed-command scoping to reuse.
- `docs/design/router/28-router-guard-hooks.md` - the lifecycle, the bounded
  guarantee wording, and the harness mapping this document follows.
- `docs/design/runtime/11-execution-progress-ledger.md` - the existing
  append-only local evidence pattern `decisions.jsonl` mirrors.
- `docs/design/security/38-boundary-validation.md` - validation level V2, which
  `decisions.jsonl` may unblock.
