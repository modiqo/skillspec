# Source Map Progressive Reader

SkillSpec imports need to handle large prose skills, sibling reference files,
code fences, scripts, assets, and dependency mentions without forcing the agent
to load the whole source package into context. The source-map workflow gives the
agent a deterministic map first, then exact handles for targeted reads.

## Problem

The old import loop depended on broad file reads:

```bash
sed -n '1,999p' <source-skill-folder>/SKILL.md
skillspec import-skill <source-skill-folder> --out <draft>/skill.spec.yml
```

That works for small files, but it is brittle for real skills:

- large `SKILL.md` files can consume context before the agent has a section map;
- referenced files can be missed or flattened into prose;
- code fences and sibling scripts can lose provenance;
- dependency mentions can be skipped, softened, or treated as proof-only details;
- source edits between mapping and import can make evidence stale.

## Command Flow

For imports, the expected flow is:

```bash
# URI sources only:
skillspec source stage <github-skill-uri> --out <staging-root> --json

# Local source path or selected_source_path from source stage:
skillspec source map <source-skill> --out <draft>/.skillspec/source-map
skillspec source coverage <draft>/.skillspec/source-map/source-map.json
skillspec source query <draft>/.skillspec/source-map/source-map.json nodes --view index
skillspec source query <draft>/.skillspec/source-map/source-map.json dependencies --view summary
skillspec source query <draft>/.skillspec/source-map/source-map.json code --view summary
skillspec source stale <draft>/.skillspec/source-map/source-map.json --root <source-skill>
skillspec import-skill <source-skill> --out <draft>/skill.spec.yml --source-map <draft>/.skillspec/source-map/source-map.json
```

For URI imports, `source stage` is the authoritative URI-to-local-source gate.
Agents should use its `selected_source_path` when there is one candidate, or ask
the user to choose from `candidates[].source_path`. Web search, raw GitHub
fetches, and ad hoc sparse-checkout probing are troubleshooting paths only after
`source stage` fails and the user approves.

Agents should query exact handles with `--view full` when they need the source
span for a heading, code block, dependency mention, local reference, or modal
obligation. A full file read remains acceptable only for bounded small sources
after the source map shows that no sibling material affects the import.

## Parser Choice

The implementation uses the Rust `markdown` crate (`markdown-rs`) as the primary
Markdown reader. It provides a Markdown AST with position information and serde
support, which lets SkillSpec record byte ranges, line ranges, text previews,
frontmatter, code blocks, and references. Heuristic string scans are limited to
classifying parsed spans, such as dependency phrases or package imports inside
fenced code.

For oversized Markdown files, SkillSpec uses a chunked fallback instead of
building one full AST. The fallback records heading, code, and paragraph-chunk
nodes with byte ranges, line ranges, hashes, previews, dependency/modal
classifications, and Markdown link references. This keeps source-map generation
bounded for large skills while preserving handles for targeted review.

Frontmatter is handled before Markdown parsing so leading `---` metadata is
preserved as a `frontmatter:<file>` node instead of being interpreted as a
thematic break and heading.

## Source Map Shape

`source-map.json` records:

- source root and generator metadata;
- file records with path, hash, byte count, line count, kind, roles, and load
  status;
- Markdown nodes with stable ids, kind, parent/children, byte ranges, line
  ranges, language, title, hash, and preview;
- classifications for modal obligations, dependency mentions, code blocks, and
  imported package candidates;
- local and external references;
- coverage counts and review-required totals.

`source-map.md` is the human review companion for quick inspection.

## Import Gate

`skillspec import-skill --source-map <source-map.json>` checks that every mapped
file still matches its recorded hash before import. If any file is stale or
missing, import fails and the agent must regenerate the source map.

This gate prevents a common failure mode where the agent maps one version of a
skill, edits or restages the source, then imports from a different version while
claiming the old evidence still applies.

## Critical Junctions

Keep these surfaces aligned whenever the source-map workflow changes:

- `crates/skillspec-cli/src/cli/args/` for command shape and help text;
- `crates/skillspec-cli/src/features/source_map.rs` for map schema and query behavior;
- `crates/skillspec-core/src/spec/grammar.rs` for the import sequence taught by
  `skillspec grammar sensemake`;
- `crates/skillspec-cli/src/features/sensemake.rs` for navigation hints;
- `spec/commandspec.md` for the formal command inventory;
- `docs/design/16-command-log.md` for the quick command log;
- `skills/skillspec/source/SKILL_md.old` and `skills/skillspec/skill.spec.yml` for
  prompt-driven multiplexer behavior;
- CLI tests in `crates/skillspec-cli/tests/cli/`.

## Quality Bar

The source map is evidence, not scratch. A high-quality import should show:

- the source package was staged locally when remote;
- `source-map.json` and `source-map.md` were produced;
- `source coverage` and relevant `source query` handles were inspected;
- dependency and code classifications were reviewed before proof or install;
- `source stale` passed before import;
- `import-skill --source-map` was used;
- `deps.toml` preserves package authority, risk, local status, install
  candidates, and degraded proof impact.
