# SkillSpec Design Documentation

This directory explains the design of SkillSpec for maintainers, skill authors,
harness authors, and reviewers. It is intentionally separate from the formal
reference in `spec/`: the reference defines the contract, while these documents
explain why the contract is shaped this way and how the pieces work together.

The design docs are not a second source of truth. If a design doc disagrees with
the Rust model, parser, schema, reference spec, or conformance fixtures, the
design doc is wrong and must be corrected.

## Source Of Truth Order

Use this order when resolving a disagreement:

1. The Rust data model and parser in `crates/skillspec-cli/src/`.
2. The JSON Schema in `spec/skill.spec.schema.json`.
3. The reference documents in `spec/`.
4. The conformance fixtures in `conformance/`.
5. The generated and hand-maintained examples in `examples/`.
6. The explanatory docs in `docs/`.

This order matters because SkillSpec is a contract. A design document may explain
intent, but only the model, parser, schema, tests, and reference docs can define
what the current implementation accepts.

## Reading Order

Read the numbered docs in filename order. `README.md` is the index and stays
unnumbered; every other file is prefixed to preserve the intended sequence in
directory listings.

| Order | Doc | Purpose |
| --- | --- | --- |
| 01 | [SkillSpec As Contract](01-spec-as-contract.md) | Why SkillSpec is a behavior contract rather than prose instructions, a prompt language, or a workflow engine. |
| 02 | [Grammar And Conformance](02-grammar-and-conformance.md) | The grammar surface, typed fields, references, validation rules, schema strictness, and conformance expectations. |
| 03 | [Package Anatomy](03-package-anatomy.md) | How the thin `SKILL.md` loader, `skill.spec.yml`, and dependency manifests fit together. |
| 04 | [Skill Authoring Lifecycle](04-skill-authoring-lifecycle.md) | How a skill moves from prose to structured SkillSpec, including import, review, execution, and iteration. |
| 05 | [Progressive Sensemaking](05-progressive-sensemaking.md) | How an agent should orient through `sensemake`, `decide`, `query`, and `refs` instead of loading the whole spec file. |
| 06 | [Rules, Routes, And Decision Algebra](06-rules-routes-and-decision-algebra.md) | How routes, rules, predicates, forbids, allows, elicitations, route order, and after-success closures combine. |
| 07 | [State Machines, Handoffs, And Jumps](07-state-machines-handoffs-and-jumps.md) | How lifecycle states, route execution plans, handoffs, and phase jumps are represented without turning SkillSpec into an execution engine. |
| 08 | [Imports, Resources, Code, And Recipes](08-imports-resources-code-and-recipes.md) | How runtime-loadable imports differ from resources, code blocks, artifacts, commands, and recipes. |
| 09 | [Phase Tool Boundaries](09-phase-tool-boundaries.md) | How `tool_boundary` is rendered by `act` as a hard per-phase permission boundary for tools, data sources, substrates, providers, adapters, APIs, CLIs, browser modes, and skills. |
| 10 | [Runtime Plan Act Progress Loop](10-runtime-plan-act-progress-loop.md) | How `plan`, `act`, `progress record`, `progress show`, and `trace align` form the visible runtime loop for a SkillSpec-backed run. |
| 11 | [Execution Progress Ledger](11-execution-progress-ledger.md) | How `execution.jsonl` records phase, requirement, handoff, route, and closure proof for progress and alignment. |
| 12 | [Traces And Alignment](12-traces-and-alignment.md) | How decision traces and alignment reports support review, replay, and self-reflection. |
| 13 | [Completion Alignment And Token Reporting](13-completion-alignment-and-token-reporting.md) | How final responses should render alignment summaries, missing proof rows, trace paths, and measured token consumption and savings. |
| 14 | [Skill Router](14-skill-router.md) | How the optional router indexes large skill libraries, applies native Codex and Claude visibility controls, and preserves a manifest-backed restore path. |
| 15 | [Capability Bootstrap](15-capability-bootstrap.md) | How durable-executor uses local capability seeds under `~/.skillspec/capabilities/` when no domain SkillSpec exists yet. |
| 16 | [Command Log](16-command-log.md) | A scannable command table with implemented command names, important args/options, explanations, and realistic examples. |
| 17 | [Design Documentation QA Process](17-qa-process.md) | The detailed review checklist used to keep the docs aligned with implementation. |

## Evidence Map

Every design claim should be grounded in one or more of these sources:

| Topic | Primary implementation and reference sources |
| --- | --- |
| Contract semantics and non-goals | `spec/README.md`, `spec/semantics.md`, `spec/grammar.md`, `docs/01-why-skillspec.md`, `docs/02-prose-vs-skillspec.md` |
| Top-level grammar shape | `crates/skillspec-cli/src/model.rs`, `spec/grammar.md`, `spec/skill.spec.schema.json` |
| Validation behavior | `crates/skillspec-cli/src/parser.rs`, `conformance/valid/`, `conformance/invalid/` |
| Route and rule decisions | `crates/skillspec-cli/src/decision.rs`, `spec/semantics.md`, `spec/relationships.md` |
| Progressive sensemaking | `crates/skillspec-cli/src/sensemake.rs`, `crates/skillspec-cli/src/compiler.rs` |
| Runtime phase loop | `crates/skillspec-cli/src/act.rs`, `crates/skillspec-cli/src/progress.rs`, `crates/skillspec-cli/src/main.rs`, `spec/commandspec.md` |
| Phase tool boundaries | `crates/skillspec-cli/src/model.rs`, `crates/skillspec-cli/src/act.rs`, `spec/grammar.md`, `spec/skill.spec.schema.json` |
| Command log | `crates/skillspec-cli/src/main.rs`, `spec/commandspec.md`, command help output |
| Imports and local loading | `spec/imports.md`, `crates/skillspec-cli/src/imports.rs`, `crates/skillspec-cli/src/parser.rs` |
| Prose import scaffolding | `crates/skillspec-cli/src/importer.rs`, `docs/02-prose-vs-skillspec.md` |
| Thin loader generation | `crates/skillspec-cli/src/compiler.rs`, `examples/durable-executor/SKILL.md` |
| Dependency checks | `crates/skillspec-cli/src/deps.rs`, `examples/*/skill.spec.yml`, `examples/*/deps.toml` |
| Capability bootstrap | `crates/skillspec-cli/src/capability.rs`, `examples/durable-executor/skill.spec.yml`, `crates/skillspec-cli/tests/cli.rs` |
| Skill router | `crates/skillspec-cli/src/router.rs`, `crates/skillspec-cli/src/visibility.rs`, `crates/skillspec-cli/src/router_lifecycle.rs`, `examples/skill-router/skill.spec.yml`, `crates/skillspec-cli/tests/cli.rs` |
| Traces, progress, and alignment | `spec/trace.md`, `crates/skillspec-cli/src/trace.rs`, `crates/skillspec-cli/src/progress.rs`, `crates/skillspec-cli/src/align.rs` |
| CLI surface | `crates/skillspec-cli/src/main.rs` |

## Terms Used In These Docs

`SkillSpec` means the structured `skill.spec.yml` contract.

`Prose skill` means a conventional `SKILL.md` file that relies on natural
language instructions.

`Thin loader` or `trampoline SKILL.md` means a small `SKILL.md` generated for a
SkillSpec-backed skill. It tells the agent to load and obey the colocated
`skill.spec.yml`; it is not the behavioral source of truth.

`Harness` means the surrounding agent runtime or product integration that reads
the spec, asks the model to act, chooses tools, enforces approval policy, and
records execution evidence. SkillSpec can describe and test steering decisions,
but the current v0 CLI does not execute arbitrary work on its own.

`Import` means runtime-loadable instruction material declared in `imports`.
Imports are local, explicit, and structured. They are not inheritance.

`Resource` means supporting provenance or reference material. Resources do not
become active instructions unless the spec connects them to code, recipes, or
other structured behavior.

`Route` means a named strategy for satisfying a task.

`Rule` means a predicate-driven steering clause that can prefer a route, replace
route order, forbid substitutions, allow narrow fallback, request elicitation, or
schedule after-success work.

`State` means a lifecycle position. In v0 it is described and validated, but not
executed as a general-purpose workflow runtime.

`Jump point` means a declared transition in a route execution plan from one phase
to another when a condition is met. It is a planning and review primitive, not an
implicit command executor.

`Phase plan` means the ordered execution phases rendered by `skillspec plan` for
the selected route and task input.

`Action checklist` means the current-route and current-phase operating procedure
rendered by `skillspec act`, including route authority, matched rules, active
forbids, transitions, handoffs, before-tool-call checks, and the effective phase
tool boundary.

`Phase tool boundary` means the effective `tool_boundary` rendered by
`skillspec act` after merging runtime defaults, entry policy, selected route
policy, current phase policy, and active forbids. It is a harness-facing
permission contract, not a standalone security sandbox.

`Progress ledger` means the structured `<run-dir>/execution.jsonl` file appended
by `skillspec progress record`. It stores compact proof events for phases,
requirements, handoffs, routes, closures, and evidence references.

`Progress report` means the derived `<run-dir>/progress.json` file and human
summary produced by `skillspec progress show`.

`Trace` means structured decision evidence emitted by `skillspec decide` or a
compatible harness.

`Alignment report` means the result of replaying decision evidence and checking
which execution obligations have proof.

`Completion summary` means the compact final status block from
`skillspec trace align`, including decision replay, phase order, requirement
proof counts, missing proof rows, forbidden-action status, alignment status, and
token usage.

## Documentation QA Gate

Each design doc must pass this gate before it is committed:

1. Identify the source files that control the topic.
2. Check field names, enum names, CLI commands, and validation behavior against
   the current code or reference docs.
3. Separate current implementation from intended design. If a behavior is only a
   harness responsibility, say that directly.
4. Avoid future-tense capability claims unless a source file already implements
   the behavior or the doc labels the claim as a design constraint.
5. Avoid security claims stronger than the reference docs. SkillSpec is not a
   security boundary by itself.
6. Avoid saying the importer understands prose semantically. The current importer
   scaffolds a spec and preserves material for review; it does not prove that the
   original skill has been converted into correct rules.
7. Run focused searches against the cited code before committing.
8. Review the diff for unsupported claims, vague language, and stale examples.
9. Commit only the doc or docs that passed review.

The goal is not to produce long docs quickly. The goal is to produce documents a
new maintainer can use without inheriting fabricated behavior.
