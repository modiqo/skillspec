# Community Outreach

The launch goal is to get concrete ports, not generic attention. Ask people to try SkillSpec on one real skill and report what the grammar cannot express.

## Positioning

Use this framing consistently:

- Agent Skills define what to load.
- MCP defines what tools and data are available.
- SkillSpec defines how the agent should decide, verify, and report behavior.

Short tagline:

```text
Keep the prose. Structure the decisions.
```

Avoid saying prose is obsolete or that SkillSpec solves prompt injection. The claim is narrower: structured contracts make skill behavior easier to test, port, review, and trace.

## Communities To Reach

Start with communities that already feel the pain of reusable agent behavior:

- Agent Skills and Claude Code users.
- Codex and OpenAI agent builders.
- MCP builders and adapter authors.
- LangChain, LlamaIndex, DSPy, AutoGen, and CrewAI users.
- Prompt-engineering and AI security communities.
- SWE-agent and coding-agent researchers.
- OpenAPI and standards-minded developer communities.

## Specific Ask

Use the same ask everywhere:

```text
Port one real skill and tell us what SkillSpec cannot express.
```

Ask for:

- Link to the original skill.
- Target harness.
- Decisions that were hard to encode.
- Ambiguous dependencies.
- Scenario tests that would prove the port works.
- Compiler output that felt wrong.

## Launch Checklist

- Open GitHub Discussions with the categories in `DISCUSSIONS.md`.
- Pin the RFC from `docs/rfc-v0.md`.
- File the starter issues from `docs/good-first-issues.md`.
- Share the before/after example from `examples/before-after/`.
- Link the minimum compliance gate: `validate`, `test`, and `deps check`.
- Ask for ports, not opinions in the abstract.

## Response Pattern

When someone reports a missing concept:

1. Ask for the smallest source-skill excerpt that demonstrates the issue.
2. Ask what scenario test would prove the behavior.
3. Decide whether it belongs in strict grammar, an explicit extension surface, importer review notes, or harness-specific policy.
4. Add a valid or invalid conformance fixture when the answer changes validation behavior.
