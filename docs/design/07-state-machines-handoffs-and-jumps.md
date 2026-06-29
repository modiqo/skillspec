# State Machines, Handoffs, And Jumps

SkillSpec has lifecycle and phase primitives, but v0 is not a workflow engine.

The design goal is visibility. The spec should make lifecycle position, skill
handoff boundaries, and conditional jump points explicit enough for agents,
harnesses, compilers, reviewers, tests, traces, and alignment reports to reason
about them.

The CLI does not execute states or route phases by itself.

## Three Related Structures

SkillSpec currently has three related but distinct structures:

- `states`: named lifecycle positions.
- route `handoff`: a boundary where another skill owns work.
- route `execution_plan`: ordered phases, optional phase handoffs, and jump
  points.

They answer different questions.

`routes` answer: which strategy should satisfy this task?

`states` answer: where are we in the lifecycle of satisfying the task?

`handoff` answers: which skill owns the next boundary of work?

`execution_plan` answers: which phases must be inspected or followed for the
selected route?

`jumps` answer: when can execution move from one declared phase to another?

Keeping these separate prevents states from becoming routes, routes from
becoming a workflow runtime, and handoffs from becoming implicit permission to
use another skill.

## States

`states` is a mapping from state id to state definition.

The current state model includes:

- `do`: a list of action ids;
- `say`: optional human-facing text;
- `ask`: optional elicitation id;
- `next`: optional next state id;
- `yes`: optional state id for a positive branch;
- `no`: optional state id for a negative branch.

The parser validates:

- each state key as an identifier;
- `states.do` references against known action ids;
- `states.ask` against known elicitations;
- `states.next`, `states.yes`, and `states.no` against known states.

The formal grammar describes states as named lifecycle positions. It also says
v0 does not execute states; it makes the state machine visible for agents,
compilers, reviewers, and tests.

## State Associations

States connect to several other primitives:

- A state can run declared actions through `do`.
- A state can ask a bounded question through `ask`.
- An elicitation choice can transition to a state through `next`.
- A state can continue to another state through `next`.
- A state can branch through `yes` and `no`.
- Commands, recipes, code blocks, and closures can be referenced as actions.

Those links make lifecycle review possible. A reviewer can ask whether every
state transition points to a known state, whether the question is bounded, and
whether a declared action has dependencies and safety information.

## Route Handoffs

A route can declare a `handoff`.

The current route handoff model includes:

- `to_skill`;
- `boundary`;
- `pass_context`;
- `forbid`;
- `reason`.

`boundary` is an enum with:

- `stop_current_skill`;
- `resume_after_handoff`.

A route-level handoff is a hard boundary in the active contract. It tells the
agent and harness which skill should own the next work and what context should
cross that boundary.

The `forbid` list inside a handoff is boundary-specific. It describes behavior
that should remain blocked while crossing or respecting that handoff. For
example, a shell-oriented skill can hand browser work to a browser skill while
forbidding direct browser tools inside the shell skill.

## Execution Plans

A route can also declare an `execution_plan`.

The current execution plan model includes:

- `mode`;
- `phases`;
- `reason`.

The only current execution-plan mode is `ordered`.

Each phase can declare:

- `id`;
- `owner_skill`;
- optional `route`;
- optional `description`;
- `requires`;
- `checks`;
- `forbid`;
- optional phase `handoff`;
- `jumps`.

The decision engine attaches the selected route's execution plan to the decision
result. It does not run the plan. The generated loader tells the agent to inspect
the selected route's plan and materialize an active checklist before using tools.

## Jumps

A phase can declare `jumps`.

The current jump model includes:

- `when`;
- `to_phase`;
- optional `reason`.

A jump is a declared conditional transition between execution phases. It is a
reviewable contract element, not executable code. The `when` string describes
the condition, and the harness or agent decides whether observed evidence
satisfies that condition.

Scenario tests can assert `plan_jumps`. The decision test runner represents a
jump as:

```text
phase_id:when->to_phase
```

That assertion proves the selected route surfaced the expected jump declaration.
It does not prove that a runtime harness took the jump correctly.

## Validation Boundary

State references are validated today. The parser checks state ids, state action
references, state elicitation references, and state transition targets.

Execution plans are typed by the model and exposed through decisions, queries,
refs, and tests. Current validation is not a full graph verifier for phase ids,
phase route references, handoff targets, or jump targets. Authors and reviewers
should treat those as contract-sensitive fields and test representative routes.

This is an important v0 boundary. The structure exists, but harness execution
and some graph-level assurances are outside the current parser.

## Sensemaking And Refs

`skillspec query` and `skillspec refs` make these structures navigable.

Useful commands include:

```sh
skillspec query skill.spec.yml state:<id> --view summary
skillspec refs skill.spec.yml state:<id> --view summary
skillspec query skill.spec.yml route:<id>.execution_plan
skillspec refs skill.spec.yml route:<id> --view summary
```

State refs expose `next`, `yes`, and `no`.

Route refs can expose:

- route `checks`;
- `handoff.to_skill`;
- `execution_plan.owner_skill`;
- `execution_plan.route`;
- `execution_plan.handoff.to_skill`;
- `execution_plan.jump.to_phase`.

This lets the agent inspect phase ownership and jump targets without reading the
whole spec.

## Traces And Alignment

Decision traces record route selection and outcome. When the selected route has
an execution plan, the decision output includes that plan.

The decision trace does not prove that a harness executed each phase, respected a
handoff, or took a jump correctly. That proof requires execution evidence from
the harness. Alignment can replay deterministic decision facts and report
execution obligations as unproven when evidence is missing.

## Design Guidance

Use states for lifecycle clarity, not for general-purpose automation.

Use route-level handoff when another skill owns a whole strategy boundary.

Use phase-level handoff when a route has ordered phases and only one phase needs
another skill.

Use jumps sparingly. A jump should name a clear condition and a declared target
phase. If a condition needs complex computation, put that logic in harness code
or a command and preserve the evidence.

Do not use a handoff to bypass the current route's forbids. Handoff context and
forbids are part of the active contract.

Do not report a phase or jump as completed unless execution evidence supports
that claim.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-core/src/spec/model.rs`, which defines `State`, `RouteHandoff`,
  `HandoffBoundary`, `ExecutionPlan`, `ExecutionPhase`, and `ExecutionJump`;
- `crates/skillspec-core/src/spec/parser/validation.rs`, which validates state references and
  route ids;
- `crates/skillspec-cli/src/execution/decision.rs`, which attaches the selected route's
  execution plan to the decision;
- `crates/skillspec-cli/src/features/sensemake.rs`, which exposes state refs and route
  execution-plan refs;
- `spec/semantics.md`, which states that v0 state transitions are descriptive
  and not a general-purpose workflow runtime;
- `spec/relationships.md`, which explains route/state separation and lifecycle
  relationships;
- `spec/grammar.md`, which describes the state grammar and the v0 state
  execution boundary.
