<p align="center">
  <img src="assets/skillspec-wordmark.svg" alt="SkillSpec" width="520">
</p>

# SkillSpec makes agent skills followable, testable, and provable.

[![CI](https://github.com/modiqo/skillspec/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/modiqo/skillspec/actions/workflows/ci.yml)

It turns messy skill instructions into a small contract an agent can follow,
check, and prove.

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

## What `/skillspec` Does In Chat

The installed `skillspec` skill is a prompt multiplexer. You give it one plain
chat request, and its `skill.spec.yml` chooses the route, phases, commands,
checks, and proof obligations.

The useful part is not the command text. It is the exchange:

```text
messy skill, fragile tool workflow, or overloaded skill library
  -> /skillspec command
  -> reviewed contract, installed harness files, and proof
```

Example:

```text
/skillspec import https://github.com/anthropics/skills/tree/main/skills/pdf,
compile it for Codex, install it, and prove it
```

That one prompt stages the source, converts prose into a SkillSpec contract,
preserves package resources, validates and tests the spec, compiles the harness
skill, installs it, and reports proof.

### Use Cases

SkillSpec is most useful when a skill, tool, or workflow has become too
important to leave as prompt text. The chat command is the user-facing
exchange: you state the problem, SkillSpec turns it into a route, a command
sequence, and proof that the value was delivered.

| User problem | SkillSpec command sequence and value exchanged |
| --- | --- |
| "My skill is over 1000+ lines and the agent is not following instructions." | `/skillspec import ./my-skill, compile it for [Codex, Claude, Agents], install it, and prove it`<br><br>Turns a long prose skill into a smaller, followable contract: routes, rules, dependencies, tests, installed harness files, and an alignment report that shows what the agent actually followed. |
| "I switched from Codex to Claude and need my skill to follow instructions and create alignment proof at the end of execution." | `/skillspec complete the task and print an alignment report`<br><br>Runs the task through the same contract shape across harnesses, then prints the proof: selected route, required steps, missing evidence if any, and final alignment status. |
| "I designed a new CLI, API, or MCP for my product and I want to distribute skills that use it in alignment with real use cases." | `/skillspec install durable-executor from /path/or/uri`<br>`/skillspec create from observed durable execution: "use function [A], [B], [C] of my CLI [name-cli]"`<br>`/skillspec disable durable-executor`<br><br>Captures a real execution as evidence, converts the observed workflow into a reusable SkillSpec-backed skill, preserves command and dependency proof, and lets you turn the durable first-hop back off after synthesis. |
| "I have too many skills and I am seeing: Skill descriptions were shortened to fit the 2% skills context budget." | `/skillspec install router`<br><br>Installs the SkillSpec router so the harness can keep every skill discoverable without loading every long description. It builds an index, routes to the right skill on demand, and frees context for the skill that actually matters. |

### Powered By SkillSpec

The `/skillspec` chat multiplexer is not a hand-written exception. SkillSpec is
powered by its own contract: [`skills/skillspec/skill.spec.yml`](skills/skillspec/skill.spec.yml).

That YAML file is the engine behind the prompt surface. It declares the routes,
rules, phase plans, dependency checks, router lifecycle, optional
durable-executor lifecycle, observed-workspace synthesis, and proof obligations
that `/skillspec` follows.

```text
/skillspec chat request
  -> skills/skillspec/skill.spec.yml
  -> selected route and phase checklist
  -> commands, checks, progress, and alignment proof
```

That is the important claim: the tool uses the same SkillSpec machinery it gives
to user skills. The multiplexer is itself a working example of a large prompt
surface compressed into a reviewable, testable contract.

### Core Workflows

| Goal | Chat prompt | What SkillSpec does |
| --- | --- | --- |
| Make skills verifiable | `/skillspec import ./my-skill, compile it for Codex, install it, and prove it` | Converts a prose `SKILL.md` into routes, rules, phases, dependencies, resources, commands, tests, progress tracking, and alignment proof. |
| Inspect installed state | `/skillspec status` | Reports router and durable-executor installed/enabled state, supported roots, last router index state, and SkillSpec-backed versus legacy prose skills. |
| Route large skill libraries | `/skillspec install router` | Installs an implicit router surface, marks routed skills explicit-only, builds a routing index, repairs out-of-band additions, and preserves `durable-executor` as implicit only when durable is enabled. |
| Make execution durable | `/skillspec install durable-executor from /path/or/public-uri` | Installs the optional durable first-hop skill after checking `rote` is on `PATH`, so tool-backed work can preserve traces, evidence, alignment, and token stats. |
| Learn skills from work | `/skillspec create from observed durable execution: "use parallel web to enrich this profile"` | Uses a durable rote workspace as evidence, shows the observed result for approval, then synthesizes a reviewable SkillSpec scaffold with observed resources, dependencies, commands, and proof gaps. |
| Revise an existing contract | `/skillspec revise this spec to add router setup checks` | Starts from the current grammar and active handles, patches the reviewed contract, then reruns structural QA. |
| Prove value before release | `/skillspec prove this installed skill` | Runs decision, test, dependency, progress, and alignment checks so release claims are backed by evidence. |

## Install

### Fast Path

1. Install the `skillspec` skill from your Codex or Claude skill marketplace.
2. In chat, ask for the outcome you want:

| You want to... | Say this in chat |
| --- | --- |
| Port and prove a skill | `/skillspec import ./my-skill, compile it for Codex, install it, and prove it` |
| See installed lifecycle state | `/skillspec status` |
| Route a large skill library | `/skillspec install router` |
| Temporarily turn router mode off | `/skillspec disable router` |
| Turn router mode back on | `/skillspec enable router` |
| Capture a tool-backed workflow | `/skillspec install durable-executor from /path/or/public-uri` |
| Synthesize a skill from observed work | `/skillspec create from observed durable execution: "use parallel web to enrich this profile"` |
| Turn durable first-hop off | `/skillspec disable durable-executor` |

The intended user experience is simple: import the existing skill, choose the
target, install it, then look at the proof report.

### Lifecycle Commands

| Command | What changes |
| --- | --- |
| `/skillspec status` | Read-only inventory of router state, durable-executor state, supported roots, router index freshness, and SkillSpec-backed versus legacy prose skills. |
| `/skillspec install router` | Installs the router, makes the router implicit, makes routed skills explicit-only, builds the routing index, and runs a clean status check. |
| `/skillspec disable router` | Keeps router files installed but makes the router explicit-only and restores routed skills to implicit/default discovery. |
| `/skillspec enable router` | Turns router mode back on and rebuilds the index from current roots. |
| `/skillspec update router` | Backs up config, manifest, index, and generated router skills, rewrites recorded harness roots, preserves enabled/disabled state, and warns you to restart active sessions. |
| `/skillspec install durable-executor from /path/or/public-uri` | Installs the optional durable first-hop after checking `rote` is on `PATH`, so tool-backed work can preserve traces, evidence, alignment, and token stats. |
| `/skillspec disable durable-executor` | Keeps durable-executor installed but makes it explicit-only. |
| `/skillspec enable durable-executor` | Checks `rote` on `PATH` before making durable-executor implicit again. |

If a skill is later added outside SkillSpec, `skillspec router index status`
detects prose-only versus SkillSpec-backed additions and
`skillspec router index refresh` reapplies explicit invocation controls and
rebuilds the index. Observed-workspace synthesis refuses to write until the
observed result and evidence summary are approved; if live rote workspace lookup
is unreliable, pass pre-captured stats, log, and metadata files explicitly.

### From Source

Install the CLI:

```sh
cargo install --git https://github.com/modiqo/skillspec --package skillspec --locked
```

During local development, install from this repo:

```sh
cargo install --path crates/skillspec-cli --force
```

Then check the CLI:

```sh
skillspec --help
```

## Port A Skill

Start with an ordinary prose skill:

```text
my-skill/
  SKILL.md
  scripts/
  references/
```

In chat, the whole journey can be one request:

```text
/skillspec import ./my-skill, compile it for Codex, install it, and prove it
```

Under the hood, SkillSpec runs the same staged pipeline every time.

### 1. Understand The Source

These commands answer: "What is in this skill, and what risk does the current
prose shape carry?"

```sh
skillspec grammar sensemake --view porting
skillspec doctor ./my-skill
```

`skillspec doctor` can also qualify a public GitHub single skill folder before
import:

```sh
skillspec doctor https://github.com/anthropics/skills/tree/main/skills/pdf
```

It stages the requested folder temporarily and rejects parent folders that
contain multiple `SKILL.md` files.

### 2. Map And Import

These commands preserve source structure before generating the first contract.

```sh
skillspec source map ./my-skill --out ./my-skill/.skillspec/source-map

skillspec import-skill ./my-skill \
  --out ./my-skill/skill.spec.yml \
  --source-map ./my-skill/.skillspec/source-map/source-map.json
```

The import is deliberately mechanical. Review it before install.

### 3. Review Gates

These checks keep the port honest before it becomes an active skill.

| Gate | Command | Value |
| --- | --- | --- |
| Structure | `skillspec validate ./my-skill/skill.spec.yml` | Confirms the contract parses and references connect. |
| Imports | `skillspec imports check ./my-skill/skill.spec.yml` | Confirms package-local guidance and resources load correctly. |
| Dependencies | `skillspec deps check ./my-skill/skill.spec.yml` | Shows tools, files, env vars, and services that must exist or be approved. |
| Behavior | `skillspec test ./my-skill/skill.spec.yml` | Runs scenario expectations for routes, rules, forbids, elicitations, and closures. |

For a release-quality port, fill the coverage matrix:

```text
prose_span | obligation | skillspec_construct | confidence | status | review_note
```

### 4. Compile For A Harness

Compilation turns the reviewed contract into the small `SKILL.md` trampoline
the harness loads. Choose the target you are installing into:

```sh
# Codex
skillspec compile --target codex-skill ./my-skill/skill.spec.yml > ./my-skill/SKILL.md

# Claude
skillspec compile --target claude-skill ./my-skill/skill.spec.yml > ./my-skill/SKILL.md

# Portable Markdown preview
skillspec compile --target markdown ./my-skill/skill.spec.yml > ./my-skill/compiled.md
```

### 5. Install With A Dry Run First

Preview writes before changing harness discovery roots:

```sh
skillspec install targets
skillspec install skill ./my-skill --target codex --dry-run --retire-existing
skillspec install skill ./my-skill --target codex --retire-existing
```

Use `--retire-existing` when replacing an active prose skill with the reviewed
SkillSpec-backed port. It backs up the old active skill outside harness
discovery before installing the replacement. Use `--name <new-name>` only when
you intentionally want side-by-side testing.

### What You Get

```text
my-skill/
  SKILL.md          # small trampoline loaded by the harness
  skill.spec.yml    # routes, rules, phases, checks, proof contract
  deps.toml         # reviewed dependency ledger
  resources/        # examples, scripts, references, and source evidence
  source/
    SKILL_md.old    # preserved original prose; not SKILL.md and not .md
```

## Prove It Worked

Proof is the difference between "the agent probably followed the skill" and
"the run produced evidence."

### Proof Flow

| Step | Command | What it proves |
| --- | --- | --- |
| Plan | `skillspec plan ...` | The input selects the expected route and phase order. |
| Act | `skillspec act ...` | The next phase has a concrete checklist and tool boundary. |
| Record | `skillspec progress show ...` | The execution ledger has progress events and no observed forbidden actions. |
| Align | `skillspec trace align ...` | The current spec can replay the decision and match execution evidence to obligations. |

Run a realistic task through the spec:

```sh
skillspec plan ./my-skill/skill.spec.yml \
  --input "do the real task" \
  --trace-dir .skillspec/traces
```

Execute the current phase checklist:

```sh
skillspec act ./my-skill/skill.spec.yml \
  --input "do the real task" \
  --run .skillspec/traces/<run-id> \
  --phase <phase-id>
```

Inspect progress:

```sh
skillspec progress show ./my-skill/skill.spec.yml \
  --run .skillspec/traces/<run-id>
```

Align decision and execution proof:

```sh
skillspec trace align ./my-skill/skill.spec.yml \
  --decision-trace .skillspec/traces/<run-id> \
  --execution-trace .skillspec/traces/<run-id>/execution.jsonl
```

### What The Report Tells You

| Field | Meaning |
| --- | --- |
| Ported / installed targets | Which skill was converted and where it was installed. |
| Extracted value | Activation triggers, routes, rules, dependencies, command templates, tests, and moved resources. |
| Decision trace | The saved route decision and matched rules for the task. |
| Execution trace | The saved progress events and evidence handles. |
| Alignment | `pass`, `partial`, or `fail`, with missing proof rows when evidence is incomplete. |

## Harness Support

SkillSpec is portable at the contract layer. Any harness can use it if it can
load a small instruction file and run local CLI commands.

Current install targets:

| Target | Install path |
| --- | --- |
| `codex` | `~/.codex/skills/<name>` |
| `agents` | `~/.agents/skills/<name>` |
| `claude-local` | `.claude/skills/<name>` |

For other harnesses, compile to Markdown today and add a native install target
when the harness discovery path is known.

## Deeper Docs

This README is the fast path.

- [Detailed README](docs/README_DETAILED.md)
- [Docs index and reader paths](docs/README.md)
- [The Reliability Gap In Agent Skills](docs/00-skills-reliability-gap.md)
- [Contract And Trace Methodology](docs/08-contract-trace-methodology.md)
- [Design docs](docs/design/README.md)
- [Why SkillSpec](docs/01-why-skillspec.md)
- [Grammar](spec/grammar.md)
- [Schema](spec/skill.spec.schema.json)
