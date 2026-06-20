# SkillSpec

Keep the prose. Structure the decisions.

SkillSpec is a structured companion to agent skills. A prose `SKILL.md` is good
for human orientation, tone, and teaching. A `skill.spec.yml` is for the parts
that should be compact, testable, portable, and hard to misread:

- intent routing
- route order
- forbidden substitutions
- bounded user questions and choices
- state transitions
- command templates
- source resources from imported multi-file skills
- code snippets with provenance, dependencies, inputs, outputs, and safety
- named artifacts consumed or produced by code and commands
- ordered recipes for procedural skills
- declared dependencies and provision choices
- user questions
- completion closures
- scenario tests
- decision traces
- proof metrics

SkillSpec is not a workflow engine and not a replacement for skills. It is a
small behavior contract that lets skills get shorter while decisions become
provable.

## Why

Agent skills increasingly become routers and state machines written as prose.
That works until a harness interprets "browse my calendar" as "search the web"
or treats a browser extraction request as a generic lookup. The fix should not
be another paragraph in a thousand-line skill. The fix should be a failing
scenario test and a structured rule.

SkillSpec exists to make that possible.

## Shape

A skill folder can look like this:

```text
my-skill/
  SKILL.md          # short prose orientation
  skill.spec.yml    # structured behavior contract
```

The prose stays useful:

```markdown
# rote-computer

Use this when the user wants to get work done across tools.

Follow `skill.spec.yml` for routing, state progression, guardrails, and
completion behavior.
```

The spec carries the decisions:

```yaml
schema: skillspec/v0
id: rote.computer
title: rote computer

rules:
  - id: browse_means_browser
    when:
      user_says_any: ["browse", "open", "click", "snapshot", "extract from page"]
    prefer: browser
    forbid: ["native_search_as_answer", "raw_playwright", "curl"]

tests:
  - name: browse calendar routes to browser
    input: "browse my calendar"
    expect:
      route: browser
      forbid: ["native_search_as_answer", "adapter_setup_first"]
```

## V0 Scope

V0 is intentionally focused:

- one file, no imports
- one complete use case can be represented end to end
- command templates, code snippets, resources, artifacts, and recipes are allowed
- scenario tests are first-class
- inheritance and sharing are documented future work, not v0 behavior

## CLI Goals

The CLI should make policies inspectable and testable:

```sh
skillspec validate skill.spec.yml
skillspec test skill.spec.yml
skillspec decide skill.spec.yml --input='browse my calendar'
skillspec decide skill.spec.yml --input='browse my calendar' --trace-dir .skillspec/traces
skillspec explain skill.spec.yml --input='browse my calendar'
skillspec deps check skill.spec.yml
skillspec deps check skill.spec.yml --command deno_replay
skillspec trace compact .skillspec/traces/<run-id>
skillspec compile skill.spec.yml --target codex-skill
skillspec import-skill SKILL.md --out skill.spec.yml
```

Pass only the task text to `--input`. Do not include the skill invocation
prefix, and prefer single quotes when the task text contains `$skill-name`
syntax so the shell does not expand it.

`import-skill` is not magic. It uses deterministic extraction first:
frontmatter, headings, Markdown resources, fenced code blocks, shell-like
command blocks, tables, "always/never/forbid" language, examples, and
references. Multi-file skill folders are imported as resources; fenced code is
preserved as `code` with provenance. Shell-like snippets can also become
command templates. An optional agent-assisted pass can propose rules, recipes,
states, artifacts, and safety classification, but uncertainty must be marked as
`review_required`.

`deps check` validates the declared dependency surface. Use `--command` to
check only the dependencies required by a specific command template before use.
It can directly check CLI tools on `PATH`, files, and environment variables.
Services, adapters, browsers, and package managers are reported as
harness-specific checks. Provision options are advisory until the harness
elicits approval; SkillSpec does not silently install global tools.

`compile` is a complete renderer, not a summary generator. Codex/Claude skill
targets include the runtime contract, activation hints, ranked routes, ordered
rules, bounded elicitations, lifecycle states, dependencies, command templates,
snippets, closures, scenario tests, proof metrics, review notes, and CLI
commands for validation and explanation.

## Repository Layout

```text
spec/       specification, schema, semantics, security notes
examples/   complete SkillSpec examples
skills/     companion skills for authoring, importing, and dogfooding specs
generators/ compiler target notes for Codex, Claude, Markdown
crates/     reference Rust CLI
fixtures/   sample skills and expected outputs
```

## Formal Model

SkillSpec v0 has a formal grammar and relationship model:

- [spec/grammar.md](spec/grammar.md) defines the v0 tree.
- [spec/relationships.md](spec/relationships.md) explains how routes, rules,
  states, commands, snippets, tests, and proof associate.
- [spec/rules.md](spec/rules.md) defines rule evaluation and negative
  steering.
- [spec/trace.md](spec/trace.md) defines append-only decision traces.

The core association is:

```text
rules steer routes, elicitations, and closures
states organize lifecycle
elicitations ask bounded questions
dependencies declare required tools and provision choices
commands perform named actions
snippets preserve product language
tests prove steering behavior
trace records runtime causality
proof summarizes accuracy and savings
```

## Status

Pre-alpha. This repository starts with a focused v0 spec, a typed Rust CLI, and
examples for `rote-computer`, `rote-shell`, repo readiness, and local CSV
reporting. The first flagship example is `rote-computer`, a task-first
supertool policy for routing work across remembered routes, services, CLIs,
browsers, and completion memory.

`examples/rote-shell.skill.spec.yml` is the first serious port target: it
turns the current rote-shell prose skill into routes, rules, states, commands,
closures, and scenario tests.

`examples/local-csv-report.skill.spec.yml` is the first non-repo rote-shell
example: it keeps local data local, declares CLI dependencies, captures file
provenance, saves report artifacts, and routes recurring reports toward
crystallization.

Two companion skills dogfood the format:

- `skills/skillspec-creator/SKILL.md` creates a reviewed `skill.spec.yml` from
  an existing prose `SKILL.md`, whether the source is a local file, local skill
  folder, public GitHub repo, or public GitHub repo path. It stages remote
  sources locally, uses deterministic extraction only as a first pass, proves
  the spec with validation/tests/explanations, and only then prepares optional
  harness installation.
- `skills/skillspec-runtime/SKILL.md` teaches an agent how to use an existing
  `skill.spec.yml` at runtime: validate, decide with a trace directory, obey
  rules/forbids/elicitations, execute through the right harness tools, and
  report the decision trace plus evidence.
