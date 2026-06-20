---
name: generic-research-codebase
description: "Read a repository, map what exists, and produce neutral architecture documentation without changing code."
---

# Research Codebase

Read a repository, map what exists, and produce neutral architecture documentation without changing code.

This skill was generated from a SkillSpec. Use this document as the loaded harness guidance, and treat the referenced structured decisions as the behavioral contract.

## Runtime Contract

- Read this generated skill for orientation and immediate rules.
- Treat routes, rules, states, commands, tests, and review notes below as authoritative.
- Rules beat prose when there is tension.
- `forbid` entries are hard negative steering, not suggestions.
- `elicit` entries require bounded user questions before guessing.
- Use the scenario tests as examples of expected behavior.
- When the `skillspec` CLI is available, prefer `skillspec decide` or `skillspec explain` over manual interpretation.
- When invoking `skillspec decide`, pass only the user's task text. Strip skill invocation prefixes such as `/rote-shell-spec`, `$rote-shell-spec`, or `/my-skill` before setting `--input`.
- Prefer `--input='<task text>'` in shell examples so `$skill-name` text is not expanded by the shell.
- Resolve `skill.spec.yml` relative to this `SKILL.md` folder, not the process working directory.
- Always pass `--trace-dir`; use `${PWD}/.skillspec/traces` unless the user or harness provides a run-specific trace directory.
- After `skillspec decide` prints trace lines, keep the emitted `run_dir` and mention it when reporting how the decision was made.

## Entry

Prompt: Clarify the research question, then perform read-only repository exploration and synthesize what exists.

## Applies When

```yaml
user_intent:
- understand a codebase
- map repository architecture
- explain where something is implemented
- document current behavior
- research modules without editing
```

## Routes

Try lower-rank routes first unless matching rules override the route or route order.

### `focused_symbol_research`
- label: Focused symbol or feature research
- rank: 10
- description: Use for questions about one function, type, module, or narrow behavior.

### `cross_component_research`
- label: Cross-component research
- rank: 20
- description: Use when the answer requires multiple files, crates, services, or data paths.

### `architecture_map`
- label: Architecture map
- rank: 30
- description: Use when the user asks for a broad map of the repository.

## Rules

Evaluate rules in order. A matching rule may choose a route, replace route order, forbid substitutions, allow narrow fallbacks, request bounded elicitation, and schedule post-success actions.

### `simple_where_questions_are_focused`
- when:
  - user_says_any: "where is", "find implementation", "where does", "what file"
- prefer: `focused_symbol_research`
- forbid: `speculative_improvement`, `ungrounded_summary`
- after_success: `produce_research_document`
- reason: Narrow location questions need grounded file references and no recommendations unless asked.

### `cross_file_questions_use_cross_component_route`
- when:
  - user_says_any: "how does", "how do these interact", "data flow", "architecture", "across modules"
- prefer: `cross_component_research`
- forbid: `single_file_answer_when_cross_component`, `refactor_recommendation_without_request`
- after_success: `produce_research_document`
- reason: Multi-file questions require a map of relationships, not a single snippet.

### `explicit_research_never_edits`
- when:
  - user_says_any: "research", "document", "explain", "understand", "map"
- route_order: `focused_symbol_research` -> `cross_component_research` -> `architecture_map`
- forbid: `code_edit`, `root_cause_analysis_without_request`, `enhancement_proposal_without_request`
- reason: Research mode documents the current system; it does not fix or critique by default.

## Decision Trace

When the `skillspec` CLI or a compatible harness evaluates this spec, record the decision path as append-only events. Rules trigger decisions; the evaluator writes the trace.

- mode: `event_log`
- required: true
- record: `input_received`, `spec_loaded`, `rule_evaluated`, `rule_matched`, `route_selected`, `after_success_scheduled`, `outcome_recorded`

## Dependencies

Check declared dependencies before using commands that require them. Missing dependencies must be handled through the declared provision or elicitation path; do not silently install global tools.

### `git`
- kind: `cli`
- description: Git is used to identify branch and commit context for research reports.
- command: `git`
- check:
  - command: `git`

### `rg`
- kind: `cli`
- description: ripgrep is the preferred repository search tool for codebase research.
- command: `rg`
- check:
  - command: `rg`

## Resources

Resources are source material and provenance, not hidden control flow. Use structured routes, rules, code, commands, and recipes for behavior.

### `source_skill`
- path: `source/SKILL.md`
- role: `source_material`
- description: Original prose skill describing neutral, read-only codebase research behavior.
- used_by:
  - route: `focused_symbol_research`
  - route: `cross_component_research`
  - route: `architecture_map`
  - recipe: `produce_research_document`

## Recipes

Recipes are ordered procedures with explicit resource, dependency, code, command, elicitation, and artifact references.

### `produce_research_document`
- description: Produce a neutral research document grounded in repository files.
- ordered: true
- steps:
  - load_resource: `source_skill`
  - run_command: `current_commit`
  - run_command: `search_code`
  - note: Read relevant files completely enough to explain relationships.
  - note: Synthesize what exists with file references; avoid recommendations unless explicitly asked.

## Command Templates

Command templates are examples and contracts, not automatic permission. Apply the safety class and the harness approval policy before executing.

### `current_commit`
- description: Capture the current commit for report provenance.
- safety: `read_only`
- template:

```bash
git rev-parse HEAD
```
- requires:
  - dependencies: `git`

### `search_code`
- description: Search repository text for symbols, files, and concepts.
- safety: `read_only`
- template:

```bash
rg <query> <paths>
```
- requires:
  - dependencies: `rg`

## Snippets

### `report_shape`
Research question, summary, detailed findings, code references, architecture notes, open questions.

### `research_posture`
Describe what exists, where it exists, how it works, and how components interact. Do not propose changes unless asked.

## Closures

Closures are post-task obligations or named lifecycle actions. Run them when referenced by states or `after_success`.

### `produce_research_document`
```yaml
description: Return a structured report with question, summary, findings, code references, and open questions.
```

## Scenario Tests

Use these as behavioral examples. The agent should make the same routing and guardrail choices for equivalent tasks.

### where question chooses focused research
- input: "where is request routing implemented?"
- expect route: `focused_symbol_research`
- expect forbid: `speculative_improvement`
- expect after_success: `produce_research_document`

### cross component question chooses cross component route
- input: "how does authentication flow across modules?"
- expect route: `cross_component_research`
- expect forbid: `single_file_answer_when_cross_component`
- expect after_success: `produce_research_document`

### research request forbids edits
- input: "research this codebase and explain the architecture"
- expect route_order: `focused_symbol_research` -> `cross_component_research` -> `architecture_map`
- expect forbid: `code_edit`, `enhancement_proposal_without_request`

## Proof Metrics

- `file_reference_coverage`
- `neutrality_adherence`
- `no_edit_compliance`

## SkillSpec CLI Commands

Use these commands when the `skillspec` CLI is available. Replace `<skill-folder>` with the folder containing this generated `SKILL.md`. The default trace location is `${PWD}/.skillspec/traces`, where `PWD` is the task working directory.

```bash
skillspec validate <skill-folder>/skill.spec.yml
skillspec test <skill-folder>/skill.spec.yml
skillspec deps check <skill-folder>/skill.spec.yml
skillspec deps check <skill-folder>/skill.spec.yml --command <command-id>
skillspec decide <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec explain <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
skillspec trace compact "${PWD}/.skillspec/traces/<run-id>"
```

