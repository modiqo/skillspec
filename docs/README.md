# SkillSpec Docs

This directory contains explanatory project docs. The formal contract remains in
`../spec/`; the docs here explain purpose, comparisons, launch materials, and
design rationale.

Read the numbered root docs in filename order. Subdirectories keep their own
indexes.

## Reading Order

| Order | Doc | Purpose |
| --- | --- | --- |
| 01 | [Why SkillSpec](01-why-skillspec.md) | The core motivation and problem statement. |
| 02 | [Prose Skills Vs SkillSpec-Backed Skills](02-prose-vs-skillspec.md) | A direct comparison between prose-only skills and SkillSpec-backed skills. |
| 03 | [RFC: SkillSpec v0](03-rfc-v0.md) | The announcement-style RFC for the v0 contract. |
| 04 | [Community Outreach](04-community-outreach.md) | Launch positioning, communities, and outreach steps. |
| 05 | [Good First Issues](05-good-first-issues.md) | Starter work for contributors. |
| 06 | [Community Post Drafts](06-community-posts.md) | Short-form launch and discussion drafts. |

## Subdirectories

- [Design docs](design/README.md): numbered maintainer docs for the contract,
  runtime model, router mode, command surface, and QA process.
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
