# SkillSpec Docs

This directory contains explanatory project docs. The formal contract remains in
`../spec/`; the docs here explain purpose, comparisons, launch materials, and
design rationale.

The docs are grouped by reader intent. Keep `docs/pages/` in place because it is
the GitHub Pages source; the other folders are maintained as the human-readable
documentation catalog.

## Reader Paths

Use these paths when you do not need the whole sequence:

| Reader | Start Here | Then Read |
| --- | --- | --- |
| Newcomer | [Detailed README](overview/README_DETAILED.md) | [Reliability Gap](overview/00-skills-reliability-gap.md), [Why SkillSpec](overview/01-why-skillspec.md), [Prose Skills Vs SkillSpec-Backed Skills](overview/02-prose-vs-skillspec.md), [visual explainers](visuals/explainers/README.md) |
| Skill author | [Why SkillSpec](overview/01-why-skillspec.md) | [Prompt Multiplexer Test Plan](runbooks/07-prompt-multiplexer-test-plan.md), [Import To Release](visuals/explainers/01-import-to-release.md), [Skill Authoring Lifecycle](design/authoring/04-skill-authoring-lifecycle.md), [Source Map Progressive Reader](design/authoring/18-source-map-progressive-reader.md) |
| CLI tester | [Prompt Multiplexer Test Plan](runbooks/07-prompt-multiplexer-test-plan.md) | [Command Log](design/operations/16-command-log.md), [Design Documentation QA Process](design/operations/17-qa-process.md) |
| Maintainer | [Design docs](design/README.md) | [Contract And Trace Methodology](overview/08-contract-trace-methodology.md), [Command Log](design/operations/16-command-log.md), [spec reference](../spec/README.md), [conformance fixtures](../conformance/) |
| Visual reviewer | [Visual explainers](visuals/explainers/README.md) | [Grammar atlas](visuals/grammar-atlas/README.md), [Grammar And Conformance](design/core/02-grammar-and-conformance.md) |

## Catalog

### Overview

| Doc | Purpose |
| --- | --- |
| [Detailed README](overview/README_DETAILED.md) | Full user-facing walkthrough. |
| [The Reliability Gap In Agent Skills](overview/00-skills-reliability-gap.md) | Research framing for why prose skills need a checkable reliability layer. |
| [Why SkillSpec](overview/01-why-skillspec.md) | Core motivation and problem statement. |
| [Prose Skills Vs SkillSpec-Backed Skills](overview/02-prose-vs-skillspec.md) | Direct comparison between prose-only skills and SkillSpec-backed skills. |
| [Contract And Trace Methodology](overview/08-contract-trace-methodology.md) | Measurement methodology for behavioral contracts, traces, and unproven verdicts. |

### Community

| Doc | Purpose |
| --- | --- |
| [RFC: SkillSpec v0](community/03-rfc-v0.md) | Announcement-style RFC for the v0 contract. |
| [Community Outreach](community/04-community-outreach.md) | Launch positioning, communities, and outreach steps. |
| [Good First Issues](community/05-good-first-issues.md) | Starter work for contributors. |
| [Community Post Drafts](community/06-community-posts.md) | Short-form launch and discussion drafts. |

### Runbooks

| Doc | Purpose |
| --- | --- |
| [Prompt Multiplexer Test Plan](runbooks/07-prompt-multiplexer-test-plan.md) | Reference runbook for testing the `/skillspec` setup multiplexer in Codex and Claude. |

### Design

- [Design docs](design/README.md): maintainer catalog for contract, authoring,
  runtime, router, and operations design records.
- [Core contract docs](design/core/): grammar, package anatomy, rules, states,
  imports, and tool boundaries.
- [Authoring docs](design/authoring/): import, source mapping, workspace
  authoring, one-shot porting, and shape-specific checklists.
- [Runtime docs](design/runtime/): sensemaking, execution loops, traces,
  alignment, capability bootstrap, trampoline, and progressive guidance.
- [Router docs](design/router/): router behavior, guard hooks, duplicate-root
  selection, execution policy boundaries, and policy profiles.
- [Operations docs](design/operations/): command log, QA process, performance,
  doctor risk, install/release, public reports, crate boundaries, and test
  matrices.

### Visuals And Examples

- [Visual explainers](visuals/explainers/README.md): diagram-first workflow
  explanations for import, runtime, router mode, and durable execution.
- [Grammar atlas](visuals/grammar-atlas/README.md): visual plates for the
  grammar, reference graph, semantics, and a worked example.
- [Why SkillSpec demo package](examples/why-skillspec-demo/README.md): runnable
  side-by-side material that supports
  [Why SkillSpec](overview/01-why-skillspec.md).

### Site Pages

- `pages/`: GitHub Pages source. Keep this folder shape stable for deployment.

## Source Of Truth

When these docs disagree with implementation or reference material, prefer:

1. Rust model, parser, CLI, and tests in `../crates/skillspec-cli/`.
2. Schema and reference docs in `../spec/`.
3. Conformance fixtures in `../conformance/`.
4. Examples in `../examples/`.
5. Explanatory docs in this directory.
