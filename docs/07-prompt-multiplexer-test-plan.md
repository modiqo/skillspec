# Prompt Multiplexer Test Plan

This runbook verifies the post-install `skillspec` prompt skill in both Codex
and Claude. The prompt skill is the user-facing multiplexer: users invoke
`/skillspec ...` in the harness, and the skill routes setup work to import,
router install, optional durable-executor install, rote-browse import/install,
or observed-workspace skill creation.

The multiplexer is not a separate `skillspec multiplexer` CLI subcommand. The
CLI is the substrate used by the prompt skill after the harness selects the
`skillspec` skill.

## User-Facing Prompt Surface

Expected prompt forms:

```text
/skillspec import <local-skill-folder-or-public-github-uri>
/skillspec install router
/skillspec install durable-executor from <local-skill-folder-or-public-github-uri>
/skillspec import <rote-browse-source>, compile it, and install it
/skillspec observe durable workspace <workspace> and create a spec skill
```

The first screen of the experience should not ask the user to pick from
separate prompt skills. The `skillspec` skill owns the initial routing question
and should explain only the missing source, target root, or confirmation needed
for the selected path.

## Test Matrix

Run the same scenario once in Codex and once in Claude.

| Harness | Skill install target | Primary root to observe | Expected invocation |
| --- | --- | --- | --- |
| Codex | `codex` or shared `agents` | `~/.codex/skills` or `~/.agents/skills` | `/skillspec ...` |
| Claude | `claude-local` or shared `agents` | `~/.claude/skills` or `~/.agents/skills` | `/skillspec ...` |

If `~/.codex/skills` and `~/.claude/skills` are symlinked to
`~/.agents/skills`, prefer the shared root. Router mode in a shared `.agents`
root should write Codex `agents/openai.yaml` controls and Claude-compatible
`disable-model-invocation: true` frontmatter.

## Backup Existing State

Before testing, capture every root that the harness may read.

```sh
export SS_TEST_STAMP="$(date +%Y%m%d-%H%M%S)"
export SS_TEST_BACKUP="$HOME/skillspec-test-backups/$SS_TEST_STAMP"
mkdir -p "$SS_TEST_BACKUP"

test -d "$HOME/.agents" && cp -R "$HOME/.agents" "$SS_TEST_BACKUP/agents"
test -d "$HOME/.codex" && cp -R "$HOME/.codex" "$SS_TEST_BACKUP/codex"
test -d "$HOME/.claude" && cp -R "$HOME/.claude" "$SS_TEST_BACKUP/claude"
test -d "$HOME/.skillspec" && cp -R "$HOME/.skillspec" "$SS_TEST_BACKUP/skillspec"

echo "backup=$SS_TEST_BACKUP"
```

Record the current root layout:

```sh
ls -la "$HOME/.agents" "$HOME/.codex" "$HOME/.claude" 2>/dev/null || true
for root in "$HOME/.agents/skills" "$HOME/.codex/skills" "$HOME/.claude/skills"; do
  test -d "$root" && find "$root" -maxdepth 1 \( -type l -o -type d \)
done | sort
```

Restore by moving the current directories aside and copying the saved backup
back into place. Do not restore over an active test session.

## Build And Install The Local Binary

From the repository root:

```sh
cargo build --release -p skillspec
mkdir -p "$HOME/.local/bin"
cp -f target/release/skillspec "$HOME/.local/bin/skillspec"
"$HOME/.local/bin/skillspec" --help
"$HOME/.local/bin/skillspec" install targets
```

Make sure the shell that launches Codex or Claude sees this binary first:

```sh
export PATH="$HOME/.local/bin:$PATH"
which skillspec
skillspec --help
```

## Install The Prompt Skill

Install the local `skillspec` prompt skill into the harness roots under test.

Shared-root test:

```sh
skillspec install skill skills/skillspec --target agents --force
```

Separate-root test:

```sh
skillspec install skill skills/skillspec --target codex --force
skillspec install skill skills/skillspec --target claude-local --force
```

Cross-harness smoke test:

```sh
skillspec install skill skills/skillspec \
  --target agents \
  --target codex \
  --target claude-local \
  --force
```

After installing the prompt skill, fully restart the Codex or Claude session.
Do not rely on an already-open chat session to reload changed skills or a new
`PATH`.

## Codex Prompt Test

Start a fresh Codex session from a shell where `which skillspec` resolves to the
local binary.

### 1. Import A Preexisting PDF Skill

Prompt:

```text
/skillspec import https://github.com/anthropics/skills/tree/main/skills/pdf, compile it for Codex, install it, and prove it
```

Expected result:

- The source is staged locally before conversion.
- The harness reads the full skill folder, not only `SKILL.md`.
- A reviewed `skill.spec.yml` is created or updated.
- `skillspec validate`, `skillspec imports check`, and `skillspec test` are run
  when available.
- The compiled skill installs into the Codex or shared root.

### 2. Install Durable Executor

Prompt:

```text
/skillspec install durable-executor from /path/to/durable-executor
```

Expected result:

- If `durable-executor` is already present, the skill reports it and keeps it
  implicit.
- If it is missing, the skill asks for or uses the approved source path or URI.
- The install path follows the same reviewed import, validate, test, compile,
  and install loop as other skills.

### 3. Install Rote Browse

Prompt:

```text
/skillspec import /path/to/rote-browse, compile it for Codex, install it, and prove it
```

Expected result:

- `rote-browse` is treated as a normal source skill unless a dedicated route is
  added later.
- The install proves the generated SkillSpec and leaves browser execution owned
  by the `rote-browse` skill, not by durable-executor.

### 4. Install Router

Prompt:

```text
/skillspec install router
```

Expected result:

- The router skill is installed into the selected managed root.
- No `--router-root` is requested or used.
- Router mode applies explicit-only controls to indexed skills.
- `durable-executor` remains implicit when present.
- Missing `durable-executor` is reported as "durable first-hop unavailable",
  not installed silently.
- The routing index is built and preparedness is checked.

### 5. Verify Router Indexing

Run from the shell after the prompt path finishes:

```sh
skillspec router index status \
  --roots "$HOME/.agents/skills" \
  --index "$HOME/.skillspec/router" \
  --visibility-manifest "$HOME/.skillspec/router/visibility-manifest.json" \
  --json

skillspec route \
  --index "$HOME/.skillspec/router" \
  --query "extract tables from a PDF" \
  --json
```

Expected result:

- `stale` is `false` after a clean install or refresh.
- PDF-related requests rank the imported PDF skill.
- `durable-executor` is not marked explicit-only.
- Other indexed skills have Codex explicit invocation controls.

If the test uses separate roots instead of the shared `.agents` root, pass the
same roots selected during router install to each `--roots` invocation.

If a skill was added manually after router install, run:

```sh
skillspec router index status \
  --roots "$HOME/.agents/skills" \
  --index "$HOME/.skillspec/router" \
  --visibility-manifest "$HOME/.skillspec/router/visibility-manifest.json" \
  --json

skillspec router index refresh \
  --roots "$HOME/.agents/skills" \
  --index "$HOME/.skillspec/router" \
  --visibility-manifest "$HOME/.skillspec/router/visibility-manifest.json" \
  --json
```

Expected result:

- Prose-only additions include `skillspec import-skill` advice.
- SkillSpec-backed additions are detected and indexed directly.
- Refresh reapplies explicit invocation controls and rebuilds the index.

## Claude Prompt Test

Start a fresh Claude session from a shell where `which skillspec` resolves to the
local binary.

Run the same prompts as the Codex test, changing the target language only when
the prompt asks for it:

```text
/skillspec import https://github.com/anthropics/skills/tree/main/skills/pdf, compile it for Claude, install it, and prove it
/skillspec install durable-executor from /path/to/durable-executor
/skillspec import /path/to/rote-browse, compile it for Claude, install it, and prove it
/skillspec install router
```

Expected Claude-specific result:

- Claude local skill files land under `~/.claude/skills` unless a shared
  `.agents` root is selected.
- Shared `.agents` roots receive `disable-model-invocation: true` frontmatter
  for router-managed explicit-only skills.
- Claude `skillOverrides` are written through the visibility manifest path when
  a Claude root is managed directly.
- Router uninstall restores snapshots from the manifest rather than guessing
  previous visibility state.

## Final Checks

Run these checks after each harness pass:

```sh
skillspec install targets
skillspec router index status \
  --roots "$HOME/.agents/skills" \
  --index "$HOME/.skillspec/router" \
  --visibility-manifest "$HOME/.skillspec/router/visibility-manifest.json"
skillspec route --index "$HOME/.skillspec/router" --query "use the browser with durable evidence"
```

Inspect these files for the selected root:

```sh
find "$HOME/.agents/skills" -maxdepth 3 \( \
  -name SKILL.md -o \
  -name skill.spec.yml -o \
  -path '*/agents/openai.yaml' \
\) | sort
```

Pass criteria:

- `/skillspec` is the only prompt entry point needed for setup.
- The PDF import path creates a reviewed SkillSpec-backed package.
- `durable-executor` is optional and remains the implicit first hop when
  present.
- `rote-browse` can be imported and installed through the normal source-skill
  path.
- Router install does not ask for a router root.
- Router status is non-stale after install or refresh.
- Out-of-band prose and SkillSpec-backed additions are detected by status and
  repaired by refresh.
- The final prompt response includes command evidence, trace or test evidence,
  and any token or workspace stats collected during the run.
