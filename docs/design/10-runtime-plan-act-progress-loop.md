# Runtime Plan Act Progress Loop

The runtime loop is the main way an agent should execute a SkillSpec-backed
skill.

The preferred operator-facing loop is now:

```text
run-loop --guide agent -> follow current gate -> record progress -> resume -> align
```

The lower-level primitive loop remains:

```text
sensemake -> plan -> act -> record progress -> show progress -> align
```

The primitive short form is:

```text
plan -> act -> progress
```

The loop exists because a route decision is not enough. A selected route tells
the agent what strategy applies. The phase plan tells the agent what order to
follow. The action checklist tells the agent what is allowed now. The progress
ledger records what actually happened. Alignment then compares the decision and
the execution proof.

## Current Implementation

The current CLI implements these runtime commands:

```sh
skillspec run-loop <spec> --input '<task>' --trace-dir .skillspec/traces --guide agent
skillspec run-loop <spec> --resume .skillspec/traces/<run-id> --guide agent
skillspec sensemake <spec> --view index
skillspec plan <spec> --input '<task>' --trace-dir .skillspec/traces
skillspec act <spec> --input '<task>' --run .skillspec/traces/<run-id> --phase <phase-id>
skillspec progress record .skillspec/traces/<run-id> <event> [phase] [requirement]
skillspec progress show <spec> --run .skillspec/traces/<run-id>
skillspec trace align <spec> \
  --decision-trace .skillspec/traces/<run-id> \
  --execution-trace .skillspec/traces/<run-id>/execution.jsonl \
  --summary \
  --proof-digest .skillspec/traces/<run-id>/proof-digest.json
```

`plan`, `act`, and `progress` are not generic workflow execution commands.
They are navigation, gating, and evidence commands for the harness and model.
The agent still executes the actual task work through the surrounding harness,
subject to harness policy and user approvals.

## Why The Loop Is Needed

Earlier SkillSpec use could stop at:

```sh
skillspec decide skill.spec.yml --input '<task>'
```

That proves the decision engine can select a route, but it does not give the
agent a visible operating procedure for the current run. It also does not tell a
reviewer which phase should have happened first, what was allowed in that phase,
or what proof was recorded after execution.

The runtime loop fixes that by making the run visible:

- `plan` lists the ordered phases for the selected route.
- `act` expands one phase into an action checklist.
- `progress record` appends structured evidence after work happens.
- `progress show` derives completed, current, blocked, and remaining phases.
- `trace align` turns decision and execution evidence into a completion report.

The agent should treat the loop as the active operating procedure, not as
background documentation.

## Command Roles

### `sensemake`

`sensemake` orients the agent to the spec shape without loading the entire YAML.

Use it when the spec is unfamiliar:

```sh
skillspec sensemake ./skill.spec.yml --view index
```

The output gives section counts, ids, and navigation handles for routes, rules,
states, dependencies, commands, recipes, closures, tests, imports, resources,
code, artifacts, snippets, and elicitations. The agent should then use `query`
and `refs` for active handles instead of dumping the whole spec into context.

`sensemake` is optional when the harness already knows the spec shape, but it is
recommended for agent-mediated work because it reduces prompt loading and helps
the agent avoid inactive material.

### `plan`

`plan` evaluates the task against the current spec, writes an optional decision
trace, and renders the selected route's ordered phase plan.

```sh
skillspec plan ./skill.spec.yml \
  --input '<task>' \
  --trace-dir .skillspec/traces
```

The output includes:

- selected route;
- route-selection basis;
- trace run directory when `--trace-dir` is supplied;
- ordered phases;
- owner skill for each phase;
- phase descriptions when declared;
- phase requirements;
- phase forbids;
- current phase;
- required transitions.

The printed `run_dir` is a durable handle for the rest of the run. The harness
must preserve it and pass it to later commands.

`plan` derives phase order from the selected route's `execution_plan`. When a
selected route has no execution plan, the selected route is the active scope and
there are no phase transitions to track.

### `act`

`act` turns the selected route and current phase into an action checklist.

```sh
skillspec act ./skill.spec.yml \
  --input '<task>' \
  --run .skillspec/traces/<run-id> \
  --phase <phase-id>
```

The output includes:

- selected route and route-selection reason;
- route authority;
- current phase;
- OODA checklist;
- matched rules;
- effective phase tool boundary;
- allowed actions now;
- active forbids;
- required elicitations;
- required transitions;
- after-success work;
- before-tool-call questions.

The important detail is that `act` is phase-specific. It is not enough to run it
once at the beginning of a multi-phase route and then act from memory. The agent
should run it for the phase it is about to execute, read the checklist, execute
only allowed work, record progress, then repeat for the next phase.

### `progress record`

`progress record` appends one structured execution event to the run ledger:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  phase-started <phase-id> \
  --source-skill <skill-id> \
  --message 'starting phase from skillspec act checklist'
```

The ledger path is:

```text
.skillspec/traces/<run-id>/execution.jsonl
```

The current CLI supports these event names:

- `phase-started`;
- `requirement-started`;
- `requirement-satisfied`;
- `requirement-failed`;
- `obligation-satisfied`;
- `route-fulfilled`;
- `after-success-completed`;
- `evidence-attached`;
- `handoff-started`;
- `handoff-completed`;
- `phase-completed`;
- `phase-blocked`.

The event may include:

- `phase`;
- `requirement`;
- `id`;
- `status`;
- `evidence.kind`;
- `evidence.ref`;
- `source.skill`;
- `message`;
- timestamp;
- run id.

Progress events should be recorded after actual work, not before. A progress
event is a proof claim. If the work did not happen or the evidence is missing,
the event should not claim success.

### `progress batch`

`progress batch` appends many structured execution events from one JSONL file or
JSON array:

```sh
skillspec progress batch .skillspec/traces/<run-id> \
  --file .skillspec/traces/<run-id>/final-proof.jsonl \
  --checkpoint "checkpointing evidence" \
  --summary
```

Use it near the end of a run when the agent needs to record several proof rows:
route fulfillment, after-success closures, elicitation approvals, evidence
attachments, and no-violation proof for forbids. This keeps the ledger exact
while avoiding a visible progress parade. Use the same foreground checkpoint
shape at natural boundaries after dry-run/planning, after mutation, after
verification, before route fulfillment, and before final alignment. The
user-facing update should be one gate-level note, such as
`[checkpointing evidence...]`; individual successful proof rows should stay in
the JSONL batch and ledger, not the transcript.

Each JSONL row is the same execution event shape used by `progress record`. The
CLI fills missing `schema`, `run_id`, and timestamp fields and normalizes event
names from hyphen form to underscore form.

### `progress show`

`progress show` reads the decision trace and `execution.jsonl`, then writes a
derived `progress.json`:

```sh
skillspec progress show ./skill.spec.yml \
  --run .skillspec/traces/<run-id>
```

The output reports:

- completed phases;
- current phase;
- blocked phases;
- remaining phases;
- open requirements for the current phase;
- execution ledger presence;
- event count;
- forbidden-action summary.

This command is the phase tracker. It answers two practical questions:

- What should the agent do next?
- What proof has already been recorded?

The agent should run `progress show` after each phase action and before moving
to another phase, but treat the output as an internal gate check. In normal chat
updates, surface only the gate result unless the user asks for details or a
blocker/failure needs evidence.

### `trace align`

`trace align` compares the current spec with the decision trace and optional
execution ledger:

```sh
skillspec trace align ./skill.spec.yml \
  --decision-trace .skillspec/traces/<run-id> \
  --execution-trace .skillspec/traces/<run-id>/execution.jsonl \
  --summary \
  --proof-digest .skillspec/traces/<run-id>/proof-digest.json
```

The compact output separates two layers:

- decision replay;
- execution proof.

Decision replay checks whether the current spec reproduces the captured route
decision. Execution proof checks whether structured events prove the route,
requirements, forbids, elicitations, and after-success obligations.

Alignment writes:

```text
.skillspec/traces/<run-id>/alignment.json
```

The final agent response should use the compact completion summary from this
report instead of saying only `unproven`.

## The Full Runtime Loop

A harness-mediated run should follow this sequence.

### 1. Load The Thin Loader

The generated `SKILL.md` is a trampoline. It tells the agent to use the colocated
`skill.spec.yml` and the runtime commands.

The loader is not a second behavior source. If the loader and spec disagree, the
spec and compiled command output control the run.

### 2. Orient With `sensemake`

For unfamiliar specs:

```sh
skillspec sensemake ./skill.spec.yml --view index
```

The agent should note the relevant route, rule, state, command, recipe, closure,
dependency, import, and resource ids. It should not load unrelated imports or
resources.

### 3. Create The Phase Plan

```sh
skillspec plan ./skill.spec.yml \
  --input '<task>' \
  --trace-dir .skillspec/traces
```

The harness must preserve the run directory printed by the command. This run
directory is the durable container for:

- decision events;
- `trace.jsonl`;
- `summary.json`;
- `execution.jsonl`;
- `progress.json`;
- `alignment.json`;
- optional external evidence references.

### 4. Expand The Current Phase

```sh
skillspec act ./skill.spec.yml \
  --input '<task>' \
  --run .skillspec/traces/<run-id> \
  --phase <phase-id>
```

The agent must read the full checklist. The checklist is the phase SOP.

The action that follows must satisfy three filters:

- it is inside the phase requirements or selected route scope;
- it is not forbidden by the decision, route, phase, or handoff;
- it is allowed by the effective phase tool boundary, or the user has granted
  explicit permission.

### 5. Record What Happened

After action, append progress evidence:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  requirement-satisfied <phase-id> <requirement-id> \
  --evidence-kind command \
  --evidence-ref '<evidence-ref>' \
  --source-skill '<skill-id>'
```

Then complete or block the phase:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  phase-completed <phase-id> \
  --status pass \
  --evidence-kind trace \
  --evidence-ref '<evidence-ref>'
```

If the phase cannot continue:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  phase-blocked <phase-id> \
  --status blocked \
  --message '<plain blocker>'
```

The message should describe the blocker without hiding missing proof. A blocked
event is better than claiming success from an unproven action.

### 6. Check Progress Before Moving On

```sh
skillspec progress show ./skill.spec.yml \
  --run .skillspec/traces/<run-id>
```

If a next phase exists, run `act` for that phase and repeat the loop. If all
phases are complete, run alignment. Do not paste the full progress report into
the user-visible transcript unless it is needed for a decision.

### 7. Align And Report

```sh
skillspec trace align ./skill.spec.yml \
  --decision-trace .skillspec/traces/<run-id> \
  --execution-trace .skillspec/traces/<run-id>/execution.jsonl \
  --summary \
  --proof-digest .skillspec/traces/<run-id>/proof-digest.json
```

Use the digest to avoid a visible re-alignment loop. If alignment reports
several missing route, route-check, requirement, elicitation, forbid/no-violation,
or closure proof rows, create `.skillspec/traces/<run-id>/final-proof.jsonl`,
append it once with `skillspec progress batch`, then rerun alignment once. Do
not rerun alignment after each individual proof row.

The final response should include:

- result;
- evidence references;
- alignment summary;
- token usage;
- trace path.

It should not report a bare `unproven`. If proof is incomplete, say which proof
row is missing and report `Alignment: partial`.

## Phase Order Semantics

The phase order comes from the selected route's `execution_plan`.

For an ordered plan:

- the first non-completed, non-blocked phase is current;
- a phase should be completed or blocked before the next phase starts;
- `progress show` determines current and remaining phases from the plan plus the
  execution ledger.

If a phase has `jumps`, `act` renders the jump conditions in required
transitions. A harness or agent may take a declared jump only when the condition
is true and the evidence is recorded.

If a phase has a handoff, `act` renders the handoff boundary. A `stop_current_skill`
handoff means the current skill should stop executing domain actions except to
pass the declared context.

## Command Ownership

Each phase has an `owner_skill`.

The owner skill is the active skill for that phase. The agent may still use
SkillSpec CLI navigation and declared commands, dependencies, imports, and
resources, but it should not silently switch to another skill, provider, tool, or
execution substrate outside the active boundary.

If a different skill is required, the spec should declare a handoff or the agent
should ask the user for permission.

## Relationship To `decide`

`plan` and `act` are built on the same decision engine as `decide`.

`decide` remains useful when a caller only needs machine-readable route
selection. `plan` is better for agent execution because it renders the route as
ordered phases. `act` is better for immediate tool use because it renders the
phase-specific checklist and effective tool boundary.

The preferred runtime sequence for harnessed work is:

```text
plan first, act before action, progress after action
```

## Relationship To `query` And `refs`

`act` intentionally does not dump every referenced object. When the checklist is
not specific enough, the agent should use precise handles:

```sh
skillspec query ./skill.spec.yml rule:<id> --view summary
skillspec refs ./skill.spec.yml route:<id> --view summary
skillspec query ./skill.spec.yml command:<id>.requires
skillspec query ./skill.spec.yml test:<name>.expect --view full
```

This preserves progressive disclosure. The phase checklist tells the agent what
is active; `query` and `refs` pull only the active details.

## Harness Responsibilities

SkillSpec can render the plan and record evidence, but the harness remains
responsible for:

- enforcing product-level approvals;
- deciding whether a tool can run;
- running commands, browser actions, API calls, adapters, or local processes;
- preventing access to unavailable substrates;
- redacting secrets;
- assigning durable evidence references;
- appending truthful progress events;
- including execution traces in alignment.

The CLI does not execute arbitrary task work. A command template in the spec is
a declaration, not permission to run it.

## Failure Modes

The loop is designed to expose failures instead of hiding them.

If no route is selected, the agent should ask for the missing task shape or use
only a declared fallback route.

If the current phase requires a dependency that fails, record a blocked phase or
ask for permission to install, authenticate, or use a different route.

If the next tool is outside the phase boundary, stop and ask for permission.

If a phase action happened but evidence was not captured, record only what can
be proven. Do not mark the requirement satisfied.

If `trace align --summary` returns incomplete execution proof, report the exact missing
proof rows.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-cli/src/cli/dispatch.rs`, which wires `plan`, `act`, `progress`, and
  `trace align`;
- `crates/skillspec-cli/src/execution/act.rs`, which renders phase plans, action
  checklists, OODA loops, transitions, and effective tool boundaries;
- `crates/skillspec-cli/src/execution/progress.rs`, which writes `execution.jsonl` and
  derives `progress.json`;
- `crates/skillspec-cli/src/execution/align.rs`, which reads decision traces and optional
  execution traces;
- `spec/commandspec.md`, which documents the CLI surface.
