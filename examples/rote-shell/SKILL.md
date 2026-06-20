---
name: rote-shell
description: "Structured version of the rote-shell skill for durable CLI, shell, process, stream, PTY, dependency, and shell-flow crystallization work."
---

# rote shell

Structured version of the rote-shell skill for durable CLI, shell, process, stream, PTY, dependency, and shell-flow crystallization work.

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

- `adapter_first_cli_fallback`: Use rote adapters, then rote exec CLI fallback
- `browser_handoff`: Defer browser work to the `rote-browse` skill; rote-shell does not prescribe browser commands
- `one_shot_process`: Capture a one-shot process
- `declared_file_io`: Capture declared file inputs or outputs
- `stream_follow`: Follow a moving file or process stream
- `background_process`: Start and track a background process lease
- `pty_transcript`: Capture a one-shot PTY transcript
- `dependency_preflight`: Check dependencies before replay or release
- `crystallized_flow`: Crystallize or replay a shell flow
- `raw_shell`: Use raw shell for disposable inspection only
