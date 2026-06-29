# Grammar And Conformance

SkillSpec has a grammar so agents, harnesses, validators, tests, and reviewers
can agree on the shape of a skill contract.

The grammar is not a programming language. It is a conformity surface: it says
which sections exist, which fields are accepted, how ids are named, and which
references must resolve.

## Canonical Shape

The canonical interchange format is YAML. The conceptual grammar is documented
in `spec/grammar.md`, the machine-readable schema lives in
`spec/skill.spec.schema.json`, and the current core implementation is typed in
`crates/skillspec-core/src/spec/model.rs`.

For visual orientation, use the
[`docs/grammar-atlas/`](../grammar-atlas/README.md) companion. It renders the
top-level type schema, reference graph, dataflow/loading model, invariants, and
a worked example as SVG plates. The atlas is explanatory; the formal grammar,
schema, Rust model, parser, and conformance fixtures remain the source of truth.

The top-level model currently includes:

- `schema`;
- `id`;
- `title`;
- `description`;
- `applies_when`;
- `entry`;
- `routes`;
- `rules`;
- `states`;
- `elicitations`;
- `trace`;
- `dependencies`;
- `imports`;
- `resources`;
- `code`;
- `artifacts`;
- `recipes`;
- `commands`;
- `snippets`;
- `closures`;
- `proof`;
- `tests`;
- `review_required`;
- `metadata`.

The required identity fields are `schema`, `id`, `title`, and `description`.
`validate_spec` rejects any schema other than `skillspec/v0`, and it rejects an
empty `id`, `title`, or `description`.

## Strict Fields

The typed Rust model uses serde `deny_unknown_fields` on the top-level spec and
on the structured sections. That means a misspelled field is not silently
accepted as prose or metadata.

This matters because a contract that accepts unknown fields too easily becomes
ambiguous. For example, if an author writes `preffer` instead of `prefer`, the
validator should reject the spec rather than letting the rule appear to exist
while doing nothing.

The repo contains parser tests that reject unknown fields across the typed
grammar. The conformance fixture
`conformance/invalid/unknown-rule-field.skill.spec.yml` exists for the same
reason.

`metadata` is the intended extension surface for non-contract information. A new
behavioral field should be added deliberately to the model, schema, parser,
reference docs, and tests.

## Identifiers

Ids are part of the public contract. They connect sections to each other and are
used by tests, traces, refs, queries, loaders, and harnesses.

The parser validates identifiers with this shape:

- the first character must be an ASCII lowercase letter;
- remaining characters may be ASCII lowercase letters, ASCII digits, `_`, `-`,
  or `.`;
- empty ids are invalid.

The parser currently applies this to top-level `id`, route ids, rule ids, state
keys, dependency keys, import keys, resource keys, code keys, artifact keys,
recipe keys, command keys, elicitation keys, elicitation choice ids, and other
ids checked directly by validation code. Closure keys are part of the action-id
set used by references such as `rules.after_success`, so they should follow the
same convention even where validation is reference-driven.

Treat renaming an id as a contract change. It can break tests, refs, traces,
alignment reports, loaders, and harness code.

## Section Roles

The sections are not interchangeable:

- `routes` name strategies for satisfying a task.
- `rules` steer route selection and obligations.
- `states` describe lifecycle positions.
- `elicitations` define bounded questions.
- `trace` declares decision evidence requirements.
- `dependencies` declare required local or harness substrates.
- `imports` declare local runtime-loadable instruction material.
- `resources` declare provenance and supporting material.
- `code` preserves executable or example code blocks.
- `artifacts` declare produced or consumed outputs.
- `recipes` declare ordered procedures.
- `commands` declare command templates.
- `snippets` declare reusable prose fragments.
- `closures` declare completion obligations.
- `proof` declares verification metrics.
- `tests` declare scenario expectations.
- `review_required` records known review work.
- `metadata` carries non-contract extension data.

The grammar should keep authors from smearing those roles together. A Markdown
file is not active guidance unless it is declared as an import. A command string
is not permission to run. A state transition is not an execution engine.

## Reference Validation

Conformance is more than field shape. The parser validates references among
sections.

Examples:

- `rules.prefer` must name a known route.
- `rules.route_order` must name known routes.
- `rules.elicit` must name known elicitations.
- `rules.after_success` must name a known action id.
- `states.do` must name known actions.
- `states.next`, `states.yes`, and `states.no` must name known states.
- `states.ask` must name a known elicitation.
- `elicitations.choices.route` must name a known route.
- `elicitations.choices.next` must name a known state.
- `elicitations.required_when.route` must name a known route.
- `elicitations.*.question` is a string; when hand-authoring YAML, quote
  question values that contain `: ` so YAML does not parse the colon as a
  nested mapping.
- `commands.requires.dependencies` must name known dependencies.
- `code.requires.dependencies`, `code.requires.imports`,
  `code.requires.resources`, and `code.requires.artifacts` must name known ids.
- `recipes.requires.*` and recipe steps must name known targets.
- `artifacts.produced_by` and `artifacts.consumed_by` must name known executable
  references. Executable references are only `command`, `code`, or `recipe`;
  routes and rules are control-flow and cannot be artifact consumers.
- test expectations that name routes, rules, actions, or elicitations must name
  known ids.

The parser also checks import cycles and orphaned imports/resources. An import
must either be referenced, explicitly used, or marked `load: always`; otherwise
it is rejected as an orphan. Resources follow the same basic provenance rule:
supporting material should be connected to the contract or removed.

## Imports In The Grammar

Imports are explicit local instruction material. They have their own grammar
because loading extra context is a behavior, not an accident.

An import can declare:

- `path`;
- `role`;
- `description`;
- `section`;
- `load`;
- `requires.imports`;
- `used_by`;
- `load_when`.

The import checker enforces local relative paths. It rejects absolute paths and
URL-like paths containing `://`. It resolves paths relative to the spec
directory, checks file existence, checks declared Markdown sections, and reports
dependency-first load order.

Nested imports are declared with `requires.imports`. They are not inheritance,
and they do not merge another SkillSpec into the current spec.

## Scenario Test Grammar

Scenario tests are contract checks for the decision engine. A test has:

- `name`;
- `input`;
- `expect`.

The expectation must contain at least one assertion. Empty expectations are
invalid because they do not prove behavior.

Current expectation fields include:

- `route`;
- `route_order`;
- `plan_phases`;
- `plan_jumps`;
- `forbid`;
- `forbid_exact`;
- `not_forbid`;
- `elicit`;
- `elicit_exact`;
- `not_elicit`;
- `after_success`;
- `after_success_exact`;
- `not_after_success`;
- `matched_rules`;
- `matched_rules_exact`;
- `not_matched_rules`.

Inclusion expectations check that a value appears. `*_exact` expectations check
the exact set. `not_*` expectations check absence.

The decision test runner also formats execution-plan jumps as
`phase_id:when->to_phase` for `plan_jumps` expectations.

## Safety Classes

Safety classes are fixed enum values:

- `read_only`;
- `local_read`;
- `local_write`;
- `network_read`;
- `network_write`;
- `browser_attach`;
- `credential_request`;
- `destructive`.

A safety class is a declaration. It helps the agent and harness reason about
risk, but it does not grant permission and it does not replace harness policy.

## Dependency Kinds

Dependency kinds are fixed enum values:

- `cli`;
- `package`;
- `file`;
- `env`;
- `service`;
- `adapter`;
- `browser`.

The CLI can directly check some dependency kinds, such as local CLI commands,
files, and environment variables. Other dependency kinds are harness-specific
and may be reported as deferred rather than present or missing.

## Conformance Fixtures

The repo keeps conformance fixtures under `conformance/`:

- `valid/minimal.skill.spec.yml`;
- `valid/imports.skill.spec.yml`;
- `invalid/import-cycle.skill.spec.yml`;
- `invalid/unknown-rule-field.skill.spec.yml`.

These fixtures are intentionally small. They are not examples of rich skills;
they are compatibility anchors. A parser or schema change should preserve the
valid fixtures and reject the invalid fixtures unless the format is deliberately
changed.

## Grammar Evolution

Changing the grammar should update all relevant surfaces:

- `crates/skillspec-core/src/spec/model.rs`;
- `crates/skillspec-core/src/spec/parser/validation.rs`;
- `crates/skillspec-cli/src/execution/decision.rs` when decision behavior changes;
- `crates/skillspec-cli/src/features/sensemake.rs` when query or refs behavior changes;
- `spec/skill.spec.schema.json`;
- `spec/grammar.md`;
- `spec/semantics.md` if meaning changes;
- `conformance/valid/` and `conformance/invalid/`;
- examples and generated golden fixtures;
- design docs that explain the affected behavior.

Do not add a field to only one layer. A field accepted by the Rust model but not
documented in the grammar is hard to teach. A field documented in prose but not
accepted by the parser is a hallucination trap.

## Source Alignment

This doc is grounded in:

- `spec/grammar.md`, which defines the conceptual grammar;
- `spec/skill.spec.schema.json`, which defines the JSON Schema surface;
- `crates/skillspec-core/src/spec/model.rs`, which defines the typed Rust model;
- `crates/skillspec-core/src/spec/parser/validation.rs`, which validates schema, required fields,
  identifiers, references, import cycles, orphaned imports/resources, and test
  expectations;
- `crates/skillspec-cli/src/execution/decision.rs`, which evaluates scenario
  expectations;
- `crates/skillspec-core/src/spec/imports.rs`, which validates local imports;
- `conformance/`, which contains valid and invalid compatibility fixtures.
