---
name: generic-commit-convention
description: "Write conventional commit messages and run the repository's pre-push checks before commit, push, or PR title work."
---

# Commit Convention

Write conventional commit messages and run the repository's pre-push checks before commit, push, or PR title work.

This skill is a thin loader for the colocated `skill.spec.yml`. The spec is the source of truth for routes, rules, dependencies, resources, recipes, tests, and trace requirements.

## Runtime Contract

1. Load `./skill.spec.yml` from this skill folder before taking task actions.
2. When the `skillspec` CLI is available, run:

   ```bash
   skillspec decide ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
   ```

3. Strip skill invocation prefixes such as `/my-skill`, `$my-skill`, or `/rote-shell-spec` before passing `--input`.
4. Preserve the emitted trace `run_dir`; mention it in the completion report so the decision path can be inspected.
5. Follow the selected route, matched rules, forbids, elicitations, dependencies, recipes, and closures from `skill.spec.yml`.
6. If the CLI is unavailable, read `skill.spec.yml` directly and apply its rules manually. Do not expand this loader into a second source of truth.

## Quick Commands

```bash
skillspec validate ./skill.spec.yml
skillspec test ./skill.spec.yml
skillspec deps check ./skill.spec.yml
skillspec explain ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
```

## Route Hints

- `message_only`: Draft commit message only
- `pre_push_gate`: Run pre-push checks
- `commit_ready`: Prepare commit-ready output

