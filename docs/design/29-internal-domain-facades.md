# Internal Domain Facades

## Purpose

SkillSpec is still distributed as a CLI first. The supported compatibility
contract remains the `skillspec` binary, command help, command JSON/text output,
schemas, examples, and tests.

The Rust crate is intentionally not a stable public API yet. Internally, the CLI
now calls small domain facades before it reaches the large implementation
modules. This is a refactor boundary for future crate extraction, not a new
external contract.

## Current Boundary

`crates/skillspec-cli/src/cli/args/` owns command parsing and help text.

`crates/skillspec-cli/src/cli/dispatch/` owns final command dispatch, stdout
format selection, and process exit decisions.

`crates/skillspec-core/` owns the extracted core contract modules:

- `src/error.rs`: shared CLI result/error type used by core and downstream
  implementation modules.
- `src/spec/model.rs`: the typed `SkillSpec` contract model.
- `src/spec/parser.rs` and `src/spec/parser/validation.rs`: YAML loading,
  validation, spec-cache behavior, and package sidecar validation.
- `src/spec/grammar.rs`: embedded grammar/schema sensemaking commands.
- `src/spec/imports.rs`: import validation and load ordering.
- `src/spec/import_dependency_ledger.rs`: generated dependency ledger helpers.

`crates/skillspec-cli/src/lib.rs` re-exports those modules under the old
internal names (`error`, `model`, `parser`, `grammar`, `imports`, and
`import_dependency_ledger`) so existing CLI implementation code and integration
tests keep compiling. Those re-exports are compatibility scaffolding for the
refactor, not a stable Rust API.

`crates/skillspec-runtime/` owns the extracted runtime execution modules:

- `src/decision.rs`: route/rule decision algebra and scenario test execution.
- `src/act.rs`: plan/action reports, phase selection, handoff boundaries, and
  effective tool-boundary rendering.
- `src/trace.rs`: decision trace envelopes, run directories, summaries,
  compaction, and fingerprints.
- `src/progress.rs`: structured execution events, progress reports, stats
  evidence, and final-response evidence.
- `src/align.rs` and `src/align/`: decision replay, execution proof alignment,
  proof digests, ledger parsing, and alignment report types.
- `src/deps.rs`: dependency checks.
- `src/command_path.rs`: local command lookup.
- `src/report.rs`: runtime report rendering used by CLI dispatch.
- `src/guide/`: guided run-loop start/resume/end anchors and persisted guide
  state.
- `src/run_loop.rs`: runtime run-loop report assembly. The CLI keeps
  `crates/skillspec-cli/src/features/run_loop.rs` as a thin wrapper for
  sensemake integration and token-metric rendering.

`crates/skillspec-cli/src/lib.rs` re-exports these runtime modules under the old
internal names (`act`, `align`, `command_path`, `decision`, `deps`, `guide`,
`progress`, `report`, and `trace`) so existing command dispatch and integration
tests keep compiling. Those re-exports are compatibility scaffolding for the
refactor, not a stable Rust API.

`crates/skillspec-doctor/` owns the extracted Doctor analysis modules:

- `src/lib.rs`: Doctor target inspection, local and remote skill shape
  classification, score assembly, issue generation, and public report model.
- `src/frontmatter.rs`, `src/risk.rs`, `src/metrics.rs`, `src/types.rs`,
  `src/renderer.rs`, and `src/workspace_report.rs`: Doctor-specific analysis
  and rendering helpers.
- `src/remote_source.rs`: public GitHub target parsing, sparse checkout, and
  stage reporting used by Doctor and current authoring commands.
- `src/source_map.rs` and `src/source_map/`: source-map schema, local source
  discovery, Markdown mapping, query, coverage, and stale checks used by Doctor
  and current authoring commands.

The CLI re-exports `doctor`, `remote_source`, and `source_map` under the old
internal names so current command dispatch, import, port-one-shot, and workspace
flows keep compiling. `remote_source` and `source_map` live in this crate for now
because Doctor requires them directly; the later authoring extraction may move
or rename that shared support without changing CLI behavior.

`crates/skillspec-cli/src/domain/` owns command-family orchestration:

- `authoring.rs`: compile, import, port-one-shot, source map, grammar,
  dependency/import checks, capability seeds, synthesis, top-level index, and
  route.
- `runtime.rs`: validate, test, decide, plan, act, run-loop, explain,
  sensemake, query, refs, trace-required checks, and runtime render wrappers.
- `doctor.rs`: doctor inspection and render modes.
- `evidence.rs`: trace compaction, alignment, progress display, and progress
  event recording.
- `harness.rs`: install targets, skill install, status, visibility, skills
  audit, router lifecycle, router guard/index status, and durable-executor
  lifecycle.
- `workspace.rs`: workspace map, validate, import, converge, compile, install,
  and workspace report rendering.

The lower-level remaining `features/` and `lifecycle/` modules still implement
the remaining CLI behavior. They remain hidden implementation modules.

## Refactor Rules

Do not change CLI command names, options, exit status behavior, JSON shapes, text
report wording, generated files, or install side effects as part of facade work.

Do keep stdout/stderr and `std::process::exit` decisions in `cli/dispatch/`.
Domain facades may write domain artifacts when the existing command already did
so, such as generated specs, source maps, reports, traces, or install files.

Do add focused tests around the command family being moved before each stack
commit. Full workspace tests and clippy should run before merging the complete
refactor stack.

Do not expose these facades as a stable Rust API until crate extraction gives
them explicit compatibility rules.

## Extraction Path

The facades are the migration seam for later internal crates:

- `skillspec-core`: implemented for error, model, parser, imports, grammar, and
  import dependency ledger. It is a publishable companion crate so crates.io
  releases remain possible: publish this crate first, then publish the CLI crate
  that depends on the same version. The crate exists as an implementation
  boundary, not as a stable Rust API promise.
- `skillspec-runtime`: implemented for runtime decisions, act/plan, run-loop
  report assembly, traces, progress, guidance, dependency checks, report
  rendering, and alignment. It is a publishable companion crate so crates.io
  releases remain possible: publish `skillspec-core`, then `skillspec-runtime`,
  then the CLI crate that depends on the same versions. The crate exists as an
  implementation boundary, not as a stable Rust API promise.
- `skillspec-doctor`: implemented for Doctor inspection, reports, renderers,
  remote source staging support, and source-map support. It is a publishable
  companion crate so crates.io releases remain possible: publish
  `skillspec-core`, then `skillspec-runtime`, then `skillspec-doctor`, then the
  CLI crate that depends on the same versions. The crate exists as an
  implementation boundary, not as a stable Rust API promise.
- `skillspec-import`: source staging, import, port-one-shot, workspace
  authoring, compile, and synthesis.
- `skillspec-harness`: install targets, visibility, router lifecycle, durable
  lifecycle, and status.

Further crates should be introduced only after the previous stack has stayed
green and the CLI continues to prove that user contracts are unchanged.
