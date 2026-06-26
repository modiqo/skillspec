# Community Post Drafts

These are short drafts for launch posts, discussions, newsletters, or forum threads.

## Prose Skills Are Good. Behavior Contracts Make Them Portable.

Agent skills are often written as Markdown because Markdown is the right starting point: it is readable, easy to edit, and good at carrying context.

The problem is not prose. The problem is that critical behavior gets buried in prose.

SkillSpec keeps the prose, but moves the decisions into a structured contract: routes, rules, dependencies, elicitations, tests, and traces. A generated `SKILL.md` can stay thin while `skill.spec.yml` becomes the source of truth.

The goal is simple:

- Agent Skills define what to load.
- MCP defines what tools and data are available.
- SkillSpec defines how the agent should decide, verify, and report behavior.

We are looking for feedback on the v0 grammar. The best way to help: port one real skill and tell us what the spec cannot express.

## Turning A Markdown Skill Into A Tested SkillSpec

A prose skill can explain intent, but it usually cannot prove behavior.

SkillSpec adds the missing testable layer:

```sh
skillspec port-one-shot path/to/SKILL.md --out ./draft --target codex-skill --prove
```

The lower-level commands remain available when you need to inspect one gate:

```sh
skillspec source map path/to/SKILL.md --out .skillspec/source-map
skillspec source coverage .skillspec/source-map/source-map.json
skillspec import-skill path/to/SKILL.md --out skill.spec.yml --source-map .skillspec/source-map/source-map.json
skillspec validate skill.spec.yml
skillspec imports check skill.spec.yml
skillspec test skill.spec.yml
skillspec deps check skill.spec.yml
```

The importer is conservative. It preserves source material, extracts obvious commands and code blocks, and marks uncertainty as `review_required`. From there, a maintainer promotes important decisions into routes, rules, dependencies, recipes, and scenario tests.

This is not about deleting prose. It is about making the important parts reviewable and portable.

We would love examples from real skills, especially cases where the importer cannot express a decision cleanly yet.

## Why Agent Skills Need Traces

When an agent uses a skill, the final answer is not enough. Maintainers need to know why the agent chose a route, which rules matched, which dependencies mattered, and what evidence was produced.

SkillSpec makes traces part of the contract:

```sh
skillspec decide skill.spec.yml --input='<user task>' --trace-dir .skillspec/traces
skillspec trace compact .skillspec/traces/<run-id>
```

That gives reviewers a decision path instead of a guess. It also gives test authors something concrete to compare when behavior changes.

Traces do not solve safety by themselves. They make behavior inspectable, which is the first step toward better tests, better reviews, and better shared skills.
