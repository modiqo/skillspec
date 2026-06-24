# Package Anatomy

A SkillSpec-backed skill is a small package. The important files are:

```text
my-skill/
  SKILL.md
  skill.spec.yml
  deps.toml
  references/
  examples/
  scripts/
```

Only `SKILL.md` and `skill.spec.yml` are required for the core SkillSpec
pattern. `deps.toml`, references, examples, scripts, and other files are
supporting material. They become part of the contract only when the spec declares
how they are used.

## The Three Core Parts

The package has three distinct layers:

- `SKILL.md`: a thin loader or trampoline that tells the agent to enter through
  `skill.spec.yml`.
- `skill.spec.yml`: the structured behavior contract and source of truth.
- `deps.toml`: an optional companion dependency manifest for surrounding tooling,
  such as rote release or replay checks.

These layers should not duplicate each other. The loader points at the contract.
The contract declares behavior. The dependency manifest supports external
dependency workflows.

## Thin `SKILL.md` Loader

The loader exists because agent products already know how to discover and load
`SKILL.md`. SkillSpec uses that discovery mechanism without letting prose become
the behavioral source of truth.

A generated loader contains:

- frontmatter with `name` and `description`;
- a short title and summary;
- an entry gate when the spec declares one;
- a runtime contract that says to load `./skill.spec.yml`;
- a command to run `skillspec decide` with `--trace-dir`;
- guidance to use `sensemake`, `query`, and `refs` instead of reading the whole
  YAML by default;
- an execution checklist for route, matched rules, forbids, elicitations,
  dependencies, imports, commands, recipes, code, and closures;
- quick commands for validation, import checks, tests, dependency checks, query,
  refs, explain, and trace alignment;
- completion reporting expectations;
- route hints generated from the spec.

The loader is intentionally small relative to the behavior it points to. Its job
is to get the agent into the structured contract, not to restate every route and
rule as prose.

### Minimum Loader Responsibilities

A minimum loader should do four things:

1. Identify the skill to the host agent with ordinary `SKILL.md` metadata.
2. Declare that `skill.spec.yml` is the source of truth.
3. Require a SkillSpec decision before task actions when the spec entry contract
   says `decision_required`.
4. Tell the agent how to inspect the active parts of the spec without dumping the
   whole file into context.

Everything else is elaboration. The generated loaders in `examples/` include
more operational detail because they are meant to be safe default guidance for
real agent runs.

### Loader Anti-Patterns

Avoid these loader mistakes:

- copying the full route and rule logic into prose;
- adding behavior that does not exist in `skill.spec.yml`;
- treating route hints as enough to act without reading the decision JSON;
- using the loader to grant tool permission;
- expanding the loader every time the spec changes instead of regenerating or
  keeping it thin.

If the loader and spec disagree, the spec should win and the loader should be
fixed.

## `skill.spec.yml`

`skill.spec.yml` is the contract file. It is the part validators, tests,
decision traces, compilers, import checks, dependency checks, and alignment
reports understand.

The top-level fields are typed by the Rust model and schema. The current model
includes:

- `schema`, `id`, `title`, `description`;
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

The parser validates the supported schema, required identity fields,
identifiers, known references, import cycles, import orphaning, resource
orphaning, and scenario-test assertions. The typed grammar rejects unknown fields
through serde `deny_unknown_fields`.

The spec is where critical behavior belongs:

- route selection and route order;
- rule predicates and effects;
- forbids and narrow allows;
- bounded elicitation questions;
- dependency readiness requirements;
- explicit imports and their load behavior;
- resources and provenance;
- command templates and safety declarations;
- recipe steps;
- state transitions;
- execution plans, handoffs, and jumps;
- completion closures;
- trace requirements;
- scenario tests and proof expectations.

The spec may reference prose, code, or dependency manifests. Those files do not
become active behavior just because they exist in the folder. They become part
of the contract only when declared through `imports`, `resources`, `code`,
`dependencies`, `commands`, `recipes`, or another typed section.

## `deps.toml`

`deps.toml` is a companion dependency manifest used by surrounding tooling. In
the `examples/durable-executor` package it looks like:

```toml
schema_version = 1

[[tools]]
id = "rote"
command = "rote"
required = true
```

The full example also lists optional tools such as `git`, `gh`, `deno`, `cargo`,
and `curl`.

The important boundary is:

- `skillspec deps check skill.spec.yml` reads dependency declarations from
  `skill.spec.yml`;
- `rote deps check deps.toml` reads the rote dependency manifest;
- a SkillSpec can declare `deps.toml` as a file dependency when that manifest is
  required for the skill's release or replay workflow.

In `examples/durable-executor/skill.spec.yml`, `deps_toml` is declared as a file
dependency with `path: deps.toml`, and the `deps_check` command template runs
`rote deps check deps.toml`. That makes the manifest visible to the SkillSpec
contract without making the SkillSpec CLI parse `deps.toml` directly.

This split is deliberate. SkillSpec dependencies describe what the skill
contract needs. External dependency manifests can describe what another tool
needs. The spec can connect to those manifests explicitly, but it should not
hide them as implicit package behavior.

For imported or shareable skills, `deps.toml` is also the dependency provenance
ledger. `skillspec import-skill` creates a scaffolded ledger beside the draft
spec and declares it as a file dependency/artifact, so `skillspec deps check`
can report a missing or byte-empty ledger. The review pass must complete that
scaffold by preserving every dependency mention found in `SKILL.md`, referenced
docs, helper scripts, fenced code imports, command examples, and package
manifests. A dependency record should include:

- dependency id and ecosystem;
- authority such as `source_required`, `source_recommended`,
  `reference_required`, `script_import`, `example_only`, or `inferred`;
- source location;
- local status such as `present`, `missing`, `unknown`, `provisionable`,
  `deferred`, or `required_but_unproven`;
- workflows that require it;
- install risk and proposed provision command;
- degraded proof impact and any user approval or waiver.

Dependency checks may update local status, but they should not delete records to
make QA pass. If a required imported dependency is missing and not provisioned
or explicitly deferred, the skill's final proof is partial or blocked.

## Supporting Files

Supporting files fall into two categories.

Active guidance belongs in `imports`. Imports are local, explicit, and
runtime-loadable. They can have a role, optional Markdown section, load mode,
dependencies on other imports, `used_by` links, and `load_when` notes.

Supporting provenance belongs in `resources`. Resources are source material,
references, examples, or required procedures that explain or prove where a
contract came from. A resource does not become an instruction just because it is
near the spec.

Code and scripts can be represented in `code`, `commands`, or `recipes`.
Fenced code imported from prose should be treated as review material until the
spec gives it a purpose, safety declaration, dependencies, inputs, outputs, and
usage conditions.

## Compile Targets

The compiler can render a SkillSpec into:

- a Codex skill loader;
- a Claude skill loader;
- Markdown documentation.

For Codex and Claude targets, the compiler writes a thin loader rather than a
full prose copy of the spec. The generated loader directs the agent to
`skill.spec.yml`, tells it to run `sensemake` when needed, requires `decide`
before tools when applicable, and points at `query`, `refs`, dependency checks,
and trace alignment.

The compile step should not create a second behavioral authority. It should
make the package usable by a host agent while preserving `skill.spec.yml` as the
contract.

## Package Review Checklist

Use this checklist when reviewing a SkillSpec-backed package:

- `SKILL.md` declares the spec as source of truth.
- `SKILL.md` does not duplicate or contradict route and rule behavior.
- `skill.spec.yml` validates.
- `skillspec imports check skill.spec.yml` passes when imports are declared.
- `skillspec test skill.spec.yml` passes scenario expectations.
- `skillspec deps check skill.spec.yml` reports dependency readiness or known
  deferred checks.
- Any `deps.toml` is either clearly external tooling metadata or declared in the
  spec as a file dependency when the skill relies on it.
- Active prose is declared as `imports`.
- Passive source material is declared as `resources`.
- Command templates and code blocks have dependencies and safety declarations
  where applicable.
- The package can produce a decision trace for representative tasks.
- Alignment can replay that decision trace and reports unproven execution
  obligations honestly.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-cli/src/compiler.rs`, which generates thin loaders for
  Codex and Claude skill targets;
- `examples/durable-executor/SKILL.md`, which shows the generated loader shape;
- `examples/durable-executor/skill.spec.yml`, which shows entry policy, routes, rules,
  dependencies, file dependency `deps_toml`, and command template `deps_check`;
- `examples/durable-executor/deps.toml`, which shows the companion tool manifest;
- `crates/skillspec-cli/src/deps.rs`, which checks dependencies declared in
  `skill.spec.yml`;
- `spec/imports.md` and `crates/skillspec-cli/src/imports.rs`, which define and
  validate local explicit imports.
