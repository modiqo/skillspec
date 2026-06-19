# Semantics

## SkillSpec Is A Behavior Contract

SkillSpec describes how an agent should steer work. It does not execute the work
itself. It can point to command templates, user questions, route order, and
state transitions, but the harness or product decides how to run tools.

## Routes

Routes are named ways to satisfy a user task.

Examples:

- remembered route
- connected service
- local CLI
- browser
- local files

Routes can have labels, ranks, checks, and user-facing descriptions. Lower rank
means earlier preference unless a rule overrides it.

## Rules

Rules match task signals and adjust behavior:

- prefer a route
- forbid a substitution
- allow a fallback only for a narrow purpose
- ask a user question
- continue to a state

Rules should be easy to test. Every production bug caused by a misroute should
become a scenario test.

## States

States describe progression:

```yaml
states:
  start:
    do: [readiness_check]
    next: ask_task
```

V0 state transitions are descriptive. They are not a general-purpose workflow
runtime. The primary consumer is an agent or compiler that needs stable
guidance.

## Commands

Commands are templates or invocation instructions. They may include variables,
parse hints, safety class, and expected output. V0 does not mandate a template
engine; implementations should treat command strings as examples unless they
explicitly support rendering.

## Tests

Tests are scenario contracts:

```yaml
tests:
  - name: browse means browser
    input: "browse my calendar"
    expect:
      route: browser
      forbid: [native_search_as_answer]
```

The reference CLI should eventually evaluate tests by running the same rule
matcher used by `skillspec decide`.

