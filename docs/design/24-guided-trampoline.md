# SkillSpec Guided Trampoline Design

Status: implemented on `doctor-agent-drift-risk`
Owner: SkillSpec
Primary branch: `doctor-agent-drift-risk`
Target reader: implementer, reviewer, and agent operator

## One-Line Decision

Move SkillSpec traversal intelligence out of the activated `SKILL.md`
trampoline and into a stateful CLI guide. The trampoline should only start or
resume the guide; the CLI should select the route, expose the current gate,
persist the resume anchor, and tell the agent what to load next.

## Problem

SkillSpec exists because long, load-bearing prose skills are hard for agents to
follow, test, and prove. The current `skills/skillspec/SKILL.md` has started to
recreate the same problem.

Dogfood output from:

```bash
skillspec doctor skills/skillspec/
```

Current observed shape:

```text
shape_kind: simple_skill
frontmatter_discovery_risk: low
agent_drift_risk: critical
activation: 175 lines, about 5885 tokens
modal obligations: 40
late modal obligations: 12
high findings:
- large_activation_body
- instruction_density
- implicit_dependency_contract
```

The frontmatter is discoverable enough. The problem begins after activation:
the trampoline asks the model to remember too much operational policy before it
has used the contract.

This is circular:

```text
SkillSpec argues against high load-bearing prose.
The SkillSpec trampoline became high load-bearing prose.
```

The fix is not to write a slightly better long `SKILL.md`. The fix is to make
the CLI guide the agent through the spec progressively.

## Research And Platform Grounding

The design is based on these practical constraints:

- Agent Skills use progressive disclosure: metadata first, then `SKILL.md`, then
  additional files only when needed.
  Reference: https://agentskills.io/
- The Agent Skills package guidance recommends keeping the activated
  instruction body small and moving detail into on-demand resources.
  Reference: https://agentskills.io/specification
- Claude Code skill content remains in context after invocation. Its docs also
  describe skill handling during compaction, including reattaching the first
  5000 tokens of recent skill content within a shared budget.
  Reference: https://code.claude.com/docs/en/skills
- "Lost in the Middle" shows that models do not use long context uniformly and
  can underuse information placed away from the beginning or end.
  Reference: https://arxiv.org/abs/2307.03172
- RULER shows effective usable context can be smaller than the advertised
  context window and degrades with length and task complexity.
  Reference: https://arxiv.org/abs/2404.06654
- SkillsBench argues for focused, checkable skills over broad, exhaustive
  documentation-style skills.
  Reference: https://arxiv.org/abs/2602.12670

Implication:

```text
The activated trampoline should be a boot pointer.
The CLI should be the guide.
The spec should be the contract.
The run directory should be the memory anchor.
```

## Goals

1. Reduce `skills/skillspec/SKILL.md` from a large operational guide into a thin
   trampoline.
2. Preserve or improve current execution precision.
3. Make the start, middle, and end of a SkillSpec-guided run explicit.
4. Persist enough state for safe resume after compaction.
5. Avoid forcing agents to read the full `skill.spec.yml`.
6. Keep the user-facing experience compact.
7. Preserve detailed proof and evidence on disk.
8. Keep compatibility with current `plan`, `act`, `progress`, `trace align`,
   `query`, `refs`, and `sensemake` behavior.

## Non-Goals

- Do not remove the structured detail currently in `skill.spec.yml`.
- Do not make `SKILL.md` smart.
- Do not require a background daemon.
- Do not replace `plan`, `act`, `progress`, `query`, or `refs`.
- Do not make resume depend on model memory.
- Do not make `sensemake` dump the whole spec by default.
- Do not hide proof. Hide the parade, not the evidence.

## Core Model

The split of responsibility should be:

```text
SKILL.md
  knows how to start or resume the CLI guide

skillspec CLI
  knows how to read, route, guide, resume, and summarize the contract

skill.spec.yml
  knows routes, rules, phases, dependencies, forbids, checks, commands,
  closures, tests, proof, and trace requirements

run_dir
  stores trace, progress, guide state, proof digest, alignment, and final
  evidence needed to survive compaction

agent
  follows the current gate printed by the CLI
```

The trampoline should not know the first route. It should call the CLI. The CLI
reads `skill.spec.yml`, selects the route, creates or resumes the trace, infers
the current gate, and prints the next allowed step.

## Start, Middle, End

Every guided run must expose two anchors and one current gate.

### Start Anchor

The start is not the first command in the whole skill. The start is the selected
route plus the first authorized gate.

Start means:

```text
The task was understood.
The spec was loaded.
The route was selected.
The run directory was opened.
The first/current phase is known.
The next action boundary is known.
```

Required start fields:

```text
spec:
run_dir:
input_sha256:
spec_fingerprint:
selected_route:
route_selection_basis:
matched_rules:
route_candidates_seen:
first_phase:
current_phase:
```

`route_candidates_seen` is only orientation. The agent normally follows one
selected route.

### Middle Gate

The middle is guided one gate at a time.

Middle means:

```text
Here is the current phase.
Here is what is open.
Here is what is allowed now.
Here is what is forbidden now.
Here are the smallest commands to run next.
Here is how to checkpoint progress evidence compactly.
```

Required current-gate fields:

```text
current_phase:
phase_description:
open_requirements:
do_now:
do_not:
allowed_commands:
recommended_queries:
progress_to_record:
when_to_advance:
```

### End Anchor

The end is the completion contract for the selected route.

End means:

```text
The route is fulfilled or intentionally partial.
Required checks have passed or gaps are named.
Progress evidence is recorded.
Final-response evidence is recorded.
Alignment summary exists.
Token usage or token economy is reported or explicitly not recorded.
```

Required end fields:

```text
done_when:
route_fulfillment_event:
final_progress_command:
alignment_command:
final_response_must_include:
proof_paths:
```

There are two end concepts:

```text
route end
  what makes the selected route complete

conversation end
  what must be reported to the user before the final answer
```

Both must be visible in the guide.

## New CLI UX

### Start A Guided Run

```bash
skillspec run-loop ./skill.spec.yml \
  --input '<user task>' \
  --trace-dir "${PWD}/.skillspec/traces" \
  --guide agent
```

Behavior:

1. Load the spec once.
2. Run `sensemake` internally without dumping full handles by default.
3. Run decision routing.
4. Write a decision trace if `--trace-dir` is present.
5. Build the phase plan.
6. Build the action checklist for the first phase unless `--phase` is passed.
7. Read progress if an execution ledger already exists for that run.
8. Write guide state.
9. Print compact guide output.

### Resume A Guided Run

```bash
skillspec run-loop ./skill.spec.yml \
  --resume <run_dir> \
  --guide agent
```

Behavior:

1. Read `<run_dir>/trace.jsonl` or compact trace events.
2. Recover original input from `input_received`.
3. Recover stored `spec_fingerprint` and `input_sha256`.
4. Recompute current `spec_fingerprint`.
5. Recompute the decision from the recovered input.
6. Compare old and new route/phase contract.
7. Read `<run_dir>/execution.jsonl`.
8. Infer completed phases, blocked phases, current phase, remaining phases, and
   open requirements.
9. Write refreshed `guide-state.json` and `guide-summary.md`.
10. Print compact guide output for the current gate.

If the input changed, resume must refuse and recommend starting a new run.

If the spec fingerprint changed, resume must warn. If the selected route,
matched rules, phase order, forbids, or elicitations changed, resume must
require replanning before continuing.

### Query More Detail

The guide should name specific handles. The agent should only query those
handles unless blocked or asked for more detail.

Examples:

```bash
skillspec query ./skill.spec.yml route:<selected-route> --view summary
skillspec refs ./skill.spec.yml route:<selected-route> --view summary
skillspec query ./skill.spec.yml command:<command-id>.requires
skillspec deps check ./skill.spec.yml --command <command-id>
```

### Full Guide Escape Hatch

The CLI should provide an explicit full guide option for debugging and expert
inspection:

```bash
skillspec run-loop ./skill.spec.yml \
  --resume <run_dir> \
  --guide full
```

The trampoline must not call `--guide full` by default.

## Guide Output Contract

Default human output for `--guide agent` should be compact and stable.

Example:

```text
SkillSpec guide

START
- spec: ./skill.spec.yml
- run_dir: .skillspec/traces/run-123
- selected_route: remote_skill_port
- route_selection: rule_prefer via route_remote_sources_to_remote_port
- matched_rules: route_remote_sources_to_remote_port
- route_candidates_seen: 20
- first_phase: approve_remote_source
- current_phase: approve_remote_source

PATH
approve_remote_source -> stage_remote_source -> local_porting_loop -> qa -> final_proof

CURRENT GATE
- phase: approve_remote_source
- purpose: approve read-only access before staging a remote source
- open_requirements: approve_remote_source_access

DO NOW
- confirm remote source access or report that approval is missing
- if approved, run `skillspec source stage <uri> --json`
- use `selected_source_path` or a chosen `candidates[].source_path` for doctor,
  source map, and import

DO NOT
- do not install from the remote checkout
- do not execute imported snippets
- do not search the web or fetch raw GitHub files to locate the same URI
- do not import before source shape is classified

NEXT COMMANDS
- skillspec source stage <uri> --out <staging-root> --json
- skillspec doctor <selected_source_path>
- skillspec progress batch <run_dir> --file <run_dir>/evidence-batch.jsonl --checkpoint "checkpointing evidence" --summary

LOAD MORE ONLY IF NEEDED
- skillspec query ./skill.spec.yml route:remote_skill_port --view summary
- skillspec refs ./skill.spec.yml route:remote_skill_port --view summary

END
- done_when: source shape classified; route obligations complete or explicitly partial; final-response evidence recorded; compact alignment summary generated
- final_progress: skillspec progress final-response <run_dir> --phase <phase-id> --requirement <requirement-id> --result --evidence --alignment --token-savings
- align: skillspec trace align ./skill.spec.yml --decision-trace <run_dir> --execution-trace <run_dir>/execution.jsonl --summary --proof-digest <run_dir>/proof-digest.json

RESUME
- skillspec run-loop ./skill.spec.yml --resume <run_dir> --guide agent
```

JSON output must expose the same fields with stable names.

## Data Model

### Existing Trace State

Current decision trace already stores:

```text
run_id
skill_id
spec_schema
spec_fingerprint
input_sha256
input_received
route_selected
route_order_set
rule_matched
forbid_added
elicitation_requested
after_success_scheduled
outcome_recorded
```

Current `spec_fingerprint` implementation:

```text
sha256("skillspec.resolved_spec/v0\n" + serialized SkillSpec model + imported file contents)
```

It includes:

```text
- resolved SkillSpec model
- local file imports declared by `imports`
```

It excludes:

```text
- execution.jsonl
- progress.json
- guide-state.json
- proof-digest.json
- alignment.json
- timestamps
- generated reports
```

Current `input_sha256` implementation:

```text
sha256(original user task text)
```

### Existing Execution State

Current execution progress is stored in:

```text
<run_dir>/execution.jsonl
```

Important events:

```text
phase_started
requirement_started
requirement_satisfied
requirement_failed
stats_collected
obligation_satisfied
route_fulfilled
route_check_completed
after_success_completed
elicitation_answered
elicitation_waived
evidence_attached
handoff_started
handoff_completed
phase_completed
phase_blocked
final_response_sent
forbidden_action
forbidden_action_observed
forbid_violated
```

`skillspec progress show` already combines trace and execution state to infer:

```text
selected_route
completed_phases
current_phase
blocked_phases
remaining_phases
open_requirements
execution_proof
```

### New Guide State

Add:

```text
<run_dir>/guide-state.json
<run_dir>/guide-summary.md
```

`guide-state.json` is the machine-readable resume anchor.

Proposed schema:

```json
{
  "schema": "skillspec.guide-state/v0",
  "run_id": "run-123",
  "run_dir": ".skillspec/traces/run-123",
  "spec_path": "./skill.spec.yml",
  "spec_id": "skillspec",
  "spec_fingerprint": "sha256:...",
  "input_sha256": "sha256:...",
  "decision_fingerprint": "sha256:...",
  "cli_version": "0.x.y",
  "mode": "start|resume",
  "guide": "agent",
  "selected_route": "remote_skill_port",
  "route_selection": {
    "basis": "rule_prefer",
    "rule_id": "route_remote_sources_to_remote_port",
    "reason": "..."
  },
  "route_candidates_seen": 20,
  "matched_rules": [
    "route_remote_sources_to_remote_port"
  ],
  "phase_order": [
    "approve_remote_source",
    "stage_remote_source",
    "local_porting_loop",
    "qa",
    "final_proof"
  ],
  "completed_phases": [],
  "blocked_phases": [],
  "remaining_phases": [
    "stage_remote_source",
    "local_porting_loop",
    "qa",
    "final_proof"
  ],
  "first_phase": "approve_remote_source",
  "current_phase": "approve_remote_source",
  "open_requirements": [
    "approve_remote_source_access"
  ],
  "current_gate": {
    "phase": "approve_remote_source",
    "description": "Approve read-only access before staging a remote source.",
    "do_now": [
      "Confirm source access or report that approval is missing.",
      "If approved, stage the URI with skillspec source stage and continue from the returned local source path."
    ],
    "do_not": [
      "Do not install from remote checkout.",
      "Do not execute imported snippets.",
      "Do not search the web or fetch raw GitHub files to locate the same URI."
    ],
    "allowed_commands": [
      "skillspec doctor <source>",
      "skillspec progress batch <run_dir> --file <run_dir>/evidence-batch.jsonl --checkpoint \"checkpointing evidence\" --summary"
    ],
    "recommended_queries": [
      "skillspec query ./skill.spec.yml route:remote_skill_port --view summary"
    ],
    "progress_to_record": [
      {
        "event": "requirement_satisfied",
        "phase": "approve_remote_source",
        "requirement": "approve_remote_source_access"
      }
    ],
    "when_to_advance": [
      "All open requirements are satisfied or explicitly failed/blocked."
    ]
  },
  "end_anchor": {
    "done_when": [
      "selected route fulfilled or intentionally partial",
      "required checks passed or gaps reported",
      "progress evidence recorded",
      "final-response evidence recorded",
      "compact alignment summary generated"
    ],
    "route_fulfillment_event": "route_fulfilled",
    "final_progress_command": "skillspec progress final-response <run_dir> ...",
    "alignment_command": "skillspec trace align ./skill.spec.yml --decision-trace <run_dir> --execution-trace <run_dir>/execution.jsonl --summary --proof-digest <run_dir>/proof-digest.json",
    "final_response_must_include": [
      "result",
      "evidence",
      "alignment summary",
      "token usage",
      "SkillSpec route/run_dir/status"
    ]
  },
  "resume_command": "skillspec run-loop ./skill.spec.yml --resume .skillspec/traces/run-123 --guide agent",
  "generated_at_unix_ms": 1234567890
}
```

`guide-summary.md` is the compact human reminder to survive context compaction.
It should include only:

```text
run_dir
selected_route
current_phase
open_requirements
next commands
end anchor
resume command
```

## Decision Fingerprint

Add `decision_fingerprint`.

Purpose:

```text
The full spec may change in a way that does not affect the active route.
The decision fingerprint tells us whether the selected execution contract
changed.
```

Recommended hash input:

```text
schema: skillspec.decision/v0
input_sha256
selected_route
route_selection_basis
matched_rules
route_order
phase_order
phase requirements
phase forbids
phase tool boundaries
route checks
selected-route handoff
forbids from matched rules
elicitations from matched rules
after_success closures from matched rules
dependency ids referenced by active route/phases/commands
```

Resume behavior:

```text
same input_sha256 + same spec_fingerprint + same decision_fingerprint
  safe resume

same input_sha256 + changed spec_fingerprint + same decision_fingerprint
  warn that spec changed but active route appears stable; allow continue with warning

same input_sha256 + changed decision_fingerprint
  require re-plan before continuing

changed input_sha256
  refuse resume; start a new run
```

## `run-loop --guide agent` Implementation

### CLI Args

Extend `run-loop` args:

```rust
RunLoop {
    path: PathBuf,
    input: Option<String>,
    resume: Option<PathBuf>,
    view: SenseViewArg,
    trace_dir: Option<PathBuf>,
    phase: Option<String>,
    guide: Option<GuideModeArg>,
    json: bool,
}

enum GuideModeArg {
    Agent,
    Full,
}
```

Validation:

```text
--input and --resume are mutually exclusive
--resume requires existing run_dir
--trace-dir is required with --input when guide state should be persisted
--phase may override inferred current phase, but guide output must say it was user-selected
```

### Internal Types

Create a dedicated guide module rather than bloating `run_loop.rs`.

Suggested files:

```text
crates/skillspec-cli/src/features/guide.rs
crates/skillspec-cli/src/features/guide/types.rs
crates/skillspec-cli/src/features/guide/state.rs
crates/skillspec-cli/src/features/guide/render.rs
crates/skillspec-cli/src/features/guide/fingerprint.rs
```

Suggested top-level structs:

```rust
pub struct GuideBuildOptions<'a> {
    pub spec: &'a SkillSpec,
    pub spec_path: &'a Path,
    pub input: Option<&'a str>,
    pub resume_run_dir: Option<&'a Path>,
    pub trace_dir: Option<&'a Path>,
    pub phase_override: Option<&'a str>,
    pub guide_mode: GuideMode,
}

pub struct GuideReport {
    pub schema: String,
    pub mode: GuideStartMode,
    pub start: StartAnchor,
    pub path: GuidePath,
    pub current_gate: CurrentGate,
    pub end: EndAnchor,
    pub resume: ResumeAnchor,
    pub warnings: Vec<GuideWarning>,
    pub state_paths: GuideStatePaths,
}

pub enum GuideMode {
    Agent,
    Full,
}

pub enum GuideStartMode {
    Start,
    Resume,
}
```

Keep these types serializable. Use enums for statuses, warning kinds, and
resume safety.

### Build Algorithm

Start mode:

```text
1. load spec
2. decide_with_events(spec, input)
3. write decision trace if trace_dir supplied
4. build act report for first phase or phase override
5. read existing execution ledger if trace exists
6. build progress snapshot
7. build guide report
8. write guide-state.json
9. write guide-summary.md
10. render guide output
```

Resume mode:

```text
1. compact/read decision trace from run_dir
2. recover original input
3. recover stored spec_fingerprint and input_sha256
4. compute current spec_fingerprint
5. decide_with_events(spec, recovered input)
6. build decision_fingerprint
7. read prior guide-state.json if present
8. compare fingerprints
9. read execution.jsonl
10. build progress snapshot
11. infer current phase unless phase override provided
12. build guide report
13. write refreshed guide-state.json
14. write guide-summary.md
15. render guide output
```

## `sensemake` Changes

Current `sensemake --view index` prints every route, rule, state, dependency,
command, recipe, closure, and test id. For large specs this is useful but too
large as a default orientation surface.

Change default behavior:

```text
index
  counts, roles, top navigation, route count, command count, selected next
  orientation commands, no exhaustive id lists

summary
  include important ids, route labels, command categories, test count, and
  section summaries

handles
  new optional view that prints all handles without full values

full
  current full behavior, including exhaustive handles and richer detail
```

If adding `handles` is too much for the first implementation, keep existing
`full` for exhaustive output and make `index` compact.

Update docs and help accordingly.

## Trampoline Rewrite

Target `skills/skillspec/SKILL.md` should be under 800 to 1200 tokens. Aim for
about 500 to 700 tokens if possible.

Proposed trampoline:

```markdown
---
name: skillspec
description: "Use for inspecting, porting, proving, installing, routing, or managing SkillSpec-backed skills and skill workspaces."
---

# SkillSpec

Start the SkillSpec guide:

```bash
skillspec run-loop ./skill.spec.yml --input '<user task>' --trace-dir "${PWD}/.skillspec/traces" --guide agent
```

Resume an existing run:

```bash
skillspec run-loop ./skill.spec.yml --resume <run_dir> --guide agent
```

Follow the printed current gate. The selected route, matched rules, forbids,
allowed commands, open requirements, and end proof from the CLI guide are
authoritative.

Use `skillspec query` and `skillspec refs` only for handles named by the guide.
Do not read the full spec unless the guide, a blocker, or the user asks for it.

Before the final response, follow the guide's end anchor: record final-response
evidence, run compact alignment, and report result, evidence, alignment summary,
token usage, selected route, and run directory.

If the CLI is unavailable, read `skill.spec.yml` directly and manually follow
the same route, rule, phase, dependency, forbid, proof, and completion contract.
Report that CLI guidance was unavailable.
```

The exact wording can be tightened, but the trampoline must not restate all
routes, completion report rules, dependency rules, workspace rules, or import
rules. Those belong in `skill.spec.yml` and guide output.

## Preserve Current Precision

Current precision must move, not disappear.

| Current precision | New home |
| --- | --- |
| Route selection | `skill.spec.yml` routes/rules plus `run-loop --guide agent` start anchor |
| Phase order | `execution_plan` plus guide path |
| Forbids | rules, route/phase forbids, guide current gate |
| Dependency checks | dependencies and command `requires`, guide allowed commands |
| Import/port sequencing | route execution plans and recipes |
| Workspace fanout/converge rules | workspace routes/recipes and guide path |
| Token economy | CLI summary metrics and final proof guide |
| Alignment loop discipline | end anchor and proof digest guidance |
| Final response shape | end anchor and closure/proof definitions |
| Resume after compaction | `guide-state.json` and `guide-summary.md` |

## Doctor Changes

`doctor` should distinguish raw prose risk from contract mitigation.

Today it reports:

```text
agent_drift_risk: critical
implicit_dependency_contract: high
recommendation: port to structured SkillSpec
```

For a SkillSpec-backed package, this is incomplete because
`skills/skillspec/skill.spec.yml` already has structured dependencies,
commands, tests, and proof surfaces.

New fields:

```json
{
  "raw_activation_risk": {
    "score": 76,
    "level": "critical"
  },
  "contract_mitigation": {
    "present": true,
    "spec_path": "skills/skillspec/skill.spec.yml",
    "routes": 20,
    "rules": 32,
    "commands": 76,
    "dependencies": 5,
    "tests": 41,
    "level": "strong"
  },
  "residual_agent_drift_risk": {
    "score": 48,
    "level": "medium",
    "reason": "contract exists, but trampoline remains large and dense"
  }
}
```

Dependency warning behavior:

```text
If deps.toml is absent but skill.spec.yml has dependencies:
  do not emit high implicit_dependency_contract
  emit info or low note: dependency surface declared in skill.spec.yml

If neither deps.toml nor skill.spec.yml dependencies exist:
  emit implicit_dependency_contract

If imported prose draft has deps.toml scaffold:
  evaluate ledger completeness
```

Recommendation behavior:

```text
If skill.spec.yml exists:
  recommend thinning trampoline and using guide output

If no skill.spec.yml exists:
  recommend porting to SkillSpec
```

## Backward Compatibility

Existing commands must keep working:

```bash
skillspec run-loop ./skill.spec.yml --input '<task>' --view index
skillspec plan ./skill.spec.yml --input '<task>'
skillspec act ./skill.spec.yml --input '<task>' --run <run_dir> --phase <phase>
skillspec progress show ./skill.spec.yml --run <run_dir>
skillspec trace align ./skill.spec.yml --decision-trace <run_dir> --summary
```

`--guide agent` is additive.

`--resume` is additive.

`sensemake --view full` should remain available for exhaustive inspection.

## Failure Behavior

### CLI Missing

Trampoline fallback:

```text
Read skill.spec.yml directly.
Follow route/rule/phase/proof manually.
Report that CLI guidance was unavailable.
```

### Resume Without Trace

Error:

```text
Cannot resume: run_dir has no decision trace. Start a new guided run.
```

### Resume With Changed Input

Error:

```text
Cannot resume: this run was created for a different task input.
Start a new run.
```

### Resume With Changed Spec, Same Decision

Warning:

```text
Spec changed since run start, but active route decision appears unchanged.
Review warning and continue only if acceptable.
```

### Resume With Changed Decision

Block:

```text
Spec changed and selected route/gates changed. Re-plan before continuing.
```

### Missing Execution Ledger

Allowed:

```text
Current phase is first phase.
Execution proof says ledger missing.
Guide should tell agent to record progress after action.
```

### No Execution Plan

Allowed:

```text
Selected route is current scope.
Guide should expose route-level forbids, elicitations, dependencies, and end
proof.
```

## Implementation Phases

### Phase 1: Guide Report Without Resume

Deliver:

```text
skillspec run-loop <spec> --input <task> --trace-dir <dir> --guide agent
```

Implement:

- guide types
- start anchor
- path
- current gate
- end anchor
- human rendering
- JSON rendering
- `guide-state.json`
- `guide-summary.md`

Tests:

- selected route appears in start anchor
- current phase appears
- end anchor includes final progress and trace align commands
- state files are written
- normal `run-loop` output unchanged without `--guide`

### Phase 2: Resume

Deliver:

```text
skillspec run-loop <spec> --resume <run_dir> --guide agent
```

Implement:

- recover input from trace
- compare spec/input fingerprints
- build decision fingerprint
- read execution ledger
- infer current phase
- refresh guide state

Tests:

- resume starts at first phase when no progress exists
- resume advances after `phase_completed`
- resume reports open requirements
- resume refuses changed input
- resume warns or blocks on changed spec/decision

### Phase 3: Compact `sensemake`

Deliver:

```text
skillspec sensemake <spec> --view index
```

as compact orientation, not exhaustive handle dump.

Implement:

- compact index view
- optional handles/full behavior
- docs/help update

Tests:

- index output does not list all 76 commands for self spec
- full output still exposes handles
- JSON remains stable or versioned

### Phase 4: Thin Trampoline

Deliver:

```text
skills/skillspec/SKILL.md
```

as true loader.

Implement:

- shrink content
- preserve frontmatter discoverability
- point to start/resume commands
- point to fallback

Tests:

- `skillspec doctor skills/skillspec/` no longer reports high activation body
  for the trampoline
- frontmatter discovery risk remains low
- `skillspec validate skills/skillspec/skill.spec.yml`
- `skillspec test skills/skillspec/skill.spec.yml`
- `skillspec run-loop skills/skillspec/skill.spec.yml --input 'what is the shape of this skill' --trace-dir /tmp/... --guide agent`

### Phase 5: Doctor Contract Mitigation

Deliver:

- raw activation risk
- contract mitigation
- residual risk
- corrected dependency warning behavior

Tests:

- SkillSpec-backed self skill does not recommend "port to SkillSpec"
- missing `deps.toml` is not high severity when spec dependencies exist
- prose-only skill still gets dependency warning
- large SkillSpec-backed loader still reports residual risk

### Phase 6: Docs And Command Surfaces

Update:

```text
spec/commandspec.md
docs/README_DETAILED.md
docs/design/16-command-log.md
skills/skillspec/skill.spec.yml command entries
skills/skillspec/skill.spec.yml tests
CLI --help
```

Include:

- `run-loop --guide agent`
- `run-loop --resume <run_dir>`
- guide-state files
- compact `sensemake`
- doctor mitigation fields

## Required Tests

CLI tests:

```text
run_loop_guide_agent_prints_start_current_end
run_loop_guide_agent_writes_state_files
run_loop_resume_uses_trace_input
run_loop_resume_advances_current_phase_from_execution_ledger
run_loop_resume_blocks_changed_decision
sensemake_index_is_compact
sensemake_full_exposes_handles
doctor_skillspec_backed_skill_reports_contract_mitigation
doctor_skillspec_backed_dependencies_not_implicit_high
self_trampoline_activation_size_below_threshold
```

Integration smoke:

```bash
cargo check -p skillspec
cargo clippy -p skillspec --all-targets -- -D warnings
cargo test -p skillspec --test cli
cargo run -p skillspec -- validate skills/skillspec/skill.spec.yml
cargo run -p skillspec -- test skills/skillspec/skill.spec.yml
cargo run -p skillspec -- run-loop skills/skillspec/skill.spec.yml --input 'what is the shape of this skill' --trace-dir /tmp/skillspec-guide-test --guide agent
cargo run -p skillspec -- run-loop skills/skillspec/skill.spec.yml --resume /tmp/skillspec-guide-test/<run-id> --guide agent
cargo run -p skillspec -- doctor skills/skillspec/
git diff --check
```

## Acceptance Criteria

The work is done only when all are true:

1. `skills/skillspec/SKILL.md` is a thin loader, not a policy manual.
2. The CLI guide prints a clear start anchor, current gate, and end anchor.
3. The CLI guide persists `guide-state.json` and `guide-summary.md`.
4. Resume works from `run_dir` without model memory.
5. Resume validates `input_sha256`, `spec_fingerprint`, and
   `decision_fingerprint`.
6. Agents do not need to read the full spec to know the next action.
7. `sensemake --view index` is compact.
8. Full handle inspection remains available.
9. `doctor` recognizes SkillSpec-backed mitigation.
10. The self skill no longer dogfoods as a high load-bearing trampoline.
11. Existing simple SkillSpec and prose skill flows remain compatible.
12. Docs, command log, CLI help, and self spec are updated.

## Key Product Principle

The trampoline should never become the skill again.

The stable pattern is:

```text
start or resume through CLI
follow one selected route
advance one gate at a time
persist state outside the model
prove the end compactly
load detail only when the guide asks for it
```
