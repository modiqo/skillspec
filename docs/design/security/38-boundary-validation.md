# Boundary Validation

Status: proposed. Nothing in this document is implemented.

## Purpose

Establish how much confidence is available that a boundary proposal from
document 37 is correct, and be precise about where that confidence runs out.

A least-privilege policy that breaks the skill is worse than no policy, because
the reader will disable it and lose the protection entirely. So the question
"does this boundary still let the skill work?" is the one that decides whether
the feature is usable.

## The Constraint That Shapes This Document

SkillSpec does not execute target skill code. This is an existing, deliberate
property of the analysis path: `docs/design/operations/27-public-doctor-reports.md`
describes doctor running against public GitHub skill URLs "without executing
target repo code," and the public report path depends on it.

Boundary validation does not get an exemption. Running a downloaded skill's
scripts to observe their effects would execute untrusted code from an arbitrary
repository, which is the exact thing the analysis exists to help people avoid
doing.

This rules out the most direct form of validation. What remains is weaker, and
the documents and CLI output must not describe it as though it were stronger.

An earlier framing of this feature described emitting a boundary and reporting
that some number of the skill's behaviors "still pass" under it. That is not
achievable under the no-execution constraint and should not be used in any
description of this work. What `skillspec test` runs today is routing-decision
scenario tests: `run_tests` in `crates/skillspec-runtime/src/decision.rs` calls
`decide` and compares the result against expected routes, route order, and plan
phases. It selects routes; it does not invoke tools. A boundary cannot change
its outcome, so it cannot validate one.

## Validation Levels

### V0: Internal Coverage. Always Available.

A static self-consistency check of the proposal against the effect surface it
came from. Every effect must be accounted for by exactly one of:

- an `allow` grant,
- a `permission_required_for` entry,
- an explicit `forbid`,
- an `unresolved` entry.

An effect matching none of these is a compiler defect. An effect matching more
than one is an ordering defect.

V0 also checks that:

- the proposal's `default` is `deny`;
- no host grant is a wildcard or a parent domain of another grant;
- no grant exists without at least one contributing effect id;
- `complete` is `false` whenever `unresolved` is non-empty.

V0 proves the proposal is internally consistent. It proves nothing about the
skill. It is cheap, it runs on every invocation, and its failures are bugs in
this crate rather than findings about the target.

### V1: Contract Cross-Check. Requires `skill.spec.yml`.

When the package has a valid contract, the contract declares surface the effect
extractor may have missed: `commands`, `dependencies`, `imports`, `resources`,
and any existing `tool_boundary`.

Cross-check both directions:

- **Contract surface not covered by the proposal.** A declared command the
  proposal would deny. This is the highest-value validation output available,
  because it identifies a break before the reader applies the policy.
- **Proposal grants with no contract basis.** An effect the extractor found that
  the contract does not account for. This is a finding about the skill, not about
  the proposal.

V1 is still entirely static. It is a comparison of two declarations plus one
extraction, and its value depends on the contract being maintained.

### V2: Recorded-Run Replay. Requires Prior Execution Evidence.

SkillSpec already records execution evidence for runs that used its runtime: the
progress ledger `execution.jsonl` described in
`docs/design/runtime/11-execution-progress-ledger.md`, and decision traces
described in `docs/design/runtime/12-traces-and-alignment.md`.

Where such evidence exists from a run that already happened, replay it against a
candidate boundary and report which recorded steps the boundary would have
refused. No new execution occurs; the evidence is read from disk.

This is the strongest validation the design can honestly offer, and it is the
only level that observes real behavior rather than declared behavior. Its limits
are equally clear:

- It only covers skills that were run through SkillSpec's runtime.
- It only covers the paths those particular runs took.
- The recorded evidence captures routes, phases, requirements, and proof events.
  Whether it captures enough effect detail to replay against a grant set needs to
  be established against the real ledger format before this level is built.
  Document 40 schedules that investigation before the implementation task.

V2 is therefore proposed with an explicit dependency: if the ledger does not
carry sufficient effect detail, either the ledger gains it or V2 is dropped. It
must not ship as a partial check that reports confident-looking results from
insufficient data.

### V3: Live Harness Observation. Out Of Scope.

Observing a real agent run under an applied boundary would validate the policy
properly. It requires a model, a real harness, and execution of the target skill.
`docs/design/operations/30-testing-matrix.md` already records which harness
behaviors cannot be automated, and this falls on that side of the line.

It is named here so that its absence is a stated scope decision rather than an
oversight.

## What A Reader Should Be Told

The human rendering must express the validation level actually achieved, in
terms that do not imply more:

```text
Validation
- internal coverage: pass (14 effects, all accounted for)
- contract cross-check: not available (no skill.spec.yml)
- recorded-run replay: not available (no execution evidence)

This boundary has not been checked against a real run of this skill.
```

And where V1 did run and found a conflict:

```text
Validation
- internal coverage: pass
- contract cross-check: 1 conflict
    command "publish" runs `gh release create`, which the proposal denies
    (no net.egress grant for the gh CLI's endpoints)
- recorded-run replay: not available
```

The phrase "validated boundary" should not appear anywhere in the CLI, the
reports, or the marketing surface unless V2 ran and passed. At V0 and V1 the
accurate word is "checked," and the check is of the proposal's consistency, not
of the skill's behavior.

## Consequence For The Product Claim

The differentiating claim available at ship time is narrower than
"we validate the boundary." It is:

```text
A least-privilege boundary, derived deterministically from an enumerated effect
surface, with every grant traceable to the line that caused it, and every
unresolvable effect named rather than guessed.
```

That is defensible with V0 and V1 alone. Whether the stronger claim becomes
available depends on the V2 investigation, and the roadmap should not be built on
the assumption that it will.

## Sources

- `crates/skillspec-runtime/src/decision.rs` - `run_tests`, `run_test`,
  `compare_expectation`; establishes what `skillspec test` actually checks.
- `crates/skillspec-runtime/src/progress.rs`,
  `crates/skillspec-runtime/src/trace.rs` - the evidence V2 would replay.
- `crates/skillspec-harness-lab/src/lab.rs` - sandbox homes and command
  construction available to integration tests.
- `docs/design/operations/27-public-doctor-reports.md` - the no-execution
  property this document inherits.
- `docs/design/operations/30-testing-matrix.md` - existing record of what cannot
  be automated at the harness boundary.
