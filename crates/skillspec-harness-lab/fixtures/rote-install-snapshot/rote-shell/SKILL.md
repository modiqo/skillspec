---
name: rote-shell
description: "Use for CLI and shell work through rote: running local commands with `rote exec`, capturing stdout/stderr/files, following logs and background processes, checking dependency manifests, mixing adapters/browser/process steps, and crystallizing CLI work into TypeScript flows. Prefer rote shell primitives over raw shell when the command result should be remembered, queried, replayed, or shared."
---

# rote-shell

Use rote for shell and CLI work when the result should become workspace memory:
commands, outputs, files, logs, process lifecycle, dependencies, and flow replay.
Raw shell is still fine for tiny inspection commands, but work that matters
should pass through rote so it can be queried, audited, and crystallized.

Load the shell guidance when you need the complete command model:

```bash
rote guidance shell essential
```

## First Decision

Choose the narrowest rote primitive that preserves evidence:

- one-shot command: `rote exec -- <program> [args...]`
- stdin from a file: `rote exec --stdin-file input.txt -- <program>`
- declared output file: `rote exec --capture-file label:path -- <program>`
- moving file/log: `rote stream follow --file logs/server.log --until READY`
- long-running process: `rote exec --background --ready-log READY -- <program>`
- process stream: `rote stream follow-process proc-1 --stream stdout --until READY`
- terminal-sensitive command: `rote pty run -- <program> [args...]`
- dependency preflight: `rote deps check deps.toml`
- crystallized replay: `rote deno run --allow-all ~/.rote/flows/<name>/main.ts`

Do not replace these with ad hoc `command > file`, `tail -f`, or `ps | grep`
when the evidence should be durable. Rote already stores typed responses,
artifacts, hashes, offsets, process leases, and command-log provenance.

## Browser Intent Router

If the user says "browse", "open this site", "attach to my browser", "use the
page", "click", "type", "snapshot", "extract from the page", "extract social
profiles", "Gmail in browser", or otherwise asks for live web UI state, stop
shell routing and invoke `/rote-browse`.

Do not satisfy browser intent with `rote exec`, raw Playwright, native web
search, WebFetch, `open`, `curl`, or a saved non-browser flow unless the user
explicitly switches substrate. Native search may help discover a URL only when
the user asked for search/discovery or the browser route cannot identify a URL;
it is not a substitute for browsing and extracting the page with rote.

Browse intent has precedence over the domain noun. For example, "browse my
calendar", "browse Gmail", "browse HubSpot", or "browse Salesforce" means the
user wants browser-state access even though those domains may also have APIs.
You may still run `rote flow search "<intent>"` first, but only use an
adapter/flow if it is installed, healthy, and completes the request. If the
adapter/flow is missing, stale, unauthenticated, or fails setup, do not ask the
user to build an adapter before trying the browser route. Hand off to
`/rote-browse` and use an existing headed browser when the task depends on the
user's logged-in profile.

For public profile extraction, such as "browse each committer's social
profile", use GitHub/CLI/API data to collect candidate URLs and identities,
then use `/rote-browse` to visit and extract the public pages. Do not replace
that browse step with native web search summaries.

Default browser decision:

- Existing login, Gmail, SSO, MFA, extensions, active tabs, or profile state:
  ask to attach to an existing headed browser.
- Public read-only page, CI, or replay-like work: ask whether headless
  new-session is acceptable.
- The user says "browse" without choosing mode: ask headed vs headless; if
  headed, ask attach-existing vs new rote-managed headed browser.

For active browser attach, hand off to `/rote-browse` and use its sequence:

```bash
rote browser attach setup --method extension --browser chrome
rote browse <url> --headed --attach-existing --new-tab --no-prompt --no-snapshot
rote browse wait --selector '<ready-selector>' --timeout 30 --quiet-ms 750
rote browse snapshot
```

For browser plus shell work, keep both in the same workspace. Use
`/rote-browse` for page leases, snapshots, slices, refs, auth state, and
readiness. Use `/rote-shell` only after browser state has been materialized as
a saved response, snapshot, slice, or file that a local CLI should process.

## Current Capability Map

Use only shipped commands. Do not invent aliases from the roadmap.

| Need | Shipped command | Notes |
| --- | --- | --- |
| One-shot process capture | `rote exec -- <program> [args...]` | Direct argv by default. |
| File stdin | `rote exec --stdin-file input.txt -- <program>` | Records stdin provenance. |
| Declared output file | `rote exec --capture-file label:path -- <program>` | Captures file metadata and artifact pointer. |
| Saved stdout/stderr files | `rote exec --stdout-file out.txt --stderr-file err.txt -- <program>` | Use for durable stream files. |
| Dependency preflight | `rote deps check deps.toml` | No install side effects. |
| Moving file/log stream | `rote stream follow --file app.log --until READY` | Supports offsets, chunks, hashes, and pattern stop. |
| Background start | `rote exec --background --ready-log READY -- <program>` | Creates a tracked lease such as `proc-1`. |
| Background status | `rote exec status proc-1` | Query lease state before acting. |
| Background wait | `rote exec wait proc-1 --timeout-ms 300000 --poll-ms 500` | Blocks until a tracked finite job exits or times out; records exit plus stdout/stderr observations. |
| Background stdout/stderr follow | `rote stream follow-process proc-1 --stream stdout --until READY` | Reads from background log artifacts. |
| Background stop and cleanup | `rote exec stop proc-1` | On Unix, stops the process group and records cleanup facts. |
| One-shot terminal transcript | `rote pty run -- <program> [args...]` | Use when the command must see a terminal. |

Deferred or not shipped as commands yet:

- `rote exec attach`
- `rote exec log`
- explicit `detach`
- persistent PTY sessions: start, send, snapshot, stop, attach
- non-log readiness probes such as HTTP, TCP, file, or command probes
- direct OS-pipe stream handles before background output reaches log artifacts
- non-Unix process-group cleanup guarantees

If a task needs a deferred feature, say so and use the nearest shipped primitive
instead. For example, use `rote exec wait` for finite tracked jobs, use
`rote stream follow-process ... --until <pattern>` for log observation or
readiness, and use `rote exec status` plus `rote exec stop` instead of raw
`ps`/`kill` when the process is tracked.

## TypeScript SDK Pattern Map

For authored TypeScript flows, use first-class SDK wrappers instead of
hand-assembling command arrays. The SDK surface mirrors the shipped shell
patterns:

| Pattern | SDK call |
| --- | --- |
| Durable one-shot | `await rote.exec({ argv: ["git", "status", "--short"], deps: ["git"] })` |
| Declared stdin/output files | `await rote.exec({ argv, stdin: { file }, capture: { stdout: { file }, files: [{ label, path }] } })` |
| Dependency preflight | `await rote.depsCheck({ manifest: "deps.toml" })` |
| Tracked background job / detach-like work | `await rote.execBackground({ argv, readyLog, readyTimeoutMs, capture })` |
| Long-running job with useful parallel work | `await rote.execBackgroundAndJoin(request, async (job) => { ... }, { timeoutMs, pollMs, stopOnWorkError })` |
| Lease status | `await rote.execStatus("proc-1")` |
| Lease wait | `await rote.execWait("proc-1", { timeoutMs: 300_000, pollMs: 500 })` or `await job.wait(...)` |
| Lease cleanup | `await rote.execStop("proc-1")` |
| Moving file stream | `await rote.followFile("logs/app.log", { until: "READY" })` |
| Background process stream | `await rote.followProcess("proc-1", "stdout", { until: "READY" })` |
| One-shot PTY transcript | `await rote.ptyRun({ argv, cols: 100, rows: 30 })` |
| Authored ordered fan-out | `await rote.execMany(requests, { stopOnError: false })` |

Use `rote.shell().<method>` when you want the shell namespace explicitly; the
top-level `rote.<method>` forms are convenience aliases for authored flows.

Do not invent SDK methods for deferred roadmap items. There is no
`rote.detach`, persistent PTY `send`, or direct OS-pipe stream handle yet. The
current detach-like pattern is a tracked background lease with stdout/stderr
files, `execWait`, `followProcess`, `execStatus`, and `execStop`.

`rote.execMany` preserves workspace response ordering by running process
requests serially. For true parallel shell fan-out, generate declarative
frontmatter `steps:` with `type: process.exec`, `for_each`, and
`max_concurrency` so the DAG runner owns the scheduling and provenance.

For long-running finite jobs in authored TypeScript, prefer
`execBackgroundAndJoin` or `job.join` when there is useful adapter, browser,
process, file, or explicit stream work to do while the lease runs. The callback
creates normal semantic DAG evidence; the final `exec wait` is the join point.
Do not generate heartbeat or polling loops as DAG work.
`rote.execBackground(...)` already prints the lease and poll commands to
stderr. Do not duplicate that announcement in crystallized flows. Use
`announce: false` only for deliberately quiet flows.

## Crystallization Router

When the user asks to turn shell exploration into a reusable flow, choose the
flow shape from the work pattern:

| Exploration pattern | Crystallized shape |
| --- | --- |
| One finite command whose output is the fact | `rote.exec({ argv, deps, capture })` |
| Command writes files that downstream work reads | `rote.exec({ capture: { files: [...] } })` plus typed file paths |
| Existing file or log is the source of truth | `rote.followFile(path, options)` |
| Long finite job where other useful work can run | `rote.execBackgroundAndJoin(request, async (job) => { ... }, options)` |
| Long service or daemon with readiness | `rote.execBackground({ readyLog, capture })`, then status/follow/stop |
| Need to inspect progress from a tracked lease | `job.follow(...)` or `rote.followProcess(...)` |
| Need completion proof from a tracked lease | `job.wait(...)` or `rote.execWait(...)` |
| Terminal behavior is the point | `rote.ptyRun({ argv, input, cols, rows })` |
| Many independent commands share one shape | declarative `steps:` with `process.exec`, `for_each`, and `max_concurrency` |

Crystallize causality, not waiting. Do not encode heartbeat loops, repeated
status polling, or sleep/retry scaffolding as business DAG nodes. Those are
observation mechanics. The reusable flow should expose semantic actions and
joins: start work, observe meaningful artifacts or streams, wait for completion,
then summarize.

Before authoring a TypeScript shell flow, read:

```bash
rote guidance typescript flow-creation
```

That guide owns frontmatter, `deps.toml`, FlowOutput, release QA, and the shell
SDK wrapper contract.

## Strategy Pattern Library

Map the user's vague request to the smallest shipped pattern that preserves
evidence:

| Signal | Pattern | Use |
| --- | --- | --- |
| Tiny disposable inspection | Raw shell allowed | direct harness shell |
| Result may be queried, compared, summarized, or replayed | Durable one-shot | `rote exec --` |
| Command reads a known file | Declared file input | `rote exec --stdin-file` or a direct argv path |
| Command creates a file that matters | Declared file output | `rote exec --capture-file` |
| Full stdout/stderr matters | Durable stream files | `--stdout-file`, `--stderr-file` |
| Required tools or input files matter | Dependency gate | `deps.toml` plus `rote deps check` |
| Existing log is moving | File stream watch | `rote stream follow --file` |
| Start a server or daemon-like process | Service lease | `rote exec --background --ready-log` |
| Long finite non-interactive job | Tracked background job | `rote exec --background --stdout-file --stderr-file` |
| Inspect background output | Process stream observation | `rote stream follow-process` |
| Need liveness before acting | Lease status | `rote exec status` |
| Need cleanup | Lease cleanup | `rote exec stop` |
| Many independent items share a command shape | Fan-out batch | `steps:` with `process.exec`, `for_each`, and `max_concurrency` |
| API result feeds CLI or CLI result feeds API | Mixed substrate chain | adapter/browser primitive plus `rote exec` |
| Browser snapshot/file feeds local CLI | Browser-file bridge | browser snapshot/file plus `rote exec` |
| Release, publish, deploy, or global mutation | Guarded mutation | deps check, tool-native dry-run, approval, then foreground unless background is approved |
| Command checks whether stdout is a terminal | One-shot PTY transcript | `rote pty run --` |
| Command may prompt interactively and can be scripted safely | One-shot PTY with bounded input | `rote pty run --input` or `--stdin-file` |
| Command needs ongoing human interaction | Foreground or defer | persistent PTY attach is not shipped |

`--dry-run`, `--resume`, `--force-resume`, and `--max-concurrency` are DAG
runner controls for frontmatter `steps:` flows. They are not universal
`rote exec` flags. For a single command, use the tool's own dry-run/check mode
when available.

## Workspace Setup

CLI work must happen inside a rote workspace:

```bash
rote init cli-work --seq --force
rote workspace sandbox cli-work off
cd ~/.rote/rote/workspaces/cli-work
```

Keep commands simple and literal. Avoid shell control operators, command
substitution, and long `&&` chains unless the user explicitly needs shell
semantics. Prefer direct argv:

```bash
rote exec -- rg TODO docs
```

Use shell parsing only when it is the real subject of the work:

```bash
rote exec -- sh -c 'printf "alpha\n" | tr a-z A-Z'
```

## Query After Every Meaningful Step

Treat every `@N` as the handle for what just happened. The handle is saved
evidence, not always the raw business payload. Route by type.

After a command finishes, read `@@result` first:

- `response_id` is the saved evidence handle
- `response_kind` tells you how to query the evidence
- `primary_query`, `primary_stdout_query`, `primary_stderr_query`, or
  `artifact_query` are exact typed queries for the saved response
- `@@next` remains the immediate next-action guide

For one-shot process output, prefer the typed query fields from `@@result`:

```bash
rote query @1 '.stdout.text' -r
rote query @1 '.stderr.text' -r
rote query @1 '.status.exit' -r
rote query @1 '.files' -r
rote query @2 '.cleanup' -r
```

Use `@proc` addresses when inspecting process responses:

```bash
rote query @proc.last.stdout '.text' -r
rote query @proc.1.exit '.' -r
rote query @proc.last.transcript '.text' -r
```

When `@@result` gives an exact query such as
`rote query @proc.4.transcript '.text' -r`, prefer it over `@proc.last` if any
other command may run before you query. Use `rote @N .` for full provenance
inspection, not as a shortcut for stdout or PTY transcript text.

Do not summarize from terminal scrollback when a structured query exists.

## Files And Artifacts

Declare file inputs and outputs instead of relying on memory:

```bash
rote exec \
  --stdin-file input.txt \
  --capture-file summary:out/summary.txt \
  -- python3 scripts/summarize.py
```

This records stdin provenance, file change state, media type, hashes when
available, and artifact paths under `.rote/artifacts/processes/@N/`.

## Background Processes

Start servers as leases, not mystery PIDs:

```bash
rote exec --background --ready-log "Listening" --ready-timeout-ms 10000 -- npm run dev
rote exec status proc-1
rote stream follow-process proc-1 --stream stdout --until "GET /health"
rote exec stop proc-1
rote query @3 '.cleanup' -r
```

On Unix, rote starts background commands in a process group and stops the group
with `TERM` followed by `KILL` if needed. Inspect `.cleanup` before claiming a
process stopped.

## Long-Running Finite Jobs

For long-running but finite commands, use a tracked background lease instead of
an untracked detach. `rote exec detach` is not shipped yet. The current
detached-like primitive is:

```bash
rote exec \
  --background \
  --stdout-file logs/job.stdout.log \
  --stderr-file logs/job.stderr.log \
  -- cargo test --all-targets --all-features

rote exec status proc-1
rote exec wait proc-1 --timeout-ms 600000 --poll-ms 1000
rote stream follow-process proc-1 --stream stdout --from-start --max-bytes 65536
rote stream follow-process proc-1 --stream stderr --from-start --max-bytes 65536
```

Use this when the command is non-interactive, can safely continue while the
agent does other work, and its stdout/stderr are enough to monitor progress.
Use `exec wait` to establish completion and exit status. Use
`follow-process` to inspect output, not as the primary completion detector.
Keep the lease id (`proc-1`) in the task notes and query status before making
claims about completion if wait has not finished.

In authored TypeScript, prefer the semantic join helper:

```ts
const joined = await rote.execBackgroundAndJoin(
  { argv: ["cargo", "test"], deps: ["cargo"] },
  async (job) => {
    const checks = await rote.exec({ argv: ["gh", "pr", "checks"], deps: ["gh"] });
    const stderr = await job.follow("stderr", { fromStart: true, maxBytes: 65536 });
    return { checks, stderr };
  },
  { timeoutMs: 600_000, pollMs: 1_000, stopOnWorkError: true },
);
```

The callback's work becomes semantic DAG evidence. The wait remains the
completion join. Poll/heartbeat internals do not become graph nodes.

Do not background a command when it may prompt for credentials, OTP, passphrase,
license acceptance, or confirmation. `rote pty run` can capture a bounded
one-shot terminal transcript, but it does not provide persistent attach. Ongoing
interactive jobs should run foreground or be postponed until the user can drive
the prompt.

## One-Shot PTY

Use PTY only when terminal behavior is the point: commands that check
`isatty`, render progress differently on terminals, or can be driven with
bounded scripted input.

```bash
rote pty run --cols 100 --rows 30 -- script-that-checks-tty
rote pty run --input "yes\n" -- interactive-but-scriptable-command
rote query @proc.last.transcript '.text' -r
```

Do not pass secrets through `--input`; terminal echo may record them in the
transcript. Do not use PTY for persistent REPLs or long human-driven sessions
until start/send/snapshot/stop support exists.

Treat release and publish commands as high risk. For examples like
`cargo release`, `npm publish`, `gh release create`, or deploy commands:

1. Run dependency preflight first.
2. Run the tool's dry-run/check mode first when available.
3. Show the planned mutation and ask the user for approval before the real run.
4. Prefer foreground for final irreversible publish steps unless the user
   explicitly approves tracked background execution.
5. If background execution is approved, capture stdout/stderr to files, follow
   process streams, and keep the flow `draft` until completion evidence is
   captured.
6. Do not stop a release/publish process unless the user asks; interruption can
   leave external state partially mutated.

Decision rule:

- short and non-interactive: foreground `rote exec`
- long and non-interactive: tracked `rote exec --background`
- long-running service: tracked `rote exec --background --ready-log ...`
- interactive: foreground or wait for PTY/attach support
- irreversible release/publish: dry-run, user approval, then foreground unless
  tracked background is explicitly requested

## Dependency Manifests

When a task depends on local tools, create or update
`~/.rote/flows/<name>/deps.toml` before replay. This is not optional for
crystallized shell flows: if the flow calls `rote.exec({ argv })` for `git`,
`gh`, `cargo`, `python`, `node`, `jq`, `rg`, or any other local executable, the
flow directory must include a matching dependency manifest before the flow is
marked `released`.

```toml
schema_version = 1

[[tools]]
id = "github-cli"
command = "gh"
required = true
version_command = ["gh", "--version"]

[files]
required = ["input.txt"]
```

Then run:

```bash
cd ~/.rote/flows/<name>
rote deps check deps.toml
```

Do not auto-install tools into global locations unless the user approved that
specific provisioning policy. A crystallized flow should declare dependencies
so the target environment can check or provision them before work begins.

If `rote deps check deps.toml` reports missing required tools, stop and elicit
an install decision from the user before continuing. Show the missing tool,
why the flow needs it, and the lowest-risk install scope available. Prefer
project-local or rote-managed installs over global package manager installs.
Only install globally when the user explicitly approves that scope.

Use this shape:

```text
The flow needs these missing tools before replay:
- gh: required for GitHub PR and Actions checks

I can install/provision them using one of these scopes:
- rote-managed/project-local: preferred when available; keeps replay isolated
- user/global package manager: broader machine mutation; requires explicit approval

Do you want me to install/provision the missing tools, or should I leave the
flow in draft until you install them?
```

After any approved install or user-managed install, rerun:

```bash
cd ~/.rote/flows/<name>
rote deps check deps.toml
```

Never mark the flow `released` while required dependencies are still missing.

## Mixed Workflows

For API plus CLI work, use the adapter for typed API calls and `rote exec` for
local CLI facts. Example shape:

```bash
rote POST /github '{"method":"GET","path":"/repos/$owner/$repo"}' -t -s
rote exec -- gh repo view "$owner/$repo" --json name,visibility
rote query @1 '.' -r
rote query @2 '.stdout.text' -r
```

For browser plus CLI work, use `rote-browse` for page state and `rote-shell`
for local processing of snapshots/artifacts. Do not drop into raw Playwright or
raw shell when a rote primitive can preserve the evidence.

Current mixed replay support is intentionally asymmetric:

- adapter and `process.exec` actions are first-class `steps:` DAG actions
- browser observations are bridged through saved responses, snapshots, and
  files until first-class browser DAG actions ship

For browser-involved crystallization today, capture the browser state with
`rote-browse`, materialize the snapshot or slice to a file, then consume that
file from `process.exec`.

## Crystallization Rule

After a useful CLI workflow works twice, ask whether to crystallize it. The
canonical replay command for TypeScript flows is:

```bash
rote deno run --allow-all ~/.rote/flows/<name>/main.ts
```

Use `rote flow run` only as compatibility syntax. New generated usage should
prefer `rote deno run --allow-all`.

## Crystallization Workflow

When the user says yes, do the full release discipline:

1. Choose the correct scaffold path:

   - Shell-only flow: write `~/.rote/flows/<name>/main.ts` manually with
     `@rote-frontmatter`. Do not run `rote flow pending save`,
     `rote flow template create`, or `rote flow frontmatter`; those commands
     require `--adapter` in the current implementation and will fail or emit an
     adapter-shaped flow.
   - Mixed adapter/shell flow: use `rote flow frontmatter` or
     `rote flow template create` so adapter fingerprints and parameters are
     captured. Then choose one execution shape:
     - Declarative DAG: add top-level `steps:` when the replay can be expressed
       as adapter calls plus `type: process.exec` actions.
     - Authored SDK: write TypeScript with `FlowOutput`, `runPreflight(...)`,
       and SDK calls such as `rote.exec({ argv })` when custom branching,
       formatting, help text, or richer TypeScript logic is needed.

   Minimal shell-only frontmatter shape:

   ```typescript
   /**
    * CSV Sales Report
    *
    * Summarizes a local CSV into JSON and a text report.
    *
    * @rote-frontmatter
    * ---
    * name: csv-sales-report
    * description: "Summarizes a local CSV into JSON and a text report using rote shell primitives."
    * provenance:
    *   tier: local
    *   workspace: csv-json-report-demo
    * metadata:
    *   status: draft
    *   kind: atomic
    *   flow_type: sequential
    *   format: typescript
    *   requires_endpoints: []
    *   requires_sessions: false
    *   parameters:
    *   - name: input
    *     type: string
    *     required: true
    *     description: "Input CSV path"
    *   - name: out_dir
    *     type: string
    *     required: false
    *     default: "out"
    *     description: "Output directory"
    *   tags:
    *   - shell
    *   - process
    *   - typescript
    * ---
    */
   ```

2. Create `~/.rote/flows/<name>/main.ts`.
3. For declarative DAG flows, put the replay graph in frontmatter `steps:`.
   For authored SDK flows, use TypeScript SDK shell primitives for process
   work: `rote.exec`, `rote.execBackground`, `rote.execStatus`,
   `rote.execStop`, `rote.followFile`, `rote.followProcess`, `rote.ptyRun`,
   `rote.depsCheck`, and `rote.execMany`. Use adapter handles from
   `runPreflight(...)` for adapter work.
4. Create `~/.rote/flows/<name>/deps.toml` for every local tool or required
   input file. For shell-only flows, prefer a real `deps.toml` file over
   frontmatter-only dependency prose so replay can run `rote deps check`.
5. Keep frontmatter parameters, usage text, and the flag parser in lockstep.
   If frontmatter names use underscores, the parser must accept those exact
   names or documented aliases; do not ship `github_repo` metadata with only a
   `--github-repo` parser unless both forms work.
6. Start with `metadata.status: draft`. Do not set `released` until dependency
   preflight and replay QA have passed.
7. Test the draft with at least three distinct inputs:

   ```bash
   cd ~/.rote/flows/<name>
   rote deps check deps.toml
   rote deno run --allow-all ~/.rote/flows/<name>/main.ts <input-a>
   rote deno run --allow-all ~/.rote/flows/<name>/main.ts <input-b>
   rote deno run --allow-all ~/.rote/flows/<name>/main.ts <input-c>
   ```

   If dependency preflight fails, ask the user whether to provision missing
   required tools, with the install scope and side effects stated plainly. Do
   not continue release QA until dependency preflight passes or the user says
   to keep the flow as `draft`.

8. Use the right QA gate for the execution shape.

   Declarative `steps:` DAG flows:

   ```bash
   rote flow validate ~/.rote/flows/<name>/main.ts
   rote deno run --allow-all ~/.rote/flows/<name>/main.ts --dry-run <params>
   rote deno run --allow-all ~/.rote/flows/<name>/main.ts <params>
   rote deno run --allow-all ~/.rote/flows/<name>/main.ts --resume latest <params>
   ```

   `rote flow lint` checks the authored `FlowOutput` contract and is not a
   release gate for pure `steps:` DAG flows whose TypeScript body is bypassed by
   the DAG runner.

   Authored SDK flows:

   ```bash
   rote flow lint <name>
   rote deno run --allow-all ~/.rote/flows/<name>/main.ts --help
   rote deno run --allow-all ~/.rote/flows/<name>/main.ts <known-good-input>
   ```

   In both shapes, loop until hardcoded paths, missing dependency declarations,
   mismatched parameters, raw shell leaks, and output-format issues are fixed.

9. Mark the flow released only after QA passes. Current practice is to edit the
   frontmatter status from `draft` to `released`, then rebuild and verify the
   index:

   ```bash
   rote flow index --rebuild
   rote flow search "<name or relevant keywords>"
   ```

10. If a pending workspace stub was used, discard it after release:

   ```bash
   rote flow pending discard <workspace>
   ```

Do not claim a shell-derived flow is released from memory. The release claim
must point back to `rote deps check deps.toml`, command output, saved responses,
and successful `rote deno run --allow-all` executions.

## Step Reference Rules For Mixed DAGs

In `steps:` flows, `@step{.path}` resolves against the step response body, not
the persisted `@N` envelope on disk. For an MCP adapter call, the response body
is the JSON-RPC body, so a GitHub adapter text payload usually needs a scalar
query such as:

```text
@repo_adapter{.result.content[0].text | fromjson | .full_name}
```

For process steps, the response body is `process.exec`, so downstream steps can
reference fields such as `.stdout.text` or captured file metadata. Keep
`process.exec` `argv`, stdin paths, and capture paths scalar after resolution.
Do not pass large adapter/browser payloads through argv when a file bridge or a
small scalar projection is clearer.
