# Execution Progress Ledger

The progress ledger is the structured record of what happened after a SkillSpec
decision.

The decision trace says what the spec selected. The progress ledger says what
the harness and agent did next.

The ledger lives in the trace run directory:

```text
.skillspec/traces/<run-id>/execution.jsonl
```

Each line is a JSON event. `skillspec progress show` reads those events and
derives phase progress. `skillspec trace align` can read the same file as
execution proof.

## Why The Ledger Exists

Decision traces are not execution proof.

A route decision can say:

- use route `install_reviewed_skill`;
- run phase `qa`;
- satisfy requirement `validate_spec`;
- avoid forbidden action `install_before_dependency_surface_approval`.

That proves the spec produced obligations. It does not prove the agent validated
the spec, completed QA, avoided a forbidden tool, or installed anything.

The ledger is the bridge between decision and execution. It gives the harness a
small, structured way to say:

- this phase started;
- this requirement was satisfied;
- this handoff happened;
- this evidence file or response id proves the action;
- this phase completed or blocked.

Without the ledger, alignment can still replay the decision, but execution proof
remains incomplete.

## Ledger File

`skillspec progress record` appends events to:

```text
<run_dir>/execution.jsonl
```

`skillspec progress show` also writes a derived report to:

```text
<run_dir>/progress.json
```

`progress.json` is a view. `execution.jsonl` is the append-only proof ledger.

## Event Shape

The current event schema is `skillspec.execution.v1`.

Fields include:

- `schema`;
- `run_id`;
- `event`;
- `phase`;
- `requirement`;
- `id`;
- `status`;
- `evidence`;
- `source`;
- `at_unix_ms`;
- `message`.

`evidence` is an object with:

- `kind`;
- `ref`.

`source` is currently an object with:

- `skill`.

The event shape is intentionally small. The ledger should store proof handles,
not full command output, browser snapshots, secrets, or bulky tool responses.

## Supported Event Types

The CLI currently accepts:

| Event | Use |
| --- | --- |
| `phase-started` | The agent began a declared phase. |
| `requirement-started` | Work began for a phase requirement. |
| `requirement-satisfied` | A phase requirement has proof. |
| `requirement-failed` | A phase requirement failed. |
| `obligation-satisfied` | A non-phase obligation has proof. |
| `route-fulfilled` | The selected route has been fulfilled. |
| `after-success-completed` | A scheduled closure completed. |
| `evidence-attached` | Extra evidence was attached to the run. |
| `handoff-started` | A declared handoff started. |
| `handoff-completed` | A declared handoff completed. |
| `phase-completed` | A phase completed. |
| `phase-blocked` | A phase cannot continue. |

These event names are CLI-facing kebab case. The JSON rows store the normalized
snake-case event name.

## Recording Phase Progress

Start a phase after reading `skillspec act` for that phase:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  phase-started <phase-id> \
  --source-skill <skill-id> \
  --message 'phase checklist read'
```

Record each proven requirement:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  requirement-satisfied <phase-id> <requirement-id> \
  --evidence-kind command \
  --evidence-ref '<command-output-ref>' \
  --source-skill <skill-id>
```

Complete the phase only after its active requirements are satisfied or
explicitly waived by the harness or user:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  phase-completed <phase-id> \
  --status pass \
  --evidence-kind trace \
  --evidence-ref '<phase-evidence-ref>' \
  --source-skill <skill-id>
```

Block a phase when a dependency, approval, auth step, missing input, or forbidden
tool prevents truthful completion:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  phase-blocked <phase-id> \
  --status blocked \
  --message 'needs user approval for unlisted tool'
```

## Recording Route And Closure Proof

Some obligations are not individual phase requirements.

Use `--id` for route, closure, elicitation, or other obligation ids:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  route-fulfilled \
  --id <route-id> \
  --status pass \
  --evidence-kind file \
  --evidence-ref '<artifact-path>'
```

For after-success work:

```sh
skillspec progress record .skillspec/traces/<run-id> \
  after-success-completed \
  --id <closure-id> \
  --status pass \
  --evidence-kind trace \
  --evidence-ref '<closure-evidence-ref>'
```

Use `obligation-satisfied` for obligations that do not map cleanly to a phase
requirement, route, or closure but still need proof.

## Evidence References

Evidence references should be durable enough for a reviewer to inspect later.

Good evidence references include:

- a relative file path under the workspace;
- a trace file path;
- a rote response id;
- a harness execution id;
- a command transcript id;
- an artifact path;
- an adapter call id;
- a browser snapshot id.

Weak evidence references include:

- "done";
- "I ran it";
- a full terminal transcript pasted into the message;
- a path outside the run or workspace that will not survive review;
- a secret-bearing output.

The ledger should name evidence, not contain all evidence.

## Progress Derivation

`skillspec progress show` derives progress by combining:

- the decision trace input;
- a fresh decision against the current spec;
- the selected route's phase list;
- events in `execution.jsonl`.

It writes:

```text
<run_dir>/progress.json
```

The report includes:

- selected route;
- completed phases;
- current phase;
- blocked phases;
- remaining phases;
- open requirements;
- per-phase requirement status;
- execution proof summary.

The current phase is the first phase in the selected route's plan that is not
completed and not blocked.

Requirements are satisfied when the ledger contains
`requirement-satisfied <phase> <requirement>`. They fail when the ledger contains
`requirement-failed <phase> <requirement>`.

## Progress Is Not Alignment

`progress show` is a tracker. It does not claim final alignment.

It can tell the agent:

- there is a ledger;
- how many events are present;
- which phase is current;
- which requirements remain open;
- whether forbidden-action events were recorded.

`trace align` is the command that turns decision and execution evidence into an
alignment report.

## Alignment Consumption

Run alignment with the execution ledger:

```sh
skillspec trace align ./skill.spec.yml \
  --decision-trace .skillspec/traces/<run-id> \
  --execution-trace .skillspec/traces/<run-id>/execution.jsonl
```

The aligner reads structured execution events and marks obligations as proven,
violated, or unproven.

The current aligner recognizes the progress events above and also understands
additional structured execution fields used by harness ledgers, such as:

- `command`;
- `executor`;
- `through_rote`;
- `operation_kind`;
- `execution_mode`;
- `workspace`;
- `response_id`;
- `lease_id`;
- `exit_code`;
- `timed_out`;
- `stdout_captured`;
- `stderr_captured`;
- `ready`;
- `included_result`;
- `included_alignment`;
- `included_evidence`;
- `included_token_savings`.

This lets the same alignment command consume both SkillSpec progress events and
broader harness execution events when those fields exist.

## Truthfulness Rules

The ledger is only useful if events are conservative.

Do not record `requirement-satisfied` until the evidence exists.

Do not record `phase-completed` when a required phase action was skipped.

Do not record `route-fulfilled` just because the route was selected.

Do not record `after-success-completed` when closure work is still pending.

If proof is missing, leave the requirement open or record a blocked event.

If an action used a forbidden tool or unapproved boundary escape, record the
violation as execution evidence instead of hiding it. Alignment should fail or
show the violation; that is the correct outcome.

## Storage And Retention

The ledger belongs under the run directory because progress is per-run,
fungible execution data.

The design intent is:

- keep skill contracts in the skill package;
- keep run decisions under `.skillspec/traces/<run-id>`;
- keep execution progress under the same run directory;
- keep bulky artifacts in workspace files or harness stores and reference them
  from the ledger.

Completed runs can later be compacted, archived, or mined for learning, but the
runtime commands should treat the run directory as the active source of progress
truth.

## Harness Responsibilities

A harness integrating SkillSpec should:

- create or preserve the run directory from `plan`;
- call `act` before tool use;
- block or ask permission for unlisted tools;
- assign stable evidence refs;
- append ledger events after actual work;
- run `progress show` before phase transitions;
- pass `execution.jsonl` to `trace align`;
- surface missing proof rows to the user.

The harness should not ask the model to remember progress from prose alone. The
ledger is the machine-readable progress memory.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-cli/src/execution/progress.rs`, which defines progress reports,
  execution events, event recording, and progress derivation;
- `crates/skillspec-cli/src/cli/args.rs` and
  `crates/skillspec-cli/src/cli/dispatch.rs`, which expose `progress show` and
  `progress record`;
- `crates/skillspec-cli/src/execution/align.rs`, which consumes structured execution
  traces for obligation proof;
- `spec/commandspec.md`, which lists the CLI arguments and event values.
