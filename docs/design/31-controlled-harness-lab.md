# Controlled Harness Lab

Status: implementation started. The core sandbox, Doctor matrix, import matrix,
reviewed imported-runtime path, router lifecycle path, durable-executor
lifecycle path, and durable rote-exec proof path now have committed
machine-readable baselines. The current tightening pass documents exact
coverage, live-check findings, and the remaining manual gates; later phases
should extend the same crate instead of adding broad CLI snapshot tests.

This document proposes a no-Docker harness lab for turning more of
`docs/design/30-testing-matrix.md` into deterministic local and CI automation.
The goal is to test SkillSpec's harness-facing contracts without pretending that
the repo can fully control Codex, Claude, or any other external agent runtime.

## Problem

SkillSpec can already test most CLI behavior directly. The hard part is the
harness boundary:

- personal roots such as `.agents/skills` and `.codex/skills`;
- project-local roots such as `.claude/skills`;
- router hooks that must run before skill selection;
- visibility state that should make non-router skills explicit/manual-only;
- trampolines that should hand off to SkillSpec instead of loading the full
  manual;
- durable-executor state that may observe a run without replacing router
  selection.

Real harnesses read their roots and settings from the current process
environment at startup. An already running harness cannot be reliably repointed
to a fresh test home. Automated tests therefore need a controlled environment
that launches commands with an isolated `HOME`, isolated `SKILLSPEC_HOME`, and a
temporary project directory.

## Current Discovery Contract

The current implementation resolves roots as follows:

- `.agents/skills` comes from `$HOME/.agents/skills`;
- `.codex/skills` comes from `$HOME/.codex/skills`;
- local Claude skills come from walking upward from the current working
  directory until a `.claude` directory is found, then using `.claude/skills`;
- router and durable state come from `$SKILLSPEC_HOME` when set, otherwise
  `$HOME/.skillspec`.

That means a sandbox test can safely control SkillSpec-owned file effects with:

```sh
HOME=<tmp>/home
SKILLSPEC_HOME=<tmp>/home/.skillspec
XDG_CONFIG_HOME=<tmp>/home/.config
XDG_CACHE_HOME=<tmp>/home/.cache
XDG_DATA_HOME=<tmp>/home/.local/share
PWD=<tmp>/home/project
```

The test runner should create this tree before every case:

```text
<tmp>/home/
  .agents/skills/
  .codex/skills/
  .skillspec/
  project/.claude/skills/
```

Commands that need `claude-local` discovery must run with
`current_dir=<tmp>/home/project`.

## Layers

### Layer 1: File-System Harness Lab

This layer is a Rust test helper, not a separate process. It creates a temporary
home, temporary harness roots, and a temporary project, then runs the real
`skillspec` binary with controlled environment variables.

It can verify:

- `skillspec install targets`;
- `skillspec install skill`;
- `skillspec workspace install`;
- router install, update, enable, disable, guard, index refresh, and uninstall;
- durable-executor install, update, enable, disable, and delete;
- visibility sidecars and native metadata;
- router config, hooks, manifest, and index files;
- retire-existing backup behavior;
- stale index and out-of-band skill repair.

This proves SkillSpec's file mutations and command outputs. It does not prove
that a real Codex or Claude process reloaded those files.

Each phase should write a machine-readable report card. The JSON report is the
stable regression artifact; any Markdown rendering is secondary. Report cards
must use stable case ids and claim ids, normalize temp paths such as
`<HOME>/.codex/skills`, and avoid timestamps or machine-specific absolute paths.

Minimum JSON shape:

```json
{
  "schema": "skillspec/harness-lab-report/v0",
  "phase": "09-harness-lab-core",
  "summary": {
    "status": "pass",
    "cases_total": 3,
    "cases_passed": 3,
    "cases_failed": 0,
    "claims_total": 17,
    "claims_passed": 17,
    "claims_failed": 0
  },
  "cases": [
    {
      "id": "detects_sandbox_targets_from_lab_environment",
      "status": "pass",
      "claims": [
        {
          "id": "install.targets.codex.detected",
          "status": "pass",
          "expected": true,
          "observed": true
        }
      ]
    }
  ]
}
```

Regression comparison should fail when a previously passing case or claim is
missing, when a previously passing case or claim becomes failed, or when a stable
observed value changes. This lets CI track behavior changes without relying on
large stdout snapshots.

Committed golden baselines live under the harness lab crate, for example:

```text
crates/skillspec-harness-lab/baselines/09-harness-lab-core.json
crates/skillspec-harness-lab/baselines/10-doctor-matrix.json
crates/skillspec-harness-lab/baselines/11-import-matrix.json
crates/skillspec-harness-lab/baselines/12-imported-skill-runtime.json
crates/skillspec-harness-lab/baselines/13-router-harness-lab.json
crates/skillspec-harness-lab/baselines/14-durable-harness-lab.json
crates/skillspec-harness-lab/baselines/15-durable-rote-exec-proof.json
crates/skillspec-harness-lab/baselines/15-durable-rote-exec-live.json
```

Normal test runs compare the candidate report against the committed baseline.
When behavior intentionally changes, refresh the baseline explicitly:

```sh
UPDATE_HARNESS_LAB_BASELINES=1 cargo test --locked -p skillspec-harness-lab
```

or:

```sh
just harness-lab-update-baselines
```

The resulting JSON diff is part of the review. Do not update baselines to hide
unexpected behavior changes.

Current committed phases:

- `09-harness-lab-core`: sandbox root discovery, project-local target
  detection, and all-detected skill install.
- `10-doctor-matrix`: Doctor target-shape coverage for non-skill files,
  missing paths, empty skills, simple folders, direct `SKILL.md` paths,
  malformed frontmatter, cross-referenced subskills, plugin workspaces, and
  ordinary code repos.
- `11-import-matrix`: import-skill target-shape coverage for missing paths,
  single folders, direct `SKILL.md`, direct Markdown files, empty and malformed
  drafts, stale source maps, multi-skill rejection, workspace fanout, and
  plugin workspace import.
- `12-imported-skill-runtime`: reviewed imported package coverage for
  compile/install into all sandbox roots, `--retire-existing`, decision-only
  `unproven` alignment, batched progress evidence, final-response proof, token
  savings, and full execution alignment.
- `13-router-harness-lab`: router install coverage for managed router skill
  files, hooks, visibility, index and config, guard repair for out-of-band
  skills, route use-skill/bypass behavior, disable/enable visibility toggling,
  and uninstall cleanup.
- `14-durable-harness-lab`: durable-executor install/update/delete coverage,
  managed-marker mutation guards, enable/disable visibility toggling, missing
  `rote` preflight behavior, and router refresh integration that keeps
  durable-executor implicit.
- `15-durable-rote-exec-proof`: durable-executor contract coverage for local
  command intent, `rote_exec` tool-boundary selection, direct CLI/shell forbids,
  `rote-shell` guidance presence, and SkillSpec alignment over `rote_exec`
  process evidence.
- `15-durable-rote-exec-live`: opt-in local proof that copies an authenticated
  local `rote` binary and the local `~/.rote` config, excluding existing
  workspaces, into the lab, sets `ROTE_HOME` to the copied tree, runs `rote exec
  -- printf skillspec-durable-proof` inside a named sandbox rote workspace, and
  aligns the resulting evidence. This is excluded from default CI because it
  depends on a real logged-in local rote environment.

Current phase entrypoints:

| Phase | Entry point | Regression baseline |
| --- | --- | --- |
| `09-harness-lab-core` | `crates/skillspec-harness-lab/tests/core.rs` | `baselines/09-harness-lab-core.json` |
| `10-doctor-matrix` | `crates/skillspec-harness-lab/tests/doctor.rs` | `baselines/10-doctor-matrix.json` |
| `11-import-matrix` | `crates/skillspec-harness-lab/tests/import.rs` | `baselines/11-import-matrix.json` |
| `12-imported-skill-runtime` | `crates/skillspec-harness-lab/tests/imported_runtime.rs` | `baselines/12-imported-skill-runtime.json` |
| `13-router-harness-lab` | `crates/skillspec-harness-lab/tests/router.rs` | `baselines/13-router-harness-lab.json` |
| `14-durable-harness-lab` | `crates/skillspec-harness-lab/tests/durable.rs` | `baselines/14-durable-harness-lab.json` |
| `15-durable-rote-exec-proof` | `crates/skillspec-harness-lab/tests/durable_rote_exec.rs` | `baselines/15-durable-rote-exec-proof.json` |
| `15-durable-rote-exec-live` | `crates/skillspec-harness-lab/tests/durable_rote_exec.rs --ignored` | `baselines/15-durable-rote-exec-live.json` |

Live checkpoint from 2026-06-29:

- `just dev-install-all` installed the debug CLI and `skills/skillspec` into
  every detected local target.
- `skillspec install targets` found `agents`, `codex`, and `claude-local`.
- `skillspec router guard --json` and `skillspec router guard --hook` both
  reported router readiness with valid first-hop hook context.
- `skillspec route --query "what is the time today"` returned `bypass`, so the
  CLI can avoid routing ordinary requests into a domain skill.
- `skillspec durable-executor enable --json` succeeded against real roots after
  filesystem permission was granted.
- `just harness-lab-live-durable-rote-exec` passed with a copied local `rote`
  binary, copied local `~/.rote` config, sandbox `ROTE_HOME`, and a real
  `rote exec -- printf skillspec-durable-proof` command.

The same checkpoint exposed two gaps that should drive the simulator phase:

- duplicate same-skill installs across `agents`, `codex`, and `claude-local`
  can produce equal route candidates and `ambiguous_match`;
- stale visibility manifest entries for removed skills can appear as guard
  warnings and need a repairable cleanup assertion.

### Layer 2: Pseudo-Harness Simulator

This layer is a small host-native test binary or helper named something like
`skillspec-harness-sim`. It should emulate only the harness contract that
SkillSpec depends on:

1. discover implicit skills from configured roots;
2. run the configured pre-call hook;
3. build the visible skill catalog after hook repair;
4. call `skillspec route` for router decisions;
5. load a domain skill only when the route decision is `use_skill`;
6. bypass all domain skills when the route decision is `bypass` or `ambiguous`;
7. invoke a trampoline for SkillSpec-backed activation tests;
8. emit a JSONL event trace for assertions.

The simulator should not call a model. Skill selection, user prompts, tool calls,
and final answers should be scripted fixtures. The purpose is deterministic
boundary testing, not behavioral evaluation of an LLM.

Example event stream:

```jsonl
{"event":"lab_started","home":"<tmp>/home"}
{"event":"roots_detected","roots":["<tmp>/home/.codex/skills"]}
{"event":"hook_invoked","harness":"codex","command":"skillspec router guard ...","exit_code":0}
{"event":"catalog_built","implicit":["skill-router"],"manual_only":["pdf","browser"]}
{"event":"route_decision","decision":"bypass","selected":null}
{"event":"domain_skill_loaded","loaded":false}
```

This layer can turn several current manual concerns into automated checks:

- router is first hop in the simulated harness contract;
- router bypass does not cause extra skill loading;
- clear domain intent loads exactly one selected skill;
- duplicate same-skill candidates across roots collapse to one logical choice
  or resolve by deterministic root preference;
- out-of-band implicit skills are repaired before catalog build;
- imported trampoline activation asks SkillSpec for guidance;
- durable-executor can remain implicit while router remains selection authority;
- durable-executor activation is not blocked by duplicate root installs.

### Layer 3: Real Harness Smoke Runner

This layer launches a real harness process with the same isolated environment:

```sh
HOME=<tmp>/home \
SKILLSPEC_HOME=<tmp>/home/.skillspec \
XDG_CONFIG_HOME=<tmp>/home/.config \
XDG_CACHE_HOME=<tmp>/home/.cache \
XDG_DATA_HOME=<tmp>/home/.local/share \
codex
```

For Claude-local tests, launch from the temporary project:

```sh
cd <tmp>/home/project
HOME=<tmp>/home \
SKILLSPEC_HOME=<tmp>/home/.skillspec \
XDG_CONFIG_HOME=<tmp>/home/.config \
XDG_CACHE_HOME=<tmp>/home/.cache \
XDG_DATA_HOME=<tmp>/home/.local/share \
claude
```

This should stay manual by default because a fresh home will not have normal
auth, plugin marketplace state, model config, or user approvals. A developer may
prepare an explicit test profile, but the lab must never silently copy secrets
or real harness credentials from the user's home.

Real harness smoke tests can verify:

- the harness actually reloads generated hook files;
- the model observes the router instruction as first hop;
- the model does not perform unnecessary router/index/status hops on bypass;
- final answer wording is understandable and honest;
- durable-executor observation UX is acceptable.

### Layer 4: Event-Tracked Harness Lab

This is the future version of Layer 3. It would wrap a real harness in a PTY,
send deterministic prompts, and collect:

- hook invocation logs;
- SkillSpec command events;
- generated trace directories;
- progress ledger files;
- router decisions;
- final assistant output.

This layer still should not be mandatory in CI unless the harness vendors expose
a stable noninteractive test mode.

## Matrix Coverage Impact

| Matrix area | Layer 1 | Layer 2 | Layer 3/4 |
| --- | --- | --- | --- |
| Install and setup | File effects, target discovery, replacement behavior. | Harness catalog after install. | Real `/skillspec` activation. |
| Doctor | CLI target-shape and output fixtures. | Harness invocation of doctor trampoline. | Agent explanation quality. |
| Import | Draft generation, compile, install, retire-existing. | Trampoline handoff to SkillSpec guidance. | Live activation quality. |
| Runtime behavior | Plan, act, progress, align, token stats. | Scripted activation path and missing-proof cases. | Live progress cadence and final summary quality. |
| Router | Config, hooks, visibility, index, guard, repair. | First-hop, bypass, selected-skill loading. | Real pre-skill hook ordering in Codex/Claude. |
| Durable executor | Install, enable, disable, delete, rote preflight. | Durable implicit observer plus router authority. | User approval, observation, record, and memory UX. |

Layer 1 and Layer 2 should be the default automation target. Layer 3 and Layer 4
are smoke and acceptance layers.

## Safety Rules

The lab must be fail-closed:

- refuse to run if `HOME` is not under the test temp directory unless an
  explicit unsafe/manual flag is set;
- refuse to write to real `.agents`, `.codex`, `.claude`, or `.skillspec`
  directories during automated tests;
- never copy credentials, tokens, auth files, or real harness config into a temp
  home automatically;
- disable network by convention for pure simulator tests;
- write every command, exit status, and touched path to the lab event trace;
- keep test fixtures static and checked into the repo;
- make cleanup best-effort but preserve the temp directory on failure when a
  debug flag is set.

## Proposed Implementation Shape

Build the lab in the same order a user adopts SkillSpec: assess the current
skill, import it, install and activate the imported package, then add router and
durable lifecycle coverage. This keeps router and durable tests on top of the
basic doctor/import/install guarantees instead of making lifecycle tests carry
the whole adoption path.

Use this stack:

| Branch | Scope | Primary rows moved toward `Covered` |
| --- | --- | --- |
| `test/09-harness-lab-core` | Shared sandbox helper, temp home, temp roots, env injection, real-home write guard. | Install/setup sandbox rows. |
| `test/09b-harness-lab-report-cards` | Machine-readable report cards and report comparison for regression detection. | Stable evidence for every later phase. |
| `test/10-doctor-matrix` | Doctor target-shape fixtures and output assertions in the lab. | Doctor positive and negative gaps. |
| `test/11-import-matrix` | Import target-shape fixtures, direct `SKILL.md`, references, workspace/plugin imports, QA commands. | Import positive, negative, and QA gaps. |
| `test/12-imported-skill-runtime` | Compile/install reviewed imported skill, retire-existing behavior, trampoline/spec presence, decision/plan/act/progress/final-response/align checks. | Imported install and runtime-proof rows. |
| `test/13-router-harness-lab` | Router install, hooks, visibility, index, guard, route bypass/use-skill, repair, disable/enable/uninstall. | Router harness-sim rows. |
| `test/14-durable-harness-lab` | Durable install/update/delete, enable/disable, missing `rote`, router plus durable ordering. | Durable harness-sim rows. |
| `test/15-durable-rote-exec-proof` | Durable route/act contract for `rote_exec`, `rote-shell` guidance fixture, alignment over `rote_exec` process evidence, and opt-in copied local rote binary/config execution proof with sandbox `ROTE_HOME`. | Durable substrate proof rows. |
| `test/16-matrix-coverage-tightening` | Update the matrix with exact test names and remaining manual gates. | Documentation accuracy. |
| `test/17-pseudo-harness-simulator` | Add deterministic pseudo-harness scenarios for pre-call hook ordering, bypass/no-load behavior, domain selected-skill loading, duplicate-root collapse, imported trampoline handoff, and durable implicit observer behavior. | Router and durable boundary rows that do not require a live harness UI. |

The implementation lives in a separate test crate so the crate refactor and CLI
boundary stay clean:

```text
crates/skillspec-harness-lab/src/
crates/skillspec-harness-lab/tests/
crates/skillspec-harness-lab/baselines/
```

The support crate should continue to expose small helpers instead of growing a
single monolithic test file:

- `HarnessLab::new()`;
- `lab.home()`;
- `lab.project()`;
- `lab.agents_root()`;
- `lab.codex_root()`;
- `lab.claude_root()`;
- `lab.command("skillspec")`;
- `lab.command_in_project("skillspec")`;
- `lab.write_skill(root, name, skill_md, spec_yml)`;
- `lab.assert_no_real_home_writes()`;
- `lab.read_router_config()`;
- `lab.read_events()`.

If the pseudo-harness becomes large enough, move it behind an internal command:

```sh
skillspec harness-sim run <scenario.yml> --jsonl <events.jsonl>
```

That command should be hidden or explicitly documented as an internal test tool
until the scenario format stabilizes.

## Scenario Format Sketch

```yaml
schema: skillspec/harness-lab/v0
name: imported-skill-activation-handoff
harness: codex
roots:
  codex:
    - imported-skill
pre:
  - skillspec import-skill ./source-skill --out ./build/skill.spec.yml
  - skillspec compile ./build/skill.spec.yml --out ./compiled
  - skillspec install skill ./compiled --target codex --retire-existing
prompt: use imported-skill for the fixture task
expect:
  trampoline_loaded: true
  spec_discovered: true
  guidance_command_available: true
  forbidden_events:
    - full_manual_loaded
```

The scenario language should remain assertion-focused. It should not become a
second SkillSpec runtime.

## Non-Goals

The controlled harness lab does not:

- replace real Codex or Claude acceptance testing;
- guarantee vendor-specific hook ordering beyond what a real smoke test proves;
- simulate model judgment;
- benchmark model quality;
- execute arbitrary tools outside the temp home;
- use Docker as a default isolation mechanism.

## Success Criteria

The proposal is successful when:

1. every `Harness-sim automatable` row in the testing matrix has a committed
   Layer 1 or Layer 2 test;
2. live harness rows remain clearly labeled `Manual` or `Manual with trace
   review`;
3. doctor, import, imported activation, router-first, bypass, selected-skill,
   visibility restore, durable lifecycle, and trampoline handoff have
   deterministic traces;
4. failures show the command, environment, touched files, and event trace needed
   to reproduce locally;
5. the external CLI and skill contracts remain unchanged.
