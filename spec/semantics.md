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

## Imports And Resources

Imports are runtime-loadable instruction material. They are the right place for
shared operating policy, branch-specific reference files, procedure documents,
examples, or another skill document that a harness should deliberately load
during a run.

Resources are provenance and supporting material. They preserve source evidence,
assets, scripts, examples, and other files without making them active guidance.
A file can influence behavior only through structured routes, rules, recipes,
commands, code, states, or an explicit import reference.

Import paths resolve relative to the directory containing `skill.spec.yml`.
Relative paths may point at sibling or parent package files such as
`../INDEX.md`; harnesses still apply their own filesystem and package-root
policy before reading. SkillSpec does not expand shell syntax, environment
variables, or Markdown links in an import path.

`load: always` imports are part of task startup. `load: on_demand` imports are
loaded only when their connected route, rule, recipe, code path, or parent
import is active. Nested imports are explicit through `requires.imports`; the
graph must be acyclic and is loaded in topological order. Markdown links inside
an imported document remain prose links unless their targets are declared as
imports.

If `section` is present, a Markdown loader should read the named heading and its
children, stopping at the next heading at the same or higher level. Missing
sections should fail closed with a clear missing-import error.

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

The reference CLI evaluates tests by running the same rule matcher used by
`skillspec decide`. Positive list expectations assert inclusion, `*_exact`
expectations assert exact sets, and `not_*` expectations assert absence. Empty
expectations are invalid because they cannot prove behavior.
