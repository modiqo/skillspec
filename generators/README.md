# Generators

SkillSpec generators compile a structured spec into harness-friendly artifacts.
They must be complete enough to test seriously. A generated skill is allowed to
be compact, but it must not be a shallow summary.

Current targets:

- `codex-skill`
- `claude-skill`
- `markdown`

Planned targets:

- `decision-table`
- `scenario-report`

## Completeness Bar

Generated harness guidance must include:

- frontmatter for skill discovery
- the runtime contract
- entry prompt
- activation hints
- ranked routes
- ordered rules with predicates, prefer, route_order, forbid, allow,
  after_success, and reason
- lifecycle states
- command templates with safety, requirements, parse hints, and success checks
- snippets
- closures
- scenario tests
- proof metrics
- review notes
- CLI commands for validate/test/decide/explain

The generated prose should be concise in wording, not incomplete in content.
The point is to remove ambiguous prose, not to hide the structured decisions.
