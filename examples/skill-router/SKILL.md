---
name: skill-router
description: "Use when selecting the right local skill from a large SkillSpec-indexed skill library, especially when many skills are installed, skill descriptions are shortened, or the user asks to install, uninstall, refresh, route, audit, or manage skill visibility through the SkillSpec router. Do not use for ordinary code generation when a specific domain skill is already selected."
metadata:
  routing:
    tags: [skills, router, discovery, codex, claude]
    triggers:
      - choose the right skill
      - route to a skill
      - many skills installed
      - skill descriptions shortened
      - install skill router
      - disable implicit skill invocation
    negative_triggers:
      - ordinary code generation
---

# Skill Router

This skill is the visible discovery surface for large local skill libraries.

The router does not execute task work. It selects a likely skill, reports confidence and candidate paths, and asks for direct or durable execution when the task will use tools and the user has not already chosen an execution mode.

## Runtime Contract

1. Load `./skill.spec.yml` before taking task actions.
2. For normal discovery, run:

   ```bash
   skillspec route --index <index-file-or-router-dir> --query '<user task>' --top 5 --json
   ```

3. If the selected candidate has high confidence, read that skill's `SKILL.md` and follow it.
4. If confidence is medium, compare the top candidates briefly and choose the best fit.
5. If confidence is low, ask the user to choose a skill or continue without a skill.
6. If route output includes `execution_mode_direct_or_durable`, ask the user whether to run direct or durable before tool-backed execution.
7. If the user chooses durable execution, hand the selected skill and task context to `durable-executor`. The router still only routes; durable-executor owns workspace evidence, rote execution policy, alignment, token stats, and final closure.

## Router Management

Use lifecycle commands for managed installation:

```bash
skillspec router install --roots <skill-root>... --router-root <skill-root> --index <index-file-or-router-dir>
skillspec router index status --roots <skill-root>... --index <index-file-or-router-dir> --visibility-manifest <manifest>
skillspec router index refresh --roots <skill-root>... --index <index-file-or-router-dir> --visibility-manifest <manifest>
skillspec router uninstall
```

Any index argument can be either the SQLite file itself or the router directory;
directory paths resolve to `skill-index.sqlite`.

Use visibility commands for explicit controls:

```bash
skillspec visibility plan --roots <skill-root>... --json
skillspec visibility apply --roots <skill-root>... --manifest <manifest> --json
skillspec visibility restore --manifest <manifest> --json
skillspec skills set-visibility <skill-name> manual-only --roots <skill-root>... --manifest <manifest>
skillspec skills disable <skill-name> --roots <skill-root>... --manifest <manifest>
skillspec skills enable <skill-name> --roots <skill-root>... --manifest <manifest>
```

The manifest is the rollback boundary. Do not bulk-restore visibility by inference when the manifest is missing.
