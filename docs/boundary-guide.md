# Analyzing and constraining agent skills with `skillspec boundary`

`skillspec boundary` answers one question about a skill:

> If an agent executed this skill, what could it reach — and what is the
> smallest permission set that still lets it work?

It works on a skill you have locally or a public git skill URL, needs no
`skill.spec.yml` and no changes to the skill, and never executes anything in the
package. You can use every part of it independently.

There are three things you can do, in increasing order of commitment:

1. **Assess** — see what a skill can reach.
2. **Create a policy** — compile that into a least-privilege permission set.
3. **Enforce** — apply a reviewed policy to the skills you already run.

Nothing forces you past step 1.

There is also a **gate** you can put in front of an install, so a skill is
assessed before it lands on disk — see [Gate an install](#gate-an-install) — and
`skillspec pull`/`update`, which make that assessment part of the install verb
itself across every harness — see [Install through skillspec](#install-through-skillspec-pull-and-update).

## Install and use it on its own

`skillspec boundary` ships inside the `skillspec` binary, but it is a standalone
tool: it does not require you to adopt SkillSpec's authoring, router, or spec
workflow, does not need a `skill.spec.yml`, and does not change or install
anything into your skills. If all you want is to analyze and constrain other
people's skills, install the binary and use `boundary` — nothing else.

```bash
# one-line installer
curl -fsSL https://raw.githubusercontent.com/modiqo/skillspec/main/install.sh | sh
# or, with Rust installed
cargo install skillspec

skillspec boundary map https://github.com/owner/skills   # orient
skillspec boundary https://github.com/owner/skills        # assess
```

The rest of SkillSpec (import, compile, run, router) is entirely optional and
independent of `boundary`.

---

## 1. Assess

### See a skill's effect surface

```bash
skillspec boundary ./my-skill
skillspec boundary https://github.com/owner/repo/tree/main/skills/my-skill
skillspec boundary https://gitlab.com/group/repo/-/tree/main/skills/my-skill
skillspec boundary https://bitbucket.org/team/repo/src/main/skills/my-skill
skillspec boundary https://git.example.com/team/skill-repo.git
```

A remote skill can live on any public git host — GitHub, GitLab, Bitbucket, or
self-hosted — using that host's folder-URL convention, a plain repo URL, or a
direct `.git` clone URL. The target is staged into a temporary checkout and
removed afterward. The
report leads with a plain-English consequence, then the evidence:

```text
SkillSpec Boundary
==================
Target: ./my-skill        Effects: 6

If executed, this skill can read ~/.aws/credentials and GITHUB_TOKEN and
send data to exfil.example.net.

Effects
  Needs a decision before this is granted:
  - fs.read:secret           ~/.aws/credentials     scripts/collect.sh:3
  env.read       GITHUB_TOKEN
  net.egress     exfil.example.net
  proc.exec      curl

Static analysis: nothing in this package was executed. SkillSpec does not
enforce a boundary; a harness, hook, or network policy does.
```

The report also surfaces two things a permission boundary cannot catch, when
present:

- **Concealment** — hidden Unicode, HTML-comment instructions, encoded blobs.
  The decoded payload is never printed; to read it, `skillspec boundary
  ./my-skill --reveal payload.txt` writes it to a file you open deliberately.
- **Directives** — visible instructions that retarget the agent ("do not tell
  the user", "ignore previous instructions", "skip the confirmation").

Add `--json` for the machine-readable surface.

### Map the shape first

Before analyzing, see how a folder of skills is put together:

```bash
skillspec boundary map ./skills-repo
```

```text
./skills-repo   (7 skills · 3 resources · 1 orphans)
├── [entry] README.md → indexes 7 skill(s)
├── prelude
│   ├── → references: coding-standards
│   └── prelude.ts
├── effect-service-design
│   ├── references/AUDIT.md
│   └── (orphan) agents/openai.yaml
└── ...

Components: 1 connected, 5 independent — 6 analysis path(s).
```

The map reads the folder's structure without analyzing effects: which skills
are present, which files each references (**resources**), which files nothing
references (**orphans** — where a payload or a directive can hide, since a
reader following the `SKILL.md` never sees them), and how the skills connect —
both skills that reference each other and **entry documents** (a root `README`
that indexes the collection). References are resolved relative to the linking
skill and, when that fails, relative to the repository root, so a
`skills/other/SKILL.md` link written from the root resolves. It's a fast
orientation step to run first, and `--json` gives the graph for tooling.

The map is the **Structure** half of what `gate` shows; the next command is the
**Security analysis** half.

### Rank the risk

```bash
skillspec boundary assess ./skills-repo
skillspec boundary assess https://github.com/owner/skills --json
```

`assess` ranks what each skill could reach by severity, so you see the worst
first and can decide:

```text
Risk across 18 skills — 0 critical · 0 high · 5 medium · 11 low · 2 clean
5 skill(s) reach beyond their own directory — review before installing
├── MEDIUM  plugins/pack/skills/main
│   └── [MEDIUM] reads outside its directory → ~/.config/agent  ·  can read files beyond the skill folder
│       └── https://github.com/owner/skills/blob/HEAD/plugins/pack/skills/main/SKILL.md#L12
└── ...

Cleared — no review needed
├── 11 clean · reach nothing outside their own directory
└── 2 low · touch only their own files
    ├── plugins/pack/skills/pdf
    └── plugins/pack/skills/docx
```

Two ideas set the ranking:

- **Scope.** A skill that reads and writes only inside its own directory is doing
  its job — low risk however much it touches. What earns review is *reaching
  outside* it: a home or absolute path, an `..` escape, the network, another
  skill's files, the agent's own config. Skills that stay in scope are named
  under **Cleared** so their absence from the review list reads as "checked and
  fine", not "unchecked".
- **Consequence.** Reading a credential outranks reading the skill's own file; a
  hidden instruction outranks a visible one; a secret read *plus* a way off the
  machine is an exfiltration path and outranks either alone. Each finding states
  how bad it is (`CRITICAL`/`HIGH`/`MEDIUM`/`LOW`, colored red/red/orange/blue in
  a terminal), what it reaches, what happens if it runs, and a clickable link to
  the exact line.

**Documentation examples are qualified, not counted as live effects.** A secret
read or a `curl` shown in a fenced code block of a *referenced Markdown doc* (a
mocking guide, a how-to) is illustrative, not something the skill executes —
`assess` downgrades it to `LOW`, tags it *"shown as an example in documentation —
not an executed effect"*, and does not elevate it to a critical exfiltration
path. It is still listed (an "example" that reaches a live host is exactly what
the tag invites checking); a real shipped script keeps full severity. `--json`
emits the per-skill severity, every finding, and the tally
(`skillspec.boundary.security.v0`).

Every read-only command works on a remote URL too — the whole collection or one
skill:

```bash
skillspec boundary map https://github.com/owner/skills
skillspec boundary https://github.com/owner/skills
skillspec boundary https://github.com/owner/skills/tree/main/skills/deploy
skillspec boundary emit https://github.com/owner/skills/tree/main/skills/deploy
```

The commands that manage or read local state — `guard`, `diff`/`check --against`
(which needs git history), and `--reveal` — are local only.

### A folder with many skills

Point `boundary` at a repository of skills — a multi-skill workspace or a
plugin — and it analyzes **each skill on its own**, never flattening them into
one synthetic surface:

```text
SkillSpec Boundary — Workspace
==============================
Target: ./skills-repo        Skills: 18

! skills/claude-api            7 directive(s); egress → api.anthropic.com
! skills/docx                  3 concealment
  skills/pdf                   clean
  ...

7 of 18 skills warrant a closer look. Run `skillspec boundary <skill-folder>`
on one for its full report.
```

Referenced resources and bundled scripts inside a skill are part of that skill's
own surface — including files nothing in the `SKILL.md` reaches, which are
reported at `unmapped` reach.

`emit`, `check`, `diff`, and `guard add` operate on a single skill. Pointed at a
folder of many, they tell you to name a specific skill folder rather than
flatten it.

### Gate an install

A skill or plugin ships in a git repository and is copied out of it on install,
so the repository can be assessed *before* anything lands on disk. `gate` maps
the target, reports what any skill could reach if executed, and — when there are
findings — asks you to confirm before running the real install command:

```bash
skillspec boundary gate https://github.com/owner/skills \
  --then 'claude plugin marketplace add owner/skills && claude plugin install pack@skills'
```

```text
Structure — what the package contains
─────────────────────────────────────
https://github.com/owner/skills   (18 skills · 7 resources · 1 orphans)
├── plugins/onboard/skills/setup
├── plugins/pack/skills/main
│   └── (orphan) references/flow.md
└── ...

Security analysis — what it could reach, ranked by risk
───────────────────────────────────────────────────────
Risk across 18 skills — 1 critical · 0 high · 2 medium · 11 low · 4 clean
2 skill(s) reach beyond their own directory — review before installing
├── CRITICAL  plugins/pack/skills/main
│   ├── [CRITICAL] sends data to the network → exfil.example.net  ·  credentials could be read and sent off the machine
│   │   └── https://github.com/owner/skills/blob/HEAD/plugins/pack/skills/main/report.py#L7
│   └── [HIGH] reads credentials → ~/.aws/credentials  ·  can read your saved credentials
│       └── https://github.com/owner/skills/blob/HEAD/plugins/pack/skills/main/report.py#L5
└── MEDIUM  plugins/onboard/skills/setup
    └── [MEDIUM] reads outside its directory → ~/.config/agent  ·  can read files beyond the skill folder
        └── https://github.com/owner/skills/blob/HEAD/plugins/onboard/skills/setup/SKILL.md#L117

Cleared — no review needed
├── 11 clean · reach nothing outside their own directory
└── 4 low · touch only their own files
    ├── pdf
    └── docx

Proceed with install? [y/N]
```

The report is in two clearly headed sections — **Structure** (the map, what the
package contains) and **Security analysis** (the ranked risk). While the
repository is cloned and scanned, a dot-matrix spinner runs on stderr; it clears
itself when the report is ready and is silent when output is not a terminal.
Severities are colored the way security tools do — critical bold red, high red,
medium orange, low blue — and color is dropped entirely when stdout is not a
terminal or `NO_COLOR` is set, so the report reads identically in a pipe or a
log. Skills needing no review are grouped under **Cleared**: clean ones (reach
nothing outside their directory) are counted, and the few low-risk ones (touch
only their own files) are named.

The findings are a **tree** — the same shape `boundary map` uses — so it never
fractures the way a bordered table does when a line runs long. Each finding sits
under its skill, and its **evidence link is a leaf directly beneath it**, alone
on its own line: clickable (an OSC 8 terminal hyperlink for a remote blob URL at
the exact line, GitLab and Bitbucket shapes handled too; an openable `path:line`
for a local target), and never divorced from the finding it belongs to.

Risk is ranked, and each finding is stated in the terms you decide on:
**how bad** (a severity earned by consequence — a credential read outranks a
read of the skill's own file; a secret read *plus* a way off the machine is an
exfiltration path and outranks either alone), **why** (the evidence), and **what
happens if it runs**. Scope sets the floor: a skill that only reads and writes
inside its own directory is low risk however much it touches — what earns a
review is *reaching outside* that directory (a home or absolute path, an `..`
escape, the network, another skill's files, the agent's own config). Skills that
stay in scope are named so their absence from the review list reads as "checked
and fine", not "unchecked".

On **y**, the `--then` command runs and the gate exits with its status. On
**N**, nothing is installed (exit 1). If a skill has findings and no terminal is
attached to confirm, the gate refuses (exit 2) rather than installing blind —
pass `--yes` to proceed anyway in a script. A clean target is approved without a
prompt. Without `--then`, `gate` reports and approves but runs nothing, so you
can wire it into your own install flow.

This is the **human-driven** interception point: `claude plugin install …` typed
in a terminal is a CLI command, not an agent tool call, so no harness hook fires
on it — `gate` is the wrapper you run instead. The **agent-driven** point is
covered separately: when an agent runs an install through its Bash tool, the
[guard hook](#3-enforce) intercepts it at `PreToolUse`. A skill install reads as
a distinct `pkg.install:skill/…` effect that no ordinary policy grants, so in
enforce mode an agent-initiated install fails closed.

### Install through skillspec (`pull` and `update`)

`gate` wraps an install command you supply. `skillspec pull` goes one step
further: it *is* the install verb, so the assessment cannot be skipped the way a
raw `claude plugin install` can, and one command works regardless of harness.

```bash
skillspec pull https://github.com/owner/skills          # assess, then install
skillspec pull owner/skills --plugin onboard            # one plugin from a marketplace
skillspec pull ./my-skill --into ~/.claude/skills       # place a local skill
```

`pull` stages the source, runs the same assessment as `gate` (the tree, then the
risk), and on approval installs it one of two ways — it detects which applies:

- **Proxy** — a Claude/Codex plugin-marketplace repo (one carrying
  `.claude-plugin/marketplace.json`) is installed through the harness CLI:
  `claude plugin marketplace add owner/repo` then `claude plugin install
  plugin@marketplace`, for each plugin (or the ones named with `--plugin`). Used
  automatically when the repo is a marketplace and the CLI is on `PATH`.
- **Place** — every other harness reads `SKILL.md` from a skills directory, so
  "install" is copying the skill folders there. skillspec does the copy itself.
  The destination is `~/.claude/skills` by default, `.claude/skills` with
  `--project`, or an explicit `--into <dir>` (a `~/.codex/skills`, an
  `AGENTS.md` `skills/` dir — anywhere a harness reads skills). This path needs
  no harness CLI and covers the whole `AGENTS.md`/`SKILL.md` gamut.

Force a path with `--harness claude|codex`. The confirmation rules are the
gate's: a clean source installs without a prompt; a source with findings needs
an interactive `y` or `--yes`, and refuses (exit 2) when neither a terminal nor
`--yes` is present.

`skillspec update <source>` re-pulls and, for a placed skill that already exists
at the destination, **shows how its capability surface changed before replacing
it** — a new network host, a new secret read — so a skill that quietly grows its
reach across a version cannot slip in on an update:

```text
Update to release-notes changes its capability surface:
Needs review
- egress_expansion  net.egress:telemetry.example.net  (new capability)
```

### Gate a skill in CI

```bash
# First contact: fail if the skill reads a secret or reaches the network.
skillspec boundary check ./my-skill

# Update: fail only if the capability envelope grew since a prior revision.
skillspec boundary check ./my-skill --against HEAD~1
```

Exit codes are a CI contract:

| Code | Meaning |
| --- | --- |
| 0 | clean / no drift |
| 1 | concerning surface, or drift that warrants review |
| 2 | incomplete surface, or the two revisions are not comparable |
| 3 | analysis error |

Absolute review is the right gate for a first install; drift is the right gate
for an update. A skill hostile from its first commit shows no drift, so the two
are deliberately different rules.

### See what changed between versions

```bash
skillspec boundary diff ./my-skill --against v1.2.0
```

```text
Needs review
- egress_expansion     net.egress:telemetry.example.net  (new capability)
- sensitive_expansion  fs.read:secret                    (new capability)
```

---

## 2. Create a policy

Compile the effect surface into a deny-by-default permission set:

```bash
skillspec boundary emit ./my-skill                          # default: skillspec
skillspec boundary emit ./my-skill --format egress-allowlist
skillspec boundary emit ./my-skill --format json -o policy.json
```

The default `skillspec` format is a `tool_boundary` block:

```yaml
# skillspec boundary emit --format skillspec
# target: ./my-skill
# SkillSpec does not enforce this boundary; a harness does.
tool_boundary:
  default: deny
  allow:
    - "net.egress:api.github.com"
    - "proc.exec:git"
    - "env.read:GITHUB_TOKEN"
  permission_required_for:
    - "fs.read:secret"
# Review before granting:
#   fs.read:secret -> ~/.aws/credentials
```

Sensitive grants — credential reads, writes to agent config, other skills'
files — are never placed in `allow`. They go under `permission_required_for`
so you decide on them explicitly.

`egress-allowlist` emits a plain host list for a proxy or container network
policy. Both formats express deny-by-default, which is what makes an
under-enumerated surface fail closed rather than open.

You can paste either artifact into your own tooling and stop here.

---

## 3. Enforce

The guard applies a reviewed policy to the skills you already run, through a
managed `PreToolUse` hook. It needs no `skill.spec.yml` and changes nothing
about the skills themselves.

### Install (observe mode)

```bash
skillspec boundary guard install
```

The guard installs in **observe** mode: it records every intercepted tool call
and whether an approved grant covers it, and blocks nothing. Observe is the
default on purpose — a guard that blocks on day one against a policy you have not
validated gets uninstalled within the hour.

### Approve skills

```bash
skillspec boundary guard add ./release-notes
skillspec boundary guard add https://github.com/owner/repo/tree/main/skills/deploy
```

`add` analyzes the skill, compiles the proposal, and stores it as a policy. The
allow grants are approved; sensitive grants are held pending, never silently
approved.

```bash
skillspec boundary guard status
```

```text
SkillSpec Boundary Guard
Mode: observe
Managed hooks: ~/.claude/settings.json
Policies: 2
- release-notes (6 allowed)
- deploy (4 allowed, 1 pending review)
```

### Watch, then promote

Use your skills normally. The guard records everything:

```bash
skillspec boundary guard log
```

```text
61 call(s) recorded; 61 covered by policy, 0 not.
```

When the log shows the policy covering real calls, promote:

```bash
skillspec boundary guard mode enforce
```

In enforce mode, a call whose effects are all covered proceeds; a call with an
uncovered effect is denied.

```text
⛔ SkillSpec boundary guard: no approved grant covers: net.egress:exfil.evil.test
```

`prompt` mode is the middle ground: an uncovered call is escalated to you rather
than refused.

### Remove

```bash
skillspec boundary guard uninstall
```

This removes only the managed hook — your own hooks are untouched. Stored
policies and the decision log are left in place.

---

## What the guard is and is not

- It is an **advisory gate** at a lifecycle event the harness exposes. If the
  hook cannot run, the harness decides; `guard status` says so plainly.
- It is **not a sandbox** — no kernel isolation, no network interposition.
- A `PreToolUse` call cannot be attributed to a single skill, so enforcement is
  against the **union** of every approved policy's grants. Per-skill policies are
  kept for review, but the runtime allow-set is global.
- It governs the effects it can model. A tool call it cannot normalize into an
  effect is not blocked — deny-by-default applies to *enumerated* effects, not
  to unrecognized tools.

## Honest limits

- **Static analysis.** It reports what a skill *can* do, never what it *will*.
  Capability is not intent, and a skill can legitimately need a broad surface.
- **Misuse of a granted effect.** A skill allowed to reach `api.github.com` can
  exfiltrate to `api.github.com`. The boundary constrains *where*, not *what*.
- **Coverage.** Shell, Python, and server-side TS/JS (Deno, Bun, Node) are read;
  browser component code has no host effects and is correctly quiet. Effects
  reached only through a runtime-fetched payload or a variable bound on an
  earlier line are not seen — and under a deny-default boundary, those fail
  closed.

## Command reference

| Command | Purpose |
| --- | --- |
| `boundary map <folder> [--json]` | Structure: skills, resources, orphans, cross-references |
| `boundary assess <target> [--json]` | Security analysis: what each skill can reach, ranked by risk |
| `boundary gate <target> [--then <cmd>] [--yes]` | Structure + security analysis, then run `<cmd>` on approval |
| `pull <source> [--harness h] [--into d] [--plugin p] [--yes]` | Assess, then install via CLI proxy or file placement |
| `update <source> [...]` | Re-pull, showing capability drift before replacing |
| `boundary <target>` | Report the effect surface |
| `boundary <target> --reveal <file>` | Write decoded concealment payloads to a file |
| `boundary check <target> [--against <ref>]` | CI gate; exit codes 0–3 |
| `boundary diff <target> --against <ref>` | Show what capability changed |
| `boundary emit <target> [--format <fmt>] [-o <file>]` | Compile a policy artifact |
| `boundary guard install` | Install the managed hook (observe mode) |
| `boundary guard add <target>` | Approve a skill's proposal as a policy |
| `boundary guard status` | Show mode, hooks, and policies |
| `boundary guard mode <observe\|prompt\|enforce>` | Set the enforcement mode |
| `boundary guard log [--json]` | Read the decision log |
| `boundary guard uninstall` | Remove the managed hook |

Design detail lives in [`docs/design/security/`](design/security/README.md).
