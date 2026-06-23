# Command Log

This command log is the design-facing quick scan for the implemented
`skillspec` CLI surface.

It answers four questions for each command:

- What is the command?
- What are the important arguments and options?
- Why does it exist?
- What does a realistic invocation look like?

This document is not the formal command reference. The source of truth remains
the clap command tree in `crates/skillspec-cli/src/main.rs` and the reference
inventory in `spec/commandspec.md`. If this table disagrees with the CLI help or
reference spec, this table is wrong.

## Runtime Loop Commands

These are the commands an agent or harness uses during a SkillSpec-backed run.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec sensemake` | `<path>`, `--view index,summary,full`, `--json` | Teaches the shape of one spec without loading the full YAML. Use it when the spec is unfamiliar and the agent needs section ids and navigation handles. | `skillspec sensemake ./skill.spec.yml --view index` |
| `skillspec decide` | `<path>`, `--input <text>`, `--trace-dir <dir>` | Evaluates routing rules for a user task and emits the selected route, matched rules, forbids, elicitations, and after-success work as JSON. | `skillspec decide ./skill.spec.yml --input 'install this skill' --trace-dir .skillspec/traces` |
| `skillspec plan` | `<path>`, `--input <text>`, `--trace-dir <dir>`, `--json` | Lists the selected route's execution phases in order and writes a decision trace when `--trace-dir` is supplied. This is the pre-action phase-order view. | `skillspec plan ./skill.spec.yml --input 'port this skill' --trace-dir .skillspec/traces` |
| `skillspec act` | `<path>`, `--input <text>`, `--trace-dir <dir>`, `--run <run-dir>`, `--phase <id>`, `--json` | Expands the selected route and current phase into an OODA action checklist, including matched rules, allowed actions, forbids, transitions, handoffs, and the effective phase tool boundary. | `skillspec act ./skill.spec.yml --input 'port this skill' --run .skillspec/traces/run-123 --phase qa_and_proof` |
| `skillspec progress record` | `<run-dir>`, `<event>`, `[phase]`, `[requirement]`, `--id <id>`, `--status <status>`, `--evidence-kind <kind>`, `--evidence-ref <ref>`, `--source-skill <id>`, `--message <text>`, `--json` | Appends one structured event to `<run-dir>/execution.jsonl`. This is how the harness records phase, requirement, route, handoff, closure, and evidence proof. | `skillspec progress record .skillspec/traces/run-123 requirement-satisfied qa_and_proof validate_spec --evidence-kind command --evidence-ref validate.log` |
| `skillspec progress show` | `<path>`, `--run <run-dir>`, `--json` | Reads the decision trace plus `execution.jsonl`, derives `progress.json`, and reports completed, current, blocked, and remaining phases plus open requirements. | `skillspec progress show ./skill.spec.yml --run .skillspec/traces/run-123` |
| `skillspec trace align` | `<path>`, `--decision-trace <run-dir>`, `--execution-trace <jsonl>`, `--json` | Replays the decision trace against the current spec and checks structured execution evidence for obligations. Writes `<run-dir>/alignment.json`. | `skillspec trace align ./skill.spec.yml --decision-trace .skillspec/traces/run-123 --execution-trace .skillspec/traces/run-123/execution.jsonl` |

## Authoring And QA Commands

These commands help authors create, inspect, validate, test, compile, and install
SkillSpec-backed skills.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec validate` | `<path>` | Parses and validates a `skill.spec.yml` file against the typed grammar, parser checks, identifiers, and cross-references. | `skillspec validate examples/durable-executor/skill.spec.yml` |
| `skillspec test` | `<path>` | Runs scenario tests declared in the spec against the decision engine. | `skillspec test examples/durable-executor/skill.spec.yml` |
| `skillspec explain` | `<path>`, `--input <text>`, `--trace-dir <dir>` | Explains the routing decision for a task in human-facing form and optionally records decision trace events. | `skillspec explain ./skill.spec.yml --input 'browse gmail' --trace-dir .skillspec/traces` |
| `skillspec query` | `<path>`, `<handle>`, `--view index,summary,full`, `--json` | Retrieves one collection, item, or field path from the spec. Use it for progressive detail instead of reading the whole YAML. | `skillspec query ./skill.spec.yml command:validate_spec.requires --view summary` |
| `skillspec refs` | `<path>`, `<handle>`, `--view index,summary,full`, `--json` | Shows outgoing references for an item handle, such as a route's checks, a command's dependencies, or a rule's preferred route. | `skillspec refs ./skill.spec.yml route:local_skill_port --view summary` |
| `skillspec grammar sensemake` | `--view index,summary,porting,full`, `--json` | Teaches the embedded grammar artifact progressively. Use before importing or revising a spec so the harness does not infer grammar from memory. | `skillspec grammar sensemake --view porting` |
| `skillspec grammar checklist` | `--for <subject>`, `--json` | Shows the embedded coverage checklist for a semantic porting or review workflow. | `skillspec grammar checklist --for import-skill` |
| `skillspec grammar schema` | `--json` | Prints or summarizes the embedded JSON Schema used by grammar-aware harnesses and reviewers. | `skillspec grammar schema --json` |
| `skillspec imports check` | `<path>` | Validates declared local imports, sections, and dependency-first load order. | `skillspec imports check ./skill.spec.yml` |
| `skillspec deps check` | `<path>`, `--command <id>` | Checks declared dependencies for the whole spec or for one command. Local checks can pass or fail; harness-specific checks are reported as deferred. | `skillspec deps check ./skill.spec.yml --command validate_spec` |
| `skillspec compile` | `<path>`, `--target codex-skill,claude-skill,markdown` | Compiles a spec into harness guidance or a full Markdown rendering. Generated skill loaders point agents back to the colocated spec and runtime commands. | `skillspec compile ./skill.spec.yml --target codex-skill` |
| `skillspec import-skill` | `<path>`, `--out <path>` | Creates a mechanical draft `skill.spec.yml` from a local `SKILL.md` file or skill folder. It is scaffolding for review, not a finished semantic port. | `skillspec import-skill ./source-skill --out ./draft/skill.spec.yml` |
| `skillspec trace compact` | `<run-dir>` | Rebuilds `trace.jsonl` and `summary.json` from append-only trace event files in a run directory. | `skillspec trace compact .skillspec/traces/run-123` |
| `skillspec install targets` | none | Lists detected harness skill roots, such as Codex, Agents, or Claude local skill directories. | `skillspec install targets` |
| `skillspec install skill` | `<folder>`, `--target agents,codex,claude-local`, `--all-detected`, `--dry-run`, `--name <name>`, `--force` | Installs a generated skill folder containing `SKILL.md` and `skill.spec.yml` into one or more detected harness roots. Use `--dry-run` before writing. | `skillspec install skill examples/pdf --target agents --target codex --dry-run` |

## Skill Router Commands

These commands support large skill catalogs without putting every skill
description into the model context.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec index` | `--roots <path>...`, `--out <sqlite>`, `--visibility-manifest <path>`, `--json` | Builds a local SQLite skill catalog from `SKILL.md`, native visibility metadata, and optional `skill.spec.yml` routing hints. | `skillspec index --roots ~/.agents/skills --out ~/.skillspec/router/skill-index.sqlite --json` |
| `skillspec route` | `--index <sqlite>`, `--query <text>`, `--top <n>`, `--execution-mode direct,durable`, `--json` | Scores candidate skills from the index and returns the selected skill path, candidates, confidence, visibility, and optional direct/durable elicitation hint. | `skillspec route --index ~/.skillspec/router/skill-index.sqlite --query 'extract text from a pdf' --json` |
| `skillspec skills audit` | `--roots <path>...`, `--json` | Audits routing metadata for overlong descriptions, vague descriptions, missing negative boundaries, and duplicate names. | `skillspec skills audit --roots ~/.agents/skills --json` |
| `skillspec skills set-visibility` | `<skill> <implicit,manual-only,name-only,off>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Sets one skill's conceptual visibility using native Codex/Claude controls and records a reversible manifest. | `skillspec skills set-visibility pdf manual-only --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec skills disable` | `<skill>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Convenience command for `set-visibility <skill> off`; off skills are excluded from router results when the manifest is used. | `skillspec skills disable legacy-skill --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec skills enable` | `<skill>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Convenience command for `set-visibility <skill> implicit`. | `skillspec skills enable pdf --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec visibility plan` | `--roots <path>...`, `--profile router-managed`, `--json` | Shows the native visibility changes router install would apply without editing files. | `skillspec visibility plan --roots ~/.agents/skills ~/.claude/skills --json` |
| `skillspec visibility apply` | `--roots <path>...`, `--profile router-managed`, `--manifest <path>`, `--dry-run`, `--json` | Applies native Codex `agents/openai.yaml` or Claude `skillOverrides` visibility controls and writes a rollback manifest. | `skillspec visibility apply --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json --json` |
| `skillspec visibility restore` | `--manifest <path>`, `--dry-run`, `--json` | Restores exact file snapshots from a visibility manifest. It does not infer previous state. | `skillspec visibility restore --manifest ~/.skillspec/router/visibility-manifest.json --json` |
| `skillspec router install` | `--roots <path>...`, `--router-root <path>`, `--index <sqlite>`, `--manifest <path>`, `--router-name <name>`, `--dry-run`, `--json` | Installs the visible `skill-router` skill, applies router-managed visibility, builds the index, and writes router config for future install hooks. | `skillspec router install --roots ~/.agents/skills --router-root ~/.agents/skills --index ~/.skillspec/router/skill-index.sqlite --json` |
| `skillspec router uninstall` | `--manifest <path>`, `--router-root <path>`, `--index <sqlite>`, `--keep-index`, `--dry-run`, `--json` | Restores visibility from the manifest, removes only the managed router skill marker directory, removes config, and optionally removes the index. | `skillspec router uninstall --json` |
| `skillspec router index refresh` | `--roots <path>...`, `--index <sqlite>`, `--visibility-manifest <path>`, `--json` | Refreshes the router index after skill additions, removals, or metadata changes. | `skillspec router index refresh --roots ~/.agents/skills --index ~/.skillspec/router/skill-index.sqlite --visibility-manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec router index status` | `--roots <path>...`, `--index <sqlite>`, `--visibility-manifest <path>`, `--json` | Compares the index against current roots and reports new, changed, missing, stale, and updated-at state. | `skillspec router index status --roots ~/.agents/skills --index ~/.skillspec/router/skill-index.sqlite --json` |

## Capability Bootstrap Commands

These commands manage local capability seeds used by durable bootstrap flows when
no domain SkillSpec exists yet.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec capability store` | none | Shows the local capability seed store path. Defaults under `~/.skillspec/capabilities/`, or under `SKILLSPEC_HOME` when set. | `skillspec capability store` |
| `skillspec capability add` | `<id>`, `--domain <domain>`, `--kind <kind>`, `--provides <capability>`, plus seed metadata flags | Creates or rewrites a local capability seed for a CLI, adapter, script, or other reusable substrate. | `skillspec capability add preferred-voice-cli --domain voice --kind cli --provides text_to_speech --command say` |
| `skillspec capability update` | `<id>`, `--domain <domain>`, patch flags such as `--add-provides`, `--remove-provides`, `--priority`, `--mark-failed` | Patches an existing seed without rewriting unspecified fields. Use this to adjust ranking, aliases, auth hints, risk flags, or verification state. | `skillspec capability update preferred-voice-cli --domain voice --priority 70 --add-provides speech_synthesis` |
| `skillspec capability list` | `--domain <domain>` | Lists local capability seeds, optionally scoped to one domain. | `skillspec capability list --domain voice` |
| `skillspec capability search` | `<capability>`, `--domain <domain>`, `--explain`, `--json`, `--local-only`, `--preferred-seed <id>` | Searches and ranks local seeds for a capability. It returns scores, reasons, risk gates, and ask policy when candidates are close. | `skillspec capability search text_to_speech --domain voice --explain --json` |
| `skillspec capability inspect` | `<id>`, `--domain <domain>`, `--json` | Reads one capability seed and reports its stored metadata. | `skillspec capability inspect preferred-voice-cli --domain voice --json` |
| `skillspec capability verify` | `<id>`, `--domain <domain>`, `--json` | Runs declared evidence checks for a seed and updates verification status. | `skillspec capability verify preferred-voice-cli --domain voice --json` |
| `skillspec capability prefer` | `<id>`, `--for <capability>`, `--domain <domain>`, `--priority <0-100>` | Updates preferred capability and priority metadata without editing durable-executor's spec. | `skillspec capability prefer preferred-voice-cli --domain voice --for text_to_speech --priority 90` |
| `skillspec capability remove` | `<id>`, `--domain <domain>` | Removes one local capability seed. | `skillspec capability remove preferred-voice-cli --domain voice` |
| `skillspec capability scan` | none | Scans for seed proposals. This is a discovery helper for local bootstrap work. | `skillspec capability scan` |

## Important Event Values

`skillspec progress record` accepts these event values:

| Event | Meaning | Example |
| --- | --- | --- |
| `phase-started` | A declared phase has begun. | `skillspec progress record .skillspec/traces/run-123 phase-started qa_and_proof` |
| `requirement-started` | Work began for a phase requirement. | `skillspec progress record .skillspec/traces/run-123 requirement-started qa_and_proof validate_spec` |
| `requirement-satisfied` | A phase requirement has structured proof. | `skillspec progress record .skillspec/traces/run-123 requirement-satisfied qa_and_proof validate_spec --evidence-kind command --evidence-ref validate.log` |
| `requirement-failed` | A phase requirement failed. | `skillspec progress record .skillspec/traces/run-123 requirement-failed qa_and_proof test_spec --status fail` |
| `obligation-satisfied` | A route, forbid, elicitation, or other obligation has explicit proof. | `skillspec progress record .skillspec/traces/run-123 obligation-satisfied --id report_alignment_status --status pass` |
| `route-fulfilled` | The selected route was fulfilled. | `skillspec progress record .skillspec/traces/run-123 route-fulfilled --id prove_skill_value --status pass` |
| `after-success-completed` | A scheduled closure completed. | `skillspec progress record .skillspec/traces/run-123 after-success-completed --id trace_align --status pass` |
| `evidence-attached` | Extra evidence was attached to the run. | `skillspec progress record .skillspec/traces/run-123 evidence-attached --evidence-kind file --evidence-ref docs/design/command-log.md` |
| `handoff-started` | A declared handoff began. | `skillspec progress record .skillspec/traces/run-123 handoff-started browser_lookup` |
| `handoff-completed` | A declared handoff completed. | `skillspec progress record .skillspec/traces/run-123 handoff-completed browser_lookup --status pass` |
| `phase-completed` | A declared phase completed. | `skillspec progress record .skillspec/traces/run-123 phase-completed qa_and_proof --status pass` |
| `phase-blocked` | A declared phase cannot continue. | `skillspec progress record .skillspec/traces/run-123 phase-blocked install_skill --status blocked --message 'needs approval'` |

## Design Notes

The command surface has three layers:

- Runtime navigation and proof: `sensemake`, `decide`, `plan`, `act`,
  `progress`, and `trace align`.
- Authoring and QA: `validate`, `test`, `query`, `refs`, `grammar`, `imports`,
  `deps`, `compile`, `import-skill`, and `install`.
- Large skill discovery: `index`, `route`, `skills`, `visibility`, and
  `router`.
- Local durable bootstrap: `capability`.

The CLI does not execute arbitrary task work. It renders contracts, validates
them, records progress evidence, and aligns traces. The surrounding harness
still owns actual tool execution, approvals, redaction, and substrate policy.

The earlier proposed namespace `skillspec skill port/install/prove/value` is not
documented here because it is not an implemented CLI namespace in the current
binary.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-cli/src/main.rs`, which defines the clap command tree;
- `spec/commandspec.md`, which is the reference command inventory;
- `crates/skillspec-cli/src/act.rs`, `progress.rs`, `align.rs`, `grammar.rs`,
  `deps.rs`, `imports.rs`, `compiler.rs`, `importer.rs`, `install.rs`,
  `router.rs`, `visibility.rs`, `router_lifecycle.rs`, and `capability.rs`,
  which implement the listed command behavior;
- `skillspec --help` and subcommand help output from the current local binary.
