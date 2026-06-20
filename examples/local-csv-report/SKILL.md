---
name: local-csv-report
description: "Use rote shell to turn a local CSV into durable JSON and text reports with file provenance, dependency checks, privacy guardrails, and optional reuse."
---

# local csv report

Use rote shell to turn a local CSV into durable JSON and text reports with file provenance, dependency checks, privacy guardrails, and optional reuse.

This skill is a thin loader for the colocated `skill.spec.yml`. The spec is the source of truth for routes, rules, dependencies, imports, resources, recipes, tests, and trace requirements.

## Runtime Contract

1. Load `./skill.spec.yml` from this skill folder before taking task actions.
2. When the `skillspec` CLI is available, run:

   ```bash
   skillspec decide ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
   ```

3. Strip skill invocation prefixes such as `/my-skill`, `$my-skill`, or `/rote-shell-spec` before passing `--input`.
4. Preserve the emitted trace `run_dir`.
5. When the CLI is available after a trace exists, run `skillspec trace align ./skill.spec.yml --decision-trace <run_dir>` and report the alignment status with the trace path.
6. Follow the selected route, matched rules, forbids, elicitations, dependencies, imports, recipes, and closures from `skill.spec.yml`.
7. If the CLI is unavailable, read `skill.spec.yml` directly and apply its rules manually. Do not expand this loader into a second source of truth.

## Quick Commands

```bash
skillspec validate ./skill.spec.yml
skillspec imports check ./skill.spec.yml
skillspec test ./skill.spec.yml
skillspec deps check ./skill.spec.yml
skillspec explain ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec trace align ./skill.spec.yml --decision-trace "${PWD}/.skillspec/traces/<run-id>"
```

## Completion Report

When reporting completion, include the selected route, the SkillSpec trace `run_dir`, the `skillspec trace align` status (`pass`, `fail`, or `unproven`), key failed or unproven alignment checks, and the concrete execution evidence ids or files.

## Route Hints

- `remembered_report`: Reuse a saved local report workflow
- `local_file_report`: Build a report from local files
- `dependency_preflight`: Check local tools before report generation
- `crystallized_flow`: Save as a reusable local reporting flow
- `ask_user`: Ask for missing report details
