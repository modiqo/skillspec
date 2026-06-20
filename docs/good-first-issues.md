# Good First Issues

These are ready-to-file starter issues. Each should stay small enough for a first contribution and include a clear verification command.

## Add JSON Schema Validation Test Against All Examples

Status: done in the current test suite. Future work can expand it to conformance fixtures outside `examples/`.

Expected verification:

```sh
cargo test --workspace --all-targets published_json_schema_validates_every_example
```

## Add Golden Snapshots For `compile --target markdown`

Status: initial snapshot added for `examples/repo-readiness.skill.spec.yml`.

Next starter task:

- Add a second markdown snapshot for a spec with resources, code, artifacts, and recipes.
- Keep the snapshot in `fixtures/golden/`.
- Add or extend an integration test that compares full output.

Expected verification:

```sh
cargo test --workspace --all-targets compiler_markdown_output_matches_golden_snapshot
```

## Add One New Example SkillSpec

Pick a real, small skill with a clear route decision. Good examples:

- Browser versus native search.
- Local file analysis versus network lookup.
- Dependency check before running a command.
- Bounded user elicitation before taking action.

Expected verification:

```sh
skillspec validate examples/<name>.skill.spec.yml
skillspec imports check examples/<name>.skill.spec.yml
skillspec test examples/<name>.skill.spec.yml
skillspec deps check examples/<name>.skill.spec.yml
```

## Add A Compiler Target

Add one target only. Keep it thin and avoid creating a second source of truth.

Suggested targets:

- A minimal human-readable checklist.
- A harness-specific loader for another agent environment.
- A trace-focused review report.

Expected verification:

```sh
cargo test --workspace --all-targets
skillspec compile examples/rote-computer.skill.spec.yml --target <target>
```

## Improve Importer Extraction

Pick one extraction rule:

- Better title and description extraction.
- Detect bounded choice lists as `elicitations`.
- Detect dependency mentions from install sections.
- Preserve source headings in review notes.

Expected verification:

```sh
cargo test --workspace --all-targets importer_output_matches_golden_snapshot
```

If output changes intentionally, update `fixtures/golden/import-fixtures-skill.spec.yml`.

## Write A Comparison Doc: Prose Skill Vs SkillSpec-Backed Skill

Status: initial version exists at `docs/prose-vs-skillspec.md`.

Next starter task:

- Add a real example from `examples/before-after/`.
- Keep claims narrow and avoid saying prose is obsolete.
- Link to the migration path.
