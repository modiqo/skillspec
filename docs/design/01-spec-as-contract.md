# SkillSpec As Contract

SkillSpec exists to make a skill's behavior inspectable, testable, and
replayable. It is not a prose instruction set, and it is not a programming
language. It is a structured behavior contract between four parties:

- the skill author, who declares intended behavior;
- the agent, which must decide what parts of the skill are active for a task;
- the harness, which decides what tools can actually run and records evidence;
- the reviewer, who needs to check whether a run followed the declared behavior.

The core design rule is:

> Keep prose for human judgment. Put steering behavior in a contract.

## What The Contract Means

A contract is not just a file format. In SkillSpec, the contract is the set of
claims that validators, tests, compilers, decision traces, and alignment reports
can inspect.

The current v0 contract includes:

- identity and compatibility: `schema`, `id`, `title`, and `description`;
- applicability signals: `applies_when` and `entry`;
- route choices: `routes`;
- steering rules: `rules`;
- bounded questions: `elicitations`;
- lifecycle positions: `states`;
- dependency declarations: `dependencies`;
- explicit local instruction loading: `imports`;
- provenance and supporting material: `resources`;
- executable or reusable fragments: `commands`, `code`, `recipes`,
  `artifacts`, and `snippets`;
- completion obligations: `closures`;
- decision evidence: `trace`;
- regression checks: `tests`;
- proof expectations and review flags: `proof` and `review_required`;
- extension metadata: `metadata`.

That shape is enforced by the Rust model and parser, the JSON Schema, and the
reference grammar. Unknown fields are rejected in the typed grammar, required
fields must be present, identifiers are validated, and references between
sections must point at known ids.

The contract is intentionally narrower than a full instruction manual. It does
not try to encode every explanation a human author might write. It captures the
behavioral parts that create risk when they stay buried in paragraphs:

- which route should be selected;
- which substitutions are forbidden;
- when a question must be asked;
- which dependencies must exist;
- which imports are allowed to load;
- which completion work must happen;
- which trace evidence should exist after a run.

## Why This Is Not Prose

Prose skills are useful because natural language carries tone, examples, domain
judgment, and explanation. SkillSpec does not reject that. The existing docs say
the intended split directly: keep prose, structure the decisions.

The problem is that prose is hard to audit mechanically. If a `SKILL.md` says
"ask before doing anything destructive" in one paragraph, "run the cleanup
command" in another paragraph, and "only use browser automation after a decision"
later in the file, the harness has to rely on the model remembering and applying
all of that text correctly. A reviewer also has to read the whole document to
discover what should have happened.

SkillSpec changes that surface:

- a destructive command can carry a safety class;
- entry policy can forbid named actions before a decision;
- an elicitation can be represented as a bounded question;
- a dependency can be checked before use;
- a decision can emit trace events;
- a test can assert route, forbid, elicitation, after-success, or matched-rule
  behavior.

Prose still belongs in imports, resources, snippets, examples, and surrounding
documentation. It should not be the only place where critical decision behavior
exists.

## Why This Is Not A Language

SkillSpec has a grammar, but that grammar is a conformance surface, not an
arbitrary expression language.

The v0 reference explicitly avoids an arbitrary expression language. Predicates
are a fixed set of typed signals such as `user_says_any`,
`user_says_all_groups`, `task_recurrence_likely`, `domain_object_task`,
`interactive_prompt_likely`, and `command_likely_long_running`. Rule effects are
fixed steering operations such as `prefer`, `route_order`, `forbid`, `allow`,
`elicit`, and `after_success`.

That choice prevents the spec from becoming another runtime hidden inside YAML.
The file can say how to steer and what to prove. It cannot execute arbitrary
logic by itself. This is why the grammar is useful:

- it constrains what authors can declare;
- it gives validators something exact to reject;
- it lets tests compare deterministic decision results;
- it gives agents query handles instead of forcing them to parse prose;
- it lets traces refer to stable ids.

The grammar is a contract language only in the legal or API sense: it defines
allowed structure and obligations. It is not a general-purpose programming
language.

## Why This Is Not An Execution Engine

The v0 CLI can validate a spec, decide a route, explain a decision, run scenario
tests, check declared dependencies, check local imports, compile loader files,
import prose into a scaffold, and replay decision traces for alignment.

The v0 CLI does not execute arbitrary task work.

That distinction is central to the design. A spec may declare a command template,
a recipe step, a state transition, an execution plan, or a jump point. Those
declarations make work inspectable, but they do not bypass the harness. The
harness still decides whether a shell command can run, whether a network call is
allowed, whether a browser session can be attached, and whether user approval is
required.

This separation keeps SkillSpec portable. The same contract can be consumed by a
CLI, a local agent harness, a browser-oriented harness, or a future product
runtime without embedding one runtime's execution policy into the format.

## What The Contract Gives Each Reader

For a skill author, SkillSpec gives a place to put decisions that should not
depend on paragraph order or model memory. The author can declare route choices,
rules, dependencies, imports, commands, recipes, tests, and review notes.

For an agent, SkillSpec gives a progressive path through the skill. The agent can
start with `sensemake`, run `decide`, inspect only the active rule, route,
dependencies, commands, recipes, imports, or state with `query` and `refs`, and
avoid loading the whole skill file unless necessary.

For a harness, SkillSpec gives a contract to enforce around the model. The
harness can require a decision before tools, load only declared imports, ask only
bounded elicitations, check dependencies, enforce external approval policy, and
record structured evidence.

For a reviewer, SkillSpec gives a way to compare what was supposed to happen with
what did happen. Tests check deterministic steering behavior. Traces record
decision events. Alignment reports replay the captured decision and identify
which execution obligations are proven, failed, or still unproven.

## Enforcement Surfaces

SkillSpec is useful because several repo components inspect the same contract:

- `skillspec validate` parses the spec, enforces the supported schema, validates
  required identity fields, rejects unknown typed fields, validates identifiers,
  and checks cross-section references.
- `skillspec test` runs scenario expectations over the decision engine.
- `skillspec decide` evaluates rules in order, records selected route, route
  order, forbids, allows, elicitations, after-success closures, matched rules,
  and optional trace events.
- `skillspec act` renders the selected route as an action checklist, including
  the effective phase tool boundary inherited from entry, route, and phase.
- `skillspec imports check` validates explicit local imports, Markdown sections,
  and dependency-first load order.
- `skillspec deps check` checks locally verifiable dependencies and marks
  harness-specific dependency kinds as deferred.
- `skillspec compile` generates thin loader skills so `SKILL.md` stays a
  trampoline into the spec rather than becoming a second source of truth.
- `skillspec import-skill` scaffolds a structured spec from existing prose and
  marks review work instead of claiming semantic conversion is complete.
- `skillspec trace align` replays decision evidence and separates deterministic
  decision alignment from execution obligations that require structured proof.

No single command makes a skill safe. The design depends on layering: typed
specification, deterministic decision checks, explicit imports, dependency
checks, harness policy, traces, and review.

## The Contract Boundary

The boundary is easiest to see by asking what a claim depends on.

If a claim depends on route choice, matched rules, forbids, elicitations, route
order, or after-success closures, it belongs in the spec and can be tested by the
decision engine.

If a claim depends on reading extra instructions, it belongs in an explicit
`imports` entry with a local relative path and optional section, or in
`resources` if it is provenance rather than active guidance.

If a claim depends on a local CLI, file, environment variable, service, adapter,
or browser substrate, it belongs in `dependencies`. The v0 CLI can check some
dependency kinds directly and marks others as harness-required.

If a claim depends on which tool, data source, execution substrate, provider,
adapter, CLI, browser mode, API, or skill the harness may use next, it belongs
in `tool_boundary`. `skillspec act` renders a default-deny effective boundary
for every phase. Anything outside that boundary requires explicit user
permission before use.

If a claim depends on actually running a command, using a browser, calling a
service, editing files, or taking a destructive action, it crosses into harness
execution policy. The spec can declare the command, safety class, dependency, and
expected evidence, but it does not grant permission or execute the action.

If a claim depends on whether a completed run followed the contract, it belongs
in trace and alignment evidence. Decision replay can be deterministic, while
execution proof may still be unproven until the harness supplies structured
execution events.

## Design Consequences

Because SkillSpec is a contract, ids are stable API. A route id, rule id,
elicitation id, dependency id, import id, command id, recipe id, state id, or
closure id is not just a label for humans. It is also what tests, refs, traces,
alignment reports, and harnesses use to connect behavior.

Because SkillSpec is not prose, critical behavior should not exist only in
paragraphs. Prose can explain why a rule exists, but the route, predicate, forbid,
elicitation, or closure should be structured.

Because SkillSpec is not a language, authors should not hide logic in strings.
If the fixed predicate and effect vocabulary cannot express a behavior, that is
either a reason to keep the behavior in harness code or a reason to extend the
spec deliberately.

Because SkillSpec is not an execution engine, command templates and recipes must
be treated as declarations. The harness decides whether and how to run them.

Because SkillSpec is not a security boundary by itself, reviewers should look for
the full chain: structured spec, tests, dependency checks, harness enforcement,
decision trace, and execution evidence.

## Source Alignment

This doc is grounded in:

- `spec/README.md`, which defines SkillSpec v0 as a structured skill format and
  names the v0 non-goals: no inheritance, no execution engine, no arbitrary
  expression language, and no hidden network or tool execution.
- `spec/semantics.md`, which says SkillSpec steers work but does not execute the
  work.
- `spec/grammar.md`, which describes the conceptual grammar and fixed section
  structure.
- `docs/01-why-skillspec.md`, which frames the purpose as keeping prose while
  structuring decisions.
- `docs/02-prose-vs-skillspec.md`, which states that import is conservative and
  review-driven.
- `crates/skillspec-cli/src/spec/model.rs`,
  `crates/skillspec-cli/src/spec/parser.rs`,
  `crates/skillspec-cli/src/execution/decision.rs`,
  `crates/skillspec-cli/src/features/compiler.rs`,
  `crates/skillspec-cli/src/execution/deps.rs`,
  `crates/skillspec-cli/src/spec/imports.rs`,
  `crates/skillspec-cli/src/features/importer.rs`, and
  `crates/skillspec-cli/src/execution/align.rs`, which implement the current
  contract surfaces.
