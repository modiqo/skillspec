# Decision Trace

Decision trace is the runtime evidence for how a harness or `skillspec` moved
from a user request to an outcome. It records steering decisions, not tool
payloads.

## Purpose

A prose skill can tell an agent what to do. A SkillSpec can prove which rule
caused the agent to do it.

Trace answers:

- which spec was loaded
- which rules were evaluated
- which rules matched
- which route won
- which forbidden substitutions were applied
- which bounded questions were required
- which post-success closures were scheduled
- which outcome was recorded

Trace does not store raw command output, browser snapshots, API responses, or
secret-bearing tool payloads. Those belong in the host harness or tool system.
Trace events may link to external evidence ids in future versions.

## Ownership

Rules trigger decisions. The evaluator writes events.

The spec declares the trace contract:

```yaml
trace:
  mode: event_log
  required: true
  record:
    - input_received
    - spec_loaded
    - rule_evaluated
    - rule_matched
    - route_selected
    - elicitation_requested
    - outcome_recorded
```

The writer can be:

- the `skillspec` CLI, when using `skillspec decide --trace-dir <dir>`
- an agent harness that interprets the spec directly
- a future SDK/runtime that exposes typed trace calls

When `trace.required` is true, the reference CLI rejects `decide` and `explain`
calls that omit `--trace-dir`.

The spec must not embed imperative "write this event file now" instructions in
each rule. That would turn a behavior contract into a logging script.

## Storage

Trace storage is event-sourced. Each event is its own file so parallel branches
do not thrash one mutable JSON document.

```text
.skillspec/traces/
  run-1781900000000-12345/
    events/
      000001.input_received.json
      000002.spec_loaded.json
      000003.rule_evaluated.json
      000004.rule_matched.json
      000005.route_selected.json
      000006.elicitation_requested.json
      000007.outcome_recorded.json
    trace.jsonl
    summary.json
```

Writers should write a temporary file and then atomically rename it into
`events/`. Compaction reads sorted event files and writes `trace.jsonl` and
`summary.json`.

## Event Envelope

Every event uses the same envelope:

```json
{
  "schema": "skillspec.trace/v0",
  "run_id": "run-1781900000000-12345",
  "seq": 4,
  "timestamp_unix_ms": 1781900000123,
  "skill_id": "durable.executor",
  "spec_schema": "skillspec/v0",
  "spec_fingerprint": "sha256:...",
  "input_sha256": "sha256:...",
  "event": "rule_matched",
  "event_name": "rule_matched",
  "data": {
    "event": "rule_matched",
    "rule_id": "browser_words_handoff_to_browse",
    "reason": "Browser intent must be satisfied by browser evidence."
  }
}
```

`seq` is monotonic inside a run. Future harnesses may add span ids for parallel
branches, but v0 does not require them.

`spec_fingerprint` hashes the resolved spec graph used for the decision,
including imported file contents. `input_sha256` hashes the task text from the
`input_received` event. Older traces may omit these fields; alignment tools
must report those checks as unproven instead of guessing.

## Event Kinds

V0 defines these decision events:

| Event | Meaning |
| --- | --- |
| `input_received` | The evaluator received the user input. |
| `spec_loaded` | The evaluator loaded a specific SkillSpec. |
| `rule_evaluated` | A rule was checked and either matched or did not match. |
| `rule_matched` | A rule matched and its effects are about to apply. |
| `route_selected` | A route was selected or replaced by a rule/default. Payload includes `route`, `basis`, optional `rule_id`, and optional `reason`. |
| `route_order_set` | A rule replaced the route order. |
| `forbid_added` | A rule added forbidden substitutions. |
| `allow_added` | A rule added narrow allowed fallbacks. |
| `elicitation_requested` | A rule required a bounded user choice. |
| `after_success_scheduled` | A rule scheduled post-success action. |
| `outcome_recorded` | The evaluator recorded the final decision outcome. |

`route_selected.basis` is one of:

- `rule_prefer`: a matched rule selected `prefer`.
- `route_order_default`: no rule selected `prefer`, but a matched rule replaced
  route order and the first route in that order was selected.
- `default_route_order`: no rule selected `prefer`; the evaluator selected the
  first route by rank/default order.

## CLI

Write a trace while making a decision:

```sh
skillspec decide examples/durable-executor/skill.spec.yml \
  --input='browse the active dashboard' \
  --trace-dir .skillspec/traces
```

Pass only the task text to `--input`. A harness should strip activation text
such as `/durable-executor-spec` or `$durable-executor-spec` before calling `skillspec`.
When invoking from a shell, prefer single quotes so `$skill-name` text is not
expanded by the shell.

Compact a run after a harness appends event files:

```sh
skillspec trace compact .skillspec/traces/run-1781900000000-12345
```

Align a spec with a decision trace:

```sh
skillspec trace align examples/durable-executor/skill.spec.yml \
  --decision-trace .skillspec/traces/run-1781900000000-12345 \
  --summary
```

Alignment re-runs the current spec against the captured input and compares the
trace to deterministic decision facts: skill id, schema, resolved-spec
fingerprint, input hash, selected route, route-selection basis, matched rules,
forbids, elicitations, and after-success closures. The human report begins with
a summary that names the selected route, route-selection basis, matched rules,
deterministic check counts, execution-obligation counts, and the grouped
unproven obligation kinds. It marks execution obligations as `unproven` unless
structured execution evidence is supplied by a future harness/export path; that
means the decision can be reproducible while route fulfillment, forbid
compliance, elicitations, and after-success closures still need execution proof.

## Independence From Tool Systems

SkillSpec trace is independent from rote, MCP, browser automation, shell
wrappers, or any particular agent harness. A trace event may say:

```json
{"event": "route_selected", "route": "browser_handoff"}
```

It should not inline the browser snapshot. If a harness has external evidence,
it can link by id:

```json
{"evidence": {"kind": "browser_snapshot", "ref": "browser:@page.4"}}
```

That link is host-specific. The decision trace remains portable.
