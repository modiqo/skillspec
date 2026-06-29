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

The lower-level `execution/`, `features/`, and `lifecycle/` modules still
implement the remaining CLI behavior. They remain hidden implementation modules.

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
  import dependency ledger. It is currently `publish = false`; a crates.io
  release that keeps the CLI depending on it must either publish this crate
  first or deliberately collapse the dependency before release.
- `skillspec-runtime`: runtime decisions, act/plan/run-loop, traces, progress,
  and alignment.
- `skillspec-doctor`: doctor reports and renderers.
- `skillspec-import`: source staging, import, port-one-shot, workspace
  authoring, compile, and synthesis.
- `skillspec-harness`: install targets, visibility, router lifecycle, durable
  lifecycle, and status.

Further crates should be introduced only after the previous stack has stayed
green and the CLI continues to prove that user contracts are unchanged.
