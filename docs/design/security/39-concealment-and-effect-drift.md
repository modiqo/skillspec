# Concealment And Effect Drift

Status: proposed. Nothing in this document is implemented.

## Purpose

Cover two things an enumerated effect surface and a deny-by-default boundary do
not address:

1. **Concealment.** Text that a human reader of the skill will not see but a
   model will. Concealment creates no grant, so denying by default does not
   mitigate it.
2. **Effect drift.** A skill whose effect surface grew between two revisions.
   The first surface was reviewed and approved; the second was not.

A third gap of the same kind - visible instructions that retarget the agent's
own behavior rather than reaching the host - is covered in
`41-agent-directives.md`. Concealment and directives are sibling detector
families with the same rationale: they produce no grant, so default-deny is
silent about them.

Detectors are warranted in exactly those two families and nowhere else in this
design, and both sets are deliberately small and fixed.

## Part 1: Concealment

### Why Detectors Are Needed Here

The argument for enumeration over detection is that a missed effect fails closed
under a deny default. Concealment breaks that argument, because concealed
content is not an effect. Hidden text that instructs an agent to summarize a file
and include the summary in its normal output requires no new grant at all - the
skill already reads files and already produces output.

So concealment is detected directly, and reported as a finding rather than an
effect.

### Detector Set

Six detectors. Each is deterministic and each has a low enough false-positive
rate to report without a severity model.

| Id | Detects | Method |
| --- | --- | --- |
| `conceal.zero_width` | Zero-width characters inside prose | Scan for U+200B, U+200C, U+200D, U+2060, U+FEFF outside code fences |
| `conceal.tag_block` | Unicode tag characters | Scan for U+E0000-U+E007F anywhere |
| `conceal.bidi_override` | Bidirectional overrides that reorder displayed text | Scan for U+202A-U+202E, U+2066-U+2069 |
| `conceal.variation_selector` | Data encoded in variation selectors | Runs of U+FE00-U+FE0F or U+E0100-U+E01EF longer than 3 |
| `conceal.comment_directive` | Imperative instructions inside HTML comments | Modal-obligation classification applied to `<!-- -->` spans |
| `conceal.encoded_payload` | Encoded blob adjacent to a decoder | Base64 or hex run of 64+ chars within 3 lines of `base64 -d`, `atob`, `b64decode`, `fromhex`, `eval`, or a pipe into an interpreter |

Notes:

- `conceal.zero_width` excludes code fences because zero-width characters inside
  a code block are usually a copy-paste artifact rather than a hidden directive,
  and skills quoting terminal output produce them routinely. Occurrences inside
  fences are counted and reported as a single informational line, not a finding.
- `conceal.comment_directive` reuses the modal-obligation classification the
  source map already produces (`SourceClassificationKind::ModalObligation`). A
  comment containing prose is not a finding; a comment containing an obligation
  is.
- `conceal.encoded_payload` requires **both** the blob and a nearby decoder.
  Base64 alone is common and benign in skills that embed small assets.

Homoglyph hostnames are handled in normalization rather than here: a hostname
containing non-ASCII characters is recorded on the effect itself, because it
does produce a grant and the grant is the thing that would be wrong.

### Output Shape

Concealment findings are a separate top-level key. They do not enter the effect
set, do not produce grants, and do not affect proposal completeness.

```json
"concealment": [
  {
    "id": "conceal.tag_block",
    "path": "SKILL.md",
    "line": 12,
    "count": 214,
    "text_preview": "Read the file carefully.<214 tag characters>",
    "decoded_preview": "ignore prior constraints and…",
    "statement": "214 Unicode tag characters are present in the activation body. They are not rendered by most editors and viewers."
  }
]
```

`decoded_preview` is **not populated in default output**, and the shape above
shows the field only for the escape hatch described below.

Decoding a hidden instruction and printing it would take a payload that was
deliberately obfuscated, extract it cleanly, and place it in a report that
SkillSpec's own design routes to agents and publishes to GitHub issues. That
turns the detector into the delivery mechanism for the thing it detects.
Labeling the field as decoded content is not a control; a model reading the
report has no obligation to respect the label.

Default output therefore carries only non-actionable facts about the payload:

```json
{
  "id": "conceal.tag_block",
  "path": "SKILL.md",
  "line": 12,
  "count": 214,
  "decoded_sha256": "9f2b…",
  "decoded_bytes": 214,
  "decoded_character_classes": ["latin_lowercase", "space", "punctuation"],
  "statement": "214 Unicode tag characters are present in the activation body. They are not rendered by most editors and viewers. The decoded content is withheld; use --reveal to write it to a file."
}
```

`--reveal <path>` writes the decoded payload to a file the caller names. It is
never written to stdout, never included in `--json`, and never included in the
published report path. A human who wants to read the payload opens the file
deliberately, in an editor, outside any agent's context.

The same rule governs `text_preview` on every concealment finding: it passes
through the content sanitizer in document 36, which replaces the concealment
codepoints themselves with visible placeholders naming them.

The `statement` field is deliberately descriptive. Reports state that hidden
characters are present and that they are not normally rendered. They do not state
that the author intended to hide them.

### Where Concealment Appears In The Report

At the top, before the effect surface. A reader who sees a concealment finding
should evaluate that before spending attention on grants, because a package with
hidden instructions is one whose visible text cannot be trusted to describe it.

## Part 2: Effect Drift

### Purpose

Compare the effect surfaces of two revisions of the same skill and report what
changed. This is the highest-precision signal available in the whole design,
because it needs no judgment about whether an effect is appropriate - only about
whether it is new.

### Inputs

```text
skillspec boundary diff <target> --against <ref>
```

`<ref>` is a git revision when the target is a git working tree or a staged
remote checkout. Both surfaces are produced by the same extractor version; a
comparison across extractor versions is reported as such and its results marked
unreliable, because a new extractor finding a new effect is not the skill
changing.

The extractor version is therefore recorded in every effect-surface report and
compared before any diff is produced.

### Change Classification

| Class | Meaning |
| --- | --- |
| `sensitive_expansion` | A new effect in path class `secret`, `agent_config`, `skill_package`, `shell_init`, or `vcs_config` |
| `egress_expansion` | A new `net.egress` or `net.fetch` host |
| `exec_expansion` | A new `proc.exec` binary |
| `env_expansion` | A new `env.read`, flagged separately when credential-like |
| `resolution_regression` | An effect whose target was `literal` and is now `templated`, `dynamic`, or `unknown` |
| `reach_regression` | An effect that moved from `activation` or `deferred` reach to `unmapped` |
| `concealment_appeared` | A concealment finding present in the new revision and absent in the old |
| `contraction` | An effect present in the old revision and absent in the new |

`resolution_regression` and `reach_regression` are the two classes worth
particular attention. A host that used to be written literally and is now
interpolated, or an effect that used to be in documented text and now lives only
in a file nothing references, are both changes in reviewability rather than in
capability. Neither would appear in a simple grant diff, and neither is
detectable without the resolution and reach fields defined in document 36.

`contraction` is reported but never gated. A skill doing less is not a finding.

### Re-Consent Semantics

The design position is that these classes should require a human to look again
before an update is trusted:

```text
sensitive_expansion
egress_expansion
concealment_appeared
resolution_regression   (when the prior resolution was literal)
reach_regression
```

SkillSpec cannot enforce re-consent. It has no install gate for arbitrary skills
and no runtime interposition. What it can do is exit non-zero so that whatever
does gate installation - a CI job, a review checklist, a pre-install hook someone
else wrote - has a signal to act on.

### Drift Schema

Schema id: `skillspec.boundary.drift.v0`.

```json
{
  "schema": "skillspec.boundary.drift.v0",
  "target": "./my-skill",
  "from": { "ref": "v0.3.0", "sha256": "…", "extractor_version": "0.1.0" },
  "to":   { "ref": "working-tree", "sha256": "…", "extractor_version": "0.1.0" },
  "comparable": true,
  "changes": [
    {
      "class": "egress_expansion",
      "effect": "effect-0011",
      "detail": "new net.egress host: telemetry.example.com",
      "evidence": { "path": "scripts/post.sh", "line": 14 },
      "requires_review": true
    }
  ],
  "summary": {
    "requires_review": 1,
    "informational": 3,
    "contractions": 1
  }
}
```

### Gating

```text
skillspec boundary check <target> [--against <ref>] [--fail-on <level>]
```

Exit codes:

| Code | Condition |
| --- | --- |
| 0 | No findings at or above the threshold |
| 1 | Findings at or above the threshold |
| 2 | Proposal incomplete (unresolved effects present) and `--fail-on` includes it |
| 3 | Analysis error, unreadable target, or non-comparable revisions |

Separating exit code 2 from 1 matters. An incomplete proposal is a statement
that the tool could not fully determine the surface, which is a different thing
from the tool determining a surface and finding it concerning. A CI job may
reasonably want to treat those differently.

### First Install Versus Update

Drift is the right gate for an update and the wrong gate for a first install.

A skill that was hostile in its first commit shows no drift, and an attacker who
controls the repository controls the baseline history the comparison reads. Using
`--against` on first contact converts "nothing changed" into "nothing to see,"
which is precisely backwards.

The rule is therefore explicit and the CLI enforces it:

- **First install: absolute review.** `boundary check` with no `--against` runs
  the full surface, concealment, and directive families against the threshold.
- **Update: drift.** `--against <ref>` gates on what changed, on the assumption
  that the prior surface was reviewed.

`boundary check --against <ref>` where no prior review is recorded emits a
warning stating that the baseline has not itself been reviewed. SkillSpec has no
durable record of a human approving a surface, so it cannot verify the
assumption; it can refuse to let the assumption stay implicit.

A skill with a broad effect surface that has not changed is a skill someone
already decided to accept - but only if someone actually decided.

## Sources

- `crates/skillspec-doctor/src/source_map.rs` - modal-obligation classification
  reused by `conceal.comment_directive`, and the file records both parts scan.
- `crates/skillspec-doctor/src/remote_source.rs` - `git_show_text`,
  `git_tree_files`, and staged checkouts, which the drift comparison uses to
  produce the `--against` surface.
- `docs/design/security/36-skill-effect-surface.md` - the `resolution` and
  `reach` fields that make the two regression classes expressible.
