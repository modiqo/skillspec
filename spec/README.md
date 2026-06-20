# SkillSpec v0

SkillSpec v0 is a structured skill format for agent behavior. It is designed to
coexist with prose skills by moving decision-heavy behavior into a compact,
testable file.

V0 is intentionally small:

- explicit local imports only
- no inheritance
- no execution engine
- no arbitrary expression language
- no hidden network or tool execution

The v0 file should be readable by humans, navigable by agents, and testable by a
small CLI.

## Required Fields

- `schema`: must be `skillspec/v0`
- `id`: stable reverse-DNS-like or dotted identifier
- `title`: human title
- `description`: short purpose statement

## Recommended Sections

- `applies_when`: activation hints
- `routes`: candidate ways to satisfy a task
- `rules`: intent and guard rules
- `states`: state machine for the skill
- `trace`: append-only decision event contract
- `dependencies`: declared tools, files, env vars, services, adapters, browsers,
  packages, checks, permissions, and provision choices
- `imports`: runtime-loadable local guidance such as shared policy,
  branch-specific references, procedures, examples, and skill docs
- `resources`: provenance, supporting material, and non-runtime files preserved
  from imported multi-file skills
- `code`: fenced snippets or scripts with provenance, requirements, safety, and
  artifact links
- `artifacts`: named files or data products consumed and produced by behavior
- `recipes`: ordered procedures that bind imports, resources, code, commands,
  artifacts, and elicitations
- `commands`: command templates or command invocation instructions
- `snippets`: reusable prose
- `closures`: post-task behavior
- `tests`: scenario expectations
- `proof`: metrics the spec intends to improve
- `review_required`: uncertainties that require human review

See [semantics.md](semantics.md) for behavior and
[grammar.md](grammar.md) for the formal v0 grammar. See [imports.md](imports.md)
for import resolution, section loading, and nesting rules. See
[relationships.md](relationships.md) for how the concepts associate,
[trace.md](trace.md) for event logs, and [skill.spec.schema.json](skill.spec.schema.json)
for the strict v0 schema.
