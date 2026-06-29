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

Use these classes in the matrix.

| Class | Meaning | Why |
| --- | --- | --- |
| `Automated` | Can run in CI or a local script using temporary files, temporary `HOME`, and CLI assertions. | The behavior is owned by SkillSpec code and can be checked without an interactive harness session. |
| `Harness-sim automated` | Can be automated with disposable harness roots and fixture files, but does not prove a real Codex or Claude session reloaded the hook. | SkillSpec owns the generated files and metadata, but the real harness runtime is outside this repo. |
| `Manual` | Needs a live harness UI/session or a human judgment of agent behavior. | The observable behavior depends on external harness skill loading, prompt hooks, model decisions, or user approval UX. |
| `Manual with trace review` | Needs a live harness run plus inspection of SkillSpec traces, progress ledgers, and final output. | The result is not just command success; it requires confirming the agent followed the trampoline and reported proof correctly. |

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

| Area | Case | Expected Result | Class |
| --- | --- | --- | --- |
| CLI install | Install released CLI with public script. | `skillspec --version` works and resolves to requested version. | Manual |
| CLI install | Install local checkout in debug mode with `just install-debug`. | Local `skillspec` resolves to checkout version. | Automated |
| CLI install | Install local checkout in release mode with `just install-release`. | Release-profile binary installs and reports version. | Automated |
| Package hygiene | Run `just preflight`. | fmt, locked check/clippy/tests, package lists, examples, deps, and conformance pass. | Automated |
| Harness target discovery | Run `skillspec install targets` in a sandbox with agents, Codex, and Claude project roots. | Detected targets match created roots. | Harness-sim automated |
| Skill install | Install `skills/skillspec` into `agents`. | Skill files are copied and support files are present. | Harness-sim automated |
| Skill install | Install `skills/skillspec` into `codex`. | Skill files are copied and support files are present. | Harness-sim automated |
| Skill install | Install `skills/skillspec` into `claude-local`. | Project-local skill files are copied. | Harness-sim automated |
| Skill install | Install into all detected targets. | All sandbox roots receive the skill exactly once. | Harness-sim automated |
| Skill install negative | Install into missing or unsupported target. | Command fails with actionable target error. | Automated |
| Skill install negative | Install a folder without `SKILL.md`. | Command fails before writing target files. | Automated |
| Skill install negative | Install a skill with unsafe nested discoverable `SKILL.md` support file. | Command rejects nested discoverable skill package. | Automated |
| Skill install replacement | Install over an existing skill without `--retire-existing` or force behavior. | Collision is reported and existing skill remains. | Harness-sim automated |
| Skill install replacement | Install with `--retire-existing`. | Existing folder is backed up and new skill is installed. | Harness-sim automated |
| Harness setup | Open Codex/Claude after install and invoke `/skillspec`. | Harness sees the installed skill and follows the trampoline. | Manual |
| Harness setup | Verify setup across every supported harness on a developer machine. | Codex, Claude, and agents roots behave as documented. | Manual |

## Doctor Matrix

| Area | Case | Expected Result | Class |
| --- | --- | --- | --- |
| Doctor negative | Pass a file path instead of a folder or `SKILL.md`-acceptable target. | Error explains expected target shape. | Automated |
| Doctor negative | Pass a folder with empty `SKILL.md`. | Report or error identifies unusable/empty skill content. | Automated |
| Doctor negative | Pass a folder with malformed frontmatter. | Report flags frontmatter discovery risk or parse problem without panic. | Automated |
| Doctor negative | Pass a folder with malformed Markdown structure and no useful instructions. | Report identifies high drift/proof risk without crashing. | Automated |
| Doctor negative | Pass non-existent path. | Command fails with clear path error. | Automated |
| Doctor positive | Pass folder with one proper `SKILL.md`. | Simple skill report includes risk, activation surface, findings, and next action. | Automated |
| Doctor positive | Pass direct `SKILL.md` path when supported by doctor target handling. | Report is equivalent to the parent single-skill target or gives a precise unsupported-shape error. | Automated |
| Doctor positive | Pass folder with multiple `SKILL.md` files and cross references. | Workspace/package report includes one package report per skill and aggregate risk. | Automated |
| Doctor positive | Pass plugin-shaped folder. | Plugin workspace shape and package namespace are reported. | Automated |
| Doctor positive | Pass SkillSpec-backed skill. | Report recognizes contract mitigation and does not grade it as plain prose only. | Automated |
| Doctor remote | Pass public GitHub folder URL. | Remote sparse checkout is staged, analyzed, and cleaned up. | Automated |
| Doctor remote negative | Pass private or invalid GitHub URL. | Error is clear and does not leak credentials. | Automated |
| Doctor output | Run text, JSON, Markdown, and HTML output modes. | Each output parses/renders and contains the same core report facts. | Automated |
| Doctor in harness | Ask `/skillspec run doctor on ./my-skill` and ask agent to explain. | Agent invokes doctor and explains the report accurately. | Manual with trace review |

## Import Matrix

| Area | Case | Expected Result | Class |
| --- | --- | --- | --- |
| Import negative | Pass a file path where an atomic package folder is required. | Command fails with expected target-shape guidance. | Automated |
| Import negative | Pass folder with empty `SKILL.md`. | Command fails or generates no false-valid contract; error is actionable. | Automated |
| Import negative | Pass folder with malformed `SKILL.md`. | Command preserves source evidence and reports review blockers. | Automated |
| Import negative | Pass parent folder with multiple `SKILL.md` files to single-skill import. | Command rejects and points to workspace map/import flow. | Automated |
| Import positive | Pass folder with proper single `SKILL.md`. | Draft `skill.spec.yml`, source map, deps ledger, and reports are generated. | Automated |
| Import positive | Pass skill with references/resources. | Generated draft preserves references as imports/resources or review notes. | Automated |
| Workspace import positive | Map/import folder with multiple cross-referenced skills. | Workspace manifest, package graph, dependency edges, and package drafts are generated. | Automated |
| Workspace import positive | Map/import plugin-shaped folder. | Plugin namespaces are preserved and install slugs are deterministic. | Automated |
| Import QA | Run validate/imports check/deps check/test/compile after import. | Generated package reaches the expected QA stage or reports explicit blockers. | Automated |
| Install imported skill | Compile and install imported skill into sandbox target. | Generated trampoline and `skill.spec.yml` are installed. | Harness-sim automated |
| Replacement install | Install imported skill over existing prose skill with `--retire-existing`. | Old files are backed up and retired; new files are active. | Harness-sim automated |
| Replacement negative | Replacement install without retire/force. | Existing files remain and collision is reported. | Harness-sim automated |
| Activation | Invoke imported skill in live harness. | Trampoline hands off to SkillSpec CLI guidance instead of re-reading the full manual. | Manual with trace review |
| Activation negative | Activate imported skill when `skillspec` binary is missing. | Trampoline reports missing CLI and does not claim full alignment proof. | Manual |

## Activation And Execution Behavior Matrix

| Area | Case | Expected Result | Class |
| --- | --- | --- | --- |
| Plan/act | Run `skillspec plan` and `skillspec act` on a known spec. | Selected route, matched rules, forbids, and phase boundary are rendered. | Automated |
| Progress ledger | Record phase-completed and requirement evidence. | `<run-dir>/execution.jsonl` receives compact structured events. | Automated |
| Batch progress | Use `skillspec progress batch` for grouped proof. | Multiple evidence events are recorded in one compact operation. | Automated |
| Progress display | Run `skillspec progress show`. | Current/completed/blocked/remaining phase summary is accurate. | Automated |
| Trace alignment | Run `skillspec trace align` with decision trace and execution ledger. | Alignment status is `aligned`, `partial`, or `unproven` with missing proof rows. | Automated |
| Final response proof | Run `skillspec progress final-response`. | Final response evidence records result/evidence/alignment/token-savings sections. | Automated |
| Token stats | Run progress stats with a valid workspace stats report. | Token consumption and savings evidence is recorded and appears in alignment. | Automated |
| Token stats negative | Run stats with missing/empty token evidence. | Command refuses to invent token savings. | Automated |
| Harness behavior | Live agent uses fewer progress updates by batching evidence. | User sees compact updates while ledger still captures proof. | Manual with trace review |
| Harness behavior | Live agent final answer includes alignment and token report. | Final answer reports alignment summary, missing proof if any, and token usage honestly. | Manual with trace review |
| Harness negative | Live agent skips a required proof step. | Alignment reports partial/unproven rather than success. | Manual with trace review |

## Router Matrix

| Area | Case | Expected Result | Class |
| --- | --- | --- | --- |
| Router install | Install router into sandbox roots. | Managed `skill-router` folders, config, visibility manifest, hooks, and index are written. | Harness-sim automated |
| Router install | Verify pre-call hook files for chosen harnesses. | Hook command invokes `skillspec router guard` with installed config. | Harness-sim automated |
| Router install | Existing skills become explicit/manual-only. | Native visibility metadata or sidecars reflect explicit invocation. | Harness-sim automated |
| Router install | Index is populated. | `skillspec router index status` shows discovered and indexed skills. | Automated |
| Router guard | Run guard after install. | `first_hop_ready=true` and hook output is valid. | Automated |
| Router route positive | Query clear skill intent. | `skillspec route` returns `use_skill` with selected skill. | Automated |
| Router route bypass | Query ordinary non-skill task. | `skillspec route` returns `bypass` or `ambiguous` and no selected skill. | Automated |
| Router drift | Add out-of-band implicit skill after install. | Guard or index refresh detects and repairs explicit visibility. | Harness-sim automated |
| Router stale index | Modify skill roots after index. | Status reports stale/missing/index mismatch. | Automated |
| Router disable | Disable router mode. | Router first-hop is disabled and managed visibility is restored from manifest. | Harness-sim automated |
| Router disable check | Verify existing skills are switched back to their previous visibility, not blindly all implicit. | Visibility manifest restore is correct. | Harness-sim automated |
| Router enable | Re-enable router after disable. | Index refreshes and routed skills become explicit/manual-only again. | Harness-sim automated |
| Router uninstall | Uninstall router. | Router skill folders and hooks are removed or disabled; visibility is restored. | Harness-sim automated |
| Router negative | Install with invalid router name. | Command rejects invalid name. | Automated |
| Router negative | Guard with missing config. | Command fails with repair/install guidance. | Automated |
| Router live | Start Codex/Claude after router install and ask ordinary task. | Hook fires, router bypasses, and answer has no unnecessary router hops. | Manual |
| Router live | Ask task that should activate a domain skill. | Router chooses one skill and only that domain skill is loaded. | Manual with trace review |

## Durable Executor Matrix

| Area | Case | Expected Result | Class |
| --- | --- | --- | --- |
| Durable install | Install durable-executor from explicit source into sandbox roots. | Managed durable-executor folders and config are written. | Harness-sim automated |
| Durable install negative | Install without `rote` on PATH. | Command fails before writing managed durable executor. | Automated |
| Durable enable | Enable durable executor. | Durable executor visibility becomes implicit and config records enabled state. | Harness-sim automated |
| Durable disable | Disable durable executor. | Durable executor visibility becomes manual-only without deleting files. | Harness-sim automated |
| Durable update | Update managed durable executor. | Existing install is backed up and refreshed from source. | Harness-sim automated |
| Durable delete | Delete managed durable executor. | Managed installs are removed; unmanaged folders are not removed. | Harness-sim automated |
| Durable with router | Install router and durable executor together. | Durable executor can be implicit outer observer while router remains selection authority. | Manual with trace review |
| Durable happy path | Enable durable executor and run one SkillSpec-backed skill. | Workspace/evidence is preserved and final response can cite durable evidence. | Manual with trace review |
| Durable negative | User declines observation/record/memory. | Durable executor does not record or memorize events and task can continue direct if allowed. | Manual |
| Durable negative | Durable handoff loses required workspace or trace path. | Run reports blocker rather than pretending proof exists. | Manual with trace review |

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
9. Enable durable executor and run one happy-path durable skill execution.
10. Confirm router and durable executor remain independently disableable.

Do not merge if any manual test only "looks okay" but leaves missing alignment
proof unexplained. Record the blocker and either fix the code or narrow the
claim in docs.
