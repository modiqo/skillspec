# Router Guard Hooks

Router mode needs an enforcement point before native implicit skill selection.
Visibility metadata makes the router the preferred first hop, but an active
harness can still miss filesystem drift until it reloads or until SkillSpec
repairs managed roots. Router guard hooks add a prompt-time repair and block
layer for harnesses that expose prompt lifecycle hooks.

## Guarantee

When router mode is enabled and the managed hook is installed for the active
harness:

- `skillspec router guard` runs before the model processes each user prompt.
- If the router index is missing, stale, or out of sync with current roots, the
  guard reapplies router-managed visibility and rebuilds the index.
- New or changed out-of-band skills are made explicit-only/manual-only through
  the same visibility path used by `skillspec router index refresh`.
- The guard reports `first_hop_ready: true` only when router config is enabled,
  managed router skill files are present, the index exists, and the index is not
  stale after repair.
- If an enabled router cannot be made ready, the hook blocks the prompt and
  gives a concrete repair command.

This is stronger than prose in the router skill, but it is still bounded by the
harness hook contract. It applies to Codex and Claude Code prompt hooks that
SkillSpec can install and manage. It does not make unsupported harnesses obey
router mode, and it cannot run if users disable hooks or run a stale session
that has not loaded them.

## Implemented Lifecycle

`skillspec router install`, `enable`, and enabled `update` install managed
Codex and Claude hook entries inferred from the configured skill roots.

`skillspec router disable` removes managed hook entries without uninstalling
the router package or deleting the index.

`skillspec router uninstall` removes managed hook entries, restores visibility,
removes managed router skill directories, and removes router config.

Hook mutation is manifest-like but scoped: SkillSpec removes only hook handlers
whose command is the managed router guard command. Existing user hooks remain in
place.

The lifecycle reports include `harness_hooks` entries with harness, path,
status, command, and message. `skillspec status --json` also reports the current
managed hook state through router status.

## Durable Executor

Router guard hooks are discovery and freshness guardrails, not durable
execution envelopes.

When `durable-executor` is installed and enabled, router visibility keeps it as
the implicit durable first-hop exception. A later opt-in mode can let the guard
handoff to durable-executor for observe/record/memorize behavior, but that must
be user-elicited because it changes what is recorded and retained.

This implementation reports durable availability and leaves the
observe/record/memorize hook mode disabled unless explicitly requested.

## Harness Mapping

Codex uses `UserPromptSubmit` hooks in `hooks.json`. SkillSpec installs a
managed command hook into the relevant `hooks.json`.

Claude Code uses `UserPromptSubmit` hooks in `settings.json`. SkillSpec installs
a managed command hook into the relevant settings file.

For shared `.agents/skills` roots, SkillSpec installs the Codex hook next to
the matching `.codex` config layer and installs the Claude hook only when a
matching `.claude` config directory is present.

Implemented root mapping:

- `<base>/.codex/skills` -> `<base>/.codex/hooks.json`
- `<base>/.claude/skills` -> `<base>/.claude/settings.json`
- `<base>/.agents/skills` -> `<base>/.codex/hooks.json`
- `<base>/.agents/skills` -> `<base>/.claude/settings.json` only when
  `<base>/.claude` already exists

## Hook Command

Managed hooks call:

```sh
skillspec router guard --config <router-config> --hook
```

`--config` pins the hook to the router config written by install, so the guard
does not depend on the current session's `SKILLSPEC_HOME`.

`--hook` emits harness hook JSON. On success it injects compact context telling
the model that router mode is ready and must be the first hop. On failure it
emits a blocking decision with the repair command.

Normal diagnostic mode is:

```sh
skillspec router guard --config <router-config> --json
```

That mode reports `repaired`, `first_hop_ready`, index status before/after, and
any visibility/index repair output.

## Local Verification

Verified locally on 2026-06-28 with temporary `HOME` and `SKILLSPEC_HOME`:

- Codex hook installs into the expected `hooks.json` and preserves unrelated
  hook commands.
- Claude hook installs into the expected `settings.json` and preserves
  unrelated hook commands.
- Adding an out-of-band skill under a managed root makes the next
  `skillspec router guard --json` run refresh visibility and rebuild the index.
- `skillspec router guard --hook` emits `UserPromptSubmit` hook output when
  `first_hop_ready` is true.
- `skillspec router disable` removes only managed router guard hook entries.
- `skillspec router uninstall` is covered by the lifecycle test and removes
  only managed router guard hook entries while restoring visibility.
- Durable-executor remains a visibility exception when enabled, but
  observe/record/memorize hook mode is not enabled by this feature.
