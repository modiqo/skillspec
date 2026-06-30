---
name: skillspec
description: "Multiplex SkillSpec post-install setup: inspect skill/repo shape with doctor, map multi-skill and plugin-shaped repositories before fanout import, import existing SKILL.md skills from local folders or public URIs, inspect installed status, install compiled workspaces with entry/support visibility planning, install/update/enable/disable router mode, optionally install/update/enable/disable/delete durable-executor, create specs from observed durable execution workspaces, revise SkillSpec YAML, and prove value before install or release. Use for skillspec, /skillspec, skillspec setup, post install setup, import SKILL.md, import existing skill, port skill, what is the shape of this skill, what is the shape of skill, shape of skill, skill shape and source shape. Use when the task needs to run SkillSpec post-install setup inside the harness prompt, inspect the shape of a skill, skill folder"
---

# SkillSpec

SkillSpec post-install setup and skill-authoring multiplexer for inspecting skill/repo shape with doctor, mapping multi-skill and plugin-shaped workspaces, importing existing prose skills, inspecting SkillSpec status, installing compiled workspaces with visibility planning, installing/updating/enabling/disabling router mode, installing/updating/enabling/disabling/deleting durable-executor, creating specs from observed durable execution workspaces, revising SkillSpecs, compiling reviewed skills, optional install, and value reporting.

Use the directory that contains this loaded `SKILL.md` as `<skill_dir>`.
The SkillSpec contract is `<skill_dir>/skill.spec.yml`; do not assume the user's current working directory contains the spec.

Start the SkillSpec guide with the user's task:

`skillspec run-loop <skill_dir>/skill.spec.yml --input '<user task>' --trace-dir "${PWD}/.skillspec/traces" --guide agent`

Resume an existing guided run:

`skillspec run-loop <skill_dir>/skill.spec.yml --resume <run_dir> --guide agent`

Follow the printed current gate. The selected route, matched rules, forbids, allowed commands, open requirements, resume command, and end proof from the CLI guide are authoritative.

Keep SkillSpec mechanics in the background. Do not narrate ledger writes, raw progress commands, evidence-batch JSONL rows, trace plumbing, or alignment internals as user-facing progress. Show simple intent-level updates only, such as what was assessed, what changed, what passed, and what remains blocked.

Use `skillspec query` and `skillspec refs` only for handles named by the guide. Do not read the full spec unless the guide, a blocker, or the user asks for it.

For read-only diagnostic routes such as Doctor/source-shape assessment, run the diagnostic command, answer directly, and stop. Do not create source maps, import drafts, progress ledgers, final-response proof, or alignment summaries unless the user explicitly asks for proof.

For proof-bearing execution routes, batch routine successful evidence into a JSONL file without displaying the rows and run one quiet `skillspec progress batch ... --quiet` checkpoint at natural phase boundaries. Do not run `skillspec progress ... --help` or query `command:progress_*` during normal execution; the guide provides the needed JSONL shape. Use individual `skillspec progress record` only for failures, blockers, or debugging. Before the final response, follow the guide's end anchor with quiet progress/alignment commands, then report result, evidence paths, alignment status or report path, token usage when recorded, selected route, and run directory.

If the `skillspec` CLI is not installed, report that this skill requires SkillSpec and ask the user to install it before continuing:

```bash
curl -fsSL https://raw.githubusercontent.com/modiqo/skillspec/main/install.sh | sh
# or, with Rust installed:
cargo install skillspec
```

If the user declines or installation is impossible, read `<skill_dir>/skill.spec.yml` directly and manually follow the same route, rule, phase, dependency, forbid, proof, and completion contract. Report that CLI guidance was unavailable and alignment proof is partial.
