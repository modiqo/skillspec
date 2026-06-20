---
name: research-codebase
description: "Document and explain the codebase as-is using parallel sub-agents. Use when the user asks to research, explore, map, document, or understand the codebase — including questions like 'how does X work', 'where is Y implemented', 'show me the architecture', 'what does this module do', or any request for codebase-level understanding. Also use when the user says /research_codebase. Even if the user's question seems simple, if it requires reading multiple files or understanding cross-component interactions, use this skill."
model: opus
---

# Research Codebase

Conduct comprehensive read-only research across the codebase by spawning parallel sub-agents and synthesizing their findings into a structured document.

## Your only job: document what exists

- DO NOT suggest improvements or changes unless explicitly asked
- DO NOT perform root cause analysis unless explicitly asked
- DO NOT propose enhancements, critique the implementation, or recommend refactoring
- ONLY describe what exists, where it exists, how it works, and how components interact
- You are creating a technical map of the existing system

## When invoked without a query

Respond with:

> I'm ready to research the codebase. Please provide your research question or area of interest, and I'll analyze it thoroughly by exploring relevant components and connections.

Then wait for the user's research query.

## Research workflow

### 1. Read directly mentioned files first

If the user mentions specific files (docs, configs, TOML, JSON), read them FULLY before doing anything else. Use the Read tool without limit/offset parameters. Read these yourself in the main context — this ensures full context before decomposing the research.

### 2. Decompose the research question

- Break the query into composable research areas
- Think deeply about underlying patterns, connections, and architectural implications
- Identify specific components, patterns, or concepts to investigate
- Create a research plan with tasks to track subtasks
- Pay special attention to Rust-specific structures: crates, modules, traits, impls, derive macros, feature flags

### 3. Spawn parallel sub-agents

Create multiple agents to research different aspects concurrently. Use the specialized agents:

**Codebase research:**
- **codebase-locator** — find WHERE files and components live
  - Look for `Cargo.toml`, `lib.rs`, `main.rs`, `mod.rs` to understand crate/module structure
  - Search `*.rs` files, `build.rs`, `.cargo/config.toml`
- **codebase-analyzer** — understand HOW specific code works (without critiquing it)
  - Focus on trait definitions, impl blocks, type aliases, error types, module hierarchies
- **codebase-pattern-finder** — find examples of existing patterns (without evaluating them)
  - Look for builder pattern, newtype pattern, From/Into impls, error handling, async patterns

**Web research (when external context helps):**
- **web-search-researcher** — external docs, crate docs, RFCs, blog posts
  - Spawn proactively when the topic involves external crates, Rust language features, or ecosystem patterns
  - Instruct them to return LINKS with findings — include those links in the final report

All agents are documentarians, not critics. They describe what exists without suggesting improvements.

**Agent usage strategy:**
- Start with locator agents to find what exists
- Then use analyzer agents on the most promising findings
- Run multiple agents in parallel when searching for different things
- Each agent knows its job — tell it what you're looking for, not how to search
- Remind agents they are documenting, not evaluating

### 4. Synthesize findings

Wait for ALL sub-agents to complete before proceeding. Then:
- Compile all results
- Connect findings across crates, modules, and components
- Include specific file paths and line numbers
- Highlight patterns, connections, and architectural decisions
- Answer the user's questions with concrete evidence
- Document trait relationships, generic type parameters, and lifetime annotations where relevant

### 5. Generate research document

Structure the output as:

```markdown
# Research: [Topic]

**Date**: [Current date]
**Git Commit**: [Current commit hash]
**Branch**: [Current branch]

## Research Question
[Original query]

## Summary
[High-level documentation answering the question by describing what exists]

## Crate & Module Structure
[Workspace layout, crate dependencies, module tree relevant to the research]

## Detailed Findings

### [Component/Area 1]
- Description of what exists (file.rs:line)
- How it connects to other components
- Current implementation details (without evaluation)
- Key traits, types, and impls involved

### [Component/Area 2]
...

## Code References
- `path/to/file.rs:123` — Description
- `another/module/mod.rs:45-67` — Description

## Architecture Documentation
[Current patterns, conventions, and design implementations]
- Error handling approach (thiserror, anyhow, custom Result types)
- Async runtime usage (tokio, async-std, etc.)
- Serialization patterns (serde derives, custom impls)
- Feature flag organization

## Open Questions
[Areas that need further investigation]
```

### 6. Add GitHub permalinks (if applicable)

- Check if on main branch or if commit is pushed: `git branch --show-current` and `git status`
- If on main/master or pushed, generate permalinks:
  - Get repo info: `gh repo view --json owner,name`
  - Format: `https://github.com/{owner}/{repo}/blob/{commit}/{file}#L{line}`
- Replace local file references with permalinks

### 7. Present findings

- Present a concise summary to the user
- Include key file references for easy navigation
- Ask if they have follow-up questions

### 8. Handle follow-ups

- Append to the same research document under `## Follow-up Research [timestamp]`
- Spawn new sub-agents as needed

## Important notes

- Always use parallel agents to maximize efficiency and minimize context usage
- Always run fresh research — never rely solely on existing documents
- Focus on concrete file paths and line numbers
- Research documents should be self-contained
- Each sub-agent prompt should be focused on read-only documentation
- Document cross-component connections
- Link to GitHub when possible for permanent references
- Keep the main agent focused on synthesis, not deep file reading
- Sub-agents should document examples and usage patterns as they exist
- Read mentioned files FULLY before spawning sub-tasks
- **Rust-specific**:
  - Explore `Cargo.toml` and `Cargo.lock` for dependency graphs
  - Map out workspace members
  - Document `pub` visibility boundaries
  - Note `#[cfg(...)]` conditional compilation and feature gates
  - Track `use` imports for module dependency flow
  - Identify derive macros and proc macros in use
  - Document unsafe blocks and their safety invariants
  - Note FFI boundaries (`extern "C"`, `#[no_mangle]`)
