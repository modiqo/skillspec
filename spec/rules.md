# Rule Semantics

Rules are the steering layer. They express small, testable decisions that prose
skills often bury in paragraphs.

## Evaluation Order

V0 evaluates rules in file order. For each matching rule:

- `prefer` sets the selected route
- `route_order` replaces the current route order
- `forbid` entries are appended
- `allow` entries are merged
- `after_success` entries are appended
- `reason` is attached to the decision explanation

After all matches, duplicate `forbid` and `after_success` entries are removed.

## Scope

V0 rules are global to the SkillSpec document. They are evaluated once against
the user input or inferred task facts, then their effects produce a decision.

Rules should be used for:

- substrate choice: remembered route, adapter, local CLI, browser, human handoff
- route order: what to try first, second, third
- risk steering: avoid hidden prompts, avoid native search as answer, avoid raw
  shell when provenance matters
- lifecycle obligations: collect trace cost, ask to remember, ask to share
- approval hints: route destructive work through visible confirmation

Rules should not be used for:

- long prose instructions
- command implementation details
- user-facing copy
- parse expressions
- environment secrets

Those belong in snippets, commands, parse fields, or external runtime policy.
The test is simple: if changing the item should change a steering decision, it
is probably a rule. If changing it only changes wording or command syntax, it
probably is not.

## Match Predicates

V0 standardizes these predicates:

- `user_says_any`
- `task_recurrence_likely`
- `domain_object_task`
- `interactive_prompt_likely`
- `command_likely_long_running`

Implementations may add `x-*` extension predicates later, but v0 examples
should stay inside the standard set.

When a rule specifies more than one predicate field, all specified fields must
match. Inside `user_says_any`, any listed phrase may match.

Example:

```yaml
when:
  command_likely_long_running: true
  interactive_prompt_likely: false
```

This means "long-running and not likely to prompt interactively." It does not
mean either condition is enough by itself.

## Negative Steering

Negative steering is a first-class feature.

Example:

```yaml
forbid:
  - native_search_as_answer
  - raw_playwright
  - curl
```

This is important because agent drift often happens when the agent chooses a
plausible substitute that violates the user's intent. A SkillSpec should not
only say what to do; it should say what not to substitute.

## Rule Composition

Multiple rules may match the same input. This is expected.

Example:

```yaml
rules:
  - id: local_repo_state_uses_cli
    when:
      user_says_any: [branch, in sync]
    prefer: local_cli

  - id: browse_profiles_uses_browser
    when:
      user_says_any: [social profile]
    prefer: browser
    forbid: [native_search_as_answer]
```

For the input:

```text
check whether the repo is in sync and browse each committer social profile
```

both rules match. The later browser rule can select the browser route while the
matched-rules trace still records that local repo state was also recognized.
This lets the harness split work: collect repo facts with CLI/API, then satisfy
the profile browsing part with browser evidence.

When composition becomes ambiguous, add a scenario test. Do not rely on prose
ordering alone.

## Narrow Allows

`allow` weakens a forbid only in a narrow way:

```yaml
allow:
  native_search: url_discovery_only
```

This says native search may help find a URL, but may not become the answer
substrate. This distinction is exactly the kind of thing prose skills lose.

## Test Obligation

Every route-changing or forbid-heavy rule should have at least one scenario
test. A rule without a test is only structured prose.

## Extension Discipline

Unknown predicates should use an `x-` prefix in future extensions. V0 tooling
should not silently treat unknown standard-looking predicates as true. A route
file should fail validation once schema validation is strict enough.

Keep predicates coarse and portable. Prefer:

```yaml
command_likely_long_running: true
```

over:

```yaml
cargo_test_takes_more_than_90_seconds: true
```

Specificity belongs in tests and examples. Predicates should stay reusable
across harnesses.
