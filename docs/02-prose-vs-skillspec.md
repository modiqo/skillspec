# Prose Skills Vs SkillSpec-Backed Skills

SkillSpec is not a rejection of prose. It is a way to separate human guidance from machine-checkable behavior.

## Comparison

| Concern | Prose-only skill | SkillSpec-backed skill |
| --- | --- | --- |
| Tone and domain context | Natural and easy to read | Still written in prose resources and snippets |
| Routing decisions | Embedded in paragraphs | Explicit `routes` and `rules` |
| Forbidden substitutions | Easy to miss | Explicit `forbid` entries |
| User questions | Often open-ended | Bounded `elicitations` with choices |
| Dependencies | Implied by examples | Declared and checkable |
| Tests | Usually absent | Scenario tests in the spec |
| Traces | Harness-specific or missing | Declared trace requirements and compactable output |
| Portability | Depends on prompt interpretation | Compiler targets can generate harness loaders |
| Review | Reviewers read everything manually | Reviewers inspect schema, tests, diffs, and traces |

## When Prose Is Enough

Prose-only skills can be enough when:

- The skill is personal and low-risk.
- The behavior is mostly tone or domain explanation.
- There are no important dependencies, routes, or safety choices.
- Nobody needs to port the skill across harnesses.

## When SkillSpec Helps

SkillSpec is worth the overhead when:

- A skill will be shared publicly or across teams.
- Different harnesses should load equivalent behavior.
- The skill chooses among tools, routes, or data sources.
- A wrong substitution would be harmful or expensive.
- Dependencies or credentials need review.
- Maintainers want tests before accepting changes.
- Users need an audit trail of why the agent chose a path.

## Migration Path

```sh
skillspec import-skill path/to/SKILL.md --out skill.spec.yml
skillspec validate skill.spec.yml
skillspec imports check skill.spec.yml
skillspec test skill.spec.yml
skillspec deps check skill.spec.yml
skillspec compile skill.spec.yml --target codex-skill > SKILL.md
```

The importer is intentionally conservative. It preserves source material,
extracts obvious structure, materializes fenced code into package-local
resources, and adds `review_required` notes. A human or agent should then
promote the important prose into rules, dependencies, recipes, commands, and
tests using only the current source, grammar guidance, and explicit
user-approved references.
