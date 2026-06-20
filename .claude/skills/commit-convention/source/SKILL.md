---
name: commit-convention
description: Use when writing commit messages, creating PR titles, pushing commits, or creating PRs — ensures conventional commit format and that formatting/linting pass before any push
---

# Commit Convention

This project uses [Conventional Commits](https://www.conventionalcommits.org/). PR titles are linted by CI and become the squash-merge commit message on `main`.

## Pre-push Gate

Before pushing commits or creating a PR, run **both** checks and confirm they pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

If either fails, fix the issues and amend/re-commit before pushing. Only skip these checks if the plan explicitly allows for it.

## Format

```
<type>[optional scope]: <description>
```

## Allowed Types

- **feat** — new feature
- **fix** — bug fix
- **docs** — documentation only
- **style** — formatting, no code change
- **refactor** — neither fix nor feature
- **perf** — performance improvement
- **test** — adding or updating tests
- **build** — build system or dependencies
- **ci** — CI configuration
- **chore** — maintenance tasks
- **revert** — revert a previous commit

## Rules

- Subject must NOT start with an uppercase letter
- Scope is optional and freeform (e.g., `feat(adapter): ...`)
- Breaking changes: use `!` after type (e.g., `feat!: ...`) or add `BREAKING CHANGE:` footer

## Examples

```
feat: add semantic search for skills
fix(grpc): handle timeout on large payloads
ci: add conventional commits PR title linting
feat!: redesign adapter API
```
