# Skill Authoring Lifecycle

SkillSpec supports two authoring paths:

- author a structured `skill.spec.yml` directly;
- import an existing prose skill and then promote the important behavior into a
  reviewed contract.

Both paths end in the same place: a SkillSpec-backed skill whose behavior is
validated, tested, traceable, and thinly exposed to agents through a loader
`SKILL.md`.

## Lifecycle Overview

The lifecycle has six stages:

1. Capture existing material.
2. Build or import a draft spec.
3. Promote decision-heavy behavior into structured routes, rules, states,
   dependencies, imports, commands, recipes, tests, and closures.
4. Validate the spec and its local import graph.
5. Compile or maintain a thin loader so agents enter through the spec.
6. Execute through a harness, preserve decision traces, and use alignment reports
   to drive review.

The important design point is that import is not the end of authoring. Import is
the start of contract creation.

## Direct Authoring Path

Direct authoring starts with a blank or example `skill.spec.yml`.

The author should begin with the smallest contract that can be tested:

- `schema: skillspec/v0`;
- stable `id`, `title`, and `description`;
- one or more `routes`;
- one or more `rules` that select or modify those routes;
- scenario `tests` that prove the expected decision behavior.

The next layer adds safety and execution context:

- `entry` for the initial invocation contract;
- `dependencies` for local CLIs, files, environment variables, services,
  adapters, and browser requirements;
- `imports` for explicit local instruction material;
- `resources` for provenance and supporting material;
- `commands`, `code`, `recipes`, and `artifacts` for declared work surfaces;
- `states` for lifecycle positions;
- `closures` and `after_success` rules for completion obligations;
- `trace` and `proof` for review requirements.

Direct authoring is appropriate when the author already knows the behavior they
want to make inspectable. The author should keep prose explanations in nearby
docs, imports, resources, and snippets, but move route choice, forbids,
elicitations, dependencies, and proof obligations into the structured spec.

## Prose Import Path

The import path is for an existing `SKILL.md` or a directory of Markdown files.

The CLI command is:

```sh
skillspec source map <path> --out <draft>/.skillspec/source-map
skillspec source coverage <draft>/.skillspec/source-map/source-map.json
skillspec source query <draft>/.skillspec/source-map/source-map.json nodes --view index
skillspec source query <draft>/.skillspec/source-map/source-map.json dependencies --view summary
skillspec source stale <draft>/.skillspec/source-map/source-map.json --root <path>
skillspec import-skill <path> --out <draft>/skill.spec.yml --source-map <draft>/.skillspec/source-map/source-map.json
```

The source-map step lets the agent inspect structure, dependencies, code blocks,
references, and exact source spans before importing. The current importer reads
local Markdown source material and creates a SkillSpec scaffold. It can:

- read a single skill file or recursively collect Markdown from a directory;
- skip dot directories, `target`, and `node_modules` during recursive collection;
- extract title and summary material;
- preserve source documents as `resources` or `imports`;
- extract shell-like command blocks into `commands`;
- extract fenced code blocks into package-local resource files and `code`
  entries;
- infer simple CLI dependencies from command blocks and code languages;
- infer package dependencies from Python and JavaScript/TypeScript imports in
  fenced code blocks;
- generate `deps.toml` beside the output draft and declare it as a file
  dependency/artifact;
- attach provenance to imported code blocks;
- create a `source_summary` snippet;
- add `metadata` counts for source kind, documents, headings, command blocks,
  code blocks, and strong directives;
- add `review_required` notes.

The importer creates a dependency ledger scaffold, not a complete dependency
review. For imported or shareable skills, the review pass must complete
`deps.toml` by preserving dependency mentions from `SKILL.md`, referenced docs,
helper scripts, fenced code imports, command examples, and package manifests.
Each entry should record source authority, local status, install risk, proposed
provision command, required workflows, and degraded proof impact. A reviewed
zero-dependency skill should keep `deps.toml` with `dependency_count = 0`; a
byte-empty ledger is not valid proof that dependencies were reviewed.

The importer deliberately does not finish the behavioral contract. The generated
spec starts with empty `applies_when`, `entry`, `routes`, `rules`, `states`,
`elicitations`, `trace`, `recipes`, `closures`, `proof`, and `tests`, while
`artifacts` contains the generated dependency ledger.

That empty structure is intentional. The importer cannot prove that prose
instructions have become correct routing rules, bounded questions, safety
classes, dependency policies, recipes, or scenario tests. It preserves material
and identifies review work so an author or harness can promote it deliberately.

## What Import Should Produce

After import, the scaffold is a working draft, not a trusted skill. The next
review pass should answer these questions:

- Which paragraphs describe route choices?
- Which phrases are hard constraints that belong in `forbid` or entry policy?
- Which phrases describe questions that should become `elicitations`?
- Which command blocks are examples, and which are command templates?
- Which command templates need a safety class?
- Which dependencies were inferred correctly, and which need permission or
  provision choices?
- Which dependency mentions came from source-required prose, reference prose,
  helper scripts, code imports, command examples, package manifests, or
  inference, and are they preserved in `deps.toml` without being deleted to make
  QA pass?
- Which Markdown files should be active `imports`, and which should remain
  passive `resources`?
- Which fenced code blocks are examples, and which are runnable code surfaces?
- Which lifecycle steps should become `states`, `recipes`, or execution phases?
- Which behaviors need scenario tests before the skill can be trusted?

The answer to those questions is the real authoring work.

Missing dependencies are not proof shortcuts. If a required imported dependency
is absent, the reviewed skill must either provision it with user approval or
record it as missing/deferred/required_but_unproven in `deps.toml`; the final
proof is partial until that dependency-backed workflow is actually proven.

## Harness As Author

A harness can participate in authoring, but it should not be treated as an
oracle. In the current repo, the implemented tools give a harness enough
structure to assist:

- `import-skill` can preserve prose material and produce a scaffold;
- `sensemake` can expose section counts, query handles, navigation, and
  escalation guidance;
- `decide` can show current route behavior for a task;
- `query` and `refs` can inspect precise structured elements;
- `test` can run scenario expectations;
- `trace align` can replay a decision trace and identify unproven execution
  obligations.

That means a harness can draft, inspect, and iterate on a spec. It should still
leave review evidence. When a harness promotes prose into rules, commands,
recipes, or tests, the change must be checked against the source material and the
current parser. The harness should also avoid rewriting the generated loader as a
second source of truth.

The safe authoring loop is:

1. Import or edit the spec.
2. Validate the spec.
3. Run scenario tests.
4. Run an example decision with a trace directory.
5. Inspect the trace and alignment report.
6. Add review notes for any unproven behavior.
7. Commit the spec and loader only after the diff matches source evidence.

## Execution Path

Execution begins with a user task and a SkillSpec-backed loader. The generated
loader tells the agent that the colocated `skill.spec.yml` is the source of
truth. The expected runtime flow is:

1. Load the spec.
2. If the shape is unfamiliar, run `skillspec sensemake ./skill.spec.yml --view
   index`.
3. Run `skillspec decide ./skill.spec.yml --input '<user task>' --trace-dir
   .skillspec/traces`.
4. Read the decision JSON before using tools.
5. Pull active details with `skillspec query` and `skillspec refs`.
6. Check dependencies for any active command or route.
7. Load only the imports required by the active route, rule, recipe, command, or
   code path.
8. Execute through the harness approval and tool policy.
9. Preserve the trace run directory.
10. Run alignment after the work, adding structured execution evidence when the
    harness has it.

The decision result is not enough by itself. A selected route says how the spec
expects the agent to proceed. The harness still needs to enforce policy, collect
evidence, and prove whether the actual run followed the contract.

## Review Path

Review should compare three layers:

- source intent: the original prose, imports, resources, and examples;
- structured intent: the spec fields and scenario tests;
- runtime evidence: decision traces and execution evidence.

A review is incomplete if it only reads the prose or only runs `validate`.
Validation proves shape and references. Scenario tests prove deterministic
decision expectations. Import checks prove local import paths and sections.
Dependency checks prove only the dependencies the CLI can locally check. Trace
alignment proves decision replay and reports which execution obligations remain
unproven.

## Iteration Path

SkillSpec should evolve through small, testable edits:

- add a route before adding rules that select it;
- add a rule before relying on a decision;
- add a test before trusting a new rule;
- add a dependency before referencing it from a command;
- add an import before expecting a harness to load extra guidance;
- add a closure before reporting completion work as required;
- add trace or proof requirements before using alignment as review evidence.

Each edit should keep ids stable when possible. Ids are referenced by tests,
trace events, alignment reports, imports, resources, code, recipes, commands,
states, and harness integrations. Renaming an id is a contract change.

## Minimal Acceptance Gate

A SkillSpec-backed skill should not be treated as reliable until it passes at
least:

```sh
skillspec validate skill.spec.yml
skillspec imports check skill.spec.yml
skillspec test skill.spec.yml
skillspec deps check skill.spec.yml
skillspec decide skill.spec.yml --input '<representative task>' --trace-dir .skillspec/traces
```

For a real run, add:

```sh
skillspec trace align skill.spec.yml --decision-trace <run-dir>
```

If the harness recorded structured execution events, include them in the
alignment command. Without execution evidence, alignment can still pass decision
replay while leaving execution obligations unproven.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-cli/src/importer.rs`, especially `import_skill`,
  `imports_resources_and_code`, `commands_from_blocks`, and
  `dependencies_from_analysis`;
- `crates/skillspec-cli/src/compiler.rs`, which generates thin loader skills;
- `crates/skillspec-cli/src/main.rs`, which exposes `validate`, `test`,
  `decide`, `sensemake`, `query`, `refs`, `imports check`, `deps check`,
  `compile`, `import-skill`, and `trace align`;
- `crates/skillspec-cli/src/decision.rs`, `imports.rs`, `deps.rs`, and
  `align.rs`, which implement the runtime and review surfaces;
- `docs/02-prose-vs-skillspec.md`, which states that import is conservative and
  review-driven.
