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
| `sensemake <path> [--view <view>] [--json]` | Teach the shape of one SkillSpec and its progressive navigation handles. |
| `query <path> <handle> [--view <view>] [--json]` | Query one SkillSpec collection, item, or field path. |
| `refs <path> <handle> [--view <view>] [--json]` | Show outgoing SkillSpec references for one item handle. |
| `doctor <target> [--json]` | Scan one prose skill folder, local or public GitHub, for static reliability and context-burden debt without executing it. |
| `source <COMMAND>` | Map and query source packages for progressive import. |
| `grammar <COMMAND>` | Teach the embedded grammar and semantic porting workflow. |
| `trace <COMMAND>` | Inspect, compact, or align SkillSpec decision traces. |
| `progress <COMMAND>` | Show or record SkillSpec execution progress for a trace run. |
| `deps <COMMAND>` | Check declared SkillSpec dependencies. |
| `imports <COMMAND>` | Validate and report SkillSpec imports. |
| `compile <path> --target <target>` | Compile a SkillSpec into harness guidance. |
| `import-skill <path> --out <path> [--source-map <path>]` | Create a mechanical draft SkillSpec from a local skill file or folder, optionally gated by a fresh source map. |
| `synthesize-from-workspace <workspace> --out <folder>` | Rote-specific optional integration that synthesizes a draft SkillSpec from durable rote workspace stats, command log, metadata, and optional dependency evidence. |
| `index --roots <path>... --out <index-file-or-router-dir>` | Build a searchable skill catalog outside model context. |
| `route --index <index-file-or-router-dir> --query <text>` | Route a user request to candidate skills from an index. |
| `skills <COMMAND>` | Audit or control installed skill visibility. |
| `visibility <COMMAND>` | Plan, apply, or restore harness-native skill visibility controls. |
| `router <COMMAND>` | Install, uninstall, refresh, or inspect the optional skill router. |
| `durable-executor <COMMAND>` | Install, update, or delete the optional durable first-hop skill. |
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
current phase, `PHASE TOOL BOUNDARY - HARD`, allowed actions, forbids,
handoffs, dependencies, and the before-tool-call allow/deny checks. The tool
boundary is inherited from `entry.tool_boundary`, selected-route
`tool_boundary`, and phase `tool_boundary`. If no boundary is declared, the
report still renders a conservative default-deny boundary and requires
permission for any unlisted tool, data source, execution substrate, provider,
adapter, CLI, browser mode, API, or skill.

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

## `sensemake`

```text
skillspec sensemake [OPTIONS] <PATH>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.

Options:

- `--view <VIEW>`: output detail level. Values are `index`, `summary`, and
  `full`. Defaults to `index`.
- `--json`: emit JSON instead of a concise human report.

`sensemake` is the progressive-disclosure entry point for a spec. It reports
section counts, ids, query handles, and navigation commands so an agent can
inspect only the route, rule, command, state, dependency, or proof detail
needed for the active task.

## `query`

```text
skillspec query [OPTIONS] <PATH> <HANDLE>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.
- `<HANDLE>`: collection, item, or field handle, such as `routes`,
  `rule:<id>`, `command:<id>.requires`, or `state:<id>`.

Options:

- `--view <VIEW>`: output detail level. Values are `index`, `summary`, and
  `full`. Defaults to `summary`.
- `--json`: emit JSON instead of a concise human report.

`query` lets harnesses retrieve specific pieces of a spec without loading the
whole YAML into model context.

## `refs`

```text
skillspec refs [OPTIONS] <PATH> <HANDLE>
```

Arguments:

- `<PATH>`: path to a `skill.spec.yml` file.
- `<HANDLE>`: item handle, such as `rule:<id>`, `command:<id>`,
  `state:<id>`, or `recipe:<id>`.

Options:

- `--view <VIEW>`: output detail level. Values are `index`, `summary`, and
  `full`. Defaults to `summary`.
- `--json`: emit JSON instead of a concise human report.

`refs` reports outgoing references from an item, such as route checks, command
dependencies, rule preferences, phase requirements, and transition edges.

## `doctor`

```text
skillspec doctor <TARGET> [--json]
```

Arguments:

- `<TARGET>`: local `SKILL.md` file, local single skill folder, or public
  GitHub single skill folder URL/shorthand to inspect.

Options:

- `--json`: emit JSON instead of a concise human report.

`doctor` is a static diagnostic. It reads files and Markdown AST structure; it
does not execute scripts, call tools, invoke a model, or simulate a task. It
reports a structural score, activation-loaded surface percentage,
frontmatter/activation/deferred surface split, and issue list.

For public GitHub targets, `doctor` performs a temporary sparse checkout of the
requested folder and removes it after the report. Supported remote forms include
`https://github.com/owner/repo/tree/main/path/to/skill` and
`owner/repo/path/to/skill`. `doctor` requires exactly one `SKILL.md` under the
target; parent folders with many skills are rejected for both local and remote
inputs.

The issue model is grounded in the reliability-gap and contract-trace
methodology: instruction-density degradation, primacy-bias exposure, code mixed
with activation instructions, ambiguous code fences, implicit dependency
contracts, missing local references, missing behavior contracts, and missing
trace/proof surfaces.

The verdict is about context and reliability burden, not observed runtime
failure. Dynamic behavior remains `unproven` until a trace can be aligned.

## `source`

```text
skillspec source <COMMAND>
```

Subcommands:

- `map <path> --out <dir> [--json]`
- `query <source-map.json> <handle> [--view <index|summary|full>] [--json]`
- `coverage <source-map.json> [--json]`
- `stale <source-map.json> [--root <path>] [--json]`

`source` is the progressive reader for prose skills and other import sources.
It uses a Markdown AST with source positions to write `source-map.json` and a
human-readable `source-map.md`. Agents should run it before importing large or
resource-heavy skills, then query structural handles and exact spans instead of
loading the entire source into model context.

### `source map`

```text
skillspec source map <PATH> --out <OUT>
```

Arguments:

- `<PATH>`: local `SKILL.md` file or skill folder to map.

Options:

- `--out <OUT>`: output directory for `source-map.json` and `source-map.md`.
- `--json`: emit JSON instead of a concise human report.

The generated map records files, hashes, Markdown nodes, frontmatter, byte and
line ranges, local/external references, code blocks, dependency mentions, modal
obligation language, and review-required classifications. It preserves
frontmatter as a first-class node instead of treating it as Markdown prose.

### `source query`

```text
skillspec source query <MAP> <HANDLE> [--view <VIEW>]
```

Arguments:

- `<MAP>`: path to `source-map.json`.
- `<HANDLE>`: collection or item handle. Common handles are `files`, `nodes`,
  `classifications`, `references`, `dependencies`, `code`, `coverage`,
  `coverage:review_required`, `frontmatter:<file>`, `heading:<file>.<slug>`,
  and `code:<file>.<n>`.

Options:

- `--view <VIEW>`: output detail level. Values are `index`, `summary`, and
  `full`. Defaults to `summary`.
- `--json`: emit JSON instead of a concise human report.

Use `--view index` to inspect structural headings and code blocks, `--view
summary` to inspect compact classifications with line ranges and previews, and
`--view full` on a specific node handle to recover the exact source span.

### `source coverage`

```text
skillspec source coverage <MAP>
```

Arguments:

- `<MAP>`: path to `source-map.json`.

Options:

- `--json`: emit JSON instead of a concise human report.

Coverage summarizes mapped nodes, review-required classifications, and stale
file counts. Import review should not proceed until the agent has inspected
review-required source classifications that affect routes, rules, commands,
dependencies, imports, resources, recipes, tests, or proof.

### `source stale`

```text
skillspec source stale <MAP> [--root <ROOT>]
```

Arguments:

- `<MAP>`: path to `source-map.json`.

Options:

- `--root <ROOT>`: source root to compare against. Defaults to the map's
  recorded source root.
- `--json`: emit JSON instead of a concise human report.

`source stale` exits non-zero when any mapped file is missing or has a different
hash. Run it before import, proof, or install when source files may have changed.

## `grammar`

```text
skillspec grammar <COMMAND>
```

Subcommands:

- `sensemake [--view <index|summary|porting|full>] [--json]`
- `checklist [--for import-skill] [--json]`
- `schema [--json]`

### `grammar sensemake`

```text
skillspec grammar sensemake [OPTIONS]
```

Options:

- `--view <VIEW>`: output detail level. Values are `index`, `summary`,
  `porting`, and `full`. Defaults to `index`.
- `--json`: emit JSON instead of a concise human report.

### `grammar checklist`

```text
skillspec grammar checklist [OPTIONS]
```

Options:

- `--for <SUBJECT>`: checklist workflow. The current value is
  `import-skill`.
- `--json`: emit JSON instead of a concise human report.

### `grammar schema`

```text
skillspec grammar schema [OPTIONS]
```

Options:

- `--json`: emit the embedded JSON Schema instead of the concise summary.

The grammar commands are for authors and reviewers. They teach the current
typed grammar, semantic porting checklist, and embedded JSON Schema before a
harness imports or revises a SkillSpec.

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
The command writes the full report to `<DECISION_TRACE>/alignment.json`.
The human report also includes a completion-facing summary:

```text
alignment_summary:
  Decision replay: pass
  Phase order: pass
  Requirements: 4/5 proven
  Missing proof: requirement `install_codex` has no progress event
  Forbidden actions: no violations recorded
  Alignment: partial
token_usage:
  Token consumption: total 1234 tokens
  Token savings: 3729702 tokens saved by query reduction (4439892 cached response tokens reduced to 710190 query-result tokens, 84.0% reduction)
```

`Alignment: partial` is the user-facing label for a non-failing `status:
unproven`. Token usage is always shown; absent stats are reported as `not
recorded`. Query-reduction stats are reported as cached response tokens reduced
to extracted query-result tokens, plus the saved-token delta and reduction
percentage.

## `progress`

```text
skillspec progress <COMMAND>
```

Subcommands:

- `show <path> --run <run-dir> [--json]`
- `record <run-dir> <event> [phase] [requirement] [--status <status>] [--evidence-kind <kind>] [--evidence-ref <ref>]`
- `stats <run-dir> [--workspace <workspace>] [--workspace-stats-report <path>] [--workspace-stats-json <path>] [--phase <phase>] [--requirement <requirement>...] [token options...]`
- `final-response <run-dir> [--phase <phase>] [--requirement <requirement>...] --result --evidence --alignment --token-savings [--message <message>] [--json]`

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
  `requirement-satisfied`, `requirement-failed`, `stats-collected`,
  `obligation-satisfied`, `route-fulfilled`, `after-success-completed`,
  `evidence-attached`, `handoff-started`, `handoff-completed`,
  `phase-completed`, or `phase-blocked`.
- `[PHASE]`: phase id for phase or requirement events.
- `[REQUIREMENT]`: requirement id for requirement events.

Options:

- `--id <ID>`: obligation, route, closure, or elicitation id for
  `obligation-satisfied`, `route-fulfilled`, `after-success-completed`, and
  related proof events.
- `--status <STATUS>`: event status, such as `pass`, `fail`, `blocked`, or
  `pending`.
- `--evidence-kind <EVIDENCE_KIND>`: evidence kind, such as `rote_response`,
  `file`, `trace`, or `command`.
- `--evidence-ref <EVIDENCE_REF>`: evidence reference, such as a rote response
  id or relative file path.
- `--source-skill <SOURCE_SKILL>`: skill that emitted this progress event.
- `--message <MESSAGE>`: human-readable event note.
- `--json`: emit JSON for the appended event.

### `progress stats`

```text
skillspec progress stats [OPTIONS] <RUN>
```

Arguments:

- `<RUN>`: trace run directory containing `execution.jsonl`.

Options:

- `--workspace <WORKSPACE>`: rote workspace name. When omitted, the command
  reads `name` from `--workspace-stats-json` or `Workspace:`/`Name:` from
  `--workspace-stats-report` if present.
- `--phase <PHASE>`: phase id whose requirement(s) this stats event satisfies.
- `--requirement <REQUIREMENT>`: requirement id satisfied by this stats event.
  Repeat for multiple requirements. Requires `--phase`.
- `--workspace-stats-report <PATH>`: human-readable report produced by
  `rote workspace stats <workspace>`.
- `--workspace-stats-json <PATH>`: JSON file produced by
  `rote workspace stats <workspace> --json`. This remains supported for
  compatibility, but durable executor defaults to the report form.
- `--total-tokens <N>`: total API request+response tokens.
- `--context-tokens <N>`: one-time context-window tokens consumed during
  exploration.
- `--query-result-tokens <N>`: tokens in extracted query results.
- `--response-tokens-cached <N>`: cached response/source tokens before query
  reduction.
- `--saved-tokens <N>`: tokens saved by query reduction or cache reuse.
- `--reduction-percent <PCT>`: percent reduction from cached/source tokens to
  query-result tokens.
- `--message <MESSAGE>`: human-readable event note.
- `--json`: emit JSON for the appended event.

`progress stats` appends a machine-readable `stats_collected` event to
`<RUN>/execution.jsonl` so `trace align` can report numeric token consumption
and savings. It understands the current rote workspace stats JSON shape, including
`metrics.total_tokens`, `metrics.context_tokens`,
`token_savings.source_tokens`, `token_savings.result_tokens`, and
`token_savings.tokens_saved`, and the human report labels `Total tokens`,
`Context tokens`, `Source tokens`, `Result tokens`, and `Tokens saved`. The
command requires `--workspace-stats-report`, `--workspace-stats-json`, or at
least one explicit token metric; it will not create an empty
`stats_collected` event. When `--phase` and `--requirement` are supplied, it
also appends matching `requirement_satisfied` events so phase completion
summaries can prove the stats requirements without manual JSONL edits.

### `progress final-response`

```text
skillspec progress final-response [OPTIONS] <RUN>
```

Arguments:

- `<RUN>`: trace run directory containing `execution.jsonl`.

Options:

- `--result`: final response includes the direct result.
- `--evidence`: final response includes evidence handles or files.
- `--alignment`: final response includes the alignment summary.
- `--token-savings`: final response includes token usage and token savings.
- `--phase <PHASE>`: phase id whose requirement(s) this final response event
  satisfies.
- `--requirement <REQUIREMENT>`: requirement id satisfied by this final
  response event. Repeat for multiple requirements. Requires `--phase`.
- `--message <MESSAGE>`: human-readable event note.
- `--json`: emit JSON for the appended event.

`progress final-response` appends the `final_response_sent` event shape that
`trace align` uses to prove the final report included evidence, alignment, and
token-savings sections. Run it after drafting those sections and before the
final answer, then rerun `trace align` and report the rerun alignment summary.
When `--phase` and `--requirement` are supplied, it also appends matching
`requirement_satisfied` events so completion summaries can prove the final
report closure requirements.

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
skillspec import-skill <PATH> --out <OUT> [--source-map <SOURCE_MAP>]
```

Arguments:

- `<PATH>`: local `SKILL.md` file or skill folder to import.

Options:

- `--out <OUT>`: output path for the generated `skill.spec.yml` draft.
- `--source-map <SOURCE_MAP>`: optional fresh `source-map.json` produced by
  `skillspec source map` for the same source. When supplied, import refuses to
  run if any mapped file is stale or missing.

Notes:

- For large or code-heavy skills, first run `skillspec source map`, inspect
  `source coverage`, query `nodes`, `dependencies`, `code`, and exact
  `--view full` handles as needed, then pass `--source-map` to `import-skill`.
- The generated file is a scaffold for semantic review, not a finished port.
- Fenced code blocks are materialized under `resources/imported-code/` next to
  the output draft and referenced from `code.source.file` with resource
  provenance.
- The original prose `SKILL.md` is preserved beside the draft as
  `source/SKILL_md.old` and referenced as source material. Do not preserve it as
  `source/SKILL.md` or another Markdown-looking `SKILL*.md` file, because
  harnesses and scanners may discover or rank Markdown skill-looking files.
- The command writes a scaffolded `deps.toml` beside the generated
  `skill.spec.yml`, declares it as a file dependency/artifact, and infers simple
  CLI plus fenced-code package dependencies from Python and JavaScript/TypeScript
  imports.
- The generated ledger is review scaffolding. Before proof or install, complete
  it with package mentions from source prose, references, helper scripts,
  command examples, and manifests, preserving authority, local status, install
  risk, and degraded proof impact.
- If no dependencies are found after review, keep the generated ledger with
  `dependency_count = 0`; do not replace it with a byte-empty `deps.toml`.

## `synthesize-from-workspace`

```text
skillspec synthesize-from-workspace <WORKSPACE> --out <OUT>
```

Arguments:

- `<WORKSPACE>`: durable rote workspace name created by durable execution.

Options:

- `--out <OUT>`: output skill folder. The command writes `skill.spec.yml` and
  `resources/observed-workspace/`.
- `--task <TASK>`: original user task that created the durable workspace.
- `--name <NAME>`: optional generated skill id/name.
- `--log-last <N>`: number of command-log rows to collect when
  `--workspace-log` is omitted. Defaults to `50`.
- `--workspace-stats-report <PATH>`: pre-captured output from
  `rote workspace stats <workspace>`.
- `--workspace-log <PATH>`: pre-captured output from
  `rote workspace inspect log --last <n>`.
- `--workspace-meta <PATH>`: pre-captured output from
  `rote workspace inspect meta`.
- `--workspace-deps <PATH>`: optional pre-captured output from
  `rote workspace inspect deps`.
- `--force`: overwrite an existing `skill.spec.yml` in the output folder.
- `--json`: emit JSON instead of a concise text report.

Notes:

- This is a rote-specific optional integration, not a generic SkillSpec
  workspace importer. It hinges on durable execution having already created a
  rote workspace.
- Without pre-captured files, it collects evidence through `rote workspace
  stats <workspace>`, `rote workspace inspect log --last <n>`, and
  `rote workspace inspect meta`.
- Synthesis fails when stats do not name the requested workspace, when the
  command log has no entries, or when metadata is empty.
- The generated SkillSpec is a reviewed scaffold. It preserves stats, log,
  metadata, optional dependency graph, a workspace report, and a coverage matrix
  under resources. Observed command templates and dependencies are inferred
  conservatively and remain review-required before replay, install, or release.

## `install`

```text
skillspec install <COMMAND>
```

Subcommands:

- `targets`
- `skill <folder> [--target <target>...] [--all-detected] [--dry-run] [--name <name>] [--force] [--retire-existing]`

## `index`

```text
skillspec index --roots <ROOTS>... --out <OUT>
```

Options:

- `--roots <ROOTS>`: skill roots to scan. Repeat or pass multiple paths.
- `--out <OUT>`: SQLite index file to write, or a router directory where
  `skill-index.sqlite` should be written.
- `--visibility-manifest <VISIBILITY_MANIFEST>`: optional manifest whose final
  states override native metadata. This is how explicit `off` states exclude
  skills from router results when a native harness has no off state.
- `--json`: emit JSON instead of a concise human report.

The indexer scans `SKILL.md` frontmatter, optional `agents/openai.yaml`, Claude
`.claude/settings.json` skill overrides, Claude `disable-model-invocation`
frontmatter, and optional `skill.spec.yml` routing metadata. It stores skill
text, routing hints, visibility, checksums, and source paths in SQLite.

## `route`

```text
skillspec route --index <INDEX> --query <QUERY>
```

Options:

- `--index <INDEX>`: SQLite index file created by `skillspec index` or
  `skillspec router index refresh`, or a router directory containing
  `skill-index.sqlite`.
- `--query <QUERY>`: user task text to route.
- `--top <TOP>`: number of candidates to return. Defaults to 5.
- `--execution-mode <direct|durable>`: execution mode already selected by the
  user or caller.
- `--json`: emit JSON instead of a concise human report.

The route result includes selected skill, candidates, scores, confidence,
visibility, SkillSpec-backed status, and an
`execution_mode_direct_or_durable` elicitation hint when no execution mode was
supplied and a candidate was selected.

## `skills`

```text
skillspec skills <COMMAND>
```

Subcommands:

- `audit --roots <path>... [--json]`
- `set-visibility <skill> <visibility> --roots <path>... --manifest <path> [--dry-run] [--json]`
- `disable <skill> --roots <path>... --manifest <path> [--dry-run] [--json]`
- `enable <skill> --roots <path>... --manifest <path> [--dry-run] [--json]`

Visibility values are `implicit`, `manual-only`, `name-only`, and `off`.
Visibility commands use native Codex or Claude controls where available and
write a reversible manifest. Shared `.agents/skills` roots receive both Codex
`agents/openai.yaml` controls and Claude `disable-model-invocation`
frontmatter, because those roots may be symlinked into more than one harness.

## `visibility`

```text
skillspec visibility <COMMAND>
```

Subcommands:

- `plan --roots <path>... [--profile router-managed] [--json]`
- `apply --roots <path>... --manifest <path> [--profile router-managed] [--dry-run] [--json]`
- `restore --manifest <path> [--dry-run] [--json]`

`restore` uses exact file snapshots from the manifest. It does not infer
previous visibility state from current files.

## `router`

```text
skillspec router <COMMAND>
```

Subcommands:

- `install --roots <path>... --index <index-file-or-router-dir> [--manifest <path>] [--router-name <name>] [--dry-run] [--json]`
- `update [--backup-dir <path>] [--dry-run] [--json]`
- `uninstall` or `delete` `[--manifest <path>] [--index <index-file-or-router-dir>] [--keep-index] [--dry-run] [--json]`
- `index refresh --roots <path>... --index <index-file-or-router-dir> [--visibility-manifest <path>] [--json]`
- `index status --roots <path>... --index <index-file-or-router-dir> [--visibility-manifest <path>] [--json]`

`router install` writes an explicit-only SkillSpec-backed `skill-router` skill
into every configured `--roots` path, a visibility manifest, a SQLite index, and a
router config. The generated router package uses a thin `SKILL.md` loader plus
`skill.spec.yml`; the YAML file is the router contract. The router-managed
visibility profile makes every indexed skill explicit-only except
`durable-executor`. If `durable-executor` is present in the managed roots, it is
kept implicit. If it is missing, install still succeeds and reports that durable
first-hop execution is unavailable until durable-executor is installed
separately. After building the index, install runs `index status` internally and
reports preparedness; a prepared router has a present, non-stale index whose
indexed skill count matches the discovered skill count. Once that config exists,
successful `skillspec install skill` calls reapply router-managed visibility,
refresh the configured index, and run the same preparedness check. `router
uninstall` restores visibility from the manifest and removes only generated
router skills that contain the managed marker file.

`router update` is the maintenance path for an existing router config. It
backs up the router config, visibility manifest, SQLite index, and managed
router skill directories before rewriting the SkillSpec-backed router package in
every recorded harness root. It then reapplies router-managed visibility,
rebuilds the index, reruns the preparedness check, and prints a warning to
restart active Codex, Claude, Agents, or vendor harness sessions so their loaded
skill metadata is refreshed.

Out-of-band skill additions are detected by `router index status` and repaired
by `router index refresh`. Status is read-only: it reports new, changed, and
missing skills, annotates each changed entry as prose-only or SkillSpec-backed,
and emits `skillspec import-skill` advice for prose-only packages. Refresh is
the mutating repair path: when router config is present, it reapplies
router-managed explicit invocation controls across the roots, preserves an
installed `durable-executor` as the implicit exception, rebuilds the index, and
runs the preparedness check. SkillSpec-backed additions are indexed directly;
prose-only additions are also made explicit-only and indexed, with conversion
advice retained in the report.

## `durable-executor`

```text
skillspec durable-executor <COMMAND>
```

Subcommands:

- `install <source-folder> [--target <target>...] [--all-detected] [--dry-run] [--force] [--json]`
- `update [--source <source-folder>] [--backup-dir <path>] [--dry-run] [--json]`
- `delete` or `uninstall` `[--dry-run] [--json]`

`durable-executor install` installs the optional first-hop skill from an
explicit local source folder. The source must contain `SKILL.md` with
`name: durable-executor` plus `skill.spec.yml`. The command installs the package
as `durable-executor`, writes a managed marker into each installed folder, and
records source plus managed install directories under
`SKILLSPEC_HOME/durable-executor/config.json`, or
`~/.skillspec/durable-executor/config.json` when `SKILLSPEC_HOME` is not set. If
router mode is configured, install refreshes router-managed visibility and the
router index; `durable-executor` remains the implicit exception.

`durable-executor update` reads the durable config, creates a backup, rewrites
every recorded marker-protected managed install from the recorded source or
`--source`, refreshes router state when router mode is configured, and warns
that active harness sessions should be restarted. It refuses to overwrite an
existing unmarked folder.

`durable-executor delete` removes only recorded durable-executor folders that
contain the durable managed marker, removes durable config, and refreshes router
state when router mode is configured. It refuses to remove an unmarked folder so
hand-installed or unrelated skills are not deleted by lifecycle cleanup.

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
- `--force`: overwrite an existing installed skill folder without prompting.
- `--retire-existing`: when the target skill folder already exists, back it up
  under `SKILLSPEC_HOME/backups/retired-skills` or
  `~/.skillspec/backups/retired-skills`, remove it from active harness
  discovery, then install the reviewed replacement at the same name. This is
  the preferred path for replacing a prose skill with a SkillSpec-backed port.
  `--force` and `--retire-existing` are mutually exclusive.
- `--name <NAME>`: override the installed skill folder name.

`install skill` copies declared package-local files from imports, resources,
code sources, and file dependencies. It rejects support files named nested
`SKILL.md`, because those become additional harness-discoverable skills. Preserve
old prose as `source/SKILL_md.old` or another non-discoverable, non-Markdown
filename.

When a ported skill replaces an already-installed prose skill, run
`skillspec install skill <folder> --target <target> --dry-run --retire-existing`
before the real install so the user can see the backup path and target root.
Do not leave both the old prose skill and the new SkillSpec-backed skill active
under the same harness roots unless the user explicitly chooses side-by-side
testing with a different `--name`.
