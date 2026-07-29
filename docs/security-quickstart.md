# SkillSpec security quickstart

The critical commands to analyze a skill's security, enforce a policy on your
harness, and install skills safely — with SkillSpec run locally.

Everything here is static analysis: nothing inside a skill is executed by the
analysis. A `<target>` is a **local folder** or a **public git URL** on any host
(GitHub, GitLab, Bitbucket, self-hosted), for example `./my-skill` or
`https://github.com/owner/repo/tree/main/skills/example`.

## Install the CLI

```bash
curl -fsSL https://skillspec.sh/install.sh | sh   # prebuilt binary
# or, with Rust installed:
cargo install skillspec
```

## 1. Analyze what a skill can reach

```bash
skillspec boundary map <target>            # Structure: skills, resources, orphan files, cross-references
skillspec boundary assess <target>         # Security: what each skill reaches, ranked by severity
skillspec boundary assess <target> --json  # machine-readable; each finding carries a resolved `link`
```

`assess` is the main command. It is scope-aware (a skill confined to its own
directory is low risk; reaching outside it — a home or absolute path, the
network, another skill's files — is what earns review), ranks findings
`critical` / `high` / `medium` / `low`, qualifies documentation examples rather
than counting them as live effects, and links each finding to the exact line.

Gate a skill in CI:

```bash
skillspec boundary check <skill>                    # exit 1 if it reads a secret or reaches the network
skillspec boundary check <skill> --against HEAD~1   # exit 1 only if the capability envelope grew (updates)
skillspec boundary diff  <skill> --against v1.0.0   # show what capability changed since a revision
```

Exit codes: `0` clean · `1` concerning / drift · `2` incomplete or not comparable · `3` error.

## 2. Guard your harness by policy

```bash
skillspec boundary emit <skill> --format skillspec   # compile a deny-by-default least-privilege policy
skillspec boundary guard install                     # install the managed PreToolUse hook (observe mode)
skillspec boundary guard add <skill>                 # approve a skill's proposal as a stored policy
skillspec boundary guard log                          # watch which tool calls a policy covers
skillspec boundary guard mode enforce                # deny effects no approved policy covers
skillspec boundary guard status                       # mode, managed hooks, stored policies
skillspec boundary guard uninstall                    # remove only the managed hook
```

Recommended path: `guard install` (starts in observe, blocks nothing) → `guard
add` your skills → watch `guard log` against real usage → `guard mode enforce`.
Modes escalate `observe → prompt → enforce`.

## 3. Install skills safely through SkillSpec

```bash
skillspec pull <target>                         # assess, show the tree + risk, install only on approval
skillspec pull <target> --into ~/.claude/skills # place skill folders into a skills directory
skillspec pull <target> --harness claude        # force the Claude/Codex plugin-marketplace CLI proxy
skillspec pull <target> --plugin <name> --yes   # pick a marketplace plugin; skip the prompt (scripts)
skillspec update <target>                        # re-pull; show capability drift before replacing
```

Or gate any install command you already use:

```bash
skillspec boundary gate <target> --then 'claude plugin install pack@marketplace'
```

`pull` and `gate` refuse (exit 2) when there are findings and no terminal is
attached to confirm; pass `--yes` to proceed in a script.

## End to end

```bash
skillspec boundary map    https://github.com/owner/skills   # orient
skillspec boundary assess https://github.com/owner/skills   # decide
skillspec pull            https://github.com/owner/skills   # install safely on approval
skillspec boundary emit <skill> --format skillspec          # compile a policy
skillspec boundary guard install && skillspec boundary guard mode enforce   # enforce going forward
```

Full reference: [`docs/boundary-guide.md`](boundary-guide.md).
