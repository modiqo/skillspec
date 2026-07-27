# Boundary Proposal Compiler

Status: proposed. Nothing in this document is implemented. It consumes the
effect model defined in `36-skill-effect-surface.md`.

## Purpose

Turn an effect surface into the smallest permission set that still lets the skill
work, and emit that permission set in a format the reader's existing harness can
enforce.

```text
effect surface  ->  grant set  ->  emitted policy artifact
```

The emitted artifact is the product. A reader who takes the artifact, pastes it
into their own configuration, and never installs SkillSpec has received the whole
value of this feature. That is intentional.

## Boundary Semantics Are Inherited, Not Invented

`docs/design/core/09-phase-tool-boundaries.md` already defines
`tool_boundary` as a contract label set, not executable policy code:

```text
`allow`, `forbid`, and `permission_required_for` are string lists. They are
contract labels, not executable policy code. Harnesses can map them to concrete
tool ids, adapter names, product permissions, or approval prompts.
```

That restriction holds here without exception. A boundary proposal is a
recommendation rendered in someone else's policy grammar. SkillSpec does not
intercept a call, does not sandbox a process, and does not guarantee that an
emitted policy is honored. Any wording in the CLI, the docs, or the report that
implies enforcement by SkillSpec is a defect.

The model type already exists at
`crates/skillspec-core/src/spec/model.rs` as `ToolBoundary` with fields
`default`, `allow`, `forbid`, and `permission_required_for`, and
`ToolBoundaryDefault` with variants `Allow` and `Deny`. The compiler targets that
type for its native output and does not extend it.

## Grant Model

A grant is a normalized string label derived from one or more effects.

```text
<class>:<target>
```

Examples:

```text
net.egress:api.github.com
net.fetch:raw.githubusercontent.com
fs.read:workspace
fs.write:workspace
fs.read:secret
proc.exec:git
proc.exec:curl
env.read:GITHUB_TOKEN
tool.invoke:Read
pkg.install:npm/prettier
```

Grants aggregate at the level the effect class defines:

| Class | Aggregation level |
| --- | --- |
| `net.egress`, `net.fetch` | Exact host. Never a wildcard, never a parent domain. |
| `fs.read`, `fs.write` | Path class, not literal path. |
| `proc.exec` | Binary basename. |
| `env.read` | Exact variable name. |
| `tool.invoke` | Exact tool identifier. |
| `agent.config` | Path class. |
| `pkg.install` | `ecosystem/package`. |

Host grants never widen to a parent domain. `api.github.com` and
`raw.githubusercontent.com` are two grants, not one `*.github.com` grant.
Widening is the single most common way a least-privilege policy silently becomes
a permissive one, and the loss is a slightly longer list.

Path grants deliberately do widen, to the path class. A literal-path allow-list
for filesystem access is unreadable and breaks on the first legitimate variation.
The path class table in document 36 is the unit that matters.

## Compilation Rules

### Rule 1: Default Deny

Every proposal sets `default: deny`. This is what makes an under-enumerated
effect surface fail closed, and it is the property the whole design rests on. A
proposal that cannot be expressed with a deny default in a given emission target
is not emitted for that target; see the emission table below.

### Rule 2: Resolved Effects Become Grants

Each entry in the deduplicated effect set with resolution `literal` or
`templated` becomes one grant.

`templated` effects grant on their resolved portion. `https://api.github.com/repos/$OWNER/$REPO`
grants `net.egress:api.github.com`, because the host is fixed and only the path
varies.

### Rule 3: Sensitive Path Classes Are Never Silently Granted

Effects whose path class is `secret`, `agent_config`, `skill_package`,
`shell_init`, or `vcs_config` do not become an `allow` entry. They are placed in
`permission_required_for` and listed in the proposal's review block.

The reasoning is that these five classes are the ones where a granted effect
would be indistinguishable from the attack. A skill that legitimately reads
`~/.aws/credentials` exists; the proposal's job is to make sure a human said so
out loud rather than inheriting it from a generated file.

### Rule 4: Unresolved Effects Block Completeness

If the effect surface contains any `unresolved` entries, the proposal is marked
`complete: false` and carries an `unresolved` block naming them.

An incomplete proposal is still emitted. It is not a failure state; a
deny-by-default boundary with an unresolved dynamic egress is more useful than
no boundary. What must not happen is emitting it as though it were complete.
The `boundary check` command in document 40 treats an incomplete proposal as a
distinct exit condition from a complete one.

### Rule 5: Declared But Unobserved Grants Are Reported, Not Emitted

When frontmatter declares `allowed-tools` entries with no matching observed
effect, list them under `declared_unused`. Do not carry them into the proposal.
This is the least-privilege narrowing that gives the feature its value, and it is
also the change most likely to break a skill, so it is called out explicitly in
the human rendering rather than applied quietly.

### Rule 6: Observed But Undeclared Effects Are Reported

The inverse case - an effect with no matching entry in a declared
`allowed-tools`, or an effect matching an existing `tool_boundary.forbid` - is
listed under `undeclared` or `contradicts_contract`. When the skill has no
declarations at all, which is the common case, both lists are empty and the
comparison section is omitted from the human rendering.

### Rule 7: Existing Contract Wins

If the package has a valid `skill.spec.yml` with a `tool_boundary`, the proposal
is rendered as a **diff against it**, not as a replacement. An author who already
declared a boundary gets to see what the effect surface says they missed or
over-granted; they do not get their file rewritten.

## Emission Targets

The proposal is rendered into whichever policy grammar the reader uses.

| Target id | Artifact | Deny default expressible |
| --- | --- | --- |
| `skillspec` | `tool_boundary` block for `skill.spec.yml` | yes |
| `claude-frontmatter` | `allowed-tools` list for `SKILL.md` frontmatter | no, allow-list only |
| `claude-settings` | permission block for a settings file | to be verified |
| `egress-allowlist` | plain host list for a proxy or network policy | yes, by construction |
| `json` | the raw proposal, for another tool to consume | not applicable |

Two implementation requirements follow from that table.

**The exact grammar of every non-SkillSpec target must be verified against the
harness's current documentation at implementation time, and re-verified before
each release.** These formats belong to other projects and change without regard
for this repo. Emitters must be small, isolated, and covered by a fixture test
whose expected output a maintainer can diff against upstream docs. Document 40
lists this as a standing task, not a one-time step.

**An emitter for a target that cannot express deny-by-default must say so in its
output.** `claude-frontmatter` produces an allow-list with no deny semantics; the
emitted snippet carries a comment stating that it narrows the tool surface but
does not deny anything the harness permits by default. Emitting a
weaker-than-intended policy without saying so would misrepresent the protection
the reader is getting.

### Example: `skillspec` Target

```yaml
tool_boundary:
  default: deny
  allow:
    - "net.egress:api.github.com"
    - "proc.exec:git"
    - "proc.exec:jq"
    - "fs.read:workspace"
    - "fs.write:workspace"
    - "env.read:GITHUB_TOKEN"
  permission_required_for:
    - "fs.read:secret"
```

### Example: `egress-allowlist` Target

```text
# skillspec boundary emit --format egress-allowlist
# Proposal is incomplete: 1 unresolved egress effect. See report.
api.github.com
```

An incomplete proposal emits the completeness warning as a comment in every
target that supports comments, and in the JSON target as a field.

## Proposal Schema

Schema id: `skillspec.boundary.proposal.v0`.

```json
{
  "schema": "skillspec.boundary.proposal.v0",
  "target": "./my-skill",
  "source_content_sha256": "…",
  "effect_surface": "skillspec.boundary.effect_surface.v0",
  "complete": false,
  "boundary": {
    "default": "deny",
    "allow": ["net.egress:api.github.com", "proc.exec:git"],
    "permission_required_for": ["fs.read:secret"],
    "forbid": []
  },
  "review_required": [
    {
      "grant": "fs.read:secret",
      "reason": "sensitive path class",
      "effects": ["effect-0004"]
    }
  ],
  "unresolved": [
    {
      "effect": "effect-0009",
      "class": "net.egress",
      "reason": "host segment is interpolated",
      "consequence": "a deny-default boundary will refuse this call at runtime"
    }
  ],
  "comparison": {
    "declared_source": "frontmatter",
    "declared_unused": ["WebSearch"],
    "undeclared": [],
    "contradicts_contract": []
  }
}
```

The `consequence` field on each unresolved entry is required. It states, in
plain language, what will happen at runtime if the reader applies the proposal
as emitted. For a dynamic egress under a deny default, the honest answer is that
the call will be refused, which may be the desired outcome or may break the
skill. The reader decides; the report must not decide for them.

## Human Output

The rendering leads with the decision the reader has to make, not with the
policy body:

```text
SkillSpec Boundary Proposal
===========================
Target: ./my-skill                     Effects: 14 (1 unresolved)

Needs your decision
- fs.read:secret        reads ~/.aws/credentials      SKILL.md:88
- net.egress (dynamic)  POST to "$ENDPOINT"           scripts/upload.sh:8
                        under a deny default this call will be refused

Proposed grants
  net.egress   api.github.com
  proc.exec    git, jq
  fs.read      workspace
  fs.write     workspace
  env.read     GITHUB_TOKEN

Narrowed from declared
- WebSearch was declared in allowed-tools but no use was found

This proposal is incomplete. One effect could not be resolved to a target.
SkillSpec does not enforce this boundary; your harness does.
```

The closing line is not decoration. It is the accurate description of what the
tool did, and it appears in every rendering.

## Open Questions

1. **Grant label grammar.** `<class>:<target>` is readable and diffable, but the
   existing `tool_boundary` strings in examples are free-form labels like
   `rote browser` or `no raw shell`. A generated boundary using a structured
   namespace next to hand-written free-form labels in the same list may be
   confusing. Options: keep the namespace and document it, prefix generated
   entries, or emit generated grants into a separate block. Resolve before the
   `skillspec` emitter ships.
2. **Path class granularity for `workspace`.** A single `fs.write:workspace`
   grant covers writing anywhere in the project, which is broad. Whether to
   subdivide by top-level directory needs a real-skill sample before deciding.
3. **Whether `declared_unused` should be applied by default.** Narrowing to
   observed use is the point, but silently dropping a declared tool that a skill
   uses only on a rare path would break it. Current position: report, do not
   apply, and revisit once validation (document 38) can tell the difference.

## Sources

- `crates/skillspec-core/src/spec/model.rs` - `ToolBoundary`,
  `ToolBoundaryDefault`.
- `crates/skillspec-runtime/src/act.rs` - how an effective boundary is currently
  merged and rendered.
- `docs/design/core/09-phase-tool-boundaries.md` - boundary semantics and the
  contract-label restriction inherited here.
