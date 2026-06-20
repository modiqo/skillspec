# RFC: SkillSpec v0

Structured behavior contracts for agent skills.

## Status

Draft for community review.

## Summary

SkillSpec turns long prose skills into compact, testable behavior contracts. The prose still teaches tone and context. The `skill.spec.yml` carries the parts agents should not guess: routes, rules, dependencies, code snippets, resources, recipes, elicitations, tests, and traces.

## Motivation

Agent skills are becoming a portable unit of behavior, but most skills still rely on prose instructions alone. That makes critical behavior hard to test, port, audit, or compare across harnesses.

SkillSpec proposes a narrow contract:

- Agent Skills define what to load.
- MCP defines what tools and data are available.
- SkillSpec defines how the agent should decide, verify, and report behavior.

## Non-Goals

- Replace prose.
- Replace MCP.
- Define every possible workflow primitive.
- Automatically install dependencies.
- Claim prompt injection is solved.
- Hide harness-specific permission policy.

## What We Want Feedback On

- Grammar: field names, strictness, missing constructs, and extension surfaces.
- Trace format: required events, compact summaries, and privacy expectations.
- Tests: expectation syntax, negative assertions, and conformance fixtures.
- Dependencies: CLI, file, package, service, browser, adapter, and credential modeling.
- Compiler targets: Codex, Claude Code, Markdown, and other harness loaders.
- Importer behavior: what should be extracted automatically versus marked for review.

## The Ask

Port one real skill and tell us what the spec cannot express.

Please include:

- Link to the original skill.
- The target harness: Codex, Claude Code, or other.
- Decisions that were hard to encode.
- Dependencies that were ambiguous.
- Scenario tests that would prove it works.
- Compiler output that felt too thin, too verbose, or wrong.

## Minimum Compliance Gate

A SkillSpec-backed skill should pass:

```sh
skillspec validate skill.spec.yml
skillspec test skill.spec.yml
skillspec deps check skill.spec.yml
```

For runtime behavior, use traced decisions:

```sh
skillspec decide skill.spec.yml --input='<user task>' --trace-dir .skillspec/traces
skillspec trace compact .skillspec/traces/<run-id>
```

## Example

```yaml
schema: skillspec/v0
id: example.browser_port
title: example browser port
description: Route browser requests to browser automation and forbid search substitution.

routes:
  - id: browser
    label: Browser automation
    rank: 10

rules:
  - id: browsing_requires_browser
    when:
      user_says_any: ["browse", "open", "click", "snapshot"]
    prefer: browser
    forbid: ["native_search_as_answer"]

tests:
  - name: browsing uses browser
    input: browse the docs
    expect:
      route: browser
      forbid: ["native_search_as_answer"]
```

## Compatibility

SkillSpec v0 should be strict by default. Unknown fields in typed sections should fail validation so typos do not become silent behavior gaps. Explicit extension surfaces remain possible, but they should be named and documented.

## Next Milestones

- Expand examples from real skill ports.
- Add conformance fixtures for valid and invalid specs.
- Add more compiler targets.
- Improve importer extraction.
- Publish a trace compatibility guide.
