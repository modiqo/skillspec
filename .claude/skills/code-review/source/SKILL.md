---
name: code-review
description: Multi-agent code review with deep analysis. Orchestrates codebase research, optional web research, parallel Rust engineers, codex second opinion, and general-purpose reviewers into a synthesized report. Use when the user asks to review code, review a PR, review changes, audit code quality, or says "review", "/review", "code review", "check my changes", "review this PR", "review diff". Trigger for ANY code review request, even partial — e.g., "look over this", "anything wrong with these changes", "sanity check".
---

# Code Review

Orchestrate a multi-agent code review pipeline: research the changes, optionally investigate external alternatives, dispatch parallel domain-specific reviewers, and synthesize everything into a structured report.

## Input Collection

Prompt the user to select an input mode. Present these options clearly:

> **What would you like me to review?**
>
> 1. **Current PR** — review the open PR on this branch (description + diff)
> 2. **Diff to main** — review all uncommitted and committed changes vs main
> 3. **Specific paths** — review specific files, crates, or directories
>
> Pick a number, or describe what you'd like reviewed.

### Collecting the diff

**Mode 1 — Current PR:**
```bash
gh pr view --json title,body,number,baseRefName
gh pr diff
```
If no open PR exists, tell the user and suggest mode 2 instead.

**Mode 2 — Diff to main:**
```bash
git diff main...HEAD
```
Also include `git log main..HEAD --oneline` for commit context.

**Mode 3 — Specific paths:**
Ask the user for paths. Read the specified files directly. No diff — review the code as-is.

Store the collected input (diff text, PR description, file contents) for use in subsequent phases.

### Handling large diffs

If the diff exceeds ~2000 lines, save it to a temporary file (`/tmp/code-review-diff.patch`) and have agents read it via the Read tool rather than inlining it in their prompts. This prevents context overflow. Reference the file path in agent prompts instead of pasting the diff.

## Phase 1: Research & Domain Mapping

Spawn an **Agent** (subagent_type: `general-purpose`) and include the full text of the research-codebase skill instructions in its prompt (read `.claude/skills/research-codebase/SKILL.md` first). Direct the agent to research the changes with this focus:

- Map all changed files to their logical domains and crate boundaries
- Identify which changes are Rust code vs non-Rust (CI, docs, config, TypeScript SDK, nix, etc.)
- Understand the architectural context around each change — what traits, types, and modules are involved
- Group changes into **at most 4 logical domain groups**, merging smaller related changes together
- Flag any changes that warrant external web research:
  - New dependencies or crate additions
  - Unfamiliar architectural patterns
  - Potentially deprecated API usage
  - Cases where alternative approaches might exist

The research agent will internally spawn codebase-locator, codebase-analyzer, and codebase-pattern-finder sub-agents. You do not need to manage those agents directly.

### Parse the research output for:

1. **Domain groups** — a list of 1-4 logical clusters, each with:
   - A descriptive name (e.g., "OAuth token refresh flow", "adapter registry refactor")
   - The files and line ranges involved
   - Key context (traits, types, modules, patterns in play)
   - Whether changes are Rust or non-Rust

2. **Web research questions** — specific questions worth investigating, or empty if none

3. **Change characterization** — new feature, refactor, bugfix, etc., to help reviewers calibrate

## Phase 2: Web Research (conditional)

**Skip this phase entirely if Phase 1 flagged no web research questions.**

If questions were flagged, spawn **web-search-researcher** agent(s) with the specific questions. Each agent should:

- Search for the specific concern (e.g., "ring crate security advisories 2026", "tokio channel vs crossbeam performance comparison")
- Return findings with source links
- Focus on actionable information relevant to the review

If web research fails or times out, proceed to Phase 3 without it. Note the failure in the final report.

## Phase 3: Parallel Review Agents

Spawn all review agents in a **single message** so they run concurrently. Every agent receives:

- The full diff (or file contents for mode 3)
- The research context from Phase 1 (domain map, architectural context)
- Web research findings from Phase 2 (if any)
- The PR description (if mode 1)

### Path-triggered guardrails (apply BEFORE building agent prompts)

If the diff touches any of these paths, the **command catalog guardrails** apply and MUST be propagated into every reviewer's prompt:

- `crates/rote-cli/src/cli/parser/parsers/<family>/{mod,spec,tests}.rs`
- `crates/rote-cli/src/cli/parser/spec/*.rs`
- `crates/rote-search/src/builder/commands.csv`
- `crates/xtask/src/gen_artifacts.rs`

When this trigger fires:

1. Read `.claude/skills/command-catalog-guardrails/SKILL.md` and extract its checklist + drift-class table.
2. Append both verbatim to every Phase 3 agent prompt (rust-engineer, general-purpose, codex-second-opinion). Each agent must explicitly evaluate the diff against every checklist item and call out failures in its findings table.
3. In the synthesis report, add a **"Catalog Guardrails"** section above "Critical" listing any checklist items the agents flagged. Use it to make catalog drift visible separately from generic Rust review findings — the failure modes (silent value drop, help-before-validate, positional ignored) don't fit cleanly into the standard severity buckets.

### Agent dispatch rules

**For Rust domain groups** — spawn one **rust-engineer** agent per domain group (max 4). Each agent's prompt should include:

- Its specific domain assignment and the files/lines it owns
- The surrounding architectural context from research
- Instruction to produce detailed findings covering:
  - Correctness, logic errors, edge cases
  - Performance implications
  - Idiomatic Rust (1.94+), type system usage
  - Error handling, safety, concurrency patterns
  - API design, trait boundaries, public interface quality
  - Testing gaps
- Instruction to format output as a markdown table with columns: `#`, `Severity`, `Location`, `Issue`, `Suggested Fix`. Also include a separate "Testing Gaps" table with columns: `Gap`, `Risk`, `Priority`.

**For non-Rust changes** — spawn one **general-purpose** agent with a code-review prompt covering:

- Correctness and completeness of config/CI/doc changes
- Consistency with existing patterns
- Security concerns (exposed secrets, overly permissive permissions)
- SDK changes: API compatibility, test coverage, documentation
- Same table output format as rust-engineer agents

**Always** — spawn one **codex-second-opinion** agent with the full diff. Codex reviews independently without domain partitioning, providing a fresh perspective across all changes. Instruct it to format findings as a markdown table with the same columns.

### Scaling

| Diff size | Rust domains | Non-Rust | Codex | Total agents |
|-----------|-------------|----------|-------|-------------|
| Tiny (<20 lines, single domain) | 1 rust-engineer | — | 1 | 2 |
| Medium (multi-domain) | 2-3 rust-engineers | if applicable | 1 | 3-5 |
| Large (many crates) | 4 rust-engineers (merged) | if applicable | 1 | 5-6 |
| Non-Rust only | — | 1 general-purpose | 1 | 2 |
| Mixed | 1-4 rust-engineers | 1 general-purpose | 1 | 3-6 |

## Phase 4: Synthesis

After **all** review agents complete, produce the final report. Do not start synthesis until every agent has returned.

### Report structure

Organize findings by severity, not by agent. Every finding goes in a table with a column showing which agent(s) reported it. Deduplicate: if multiple agents report the same issue, merge into one row and list all reporters. Mark multi-reporter findings ✅.

```markdown
# Code Review: [PR title / branch name / paths reviewed]

## Overall Assessment
[One sentence: ready to merge / needs changes / needs discussion]

## Cross-Cutting Themes

| Theme | Occurrences | Files | Impact |
|-------|-------------|-------|--------|
| Silent error swallowing | 20+ | state.rs, db.rs, storage.rs, snapshot.rs | Data corruption goes undetected |
| Missing `#[must_use]` | 15+ | dependency.rs, health.rs, state.rs | Discarded return values hide bugs |

## Critical

| # | Location | Issue | Suggested Fix | Reported By |
|---|----------|-------|---------------|-------------|
| 1 | `db.rs:969` | `replace_command_log` not in transaction — crash = data loss | Wrap in `BEGIN EXCLUSIVE...COMMIT` | rust-eng-state, codex ✅ |
| 2 | `manager.rs:65` | Path traversal — unsanitized workspace name in `join()` | Validate `[a-zA-Z0-9_-]+` | rust-eng-lifecycle, codex ✅ |
| 3 | `state.rs:557` | `conn()` panics via `.expect()` in library code | Return `Result<&Connection>` | rust-eng-state |

## High

| # | Location | Issue | Suggested Fix | Reported By |
|---|----------|-------|---------------|-------------|
| 4 | `state.rs:1433` | Counter set to `entries.len()` instead of max response ID | Derive from `response_ids.max()` | rust-eng-state, codex ✅ |
| 5 | `storage.rs:468` | Header obfuscation only covers `Authorization: Bearer` | Case-insensitive denylist for secret headers | rust-eng-storage, codex ✅ |

## Medium

| # | Location | Issue | Suggested Fix | Reported By |
|---|----------|-------|---------------|-------------|
| 6 | `db.rs:479` | Timestamp parse failures silently replaced with `Utc::now()` | Log warning or propagate error | rust-eng-state |
| 7 | `dependency.rs:271` | Broken dependency chains not detected transitively | Iterative fixed-point propagation | rust-eng-deps |

## Low

| # | Location | Issue | Suggested Fix | Reported By |
|---|----------|-------|---------------|-------------|
| 8 | `dependency.rs:131` | `$100` accepted as variable reference (all-numeric) | Require first char `[A-Za-z_]` | rust-eng-deps, codex ✅ |
| 9 | `lib.rs:322` | `root_path()` returns `&PathBuf` instead of `&Path` | Change return type to `&Path` | rust-eng-storage, rust-eng-lifecycle |

## Testing Gaps

| # | Gap | Risk | Priority | Reported By |
|---|-----|------|----------|-------------|
| 1 | No round-trip test for log_command → reload | Sequence corruption undetected | High | rust-eng-state |
| 2 | Zero tests for dependency inference pipeline | False positives/negatives in production | High | rust-eng-deps |
| 3 | No test for snapshot `restore` | Corrupt workspace state after restore | Medium | rust-eng-lifecycle |

## Web Research
[Only present if Phase 2 ran]

| Question | Finding | Source | Impact on Review |
|----------|---------|--------|-----------------|
| ... | ... | [link] | ... |
```

### Severity definitions

- **Critical** — Bugs that corrupt data, lose work, crash in production, or create security vulnerabilities. Must fix before merge.
- **High** — Significant correctness or safety issues that will cause problems under realistic conditions. Should fix before merge.
- **Medium** — Code quality, robustness, or minor correctness issues. Fix soon, but not necessarily blocking.
- **Low** — Style, idiom, minor improvements. Fix at convenience.

### Synthesis rules

- Organize all findings into severity tables — not per-agent sections
- Deduplicate: if multiple agents report the same issue, merge into one row listing all reporters
- Multi-reporter findings get ✅ in the Reported By column (high confidence signal)
- Within each severity table, sort by number of confirming reporters (most first), then by file path
- Preserve all file:line references in the Location column
- The "Reported By" column uses short labels: `rust-eng-[domain]`, `codex`, `general`
- Testing gaps get their own consolidated table, also with a Reported By column
- Cross-cutting themes go above the severity tables — these are patterns, not individual findings
- If no findings at a severity level, omit that table entirely

### Output

Print the report as markdown in the conversation. Do not write to a file unless the user asks.
