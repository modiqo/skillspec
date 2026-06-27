---
name: skillspec
description: "Use for SkillSpec tasks: inspect skill/repo shape, run doctor, import or port SKILL.md skills, map/import/converge/compile/install workspaces, manage router or durable-executor lifecycle, revise specs, and prove value. Use for /skillspec, skillspec setup, shape of skill, run doctor on this repo/url, port skill, workspace map/import/converge/compile/install, status, router, and proof."
---

# SkillSpec

Use the directory that contains this loaded `SKILL.md` as `<skill_dir>`.
The SkillSpec contract is `<skill_dir>/skill.spec.yml`; do not assume the
user's current working directory contains the spec.

Start the SkillSpec guide with the user's task:

`skillspec run-loop <skill_dir>/skill.spec.yml --input '<user task>' --trace-dir "${PWD}/.skillspec/traces" --guide agent`

Resume an existing guided run:

`skillspec run-loop <skill_dir>/skill.spec.yml --resume <run_dir> --guide agent`

Follow the printed current gate. The selected route, matched rules, forbids,
allowed commands, open requirements, resume command, and end proof from the CLI
guide are authoritative.

Use `skillspec query` and `skillspec refs` only for handles named by the guide.
Do not read the full spec unless the guide, a blocker, or the user asks for it.

Before the final response, follow the guide's end anchor: record final-response
evidence, run the printed `skillspec trace align ... --summary` command as the
completion summary source, and report result, evidence, alignment summary,
token usage, selected route, and run directory.

If the `skillspec` CLI is not installed, report that the CLI is unavailable and
ask the user to install it before continuing with SkillSpec-guided work. If the
CLI exists but guide mode fails, read `<skill_dir>/skill.spec.yml` directly and
manually follow the same route, rule, phase, dependency, forbid, proof, and
completion contract. Report that CLI guidance was unavailable.
