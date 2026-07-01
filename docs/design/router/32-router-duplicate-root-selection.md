# Router Duplicate Root Selection

Router mode has to work on machines where the same skill exists in more than
one harness root. A common local setup has the same package in `.agents/skills`,
`.codex/skills`, and a project-local `.claude/skills` directory. Those are
deployment copies of one logical skill, not independent alternatives.

## Contract

`skillspec route` must separate two decisions:

1. Logical selection: should this user request load a skill, and which logical
   skill is it?
2. Physical selection: once a logical skill is selected, which installed copy
   should the current harness load?

Duplicate roots must not create `ambiguous_match` by themselves. Ambiguity is
reserved for real user-intent conflicts, such as two different skills with
similar names and similar scores.

## Logical Identity

The router groups scored candidates by logical identity before applying the
match gate.

For SkillSpec-backed skills, logical identity is the resolved `skill.spec.yml`
`id`:

```text
spec:<id>
```

For prose-only skills, logical identity is the skill name plus a normalized
prose checksum:

```text
skill:<name>:<normalized-checksum>
```

The normalized checksum strips only the visibility-only
`disable-model-invocation` frontmatter key before hashing. Router mode may add
that key to shared `.agents` roots while Codex and project-local Claude copies
express the same visibility through sidecars or settings. That metadata must
not split one logical skill into multiple route candidates.

This still preserves safety for prose skills. Two folders with the same name but
different prose bodies remain different logical candidates, so they can still
produce `ambiguous` instead of being silently merged.

## Physical Preference

After candidates are grouped, each group chooses one representative physical
copy. The representative keeps the strongest duplicate score, so match
confidence is still based on route intent rather than filesystem placement.

The physical preference order is:

1. `--current-root`, when the caller knows the exact active skill root.
2. `--current-harness`, when the caller knows whether the active harness is
   `agents`, `codex`, or `claude-local`.
3. Project-local `.claude/skills`, when the current working directory is inside
   that project.
4. A SkillSpec-backed copy.
5. Configured index root order, then stable path order.

Only physical selection uses this preference. The router does not give a skill a
better semantic match merely because it is installed in the current harness.

## Harness Context

Managed router hooks pass their harness identity into guard:

```sh
skillspec router guard --config <router-config> --hook --harness codex
skillspec router guard --config <router-config> --hook --harness claude-local
```

The generated `skill-router` then includes route context:

```sh
skillspec route \
  --index <router-index> \
  --query '<user task>' \
  --current-harness codex \
  --current-root <active-skill-root> \
  --top 5 \
  --json
```

If the context is missing, route remains deterministic by falling back to
project-local `.claude`, SkillSpec-backed copies, configured root order, and
path order.

## Multi-Harness Principle

SkillSpec stays harness-agnostic for logical selection. It does not try to infer
which model or product the user will switch to next, and it does not globally
prefer one vendor root over another. The active harness can provide context for
the current turn; otherwise the fallback order is deterministic and documented.

This matters for systems where a user moves between Codex, Claude, and Agents
against the same shared skill library. The same route decision should select the
same logical skill everywhere. The only thing that changes is the physical
loader path best suited to the harness currently invoking the router.

## Regression Coverage

The pseudo-harness simulator owns this boundary:

- duplicate logical skills across multiple roots collapse to one route
  candidate;
- the selected physical copy follows harness/root preference;
- same-name skills with different logical identity remain separate candidates;
- durable-executor duplicates do not block durable activation with
  `ambiguous_match`.

The baseline is
`crates/skillspec-harness-lab/baselines/17-pseudo-harness-simulator.json`.
