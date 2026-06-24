# Completion Alignment And Token Reporting

Every SkillSpec-backed run should end with a concrete completion report.

The report should answer:

- What happened?
- What evidence can the user inspect?
- Did the run follow the selected route?
- Which requirements are proven?
- Which proof is missing?
- Were forbidden actions recorded?
- What token consumption and savings were measured?
- Where is the trace?

The report should not say only:

```text
unproven
```

That word is too ambiguous for users. A completion report must explain what is
good, what is incomplete, and what failed.

## Required Final Blocks

A SkillSpec-backed skill loader should require these final blocks:

```text
Result:
Evidence:
Alignment summary:
Token usage:
SkillSpec:
```

The exact rendering can vary by harness, but the information should not be
omitted.

## Alignment Summary Format

Use the compact summary produced by `skillspec trace align`.

The preferred human format is:

```text
Decision replay: pass
Phase order: pass
Requirements: 4/5 proven
Missing proof: requirement `install_codex` has no progress event
Forbidden actions: no violations recorded
Alignment: partial
```

This is intentionally plain. It separates the layers:

- decision replay;
- phase order;
- requirement proof;
- missing proof;
- forbidden actions;
- overall alignment.

If there are multiple missing proof rows, list each one.

## Status Language

The aligner still has three machine statuses:

- `pass`;
- `fail`;
- `unproven`.

The user-facing completion should translate incomplete non-failing runs to:

```text
Alignment: partial
```

Then it should include the missing proof rows.

Use this language:

- `pass`: decision replay passed and execution obligations are proven.
- `partial`: no deterministic decision check failed, but one or more execution
  proof rows are missing.
- `fail`: a deterministic decision check failed or execution evidence
  contradicted an obligation.

Avoid "unproven" as the only visible conclusion. It is acceptable inside a proof
row, but not as the whole summary.

## Decision Replay

Decision replay checks whether the current spec reproduces the captured decision
facts from the trace.

It can pass even when execution proof is incomplete.

A passing decision replay means:

- the captured input can be replayed;
- the selected route matches;
- matched rules match;
- route order, forbids, allows, elicitations, and after-success facts are
  consistent with the current spec, subject to available trace events.

It does not mean the task work was actually done.

## Execution Proof

Execution proof checks structured action evidence.

It consumes execution traces supplied through:

```sh
skillspec trace align ./skill.spec.yml \
  --decision-trace .skillspec/traces/<run-id> \
  --execution-trace .skillspec/traces/<run-id>/execution.jsonl
```

Execution proof can show:

- route fulfillment;
- phase completion;
- requirement satisfaction;
- after-success completion;
- forbidden action violations;
- missing proof rows;
- token consumption and savings evidence when recorded.

If the execution ledger is absent, execution proof should remain incomplete.
That is not a failure by itself. It means the system cannot prove the work from
structured evidence.

## Missing Proof Rows

A missing proof row should be specific.

Good:

```text
Missing proof: requirement `install_codex` has no progress event
```

Good:

```text
Missing proof: after-success `report_alignment_status` has no completion event
```

Bad:

```text
Missing proof: execution proof not checked
```

Bad:

```text
Alignment: unproven
```

The row should name the requirement, route, closure, elicitation, or forbid that
lacks proof and the expected evidence class.

## Token Usage

Token usage must always be rendered.

If no token stats were recorded:

```text
Token consumption: not recorded
Token savings: not recorded
```

Do not omit the section. Missing data is an important result.

When query-reduction stats exist, use precise language:

```text
Token consumption: 710190 query-result tokens recorded
Token savings: 3729702 tokens saved by query reduction
  (4439892 cached response tokens reduced to 710190 query-result tokens,
  84.0% reduction)
```

This avoids the misleading claim that cached response tokens were all consumed
in the prompt. The measured consumption is the extracted or used token surface.
The savings are the delta between cached response tokens and extracted
query-result tokens.

When rote workspace stats exist, report measured context-window or API tokens
separately from query-reduction savings:

```text
Token consumption: 752608 rote workspace data tokens recorded
Token savings: 3729702 tokens saved by query reduction
```

If a run has both stats, keep them distinct:

- workspace data tokens measure recorded evidence volume;
- query reduction measures avoided context loading;
- prompt or API tokens measure actual model-facing consumption when available.

Do not invent replay savings. Only report savings that the execution ledger or
alignment report can support.

## Evidence Storage

`skillspec trace align` writes:

```text
<run_dir>/alignment.json
```

The run directory can also contain:

- `trace.jsonl`;
- `summary.json`;
- `execution.jsonl`;
- `progress.json`;
- evidence files or references.

The completion report should name the trace path and the alignment report path.
The user should be able to inspect or archive the run later.

## Loader Requirements

Compiled SkillSpec loader files should instruct agents to:

- preserve the trace run directory;
- record progress after phase actions;
- run `progress show` before phase transitions;
- run `trace align` at completion;
- include alignment summary;
- include token usage;
- report missing proof rows;
- avoid a bare `unproven` result.

This keeps the final response consistent across ported skills.

## Harness Requirements

A harness should:

- pass `--execution-trace <run_dir>/execution.jsonl` to `trace align` when the
  ledger exists;
- preserve token stats in structured execution events;
- preserve query-reduction stats separately from prompt consumption;
- expose `alignment.json` as a durable artifact;
- fail or warn when a final answer omits the required alignment and token
  sections for a SkillSpec-backed run.

The model can render the report, but the harness should provide the data.

## Report Examples

### Full Pass

```text
Alignment summary:
Decision replay: pass
Phase order: pass
Requirements: 5/5 proven
Missing proof: none
Forbidden actions: no violations recorded
Alignment: pass

Token usage:
Token consumption: 710190 query-result tokens recorded
Token savings: 3729702 tokens saved by query reduction
  (4439892 cached response tokens reduced to 710190 query-result tokens,
  84.0% reduction)
```

### Partial

```text
Alignment summary:
Decision replay: pass
Phase order: pass
Requirements: 4/5 proven
Missing proof: requirement `install_codex` has no progress event
Forbidden actions: no violations recorded
Alignment: partial

Token usage:
Token consumption: not recorded
Token savings: not recorded
```

### Failure

```text
Alignment summary:
Decision replay: fail
Phase order: pass
Requirements: 3/5 proven
Missing proof: requirement `run_tests` has no progress event
Forbidden actions: violation recorded for `native_web_search`
Alignment: fail
```

## Source Alignment

This doc is grounded in:

- `crates/skillspec-cli/src/align.rs`, which defines alignment summaries,
  completion fields, token summaries, proof rows, and layer statuses;
- `crates/skillspec-cli/src/compiler.rs`, which emits loader instructions for
  alignment and token reporting;
- `docs/design/12-traces-and-alignment.md`, which explains decision traces and
  execution traces;
- `spec/commandspec.md`, which documents `trace align`.
