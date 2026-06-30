# Progressive Agent Guidance

Status: implemented across source import, guided run-loop, workspace summaries,
doctor, and alignment reporting
Owner: SkillSpec
Target reader: maintainers, agent operators, harness authors, and reviewers

## One-Line Thesis

SkillSpec uses the CLI as an agent-facing guide, not just a validator. The CLI
turns large skill sources and large SkillSpec contracts into progressive,
bounded steps: show the map, expose the current gate, preserve proof on disk,
and tell the agent exactly what to do next.

## Why This Exists

Plain skills and large `skill.spec.yml` files can both become too load-bearing
for a model to hold in working context. The failure mode is similar in both
places:

- the agent reads too much before it has a map;
- important obligations sit in the middle of long context;
- compaction can remove or summarize the wrong detail;
- the model may continue from memory instead of the current source;
- proof can become a final-response story instead of a trace-backed artifact.

SkillSpec addresses this by moving navigation and progress guidance into CLI
commands that are purpose-built for agents. The agent does not need to infer the
whole workflow from prose. It asks the CLI for the current slice, follows that
slice, records proof, and asks for the next slice.

This design is grounded in the concerns documented in:

- [05 Progressive Sensemaking](05-progressive-sensemaking.md)
- [10 Runtime Plan Act Progress Loop](10-runtime-plan-act-progress-loop.md)
- [11 Execution Progress Ledger](11-execution-progress-ledger.md)
- [12 Traces And Alignment](12-traces-and-alignment.md)
- [13 Completion Alignment And Token Reporting](13-completion-alignment-and-token-reporting.md)
- [18 Source Map Progressive Reader](18-source-map-progressive-reader.md)
- [20 Performance, Token Economy, And Incremental Processing](20-performance-token-speed.md)
- [22 Doctor Agent Drift Risk](22-doctor-agent-drift-risk.md)
- [23 Guided Run Loop From Doctor Dogfood](23-guided-run-loop-from-doctor-dogfood.md)
- [24 Guided Trampoline](24-guided-trampoline.md)

## Product Pattern

The same pattern appears in multiple parts of the product.

```text
large or risky source
  -> CLI shape/map gate
  -> compact current view
  -> exact handle or phase
  -> local action
  -> structured progress/proof
  -> next CLI gate
```

The CLI is the conduit between the full artifact and the agent-facing response.
It does not hide detail. It keeps full detail in files and reports, then prints
only the next useful slice into chat.

## Part 1: Importing A Prose Skill Progressively

When SkillSpec analyzes a `SKILL.md` package for import, it should not ask the
agent to read the whole skill first. The import path starts with staging and
mapping.

For local source packages or public GitHub skill URIs:

```bash
skillspec source map <source-skill-or-github-uri> --out <draft>/.skillspec/source-map
skillspec source coverage <draft>/.skillspec/source-map/source-map.json
skillspec source query <draft>/.skillspec/source-map/source-map.json nodes --view index
skillspec source query <draft>/.skillspec/source-map/source-map.json dependencies --view summary
skillspec source stale <draft>/.skillspec/source-map/source-map.json --root <source-skill>
skillspec import-skill <source-skill> --out <draft>/skill.spec.yml --source-map <draft>/.skillspec/source-map/source-map.json
```

When the source is a GitHub URI, `source map` stages the repository through
SkillSpec's sparse checkout path and prints the selected local `source_path`.
Use that path for `source stale` and `import-skill`. If the URI resolves to
multiple `SKILL.md` packages, the command reports candidates instead of
guessing; the agent should ask the user which candidate to map. This prevents
agents from drifting into web search, raw GitHub URLs, or ad hoc sparse checkout
loops for normal imports.

The agent learns the source in parts:

- `source map` records files, headings, code blocks, references, hashes, and
  classifications.
- `source coverage` tells the agent where review is still needed.
- `source query` exposes exact handles instead of broad file reads.
- `source stale` proves the mapped source still matches before import.
- `import-skill --source-map` binds the generated draft to that source evidence.

This is the source-reading version of progressive disclosure. It protects token
budget, avoids accidental summarization of important middle sections, and keeps
the source map as evidence rather than scratch.

See [18 Source Map Progressive Reader](18-source-map-progressive-reader.md) and
[21 One-Shot Porting Workflow](21-one-shot-porting.md).

## Part 2: Using SkillSpec Progressively

When SkillSpec itself is invoked through a thin `SKILL.md` trampoline, the
agent should not read all of `skill.spec.yml` and then improvise. The trampoline
starts the guided loop:

```bash
skillspec run-loop ./skill.spec.yml \
  --input "<user task>" \
  --trace-dir "${PWD}/.skillspec/traces" \
  --guide agent
```

For resume after compaction:

```bash
skillspec run-loop ./skill.spec.yml --resume <run_dir> --guide agent --json
```

The guided loop prints:

- the selected route;
- matched rules;
- current phase;
- open requirements;
- allowed commands;
- forbids;
- next commands;
- resume anchor;
- end proof instructions.

The agent then completes the current gate, records progress, and asks the CLI
for the next gate. The important point is that the CLI guides the sequence. The
model does not need to remember every route, rule, phase, and closure from the
full YAML.

This is the runtime-guidance version of progressive disclosure. It reduces
agent-visible tokens while preserving deterministic execution state in the run
directory.

See [23 Guided Run Loop From Doctor Dogfood](23-guided-run-loop-from-doctor-dogfood.md)
and [24 Guided Trampoline](24-guided-trampoline.md).

## Part 3: Progress Is Recorded, Not Remembered

The progressive loop only works if completion state is not held in model memory.
SkillSpec records progress in trace artifacts:

```bash
skillspec progress record <run_dir> requirement-satisfied <phase> <requirement> \
  --evidence-kind <kind> \
  --evidence-ref <path-or-id>

skillspec progress batch <run_dir> \
  --file <run_dir>/evidence-batch.jsonl \
  --checkpoint "checkpointing evidence" \
  --quiet

skillspec progress show <skill.spec.yml> --run <run_dir> --quiet
```

Use `progress record` for a single exceptional row or a user-relevant failure.
Use `progress batch --quiet` when several successful routine proof rows are
ready at the same boundary. The command still completes synchronously, but the
transcript should stay focused on plain-language status while `execution.jsonl`
receives every granular event.

The run directory becomes the durable working memory:

```text
<run_dir>/
  trace.jsonl
  summary.json
  execution.jsonl
  guide-state.json
  guide-summary.md
  proof-digest.json
  alignment.json
```

This matters under compaction. If the harness summarizes prior turns, the next
agent step can resume from `guide-state.json`, `execution.jsonl`, and the trace,
not from a lossy memory of prior chat.

See [10 Runtime Plan Act Progress Loop](10-runtime-plan-act-progress-loop.md),
[11 Execution Progress Ledger](11-execution-progress-ledger.md), and
[12 Traces And Alignment](12-traces-and-alignment.md).

## Part 4: Completion Is A Gate, Not A Paragraph

At completion, the agent should not narrate a long audit loop. It should run a
quiet alignment gate, batch missing proof rows if needed, then report the final
result in plain language with artifact paths:

```bash
skillspec trace align <skill.spec.yml> \
  --decision-trace <run_dir> \
  --execution-trace <run_dir>/execution.jsonl \
  --proof-digest <run_dir>/proof-digest.json \
  --quiet
```

The final response should include the selected route, trace path, alignment
status or report path, token usage, and evidence paths. It should not dump the
alignment report unless debugging, failure, or the user asks for it.

This prevents the "progress parade" failure mode: many small visible alignment
reruns after each tiny proof row. The intended pattern is:

1. Run one quiet alignment pass with proof digest.
2. Batch real missing proof rows quietly.
3. Record final-response evidence quietly.
4. Run one final quiet alignment pass.
5. Report only the final result and paths.

See [13 Completion Alignment And Token Reporting](13-completion-alignment-and-token-reporting.md).

## Part 5: Workspace And Performance Loops Use The Same Idea

Workspace commands also follow the progressive pattern:

```bash
skillspec workspace map <source-root> --out <build>/skillspec.workspace.yml --summary
skillspec workspace validate <build>/skillspec.workspace.yml --summary
skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build> --summary
skillspec workspace converge <build>/skillspec.workspace.yml --build-root <workspace-build> --summary
skillspec workspace compile <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex-skill --summary
skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --dry-run --summary
```

The agent sees summary counts, report paths, cache hits, cache misses, and token
economy estimates. Full manifests, package reports, source maps, generated
drafts, loaders, install manifests, and proof artifacts stay on disk.

When the source root is a local Git checkout, the final import/install summaries
can add one more progressive handoff: recommend a pull request back to the
source skill repository after review, QA, compile, retired-skill install,
harness restart, and a real agent interaction with the SkillSpec-backed skill.
The recommendation should name the source repo detected from `.git`, the
generated contracts and proof artifacts to include, and a suggested branch
shape. It must remain advisory unless the user explicitly asks the agent to
create a branch, push, or open a PR.

This is the workspace version of the same product principle:

```text
print compact guidance into chat
preserve complete evidence on disk
```

See [19 Workspace Authoring Graph](19-workspace-authoring-graph.md) and
[20 Performance, Token Economy, And Incremental Processing](20-performance-token-speed.md).

## Environmental Risks The Loop Addresses

The progressive guidance system is designed around known operating constraints.

### Primacy And Position Bias

Long prompts are not used uniformly. Important middle sections can be underused
or lost. SkillSpec counters this by asking the CLI for the exact current route,
phase, handle, requirement, or proof gap instead of relying on one large read.

Relevant docs:

- [22 Doctor Agent Drift Risk](22-doctor-agent-drift-risk.md)
- [24 Guided Trampoline](24-guided-trampoline.md)

### Compaction

Compaction can summarize away details that mattered to the next step. SkillSpec
stores the current gate, trace, progress, proof digest, and alignment reports in
the run directory so the next turn can resume from persisted artifacts.

Relevant docs:

- [11 Execution Progress Ledger](11-execution-progress-ledger.md)
- [23 Guided Run Loop From Doctor Dogfood](23-guided-run-loop-from-doctor-dogfood.md)

### Token Budget

SkillSpec treats token economy as a product requirement. Summary commands print
estimated agent-visible tokens, artifact tokens preserved on disk, and avoided
tokens where applicable. This is distinct from measured model API usage.

Relevant docs:

- [13 Completion Alignment And Token Reporting](13-completion-alignment-and-token-reporting.md)
- [20 Performance, Token Economy, And Incremental Processing](20-performance-token-speed.md)

### Execution Drift

Agents can skip steps, use the wrong tool, or claim proof that does not exist.
SkillSpec counters this with route selection, phase tool boundaries, explicit
forbids, progress records, scenario tests, dependency checks, and trace
alignment.

Relevant docs:

- [06 Rules, Routes, And Decision Algebra](06-rules-routes-and-decision-algebra.md)
- [09 Phase Tool Boundaries](09-phase-tool-boundaries.md)
- [12 Traces And Alignment](12-traces-and-alignment.md)

## What Makes This Different From A Larger Prompt

A larger prompt can tell the agent what to do. SkillSpec's progressive guidance
can tell the agent what to do now, what it must not do now, what proof is still
missing, where the full evidence lives, and how to resume after context loss.

The innovation is not just that SkillSpec has a YAML contract. The innovation is
that the CLI acts as a live, deterministic guide over that contract:

```text
source stage/map/query
  guides import reading

doctor
  guides shape and drift-risk decisions

run-loop --guide agent
  guides route, phase, current gate, and resume

progress record/batch/show
  guides what is complete and what remains

trace align --quiet
  writes completion proof artifacts without transcript noise

workspace --summary
  guides large package graphs without dumping reports
```

This lets SkillSpec keep details where they belong: structured and inspectable
on disk, surfaced to the agent only when needed.

## Design Rule

When adding a new SkillSpec workflow, do not start by writing a longer
trampoline instruction. Add or reuse a CLI gate that can answer:

- What is the current shape?
- What is the current route or phase?
- What exact handle or file should be read next?
- What is allowed now?
- What is forbidden now?
- What proof must be recorded before moving on?
- What command resumes after compaction?
- Where is the full evidence preserved?

If the answer only exists in prose, the workflow is not finished.
