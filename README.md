<p align="center">
  <img src="https://github.com/modiqo/skillspec/raw/main/assets/skillspec-wordmark.svg" alt="SkillSpec" width="520">
</p>

# Skills that agents can actually follow

[![CI](https://github.com/modiqo/skillspec/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/modiqo/skillspec/actions/workflows/ci.yml)

You wrote a good `SKILL.md`. But did the agent actually follow it, or skip the
late safety rule, grab an undeclared tool, and report "done" with no proof?

**SkillSpec tells you.** Run one command and get a risk report. Then turn any
skill into a contract the agent has to follow, with a record you can inspect at
the end.

No new agent runtime. No orchestration platform. Just a CLI and a small
`skill.spec.yml` that lives next to your `SKILL.md`.

<p align="center">
  <a href="https://github.com/modiqo/skillspec/blob/main/assets/skillspec-layer-stack.svg">
    <img src="https://raw.githubusercontent.com/modiqo/skillspec/main/assets/skillspec-layer-stack.svg" alt="SkillSpec sits inside the skills layer" width="900">
  </a>
</p>

## See It In 30 Seconds

Point Doctor at any skill, a local folder or a public GitHub URL:

```bash
skillspec doctor ./my-skill
```

```text
SkillSpec Doctor
================
Target: ./my-skill        Shape: simple_skill

Agent follow-through risk: HIGH (74/100)

Findings
- description is short and generic -> automatic discovery may be unreliable
- active skill load is 8,482 tokens -> above the balanced target
- 14 must/never obligations appear after 60% of the body -> easy to miss
- tools and commands are used, but dependencies are never declared
- no tests and no progress/trace surface -> "done" can't be checked

Likely consequence
An agent may follow the broad task but skip a late safety gate, use an
undeclared tool, or claim completion without evidence.

Next step
Ask your agent: /skillspec import ./my-skill, compile it, test it, install it,
and print the alignment summary.
```

No install required to try it. Paste a public skill URL into the hosted page:

**<https://skillspec.sh/>**

## Why This Exists

A `SKILL.md` is just text. The harness loads it and hopes the model reads the
right part. For a throwaway skill, that can be fine. For a skill you rely on,
"hope" is not a plan:

- **Buried rules get skipped.** The important "never do X" sits at line 400,
  and models are most reliable at the start and end of context, not the middle.
- **Every miss grows the prose.** Each failure becomes another paragraph, which
  makes the next miss more likely.
- **You only see the final answer.** There is no durable record of which route
  ran, which steps happened, or what was skipped.

SkillSpec moves the load-bearing parts out of prose and into a small structured
contract:

- when to use the skill
- which route to take
- what is forbidden
- what dependencies must exist
- what checks must pass
- what proof should exist at the end

## Install

Install the CLI:

```bash
curl -fsSL https://skillspec.sh/install.sh | sh
skillspec --version
```

Or with Cargo:

```bash
cargo install skillspec
skillspec --version
```

Then add the plugin to your harness.

Claude Code:

```bash
claude plugin marketplace add modiqo/skillspec --sparse .claude-plugin plugins/skillspec
claude plugin install skillspec@skillspec
claude plugin list
```

Codex:

```bash
codex plugin marketplace add modiqo/skillspec --ref main --sparse .agents --sparse plugins/skillspec
codex plugin add skillspec@skillspec
```

<details>
<summary>Other platforms, pinned releases, direct downloads, and local development</summary>

Prebuilt binaries are available on the
[releases page](https://github.com/modiqo/skillspec/releases):

- `skillspec-macos.tar.gz`
- `skillspec-linux-x86_64.tar.gz`
- `skillspec-windows-x86_64.zip`

Release artifacts include `.sha256` checksums. The installer verifies the
checksum and writes to `~/.local/bin` by default.

Pin a version or choose an install directory:

```bash
curl -fsSL https://skillspec.sh/install.sh \
  | SKILLSPEC_VERSION=v0.1.0 SKILLSPEC_INSTALL_DIR="$HOME/.local/bin" sh
```

Install unreleased `main`:

```bash
cargo install --git https://github.com/modiqo/skillspec --package skillspec --force
skillspec --version
```

Install from a local checkout:

```bash
cargo install --path crates/skillspec-cli --force
skillspec --version
```

Local development can also install the skill folder directly:

```bash
# Codex
skillspec install skill skills/skillspec --target codex --retire-existing

# Agents
skillspec install skill skills/skillspec --target agents --retire-existing

# Claude local project
skillspec install skill skills/skillspec --target claude-local --retire-existing
```

Full install notes:
[docs/install](https://github.com/modiqo/skillspec/blob/main/docs/README.md)

</details>

## The Loop: Assess -> Port -> Prove

Once the plugin is installed, ask your agent for the outcome in chat. SkillSpec
picks the commands and keeps the run aligned.

**1. Assess** a skill before you touch it.

> `/skillspec run doctor on ./my-skill`

You get a baseline: discovery risk, context load, buried obligations,
undeclared dependencies, missing proof, and the likely consequence for agent
follow-through.

**2. Port** it into a contract.

> `/skillspec import ./my-skill, compile it for Codex, install it, and prove it`

SkillSpec generates a `skill.spec.yml` next to your `SKILL.md`: routes, rules,
forbidden actions, dependencies, checks, tests, and proof expectations. It also
compiles a thin loader so the active prompt stays small.

**3. Prove** it ran the way it was supposed to.

Every run can leave an alignment summary you can read: selected route,
completed steps, missing proof, forbidden-action status, token usage, and wall
clock metrics when available. Not just "done" - a record.

Crowded skill library?

> `/skillspec install router`

Router mode routes to the one skill that matters instead of making the harness
expose too many skills at once.

## What SkillSpec Is, And Is Not

Four things you can do with it:

- **Import** an existing prose `SKILL.md` into a structured SkillSpec contract.
- **Run** a SkillSpec-backed skill in your harness, then review the alignment
  and token report.
- **Route** many skills through an explicit router when harness listing budgets
  make discovery unreliable.
- **Capture** durable execution traces and turn observed CLI/API/MCP work into
  reusable skills. This path is powered by [Rote](https://www.modiqo.ai).

| It is | It is not |
| --- | --- |
| A contract that sits beside `SKILL.md`. | A replacement for skills. |
| A CLI that scores, ports, compiles, and records. | A new agent runtime or orchestration platform. |
| A way to make skills easier to compare across Codex, Claude, and Agents. | A promise that every harness will behave identically. |
| A run record you can audit after the task. | A security sandbox. |

That last row matters. SkillSpec makes a run **auditable**: you can see what was
claimed and check it against the contract. Enforcement of tool boundaries is
still the harness's job.

## Public Doctor Reports

Want to check a public skill before installing or porting it? Use the hosted
Doctor page:

**<https://skillspec.sh/>**

You can also open a
[Doctor report request](https://github.com/modiqo/skillspec/issues/new?template=doctor-report.yml)
with a public GitHub skill repo or folder URL. GitHub Actions validates the
target, runs `skillspec doctor`, comments with a Markdown report, and attaches
Markdown, HTML, JSON, and text artifacts.

Private repositories are not inspected by public Actions. For private skills,
install SkillSpec locally:

```bash
skillspec doctor /path/to/local/skill
skillspec doctor /path/to/local/skill --markdown > skillspec-doctor.md
skillspec doctor /path/to/local/skill --html > skillspec-doctor.html
```

Use Doctor as the baseline. Then ask your harness to import the skill:

```text
/skillspec import <skill-repo-or-folder>, compile it, verify it, test it, and prove it. Print the alignment summary.
```

Publish the baseline report, generated `skill.spec.yml`, compiled loader, and
alignment report with the repo or pull request so reviewers can see both the
original skill risk and the proof after porting.

## Why The Scores Are Credible

Doctor is not vibes. Every risk condition cites published work or local
SkillSpec methodology on how agents fail: context-position effects, effective
context limits, verifiable instruction following, process-level agent
evaluation, and skill-metadata routing.

The report is explicit about what is measured versus what is a policy
threshold. Start here:

- [Doctor Agent Drift Risk](https://github.com/modiqo/skillspec/blob/main/docs/design/22-doctor-agent-drift-risk.md)
- [Why SkillSpec](https://github.com/modiqo/skillspec/blob/main/docs/01-why-skillspec.md)
- [Contract Trace Methodology](https://github.com/modiqo/skillspec/blob/main/docs/08-contract-trace-methodology.md)

The contract itself is a real spec: a typed Rust model, JSON Schema, reference
grammar, and
[conformance suite](https://github.com/modiqo/skillspec/tree/main/conformance).

## Learn More

- [How it works](https://github.com/modiqo/skillspec/blob/main/docs/design/README.md)
- [Command reference](https://github.com/modiqo/skillspec/blob/main/docs/design/16-command-log.md)
- [Plugin marketplace install](https://github.com/modiqo/skillspec/blob/main/docs/design/26-plugin-marketplace-install.md)
- [Request a public Doctor report](https://github.com/modiqo/skillspec/issues/new?template=doctor-report.yml)
- [Contributing](https://github.com/modiqo/skillspec/blob/main/CONTRIBUTING.md)

## License

SkillSpec is dual-licensed under either:

- [MIT](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

You may choose either license. Contributions are accepted under the same dual
license unless explicitly stated otherwise.
