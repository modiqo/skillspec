# Command Templates

Command templates let a SkillSpec describe how to invoke tools without burying
the invocation in prose.

Example:

```yaml
commands:
  trace_cost:
    description: Collect trace and dependency cost after task completion.
    template: "rote trace --deps --format json"
    safety: read_only
    parse:
      tokens: "$.totals.tokens"
      estimated_cost_usd: "$.totals.estimated_cost_usd"
```

## Recommended Fields

- `description`
- `template`
- `safety`
- `requires`
- `cwd`
- `output`
- `parse`
- `success_when`

## Safety Classes

Suggested values:

- `read_only`
- `local_read`
- `local_write`
- `network_read`
- `network_write`
- `browser_attach`
- `credential_request`
- `destructive`

V0 does not enforce these classes, but generators and harnesses should surface
them before execution.

## Import-Skill Strategy

`skillspec import-skill SKILL.md --out skill.spec.yml` should not pretend to
understand all prose. It should:

1. parse frontmatter
2. extract headings
3. extract fenced commands
4. materialize fenced code into package-local `resources/imported-code/`
   files and reference them from `code.source.file`
5. extract tables
6. identify always/never/forbid/required language
7. identify examples and trigger phrases
8. write a scaffold with `review_required`

An agent-assisted pass may improve the scaffold, but uncertainty must remain
visible.
