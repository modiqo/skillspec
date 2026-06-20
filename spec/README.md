# SkillSpec v0

SkillSpec v0 is a structured skill format for agent behavior. It is designed to
coexist with prose skills by moving decision-heavy behavior into a compact,
testable file.

V0 is intentionally small:

- no imports
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
- `commands`: command templates or command invocation instructions
- `snippets`: reusable prose
- `closures`: post-task behavior
- `tests`: scenario expectations
- `proof`: metrics the spec intends to improve
- `review_required`: uncertainties that require human review

See [semantics.md](semantics.md) for behavior and
[grammar.md](grammar.md) for the formal v0 grammar. See
[relationships.md](relationships.md) for how the concepts associate and
[skill.spec.schema.json](skill.spec.schema.json) for a permissive v0 schema.
