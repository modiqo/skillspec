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
| `explain <path> --input <text> [--trace-dir <dir>]` | Explain routing decisions for a user task. |
| `trace <COMMAND>` | Inspect, compact, or align SkillSpec decision traces. |
| `deps <COMMAND>` | Check declared SkillSpec dependencies. |
| `imports <COMMAND>` | Validate and report SkillSpec imports. |
| `compile <path> --target <target>` | Compile a SkillSpec into harness guidance. |
| `import-skill <path> --out <path>` | Create a mechanical draft SkillSpec from a local skill file or folder. |
| `install <COMMAND>` | Detect harness roots and install SkillSpec-backed skills. |

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
- `align <path> --decision-trace <run-dir> [--json]`

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
- `--json`: emit JSON instead of a concise human report.

Current alignment compares deterministic decision-trace facts and emits
execution obligations as `unproven` until structured execution evidence is
supplied.

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
