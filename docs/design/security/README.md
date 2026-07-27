# Skill Effect Surface And Boundary Compilation

Status: implemented, in `crates/skillspec-boundary`. These documents describe
the design; for how to use the shipped commands, see the user guide at
[`docs/boundary-guide.md`](../../boundary-guide.md).

Milestones M0–M5, M3, and M8 are built: effect-surface enumeration (shell,
Python, Deno/Bun/Node TypeScript), the concealment and directive detector
families, the proposal compiler and emitters, drift and CI gating, and the
guard hook that enforces a reviewed policy. Not built: the flow graph
(document 42, M7), validation level V2, and the workspace-shape aggregation
(investigation I4). Where an emitter was cancelled by its verification step
(`claude-frontmatter`), document 37 records why.

This folder describes an analysis surface that answers one question about an
agent skill:

```text
If an agent executed this skill, what could it reach, and what is the smallest
permission set that still lets the skill work?
```

It is deliberately not a malware scanner. The reasoning behind that choice is in
[Why Enumeration Instead Of Detection](#why-enumeration-instead-of-detection)
below and in `36-skill-effect-surface.md`.

## Document Set

| Order | Doc | Purpose |
| --- | --- | --- |
| 36 | [Skill Effect Surface](36-skill-effect-surface.md) | The effect model, effect classes, target resolution, reach, extraction sources, and the effect-surface report schema. |
| 37 | [Boundary Proposal Compiler](37-boundary-proposal-compiler.md) | How an effect surface becomes a least-privilege boundary proposal, and how that proposal is emitted into harness-native permission formats. |
| 38 | [Boundary Validation](38-boundary-validation.md) | How a candidate boundary is checked for grant coverage, and the narrow conditions under which behavioral validation is possible. |
| 39 | [Concealment And Effect Drift](39-concealment-and-effect-drift.md) | The concealment detector set that default-deny cannot cover, and version-to-version effect drift with re-consent semantics. |
| 40 | [Implementation Plan](40-implementation-plan.md) | Crate layout, module-by-module build order, types, CLI wiring, fixtures, tests, milestones, and acceptance criteria. |
| 41 | [Agent Directives](41-agent-directives.md) | The second detector family: visible instructions that retarget the agent's behavior rather than reaching the host. Also fixes the whole-report ordering. |
| 42 | [Effect Flow Graph](42-effect-flow-graph.md) | Relating effects to each other, so a report can say a network call carries credential material. Explanation only; never alters the proposal. |
| 43 | [Boundary Guard Hook](43-boundary-guard-hook.md) | The enforcement point: a managed pre-tool hook that applies a reviewed policy to skills the user already has, with no change to those skills. |

Read 36 first. Documents 37, 38, 39, 41, and 42 all consume the effect model it
defines. Document 43 is where the output is finally enforced, and document 40 is
the build order for all of them.

## The Three Steps A User Takes

The design is only useful if each step is worth taking on its own, because most
users will stop after the first.

```text
1. report    skillspec boundary <skill>          see what it can reach
2. policy    skillspec boundary emit <skill>     get a least-privilege policy
3. guard     skillspec boundary guard install    have it enforced, in observe
                                                 mode, against skills you
                                                 already have
```

Nothing in that sequence asks the user to change a skill, write a
`skill.spec.yml`, compile anything, or change how they work. Step 3 defaults to
observing rather than blocking, because a control that gets uninstalled protects
nothing.

## Three Families Of Finding

The reports produced by this work carry three kinds of finding, and the
difference between them is the design's central argument.

| Family | What it is | Why default-deny does or does not cover it |
| --- | --- | --- |
| **Effects** (36) | What the skill reaches on the host | Covered. A missed effect has no grant, and a deny default refuses it. |
| **Concealment** (39) | Text a reader will not see but a model will | Not covered. Hidden text creates no grant. Detectors required. |
| **Directives** (41) | Instructions retargeting the agent's own behavior | Not covered. A directive can operate entirely inside permissions already granted. Detectors required. |

Chains (42) are not a fourth family. They are relationships among effects, and
they change how a report reads, never what it grants.

## Why This Exists

`skillspec doctor` measures follow-through risk: whether an agent reading a
skill as prose is likely to skip, reorder, improvise, or finish without proof.
That is a reliability question.

It does not measure what a skill can reach if an agent obeys it. A skill can be
well-structured, compactly written, and score low on drift risk while still
reading credential paths and sending data off-host. Those are independent axes,
and they must stay independent in the reporting model. Folding an effect finding
into the drift score would let good structure mask a broad effect surface.

## Why Enumeration Instead Of Detection

The existing tools in this space solve a classification problem: is this
particular pattern malicious? That question cannot be answered statically,
because the same instruction is a feature in one skill and an attack in another.
Tools compensate with large rule sets, taint analysis, LLM stages, and multi-model
juries, and they publish precision and recall against labeled corpora.

Enumeration asks a different question: what does this skill touch, all of it,
including the ordinary parts? That question has no intent component. A target is
present in the source or it is not.

The consequential difference is the failure mode:

- A classifier that misses an effect **fails open**. The effect is not reported,
  and the skill still performs it at runtime.
- An enumerator that misses an effect **fails closed** - *if and only if* the
  policy is enforced by a substrate with deny-by-default semantics. The effect
  that was never enumerated is the effect that has no grant, and a deny-default
  enforcer refuses it.

That condition is not decoration, and it is the single most important limit in
this design. Fail-closed is a property of **the enforcement substrate**, not of
the analysis. Where a substrate can only express an allow-list over a permissive
default - which is the case for a `SKILL.md` `allowed-tools` list - a missed
effect is simply permitted, and the argument above provides nothing.

| Enforcement substrate | Deny default | Fail-closed holds |
| --- | --- | --- |
| SkillSpec guard hook (document 43) | yes | yes |
| Container / proxy egress policy | yes | yes |
| `tool_boundary` in a contract-aware harness | yes | yes, if the harness honors it |
| `allowed-tools` frontmatter | no | **no** |
| Harness settings permission block | depends on the harness | verify per harness |

Document 43 exists because of this table. An analysis that can only emit into
allow-list substrates would have a sound argument and no way to realize it, so
the design ships an enforcement point of its own for skills that are already
installed and unmodified.

This does not make enumeration strictly better. It makes its residual risk
different, and in a direction that is easier to reason about. It also means the
evasion techniques that defeat classifiers - encoding, homoglyph substitution,
cross-file splitting, conditional triggers, delayed execution - have much less
purchase, because none of them create a grant.

What enumeration cannot do is bound the misuse of an effect that was correctly
enumerated and legitimately granted. A skill that needs `api.github.com` can
exfiltrate to `api.github.com`. That gap is real, is stated in every document
here, and is not closed by this design.

The argument also stops at the boundary of the effect model itself. Concealment
and directives are outside it, which is why they get detectors, and why the
detector sets are small and fixed rather than growing toward a general
classifier.

## Non-Goals

- **Not a security boundary.** SkillSpec does not sandbox, intercept, or enforce.
  A boundary proposal is a policy artifact that some other component - a harness
  permission system, a container network policy, an approval prompt - enforces.
  This restriction is inherited from `docs/design/core/09-phase-tool-boundaries.md`
  and is not relaxed here.
- **No intent claims.** Reports state what a skill can reach and where the
  evidence is. They do not state that a skill is malicious, hostile, or safe.
- **No model calls in v0.** Extraction, normalization, and proposal generation
  are deterministic. An optional semantic layer may be considered later, and if
  added it may only raise the visibility of a finding, never clear one.
- **Not a replacement for scanners.** Dedicated scanners cover vulnerability
  patterns, known CVEs, and signature matching. This design covers none of that
  and is complementary to it.
- **Not adoption-gated.** The analysis must run against an unmodified prose
  `SKILL.md` with no `skill.spec.yml`, no compilation, and no install. Where a
  contract does exist, the analysis uses it; it never requires one.
- **No identity or reputation signals.** Typosquatting, name similarity to
  popular skills, publisher reputation, and registry ranking are real parts of
  the threat model and are deliberately excluded. None of them is derivable from
  package contents; they need a registry corpus this project does not have.
  Half-implementing them from a hardcoded list of popular names would produce
  confident-looking findings from insufficient data.
- **No taint semantics.** Document 42 relates effects to one another with
  explicit per-edge confidence. That is reachability over a graph, not taint
  analysis, and the reports say so.

## Terms

`Effect` means a host-visible action a skill could cause if an agent executed
its instructions: a network request, a file read, a process invocation, an
environment variable read, a tool call, a package install.

`Effect surface` means the complete set of effects enumerated from a skill
package. It is distinct from doctor's `activation surface`, which measures how
much text loads into context at activation time.

`Effect observation` means one evidence-backed record of a single effect,
carrying its source file, line, raw text, and reach.

`Reach` means where in the package an effect was found: `activation` for the
`SKILL.md` body, `deferred` for a file the Markdown references, `unmapped` for a
package file nothing in the Markdown reaches.

`Target resolution` means how completely an effect's target could be determined:
`literal`, `templated`, `dynamic`, or `unknown`.

`Boundary proposal` means the least-privilege permission set derived from an
effect surface.

`Effect drift` means the difference between the effect surfaces of two revisions
of the same skill.

`Directive` means an instruction addressed to the agent's own behavior rather
than to the host: withholding information from the user, skipping a
confirmation, overriding earlier guidance. Directives produce no effects and no
grants.

`Chain` means a path from a source-class effect to a sink-class effect over the
flow graph in document 42, carrying the confidence of its weakest edge.

This vocabulary deliberately avoids the word `capability`, which in SkillSpec
already means a local bootstrap seed under `~/.skillspec/capabilities/` as
described in `docs/design/runtime/15-capability-bootstrap.md`.
