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
  "skill_id": "rote.shell",
  "spec_schema": "skillspec/v0",
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

## Event Kinds

V0 defines these decision events:

| Event | Meaning |
| --- | --- |
| `input_received` | The evaluator received the user input. |
| `spec_loaded` | The evaluator loaded a specific SkillSpec. |
| `rule_evaluated` | A rule was checked and either matched or did not match. |
| `rule_matched` | A rule matched and its effects are about to apply. |
| `route_selected` | A route was selected or replaced by a rule/default. |
| `route_order_set` | A rule replaced the route order. |
| `forbid_added` | A rule added forbidden substitutions. |
| `allow_added` | A rule added narrow allowed fallbacks. |
| `elicitation_requested` | A rule required a bounded user choice. |
| `after_success_scheduled` | A rule scheduled post-success action. |
| `outcome_recorded` | The evaluator recorded the final decision outcome. |

## CLI

Write a trace while making a decision:

```sh
skillspec decide examples/rote-shell.skill.spec.yml \
  --input='browse the active dashboard' \
  --trace-dir .skillspec/traces
```

Pass only the task text to `--input`. A harness should strip activation text
such as `/rote-shell-spec` or `$rote-shell-spec` before calling `skillspec`.
When invoking from a shell, prefer single quotes so `$skill-name` text is not
expanded by the shell.

Compact a run after a harness appends event files:

```sh
skillspec trace compact .skillspec/traces/run-1781900000000-12345
```

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
