# Formal Grammar

This document defines the v0 grammar in a compact EBNF-like notation. The
canonical interchange format is YAML, but the grammar describes the conceptual
tree independent of YAML spelling.

V0 is deliberately not a programming language. It is a structured skill
contract: enough grammar to express routes, rules, elicitations, states,
commands, imports, resources, code, artifacts, recipes, snippets, closures,
tests, and proof hooks without becoming an execution engine.

## Lexical Conventions

```text
identifier      = lowercase-letter , { lowercase-letter | digit | "_" | "-" | "." } ;
route-id        = identifier ;
rule-id         = identifier ;
state-id        = identifier ;
elicitation-id = identifier ;
command-id      = identifier ;
import-id       = identifier ;
resource-id     = identifier ;
code-id         = identifier ;
artifact-id     = identifier ;
recipe-id       = identifier ;
choice-id       = identifier ;
snippet-id      = identifier ;
metric-id       = identifier ;
trace-event-id  = identifier ;
string          = YAML string scalar ;
number          = YAML integer or float scalar ;
boolean         = "true" | "false" ;
value           = string | number | boolean | sequence | mapping ;
sequence        = YAML sequence ;
mapping         = YAML mapping ;
```

Identifiers are stable API. They are what tests, compilers, and importing tools
refer to. Human labels can change; identifiers should not churn.

## Reference Rules

References are symbolic. A v0 document is well-formed when:

- every `Rule.prefer` references an existing `Route.id`
- every `Rule.route_order` item references an existing `Route.id`
- every `Rule.elicit` item references an existing elicitation
- every `State.ask` references an existing elicitation
- every `State.next`, `State.yes`, and `State.no` references an existing state
- every command id referenced from `State.do` or `Rule.after_success` exists in
  `commands` or `closures`
- every `Elicitation.required_when.route` references an existing route
- every `Elicitation.default` references one of its choices
- every `ElicitationChoice.route` references an existing route
- every `ElicitationChoice.next` references an existing state
- every `Test.expect.route` references an existing `Route.id`
- every `Test.expect.route_order` item references an existing `Route.id`
- every `Test.expect.elicit` item references an existing elicitation
- every `Trace.record` item is one of the v0 trace event kinds
- every `Command.requires.dependencies` item references an existing dependency
- every `Command.requires.resources` item references an existing resource
- every `Dependency.provision.elicit` references an existing elicitation
- every `Import.requires.imports` item references an existing import
- every `Import.used_by` item references the target it names
- every on-demand import is referenced by code or recipe, required by another
  import, or declares `used_by`
- the import dependency graph is acyclic
- every `Resource.used_by` item references the target it names
- every resource is referenced by code or recipe, or declares `used_by`
- every `Code.requires.dependencies` item references an existing dependency
- every `Code.requires.imports` item references an existing import
- every `Code.requires.resources` item references an existing resource
- every `Code.requires.artifacts`, `Code.inputs`, and `Code.outputs` item
  references an existing artifact
- every `Code.provenance.resource` references an existing resource
- every `Code.provenance.import` references an existing import
- every `Code.provenance` declares exactly one of `resource` or `import`
- every `Code.source.from_resource` references an existing resource
- every artifact producer/consumer references an existing command, code block,
  or recipe
- every recipe import/dependency/resource/artifact/code/command/elicitation
  reference points at an existing id
- every scenario test declares at least one concrete expectation
- every expectation reference to a route, elicitation, action, or rule points at
  an existing id

V0 CLI validation performs these structural and cross-reference checks. Typed v0
objects are strict: unknown fields are invalid so misspelled behavior does not
silently disappear. Explicit extension surfaces such as `metadata`, `allow`,
choice `sets`, artifact `schema`, command `success_when`, and `closures` may
carry arbitrary structured values.

## Document

```text
skillspec       = header ,
                  [ activation ] ,
                  [ applies-when ] ,
                  [ entry ] ,
                  [ routes ] ,
                  [ rules ] ,
                  [ elicitations ] ,
                  [ trace ] ,
                  [ dependencies ] ,
                  [ imports ] ,
                  [ resources ] ,
                  [ code ] ,
                  [ artifacts ] ,
                  [ recipes ] ,
                  [ states ] ,
                  [ commands ] ,
                  [ snippets ] ,
                  [ closures ] ,
                  [ proof ] ,
                  [ tests ] ,
                  [ review-required ] ,
                  [ metadata ] ;

header          = schema , id , title , description ;
schema          = "schema" ":" "skillspec/v0" ;
id              = "id" ":" identifier ;
title           = "title" ":" string ;
description     = "description" ":" string ;
```

## Activation

```text
activation      = "activation" ":" mapping ;
activation.summary = "summary" ":" string ;
activation.keywords = "keywords" ":" sequence-of string ;
activation.priority = "priority" ":" string ;
applies-when    = "applies_when" ":" sequence-of activation-hint ;
activation-hint = mapping ;
```

`activation.summary` is a harness-selection hint. Compiler targets that emit a
trampoline skill should place it at the start of the generated frontmatter
description so the harness can choose the skill before loading the full spec.
`activation.keywords` can widen that selection surface without changing route
decision. `activation.priority` is advisory metadata for humans and harnesses;
it does not execute routing by itself.

V0 does not standardize all `applies_when` predicates. The common shape is:

```yaml
activation:
  summary: Universal durable-work router with trace and alignment benefits.
  keywords:
    - remember this workflow
    - route through durable executor
applies_when:
  - user_intent:
      - recurring task
      - browser and CLI work
```

Activation is advisory. A harness may use it to decide whether to load the
skill, but it is not part of route decision.

## Entry

```text
entry           = "entry" ":" mapping ;
entry.prompt    = "prompt" ":" string ;
```

Entry describes the first user-facing question or setup posture.

## Routes

```text
routes          = "routes" ":" sequence-of route ;
route           = "id" ":" route-id ,
                  "label" ":" string ,
                  [ "rank" ":" number ] ,
                  [ "description" ":" string ] ,
                  [ "checks" ":" sequence-of command-id ] ;
```

Routes are candidate ways to satisfy work. Lower `rank` means earlier default
preference unless a rule overrides it.

## Rules

```text
rules           = "rules" ":" sequence-of rule ;
rule            = "id" ":" rule-id ,
                  [ "when" ":" predicate ] ,
                  [ "prefer" ":" route-id ] ,
                  [ "route_order" ":" sequence-of route-id ] ,
                  [ "forbid" ":" sequence-of identifier ] ,
                  [ "allow" ":" mapping ] ,
                  [ "elicit" ":" sequence-of elicitation-id ] ,
                  [ "after_success" ":" sequence-of command-id-or-closure-id ] ,
                  [ "reason" ":" string ] ;

predicate       = [ "user_says_any" ":" sequence-of string ] ,
                  [ "task_recurrence_likely" ":" boolean ] ,
                  [ "domain_object_task" ":" boolean ] ,
                  [ "interactive_prompt_likely" ":" boolean ] ,
                  [ "command_likely_long_running" ":" boolean ] ;
```

Rules are evaluated in file order. V0 rule effects are additive except
`prefer`, which sets the currently selected route, and `route_order`, which
replaces the current order.

Predicate fields compose with logical AND. The values inside
`user_says_any` compose with logical OR.

## Rule Effect Algebra

```text
decision        = input ,
                  route ,
                  route-order ,
                  forbid-set ,
                  allow-map ,
                  elicitation-list ,
                  after-success-list ,
                  matched-rule-list ,
                  reason ;

apply(rule)     = if rule.prefer then route := rule.prefer ,
                  if rule.route_order then route-order := rule.route_order ,
                  forbid-set := forbid-set union rule.forbid ,
                  allow-map := allow-map merge rule.allow ,
                  elicitation-list := append elicitation-list rule.elicit ,
                  after-success-list := append after-success-list rule.after_success ,
                  matched-rule-list := append matched-rule-list rule.id ,
                  reason := rule.reason or reason ;
```

This algebra is intentionally boring. The goal is inspectable steering, not a
hidden policy language.

## Elicitations

```text
elicitations    = "elicitations" ":" mapping-of elicitation-id to elicitation ;
elicitation     = "question" ":" string ,
                  [ "required_when" ":" sequence-of elicitation-condition ] ,
                  "choices" ":" sequence-of elicitation-choice ,
                  [ "default" ":" choice-id ] ,
                  [ "max_choices" ":" number ] ;

elicitation-condition
                = [ "route" ":" route-id ] ,
                  [ "missing" ":" identifier ] ,
                  [ "predicate" ":" predicate ] ;

elicitation-choice
                = "id" ":" choice-id ,
                  "label" ":" string ,
                  [ "description" ":" string ] ,
                  [ "sets" ":" mapping ] ,
                  [ "route" ":" route-id ] ,
                  [ "next" ":" state-id ] ,
                  [ "safety" ":" safety-class ] ;
```

Elicitations are bounded questions. They are used when the skill should ask
for a specific missing decision instead of guessing or asking an open-ended
question.

Choices may set facts, steer a route, or advance to a state. They do not
execute commands by themselves.

## Trace

```text
trace           = "trace" ":" mapping ;
trace.mode      = "mode" ":" trace-mode ;
trace.required  = [ "required" ":" boolean ] ;
trace.record    = [ "record" ":" sequence-of trace-event-kind ] ;

trace-mode      = "event_log" ;

trace-event-kind
                = "input_received"
                | "spec_loaded"
                | "rule_evaluated"
                | "rule_matched"
                | "route_selected"
                | "route_order_set"
                | "forbid_added"
                | "allow_added"
                | "elicitation_requested"
                | "after_success_scheduled"
                | "outcome_recorded" ;
```

Trace declares which decision events should be persisted by an evaluator or
harness. A rule causes a decision; the evaluator writes the event. The spec
does not contain per-rule file writing instructions.

If `record` is empty or absent, an evaluator may record every v0 event kind.
Event payload fields such as `route_selected.basis`, `spec_fingerprint`, and
`input_sha256` are defined by the trace envelope and event schema, not by the
`skill.spec.yml` grammar.
If `required` is true, a conforming harness should either write the trace or
state that tracing is unavailable before relying on the decision.

## Dependencies

```text
dependencies    = "dependencies" ":" mapping-of dependency-id to dependency ;
dependency      = "kind" ":" dependency-kind ,
                  [ "description" ":" string ] ,
                  [ "command" ":" string ] ,
                  [ "path" ":" string ] ,
                  [ "env" ":" string ] ,
                  [ "check" ":" dependency-check ] ,
                  [ "permission" ":" dependency-permission ] ,
                  [ "provision" ":" dependency-provision ] ;

dependency-kind = "cli"
                | "package"
                | "file"
                | "env"
                | "service"
                | "adapter"
                | "browser" ;

dependency-check
                = [ "command" ":" string ] ,
                  [ "path" ":" string ] ,
                  [ "env" ":" string ] ;

dependency-permission
                = [ "required" ":" boolean ] ,
                  [ "reason" ":" string ] ,
                  [ "safety" ":" safety-class ] ;

dependency-provision
                = [ "elicit" ":" elicitation-id ] ,
                  [ "options" ":" sequence-of dependency-provision-option ] ;

dependency-provision-option
                = "id" ":" identifier ,
                  "label" ":" string ,
                  [ "description" ":" string ] ,
                  [ "command" ":" string ] ,
                  [ "safety" ":" safety-class ] ;
```

Dependencies declare tools, files, env vars, services, adapters, browsers, or
packages needed by commands or routes. They do not grant permission to install
or execute anything. `check` describes how a harness can determine presence.
`permission` describes whether use needs approval. `provision` describes
install or connection options that must be selected through elicitation before
mutation.

The reference CLI can directly check `cli`, `file`, and `env` dependencies.
`package`, `service`, `adapter`, and `browser` dependencies are
harness-specific and should be reported as requiring harness checks.

## Imports

```text
imports         = "imports" ":" mapping-of import-id to import ;
import          = "path" ":" string ,
                  "role" ":" import-role ,
                  [ "description" ":" string ] ,
                  [ "section" ":" string ] ,
                  [ "load" ":" import-load ] ,
                  [ "requires" ":" import-requires ] ,
                  [ "used_by" ":" sequence-of import-use ] ,
                  [ "load_when" ":" sequence-of string ] ;

import-role     = "policy"
                | "reference"
                | "procedure"
                | "example"
                | "skill" ;

import-load     = "always"
                | "on_demand" ;

import-requires = [ "imports" ":" sequence-of import-id ] ;

import-use      = "kind" ":" import-use-kind ,
                  "id" ":" identifier ;

import-use-kind = "route"
                | "rule"
                | "state"
                | "elicitation"
                | "dependency"
                | "command"
                | "code"
                | "artifact"
                | "recipe"
                | "snippet" ;
```

Imports are runtime-loadable instruction material. Use them for shared policy,
branch-specific references, procedures, examples, and other Markdown that a
harness should load deliberately during a run.

An import is not inheritance and does not merge another SkillSpec into the
current document. Importing Markdown never creates routes, rules, commands, or
tests implicitly. The current `skill.spec.yml` remains the only grammar tree.

`path` resolves relative to the directory containing the current
`skill.spec.yml`. Resolution is lexical: do not expand `~`, environment
variables, command substitutions, or Markdown links. Relative paths such as
`../INDEX.md` are allowed so a skill can import plugin-level shared guidance;
harnesses may still apply package-root or sandbox policy before reading the
file. Absolute paths and URLs are harness-specific and should be treated as
local policy decisions, not portable v0 assumptions.

`skillspec imports check <skill.spec.yml>` validates declared import paths,
Markdown sections, explicit nesting, and dependency-first load order. The
command checks static resolution only; it does not load import bodies into an
agent context.

`section` narrows a Markdown import to the named heading and its child content
until the next heading at the same or higher level. If `section` is absent, the
whole file is the import body. If a harness cannot find the section, it should
report a missing import target rather than silently loading unrelated material.

`load` defaults to `on_demand`. `always` imports are loaded after the spec is
loaded and before task actions; they can stand alone without a `used_by`
reference. `on_demand` imports must be connected to the runtime graph by
`used_by`, `requires.imports`, `Code.requires.imports`, `Recipe.requires.imports`,
or a `load_import` recipe step.

Nested imports are explicit. `requires.imports` forms a directed acyclic graph
of import ids; it is not discovered by scanning Markdown links. When loading an
import, load its required imports first in topological order, then the import
itself. Sibling ordering follows the sequence order in `requires.imports`.
Cycles are invalid.

Markdown links inside an imported file remain ordinary prose links. A harness
may let a human or agent follow them, resolving those links relative to the
imported file's path, but following them is outside SkillSpec import semantics
unless the target is declared as its own import.

## Resources

```text
resources       = "resources" ":" mapping-of resource-id to resource ;
resource        = "path" ":" string ,
                  "role" ":" resource-role ,
                  [ "description" ":" string ] ,
                  [ "used_by" ":" sequence-of resource-use ] ,
                  [ "load_when" ":" sequence-of string ] ;

resource-role   = "source_material"
                | "reference"
                | "required_procedure"
                | "example"
                | "script"
                | "asset" ;

resource-use    = "kind" ":" resource-use-kind ,
                  "id" ":" identifier ;

resource-use-kind
                = "route"
                | "rule"
                | "state"
                | "elicitation"
                | "dependency"
                | "command"
                | "code"
                | "artifact"
                | "recipe"
                | "snippet" ;
```

Resources are provenance and supporting source material. They exist so an
imported multi-file skill can preserve original evidence and non-runtime assets
without turning every file into runtime guidance. Runtime-loadable Markdown
belongs in `imports`; provenance, examples, scripts, assets, and source evidence
belong in `resources`. Runtime behavior belongs in routes, rules, states,
commands, code, artifacts, and recipes. A resource with no incoming reference
and no `used_by` declaration is invalid because it has no stated role.

## Code

```text
code            = "code" ":" mapping-of code-id to code-block ;
code-block      = "language" ":" string ,
                  "kind" ":" code-kind ,
                  "source" ":" code-source ,
                  [ "provenance" ":" code-provenance ] ,
                  [ "purpose" ":" string ] ,
                  [ "requires" ":" code-requires ] ,
                  [ "inputs" ":" sequence-of artifact-id ] ,
                  [ "outputs" ":" sequence-of artifact-id ] ,
                  [ "safety" ":" code-safety ] ,
                  [ "use_when" ":" sequence-of string ] ;

code-kind       = "example"
                | "runnable_script"
                | "probe"
                | "transform"
                | "validator"
                | "troubleshooting"
                | "reference" ;

code-source     = "inline" ":" string
                | "file" ":" string ,
                  [ "from_resource" ":" resource-id ] ,
                  [ "fence_index" ":" number ] ,
                  [ "heading" ":" string ] ,
                  [ "sha256" ":" string ] ;

code-provenance = ( "resource" ":" resource-id
                  | "import" ":" import-id ) ,
                  [ "fence_index" ":" number ] ,
                  [ "heading" ":" string ] ,
                  [ "line_start" ":" number ] ,
                  [ "line_end" ":" number ] ;

code-requires   = [ "dependencies" ":" sequence-of dependency-id ] ,
                  [ "imports" ":" sequence-of import-id ] ,
                  [ "resources" ":" sequence-of resource-id ] ,
                  [ "artifacts" ":" sequence-of artifact-id ] ;

code-safety     = [ "mutates_input" ":" boolean ] ,
                  [ "writes_files" ":" boolean ] ,
                  [ "network" ":" boolean ] ,
                  [ "notes" ":" sequence-of string ] ;
```

Code blocks preserve executable knowledge. Importers should preserve snippets
first and only promote them to runnable recipes after review. A fenced block
from a prose skill is not automatically safe to run merely because it is
represented here.

## Artifacts

```text
artifacts       = "artifacts" ":" mapping-of artifact-id to artifact ;
artifact        = "kind" ":" artifact-kind ,
                  [ "description" ":" string ] ,
                  [ "path" ":" string ] ,
                  [ "schema" ":" value ] ,
                  [ "produced_by" ":" sequence-of executable-ref ] ,
                  [ "consumed_by" ":" sequence-of executable-ref ] ;

artifact-kind   = "file"
                | "directory"
                | "json"
                | "text"
                | "image"
                | "pdf"
                | "transcript"
                | "report" ;

executable-ref  = "kind" ":" executable-ref-kind ,
                  "id" ":" identifier ;

executable-ref-kind
                = "command"
                | "code"
                | "recipe" ;
```

Artifacts name the files or data products that code, commands, and recipes
consume or produce. They make dataflow explicit without requiring a workflow
engine.

## Recipes

```text
recipes         = "recipes" ":" mapping-of recipe-id to recipe ;
recipe          = [ "description" ":" string ] ,
                  [ "ordered" ":" boolean ] ,
                  [ "requires" ":" recipe-requires ] ,
                  [ "steps" ":" sequence-of recipe-step ] ;

recipe-requires = [ "imports" ":" sequence-of import-id ] ,
                  [ "resources" ":" sequence-of resource-id ] ,
                  [ "dependencies" ":" sequence-of dependency-id ] ,
                  [ "artifacts" ":" sequence-of artifact-id ] ;

recipe-step     = "load_import" ":" import-id
                | "load_resource" ":" resource-id
                | "run_command" ":" command-id
                | "run_code" ":" code-id
                | "produce_artifact" ":" artifact-id
                | "consume_artifact" ":" artifact-id
                | "ask" ":" elicitation-id
                | "branch" ":" recipe-branch
                | "note" ":" string ;

recipe-branch   = "if" ":" string ,
                  "then" ":" identifier ,
                  [ "otherwise" ":" identifier ] ;
```

Recipes are structured procedures. They are especially useful for imported
skills where a support file says "do these steps in order" or "probe first,
then branch." The reference CLI validates recipe references, but it does not
execute recipes.

## States

```text
states          = "states" ":" mapping-of state-id to state ;
state           = [ "do" ":" sequence-of command-id-or-action-id ] ,
                  [ "say" ":" snippet-id ] ,
                  [ "ask" ":" elicitation-id ] ,
                  [ "next" ":" state-id ] ,
                  [ "yes" ":" state-id ] ,
                  [ "no" ":" state-id ] ;
```

States are named lifecycle positions. V0 does not execute states; it makes the
state machine visible for agents, compilers, reviewers, and tests.

## Commands

```text
commands        = "commands" ":" mapping-of command-id to command ;
command         = [ "description" ":" string ] ,
                  "template" ":" string ,
                  [ "safety" ":" safety-class ] ,
                  [ "requires" ":" command-requires ] ,
                  [ "parse" ":" mapping-of identifier to string ] ,
                  [ "success_when" ":" mapping ] ;

command-requires
                = [ "dependencies" ":" sequence-of dependency-id ] ,
                  [ "resources" ":" sequence-of resource-id ] ,
                  [ "files" ":" sequence-of string ] ,
                  [ "env" ":" sequence-of string ] ,
                  [ "auth" ":" sequence-of string ] ,
                  [ mapping ] ;

safety-class    = "read_only"
                | "local_read"
                | "local_write"
                | "network_read"
                | "network_write"
                | "browser_attach"
                | "credential_request"
                | "destructive" ;
```

Command templates are instructions, not implicit permission to execute. Harnesses
must still apply their own safety and approval policy.

## Snippets

```text
snippets        = "snippets" ":" mapping-of snippet-id to snippet ;
snippet         = "text" ":" string ;
```

Snippets carry prose that should remain prose: questions, user-facing copy,
completion prompts, or short explanations.

## Closures

```text
closures        = "closures" ":" mapping ;
```

Closures describe post-task behavior such as summarizing cost, writing digest,
asking to remember, and asking to share. V0 keeps this open because closure
names are often product-specific.

## Tests

```text
tests           = "tests" ":" sequence-of scenario-test ;
scenario-test   = "name" ":" string ,
                  "input" ":" string ,
                  "expect" ":" expectation ;

expectation     = [ "route" ":" route-id ] ,
                  [ "route_order" ":" sequence-of route-id ] ,
                  [ "forbid" ":" sequence-of identifier ] ,
                  [ "forbid_exact" ":" sequence-of identifier ] ,
                  [ "not_forbid" ":" sequence-of identifier ] ,
                  [ "elicit" ":" sequence-of elicitation-id ] ,
                  [ "elicit_exact" ":" sequence-of elicitation-id ] ,
                  [ "not_elicit" ":" sequence-of elicitation-id ] ,
                  [ "after_success" ":" sequence-of command-id-or-closure-id ] ,
                  [ "after_success_exact" ":" sequence-of command-id-or-closure-id ] ,
                  [ "not_after_success" ":" sequence-of command-id-or-closure-id ] ,
                  [ "matched_rules" ":" sequence-of rule-id ] ,
                  [ "matched_rules_exact" ":" sequence-of rule-id ] ,
                  [ "not_matched_rules" ":" sequence-of rule-id ] ;
```

Tests are the proof mechanism. Every meaningful route rule should have at least
one scenario. An expectation must contain at least one assertion. Plain list
expectations assert inclusion; `*_exact` expectations assert the exact set; and
`not_*` expectations assert absence.

## Proof

```text
proof           = "proof" ":" mapping ;
proof.metrics   = "metrics" ":" sequence-of metric-id ;
```

Proof metrics describe what the policy is trying to improve: steering accuracy,
token reduction, failed branches avoided, saved route savings, and similar
measures.
