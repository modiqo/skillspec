# Skill Effect Surface

Status: proposed. Nothing in this document is implemented. It defines the data
model that documents 37, 38, 39, and 40 build on.

## Purpose

Enumerate every host-visible effect a skill package could cause if an agent
followed its instructions, with evidence for each one, so that a least-privilege
boundary can be derived from the result.

The analysis must answer:

```text
What hosts, paths, binaries, environment variables, tools, and packages does
this skill reach, where is each one written, and how certain is the target?
```

It must not answer whether any of those reaches is malicious. That judgment is
left to the reader of the report and to the boundary they approve.

## Design Constraints

1. **Runs on unmodified sources.** Input is a folder or a public GitHub target,
   exactly as `skillspec doctor` accepts today. No `skill.spec.yml`, no compile,
   no install, no model call.
2. **Deterministic.** Same input bytes produce the same report. This is what
   makes drift comparison (document 39) meaningful.
3. **Evidence-bound.** Every observation carries file, line, and the raw text it
   was derived from. An observation a reviewer cannot locate is a defect.
4. **Honest about resolution.** An effect whose target cannot be determined is
   reported as unresolved, never guessed. Unresolved targets are the primary
   input to human review.
5. **Complete over precise.** Over-enumeration produces a boundary that is too
   permissive in review but visible; under-enumeration produces a boundary that
   denies something the skill needs, which validation catches. Both failure modes
   are recoverable. Silent omission from the report is not.

## Effect Model

### Effect Classes

Version 0 defines nine classes. The set is deliberately small: each class must
map onto something a real permission system can express.

| Class | Meaning | Target shape |
| --- | --- | --- |
| `net.egress` | Sends data off-host | host, scheme, optional port |
| `net.fetch` | Brings remote content into the run | host, scheme |
| `fs.read` | Reads a path | path pattern plus path class |
| `fs.write` | Writes, creates, or deletes a path | path pattern plus path class |
| `proc.exec` | Invokes a binary or subprocess | binary name plus argv shape |
| `env.read` | Reads an environment variable | variable name |
| `tool.invoke` | Calls a harness or MCP tool | tool identifier |
| `agent.config` | Writes agent-controlling state | config path class |
| `pkg.install` | Installs a dependency | ecosystem, package, pinned flag |

Notes on the less obvious ones:

- `net.egress` and `net.fetch` are separated because they need different
  boundary treatment. Egress is the exfiltration channel. Fetch is the untrusted
  content channel, and content fetched into an agent's context is an instruction
  vector regardless of whether any data left the host. A single request can be
  both; when the extractor cannot tell, emit both and let the proposal collapse
  them.
- `agent.config` is a narrowed `fs.write`, not a separate mechanism. It exists as
  its own class because writes to agent-controlling state persist beyond the
  current run and therefore deserve separate boundary treatment. See the path
  class table below.
- `pkg.install` records whether the requested version is pinned. Unpinned
  installs mean the effect surface is not fully determined by the package
  contents.

### Path Classes

Paths are normalized and then classified. The class, not the literal path,
drives boundary generation.

| Path class | Matches (illustrative, not exhaustive) |
| --- | --- |
| `secret` | `~/.ssh`, `~/.aws`, `~/.config/gcloud`, `~/.kube`, `.env`, `.netrc`, `.pgpass`, keychain paths, `id_rsa`, `*.pem` |
| `agent_config` | `~/.claude/`, `.claude/`, `settings.json`, `~/.codex/`, `~/.agents/`, `~/.skillspec/`, `CLAUDE.md`, `AGENTS.md`, `MEMORY.md`, hook configuration |
| `skill_package` | Any `SKILL.md` or skill folder other than the one being analyzed |
| `shell_init` | `~/.bashrc`, `~/.zshrc`, `~/.profile`, `~/.zshenv` |
| `vcs_config` | `.git/config`, `.git/hooks/`, `~/.gitconfig` |
| `workspace` | Paths under the current project root |
| `user_home` | Other paths under the user's home directory |
| `system` | `/etc`, `/usr`, `/bin`, `/Library`, `C:\Windows` |
| `temp` | `/tmp`, `$TMPDIR`, platform temp roots |
| `unknown` | Anything that normalization could not place |

`secret`, `agent_config`, `skill_package`, `shell_init`, `vcs_config`, and
`unknown` are the six classes that never become a silent allow grant. Document 37
defines how the proposal handles them.

`unknown` is in that list deliberately. Path normalization is lexical - it never
touches the filesystem, because a report must not depend on the analyst's
machine - so it will fail to place paths that a real process would resolve
without difficulty: absolute home paths on an unfamiliar layout, `..` traversal
through a symlink, platform-specific variable syntax, case variation on a
case-insensitive volume. Those are exactly the forms an evasion would take.

Treating `unknown` as ordinary would make it the one place in the model where
uncertainty resolves toward permission, which contradicts the design's central
property. Uncertainty resolves toward review everywhere, including here.

### Target Resolution

Every observation records how completely its target was determined.

| Resolution | Meaning | Example |
| --- | --- | --- |
| `literal` | Fully determined from source text | `curl https://api.github.com/user` |
| `templated` | Determined except for interpolated segments | `curl https://api.github.com/repos/$OWNER/$REPO` |
| `dynamic` | Target is computed at runtime | `curl "$ENDPOINT"`, `eval "$CMD"` |
| `unknown` | An effect is present but the extractor could not attribute a target | a shell pipeline the lexer declined to parse |

This field carries most of the honesty in the model. `dynamic` and `unknown`
observations cannot be turned into a grant. They are reported as
**unresolved effects**, and document 37 requires that a proposal containing them
be marked as incomplete rather than silently narrowed.

### Reach

Reach records where in the package an effect was found. It reuses the file
classification `skillspec doctor` already computes in
`crates/skillspec-doctor/src/lib.rs`.

| Reach | Meaning |
| --- | --- |
| `activation` | In the `SKILL.md` body, loaded into context at activation |
| `deferred` | In a file the Markdown references |
| `unmapped` | In a package file that nothing in the Markdown references |

`unmapped` is the notable one. Doctor already counts unmapped package files as a
hygiene finding (`unmapped_package_surface`). An effect found only in an unmapped
file is present in the shipped package and absent from anything a reviewer would
read by following the documentation. That is not evidence of intent, and the
report must not describe it as such, but it is a fact worth surfacing on its own
line.

Reach does not change whether an effect is granted. It changes how the effect is
presented for review, and it is recorded in the report so that a reviewer can
sort by it.

### Observation Record

```rust
pub struct EffectObservation {
    /// Stable within one report: "effect-0001".
    pub id: String,
    pub class: EffectClass,
    pub target: EffectTarget,
    /// Source text the observation was derived from, trimmed to 200 chars.
    pub raw: String,
    pub resolution: TargetResolution,
    pub origin: EffectOrigin,
    pub reach: Reach,
    pub confidence: Confidence,
    pub evidence: EffectEvidence,
}

pub struct EffectEvidence {
    pub path: String,
    pub line: Option<usize>,
    /// Source-map node id when the observation came from Markdown.
    pub node: Option<String>,
    pub text_preview: String,
}

pub enum EffectOrigin {
    Frontmatter,
    MarkdownProse,
    MarkdownCommandExample,
    MarkdownCodeBlock,
    ScriptFile,
    Manifest,
}

pub enum Confidence {
    /// Syntactically unambiguous: a URL literal, an argv[0].
    High,
    /// Pattern-derived: a path mentioned in prose.
    Medium,
    /// Weak signal retained because omission fails open.
    Low,
}
```

`confidence` describes how sure the extractor is that the effect exists at all.
`resolution` describes how sure it is about the target. They are independent: a
`curl "$URL"` is a `High` confidence `net.egress` with `dynamic` resolution.

## Extraction Sources

Extraction runs over the `SourceMap` that
`crates/skillspec-doctor/src/source_map.rs` already produces, plus raw file
bytes for non-Markdown files.

### 1. Frontmatter

Frontmatter yields **declarations**, not observations. Parse `allowed-tools` and
`disallowed-tools` when present. These feed the declared-versus-observed
comparison in document 37 and are recorded separately from the effect set.

`skillspec doctor` currently parses frontmatter in
`crates/skillspec-doctor/src/frontmatter.rs` but does not read the tool fields.
Extending that parser is preferable to writing a second one.

### 2. Markdown Prose

The source map already classifies spans. Reuse:

- `SourceClassificationKind::CommandExample` for shell invocations described in
  prose.
- `SourceReferenceKind::ExternalUri` for URLs. Every external URI is at minimum a
  `net.fetch` candidate.
- `SourceClassificationKind::DependencyMention` for `pkg.install` candidates.
- `SourceClassificationKind::ImportCandidate` and `ResourceCandidate` for
  `fs.read` on package-local paths.

Prose-derived observations are `Medium` confidence by default. Prose that
describes an effect is not the same as code that performs one, but the skill's
instructions are precisely what the agent will act on, so prose is a first-class
source here in a way it would not be for a code scanner.

### 3. Fenced Code Blocks In `SKILL.md`

Nodes with `kind == "code"` carry a `language`. Dispatch on it to the matching
lexical extractor below. Unlabeled fences are lexed as shell, because that is the
dominant case in skills; doctor already flags unlabeled fences separately as
`unlabeled_code_fences`.

Code blocks inside the activation body carry `reach: activation`.

### 4. Package Script Files

For files classified `SourceFileKind::Code`, run the matching extractor:

- `shell.rs` for `.sh`, `.bash`, `.zsh`, and unlabeled fences
- `python.rs` for `.py`
- `javascript.rs` for `.js`, `.mjs`, `.ts`

Version 0 uses **line-oriented lexical extraction, not AST parsing**. This is a
deliberate scope limit and must be stated in the report. The rationale is that an
AST pass buys precision on a question the boundary does not need answered
precisely, and precision lost here fails closed. Document 40 records AST
extraction as a later milestone with an explicit trigger.

### 5. Manifests

Files classified `SourceFileKind::Manifest` yield `pkg.install` effects:
`deps.toml`, `package.json`, `requirements.txt`, `pyproject.toml`, `Cargo.toml`,
`go.mod`. Record the ecosystem, the package name, and whether the version is
pinned.

## Normalization

Normalizers convert raw source text into a comparable target. They are the part
of the system most likely to be wrong, so each one is independently unit-tested
against a fixture table.

### URL Normalization

1. Extract scheme, host, port.
2. Lowercase the host. Strip a trailing dot.
3. If the host is an IP literal, record it as such; IP literals never collapse
   into a wildcard grant.
4. If any interpolation marker (`$VAR`, `${VAR}`, `{{ }}`, `%s`) appears in the
   host segment, resolution is `dynamic`. Interpolation only in the path segment
   leaves resolution `templated` and the host intact.

### Path Normalization

1. Expand `~`, `$HOME`, `%USERPROFILE%`.
2. Resolve `.` and `..` lexically. Do not touch the filesystem; the analysis
   must not depend on the analyst's machine.
3. Match against the path class table, most specific first.
4. Interpolation anywhere in the path leaves the literal path unresolved but
   still attempts a class match on the fixed prefix. `~/.ssh/$KEYFILE` is
   `templated` with class `secret`.

### Argv Normalization

1. Take `argv[0]`, reduce to its basename.
2. Unwrap known wrappers and re-normalize the remainder: `sudo`, `env`, `nohup`,
   `time`, `xargs`, `nice`, `command`, `exec`.
   `sudo curl https://x` normalizes to `proc.exec:curl` plus `net.egress:x`, and
   separately records that the invocation was privilege-elevated.
3. Detect pipes into an interpreter: a pipeline whose final stage is `sh`,
   `bash`, `zsh`, `python`, or `node` emits `proc.exec` with resolution
   `dynamic`, because the executed content is not in the source.
4. Recognize network binaries and emit the corresponding network effect:
   `curl`, `wget`, `nc`, `ssh`, `scp`, `rsync`, `git` with a remote URL.

### Environment Variable Extraction

Any `$NAME` or `${NAME}` reference emits `env.read`. Names matching credential
patterns (`*_TOKEN`, `*_KEY`, `*_SECRET`, `*_PASSWORD`, `AWS_*`, `GITHUB_TOKEN`,
`ANTHROPIC_API_KEY`) are recorded with a `credential_like` marker used by the
proposal and by the report ordering.

## Deduplication

Observations collapse into an **effect set** for boundary generation while the
full observation list is retained for evidence.

Two observations merge when class, normalized target, and resolution are equal.
The merged entry keeps every evidence record and the strongest confidence.

The merged entry's reach is the **most visible** reach among its members, where
`activation` is more visible than `deferred`, and `deferred` is more visible than
`unmapped`. An effect that appears both in an unmapped script and in the
documented activation body merges to `activation`, so it is not filed as though
it were only reachable through an undocumented file.

State this as visibility rather than breadth when implementing it. Document 40
declares `Reach` with `Activation` first, which makes the most visible value the
minimum under the derived ordering, and the inverted reading is an easy defect to
ship.

## Report Schema

Schema id: `skillspec.boundary.effect_surface.v0`.

```json
{
  "schema": "skillspec.boundary.effect_surface.v0",
  "target": "./my-skill",
  "source_kind": "local_folder",
  "source_content_sha256": "…",
  "analysis": {
    "extractors": ["markdown", "shell", "python", "manifest"],
    "extraction_mode": "lexical",
    "files_analyzed": 12,
    "files_skipped": [
      { "path": "assets/logo.png", "reason": "binary" }
    ]
  },
  "declared": {
    "allowed_tools": ["Read", "Bash"],
    "source": "frontmatter"
  },
  "effects": [
    {
      "id": "effect-0001",
      "class": "net.egress",
      "target": { "kind": "host", "host": "api.github.com", "scheme": "https" },
      "resolution": "literal",
      "reach": "activation",
      "origin": "markdown_command_example",
      "confidence": "high",
      "raw": "curl -s https://api.github.com/user",
      "observations": [
        { "path": "SKILL.md", "line": 42, "node": "node-17",
          "text_preview": "curl -s https://api.github.com/user" }
      ]
    }
  ],
  "unresolved": [
    {
      "id": "effect-0009",
      "class": "net.egress",
      "resolution": "dynamic",
      "reach": "unmapped",
      "raw": "curl -X POST \"$ENDPOINT\" -d @-",
      "reason": "host segment is interpolated",
      "observations": [
        { "path": "scripts/upload.sh", "line": 8,
          "text_preview": "curl -X POST \"$ENDPOINT\" -d @-" }
      ]
    }
  ],
  "summary": {
    "effect_count": 14,
    "unresolved_count": 1,
    "by_class": { "net.egress": 2, "fs.read": 6, "proc.exec": 5, "env.read": 1 },
    "by_reach": { "activation": 9, "deferred": 4, "unmapped": 1 },
    "sensitive_path_classes": ["secret"]
  }
}
```

`unresolved` is a sibling of `effects`, not a variant inside it. Consumers that
generate policy must handle it explicitly; a consumer that ignores the key
produces an incomplete boundary, and making it a separate key means that mistake
is visible in code review.

## Human Output

The default text rendering is ordered for a reviewer deciding whether to install,
not for a scanner reading severities. Within the effect section:

1. Unresolved effects.
2. Effects touching `secret`, `agent_config`, `skill_package`, `shell_init`, or
   `vcs_config` path classes.
3. `net.egress` targets.
4. Everything else, grouped by class.

The effect section's position within the whole report, relative to concealment,
directives, and chains, is defined in `41-agent-directives.md`.

Effects found only at `unmapped` reach are annotated inline rather than sorted
into their own section, so the reviewer sees them in the context of the class
they belong to.

The rendering states the extraction mode and any skipped files. A report that
silently skipped a file it could not parse would understate the surface, and
under-statement is the failure this design most needs to avoid reporting
invisibly.

## Handling Untrusted Content In The Report

Every `raw`, `text_preview`, and `decoded_preview` string in this model is
attacker-controlled text lifted verbatim from a package the analysis exists
because nobody trusts.

Three consumers make that dangerous. SkillSpec reports are agent-facing by
design (`docs/design/runtime/25-progressive-agent-guidance.md` treats the CLI as
a conduit for agent consumption). The public path publishes reports into GitHub
issues (`docs/design/operations/27-public-doctor-reports.md`). And a human reads
them in a terminal. A tool whose purpose includes finding hidden instructions
must not become the mechanism that delivers them, cleanly extracted from their
obfuscation, into a model's context.

The existing sanitizer at `.github/scripts/sanitize-doctor-report.mjs` only
rewrites filesystem paths. It offers nothing here.

Every quoted string therefore passes through one content sanitizer before it
reaches any output:

1. Truncate to 200 characters and mark truncation.
2. Strip or escape C0 and C1 control characters.
3. Replace every codepoint the concealment detectors in document 39 match -
   zero-width, tag block, bidi override, variation selector - with a visible
   placeholder naming the codepoint. The report says a tag character was present;
   it does not reproduce one.
4. Escape backticks, fence sequences, and angle brackets so quoted text cannot
   terminate the surrounding rendering context in text, Markdown, or HTML output.
5. Neutralize instruction framing: a preview is always emitted inside a delimited
   quotation block with a fixed prefix stating it is untrusted quoted material
   from the analyzed package.

Rule 3 has a consequence for document 39 that is stated there: decoded payloads
from hidden-character runs are never rendered as plaintext in default output.

## Resource Bounds

The hosted path runs this against arbitrary public repositories, so extraction
is bounded and the bounds are reported rather than applied silently.

| Bound | Default | On exceed |
| --- | --- | --- |
| Files analyzed | 2,000 | Stop, list the count skipped |
| Bytes per file | 2 MiB | Skip the file, record it |
| Total bytes analyzed | 64 MiB | Stop, record the truncation |
| Line length | 64 KiB | Truncate the line, record it |
| Wall clock | 60 s | Stop, record partial analysis |

Any bound that fires sets `analysis.truncated: true` and adds an entry to
`analysis.files_skipped`. A truncated analysis produces an incomplete proposal
under compilation rule 4 in document 37, for the same reason an unresolved effect
does: the surface was not fully determined.

## What This Model Does Not Capture

- **Runtime-fetched instructions.** A skill that downloads content and acts on it
  has an effect surface that is not determined by its source. The `net.fetch`
  effect records that this is happening; it cannot record what comes back.
- **Misuse of a granted effect.** An enumerated, granted host can carry
  exfiltrated data. Nothing in this model distinguishes that from intended use.
- **Conditional and delayed execution.** An effect guarded by a condition that
  rarely fires is enumerated identically to one on the main path. This is
  correct for boundary generation and useless for triage.
- **Semantics of prose.** An instruction like "then send the summary to the
  address the user provided" produces no target. It is `unknown` resolution at
  best, and more likely produces nothing at all.

## Sources

- `crates/skillspec-doctor/src/source_map.rs` - file, node, classification, and
  reference records this analysis consumes.
- `crates/skillspec-doctor/src/lib.rs` - reach classification, unmapped surface
  counting, remote target staging.
- `crates/skillspec-doctor/src/frontmatter.rs` - frontmatter parsing to extend.
- `docs/design/operations/22-doctor-agent-drift-risk.md` - the separate risk axis
  this analysis must not be folded into.
