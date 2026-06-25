# Design Documentation QA Process

This process exists to keep SkillSpec design docs aligned with the repository.
It is intentionally stricter than ordinary documentation review because the
project is a contract format. A false claim in a design doc can teach agents,
harness authors, and maintainers to rely on behavior that does not exist.

## Review Standard

Every design doc must be reviewed as a contract explanation, not as a marketing
page.

A design doc passes QA only when:

- every behavioral claim has a source;
- current implementation is separated from intended design;
- parser, model, schema, reference docs, examples, and tests agree with the
  claim or the doc states the disagreement;
- non-goals are preserved;
- unsupported automation claims are removed;
- the doc can be committed independently.

Long docs are acceptable. Unsupported docs are not.

## Evidence Levels

Use these evidence levels when reviewing a claim.

`P0`: implementation source.

Examples include the `spec/`, `execution/`, `features/`, and `cli/` modules
under `crates/skillspec-cli/src/`.

`P1`: formal reference source.

Examples: `spec/grammar.md`, `spec/semantics.md`, `spec/imports.md`,
`spec/relationships.md`, `spec/trace.md`, `spec/skill.spec.schema.json`.

`P2`: conformance and fixtures.

Examples: `conformance/valid/`, `conformance/invalid/`, generated golden
fixtures, `examples/*/skill.spec.yml`, generated `SKILL.md` loaders.

`P3`: explanatory docs.

Examples: `docs/01-why-skillspec.md`, `docs/02-prose-vs-skillspec.md`, RFC and
community docs.

Claims about accepted fields, validation, decision behavior, CLI behavior,
trace behavior, or alignment status need P0 evidence. P1-P3 evidence is useful,
but it is not enough when implementation behavior is the subject.

## Claim Categories

Classify claims before approving them.

Implementation claim:

- Says what the current code accepts, rejects, emits, or checks.
- Must cite or be checked against P0 evidence.

Reference claim:

- Says what the spec intends.
- Must cite or be checked against P1 evidence.

Example claim:

- Says what a current example demonstrates.
- Must cite or be checked against P2 evidence.

Design guidance:

- Says what authors should do.
- Must not contradict P0 or P1 evidence.

Inference:

- Connects source facts into an explanation.
- Must be phrased as an inference or guidance, not as implemented behavior.

Future or desired behavior:

- Should be avoided in these docs unless the text explicitly labels it as
  future work or a design constraint.

## Per-Doc Workflow

Use this workflow for each design doc.

1. Define the doc scope in one sentence.
2. List the controlling source files.
3. Read the relevant source chunks.
4. Draft the doc.
5. Red-team the doc for hallucination risks.
6. Run focused searches for field names, enum names, command names, and behavior
   claims.
7. Scan for banned or risky language.
8. Read the full generated doc once.
9. Stage only that doc.
10. Commit that doc with a focused message.

Do not batch unreviewed docs into one commit. Atomic commits make it possible to
revert or rewrite one explanation without disturbing the rest of the set.

## Red-Team Checklist

Ask these questions before commit:

- Does the doc claim the CLI executes work that it only declares?
- Does the doc claim the importer semantically understands prose?
- Does the doc claim an import is loaded implicitly because a file exists?
- Does the doc claim `deps.toml` is parsed by `skillspec deps check`?
- Does the doc claim traces contain execution payloads?
- Does the doc claim alignment `ok: true` means full success?
- Does the doc claim state transitions or execution-plan jumps are runtime
  executed by the CLI?
- Does the doc claim every graph invariant is parser-validated?
- Does the doc claim SkillSpec is a security boundary?
- Does the doc invent field names, enum values, commands, or status values?
- Does the doc describe future behavior as current behavior?
- Does the doc turn examples into universal guarantees?

If the answer to any question is yes, patch the doc or cite the exact source that
proves the claim.

## Risky Language Scan

Search for language that often hides hallucination:

```sh
rg -n "TODO|TBD|FIXME|probably|maybe|I think|guarantee|seamless|seamlessly|automatic|magically" docs/design/<doc>.md
```

Not every hit is wrong. Some docs intentionally say something is not automatic.
Every hit must be inspected.

For docs about execution, also search for:

```sh
rg -n "execute|executes|permission|safe|security boundary|proof|proves|full success" docs/design/<doc>.md
```

For docs about imports and dependency manifests, also search for:

```sh
rg -n "implicit|inheritance|deps.toml|imports check|deps check|load" docs/design/<doc>.md
```

## Source Search Examples

Use focused source searches before committing.

For grammar fields:

```sh
rg -n "pub struct SkillSpec|deny_unknown_fields|pub struct Expectation" crates/skillspec-cli/src/spec/model.rs
```

For parser validation:

```sh
rg -n "validate_|UnknownReference|imports.orphan|resources.orphan|requires.imports" crates/skillspec-cli/src/spec/parser/validation.rs
```

For decision behavior:

```sh
rg -n "default_route_order|matches_predicate|apply_rule|RouteSelectionBasis|dedupe_strings" crates/skillspec-cli/src/execution/decision.rs
```

For sensemaking:

```sh
rg -n "SensemakeReport|navigation|outgoing_refs|select_value|query_hints" crates/skillspec-cli/src/features/sensemake.rs
```

For imports:

```sh
rg -n "import path must be local and relative|topological_load_order|markdown_has_section" crates/skillspec-cli/src/spec/imports.rs
```

For traces and alignment:

```sh
rg -n "TraceEnvelope|write_decision_trace|AlignReport|AlignStatus|obligations_for|report_status" crates/skillspec-cli/src/execution/trace.rs crates/skillspec-cli/src/execution/align.rs
```

## Final Repo QA

After the docs set is complete, run a broader check:

```sh
rg -n "TODO|TBD|FIXME|probably|maybe|I think|guarantee|seamless|seamlessly|automatic|magically" docs/design
```

Run docs/source smoke checks:

```sh
git status --short
git log --oneline --decorate -n 12
```

Run implementation checks appropriate for the change:

```sh
cargo test
```

Run representative SkillSpec checks:

```sh
target/debug/skillspec validate examples/durable-executor/skill.spec.yml
target/debug/skillspec imports check examples/durable-executor/skill.spec.yml
target/debug/skillspec test examples/durable-executor/skill.spec.yml
target/debug/skillspec deps check examples/durable-executor/skill.spec.yml
```

If `target/debug/skillspec` does not exist before the representative checks, run
`cargo build`. If a command fails because the environment lacks a dependency,
report the exact failure instead of hiding it.

## Commit Policy

Commit one reviewed document at a time.

Stage only the files that passed QA:

```sh
git add docs/design/<doc>.md
git commit -m "Add SkillSpec <topic> design doc"
```

Do not include local trace directories, temporary output, or unrelated working
tree changes in doc commits.

## Reviewer Notes

A reviewer should be able to pick any paragraph and ask:

- Which source supports this?
- Is this implementation behavior or design guidance?
- Is this current v0 behavior?
- Does the parser enforce it, or does a harness have to enforce it?
- Does the doc say what remains unproven?

If the doc cannot answer those questions, it needs another pass.

## Source Alignment

This doc is grounded in the QA pattern used for the design docs in this
directory and the repository source map in `docs/design/README.md`. It also
reflects the repo's existing command surface in
`crates/skillspec-cli/src/cli/args.rs` and
`crates/skillspec-cli/src/cli/dispatch.rs`, plus the implementation files cited
throughout the design-doc set.
