# Formal Grammar

This document defines the v0 grammar in a compact EBNF-like notation. The
canonical interchange format is YAML, but the grammar describes the conceptual
tree independent of YAML spelling.

V0 is deliberately not a programming language. It is a structured skill
contract: enough grammar to express routes, rules, elicitations, states,
commands, snippets, closures, tests, and proof hooks without becoming an
execution engine.

## Lexical Conventions

```text
identifier      = lowercase-letter , { lowercase-letter | digit | "_" | "-" | "." } ;
route-id        = identifier ;
rule-id         = identifier ;
state-id        = identifier ;
elicitation-id = identifier ;
command-id      = identifier ;
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
                  [ "requires" ":" mapping ] ,
                  [ "parse" ":" mapping-of identifier to string ] ,
                  [ "success_when" ":" mapping ] ;

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
