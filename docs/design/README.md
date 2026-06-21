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

## Document Map

The intended design-doc set is:

- `spec-as-contract.md`: why SkillSpec is a behavior contract rather than prose
  instructions, a prompt language, or a workflow engine.
- `skill-authoring-lifecycle.md`: how a skill moves from prose to structured
  SkillSpec, including import, review, execution, and iteration.
- `package-anatomy.md`: how the thin `SKILL.md` loader, `skill.spec.yml`, and
  dependency manifests fit together.
- `progressive-sensemaking.md`: how an agent should orient through `sensemake`,
  `decide`, `query`, and `refs` instead of loading the whole spec file.
- `grammar-and-conformance.md`: the grammar surface, typed fields, references,
  validation rules, schema strictness, and conformance expectations.
- `rules-routes-and-decision-algebra.md`: how routes, rules, predicates, forbids,
  allows, elicitations, route order, and after-success closures combine.
- `state-machines-handoffs-and-jumps.md`: how lifecycle states, route execution
  plans, handoffs, and phase jumps are represented without turning SkillSpec into
  an execution engine.
- `imports-resources-code-and-recipes.md`: how runtime-loadable imports differ
  from resources, code blocks, artifacts, commands, and recipes.
- `traces-and-alignment.md`: how decision traces and alignment reports support
  review, replay, and self-reflection.
- `qa-process.md`: the detailed review checklist used to keep the docs aligned
  with implementation.

## Evidence Map

Every design claim should be grounded in one or more of these sources:

| Topic | Primary implementation and reference sources |
| --- | --- |
| Contract semantics and non-goals | `spec/README.md`, `spec/semantics.md`, `spec/grammar.md`, `docs/why-skillspec.md`, `docs/prose-vs-skillspec.md` |
| Top-level grammar shape | `crates/skillspec-cli/src/model.rs`, `spec/grammar.md`, `spec/skill.spec.schema.json` |
| Validation behavior | `crates/skillspec-cli/src/parser.rs`, `conformance/valid/`, `conformance/invalid/` |
| Route and rule decisions | `crates/skillspec-cli/src/decision.rs`, `spec/semantics.md`, `spec/relationships.md` |
| Progressive sensemaking | `crates/skillspec-cli/src/sensemake.rs`, `crates/skillspec-cli/src/compiler.rs` |
| Imports and local loading | `spec/imports.md`, `crates/skillspec-cli/src/imports.rs`, `crates/skillspec-cli/src/parser.rs` |
| Prose import scaffolding | `crates/skillspec-cli/src/importer.rs`, `docs/prose-vs-skillspec.md` |
| Thin loader generation | `crates/skillspec-cli/src/compiler.rs`, `examples/rote-shell/SKILL.md` |
| Dependency checks | `crates/skillspec-cli/src/deps.rs`, `examples/*/skill.spec.yml`, `examples/*/deps.toml` |
| Traces and alignment | `spec/trace.md`, `crates/skillspec-cli/src/trace.rs`, `crates/skillspec-cli/src/align.rs` |
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

`Trace` means structured decision evidence emitted by `skillspec decide` or a
compatible harness.

`Alignment report` means the result of replaying decision evidence and checking
which execution obligations have proof.

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
