# Analyzing and constraining agent skills with `skillspec boundary`

`skillspec boundary` answers one question about a skill:

> If an agent executed this skill, what could it reach — and what is the
> smallest permission set that still lets it work?

It works on a skill you have locally or a public GitHub skill URL, needs no
`skill.spec.yml` and no changes to the skill, and never executes anything in the
package. You can use every part of it independently.

There are three things you can do, in increasing order of commitment:

1. **Assess** — see what a skill can reach.
2. **Create a policy** — compile that into a least-privilege permission set.
3. **Enforce** — apply a reviewed policy to the skills you already run.

Nothing forces you past step 1.

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
