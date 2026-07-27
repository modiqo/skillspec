# Implementation Plan

Status: proposed. This is the build order for documents 36 through 39. No code
exists yet.

## Scope

Deliver `skillspec boundary`: a command that enumerates a skill's effect surface,
compiles a least-privilege boundary proposal, emits that proposal into
harness-native formats, detects concealment, and diffs effect surfaces across
revisions.

Out of scope for this plan: AST-based extraction, any model call, any runtime
enforcement, and validation level V2 from document 38 (which is gated behind an
investigation task below).

## 1. Crate Decision

Create a new crate `crates/skillspec-boundary`.

Rationale: doctor owns the follow-through-risk rubric and its scoring types. The
effect surface is a separate axis and documents 36 and the folder README require
that the two never merge into one score. Keeping them in separate crates makes
that structural rather than a matter of discipline. The dependency is one-way.

`source_map` is already `pub mod` in `crates/skillspec-doctor/src/lib.rs`, and
`remote_source` is `pub mod` as well, so the new crate can consume both without
changing doctor's visibility.

### Cargo Wiring

Add to the workspace members list in the root `Cargo.toml`, after
`crates/skillspec-doctor`:

```toml
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
skillspec-doctor = { version = "0.1.8", path = "../skillspec-doctor" }
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.140"
serde_yaml = "0.9.34"
sha2 = "0.10.9"
```

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
      shell.rs           lexical shell extraction
      python.rs          lexical python extraction
      javascript.rs      lexical js/ts extraction
      manifest.rs        deps.toml, package.json, requirements.txt,
                         pyproject.toml, Cargo.toml, go.mod
    normalize/
      mod.rs             shared entry points
      url.rs             scheme/host/port, interpolation, IP and IDN handling
      path.rs            expansion, lexical resolution, path class table
      argv.rs            basename, wrapper unwrapping, pipe-to-interpreter
      env.rs             variable extraction, credential-like marking
    concealment.rs       the six detectors from document 39
    proposal.rs          effect set -> grant set, compilation rules 1-7
    emit/
      mod.rs             target registry and dispatch
      skillspec.rs       tool_boundary YAML block
      claude_frontmatter.rs
      claude_settings.rs
      egress_allowlist.rs
    drift.rs             two surfaces -> change list
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
    /// The five classes that never become a silent allow grant.
    pub fn is_sensitive(self) -> bool {
        matches!(self, Self::Secret | Self::AgentConfig | Self::SkillPackage
                     | Self::ShellInit | Self::VcsConfig)
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
skillspec boundary <target> [--json] [--markdown]
skillspec boundary emit <target> --format <fmt> [-o <file>]
skillspec boundary diff <target> --against <ref> [--json]
skillspec boundary check <target> [--against <ref>] [--fail-on <level>]
```

Exit codes for `check` are defined in document 39 and must be implemented
exactly, including the separation of code 2 for incompleteness.

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

### M1: Effect Model And Markdown/Shell Extraction

- `effect.rs`, `dedupe.rs`, `extract/mod.rs`, `extract/markdown.rs`,
  `extract/shell.rs`, `normalize/*`
- `lib.rs::analyze` producing an `EffectSurface`
- `report.rs` with `skillspec.boundary.effect_surface.v0`
- Fixtures: `clean-formatter`, `github-reporter`, `unmapped-payload`,
  `dynamic-endpoint`, `wrapped-exec`
- Unit tests for all normalizers

Acceptance: `analyze` on `github-reporter` returns the expected grants with
correct reach and resolution; `dynamic-endpoint` produces exactly one unresolved
entry; determinism test passes.

### M2: CLI Surface And Human Rendering

- Domain facade, args, dispatch, module registration
- `render.rs` with the ordering from document 36
- `--json` and `--markdown`
- CLI integration tests
- Golden reports for all M1 fixtures

Acceptance: `skillspec boundary ./fixtures/effects/github-reporter` renders the
documented layout; `skillspec boundary <public-github-url>` works via the reused
doctor staging path.

### M3: Concealment

- `concealment.rs` with all six detectors
- `hidden-unicode` fixture, with the round-trip verification noted above
- Report integration and top-of-report placement

Acceptance: all six detectors fire on constructed inputs and stay silent on the
`clean-formatter` and `github-reporter` fixtures. The false-positive check on
clean fixtures is the acceptance criterion that matters.

### M4: Proposal And Emitters

- `proposal.rs` implementing compilation rules 1 through 7
- V0 internal-coverage checks from document 38
- `emit/skillspec.rs` and `emit/egress_allowlist.rs`
- `emit/claude_frontmatter.rs` and `emit/claude_settings.rs`, **after** the
  syntax verification task below
- `skillspec boundary emit`

Acceptance: `secret-reader` produces `permission_required_for` and never
`allow`; `dynamic-endpoint` produces `complete: false`; V0 coverage passes on
every fixture; each emitter has a golden output file.

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

## 9. Standing Tasks

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

## 10. Investigations Before Implementation

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

**I3: Workspace shapes.** This plan covers `simple_skill` only. Multi-skill,
entry-with-subskills, and plugin workspaces need a decision about whether the
effect surface aggregates or stays per-package, and cross-skill writes are a
finding class this plan does not yet define. Schedule after M5; do not let the
single-skill types harden in a way that blocks per-package reporting.

## 11. Risks

| Risk | Mitigation |
| --- | --- |
| Lexical extraction misses effects | Deny-by-default means misses fail closed. The report states extraction mode. AST extraction is a later milestone with an explicit trigger. |
| Over-enumeration produces noisy grants | `clean-formatter` fixture is the guard, and its expected output is deliberately small. |
| Emitted policy syntax goes stale | Standing verification task, dated comment per emitter, golden output per emitter. |
| Feature is read as a security guarantee | The no-enforcement line appears in every rendering, and wording review is added to the QA checklist. |
| Harness vendors ship native permission inference | Accepted. The durable part is the evidence trail from grant to source line, not the grant list. |
| Scope creep toward a classifier | The detector set is fixed at six and lives in one module. Adding a seventh is a design decision, not an implementation detail. |

## 12. Preflight

Per `CONTRIBUTING.md`, before any PR from this work:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
```

## 13. Sources

- `crates/skillspec-doctor/src/lib.rs`, `source_map.rs`, `remote_source.rs`,
  `frontmatter.rs` - everything this crate consumes.
- `crates/skillspec-cli/src/cli/args/mod.rs`,
  `crates/skillspec-cli/src/cli/dispatch/doctor_cmd.rs`,
  `crates/skillspec-cli/src/domain/doctor.rs` - the CLI patterns to copy.
- `crates/skillspec-harness-lab/src/report.rs`, `lab.rs` - baseline comparison
  and sandbox helpers to reuse.
- `docs/design/operations/29-internal-domain-facades.md` - facade requirement.
- `CONTRIBUTING.md` - preflight gate.
