---
name: skill-router
description: "Use for every user request when SkillSpec router mode is enabled. First check the local SkillSpec router index, load the selected skill only when route decision is use_skill, and continue with normal agent behavior when the decision is bypass or ambiguous."
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

This skill is a thin native harness loader for the SkillSpec router.
Load and follow `./skill.spec.yml`; that file is the router contract.
When router mode is enabled and the harness has been restarted, this router is
the first hop for every user request in managed skill roots. All routed skills
are explicit-only/manual-only, so check the router index before reading,
searching for, or guessing at a domain skill.

The router does not execute task work. It decides whether a local skill should
be loaded, reports confidence and candidate paths, and asks for direct or
durable execution only after a `use_skill` decision when the task will use tools
and the user has not already chosen an execution mode.

## Runtime Contract

1. Load `./skill.spec.yml` before taking task actions.
2. For normal discovery, run:

   ```bash
   skillspec route --index <index-file-or-router-dir> --query '<user task>' --top 5 --json
   ```

3. If route output says `decision: "use_skill"` and `selected` is non-null, read that skill's `SKILL.md` and follow it.
4. If route output says `decision: "bypass"`, do not load any candidate skill; continue with normal agent behavior for the request.
5. If route output says `decision: "ambiguous"`, do not silently choose a candidate. Ask only when the user explicitly requested skill selection; otherwise continue with normal agent behavior.
6. If route output includes `execution_mode_direct_or_durable` for a `use_skill` decision, ask the user whether to run direct or durable before tool-backed execution.
7. If the user chooses durable execution, hand the selected skill and task context to `durable-executor`. The router still only routes; durable-executor owns workspace evidence, rote execution policy, alignment, token stats, and final closure.

## Router Management

Use lifecycle commands for managed installation:

```bash
skillspec router install --roots <skill-root>... --index <index-file-or-router-dir>
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
