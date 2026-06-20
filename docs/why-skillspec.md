# Why SkillSpec

SkillSpec exists because prose-only agent skills are useful but hard to reuse with confidence.

The core story:

- Agent Skills define what to load.
- MCP defines what tools and data are available.
- SkillSpec defines how the agent should decide, verify, and report behavior.

The tagline is intentionally narrow:

> Keep the prose. Structure the decisions.

SkillSpec does not argue that prose is bad. Prose is still the right medium for tone, context, domain judgment, examples, and human explanation. The problem is that critical behavior often hides inside paragraphs: route choices, forbidden substitutions, dependency assumptions, elicitation points, safety rules, and success conditions.

## Why Structure Helps

Structured behavior contracts make the important parts inspectable:

- Routes and rules can be tested.
- Dependencies can be checked before execution.
- Elicitations become bounded questions instead of open-ended guessing.
- Traces make decision paths reviewable after the run.
- Compiler targets can generate thin harness loaders without creating a second source of truth.
- Importers can preserve original prose while marking uncertainty as `review_required`.

This follows the same broad lesson that made OpenAPI useful for REST APIs: formal descriptions reduce guesswork and enable code, docs, tests, and review to share one contract.

## Why Prose Alone Has Gaps

Research and platform guidance point to several recurring limits in prose-only prompting:

- Model behavior can be sensitive to prompt wording and formatting, even when meaning is preserved. See Sclar et al., ["Quantifying Language Models' Sensitivity to Spurious Features in Prompt Design"](https://arxiv.org/abs/2310.11324).
- Long contexts can bury relevant instructions. See ["Lost in the Middle"](https://arxiv.org/abs/2307.03172).
- Prompt injection is a live risk when instructions and untrusted content share the same context. OWASP recommends constrained output formats, validation, least privilege, human approval for high-impact actions, and adversarial testing in its [LLM01 Prompt Injection guidance](https://genai.owasp.org/llmrisk/llm01-prompt-injection/).
- Structured output work shows the value of schema-backed contracts when applications need reliable machine-readable behavior. OpenAI's [Structured Outputs](https://openai.com/index/introducing-structured-outputs-in-the-api/) announcement frames schemas as a way to make outputs adhere to developer-supplied structure.

SkillSpec applies that lesson to skills: keep the human instructions, but move the high-stakes behavioral contract into a format that validators, tests, compilers, and traces can understand.

For a demo-ready side-by-side comparison, see
[docs/why_skillspec/README.md](why_skillspec/README.md). It includes a
prose-only `SKILL.md`, a SkillSpec-backed loader, a runnable `skill.spec.yml`,
and a table designed for launch/demo material.

## What SkillSpec Is Not

SkillSpec is not:

- A security boundary by itself.
- A replacement for harness policy.
- A workflow engine.
- A universal ontology for every agent behavior.
- A claim that prompt injection is solved.

It is a portable contract that makes the behavior of a skill easier to inspect, test, compile, and discuss.

## Adoption Standard

A SkillSpec-backed skill should pass this minimum gate before people rely on it:

```sh
skillspec validate skill.spec.yml
skillspec imports check skill.spec.yml
skillspec test skill.spec.yml
skillspec deps check skill.spec.yml
```

For runtime use, a traced decision should be available:

```sh
skillspec decide skill.spec.yml --input='<user task>' --trace-dir .skillspec/traces
```
