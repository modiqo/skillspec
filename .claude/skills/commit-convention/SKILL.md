---
name: generic-commit-convention
description: "Write conventional commit messages and run the repository's pre-push checks before commit, push, or PR title work."
---

# Commit Convention

Write conventional commit messages and run the repository's pre-push checks before commit, push, or PR title work.

This skill was generated from a SkillSpec. Use this document as the loaded harness guidance, and treat the referenced structured decisions as the behavioral contract.

## Runtime Contract

- Read this generated skill for orientation and immediate rules.
- Treat routes, rules, states, commands, tests, and review notes below as authoritative.
- Rules beat prose when there is tension.
- `forbid` entries are hard negative steering, not suggestions.
- `elicit` entries require bounded user questions before guessing.
- Use the scenario tests as examples of expected behavior.
- When the `skillspec` CLI is available, prefer `skillspec decide` or `skillspec explain` over manual interpretation.
- When invoking `skillspec decide`, pass only the user's task text. Strip skill invocation prefixes such as `/rote-shell-spec`, `$rote-shell-spec`, or `/my-skill` before setting `--input`.
- Prefer `--input='<task text>'` in shell examples so `$skill-name` text is not expanded by the shell.
- Resolve `skill.spec.yml` relative to this `SKILL.md` folder, not the process working directory.
- Always pass `--trace-dir`; use `${PWD}/.skillspec/traces` unless the user or harness provides a run-specific trace directory.
- After `skillspec decide` prints trace lines, keep the emitted `run_dir` and mention it when reporting how the decision was made.

## Entry

Prompt: Decide whether the user needs a message only, a pre-push gate, or a complete commit/push preparation loop.

## Applies When

```yaml
user_intent:
- write a commit message
- create a conventional commit
- prepare to push a branch
- create or update a pull request title
```

## Routes

Try lower-rank routes first unless matching rules override the route or route order.

### `message_only`
- label: Draft commit message only
- rank: 10
- description: Use when the user only asks for a commit message or PR title.

### `pre_push_gate`
- label: Run pre-push checks
- rank: 20
- description: Use when the user is about to commit, push, or create a PR.

### `commit_ready`
- label: Prepare commit-ready output
- rank: 30
- description: Use after checks pass to provide the final conventional commit subject.

## Rules

Evaluate rules in order. A matching rule may choose a route, replace route order, forbid substitutions, allow narrow fallbacks, request bounded elicitation, and schedule post-success actions.

### `commit_message_uses_conventional_format`
- when:
  - user_says_any: "commit message", "conventional commit", "pr title", "pull request title"
- prefer: `message_only`
- forbid: `uppercase_subject_start`, `co_authored_by_footer`
- after_success: `validate_commit_subject`
- reason: Commit and PR subjects must use Conventional Commits and avoid generated co-author footers.

### `push_or_pr_requires_pre_push_gate`
- when:
  - user_says_any: "push", "before pushing", "create pr", "open pr", "ready to commit"
- prefer: `pre_push_gate`
- forbid: `push_without_checks`, `skip_check_claim_without_evidence`
- after_success: `run_pre_push_checks`, `validate_commit_subject`
- reason: Push and PR work should not proceed until the configured formatting and lint checks pass.

### `failing_checks_block_commit_ready_claim`
- when:
  - user_says_any: "check failed", "clippy failed", "fmt failed"
- route_order: `pre_push_gate` -> `message_only`
- forbid: `commit_ready_claim`
- reason: Failed checks must be fixed or explicitly reported before declaring the branch ready.

## Decision Trace

When the `skillspec` CLI or a compatible harness evaluates this spec, record the decision path as append-only events. Rules trigger decisions; the evaluator writes the trace.

- mode: `event_log`
- required: true
- record: `input_received`, `spec_loaded`, `rule_evaluated`, `rule_matched`, `route_selected`, `after_success_scheduled`, `outcome_recorded`

## Dependencies

Check declared dependencies before using commands that require them. Missing dependencies must be handled through the declared provision or elicitation path; do not silently install global tools.

### `cargo`
- kind: `cli`
- description: Cargo is used by the source skill's Rust pre-push gate.
- command: `cargo`
- check:
  - command: `cargo`
- permission:
  - required: true
  - reason: Cargo checks may execute project build scripts and compiler plugins.
  - safety: `local_read`

## Resources

Resources are source material and provenance, not hidden control flow. Use structured routes, rules, code, commands, and recipes for behavior.

### `source_skill`
- path: `source/SKILL.md`
- role: `source_material`
- description: Original prose skill with allowed commit types, format rules, and pre-push gate.
- used_by:
  - route: `message_only`
  - route: `pre_push_gate`
  - command: `cargo_fmt_check`
  - command: `cargo_clippy_check`

## Recipes

Recipes are ordered procedures with explicit resource, dependency, code, command, elicitation, and artifact references.

### `run_pre_push_checks`
- description: Execute the source skill's pre-push gate before push or PR operations.
- ordered: true
- steps:
  - load_resource: `source_skill`
  - run_command: `cargo_fmt_check`
  - run_command: `cargo_clippy_check`
  - note: Report any failure and do not claim the branch is ready until the checks pass.

## Command Templates

Command templates are examples and contracts, not automatic permission. Apply the safety class and the harness approval policy before executing.

### `cargo_clippy_check`
- description: Run Rust lint checks with warnings denied.
- safety: `local_read`
- template:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
- requires:
  - dependencies: `cargo`

### `cargo_fmt_check`
- description: Check Rust formatting without mutating files.
- safety: `read_only`
- template:

```bash
cargo fmt --all -- --check
```
- requires:
  - dependencies: `cargo`

## Snippets

### `allowed_types`
Allowed types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert.

### `commit_format`
<type>[optional scope]: <description>

## Closures

Closures are post-task obligations or named lifecycle actions. Run them when referenced by states or `after_success`.

### `run_pre_push_checks`
```yaml
description: Run the pre-push gate and report pass/fail evidence.
```

### `validate_commit_subject`
```yaml
description: 'Ensure the final subject is `<type>[optional scope]: <description>`, lowercase after the prefix, and has no co-author footer.'
```

## Scenario Tests

Use these as behavioral examples. The agent should make the same routing and guardrail choices for equivalent tasks.

### commit message request drafts message route
- input: "write a commit message for this diff"
- expect route: `message_only`
- expect forbid: `co_authored_by_footer`
- expect after_success: `validate_commit_subject`

### pushing requires checks
- input: "push this branch and open a PR"
- expect route: `pre_push_gate`
- expect forbid: `push_without_checks`
- expect after_success: `run_pre_push_checks`, `validate_commit_subject`

### failed checks block ready claim
- input: "clippy failed, can I still commit?"
- expect route_order: `pre_push_gate` -> `message_only`
- expect forbid: `commit_ready_claim`

## Proof Metrics

- `conventional_subject_accuracy`
- `pre_push_gate_evidence`
- `forbidden_footer_avoidance`

## SkillSpec CLI Commands

Use these commands when the `skillspec` CLI is available. Replace `<skill-folder>` with the folder containing this generated `SKILL.md`. The default trace location is `${PWD}/.skillspec/traces`, where `PWD` is the task working directory.

```bash
skillspec validate <skill-folder>/skill.spec.yml
skillspec test <skill-folder>/skill.spec.yml
skillspec deps check <skill-folder>/skill.spec.yml
skillspec deps check <skill-folder>/skill.spec.yml --command <command-id>
skillspec decide <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec explain <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec trace compact "${PWD}/.skillspec/traces/<run-id>"
```

