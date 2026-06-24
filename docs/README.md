# SkillSpec Docs

This directory contains explanatory project docs. The formal contract remains in
`../spec/`; the docs here explain purpose, comparisons, launch materials, and
design rationale.

Read the numbered root docs in filename order for the full narrative.
Subdirectories keep their own indexes.

## Reader Paths

Use these paths when you do not need the whole sequence:

| Reader | Start Here | Then Read |
| --- | --- | --- |
| Newcomer | [Reliability Gap](00-skills-reliability-gap.md) | [Why SkillSpec](01-why-skillspec.md), [Prose Skills Vs SkillSpec-Backed Skills](02-prose-vs-skillspec.md), [visual explainers](design/explained/README.md) |
| Skill author | [Why SkillSpec](01-why-skillspec.md) | [Prompt Multiplexer Test Plan](07-prompt-multiplexer-test-plan.md), [Import To Release](design/explained/01-import-to-release.md), [Skill Authoring Lifecycle](design/04-skill-authoring-lifecycle.md), [Source Map Progressive Reader](design/18-source-map-progressive-reader.md) |
| CLI tester | [Prompt Multiplexer Test Plan](07-prompt-multiplexer-test-plan.md) | [Command Log](design/16-command-log.md), [Design Documentation QA Process](design/17-qa-process.md) |
| Maintainer | [Design docs](design/README.md) | [Contract And Trace Methodology](08-contract-trace-methodology.md), [Command Log](design/16-command-log.md), [spec reference](../spec/README.md), [conformance fixtures](../conformance/) |
| Visual reviewer | [Visual explainers](design/explained/README.md) | [Grammar atlas](grammar-atlas/README.md), [Grammar And Conformance](design/02-grammar-and-conformance.md) |

## Reading Order

| Order | Doc | Purpose |
| --- | --- | --- |
| 00 | [The Reliability Gap In Agent Skills](00-skills-reliability-gap.md) | Research framing for why prose skills need a checkable reliability layer. |
| 01 | [Why SkillSpec](01-why-skillspec.md) | The core motivation and problem statement. |
| 02 | [Prose Skills Vs SkillSpec-Backed Skills](02-prose-vs-skillspec.md) | A direct comparison between prose-only skills and SkillSpec-backed skills. |
| 03 | [RFC: SkillSpec v0](03-rfc-v0.md) | The announcement-style RFC for the v0 contract. |
| 04 | [Community Outreach](04-community-outreach.md) | Launch positioning, communities, and outreach steps. |
| 05 | [Good First Issues](05-good-first-issues.md) | Starter work for contributors. |
| 06 | [Community Post Drafts](06-community-posts.md) | Short-form launch and discussion drafts. |
| 07 | [Prompt Multiplexer Test Plan](07-prompt-multiplexer-test-plan.md) | Reference runbook for testing the `/skillspec` setup multiplexer in Codex and Claude. |
| 08 | [Contract And Trace Methodology](08-contract-trace-methodology.md) | Measurement methodology for static well-formedness, behavioral contracts, trace alignment, and unproven verdicts. |

## Subdirectories

- [Design docs](design/README.md): numbered maintainer docs for the contract,
  runtime model, router mode, command surface, QA process, and the visual
  explainers in `design/explained/`.
- [Grammar atlas](grammar-atlas/README.md): visual plates for the grammar,
  reference graph, semantics, and a worked example.
- [Why SkillSpec demo package](why-skillspec-demo/README.md): runnable
  side-by-side material that supports `01-why-skillspec.md`.

## Source Of Truth

When these docs disagree with implementation or reference material, prefer:

1. Rust model, parser, CLI, and tests in `../crates/skillspec-cli/`.
2. Schema and reference docs in `../spec/`.
3. Conformance fixtures in `../conformance/`.
4. Examples in `../examples/`.
5. Explanatory docs in this directory.
