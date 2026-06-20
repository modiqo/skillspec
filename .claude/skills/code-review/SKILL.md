---
name: generic-code-review
description: "Review code changes by collecting the review target, researching context, checking risks, and reporting findings before summary."
---

# Code Review

Review code changes by collecting the review target, researching context, checking risks, and reporting findings before summary.

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

Prompt: Determine the review target first, then collect evidence and report findings ordered by severity.

## Applies When

```yaml
user_intent:
- review code
- review a pull request
- sanity check changes
- audit a diff
- inspect changed files for bugs
```

## Routes

Try lower-rank routes first unless matching rules override the route or route order.

### `current_pr`
- label: Review current pull request
- rank: 10
- description: Use GitHub CLI to inspect the current PR metadata and diff.

### `diff_to_main`
- label: Review diff to main
- rank: 20
- description: Use local git diff when the work is not necessarily attached to a PR.

### `specific_paths`
- label: Review specific paths
- rank: 30
- description: Use when the user names files, crates, directories, or a narrow ownership surface.

### `second_opinion`
- label: Provide review synthesis from supplied evidence
- rank: 40
- description: Use when the diff or findings are already provided and the task is synthesis.

## Rules

Evaluate rules in order. A matching rule may choose a route, replace route order, forbid substitutions, allow narrow fallbacks, request bounded elicitation, and schedule post-success actions.

### `explicit_pr_review_uses_current_pr`
- when:
  - user_says_any: "review this pr", "current pr", "pull request", "pr review"
- prefer: `current_pr`
- forbid: `summary_before_findings`, `ungrounded_review`
- after_success: `collect_pr_diff`, `produce_findings_first_report`
- reason: PR review needs PR metadata, base branch, and diff evidence before findings.

### `diff_review_uses_local_diff`
- when:
  - user_says_any: "review my changes", "diff to main", "uncommitted changes", "committed changes"
- prefer: `diff_to_main`
- forbid: `github_required_for_local_diff`, `summary_before_findings`
- after_success: `collect_local_diff`, `produce_findings_first_report`
- reason: Local diff review should not require GitHub when the local repository has the needed evidence.

### `path_review_stays_scoped`
- when:
  - user_says_any: "specific files", "these files", "this crate", "this directory"
- prefer: `specific_paths`
- forbid: `unrelated_refactor`, `broad_review_when_paths_named`
- after_success: `produce_findings_first_report`
- reason: Named paths define the review boundary unless the user asks to broaden it.

### `findings_lead_the_response`
- when:
  - user_says_any: "review", "audit", "sanity check", "anything wrong"
- route_order: `current_pr` -> `diff_to_main` -> `specific_paths` -> `second_opinion`
- forbid: `praise_before_risks`, `change_summary_as_primary_answer`
- after_success: `produce_findings_first_report`
- reason: Code review output should prioritize bugs, risks, regressions, and missing tests before summary.

## Decision Trace

When the `skillspec` CLI or a compatible harness evaluates this spec, record the decision path as append-only events. Rules trigger decisions; the evaluator writes the trace.

- mode: `event_log`
- required: true
- record: `input_received`, `spec_loaded`, `rule_evaluated`, `rule_matched`, `route_selected`, `after_success_scheduled`, `outcome_recorded`

## Dependencies

Check declared dependencies before using commands that require them. Missing dependencies must be handled through the declared provision or elicitation path; do not silently install global tools.

### `gh`
- kind: `cli`
- description: GitHub CLI is required only for current-PR review.
- command: `gh`
- check:
  - command: `gh`
- permission:
  - required: true
  - reason: GitHub CLI may use authenticated account state and network access.
  - safety: `network_read`

### `git`
- kind: `cli`
- description: Git is required to inspect local branch and diffs.
- command: `git`
- check:
  - command: `git`

## Resources

Resources are source material and provenance, not hidden control flow. Use structured routes, rules, code, commands, and recipes for behavior.

### `source_skill`
- path: `source/SKILL.md`
- role: `source_material`
- description: Original prose skill with review target collection, phased review flow, and report format.
- used_by:
  - route: `current_pr`
  - route: `diff_to_main`
  - route: `specific_paths`
  - recipe: `produce_findings_first_report`

## Recipes

Recipes are ordered procedures with explicit resource, dependency, code, command, elicitation, and artifact references.

### `collect_local_diff`
- description: Gather local diff evidence.
- ordered: true
- steps:
  - run_command: `local_diff_to_main`

### `collect_pr_diff`
- description: Gather PR metadata and diff evidence.
- ordered: true
- steps:
  - run_command: `pr_metadata`
  - run_command: `pr_diff`

### `produce_findings_first_report`
- description: Write findings first, ordered by severity, then questions, then secondary summary.
- ordered: true
- steps:
  - load_resource: `source_skill`
  - note: Prioritize bugs, behavioral regressions, security issues, and missing tests.
  - note: Include file and line references when available.
  - note: Put change summary after findings, not before.

## Command Templates

Command templates are examples and contracts, not automatic permission. Apply the safety class and the harness approval policy before executing.

### `local_diff_to_main`
- description: Collect the local diff against main.
- safety: `read_only`
- template:

```bash
git diff main...HEAD
```
- requires:
  - dependencies: `git`

### `pr_diff`
- description: Collect the current pull request diff.
- safety: `network_read`
- template:

```bash
gh pr diff
```
- requires:
  - dependencies: `gh`

### `pr_metadata`
- description: Collect pull request title, body, number, and base branch.
- safety: `network_read`
- template:

```bash
gh pr view --json title,body,number,baseRefName
```
- requires:
  - dependencies: `gh`

## Snippets

### `review_output_order`
Findings first, ordered by severity; then open questions; then a short change summary only as secondary context.

## Closures

Closures are post-task obligations or named lifecycle actions. Run them when referenced by states or `after_success`.

### `collect_local_diff`
```yaml
description: Collect local diff before reviewing.
```

### `collect_pr_diff`
```yaml
description: Collect PR metadata and diff before reviewing.
```

### `produce_findings_first_report`
```yaml
description: Return findings first by severity, then open questions, then brief summary.
```

## Scenario Tests

Use these as behavioral examples. The agent should make the same routing and guardrail choices for equivalent tasks.

### current pr review uses gh
- input: "review this PR"
- expect route: `current_pr`
- expect forbid: `summary_before_findings`
- expect after_success: `collect_pr_diff`, `produce_findings_first_report`

### local changes use git diff
- input: "review my changes against main"
- expect route: `diff_to_main`
- expect forbid: `github_required_for_local_diff`
- expect after_success: `collect_local_diff`, `produce_findings_first_report`

### review response leads with findings
- input: "sanity check this change, anything wrong?"
- expect route_order: `current_pr` -> `diff_to_main` -> `specific_paths` -> `second_opinion`
- expect forbid: `praise_before_risks`, `change_summary_as_primary_answer`

## Proof Metrics

- `evidence_collection_before_review`
- `severity_ordering`
- `line_reference_coverage`
- `missing_test_detection`

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

