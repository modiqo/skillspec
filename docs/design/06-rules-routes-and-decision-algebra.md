# Rules, Routes, And Decision Algebra

Routes and rules are the center of the SkillSpec decision contract.

A route is a named strategy. A rule is a predicate-driven steering clause. The
decision engine combines them into one decision object that tells the agent what
route is active, what is forbidden, what questions must be asked, what
completion work is scheduled, and which rules matched.

## Core Concepts

`routes` define available strategies. Each route has an `id`, `label`, optional
`rank`, optional `description`, optional `checks`, optional `handoff`, and
optional `execution_plan`.

`rules` define steering behavior. Each rule has an `id`, a `when` predicate, and
zero or more effects:

- `prefer`;
- `route_order`;
- `forbid`;
- `allow`;
- `elicit`;
- `after_success`;
- `reason`.

`decide` evaluates those sections against a user task and returns a `Decision`.

The decision output includes:

- original `input`;
- selected `route`;
- `route_selection` with basis, rule id, and reason;
- `route_order`;
- selected route `execution_plan`;
- `forbid`;
- `allow`;
- `elicit`;
- `after_success`;
- `matched_rules`;
- `reason`.

## Route Order

Before rules run, the engine builds a default route order from `routes`.

Routes are sorted by `rank`. A missing rank sorts as `i64::MAX`, so explicitly
ranked routes come before unranked routes. The route order contains route ids,
not route objects.

Rules can replace the route order with `route_order`. Replacement is deliberate:
a matching rule's route order is not appended to the existing order. If multiple
matching rules set `route_order`, later matching rules overwrite earlier route
orders because rules are applied in file order.

If no rule selects a route with `prefer`, the engine selects the first route in
the final route order.

## Route Selection Basis

The decision records why a route was selected. The basis can be:

- `rule_prefer`: a matched rule selected its `prefer` route.
- `route_order_default`: a matched rule changed route order, and the first route
  in that route order won because no rule selected a route directly.
- `default_route_order`: no rule selected or reordered routes, so the first
  ranked/default route won.

This basis is important for trace alignment. A route id alone does not explain
whether the decision came from an explicit rule or from fallback order.

## Predicate Semantics

Predicates are fixed, typed task signals. The current model includes:

- `user_says_any`;
- `user_says_all_groups`;
- `task_recurrence_likely`;
- `domain_object_task`;
- `interactive_prompt_likely`;
- `command_likely_long_running`.

Predicate fields compose as AND across fields. A rule matches only if every
present predicate field matches.

Inside `user_says_any`, values compose as OR. At least one phrase must appear in
the normalized input.

Inside `user_says_all_groups`, each group must match, and a group matches when
at least one phrase in that group appears.

Boolean predicate fields compare the engine's heuristic result against the
declared expected boolean.

A rule with no predicate conditions does not match. The implementation tracks
whether any condition was present and returns false when a predicate is empty.

## Rule Application Order

Rules are evaluated in file order. Every matching rule is applied.

This gives SkillSpec additive behavior for most obligations and overwrite
behavior for route selection:

- `prefer` sets the selected route and records `rule_prefer` as the selection
  basis. If a later matching rule also has `prefer`, the later rule overwrites
  the selected route.
- `route_order` replaces the current route order. If a later matching rule also
  has `route_order`, the later route order wins.
- `forbid` values are appended, then deduplicated after all rules run.
- `elicit` values are appended, then deduplicated after all rules run.
- `after_success` values are appended, then deduplicated after all rules run.
- `allow` maps are extended; duplicate keys follow map overwrite behavior from
  later matching rules.
- `reason` is set from the matching rule's reason when present; otherwise the
  previous reason is retained.
- every matching rule is recorded in `matched_rules`.

The string dedupe pass preserves first occurrence order by retaining the first
time each string appears.

## Decision Events

The decision engine emits events for trace writers:

- `input_received`;
- `spec_loaded`;
- `rule_evaluated`;
- `rule_matched`;
- `route_selected`;
- `route_order_set`;
- `forbid_added`;
- `allow_added`;
- `elicitation_requested`;
- `after_success_scheduled`;
- `outcome_recorded`.

These events are decision evidence. They do not include tool payloads, command
stdout, browser snapshots, or service responses.

## Forbid And Allow

`forbid` is a hard negative steering signal. It declares behavior the agent
should not substitute in for the active route. For example, a rule may forbid
native web search when the route requires browser evidence, or forbid direct
shell when the route requires a captured rote process.

`allow` is narrower. It is a structured map for explicit fallback or exception
data. The decision engine records it and merges maps from matching rules, but
the meaning of particular `allow` keys belongs to the skill and harness.

The presence of `allow` does not erase `forbid`. A harness or loader should
interpret both through the active contract and user instructions.

## Elicitations

`elicit` schedules bounded questions by id. The rule does not contain the whole
question; it references an item in `elicitations`.

This keeps question content reusable and typed. The parser validates that
`rules.elicit` names known elicitation ids.

An elicitation can also steer the contract through choices that set values,
route to another route, or transition to a state. The decision engine's rule
output only says which elicitations are requested; the harness handles the
interaction.

## After-Success Obligations

`after_success` schedules completion work by action id. The parser validates
that `rules.after_success` names a known action id from commands, recipes, code,
or closures.

After-success work is not optional decoration. The loader guidance treats it as
a completion obligation: complete it before the final response, or report why it
remains unproven.

## Execution Plan Attachment

After rules finish and the final route is selected, the decision attaches the
selected route's `execution_plan`, if one exists.

The engine does not execute the plan. It places the plan into the decision so
the agent and harness can inspect ordered phases, owner skills, handoffs,
requires, checks, forbids, and jumps.

## Scenario Tests For Decision Algebra

Scenario tests check the decision algebra. They can assert:

- selected `route`;
- `route_order`;
- `plan_phases`;
- `plan_jumps`;
- included, exact, or absent `forbid` values;
- included, exact, or absent `elicit` values;
- included, exact, or absent `after_success` values;
- included, exact, or absent `matched_rules`.

Tests are the fastest way to make rule behavior reviewable. A rule that changes
route selection or safety-critical obligations should have a scenario test.

## Design Guidance

Prefer small rules with clear predicates. If one rule tries to handle unrelated
task shapes, it becomes hard to test and hard to explain in traces.

Use `prefer` when a signal directly selects a strategy.

Use `route_order` when a signal changes preference order but does not make every
other route invalid.

Use `forbid` for substitutions that must not happen silently.

Use `elicit` when the task cannot proceed safely without a bounded answer.

Use `after_success` for completion work that is easy to forget, such as
alignment reporting, cleanup, evidence preservation, or final verification.

Keep route ids and rule ids stable. They are part of tests, trace replay,
alignment reports, refs, and harness behavior.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-core/src/spec/model.rs`, which defines `Route`, `Rule`,
  `Predicate`, and `Expectation`;
- `crates/skillspec-cli/src/execution/decision.rs`, which defines `RouteSelection` and
  `RouteSelectionBasis`, and implements default route order, predicate matching,
  rule application, dedupe, decision events, execution-plan attachment, and
  scenario-test comparison;
- `crates/skillspec-core/src/spec/parser/validation.rs`, which validates route, rule,
  elicitation, action, and test references;
- `spec/semantics.md` and `spec/relationships.md`, which describe the intended
  meaning of routes, rules, predicates, elicitations, tests, and traces.
