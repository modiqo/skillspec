# Formal Grammar

This document defines the v0 grammar in a compact EBNF-like notation. The
canonical interchange format is YAML, but the grammar describes the conceptual
tree independent of YAML spelling.

V0 is deliberately not a programming language. It is a structured skill
contract: enough grammar to express routes, rules, elicitations, states,
commands, resources, code, artifacts, recipes, snippets, closures, tests, and
proof hooks without becoming an execution engine.

## Lexical Conventions

```text
identifier      = lowercase-letter , { lowercase-letter | digit | "_" | "-" | "." } ;
route-id        = identifier ;
rule-id         = identifier ;
state-id        = identifier ;
elicitation-id = identifier ;
command-id      = identifier ;
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
- every `Dependency.provision.elicit` references an existing elicitation
- every `Resource.used_by` item references the target it names
- every resource is referenced by code or recipe, or declares `used_by`
- every `Code.requires.dependencies` item references an existing dependency
- every `Code.requires.resources` item references an existing resource
- every `Code.requires.artifacts`, `Code.inputs`, and `Code.outputs` item
  references an existing artifact
- every `Code.provenance.resource` and `Code.source.from_resource` references
  an existing resource
- every artifact producer/consumer references an existing command, code block,
  or recipe
- every recipe dependency/resource/artifact/code/command/elicitation reference
  points at an existing id

V0 CLI validation performs these structural and cross-reference checks.

## Document

```text
skillspec       = header ,
                  [ applies-when ] ,
                  [ entry ] ,
                  [ routes ] ,
                  [ rules ] ,
                  [ elicitations ] ,
                  [ trace ] ,
                  [ dependencies ] ,
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
applies-when    = "applies_when" ":" sequence-of activation-hint ;
activation-hint = mapping ;
```

V0 does not standardize all activation predicates. The common shape is:

```yaml
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
imported multi-file skill can preserve its original files without turning
Markdown into the runtime model. Runtime behavior belongs in routes, rules,
states, commands, code, artifacts, and recipes. A resource with no incoming
reference and no `used_by` declaration is invalid because it has no stated role.

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

code-provenance = "resource" ":" resource-id ,
                  [ "fence_index" ":" number ] ,
                  [ "heading" ":" string ] ,
                  [ "line_start" ":" number ] ,
                  [ "line_end" ":" number ] ;

code-requires   = [ "dependencies" ":" sequence-of dependency-id ] ,
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

recipe-requires = [ "resources" ":" sequence-of resource-id ] ,
                  [ "dependencies" ":" sequence-of dependency-id ] ,
                  [ "artifacts" ":" sequence-of artifact-id ] ;

recipe-step     = "load_resource" ":" resource-id
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
                  [ "elicit" ":" sequence-of elicitation-id ] ,
                  [ "after_success" ":" sequence-of command-id-or-closure-id ] ;
```

Tests are the proof mechanism. Every meaningful route rule should have at least
one scenario.

## Proof

```text
proof           = "proof" ":" mapping ;
proof.metrics   = "metrics" ":" sequence-of metric-id ;
```

Proof metrics describe what the policy is trying to improve: steering accuracy,
token reduction, failed branches avoided, saved route savings, and similar
measures.
