---
name: skillspec-importer
description: Use when converting an existing prose SKILL.md into a SkillSpec v0 file. Preserves the useful prose while extracting routes, rules, states, commands, snippets, tests, proof metrics, and review notes into `skill.spec.yml`.
---

# skillspec-importer

Use this skill when the user asks to port an existing `SKILL.md` into
`skill.spec.yml`, create a structured companion for a prose skill, or test
whether a skill's routing behavior can be made smaller and more provable.

The goal is not to erase prose. The goal is:

```text
Keep the prose. Structure the decisions.
```

## Import Stance

Treat prose skills as source material, not executable truth. A good import
should preserve:

- the human orientation and tone in `SKILL.md`
- the routing decisions in `rules`
- the candidate substrates or strategies in `routes`
- the task lifecycle in `states`
- command examples as `commands`
- reusable user-facing text as `snippets`
- post-task obligations as `closures`
- known failure modes as `tests`
- uncertainty as `review_required`

Never claim a port is complete until `skillspec validate` and
`skillspec test` pass.

## Procedure

1. Read the full source `SKILL.md`.
2. Run the deterministic importer to get a first draft:

   ```bash
   skillspec import-skill path/to/SKILL.md --out skill.spec.yml
   ```

3. Inspect the draft. The importer is intentionally conservative; expect to
   edit it.
4. Extract routes from strategy choices. Examples:

   - adapter/API
   - CLI/process
   - browser
   - PTY
   - background job
   - remembered/reused route
   - human approval

5. Extract rules from words such as "always", "never", "prefer", "do not",
   "if", "unless", "when", "before", "after", and "ask".
6. Convert command examples into `commands` with `template`, `safety`, and
   requirements.
7. Convert task phases into `states`. States should reference command or
   closure ids; do not inline paragraphs into `states.do`.
8. Convert stable user-facing copy into `snippets`.
9. Convert post-success behavior into `closures`. Examples:

   - collect trace cost
   - ask whether to remember
   - ask whether to share
   - write digest
   - run release QA

10. Add scenario tests for every important routing or forbid decision.
11. Run validation and tests:

    ```bash
    skillspec validate skill.spec.yml
    skillspec test skill.spec.yml
    skillspec explain skill.spec.yml --input "<representative task>"
    ```

12. Compile a compact skill only after the structured spec is valid:

    ```bash
    skillspec compile skill.spec.yml --target codex-skill
    skillspec compile skill.spec.yml --target claude-skill
    ```

## Rule Extraction

Rules should be short, testable decisions. Use document-level rules for:

- route choice
- route order
- forbidden substitutions
- narrow allowed fallbacks
- post-success obligations

Do not put long prose in rules. If the text is guidance to the user, make it a
snippet. If the text is an action, make it a command or closure. If the text is
a steering decision, make it a rule.

Good:

```yaml
rules:
  - id: browser_words_handoff_to_browse
    when:
      user_says_any:
        - browse
        - click
        - snapshot
    prefer: browser
    forbid:
      - native_search_as_answer
```

Weak:

```yaml
rules:
  - id: browser
    reason: Use the browser when it seems appropriate and be careful.
```

## State Extraction

States should be lifecycle positions, not hidden workflows.

Good:

```yaml
states:
  execute:
    do:
      - run_selected_route
      - persist_evidence
    next: complete
```

Bad:

```yaml
states:
  execute:
    say: Run the selected route, save everything, maybe ask the user, and do the right thing.
```

## Command Extraction

Every command template needs a safety class:

- `read_only`
- `local_read`
- `local_write`
- `network_read`
- `network_write`
- `browser_attach`
- `credential_request`
- `destructive`

If a command depends on a tool, file, env var, or auth state, record that under
`requires` or add a review note if the requirement cannot be represented yet.

## Test Extraction

Every meaningful route rule should have at least one test.

When porting an old skill, prioritize tests for known harness drift:

- browser request answered with web search
- adapter setup attempted before browser fallback
- shell output summarized from scrollback instead of a typed response
- long-running process run in a blocking foreground path
- release/publish run without dry-run or explicit approval
- dependency-dependent flow marked released without `deps.toml`

## Review Notes

Use `review_required` whenever the importer or author is uncertain.

Examples:

- "confirm whether browser route should ask attach-existing before headless"
- "confirm whether release command is destructive"
- "confirm whether this command template requires a project-local dependency"
- "confirm whether this state should be a closure instead"

Do not hide uncertainty in comments. Put it in the spec.

## Done Definition

A port is ready for serious testing when:

- `skillspec validate skill.spec.yml` passes
- `skillspec test skill.spec.yml` passes
- `skillspec explain` gives the expected route for at least three realistic
  user inputs
- the generated Codex/Claude skill is smaller than the original prose skill
  but still points to the structured spec
- all unresolved uncertainties are explicit in `review_required`

