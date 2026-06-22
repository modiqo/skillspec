# SkillSpec CLI Command Inventory

This document is the review-facing inventory for the `skillspec` CLI. The
runtime source of truth is the clap command tree in
`crates/skillspec-cli/src/main.rs`; this inventory exists so reviewers can scan
the public command surface and spot missing docs, tests, or help text.

When adding or changing a command:

1. Update the clap command definition and help text.
2. Update this inventory.
3. Add or update CLI tests for the command behavior and important help output.
4. Update README or spec docs when the command changes user workflow.

## Top Level

```text
skillspec <COMMAND>
```

| Command | Purpose |
| --- | --- |
| `validate <path>` | Validate a `skill.spec.yml` file. |
| `test <path>` | Run scenario tests declared in a SkillSpec. |
| `decide <path> --input <text> [--trace-dir <dir>]` | Evaluate routing rules for a user task and emit JSON. |
| `plan <path> --input <text> [--trace-dir <dir>]` | List selected-route execution phases in order. |
| `act <path> --input <text> [--trace-dir <dir> \| --run <run-dir>] [--phase <id>]` | Turn a SkillSpec decision into a current-route action checklist. |
| `explain <path> --input <text> [--trace-dir <dir>]` | Explain routing decisions for a user task. |
| `trace <COMMAND>` | Inspect, compact, or align SkillSpec decision traces. |
| `progress <COMMAND>` | Show or record SkillSpec execution progress for a trace run. |
| `deps <COMMAND>` | Check declared SkillSpec dependencies. |
| `imports <COMMAND>` | Validate and report SkillSpec imports. |
| `compile <path> --target <target>` | Compile a SkillSpec into harness guidance. |
| `import-skill <path> --out <path>` | Create a mechanical draft SkillSpec from a local skill file or folder. |
| `install <COMMAND>` | Detect harness roots and install SkillSpec-backed skills. |
| `capability <COMMAND>` | Manage local capability seeds for durable bootstrap. |

All commands support `-h, --help` through clap.

## `validate`

```text
skillspec validate <PATH>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

## `test`

```text
skillspec test <PATH>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

## `decide`

```text
skillspec decide [OPTIONS] <PATH> --input <INPUT>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--input <INPUT>`: user task text to route. Strip skill invocation prefixes
  before passing it.
- `--trace-dir <TRACE_DIR>`: directory where append-only decision trace events
  should be written.

## `plan`

```text
skillspec plan [OPTIONS] <PATH> --input <INPUT>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--input <INPUT>`: user task text to route. Strip skill invocation prefixes
  before passing it.
- `--trace-dir <TRACE_DIR>`: directory where append-only decision trace events
  should be written.
- `--json`: emit JSON instead of a concise human report.

`plan` emits the selected route, ordered execution phase ids, current phase,
and transition obligations. It is the pre-action view a harness can use to
know which phase names exist and in what order they should run.

## `act`

```text
skillspec act [OPTIONS] <PATH> --input <INPUT>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--input <INPUT>`: user task text to route. Strip skill invocation prefixes
  before passing it.
- `--trace-dir <TRACE_DIR>`: directory where append-only decision trace events
  should be written.
- `--run <RUN>`: existing trace run directory to associate with this action
  checklist.
- `--phase <PHASE>`: expand this execution phase instead of the first pending
  phase.
- `--json`: emit JSON instead of a concise human report.

`act` emits the current-route OODA checklist: selected route, matched rules,
current phase, allowed actions, forbids, handoffs, dependencies, and the
before-tool-call allow/deny checks.

## `explain`

```text
skillspec explain [OPTIONS] <PATH> --input <INPUT>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--input <INPUT>`: user task text to explain. Strip skill invocation prefixes
  before passing it.
- `--trace-dir <TRACE_DIR>`: directory where append-only decision trace events
  should be written.

## `trace`

```text
skillspec trace <COMMAND>
```

Subcommands:

- `compact <run-dir>`
- `align <path> --decision-trace <run-dir> [--execution-trace <jsonl>...] [--json]`

### `trace compact`

```text
skillspec trace compact <RUN_DIR>
```

Arguments:

- `<RUN_DIR>`: trace run directory produced by `decide` or `explain`
  `--trace-dir`.

### `trace align`

```text
skillspec trace align [OPTIONS] --decision-trace <DECISION_TRACE> <PATH>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--decision-trace <DECISION_TRACE>`: trace run directory produced by
  `decide` or `explain` `--trace-dir`.
- `--execution-trace <EXECUTION_TRACE>`: JSONL execution ledger with sanitized
  action evidence. Repeat for multiple ledgers.
- `--json`: emit JSON instead of a concise human report.

Current alignment compares deterministic decision-trace facts and emits a
summary before the detailed check list. The summary includes the selected
route, route-selection basis, matched rules, pass/fail/unproven counts for
deterministic checks, and pass/fail/unproven counts for execution obligations.
`unproven` means no deterministic drift was found but one or more required
facts or execution obligations still lack structured proof. Execution
obligations remain `unproven` until structured execution evidence is supplied.

## `progress`

```text
skillspec progress <COMMAND>
```

Subcommands:

- `show <path> --run <run-dir> [--json]`
- `record <run-dir> <event> [phase] [requirement] [--status <status>] [--evidence-kind <kind>] [--evidence-ref <ref>]`

### `progress show`

```text
skillspec progress show [OPTIONS] --run <RUN> <PATH>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--run <RUN>`: trace run directory produced by `plan`, `decide`, or
  `explain` with `--trace-dir`.
- `--json`: emit JSON instead of a concise human report.

`progress show` reads the decision trace and the run's `execution.jsonl`,
then writes a derived `progress.json`. It reports completed, current, blocked,
and remaining phases plus open requirements for the current phase.

### `progress record`

```text
skillspec progress record [OPTIONS] <RUN> <EVENT> [PHASE] [REQUIREMENT]
```

Arguments:

- `<RUN>`: trace run directory containing `execution.jsonl`.
- `<EVENT>`: one of `phase-started`, `requirement-started`,
  `requirement-satisfied`, `requirement-failed`, `evidence-attached`,
  `handoff-started`, `handoff-completed`, `phase-completed`, or
  `phase-blocked`.
- `[PHASE]`: phase id for phase or requirement events.
- `[REQUIREMENT]`: requirement id for requirement events.

Options:

- `--status <STATUS>`: event status, such as `pass`, `fail`, `blocked`, or
  `pending`.
- `--evidence-kind <EVIDENCE_KIND>`: evidence kind, such as `rote_response`,
  `file`, `trace`, or `command`.
- `--evidence-ref <EVIDENCE_REF>`: evidence reference, such as a rote response
  id or relative file path.
- `--source-skill <SOURCE_SKILL>`: skill that emitted this progress event.
- `--message <MESSAGE>`: human-readable event note.
- `--json`: emit JSON for the appended event.

## `deps`

```text
skillspec deps <COMMAND>
```

Subcommands:

- `check <path> [--command <id>]`

### `deps check`

```text
skillspec deps check [OPTIONS] <PATH>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--command <COMMAND>`: check only dependencies required by this command id.

## `imports`

```text
skillspec imports <COMMAND>
```

Subcommands:

- `check <path>`

### `imports check`

```text
skillspec imports check <PATH>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

## `compile`

```text
skillspec compile <PATH> --target <TARGET>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--target <TARGET>`: output target to render. Values:
  - `codex-skill`
  - `claude-skill`
  - `markdown`

## `import-skill`

```text
skillspec import-skill <PATH> --out <OUT>
```

Arguments:

- `<PATH>`: local `SKILL.md` file or skill folder to import.

Options:

- `--out <OUT>`: output path for the generated `skill.spec.yml` draft.

## `install`

```text
skillspec install <COMMAND>
```

Subcommands:

- `targets`
- `skill <folder> [--target <target>...] [--all-detected] [--dry-run] [--name <name>]`

## `capability`

```text
skillspec capability <COMMAND>
```

Capability commands manage the local seed store used by durable-executor's
`capability_bootstrap` route. The default seed store is:

```text
~/.skillspec/capabilities/
```

Set `SKILLSPEC_HOME` to override the parent directory. For example,
`SKILLSPEC_HOME=/tmp/skillspec-home` stores seeds under
`/tmp/skillspec-home/capabilities/`.

Subcommands:

- `store`
- `add <id> --domain <domain> --kind <kind> --provides <capability>...`
- `update <id> [--domain <domain>] [patch options...]`
- `list [--domain <domain>]`
- `search <capability> [--domain <domain>] [--explain] [--json] [--local-only] [--preferred-seed <id>]`
- `inspect <id> [--domain <domain>] [--json]`
- `verify <id> [--domain <domain>] [--json]`
- `prefer <id> --for <capability> [--domain <domain>] [--priority <0-100>]`
- `remove <id> [--domain <domain>]`
- `scan`

All capability subcommands emit JSON.

### `capability add`

```text
skillspec capability add <ID> --domain <DOMAIN> --kind <KIND> --provides <CAPABILITY>...
```

Important options:

- `--command <COMMAND>`: CLI command name or path.
- `--adapter <ADAPTER>`: adapter id or name.
- `--script <SCRIPT>`: local script path.
- `--alias <ALIAS>`: user phrase alias. Repeat for multiple aliases.
- `--priority <0-100>`: default ranking priority, used as a tie-breaker.
- `--preferred-for <CAPABILITY>`: capability this seed is preferred for.
- `--avoid-for <CAPABILITY>`: capability this seed should avoid.
- `--tie <KEY=VALUE>`: tie-breaker metadata such as `quality=high`.
- `--auth-env <ENV>`: auth environment variable.
- `--external-service`: mark the seed as using an external service.
- `--may-cost-money`: mark the seed as potentially spending credits or money.
- `--evidence-command <COMMAND>`: verification evidence command such as
  `<tool> --help`.
- `--suggested-skill-id <ID>`: domain SkillSpec id to draft after a successful
  trace.

Seeds are written to:

```text
~/.skillspec/capabilities/<domain>/<id>.yml
```

`add` rewrites the seed from the supplied flags. Use `update` for patch-style
changes that preserve unspecified fields.

### `capability update`

```text
skillspec capability update preferred-voice-cli --domain voice --add-provides speech_synthesis --priority 70
```

`update` patches an existing seed and preserves unspecified fields. It fails if
the seed does not exist. Patch options include:

- `--kind <KIND>`, `--command <COMMAND>`, `--adapter <ADAPTER>`, `--script <SCRIPT>`
- `--clear-command`, `--clear-adapter`, `--clear-script`
- `--add-provides <CAPABILITY>`, `--remove-provides <CAPABILITY>`
- `--add-alias <ALIAS>`, `--remove-alias <ALIAS>`
- `--priority <0-100>`, `--clear-priority`
- `--add-preferred-for <CAPABILITY>`, `--remove-preferred-for <CAPABILITY>`
- `--add-avoid-for <CAPABILITY>`, `--remove-avoid-for <CAPABILITY>`
- `--add-tie <KEY=VALUE>`, `--remove-tie <KEY>`
- `--add-auth-env <ENV>`, `--remove-auth-env <ENV>`
- `--external-service <true|false>`, `--may-cost-money <true|false>`
- `--add-evidence-command <COMMAND>`, `--remove-evidence-command <COMMAND>`
- `--suggested-skill-id <ID>`, `--clear-suggested-skill-id`
- `--mark-unverified`, `--mark-failed`

When a seed stops working for a capability, de-prioritize it without deleting
historical metadata:

```text
skillspec capability update preferred-voice-cli --domain voice --remove-preferred-for text_to_speech --add-avoid-for text_to_speech --priority 0 --mark-failed
```

### `capability search`

```text
skillspec capability search text_to_speech --domain voice --explain --json
```

Search ranks matching seeds with deterministic, explainable scoring. It returns
candidate ids, paths, scores, reasons, risk flags, and required gates. When the
top two candidates are within 10 points, `selected` is `null` and `ask_policy`
explains that the agent should ask the user instead of auto-picking.

`--local-only` filters out external-service seeds. `--preferred-seed <id>` adds
an explicit preference bonus but does not bypass verification or risk gates.

An empty result for the first capability/domain pair is not permission to use an
unseeded local fallback. Durable agents should preserve the empty result as
evidence, broaden through related capability and domain terms, and search again.
For voice/audio work, related terms commonly include `voice`, `text_to_speech`,
`voice_generation`, `speech_synthesis`, `audio_generation`, and
`voice_message` across plausible domains such as `voice` and `audio`. If no
seed is found after related searches, ask before using the fallback or create
and verify a seed for that fallback first.

### `capability verify`

```text
skillspec capability verify preferred-voice-cli --domain voice --json
```

Verification runs declared low-level evidence checks, such as path lookup for a
CLI command and evidence commands like `<tool> --help`. The seed file is updated
with verification status and outcomes.

### `capability prefer`

```text
skillspec capability prefer preferred-voice-cli --domain voice --for text_to_speech --priority 90
```

`prefer` updates ranking metadata without editing durable-executor's spec. This
is the supported way to make a newly installed CLI preferred for a capability.

### `install targets`

```text
skillspec install targets
```

Lists detected harness skill roots.

### `install skill`

```text
skillspec install skill [OPTIONS] <FOLDER>
```

Arguments:

- `<FOLDER>`: generated skill folder containing `SKILL.md` and
  `skill.spec.yml`.

Options:

- `--target <TARGET>`: harness target to install into. Repeat for multiple
  targets. Values:
  - `agents`
  - `codex`
  - `claude-local`
- `--all-detected`: install into every harness root detected on this machine.
- `--dry-run`: show the install plan without writing files.
- `--name <NAME>`: override the installed skill folder name.
