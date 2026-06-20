---
name: browser-research
description: Route browser inspection tasks through the colocated SkillSpec contract.
---

# browser research

Route browser inspection tasks to browser automation and keep URL discovery separate from page evidence.

This skill is a thin loader for the colocated `skill.spec.yml`. The spec is the source of truth for routes, rules, elicitations, tests, and trace requirements.

## Runtime Contract

1. Load `./skill.spec.yml` from this skill folder before taking task actions.
2. Run `skillspec decide ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"` when the CLI is available.
3. Preserve the emitted trace `run_dir`.
4. Follow the selected route, matched rules, forbids, elicitations, and closures from `skill.spec.yml`.
5. If the CLI is unavailable, read `skill.spec.yml` directly and apply its rules manually.
