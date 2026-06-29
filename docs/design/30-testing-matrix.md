# Testing Matrix

This matrix defines the release-candidate test surface for SkillSpec after the
crate split, router lifecycle work, and durable-executor lifecycle work. It is a
design and QA planning document, not a replacement for source-level tests.

The goal is to prove that the external contract remains stable:

- installing the CLI still gives users one `skillspec` command;
- the SkillSpec skill can be installed into supported harness roots;
- doctor, import, execution guidance, router mode, and durable execution keep
  their user-visible behavior;
- optional features remain optional and separable.

## Automation Classes

Use these classes in the matrix. The class is automation feasibility, not a
claim that the exact row is already covered by a committed test. Current
coverage is tracked separately in the `Coverage` column.

| Class | Meaning | Why |
| --- | --- | --- |
| `Automatable` | Can run in CI or a local script using temporary files, temporary `HOME`, and CLI assertions. | The behavior is owned by SkillSpec code and can be checked without an interactive harness session. |
| `Harness-sim automatable` | Can be automated with disposable harness roots and fixture files, but does not prove a real Codex or Claude session reloaded the hook. | SkillSpec owns the generated files and metadata, but the real harness runtime is outside this repo. |
| `Manual` | Needs a live harness UI/session or a human judgment of agent behavior. | The observable behavior depends on external harness skill loading, prompt hooks, model decisions, or user approval UX. |
| `Manual with trace review` | Needs a live harness run plus inspection of SkillSpec traces, progress ledgers, and final output. | The result is not just command success; it requires confirming the agent followed the trampoline and reported proof correctly. |

## Coverage Labels

Use these labels to state what has been verified in the current repo.

| Coverage | Meaning |
| --- | --- |
| `Covered` | Current Rust integration/unit tests, conformance tests, examples, package checks, or CI preflight exercise this row directly. |
| `Partial` | Current tests cover the command family or a neighboring case, but not the exact row end-to-end. |
| `Gap` | The row is automatable, but no exact committed coverage has been verified yet. |
| `Manual` | The row depends on live Codex/Claude/harness behavior or human review and is not fully provable inside this repo. |

The verified coverage map at the time this matrix was written is:

- doctor: `crates/skillspec-cli/tests/cli/doctor.rs`;
- import, workspace, and install: `crates/skillspec-cli/tests/cli/authoring.rs`
  and `crates/skillspec-cli/tests/cli/workspace_install.rs`;
- runtime execution, progress, alignment, and token reports:
  `crates/skillspec-cli/tests/cli/runtime_contracts.rs`;
- router and durable-executor lifecycle:
  `crates/skillspec-cli/tests/cli/lifecycle.rs`;
- controlled harness-lab regression cards:
  `crates/skillspec-harness-lab/tests/core.rs`,
  `crates/skillspec-harness-lab/tests/doctor.rs`,
  `crates/skillspec-harness-lab/tests/import.rs`,
  `crates/skillspec-harness-lab/tests/imported_runtime.rs`,
  `crates/skillspec-harness-lab/tests/router.rs`,
  `crates/skillspec-harness-lab/tests/durable.rs`,
  `crates/skillspec-harness-lab/tests/durable_rote_exec.rs`, and committed
  baselines under `crates/skillspec-harness-lab/baselines/`;
- command help and sensemaking surfaces:
  `crates/skillspec-cli/tests/cli/cli_core.rs` and
  `crates/skillspec-cli/tests/cli/capability_sensemake.rs`;
- package hygiene: `Justfile`, `.github/workflows/ci.yml`, conformance
  fixtures, and package dry-run checks.

## Test Environment Model

Reliable automation needs isolated homes and harness roots. A local or CI test
runner should create a fresh sandbox for every case:

```text
<tmp>/home/
  .agents/skills/
  .codex/skills/
  project/.claude/skills/
  .skillspec/
```

The runner should set at least:

```sh
HOME=<tmp>/home
PWD=<tmp>/repo-or-project
```

For local project Claude tests, run from `<tmp>/home/project` so
`claude-local` installs resolve into that project. Fixture roots should be
created from static test data, never from a developer's real skill library.

For each test, capture:

- command;
- exit status;
- stdout/stderr;
- files written, modified, or removed;
- visibility sidecars or native metadata;
- router index status where relevant;
- trace directory and alignment files where relevant.

## Install And Setup Matrix

| Area | Case | Expected Result | Class | Coverage |
| --- | --- | --- | --- | --- |
| CLI install | Install released CLI with public script. | `skillspec --version` works and resolves to requested version. | Manual | Manual |
| CLI install | Install local checkout in debug mode with `just install-debug`. | Local `skillspec` resolves to checkout version. | Automatable | Gap |
| CLI install | Install local checkout in release mode with `just install-release`. | Release-profile binary installs and reports version. | Automatable | Gap |
| Package hygiene | Run `just preflight`. | fmt, locked check/clippy/tests, package lists, examples, deps, and conformance pass. | Automatable | Covered |
| Harness target discovery | Run `skillspec install targets` in a sandbox with agents, Codex, and Claude project roots. | Detected targets match created roots. | Harness-sim automatable | Partial |
| Skill install | Install `skills/skillspec` into `agents`. | Skill files are copied and support files are present. | Harness-sim automatable | Partial |
| Skill install | Install `skills/skillspec` into `codex`. | Skill files are copied and support files are present. | Harness-sim automatable | Partial |
| Skill install | Install `skills/skillspec` into `claude-local`. | Project-local skill files are copied. | Harness-sim automatable | Covered |
| Skill install | Install into all detected targets. | All sandbox roots receive the skill exactly once. | Harness-sim automatable | Partial |
| Skill install negative | Install into missing or unsupported target. | Command fails with actionable target error. | Automatable | Gap |
| Skill install negative | Install a folder without `SKILL.md`. | Command fails before writing target files. | Automatable | Gap |
| Skill install negative | Install a skill with unsafe nested discoverable `SKILL.md` support file. | Command rejects nested discoverable skill package. | Automatable | Covered |
| Skill install replacement | Install over an existing skill without `--retire-existing` or force behavior. | Collision is reported and existing skill remains. | Harness-sim automatable | Covered |
| Skill install replacement | Install with `--retire-existing`. | Existing folder is backed up and new skill is installed. | Harness-sim automatable | Covered |
| Harness setup | Open Codex/Claude after install and invoke `/skillspec`. | Harness sees the installed skill and follows the trampoline. | Manual | Manual |
| Harness setup | Verify setup across every supported harness on a developer machine. | Codex, Claude, and agents roots behave as documented. | Manual | Manual |

## Doctor Matrix

| Area | Case | Expected Result | Class | Coverage |
| --- | --- | --- | --- | --- |
| Doctor shape-only | Pass a non-`SKILL.md` file path. | Current contract returns a shape-only `non_skill_repository` report with `no_skill_entrypoint`. | Automatable | Covered |
| Doctor negative | Pass a folder with empty `SKILL.md`. | Report identifies unusable/empty skill content without panic. | Automatable | Covered |
| Doctor negative | Pass a folder with malformed frontmatter. | Report flags frontmatter discovery risk or parse problem without panic. | Automatable | Covered |
| Doctor negative | Pass a folder with malformed Markdown structure and no useful instructions. | Report identifies high drift/proof risk without crashing. | Automatable | Partial |
| Doctor negative | Pass non-existent path. | Command fails with clear path error. | Automatable | Covered |
| Doctor positive | Pass folder with one proper `SKILL.md`. | Simple skill report includes risk, activation surface, findings, and next action. | Automatable | Covered |
| Doctor positive | Pass direct `SKILL.md` path. | Report is equivalent to the parent single-skill target. | Automatable | Covered |
| Doctor positive | Pass folder with multiple `SKILL.md` files and cross references. | Workspace/package report includes one package report per skill and aggregate risk. | Automatable | Covered |
| Doctor positive | Pass plugin-shaped folder. | Plugin workspace shape and package namespace are reported. | Automatable | Covered |
| Doctor positive | Pass SkillSpec-backed skill. | Report recognizes contract mitigation and does not grade it as plain prose only. | Automatable | Covered |
| Doctor shape-only | Pass ordinary code repository without `SKILL.md`. | Shape-only report returns `non_skill_repository` and `no_skill_entrypoint`. | Automatable | Covered |
| Doctor remote | Pass public GitHub folder URL. | Remote sparse checkout is staged, analyzed, and cleaned up. | Automatable | Gap |
| Doctor remote negative | Pass private or invalid GitHub URL. | Error is clear and does not leak credentials. | Automatable | Gap |
| Doctor output | Run text, JSON, Markdown, and HTML output modes. | Each output parses/renders and contains the same core report facts. | Automatable | Covered |
| Doctor in harness | Ask `/skillspec run doctor on ./my-skill` and ask agent to explain. | Agent invokes doctor and explains the report accurately. | Manual with trace review | Manual |

## Import Matrix

| Area | Case | Expected Result | Class | Coverage |
| --- | --- | --- | --- | --- |
| Import negative | Pass non-existent path. | Command fails and does not write a draft. | Automatable | Covered |
| Import positive | Pass a direct Markdown file path. | Current contract imports the file as a review-required `source_kind: file` draft. | Automatable | Covered |
| Import draft | Pass folder with empty `SKILL.md`. | Current contract writes a review-required draft with dependency ledger evidence instead of pretending it is final. | Automatable | Covered |
| Import draft | Pass folder with malformed `SKILL.md`. | Command preserves source evidence and writes a review-required draft without parsing frontmatter as a hard gate. | Automatable | Covered |
| Import negative | Pass parent folder with multiple `SKILL.md` files to single-skill import. | Command rejects and points to workspace map/import flow. | Automatable | Covered |
| Import positive | Pass folder with proper single `SKILL.md`. | Draft `skill.spec.yml`, preserved non-discoverable source copy, deps ledger, and review notes are generated. | Automatable | Covered |
| Import positive | Pass direct `SKILL.md` path. | Draft output is generated from the file target without requiring the caller to pass the parent folder. | Automatable | Covered |
| Import positive | Pass skill with references/resources. | Generated draft preserves references as imports/resources or review notes. | Automatable | Covered |
| Import negative | Pass stale `source-map.json` to import. | Command rejects the stale map and tells caller to rerun source map before import. | Automatable | Covered |
| Workspace import positive | Map/import folder with multiple cross-referenced skills. | Workspace manifest, package graph, dependency edges, and package drafts are generated. | Automatable | Covered |
| Workspace import positive | Map/import plugin-shaped folder. | Plugin namespaces are preserved and install slugs are deterministic. | Automatable | Covered |
| Import QA | Run validate/imports check/deps check/test/compile after import. | Generated package reaches the expected QA stage or reports explicit blockers. | Automatable | Partial |
| Install imported skill | Compile and install a reviewed imported skill into sandbox targets. | Generated trampoline, `skill.spec.yml`, dependency ledger, and preserved source are installed. | Harness-sim automatable | Covered |
| Replacement install | Install imported skill over existing prose skill with `--retire-existing`. | Old files are backed up and retired; new files are active. | Harness-sim automatable | Covered |
| Replacement negative | Replacement install without retire/force. | Existing files remain and collision is reported. | Harness-sim automatable | Covered |
| Activation | Invoke imported skill in live harness. | Trampoline hands off to SkillSpec CLI guidance instead of re-reading the full manual. | Manual with trace review | Manual |
| Activation negative | Activate imported skill when `skillspec` binary is missing. | Trampoline reports missing CLI and does not claim full alignment proof. | Manual | Manual |

## Activation And Execution Behavior Matrix

| Area | Case | Expected Result | Class | Coverage |
| --- | --- | --- | --- | --- |
| Plan/act | Run `skillspec plan` and `skillspec act` on a known spec. | Selected route, matched rules, forbids, and phase boundary are rendered. | Automatable | Covered |
| Reviewed import runtime | Import a prose skill, review it into a route/phase contract, then decide, plan, act, record progress, and align. | Decision-only traces remain `unproven`; complete execution evidence aligns and writes `alignment.json`. | Automatable | Covered |
| Progress ledger | Record phase-completed and requirement evidence. | `<run-dir>/execution.jsonl` receives compact structured events. | Automatable | Covered |
| Batch progress | Use `skillspec progress batch` for grouped proof. | Multiple evidence events are recorded in one compact operation. | Automatable | Covered |
| Progress display | Run `skillspec progress show`. | Current/completed/blocked/remaining phase summary is accurate. | Automatable | Covered |
| Trace alignment | Run `skillspec trace align` with decision trace and execution ledger. | Alignment status is `aligned`, `partial`, or `unproven` with missing proof rows. | Automatable | Covered |
| Final response proof | Run `skillspec progress final-response`. | Final response evidence records result/evidence/alignment/token-savings sections. | Automatable | Covered |
| Token stats | Run progress stats with a valid workspace stats report. | Token consumption and savings evidence is recorded and appears in alignment. | Automatable | Covered |
| Token stats negative | Run stats with missing/empty token evidence. | Command refuses to invent token savings. | Automatable | Covered |
| Harness behavior | Live agent uses fewer progress updates by batching evidence. | User sees compact updates while ledger still captures proof. | Manual with trace review | Manual |
| Harness behavior | Live agent final answer includes alignment and token report. | Final answer reports alignment summary, missing proof if any, and token usage honestly. | Manual with trace review | Manual |
| Harness negative | Live agent skips a required proof step. | Alignment reports partial/unproven rather than success. | Manual with trace review | Manual |

## Router Matrix

| Area | Case | Expected Result | Class | Coverage |
| --- | --- | --- | --- | --- |
| Router install | Install router into sandbox roots. | Managed `skill-router` folders, config, visibility manifest, hooks, and index are written. | Harness-sim automatable | Covered |
| Router install | Verify pre-call hook files for chosen harnesses. | Hook command invokes `skillspec router guard` with installed config. | Harness-sim automatable | Covered |
| Router install | Existing skills become explicit/manual-only. | Native visibility metadata or sidecars reflect explicit invocation. | Harness-sim automatable | Covered |
| Router install | Index is populated. | `skillspec router index status` shows discovered and indexed skills. | Automatable | Covered |
| Router guard | Run guard after install. | `first_hop_ready=true` and hook output is valid. | Automatable | Covered |
| Router guard repair | Add an out-of-band skill after install, then run guard. | Guard repairs visibility/index drift and hook context reports `first_hop_ready=true`. | Harness-sim automatable | Covered |
| Router route positive | Query clear skill intent. | `skillspec route` returns `use_skill` with selected skill. | Automatable | Covered |
| Router route bypass | Query ordinary non-skill task. | `skillspec route` returns `bypass` or `ambiguous` and no selected skill. | Automatable | Covered |
| Router drift | Add out-of-band implicit skill after install. | Guard or index refresh detects and repairs explicit visibility. | Harness-sim automatable | Covered |
| Router stale index | Modify skill roots after index. | Status reports stale/missing/index mismatch. | Automatable | Covered |
| Router disable | Disable router mode. | Router first-hop is disabled and managed visibility is restored from manifest. | Harness-sim automatable | Covered |
| Router disable check | Verify existing skills are switched back to their previous visibility, not blindly all implicit. | Visibility manifest restore is correct. | Harness-sim automatable | Covered |
| Router enable | Re-enable router after disable. | Index refreshes and routed skills become explicit/manual-only again. | Harness-sim automatable | Covered |
| Router uninstall | Uninstall router. | Router skill folders and hooks are removed or disabled; visibility is restored. | Harness-sim automatable | Covered |
| Router negative | Install with invalid router name. | Command rejects invalid name. | Automatable | Covered |
| Router negative | Guard with missing config. | Command fails with repair/install guidance. | Automatable | Gap |
| Router live | Start Codex/Claude after router install and ask ordinary task. | Hook fires, router bypasses, and answer has no unnecessary router hops. | Manual | Manual |
| Router live | Ask task that should activate a domain skill. | Router chooses one skill and only that domain skill is loaded. | Manual with trace review | Manual |

## Durable Executor Matrix

| Area | Case | Expected Result | Class | Coverage |
| --- | --- | --- | --- | --- |
| Durable install | Install durable-executor from explicit source into sandbox roots. | Managed durable-executor folders and config are written. | Harness-sim automatable | Covered |
| Durable install negative | Install without `rote` on PATH. | Command fails before writing managed durable executor. | Automatable | Covered |
| Durable enable | Enable durable executor. | Durable executor visibility becomes implicit and config records enabled state. | Harness-sim automatable | Covered |
| Durable disable | Disable durable executor. | Durable executor visibility becomes manual-only without deleting files. | Harness-sim automatable | Covered |
| Durable update | Update managed durable executor. | Existing install is backed up and refreshed from source. | Harness-sim automatable | Covered |
| Durable delete | Delete managed durable executor. | Managed installs are removed; unmanaged folders are not removed. | Harness-sim automatable | Covered |
| Durable marker guard | Remove the managed marker before update/delete. | Update/delete refuse to mutate unmanaged durable-executor folders. | Harness-sim automatable | Covered |
| Durable with router | Install router and durable executor together. | Durable install refreshes the router index and durable-executor remains implicit while router stays installed. | Harness-sim automatable | Covered |
| Durable rote-exec contract | Ask durable-executor to run a local command and remember the result. | Plan/act select `one_shot_process`, allow `rote_exec`, forbid direct CLI/shell, and alignment accepts `rote_exec` process evidence. | Automatable | Covered |
| Durable live rote-exec proof | Copy `/Users/chetanconikee/.local/bin/rote` plus `~/.rote` config into the lab, excluding existing workspaces, set `ROTE_HOME` to that copied tree, and run `rote exec -- printf skillspec-durable-proof`. | The copied authenticated local `rote` creates a named sandbox workspace, captures the command output, and SkillSpec alignment reaches `pass`. | Manual with trace review | Covered by opt-in `just harness-lab-live-durable-rote-exec` |
| Durable happy path | Enable durable executor and run one SkillSpec-backed skill. | Workspace/evidence is preserved and final response can cite durable evidence. | Manual with trace review | Manual |
| Durable negative | User declines observation/record/memory. | Durable executor does not record or memorize events and task can continue direct if allowed. | Manual | Manual |
| Durable negative | Durable handoff loses required workspace or trace path. | Run reports blocker rather than pretending proof exists. | Manual with trace review | Manual |

## Automation Plan

Automating this matrix is feasible in layers.

### Layer 1: Pure CLI Fixtures

Can be added to Rust integration tests or shell-driven CI:

- doctor target-shape cases;
- import target-shape cases;
- plan/act/progress/align/token-stat cases;
- package file-list and command help checks;
- router route decisions against a fixture SQLite index;
- conformance and examples.

These tests do not need real harnesses.

### Layer 2: Disposable Harness Roots

Can be automated with temporary homes:

- `skillspec install targets`;
- `skillspec install skill`;
- router install/enable/disable/uninstall;
- durable install/enable/disable/delete;
- visibility sidecar/native metadata assertions;
- index freshness and drift repair;
- retire-existing backup behavior.

The runner should create fake `.agents`, `.codex`, and project `.claude` roots,
then assert file-system effects. This proves SkillSpec's harness mutations, but
not that a real harness process reloads and obeys the files.

### Layer 3: Real Harness Smoke Tests

These remain manual unless the project gains first-party harness test drivers:

- opening Codex or Claude and proving the hook actually fires before skill
  selection;
- proving the model loads only the selected skill;
- proving imported trampolines hand off to `skillspec run-loop`;
- assessing whether progress updates are suitably compact;
- validating final answer quality, alignment wording, and token-savings
  presentation.

These are manual because the observable behavior depends on external harness
runtime, model behavior, session reload, plugin installation state, and user
approval UX.

### Layer 4: Event-Tracked Harness Lab

The ideal future automation is a controlled harness lab:

- creates a fresh OS user home or container volume per test;
- installs local debug `skillspec`;
- creates isolated harness roots;
- starts a harness process with deterministic prompts;
- captures hook invocation logs, skill load events, CLI command events, trace
  dirs, and final assistant output;
- tears down the home after every test.

Until such a lab exists, manual harness testing should save:

- terminal transcript;
- generated trace directory;
- router guard output;
- relevant installed skill folders;
- final answer text;
- any observed mismatch between expected and actual harness behavior.

## Manual Release-Candidate Checklist

Before merging a large lifecycle or crate-boundary PR, manually test at least:

1. Install local debug CLI with `just dev-install-all`.
2. Start Codex and Claude fresh so they reload installed skills/hooks.
3. Run doctor from terminal on positive and negative fixtures.
4. Run doctor through `/skillspec` in a harness and ask for explanation.
5. Import one atomic skill, compile it, install it, and activate it.
6. Run one activation path through plan, act, progress, align, and final summary.
7. Install and enable router; verify hook, explicit visibility, index, bypass,
   and selected-skill behavior.
8. Disable router and verify visibility restoration.
9. Run `just harness-lab-live-durable-rote-exec` to prove the local copied
   `rote` binary and copied `~/.rote` config can execute a one-shot command
   through `rote exec --` with `ROTE_HOME` inside the lab, then align the
   resulting evidence.
10. Enable durable executor and run one happy-path durable skill execution in a
   live harness.
11. Confirm router and durable executor remain independently disableable.

Do not merge if any manual test only "looks okay" but leaves missing alignment
proof unexplained. Record the blocker and either fix the code or narrow the
claim in docs.
