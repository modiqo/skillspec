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
# Local source path or public GitHub skill URI:
skillspec source map <source-skill-or-github-uri> --out <draft>/.skillspec/source-map
skillspec source coverage <draft>/.skillspec/source-map/source-map.json
skillspec source lens <draft>/.skillspec/source-map/source-map.json --cursor 1
skillspec source query <draft>/.skillspec/source-map/source-map.json nodes --view index
skillspec source query <draft>/.skillspec/source-map/source-map.json dependencies --view summary
skillspec source query <draft>/.skillspec/source-map/source-map.json code --view summary
skillspec source stale <draft>/.skillspec/source-map/source-map.json --root <source-skill>
skillspec import-skill <source-skill> --out <draft>/skill.spec.yml --source-map <draft>/.skillspec/source-map/source-map.json
```

For URI imports, `source map` owns the normal sparse checkout path. It stages
the public GitHub source through the same parser used by `source stage`, maps
the selected local source, and reports `source_path` for the later `stale` and
`import-skill` commands. When a repo URI contains multiple `SKILL.md`
candidates, the command refuses to guess and prints the candidate source paths.
Use `source stage` explicitly when the task is candidate discovery, when the
user wants a persistent checkout before mapping, or when staging needs
troubleshooting. Web search, raw GitHub fetches, and ad hoc sparse-checkout
probing remain troubleshooting paths only after SkillSpec staging fails and the
user approves.

Agents should query exact handles with `--view full` when they need the source
span for a heading, code block, dependency mention, local reference, or modal
obligation. A full file read remains acceptable only for bounded small sources
after the source map shows that no sibling material affects the import.

`source lens` is the progressive promotion view. It walks parsed Markdown blocks
in deterministic order and defaults to one unit per call. Each unit includes:

- review position and remaining count;
- source node id, line range, preview, and source hash;
- attached classifications and references;
- suggested SkillSpec constructs;
- required target kinds for proof, such as `rule`, `resource`, `code`, or
  `dependency`.

The intended loop is: inspect one lens unit, port that unit into matching
SkillSpec constructs, validate the package draft, record the unit hash and
target in promotion proof, then advance to the returned `next_cursor`. This is
deliberately different from a bulk edit that rewrites every scaffold and points
all source obligations at one generic route.

Conditional workflow language is a first-class review signal. Parsed text with
phrases such as `if`, `when`, `unless`, `only if`, or `provided that` is emitted
as `conditional_rule_candidate` and requires a structural `rule` target in
promotion proof.

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
- `crates/skillspec-doctor/src/source_map.rs` for map schema and query behavior;
- `crates/skillspec-core/src/spec/grammar.rs` for the import sequence taught by
  `skillspec grammar sensemake`;
- `crates/skillspec-cli/src/features/sensemake.rs` for navigation hints;
- `spec/commandspec.md` for the formal command inventory;
- `docs/design/operations/16-command-log.md` for the quick command log;
- `skills/skillspec/source/SKILL_md.old` and `skills/skillspec/skill.spec.yml` for
  prompt-driven multiplexer behavior;
- CLI tests in `crates/skillspec-cli/tests/cli/`.

## Quality Bar

The source map is evidence, not scratch. A high-quality import should show:

- the source package was staged locally when remote;
- `source-map.json` and `source-map.md` were produced;
- `source coverage`, `source lens`, and relevant `source query` handles were
  inspected;
- promotion proof includes lens source hashes and construct-compatible targets
  for every promoted source obligation;
- dependency and code classifications were reviewed before proof or install;
- `source stale` passed before import;
- `import-skill --source-map` was used;
- `deps.toml` preserves package authority, risk, local status, install
  candidates, and degraded proof impact.
