# Guided Run Loop From Doctor Dogfood

Status: implemented on `doctor-agent-drift-risk`
Owner: SkillSpec
Related docs:
- [22 Doctor Agent Drift Risk](22-doctor-agent-drift-risk.md)
- [24 Guided Trampoline](24-guided-trampoline.md)

## Purpose

This document explains why SkillSpec added guided `run-loop` mode and how it
was built as a direct consequence of dogfooding `skillspec doctor` on the
SkillSpec skill itself.

The short version:

```text
Doctor showed that our own trampoline had become high load-bearing prose.
We moved the load-bearing navigation from SKILL.md into the CLI.
The CLI now guides the agent one gate at a time, persists state, and resumes
from trace instead of depending on model memory.
```

This is not just a command convenience. It is a design correction. SkillSpec
argues that critical agent behavior should not hide inside prose. The original
`skills/skillspec/SKILL.md` had started to violate that principle.

## The Dogfood Finding

We ran:

```sh
skillspec doctor skills/skillspec/
```

The important finding was not that the skill was undiscoverable. The
frontmatter was adequate. The issue was that after activation, the trampoline
was asking the model to carry too much operational policy before it had even
selected a route.

The bad shape looked like this:

```text
shape_kind: simple_skill
frontmatter_discovery_risk: low
agent_drift_risk: critical
activation: about 175 lines, about 5885 tokens
modal obligations: 40
late modal obligations: 12
high findings:
- large_activation_body
- instruction_density
- implicit_dependency_contract
```

That exposed a circular problem:

```text
SkillSpec exists because high load-bearing prose is unreliable.
The SkillSpec trampoline had become high load-bearing prose.
```

The fix was not another rewrite of the same long prompt. A better long prompt
would still rely on the agent remembering and applying policy from an activated
Markdown body. The fix was to move traversal intelligence into software.

## Design Move

The design move was:

```text
SKILL.md becomes a boot pointer.
skill.spec.yml remains the contract.
skillspec run-loop --guide agent becomes the runtime navigator.
run_dir becomes the memory anchor.
```

The trampoline should not know every route, rule, phase, command, dependency,
or closure. It should know only how to start or resume the guide:

```sh
skillspec run-loop ./skill.spec.yml \
  --input '<user task>' \
  --trace-dir "${PWD}/.skillspec/traces" \
  --guide agent

skillspec run-loop ./skill.spec.yml \
  --resume <run_dir> \
  --guide agent
```

The CLI then reads the contract, selects the route, opens a trace run, computes
the current gate, and tells the agent what to do next.

## What We Innovated

The useful innovation is not batching alone. Plain `run-loop` already reduced
repeated spec parsing by combining sensemake, decide, plan, and act.

The stronger idea is guided execution:

```text
The CLI does not merely print information.
It becomes a progressive guide over the contract.
```

Guided `run-loop` gives the agent:

- the start anchor: what route was selected and why;
- the path: what ordered phases exist and which are done;
- the current gate: what is open now;
- next commands: which commands are allowed and useful now;
- load-more handles: what to query only if needed;
- the end anchor: what proof must exist before final response;
- the resume command: how to continue after compaction or interruption.

The agent no longer has to infer the first route from prose or scan the full
YAML. It asks the CLI and follows the current gate.

## Runtime Contract

The guided run loop has four surfaces.

### 1. Start

Start proves that the task has been interpreted against the contract.

It includes:

```text
spec
run_dir
input_sha256
spec_fingerprint
decision_fingerprint
selected_route
route_selection
matched_rules
route_candidates_seen
first_phase
current_phase
```

The key idea is that route selection is a computed event, not a paragraph in a
prompt. The selected route and matched rules come from the same decision logic
used by `decide`, `plan`, and `act`.

### 2. Path

Path gives the ordered execution shape:

```text
phase_order
completed_phases
blocked_phases
remaining_phases
required_transitions
```

This is the antidote to skipped-step drift. The model does not have to remember
the entire phase structure from a long activation body. The CLI prints the
current route path every time the guide starts or resumes.

### 3. Current Gate

Current gate is the heart of the design.

It answers:

```text
What should the agent do now?
What is still open?
What is forbidden now?
What commands are allowed now?
What should the agent load only if needed?
What progress row should be recorded before advancing?
```

The current gate includes:

```text
phase
owner_skill
route_scope
description
open_requirements
checks
do_now
do_not
allowed_now
allowed_commands
recommended_queries
progress_to_record
when_to_advance
```

This is intentionally narrow. It does not print the full spec. It prints the
next useful slice of the spec.

### 4. End

End gives the proof contract:

```text
selected route is fulfilled or intentionally partial
required checks passed or proof gaps are named
progress evidence is recorded in execution.jsonl
final-response evidence is recorded
compact alignment summary is generated
```

The end anchor prints the concrete commands for final progress and alignment.
It keeps final proof explicit without narrating every proof-row command to the
user.

## Persistent Resume Model

Guided `run-loop` is deliberately not dependent on model memory.

It persists:

```text
<run_dir>/decision.jsonl or compact decision trace events
<run_dir>/execution.jsonl
<run_dir>/guide-state.json
<run_dir>/guide-summary.md
<run_dir>/progress.json
<run_dir>/alignment.json
<run_dir>/proof-digest.json
```

The most important files are:

- `decision trace`: what task was routed and what route/rules were selected;
- `execution.jsonl`: what the agent actually recorded as progress;
- `guide-state.json`: machine-readable current guide state and fingerprints;
- `guide-summary.md`: compact human resume note for context compaction.

Resume works by reading persisted trace and progress, not by trusting the
agent's remembered state.

Resume flow:

```text
1. read run_dir
2. compact/read decision trace
3. recover original input from trace
4. verify input_sha256
5. reload current skill.spec.yml
6. recompute decision
7. compute spec_fingerprint and decision_fingerprint
8. compare with prior guide-state when present
9. read execution.jsonl through progress show
10. compute current phase
11. render current gate
12. rewrite guide-state.json and guide-summary.md
```

If the spec changed, resume is conservative:

- if the decision fingerprint changed, block resume and ask for a new run;
- if the spec fingerprint changed but the decision fingerprint remains stable,
  warn and continue;
- if no prior guide state exists, require selected route stability at minimum.

This keeps resume useful without pretending that every spec edit is safe.

## Why Trace Is The Right Memory Anchor

The trace is better than chat history because it is:

- durable across compaction;
- deterministic enough to replay;
- inspectable by commands;
- linked to proof artifacts;
- independent of whether the model remembered a previous paragraph.

The run directory is the execution memory. The model is the worker. The CLI is
the guide. The spec is the contract.

## Relationship To Existing Commands

Guided `run-loop` does not replace the primitive commands. It orchestrates them
into a safer default path.

| Primitive | Role Under Guide |
| --- | --- |
| `sensemake` | Still useful for unfamiliar specs and progressive map/query orientation. |
| `decide` | Decision logic used by start and resume. |
| `plan` | Phase order logic used by path. |
| `act` | Current-route/current-phase checklist used by current gate. |
| `progress record` | Records proof rows before advancing. |
| `progress show` | Computes completed/current/blocked/remaining state. |
| `query` | Loads exact handles named by the guide. |
| `refs` | Loads relationship edges for active handles. |
| `trace align` | Produces final alignment summary and proof digest. |

The important policy is:

```text
Use guided run-loop as the default agent-facing entry point.
Use primitive commands for debugging, explicit inspection, and proof details.
```

## Why This Saves Tokens

The old trampoline spent tokens teaching the model how to traverse SkillSpec.
The new trampoline spends tokens telling the model to ask the CLI.

That changes the cost profile:

```text
Old path:
load long SKILL.md -> remember policy -> infer route -> load more YAML -> act

New path:
load thin SKILL.md -> call run-loop guide -> follow current gate -> query only
needed handles
```

The savings are not only from fewer tokens in `SKILL.md`. They also come from:

- avoiding full `skill.spec.yml` reads;
- avoiding repeated command help lookups;
- avoiding repeated alignment/progress narration;
- avoiding route and phase inference in the model;
- avoiding reloading prior context after compaction.

## Why This Improves Alignment

The guide makes the agent's next action falsifiable:

- selected route is printed;
- matched rules are printed;
- current phase is printed;
- open requirements are printed;
- forbidden actions are printed;
- allowed commands are printed;
- progress rows are named;
- final proof commands are printed.

That means a reviewer can ask:

```text
Did the agent follow the current gate?
Did it use an allowed command?
Did it record the required progress?
Did final alignment pass or name the missing proof?
```

The old trampoline could only hope the model remembered the policy. The guided
loop gives a concrete checkpoint.

## Keeping The Trampoline Optimal

The trampoline should never become the skill again.

The allowed contents of `skills/skillspec/SKILL.md` are:

- frontmatter `name`;
- frontmatter `description`;
- command to start guided run-loop;
- command to resume guided run-loop;
- instruction to follow the printed current gate;
- instruction to use `query`/`refs` only for handles named by the guide;
- final-response instruction tied to the guide end anchor;
- fallback instruction if the CLI guide is unavailable.

The trampoline must not contain:

- route lists;
- rule lists;
- command catalogs;
- dependency policy;
- router lifecycle policy;
- workspace fanout details;
- durable-executor handoff policy;
- YAML authoring rules;
- final alignment cleanup recipes;
- long examples;
- code blocks;
- duplicated text from `skill.spec.yml`;
- instructions that compete with the CLI guide.

If a future change wants to add operational policy to the trampoline, the
default answer is:

```text
Put it in skill.spec.yml or the CLI guide, not SKILL.md.
```

## Trampoline Fitness Gates

Every meaningful change to the SkillSpec self skill should dogfood doctor:

```sh
skillspec doctor skills/skillspec/
```

Expected self-skill shape:

```text
shape_kind: simple_skill
frontmatter_discovery_risk: low
agent_drift_risk: low
raw_activation_risk: low
contract_mitigation: strong
residual_agent_drift_risk: low
large_surface: 0% activation-loaded
```

Expected contract shape:

```text
routes > 0
rules > 0
commands > 0
dependencies > 0
tests > 0
```

Expected activation shape:

```text
short SKILL.md
no fenced code blocks
no route catalog
no command catalog
no policy manual
no duplicated spec sections
```

If doctor reports medium or higher activation risk on the self trampoline, the
change should be treated as a design regression until justified.

## CI And Review Recommendations

The immediate review checklist:

```sh
cargo test -p skillspec --test cli run_loop_guide
cargo test -p skillspec --test cli sensemake_and_query_teach_progressive_navigation
skillspec validate skills/skillspec/skill.spec.yml
skillspec test skills/skillspec/skill.spec.yml
skillspec run-loop skills/skillspec/skill.spec.yml \
  --input 'what is the shape of this skill' \
  --trace-dir /tmp/skillspec-guide-test \
  --guide agent
skillspec doctor skills/skillspec/
```

Longer term, CI should add a self-trampoline budget test:

- fail if `skills/skillspec/SKILL.md` grows beyond a configured line/token
  budget;
- fail if it gains fenced code blocks;
- fail if doctor reports high or critical raw activation risk;
- fail if `skill.spec.yml` is invalid;
- fail if contract mitigation is absent or weak;
- warn if frontmatter discovery risk rises above low;
- warn if `sensemake` stops advertising guided start/resume.

These gates should be policy-backed, not vibes. The exact thresholds can evolve
under `skillspec_policy_v1`, but the invariant stays the same:

```text
The trampoline stays thin.
The CLI stays the guide.
The spec stays the contract.
The trace stays the memory.
```

## What Success Looks Like

After the redesign, dogfood should look like this:

```text
skillspec doctor: skills/skillspec/
verdict: low reliability debt
agent_drift_risk: low
raw_activation_risk: low
contract_mitigation: strong
residual_agent_drift_risk: low
activation: small
```

And guided run-loop should answer a shape question like this:

```text
selected_route: inspect_source_shape
current_phase: run_shape_doctor
open_requirements: doctor_source_shape
do_not: port, import, compile, install
next command: skillspec doctor <source>
resume: skillspec run-loop <spec> --resume <run_dir> --guide agent
```

That is the behavioral improvement: the CLI selects the right route, names the
current action, blocks unrelated work, and leaves a durable resume anchor.

## Open Questions

- Should the self-trampoline doctor budget become a required CI test before
  every release?
- Should `run-loop --guide agent` record an explicit guide-start event into
  `execution.jsonl`, or is `guide-state.json` enough?
- Should final proof batching become a first-class guide command rather than an
  end-anchor instruction?
- Should resume support spec migrations with explicit compatibility metadata?
- Should doctor consume guide-state files to score whether a SkillSpec-backed
  run is staying in low-risk mode?
