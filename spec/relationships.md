# Associative Relationships

SkillSpec is small, but the concepts are intentionally connected. This document
defines the relationships that make the format useful rather than just a pile
of YAML sections.

## Concept Graph

```text
SkillSpec
  has one Header
  may have Entry
  has many Routes
  has many Rules
  has many States
  has many Elicitations
  has many Commands
  has many Snippets
  has many Tests
  may have Proof

Rule
  matches Predicate
  may prefer Route
  may replace RouteOrder
  may forbid Substitution
  may allow NarrowFallback
  may request Elicitation
  may schedule AfterSuccess closure/action

Elicitation
  asks bounded Question
  has many Choices
  may be required by Route or MissingFact
  may set Fact
  may steer Route
  may transition to State

State
  may say Snippet
  may ask Elicitation
  may do Command or Action
  may transition to State
  may branch to State by user answer

Command
  may require Tool/File/Env
  has SafetyClass
  may parse Output
  may prove State or Closure

Test
  provides Input
  expects Route, RouteOrder, Forbid, Elicit, AfterSuccess
  proves Rule behavior

Proof
  summarizes aggregate Test and runtime evidence
```

## Where Rules Can Be Used

Rules are not only route selectors. A rule can influence five parts of behavior:

1. **Route choice**
   `prefer: browser`

2. **Route ordering**
   `route_order: [remembered_route, connected_service, local_cli, browser]`

3. **Negative steering**
   `forbid: [native_search_as_answer, raw_playwright]`

4. **Post-task closure**
   `after_success: [collect_trace_cost, ask_to_remember]`

5. **Bounded elicitation**
   `elicit: [browser_mode]`

The important design decision: rules can shape both **what happens next** and
**what must not happen instead**. That is how a SkillSpec prevents prose drift.

## Rule Placement

Rules live at the document level in v0. That is intentional. A rule describes
how user intent, risk, recurrence, or environment facts should steer the whole
skill, not just one command.

Use a document-level rule when it answers one of these questions:

- Which route should satisfy this task?
- Which route order should be tried before asking the user?
- Which plausible substitutes are forbidden?
- Which narrow fallbacks are allowed?
- Which bounded question must be asked before guessing?
- Which post-success actions are mandatory?

Do not hide routing rules inside command descriptions or snippets. Commands
describe actions. Snippets describe words. Rules describe decisions.

Future versions may add scoped rules on routes, states, or commands. V0 keeps
rules flat so they are easy to test, compile, and audit.

## Primitive Association Matrix

The primitives are deliberately small, but they are not isolated. Their
associations define the useful behavior.

| Primitive | Owns | Points To | Is Proven By |
| --- | --- | --- | --- |
| `Route` | A strategy for satisfying intent | `Command.checks`, `State` lifecycle, `Rule.prefer` | scenario tests and runtime outcomes |
| `Rule` | A steering decision | `Predicate`, `Route`, forbids, allows, elicitations, closures | scenario tests |
| `Predicate` | A match condition | user wording or inferred task properties | decision traces |
| `Elicitation` | A bounded question | choices, facts, route, next state | scenario tests and user answers |
| `State` | Lifecycle position | commands, snippets, elicitations, next/yes/no states | state graph review and flow replay |
| `Command` | A named action template | required tools/files/env, parse rules, safety class | captured command output |
| `Snippet` | Stable human-facing prose | states or generated skill docs | generated skill output |
| `Closure` | Post-task behavior | commands, digest, memory, hub share | trace/cost evidence |
| `Test` | Steering regression case | rules and expectations | `skillspec test` |
| `Proof` | Aggregate confidence | metrics and evidence sources | reports and CI |

## Causality Shape

The intended mental model is:

```text
user input
  -> matched predicates
  -> matched rules
  -> route choice, route order, and required elicitations
  -> bounded user choice when needed
  -> state lifecycle
  -> command/snippet execution plan
  -> captured evidence
  -> closure actions
  -> proof metrics
```

This matters because a SkillSpec should not merely tell an agent "be careful."
It should make the steering chain inspectable:

```text
"browse recent committers social profiles"
  -> user_says_any("social profile")
  -> browse_profiles_uses_browser
  -> route = browser
  -> forbid native_search_as_answer
  -> elicit browser_mode
  -> collect browser evidence
```

The association is also how we compare alternatives. If two routes can satisfy
the same task, rules explain why one route wins and tests make that choice
repeatable.

## Route And State Relationship

Routes answer:

```text
Which way should the agent satisfy this task?
```

States answer:

```text
Where are we in the lifecycle of satisfying this task?
```

Example:

```text
route = browser
state = execute
```

The route says use browser observation. The state says we are currently running
the chosen route. Keeping these separate avoids turning the state machine into
a giant set of route-specific branches.

## Elicitation Relationship

Elicitations answer:

```text
What bounded choice must the user make before the agent can continue safely?
```

Rules can request an elicitation:

```yaml
rules:
  - id: browse_words_handoff_to_browse
    when:
      user_says_any: [browse]
    prefer: browser
    elicit: [browser_mode]
```

States can ask an elicitation:

```yaml
states:
  choose_browser_mode:
    ask: browser_mode
    next: execute
```

The elicitation owns the actual question and choices:

```yaml
elicitations:
  browser_mode:
    question: How should I access the browser state?
    choices:
      - id: attach_existing
        label: Attach to active browser
        sets:
          browser_mode: attach_existing
```

This keeps "ask the user" from becoming an open-ended prompt. A good
elicitation is small, mutually understandable, and tied to facts the skill
needs before it can route or execute safely.

Use elicitations for:

- browser attach/headed/headless choices
- install/provisioning scope
- destructive release approval
- auth/session selection
- ambiguous target selection when the alternatives are known

Do not use elicitations for broad discovery questions. If the agent does not
yet know the alternatives, it should inspect safely first, then ask a bounded
elicitation.

## Command And State Relationship

Commands are reusable actions. States reference them by name:

```yaml
states:
  complete:
    do:
      - summarize_result
      - collect_trace_cost
```

```yaml
commands:
  collect_trace_cost:
    template: "rote trace --deps --format json"
```

This keeps command invocation out of prose and makes it inspectable by agents
and tests.

## Snippet And State Relationship

Snippets carry words that should remain words:

```yaml
states:
  ask_task:
    say: task_entry
```

```yaml
snippets:
  task_entry:
    text: "What would you like to do today?"
```

This lets generated `SKILL.md` stay small without losing the product voice.

## Test And Rule Relationship

Every rule should have a test. If a rule exists because a harness once made a
bad choice, the test should encode that regression.

Example:

```yaml
rules:
  - id: browse_means_browser
    when:
      user_says_any: [browse]
    prefer: browser
    forbid: [native_search_as_answer]
```

```yaml
tests:
  - name: browse calendar routes to browser
    input: browse my calendar
    expect:
      route: browser
      forbid: [native_search_as_answer]
```

This is the move from prose hope to steering proof.

## Command Safety Relationship

`Command.safety` is not an execution permission. It is a declaration that lets a
harness decide how much approval, isolation, or preflight is required.

Examples:

- `read_only`: safe to run automatically when dependencies exist.
- `local_read`: reads local workspace state; should stay inside the chosen
  working directory unless the user asked otherwise.
- `network_read`: may use authenticated network state; should preserve
  response provenance.
- `browser_attach`: depends on live user session state; should ask before
  attaching or opening a browser.
- `credential_request`: must stay visible and bounded.
- `destructive`: requires explicit user approval.

Rules can route around unsafe commands. Commands should not route around rules.

## Closure Relationship

Closures are how a skill finishes work without losing the value it just
created. Typical closures include:

- collect trace cost
- estimate recurrence savings
- ask whether to remember the route
- write the session digest
- ask whether to push/share with a team

Closures are referenced from `after_success` so they remain tied to the rule
that made them necessary. For example, a recurrence rule can require
`collect_trace_cost` and `ask_to_remember` without forcing those steps onto
every one-off task.

## Proof And Runtime Relationship

Tests prove steering before runtime. Runtime evidence proves value after use.

Proof can combine:

- scenario pass rate
- route decision accuracy
- prose tokens reduced
- failed branches avoided
- remembered-route token savings
- completion cost projections

SkillSpec should eventually support reports such as:

```text
42/44 scenarios passed
skill prose reduced by 82%
3 known drift bugs covered by tests
estimated 18,400 tokens avoided per remembered run
```

## Future Inheritance Relationship

V0 has no imports or inheritance. The future model should be:

```text
base skill
  -> domain skill
    -> team skill
      -> project skill
        -> user override
```

Safety weakening should require explicit review. Forbid rules should be sticky
by default.
