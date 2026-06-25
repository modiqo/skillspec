<p align="center">
  <img src="assets/skillspec-wordmark.svg" alt="SkillSpec" width="520">
</p>

# Stop guessing whether your agent followed the skill.

[![CI](https://github.com/modiqo/skillspec/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/modiqo/skillspec/actions/workflows/ci.yml)

Do any of these sound familiar?

- You are building a skill for a CLI, API, MCP server, or product workflow.
- Your skill has step-by-step instructions, scripts, code blocks, or extra
  reference files.
- You have too many skills and need the agent to pick the right one.
- You need to know whether the agent skipped a step.
- You need to control which tools the agent can use.
- You need dependency checks before the agent acts.
- You need proof of what happened after the run.

If even two of these sound familiar, SkillSpec matters.

It keeps your normal `SKILL.md`: instructions, examples, scripts, and
references still work.

SkillSpec adds a small contract beside it so the agent can plan the work, follow
the right steps, stay inside tool boundaries, check dependencies, record
progress, and show proof at the end.

No new agent runtime. No orchestration platform. Just a `skill.spec.yml`, a
small generated `SKILL.md` loader, and a CLI that validates, plans, records, and
reports.

## What SkillSpec Adds

A regular skill tells the agent what to do. SkillSpec makes the important parts
checkable.

It adds structure for:

- when the skill should be used
- which route the agent should take
- which phases must run in order
- which tools are allowed or blocked
- which dependencies must be checked
- which progress events should be recorded
- which proof should exist after the run

It does not compete with skills. It makes important skills easier to operate,
review, and trust.

## Why It Exists

A normal `SKILL.md` is still prompt text. It can be loaded late, skipped,
misread, reordered, or followed only halfway.

SkillSpec moves the load-bearing parts out of long instruction text and into a
contract:

```text
Skill:     "Load these instructions."
MCP:       "These tools are available."
SkillSpec: "Choose this route, follow these phases, prove these checks."
```

The honest guarantee:

SkillSpec does not make a model obey. It makes the contract checkable,
non-compliance detectable, and the gateable parts enforceable by a harness.

## How SkillSpec Works

SkillSpec gives you two pieces:

- a CLI that does the structured work
- a `skillspec` skill you install into your agent environment

Together, they help create planned skills: a normal `SKILL.md` plus a
`skill.spec.yml`.

The `skill.spec.yml` spells out:

- when to use the skill
- what steps to follow
- what to check
- what proof to show

The CLI can:

- create or update `skill.spec.yml`
- validate and test the contract
- compile a small `SKILL.md` loader
- install the skill into a harness
- plan a run
- record progress
- produce proof

The installed `skillspec` skill is the chat entry point. It lets you use the
CLI from inside Codex, Claude, or Agents without thinking about every command.
You stay in the harness, ask for the outcome, and let the skill use its own
contract to choose the route and run the right steps.

From this repo checkout, install the CLI:

```sh
cargo install --path crates/skillspec-cli --force
skillspec --help
```

Then install the `skillspec` skill into your harness:

```sh
# Codex
skillspec install skill skills/skillspec --target codex --retire-existing

# Agents
skillspec install skill skills/skillspec --target agents --retire-existing

# Claude local project
skillspec install skill skills/skillspec --target claude-local --retire-existing
```

After that, you can stay in chat:

```text
/skillspec import ./my-skill, compile it for Codex, install it, and prove it
```

That one request stages the source, turns the existing instructions into a
SkillSpec contract, preserves package resources, validates and tests the spec,
compiles the generated skill, installs it, and reports proof.

The important loop is:

```text
problem or skill folder
  -> /skillspec request in chat
  -> SkillSpec CLI commands behind the scenes
  -> reviewed skill files, checks, progress, and proof
```

The prompt surface is powered by SkillSpec too:

```text
/skillspec chat request
  -> skills/skillspec/skill.spec.yml
  -> selected route and phase checklist
  -> commands, checks, progress, and alignment proof
```

The `/skillspec` multiplexer is not a hand-written exception. It is powered by
[`skills/skillspec/skill.spec.yml`](skills/skillspec/skill.spec.yml), the same
kind of contract it helps create for other skills.

## Common Operator Flows

SkillSpec is most useful when a skill, tool, or workflow has become too
important to leave as instructions alone.

| User problem | What you ask for | What SkillSpec does |
| --- | --- | --- |
| "My skill is over 1000+ lines and the agent is not following instructions." | `/skillspec import ./my-skill, compile it for Codex, install it, and prove it` | Turns a long skill into a smaller, followable contract: routes, rules, dependencies, tests, installed harness files, and an alignment report that shows what the agent actually followed. |
| "I switched from Codex to Claude and need proof at the end of execution." | `/skillspec complete the task and print an alignment report` | Runs the task through the same contract shape across harnesses, then prints selected route, required steps, missing evidence if any, and final alignment status. |
| "I designed a CLI, API, or MCP and want skills that use it correctly." | `/skillspec install durable-executor from /path/or/uri`<br>`/skillspec create from observed durable execution: "use function [A], [B], [C] of my CLI [name-cli]"` | Uses [Rote by Modiqo](https://www.modiqo.ai) as the trace substrate, captures a real execution as evidence, converts the observed workflow into a reusable SkillSpec-backed skill, and preserves command and dependency proof. |
| "I have too many skills and my agent environment shortened descriptions to fit context." | `/skillspec install router` | Installs the SkillSpec router, builds an index, routes to the right skill on demand, and frees context for the skill that actually matters. |
| "I need to prove a skill before release." | `/skillspec prove this installed skill` | Runs decision, test, dependency, progress, and alignment checks so release claims are backed by evidence. |
| "I need a read-only view of installed SkillSpec state." | `/skillspec status` | Reports router and [durable-executor](#rote-prerequisite-for-agent-traces) installed/enabled state, supported roots, last router index state, and SkillSpec-backed versus legacy text-only skills. |

## Start Here

Once the CLI and `skillspec` skill are installed, ask for the outcome you want in
chat.

| You want to... | Say this in chat |
| --- | --- |
| Port and prove a skill | `/skillspec import ./my-skill, compile it for Codex, install it, and prove it` |
| See installed lifecycle state | `/skillspec status` |
| Route a large skill library | `/skillspec install router` |
| Temporarily turn router mode off | `/skillspec disable router` |
| Turn router mode back on | `/skillspec enable router` |
| Capture a tool-backed workflow | `/skillspec install durable-executor from /path/or/public-uri`<br>Requires [Rote by Modiqo](https://www.modiqo.ai) for agent traces. |
| Synthesize a skill from observed work | `/skillspec create from observed durable execution: "use parallel web to enrich this profile"` |
| Turn durable first-hop off | `/skillspec disable durable-executor`<br>Keeps the [durable-executor](#rote-prerequisite-for-agent-traces) files installed but stops automatic routing. |

The intended user experience is simple: describe the problem, let SkillSpec run
the structured steps, then inspect the proof report.

### Lifecycle Commands

Once SkillSpec is installed, these commands manage the local skill environment.

| Command | What changes |
| --- | --- |
| `/skillspec status` | Read-only inventory of router state, [durable-executor](#rote-prerequisite-for-agent-traces) state, supported roots, router index freshness, and SkillSpec-backed versus legacy text-only skills. |
| `/skillspec install router` | Installs the router, makes the router implicit, makes routed skills explicit-only, builds the routing index, and runs a clean status check. |
| `/skillspec disable router` | Keeps router files installed but makes the router explicit-only and restores routed skills to implicit/default discovery. |
| `/skillspec enable router` | Turns router mode back on and rebuilds the index from current roots. |
| `/skillspec update router` | Backs up config, manifest, index, and generated router skills, rewrites recorded harness roots, preserves enabled/disabled state, and warns you to restart active sessions. |
| `/skillspec install durable-executor from /path/or/public-uri` | Installs the optional [durable-executor](#rote-prerequisite-for-agent-traces) first-hop after checking [Rote by Modiqo](https://www.modiqo.ai) is on `PATH`, so tool-backed work can preserve traces, evidence, alignment, and token stats. |
| `/skillspec disable durable-executor` | Keeps [durable-executor](#rote-prerequisite-for-agent-traces) installed but makes it explicit-only. |
| `/skillspec enable durable-executor` | Checks [Rote by Modiqo](https://www.modiqo.ai) on `PATH` before making [durable-executor](#rote-prerequisite-for-agent-traces) implicit again. |

If a skill is later added outside SkillSpec, `skillspec router index status`
detects text-only versus SkillSpec-backed additions and
`skillspec router index refresh` reapplies explicit invocation controls and
rebuilds the index. Observed-workspace synthesis refuses to write until the
observed result and evidence summary are approved; if live
[Rote by Modiqo](https://www.modiqo.ai) workspace lookup is unreliable, pass
pre-captured stats, log, and metadata files explicitly.

### Rote Prerequisite For Agent Traces

The [durable-executor](#rote-prerequisite-for-agent-traces) path is optional,
but it depends on [Rote by Modiqo](https://www.modiqo.ai). Rote is the trace
substrate SkillSpec uses to run tool-backed work, preserve command logs,
capture agent evidence, and report token stats outside the prompt.

Install Rote before enabling or installing the
[durable-executor](#rote-prerequisite-for-agent-traces):

```sh
curl -fsSL https://raw.githubusercontent.com/modiqo/rote-releases/main/install.sh | bash
```

After `rote` is on `PATH`, SkillSpec can safely use the
[durable-executor](#rote-prerequisite-for-agent-traces) first-hop for workflows
that need durable traces, evidence handles, replayable command history, and
alignment proof.

## Install From Source

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

Start with an ordinary skill folder:

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
instruction shape carry?"

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

If the source is a whole skills repo with many `SKILL.md` files, map the
workspace first. This is authoring-side structure recon, not router indexing:

```sh
skillspec workspace map ./skills --out ./build/skillspec.workspace.yml
skillspec workspace validate ./build/skillspec.workspace.yml
skillspec workspace import ./build/skillspec.workspace.yml --out ./workspace-build
skillspec workspace converge ./build/skillspec.workspace.yml --build-root ./workspace-build
skillspec workspace compile ./build/skillspec.workspace.yml --build-root ./workspace-build --target codex-skill
skillspec workspace install ./build/skillspec.workspace.yml --build-root ./workspace-build --target codex --dry-run
```

The workspace manifest names each atomic skill package, records deterministic
install slugs, and captures cross-skill references such as shared standards
packages before fanout import. The import step writes one generated package per
atomic skill under the build root; it does not compile, install, or refresh the
router. The converge step verifies those generated package drafts against the
workspace graph. The compile step writes harness-ready `SKILL.md` loaders for
ready packages only. The install step plans every harness write first, uses the
manifest `install_slug` folders, blocks collisions, and writes install proof; it
still does not refresh the router.

### 2. Map And Import

These commands preserve source structure before generating the first contract
for one atomic skill package.

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

Compilation turns the reviewed contract into the small `SKILL.md` loader the
agent environment loads. Choose the target you are installing into:

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

Use `--retire-existing` when replacing an active text-only skill with the reviewed
SkillSpec-backed port. It backs up the old active skill outside harness
discovery before installing the replacement. Use `--name <new-name>` only when
you intentionally want side-by-side testing.

### What You Get

```text
my-skill/
  SKILL.md          # small loader for the agent environment
  skill.spec.yml    # routes, rules, phases, checks, proof contract
  deps.toml         # reviewed dependency ledger
  resources/        # examples, scripts, references, and source evidence
  source/
    SKILL_md.old    # preserved original instructions; not SKILL.md and not .md
```

## Prove It Worked

Proof is the difference between "the agent probably followed the skill" and
"the run produced evidence."

### Proof Flow

| Step | Command | What it proves |
| --- | --- | --- |
| Plan | `skillspec plan ...` | The input selects the expected route and phase order. |
| Act | `skillspec act ...` | The next phase has a concrete checklist and tool boundary. |
| Record | `skillspec progress record ...` | The run ledger captures phase, requirement, and evidence events. |
| Review | `skillspec progress show ...` | The execution ledger has progress events and no observed forbidden actions. |
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
