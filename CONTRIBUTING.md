# Contributing To SkillSpec

SkillSpec is a behavior contract for agent skills. Contributions should make the grammar clearer, the CLI more reliable, the examples more realistic, or the migration path from prose skills easier to trust.

## Where SkillSpec Fits

- Agent Skills define what to load.
- MCP defines what tools and data are available.
- SkillSpec defines how the agent should decide, verify, and report behavior.

Keep that boundary crisp. SkillSpec should not become a workflow engine, a package manager, or a replacement for all prose.

## Local Setup

```sh
cargo build
cargo test --workspace --all-targets
```

During development, run the local CLI directly:

```sh
./target/debug/skillspec --help
./target/debug/skillspec validate examples/rote-computer.skill.spec.yml
```

## Preflight

Before opening a PR, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
```

For changed examples, also run:

```sh
skillspec validate path/to/skill.spec.yml
skillspec test path/to/skill.spec.yml
skillspec deps check path/to/skill.spec.yml
```

The CI workflow runs the same categories of checks. It provides small command stubs for example-only external CLIs so example dependency declarations can be checked consistently in GitHub Actions.

CI also runs locked native build and test jobs on Linux, macOS, and Windows so
platform assumptions are caught before release.

## Changing The Spec

Spec changes should update these surfaces together:

- `spec/grammar.md` and related spec docs.
- `spec/skill.spec.schema.json`.
- Rust model parsing and validation in `crates/skillspec-cli/src/`.
- At least one valid example and one negative or strictness test when the change affects validation.
- Compiler output snapshots if rendering changes.

Unknown fields are rejected across typed grammar sections. If you need extensibility, add an explicit extension surface and document why it is intentionally open.

## Golden Snapshots

Compiler and importer output snapshots live in `fixtures/golden/`. They are ordinary files on purpose. If behavior changes intentionally:

1. Run the command that produces the output.
2. Review the full diff.
3. Update the golden file in the same PR.

Snapshot changes should explain whether the change is cosmetic, semantic, or a compatibility break.

## Good First Contributions

Start with [docs/good-first-issues.md](docs/good-first-issues.md). The best early contributions add one focused example, one compiler target, one importer improvement, or one conformance fixture with clear expected behavior.

## Security And Safety

SkillSpec helps constrain, test, and audit agent behavior. It is not a complete security boundary. Treat dependency installation, credentials, browser attachment, network writes, and destructive commands as explicit permission paths.

Report sensitive security issues privately to the maintainers before opening a public issue.
