# SkillSpec

[![CI](https://github.com/modiqo/skillspec/actions/workflows/ci.yml/badge.svg)](https://github.com/modiqo/skillspec/actions/workflows/ci.yml)

Keep the prose. Structure the decisions.

SkillSpec turns long prose skills into compact, testable behavior contracts.
The prose still teaches tone and context. The `skill.spec.yml` carries the
parts agents should not guess: routes, rules, dependencies, code snippets,
resources, recipes, elicitations, tests, and traces.

Use it when you want a skill to be portable across Codex, Claude, Hermes, or
another harness without relying on paragraphs of instructions alone.

## Where SkillSpec Fits

- Agent Skills define what to load.
- MCP defines what tools and data are available.
- SkillSpec defines how the agent should decide, verify, and report behavior.

SkillSpec is not a replacement for prose, MCP, or harness policy. It is the
machine-checkable contract for the behavioral parts of a skill that should be
tested, traced, compiled, and reviewed.

## Install The CLI

From this repository:

```sh
cargo install --path crates/skillspec-cli --force
```

During development you can also run the local binary directly:

```sh
cargo build
./target/debug/skillspec --help
```

Check the installed CLI has the expected surface:

```sh
skillspec --help
skillspec import-skill --help
skillspec deps check --help
```

## Create A SkillSpec From An Existing Skill

For serious ports, use the creator skill:

```text
/skillspec-creator port <local-skill-folder-or-github-url>
```

Examples:

```text
/skillspec-creator port /Users/me/.agents/skills/rote-shell
/skillspec-creator port https://github.com/anthropics/skills/tree/main/skills/pdf
```

The creator skill does the careful path:

1. Stages remote sources locally.
2. Reads the skill folder, not just `SKILL.md`.
3. Runs the deterministic importer.
4. Promotes resources, code snippets, artifacts, recipes, dependencies, rules,
   and tests into a reviewed `skill.spec.yml`.
5. Validates and tests the spec.
6. Optionally compiles and installs a generated harness skill.

The mechanical importer is available directly when you only want a draft:

```sh
skillspec import-skill path/to/skill-folder --out skill.spec.yml
skillspec validate skill.spec.yml
skillspec test skill.spec.yml
skillspec deps check skill.spec.yml
```

`import-skill` preserves source material; it does not pretend to understand the
whole skill. It extracts Markdown resources, fenced code blocks, shell-like
commands, obvious dependencies, headings, and strong directive language, then
marks uncertainty as `review_required`.

## Install A SkillSpec-Backed Skill

A generated skill folder should look like this:

```text
my-skill/
  SKILL.md          # minimal harness-facing loader
  skill.spec.yml    # structured behavior contract and source of truth
  source/           # optional preserved source skill/resources
```

Detect available harness skill roots:

```sh
skillspec install targets
```

Preview an install:

```sh
skillspec install skill my-skill --target agents --target codex --dry-run
```

Install into one or more harnesses:

```sh
skillspec install skill my-skill --target agents
skillspec install skill my-skill --target agents --target codex
skillspec install skill my-skill --all-detected
```

The creator skill can prepare this folder after validation. Do not install a
generated skill until:

```sh
skillspec validate my-skill/skill.spec.yml
skillspec test my-skill/skill.spec.yml
skillspec deps check my-skill/skill.spec.yml
```

If `deps check` reports confirmed missing local dependencies, leave the skill
draft-only or add explicit provision choices. Package, service, adapter, and
browser checks may be reported as `deferred`; those remain visible and must be
verified by the harness or runtime path before use. SkillSpec should not
silently install global dependencies.

Current install targets:

- `agents`: `~/.agents/skills/<skill-name>`
- `codex`: `~/.codex/skills/<skill-name>`
- `claude-local`: nearest `.claude/skills/<skill-name>` in the current repo

## Use A SkillSpec-Backed Skill

In a harness session, invoke the generated skill normally:

```text
/my-skill do the task
```

The generated `SKILL.md` should tell the agent to use the sibling
`skill.spec.yml`. The runtime flow is:

```sh
skillspec validate path/to/skill.spec.yml
skillspec deps check path/to/skill.spec.yml
skillspec decide path/to/skill.spec.yml \
  --input='the user task text' \
  --trace-dir .skillspec/traces
```

For debugging:

```sh
skillspec explain path/to/skill.spec.yml \
  --input='the user task text' \
  --trace-dir .skillspec/traces

skillspec trace compact .skillspec/traces/<run-id>
```

The runtime skill [skills/skillspec-runtime/SKILL.md](skills/skillspec-runtime/SKILL.md)
teaches agents how to use an existing `skill.spec.yml`: validate first, check
dependencies, decide with a trace, obey forbids and elicitations, execute with
the right harness tools, then report trace and evidence.

## Compile A SkillSpec Into Harness Guidance

Render a `skill.spec.yml` into a harness-facing `SKILL.md`:

```sh
skillspec compile skill.spec.yml --target codex-skill > SKILL.md
skillspec compile skill.spec.yml --target claude-skill > SKILL.md
skillspec compile skill.spec.yml --target markdown > skill.spec.md
```

For harness skill targets, compilation emits a minimal loader by default. The
loader points the agent at the colocated `skill.spec.yml`, tells it to run
`skillspec decide ... --trace-dir`, preserve the emitted `run_dir`, and follow
the spec. This keeps `SKILL.md` small and prevents it from becoming a second
source of truth.

Use the Markdown target when you want a full human-readable rendering of the
contract. `--target markdown` includes routes, rules, dependencies, resources,
code, artifacts, recipes, commands, snippets, closures, scenario tests, proof
metrics, review notes, and CLI commands for validation and explanation.

## What Goes In A Spec

A `skill.spec.yml` can describe:

- intent routing
- route order
- forbidden substitutions
- bounded user questions and choices
- state transitions
- declared dependencies and provision choices
- source resources from imported multi-file skills
- code snippets with provenance, dependencies, inputs, outputs, and safety
- named artifacts consumed or produced by code and commands
- ordered recipes for procedural skills
- command templates
- completion closures
- scenario tests
- decision traces
- proof metrics

Example shape:

```yaml
schema: skillspec/v0
id: rote.computer
title: rote computer
description: Route task-first work across remembered routes, services, CLIs, and browsers.

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

## Repository Layout

```text
spec/       specification, schema, semantics, security notes
examples/   complete SkillSpec examples
conformance/ valid and invalid fixtures for CLI conformance behavior
skills/     creator/runtime skills for authoring and using specs
.claude/    repo-local SkillSpec-backed skills for this repository
generators/ compiler target notes for Codex, Claude, Markdown
crates/     reference Rust CLI
fixtures/   sample skills and expected outputs
```

Useful examples:

- [examples/rote-computer.skill.spec.yml](examples/rote-computer.skill.spec.yml)
- [examples/rote-shell.skill.spec.yml](examples/rote-shell.skill.spec.yml)
- [examples/local-csv-report.skill.spec.yml](examples/local-csv-report.skill.spec.yml)
- [examples/pdf-processing/skill.spec.yml](examples/pdf-processing/skill.spec.yml)
- [examples/before-after/](examples/before-after/) shows a prose skill before
  and after a SkillSpec-backed port.

## Verification Suite

Run the Rust unit and CLI integration suite:

```sh
cargo test --workspace --all-targets
```

Run the repository-level conformance sweep:

```sh
cargo build
find examples -name '*.yml' -exec target/debug/skillspec validate {} \;
find examples -name '*.yml' -exec target/debug/skillspec test {} \;
find examples -name '*.yml' -exec target/debug/skillspec deps check {} \;
```

The test suite covers strict typo rejection across typed grammar nodes,
scenario-test pass/fail behavior, required trace enforcement and compaction,
dependency status semantics and command scoping, compiler targets, importer
draft generation, install target behavior, full JSON Schema validation against
examples, conformance fixtures, and golden snapshots for compiler/importer
output.

CI runs a full Ubuntu quality gate plus native locked build/test jobs on Linux,
macOS, and Windows.

The minimum compliance gate for a SkillSpec-backed skill is:

```sh
skillspec validate skill.spec.yml
skillspec test skill.spec.yml
skillspec deps check skill.spec.yml
```

## Community And RFC

- [docs/rfc-v0.md](docs/rfc-v0.md) is the RFC-style announcement draft.
- [docs/why-skillspec.md](docs/why-skillspec.md) explains why structured
  behavior contracts help.
- [docs/prose-vs-skillspec.md](docs/prose-vs-skillspec.md) compares prose-only
  skills with SkillSpec-backed skills.
- [docs/community-outreach.md](docs/community-outreach.md) names the launch
  audiences and the specific ask.
- [DISCUSSIONS.md](DISCUSSIONS.md) defines recommended GitHub Discussions
  categories.
- [CONTRIBUTING.md](CONTRIBUTING.md) describes local development, spec changes,
  and golden snapshot updates.
- [docs/good-first-issues.md](docs/good-first-issues.md) lists starter issues
  for contributors.
- [docs/community-posts.md](docs/community-posts.md) contains short launch post
  drafts.

Repo-local skills live in `.claude/skills/<name>/` and are checked in despite
common global ignores for `.claude*`. Each one keeps:

```text
.claude/skills/<name>/
  SKILL.md          # minimal loader generated by skillspec compile
  skill.spec.yml    # reviewed structured contract
  source/SKILL.md   # original prose skill retained as provenance
```

## Formal Model

SkillSpec v0 has a formal grammar and relationship model:

- [spec/grammar.md](spec/grammar.md) defines the v0 tree.
- [spec/relationships.md](spec/relationships.md) explains how concepts
  associate.
- [spec/rules.md](spec/rules.md) defines rule evaluation and negative
  steering.
- [spec/trace.md](spec/trace.md) defines append-only decision traces.
- [spec/skill.spec.schema.json](spec/skill.spec.schema.json) is the strict JSON
  schema for typed v0 fields.

The core association is:

```text
rules steer routes, elicitations, and closures
states organize lifecycle
elicitations ask bounded questions
dependencies declare required tools and provision choices
resources preserve source provenance
code preserves executable knowledge
artifacts name consumed and produced data
recipes bind ordered procedures
commands perform named actions
tests prove steering behavior
trace records runtime causality
proof summarizes accuracy and savings
```

## Status

Pre-alpha. The CLI and spec are useful for dogfooding now, but the format is
still moving. The current focus is proving that existing prose skills can be
ported into structured, testable, cross-harness behavior without losing their
source material.
