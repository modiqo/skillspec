# Traces And Alignment

Traces and alignment make SkillSpec reviewable after a run.

A decision trace records how the spec steered the task. An alignment report
replays that decision against the current spec and checks which execution
obligations have proof.

The key boundary is:

- decision traces prove decision facts;
- execution traces prove what actually happened;
- alignment separates those layers instead of pretending one proves the other.

## Decision Traces

A decision trace is an event log for `skillspec decide` or a compatible harness.
It records stable decision facts, not payload-heavy execution evidence.

Trace event kinds include:

- `input_received`;
- `spec_loaded`;
- `rule_evaluated`;
- `rule_matched`;
- `route_selected`;
- `route_order_set`;
- `forbid_added`;
- `allow_added`;
- `elicitation_requested`;
- `after_success_scheduled`;
- `outcome_recorded`.

The trace envelope deliberately stores decision facts and stable references. It
does not store command output, browser snapshots, API responses, secrets, or
other large tool payloads. Those belong in separate execution evidence.

## Trace Configuration

The spec can declare:

- `trace.mode`;
- `trace.required`;
- `trace.record`.

The only current trace mode is `event_log`.

When `trace.required` is true, `skillspec decide` and `skillspec explain` require
`--trace-dir`. Without that argument, the CLI rejects the call.

When `trace.record` is empty or omitted, all decision events are written. When
`trace.record` names event kinds, only those events are selected for the trace.

## Trace Storage

`skillspec decide <spec> --input '<task>' --trace-dir <dir>` writes a new run
directory under the trace root.

A trace run contains:

- `events/`, with one JSON file per selected event;
- `trace.jsonl`, a compact JSONL stream;
- `summary.json`, a compact summary.

Each event envelope includes:

- `schema`, currently `skillspec.trace/v0`;
- `run_id`;
- monotonic `seq`;
- `timestamp_unix_ms`;
- `skill_id`;
- `spec_schema`;
- `spec_fingerprint`;
- `input_sha256`;
- `event`;
- `event_name`;
- event-specific `data`.

The spec fingerprint includes the resolved spec plus local imported files. The
envelope metadata includes `input_sha256`; event-specific `data` can still carry
the payload for events such as `input_received`.

## Trace Compaction

`skillspec trace compact <run_dir>` rebuilds `trace.jsonl` and `summary.json`
from the per-event files.

Compaction is useful when a harness writes or moves event files and wants a
stable JSONL stream and summary for later review.

## Alignment

`skillspec trace align <spec> --decision-trace <run_dir>` compares the current
spec with a decision trace.

The aligner:

1. reads the decision trace envelopes;
2. replays the current spec against the captured input facts;
3. compares deterministic decision facts;
4. derives execution obligations from the selected route and matched decision
   output;
5. reads optional structured execution trace files;
6. evaluates which obligations are proven, failed, or unproven;
7. returns an alignment report.

The report schema is `skillspec.align/v0`.

The report includes:

- `ok`;
- `status`;
- `summary`;
- `spec`;
- `decision_trace`;
- `execution_traces`;
- deterministic `checks`;
- execution `obligations`;
- user-facing `proof_rows`.

`skillspec trace align` writes the full report to `<run_dir>/alignment.json`.
This keeps the completion summary and token usage with the run artifacts
instead of leaving them only in terminal output.

`ok` is true when no deterministic check failed. A report can have `ok: true`
and `status: unproven` when decision replay succeeded but execution proof is
missing.

## Alignment Status

Alignment has three statuses:

- `pass`: every deterministic decision check passed and every active execution
  obligation has structured proof.
- `fail`: at least one deterministic check failed, or execution evidence
  contradicted an obligation.
- `unproven`: no deterministic check failed, but one or more checks or execution
  obligations lack proof.

This status model prevents false confidence. A traced decision can be perfectly
reproducible while the actual work remains unproven.

The human completion summary should not stop at a bare `unproven`. For a
non-failing incomplete report, render `Alignment: partial` and include the
specific missing proof rows. The compact completion block is:

```text
alignment_summary:
  Decision replay: pass
  Phase order: pass
  Requirements: 4/5 proven
  Missing proof: requirement `install_codex` has no progress event
  Forbidden actions: no violations recorded
  Alignment: partial
token_usage:
  Token consumption: total 1234 tokens
  Token savings: 3729702 tokens saved by query reduction (4439892 cached response tokens reduced to 710190 query-result tokens, 84.0% reduction)
```

Token usage must be present even when stats are absent. In that case the values
are `not recorded`, not omitted. When query-reduction fields are present,
reported savings are the difference between cached response tokens and extracted
query-result tokens.

## Alignment Layers

Alignment has two layers:

- `decision_replay`;
- `execution_proof`.

`decision_replay` measures whether the current spec reproduces the captured
decision facts.

`execution_proof` measures whether structured execution evidence proves the
obligations implied by the selected route and matched rules.

The summary reports counts for both layers.

## Execution Obligations

The aligner derives execution obligations from the decision:

- selected route fulfillment;
- selected route `checks`;
- forbid compliance;
- required elicitation fulfillment or waiver;
- after-success fulfillment;
- direct user requirements inferred by the aligner.

Every obligation starts unproven. Structured execution evidence can satisfy,
partially satisfy, violate, or leave it unproven.

Without execution evidence, route fulfillment, forbid compliance, elicitations,
and after-success work generally remain unproven even when decision replay
passes.

## Execution Trace Evidence

Execution evidence is separate from the decision trace. The aligner can read
JSONL or JSON array execution trace files supplied through `--execution-trace`.

The current execution ledger parser recognizes structured fields such as:

- `event`;
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
- `anonymous`;
- `included_result`;
- `included_alignment`;
- `included_evidence`;
- `included_token_savings`;
- `fallback_needed`;
- `matches_len`;
- `id`.

The aligner does not parse arbitrary human-readable transcripts as proof. A
harness that wants strong alignment should emit structured execution events.

## Proof Rows And Evidence Gaps

Alignment reports include user-facing proof rows. A proof row connects:

- the requirement;
- the obligation;
- expected evidence;
- observed evidence;
- proof status;
- explanation.

The summary also includes evidence gaps. Evidence gaps explain what proof is
missing and whether the gap belongs to decision trace evidence or execution
obligations.

These fields are intended for self-reflection. An agent should report not only
what it did, but also which parts are proven and which remain unproven.

## Practical Runtime Pattern

For a real task:

```sh
skillspec decide skill.spec.yml --input '<task>' --trace-dir .skillspec/traces
```

After execution:

```sh
skillspec trace align skill.spec.yml --decision-trace .skillspec/traces/<run-id>
```

When structured execution evidence exists:

```sh
skillspec trace align skill.spec.yml \
  --decision-trace .skillspec/traces/<run-id> \
  --execution-trace execution-ledger.jsonl
```

The final report should include:

- selected route;
- trace run directory;
- compact alignment summary;
- token consumption and savings, or `not recorded`;
- alignment status and status meaning;
- decision replay layer result;
- execution proof layer result;
- evidence gaps;
- proof rows.

## Common Misreadings

Do not treat a trace as proof that tools were used correctly. A trace proves
decision events.

Do not treat `ok: true` as full success. If status is `unproven`, execution
proof is missing.

Do not paste raw command output or secrets into decision traces. Use separate
execution evidence with appropriate redaction.

Do not claim a forbid was respected unless execution evidence supports that
claim.

Do not claim after-success work completed unless execution evidence supports it.

## Source Alignment

This doc is grounded in:

- `spec/trace.md`, which defines the decision trace contract and the alignment
  purpose;
- `crates/skillspec-cli/src/trace.rs`, which writes trace envelopes, run
  directories, compact JSONL, summaries, event selection, spec fingerprints, and
  input hashes;
- `crates/skillspec-cli/src/main.rs`, which enforces `trace.required` for
  `decide` and `explain`;
- `crates/skillspec-cli/src/align.rs`, which defines the alignment report,
  statuses, layers, obligations, proof rows, execution ledger, and pass/fail/
  unproven rules;
- `crates/skillspec-cli/src/decision.rs`, which emits decision events for trace
  writing.
