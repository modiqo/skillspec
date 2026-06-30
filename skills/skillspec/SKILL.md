---
name: skillspec
description: "Multiplex SkillSpec post-install setup: inspect skill/repo shape with doctor, map multi-skill and plugin-shaped repositories before fanout import, import existing SKILL.md skills from local folders or public URIs, inspect installed status, install compiled workspaces with entry/support visibility planning, install/update/enable/disable router mode, optionally install/update/enable/disable/delete durable-executor, create specs from observed durable execution workspaces, revise SkillSpec YAML, and prove value before install or release. Use for skillspec, /skillspec, skillspec setup, post install setup, import SKILL.md, import existing skill, port skill, what is the shape of this skill, what is the shape of skill, shape of skill, skill shape and source shape. Use when the task needs to run SkillSpec post-install setup inside the harness prompt, inspect the shape of a skill, skill folder"
---

# SkillSpec

SkillSpec post-install setup and skill-authoring multiplexer for inspecting skill/repo shape with doctor, mapping multi-skill and plugin-shaped workspaces, importing existing prose skills, inspecting SkillSpec status, installing compiled workspaces with visibility planning, installing/updating/enabling/disabling router mode, installing/updating/enabling/disabling/deleting durable-executor, creating specs from observed durable execution workspaces, revising SkillSpecs, compiling reviewed skills, optional install, and value reporting.

Use the directory that contains this loaded `SKILL.md` as `<skill_dir>`.
The SkillSpec contract is `<skill_dir>/skill.spec.yml`; do not assume the user's current working directory contains the spec.

Start the SkillSpec guide with the user's task:

`skillspec run-loop <skill_dir>/skill.spec.yml --input '<user task>' --trace-dir "${PWD}/.skillspec/traces" --guide agent --json`

Resume an existing guided run:

`skillspec run-loop <skill_dir>/skill.spec.yml --resume <run_dir> --guide agent --json`

Use the JSON current gate as internal control data. The selected route, matched rules, forbids, allowed commands, open requirements, resume command, and end proof from the CLI guide are authoritative; do not narrate the raw JSON to the user.

Keep SkillSpec mechanics in the background. Do not narrate ledger writes, raw progress commands, checkpoint rows, trace plumbing, or alignment internals as user-facing progress. Show simple intent-level updates only, such as what was assessed, what changed, what passed, and what remains blocked.

Do not run `skillspec act`, `skillspec query`, `skillspec refs`, or `skillspec --help` during normal execution. Use them only when the guide explicitly names an exact command, a blocker proves the current gate is insufficient, or the user asks to inspect internals.

For read-only diagnostic routes such as Doctor/source-shape assessment, run the diagnostic command, answer directly, and stop. Do not create source maps, import drafts, progress ledgers, final-response proof, or alignment summaries unless the user explicitly asks for proof.

For proof-bearing execution routes, record routine successful evidence with one quiet `skillspec progress checkpoint ... --quiet` command at natural phase boundaries. Do not hand-author event JSONL for routine successful rows. Do not run `skillspec progress ... --help` or query `command:progress_*` during normal execution; the guide provides the needed checkpoint flag shape. Use individual `skillspec progress record` only for failures, blockers, or debugging, and use file-based `skillspec progress batch` only when a JSON/JSONL proof artifact already exists. Before the final response, follow the guide's end anchor with quiet token-stats, final-progress, and alignment commands when metrics are available, then report result, evidence paths, alignment status or report path, token usage from alignment or why it was not recorded, selected route, and run directory.

For workspace or plugin imports, process package review in manifest order with a countdown: package `<index>/<count>`, `<remaining_after>` remaining. Within each package, use `skillspec source lens <source-map.json> --cursor <n>` to review one parsed source block at a time, port that block into matching SkillSpec constructs, validate, then advance the cursor. Conditional workflow language such as if/when/unless must become structural rules, not prose comments. Promotion proof must carry the lens block `source_hash` and target kinds required by the lens. Do not review one representative package and bulk-apply that promotion to other packages.

For import-and-install requests, do not stop after `workspace import` or a scaffold/converge blocker unless there is a real unresolved blocker. Continue package-by-package semantic promotion, converge, compile, dry-run install, and install. If an active skill already exists in the target harness root, use the approved `--retire-existing` path by default so the old skill is backed up and removed; side-by-side installs require an explicit distinct name or install slug.

Missing alignment proof is not a prompt-writing task. Do not create route, obligation, elicitation, or phase proof rows after the fact to make alignment pass. If evidence was not captured when the work happened, report partial alignment and the exact missing proof instead of manufacturing progress.

If the `skillspec` CLI is not installed, report that this skill requires SkillSpec and ask the user to install it before continuing:

```bash
curl -fsSL https://raw.githubusercontent.com/modiqo/skillspec/main/install.sh | sh
# or, with Rust installed:
cargo install skillspec
```

If the user declines or installation is impossible, read `<skill_dir>/skill.spec.yml` directly and manually follow the same route, rule, phase, dependency, forbid, proof, and completion contract. Report that CLI guidance was unavailable and alignment proof is partial.
