<p align="center">
  <img src="assets/skillspec-wordmark.svg" alt="SkillSpec" width="520">
</p>

# Skills load instructions. SkillSpec turns them into a plan the agent can prove.

[![CI](https://github.com/modiqo/skillspec/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/modiqo/skillspec/actions/workflows/ci.yml)

Agent skills made reusable prompts portable. SkillSpec is the next layer: a tiny
behavior contract that keeps agents on track, measures what happened, and makes
missed steps visible.

It does not compete with skills. It makes skills stronger.

## What It Is

SkillSpec is a grammar for agent skills.

It turns prose into:

- routes the agent can choose
- phases the agent can execute in order
- tool boundaries the agent should not cross without permission
- checks the harness can run
- progress events the run can record
- proof reports the user can inspect

No heavyweight agent runtime. No new orchestration system. Just a
`skill.spec.yml`, a tiny `SKILL.md` trampoline, and a CLI that validates,
plans, records, and reports.

## Why It Exists

A normal `SKILL.md` is still prompt text. It can be loaded late, skipped,
misread, reordered, or followed only halfway.

SkillSpec moves the load-bearing parts out of prose and into a contract:

```text
Skill:     "Load these instructions."
MCP:       "These tools are available."
SkillSpec: "Choose this route, follow these phases, prove these checks."
```

The honest guarantee:

SkillSpec does not make a model obey. It makes the contract checkable,
non-compliance detectable, and the gateable parts enforceable by a harness.

## How It Works

<p align="center">
  <img src="assets/skillspec-grammar-plan-loop.svg" alt="SkillSpec converts prose skills into routed plans, progress ledgers, verifiers, and proof reports." width="860">
</p>

The loop is intentionally visible:

```text
import existing skill -> compile for harness -> install -> run -> prove value
```

Inside a harness run:

```text
sensemake -> plan -> act -> progress -> align -> value report
```

The user sees concrete proof, not vague confidence:

```text
Decision replay: pass
Phase order: pass
Requirements: 4/5 proven
Missing proof: requirement `install_codex` has no progress event
Forbidden actions: no violations recorded
Alignment: partial
```

## What `/skillspec` Does In Chat

The installed `skillspec` skill is a prompt multiplexer. You give it one plain
chat request, and its `skill.spec.yml` chooses the route, phases, commands,
checks, and proof obligations.

```text
/skillspec import https://github.com/anthropics/skills/tree/main/skills/pdf,
compile it for Codex, install it, and prove it
```

That one prompt stages the source, converts prose into a SkillSpec contract,
preserves package resources, validates and tests the spec, compiles the harness
skill, installs it, and reports proof.

The broader use cases are:

| Goal | Chat prompt | What SkillSpec does |
| --- | --- | --- |
| Make skills verifiable | `/skillspec import ./my-skill, compile it for Codex, install it, and prove it` | Converts a prose `SKILL.md` into routes, rules, phases, dependencies, resources, commands, tests, progress tracking, and alignment proof. |
| Route large skill libraries | `/skillspec install router` | Marks managed skills explicit-only, builds a routing index, repairs out-of-band additions, and preserves `durable-executor` as the implicit first hop when present. |
| Make execution durable | `/skillspec install durable-executor from /path/or/public-uri` | Installs the optional durable first-hop skill so tool-backed work can preserve traces, evidence, alignment, and token stats. |
| Learn skills from work | `/skillspec create from observed durable execution: "use parallel web to enrich this profile"` | Uses a durable rote workspace as evidence, then synthesizes a reviewable SkillSpec scaffold with observed resources, dependencies, commands, and proof gaps. |
| Revise an existing contract | `/skillspec revise this spec to add router setup checks` | Starts from the current grammar and active handles, patches the reviewed contract, then reruns structural QA. |
| Prove value before release | `/skillspec prove this installed skill` | Runs decision, test, dependency, progress, and alignment checks so release claims are backed by evidence. |

## Install

Marketplace path:

1. Install the `skillspec` skill from your Codex or Claude skill marketplace.
2. In the harness, ask it to run setup or import, compile, install, and prove a skill:

```text
/skillspec import ./my-skill, compile it for Codex, install it, and prove it
/skillspec install router
/skillspec update router
/skillspec install durable-executor from /path/or/public-uri
/skillspec create from observed durable execution: "use parallel web to enrich this profile"
```

That is the intended user experience: import the existing skill, choose the
target, install it, then look at the proof report. Router setup and optional
durable-executor setup stay inside the same prompt surface. Router install
applies explicit-only native controls across managed roots, builds the routing
index, runs a clean status check, and preserves an installed durable-executor as
the implicit first hop. Router update backs up the existing config, manifest,
index, and generated router skills, rewrites every recorded harness root, and
warns you to restart active Codex, Claude, Agents, or vendor sessions. If a
skill is later added outside SkillSpec, `skillspec router index status` detects
prose-only versus SkillSpec-backed additions and `skillspec router index
refresh` reapplies explicit invocation controls and rebuilds the index.

From source:

```sh
cargo install --git https://github.com/modiqo/skillspec --package skillspec --locked
```

During local development:

```sh
cargo install --path crates/skillspec-cli --force
```

Then check the CLI:

```sh
skillspec --help
```

## Port A Skill

Take an existing prose skill folder:

```text
my-skill/
  SKILL.md
  scripts/
  references/
```

Import it:

```sh
skillspec grammar sensemake --view porting
skillspec import-skill ./my-skill --out ./my-skill/skill.spec.yml
skillspec validate ./my-skill/skill.spec.yml
skillspec test ./my-skill/skill.spec.yml
```

Compile it for a harness:

```sh
skillspec compile --target codex-skill ./my-skill
skillspec compile --target claude-skill ./my-skill
skillspec compile --target markdown ./my-skill
```

Install it:

```sh
skillspec install skill ./my-skill --target codex --target agents --force
```

That gives you:

```text
my-skill/
  SKILL.md          # small trampoline for the harness
  skill.spec.yml    # routes, rules, phases, checks, proof
  source/           # preserved original material, when present
```

## Prove It Worked

Run a realistic task through the spec:

```sh
skillspec plan ./my-skill/skill.spec.yml \
  --input "do the real task" \
  --trace-dir .skillspec/traces

skillspec act ./my-skill/skill.spec.yml \
  --input "do the real task" \
  --run .skillspec/traces/<run-id> \
  --phase <phase-id>

skillspec progress show ./my-skill/skill.spec.yml \
  --run .skillspec/traces/<run-id>

skillspec trace align ./my-skill/skill.spec.yml \
  --decision-trace .skillspec/traces/<run-id> \
  --execution-trace .skillspec/traces/<run-id>/execution.jsonl
```

Expected report shape:

```text
Ported: rote-browse
Installed: Codex, Agents

Extracted value:
- 7 activation triggers
- 5 routes
- 12 rules
- 6 dependency checks
- 4 command templates
- 9 tests
- 3 references moved out of prompt path
- estimated context reduction: 68%
- decision trace: .skillspec/traces/run-...
- alignment: pass / partial / fail
```

## Harness Support

SkillSpec is portable at the contract layer. Any harness can use it if it can
load a small instruction file and run local CLI commands.

Current install targets:

```text
codex        ~/.codex/skills/<name>
agents       ~/.agents/skills/<name>
claude-local .claude/skills/<name>
```

For other harnesses, compile to Markdown today and add a native install target
when the harness discovery path is known.

## Deeper Docs

This README is the fast path.

- [Detailed README](README_DETAILED.md)
- [Docs index and reader paths](docs/README.md)
- [The Reliability Gap In Agent Skills](docs/00-skills-reliability-gap.md)
- [Design docs](docs/design/README.md)
- [Why SkillSpec](docs/01-why-skillspec.md)
- [Grammar](spec/grammar.md)
- [Schema](spec/skill.spec.schema.json)
