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

The lower-level `spec/`, `execution/`, `features/`, and `lifecycle/` modules
still implement the actual behavior. They remain hidden implementation modules.

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

- `skillspec-core`: model, parser, imports, grammar, source maps, decision logic.
- `skillspec-runtime`: runtime decisions, act/plan/run-loop, traces, progress,
  and alignment.
- `skillspec-doctor`: doctor reports and renderers.
- `skillspec-import`: source staging, import, port-one-shot, workspace
  authoring, compile, and synthesis.
- `skillspec-harness`: install targets, visibility, router lifecycle, durable
  lifecycle, and status.

Those crates should be introduced only after the facade layer has stayed green
and the CLI continues to prove that user contracts are unchanged.
