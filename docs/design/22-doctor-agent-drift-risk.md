# Doctor Agent Drift Risk

Status: implemented in part on `doctor-agent-drift-risk`; extended model design
Owner: SkillSpec
Source design: `~/tulving/design/skillspec-agent-drift-risk/README.md`
Last synchronized: 2026-06-27

## Purpose

`skillspec doctor` is a static risk assessment command for agent skills. It is
not only a structural hygiene report.

It should answer:

```text
If an agent reads this skill as prose, how likely is it to miss, distort,
skip, over-trigger, under-trigger, or fail to prove the instructions that
matter?
```

This is different from saying complex skills are bad. A complex skill can be
high quality. The risk is load-bearing behavior trapped in prose:

- late safety rules;
- many independent constraints;
- ambiguous execution steps;
- implicit dependencies;
- broad activation-loaded text;
- weak or malformed discovery metadata;
- multi-skill or plugin folders flattened into one package;
- missing test, trace, progress, or proof surfaces.

The command keeps the legacy positive `structural_score` for compatibility, but
the new interpretation is explicit:

- `structural_score`: positive hygiene score; higher is cleaner;
- `agent_drift_risk.score`: risk score; higher means more likely drift;
- `agent_drift_risk.conditions`: measured reasons for risk;
- `frontmatter_discovery_risk`: discovery and selection risk before the body is
  loaded;
- `workspace_agent_drift_risk`: aggregate workspace/package risk for folder
  shapes;
- `contract_mitigation`: whether a valid `skill.spec.yml` reduces prose drift;
- `basis_registry`: research, docs, or local methodology used to justify each
  condition.

## Non-Goals

- Do not claim runtime failure was observed. `doctor` is static.
- Do not claim exact probabilities before calibration against execution traces.
- Do not punish long documentation by itself. Penalize long active instruction
  load, buried obligations, ambiguous execution, missing proof, and context
  pressure.
- Do not cite a paper for a threshold the paper did not define. Papers justify
  risk direction and measurement families. Thresholds are SkillSpec policy until
  calibrated.
- Do not flatten a repository of many skills into one synthetic skill.
- Do not treat plugin namespace, nested skill path, or install slug identity as
  cosmetic. They are part of the routing surface.

## Current Implementation

The current branch implements these foundations:

- local and public GitHub target support through the existing doctor staging
  path;
- shape-first classification:
  - `simple_skill`;
  - `entry_skill_with_subskills`;
  - `multi_skill_workspace`;
  - `plugin_workspace`;
  - `non_skill_repository`;
- full single-package analysis for `simple_skill`;
- workspace aggregate plus one package report per `SKILL.md` for multi-skill,
  entry-with-subskills, and plugin-shaped folders;
- frontmatter parsing and discovery risk;
- agent drift risk;
- raw activation risk;
- SkillSpec contract mitigation and residual risk;
- namespace-preserving plugin package identity;
- compact text output and complete JSON output;
- command help, command log, command spec, README, and `sensemake` discovery
  updates for the revised `doctor` command.

Future sections in this document intentionally go beyond the current
implementation where they describe tokenizers, model/harness profiles,
compaction inputs, and calibrated scoring. Those sections are design
requirements for the next doctor iterations.

## Research Basis

Every report condition must cite at least one basis entry. Basis entries must
state the claim narrowly so the JSON does not overstate what a paper or doc
proved.

| Basis ID | Kind | Source | Narrow Claim Used By Doctor |
| --- | --- | --- | --- |
| `lost_middle_position_effect` | research paper | Liu et al., 2023, "Lost in the Middle: How Language Models Use Long Contexts", https://arxiv.org/abs/2307.03172 | Long-context models use relevant information less reliably when it appears in the middle of long contexts than near the beginning or end. Doctor may flag load-bearing obligations buried in middle or late regions as position risk. |
| `ruler_effective_context` | research paper | Hsieh et al., 2024, "RULER: What's the Real Context Size of Your Long-Context Language Models?", https://arxiv.org/abs/2404.06654 | Advertised context length is not the same as reliable usable context across tasks. Doctor may treat high context load as risk, especially when active skill text approaches a context or compaction budget. |
| `ifeval_verifiable_instructions` | research paper | Zhou et al., 2023, "Instruction-Following Evaluation for Large Language Models", https://arxiv.org/abs/2311.07911 | Instruction following can be evaluated with checkable constraints. Doctor may treat untestable instructions and missing proof surfaces as risk. |
| `skillsbench_focused_skills` | research benchmark | SkillsBench, "Can Skills Make AI Agents Competent?", https://arxiv.org/abs/2602.12670 and https://www.skillsbench.ai/ | Focused skills outperform broad comprehensive documentation; skills include instructions plus resources and are evaluated with deterministic task success checks. Doctor may flag broad active bodies and recommend focused, checkable contracts. |
| `agentboard_process_metrics` | research benchmark | AgentBoard, "An Analytical Evaluation Board of Multi-turn LLM Agents", https://arxiv.org/abs/2401.13178 | Agent evaluation benefits from process-level metrics, not only final success. Doctor may treat absence of phase/progress/proof evidence as an execution-risk signal. |
| `tiktoken_token_accounting` | tooling documentation | OpenAI tiktoken project, https://github.com/openai/tiktoken and OpenAI Cookbook token counting guide, https://cookbook.openai.com/examples/how_to_count_tokens_with_tiktoken | Token counts should be measured with model/encoding-aware tokenization rather than byte or word estimates when judging prompt/context load. |
| `claude_token_counting` | vendor documentation | Anthropic token counting docs, https://platform.claude.com/docs/en/build-with-claude/token-counting | Claude token counts are model-dependent and can be counted with Anthropic's token-counting endpoint before sending a message. The endpoint should be preferred over generic tokenizers for Claude load. |
| `anthropic_context_management` | vendor documentation | Anthropic context-window and compaction docs, https://platform.claude.com/docs/en/build-with-claude/context-windows and https://platform.claude.com/docs/en/build-with-claude/compaction | As conversations grow, token count and recall/focus degrade; compaction summarizes older context near configured thresholds. Doctor may treat proximity to compaction as context-management risk. |
| `governance_decay_compaction` | research paper | Chen, 2026, "Governance Decay: How Context Compaction Silently Erases Safety Constraints in Long-Horizon LLM Agents", https://arxiv.org/abs/2606.22528 | In long-horizon agent settings, compaction/summarization can drop governance constraints and increase prohibited-tool-action violations. Doctor may treat compaction proximity as risk for load-bearing safety/permission instructions unless constraints are pinned in durable contract form. |
| `claude_skill_frontmatter_discovery` | harness documentation | Claude Code skills documentation, https://code.claude.com/docs/en/skills | Claude Code uses `SKILL.md` frontmatter, especially `description` and `when_to_use`, to decide when to load a skill automatically. Listing budgets can shorten or drop descriptions. Doctor may treat weak, missing, malformed, overbroad, or budget-truncated frontmatter as discovery risk. |
| `skilldex_format_conformance` | research paper | Skilldex, "A Package Manager and Registry for Agent Skill Packages with Hierarchical Scope-Based Distribution", https://arxiv.org/abs/2604.16911 | Skill-package tooling can score packages against Anthropic-style format specs and provide diagnostics on description specificity, frontmatter validity, and structural adherence. |
| `skill_metadata_supply_chain` | research paper | Saha et al., 2026, "Under the Hood of SKILL.md: Semantic Supply-chain Attacks on AI Agent Skill Registry", https://arxiv.org/abs/2605.11418 | Natural-language skill metadata and instructions affect which skills are admitted, surfaced, selected, and loaded. Doctor may treat frontmatter as operational routing text, not passive documentation. |
| `skillspec_local_contract_trace` | local methodology | [docs/08-contract-trace-methodology.md](../08-contract-trace-methodology.md) | SkillSpec-specific engineering inference: route choice, forbids, dependencies, tool boundaries, tests, and trace/progress proof are checkable surfaces that reduce drift. |
| `skillspec_local_reliability_gap` | local methodology | [docs/00-skills-reliability-gap.md](../00-skills-reliability-gap.md) | SkillSpec-specific engineering inference: large activation bodies, implicit dependencies, mixed code/instructions, missing contracts, and missing proof surfaces create reliability debt. |

## Citation Rule

Each condition in the report must include:

```json
{
  "basis_ids": ["lost_middle_position_effect", "ruler_effective_context"],
  "claim_scope": "directional_risk_not_probability",
  "threshold_source": "skillspec_policy_v0"
}
```

This means:

- `basis_ids` explain why the condition matters;
- `claim_scope` prevents overclaiming;
- `threshold_source` explains where the cutoff came from.

No condition should imply a paper proved a specific SkillSpec threshold unless
that threshold was actually measured and published.

## CLI Shape

Current command:

```sh
skillspec doctor <target> --json
```

Designed extensions:

```sh
skillspec doctor <target> \
  --json \
  --tokenizer-profile auto \
  --model claude-sonnet-4-6 \
  --harness-profile claude-code \
  --current-session-tokens 120000
```

Designed options:

- `--tokenizer-profile <profile>`: token accounting strategy. Values:
  `auto`, `codex`, `claude`, `dual`, `tiktoken`, `estimated`.
- `--model <name>`: optional model name used to choose tokenizer and
  context-window profile.
- `--encoding <name>`: explicit tiktoken encoding. Overrides model mapping for
  OpenAI/Codex-style estimates.
- `--harness-profile <profile>`: optional harness policy profile. Values:
  `codex`, `claude-code`, `agents`, `api`, `unknown`.
- `--current-session-tokens <n>`: tokens already present before this skill
  loads. Without it, doctor reports package load risk and hypothetical
  compaction thresholds, not actual current-session compaction pressure.
- `--context-window-tokens <n>`: optional override for total usable context
  window. This should not be required for normal use; doctor should keep a
  versioned built-in profile for common models/harnesses.
- `--reserved-output-tokens <n>`: expected output/tool-result reserve.
- `--compaction-threshold <ratio>`: harness-specific compaction warning point,
  such as `0.85` or `0.90`. This is environment policy, not paper-derived.
- `--risk-profile <profile>`: optional preset. Values: `conservative`,
  `balanced`, `strict`.

If token options are absent, doctor should use `--tokenizer-profile auto`.
Auto means:

1. If `--model` is a known Claude model, use Claude token counting when the
   endpoint is available; otherwise use a Claude estimate profile and mark
   `fallback_used: true`.
2. If `--model` is a known OpenAI/Codex model, use tiktoken with the mapped
   encoding.
3. If no model is supplied, compute a `dual` report: one Codex/OpenAI-style
   tiktoken estimate and one Claude-style estimate. The risk score should use
   the higher active-load token count unless the caller selects a profile.

When current-session tokens are absent, `session_compaction_proximity` should
not claim the current turn is close to compaction. It should instead report the
headroom needed:

```text
This skill activates with 8,482 tokens. If the current session is already above
159,518 / 200,000 tokens under the selected profile, this activation plus the
reserved output budget can cross a 90% compaction threshold.
```

## Token Accounting Model

Doctor should stop using approximate token counts as the primary metric once a
tokenizer is available.

### Token Fields

```json
{
  "token_accounting": {
    "tokenizer": {
      "profile": "dual",
      "selected_for_score": "max_active_load",
      "estimates": [
        {
          "name": "codex",
          "source": "tiktoken",
          "encoding": "o200k_base",
          "model": null,
          "fallback_used": false
        },
        {
          "name": "claude",
          "source": "anthropic_count_tokens_or_estimate",
          "encoding": null,
          "model": null,
          "fallback_used": true
        }
      ]
    },
    "package_tokens": {
      "frontmatter": 142,
      "activation_body": 8340,
      "referenced_markdown": 27600,
      "code_blocks_in_skill": 3180,
      "unmapped_files": 9100,
      "total_package": 48362
    },
    "load_tokens": {
      "startup_discovery": 142,
      "on_activation": 8482,
      "deferred": 39880
    },
    "context_pressure": {
      "context_window_tokens": 200000,
      "context_window_source": "model_profile_registry",
      "current_session_tokens": null,
      "current_session_source": "not_provided",
      "reserved_output_tokens": 12000,
      "tokens_after_activation": null,
      "load_ratio_after_activation": null,
      "compaction_threshold": 0.9,
      "status": "hypothetical",
      "activation_crosses_threshold_if_session_at_or_above": 159518
    }
  }
}
```

### Token Definitions

- `startup_discovery`: frontmatter tokens visible before activation.
- `on_activation`: frontmatter plus active `SKILL.md` body tokens.
- `deferred`: referenced files/resources not loaded unless the agent opens
  them.
- `tokens_after_activation`: current session + on-activation skill load +
  reserved output.
- `load_ratio_after_activation`: `tokens_after_activation / context_window`.
- `tokens_until_compaction_threshold`: threshold budget minus
  `tokens_after_activation`.
- `activation_crosses_threshold_if_session_at_or_above`: current-session token
  count at which loading this skill would cross the configured compaction
  threshold.

## Frontmatter Discovery Risk

Doctor treats `SKILL.md` frontmatter as a routing surface, not just metadata.

A weak description can make a useful skill hard to discover. A broad
description can make a skill trigger too often. Malformed or hidden metadata
can make a skill slash-invocable while removing the description signal the
harness uses for automatic selection.

### Frontmatter Baseline

There is no universal frontmatter contract across every harness. Doctor
normalizes a small common core and then adds harness-specific fields.

Common baseline:

- `name`: human/display identity; often defaults to the directory name when
  absent.
- `description`: short description of what the skill does and when to use it.
  This is the key discovery field and should be treated as recommended even
  when a harness technically allows omission.

Claude Code fields doctor should understand when present:

- `when_to_use`: extra trigger context appended to `description`;
- `disable-model-invocation`: removes the skill from automatic model
  invocation when true;
- `user-invocable`: controls slash-menu visibility;
- `allowed-tools` and `disallowed-tools`: tool-surface hints that can affect
  execution-risk scoring;
- `context` and `agent`: execution-context hints that affect load and proof
  expectations.

Codex/OpenAI-style installed skills commonly use `name` plus `description`.
Doctor should not assume every Claude-only field is portable, but it should
preserve and report those fields when analyzing a Claude target.

### Measurements

For every discovered `SKILL.md`, doctor should emit:

```json
{
  "frontmatter_discovery_risk": {
    "score": 42,
    "level": "medium",
    "fields": {
      "name": "oss-review",
      "description": "Review OSS.",
      "when_to_use": null,
      "disable_model_invocation": false,
      "user_invocable": true,
      "description_chars": 11,
      "description_tokens": 4,
      "combined_discovery_chars": 11,
      "combined_discovery_tokens": 4,
      "harness_cap_chars": 1536,
      "harness_profile": "claude-code"
    },
    "conditions": []
  }
}
```

Candidate measurements:

- `description_chars` and `description_tokens`;
- `combined_discovery_chars` and `combined_discovery_tokens` for
  `description + when_to_use`;
- `harness_cap_chars`;
- `domain_term_count`, `action_term_count`, and `trigger_phrase_count`;
- `body_heading_overlap`;
- `generic_term_ratio`;
- `manual_only`;
- `visibility_state`, such as `on`, `name-only`, `user-invocable-only`, or
  `off`.

### Frontmatter Conditions

Example:

```json
{
  "id": "ambiguous_short_description",
  "kind": "discovery_risk",
  "level": "medium",
  "score_delta": 8,
  "confidence": "medium",
  "measurement": {
    "description_chars": 11,
    "domain_term_count": 1,
    "action_term_count": 1,
    "trigger_phrase_count": 0,
    "generic_term_ratio": 0.5
  },
  "basis_ids": [
    "claude_skill_frontmatter_discovery",
    "skilldex_format_conformance",
    "skill_metadata_supply_chain"
  ],
  "claim_scope": "discovery_risk_not_observed_routing_failure",
  "threshold_source": "skillspec_policy_v0",
  "consequence": "The harness has little specific text to match against user requests, so automatic skill discovery may be unreliable.",
  "recommended_action": "Rewrite the description with the primary domain, action, and natural trigger phrases first."
}
```

Doctor should distinguish:

- `false_negative_discovery_risk`: the skill may not be selected when it should
  be selected;
- `false_positive_discovery_risk`: the skill may trigger for work it should not
  own;
- `listing_budget_risk`: discovery text may be shortened, dropped, or reduced
  to name-only in a crowded skill environment;
- `manual_invocation_only`: automatic routing is intentionally unavailable.

This section should not claim that a short description definitely fails. It
should say selection evidence is weak and cite the frontmatter/discovery bases.

## Workspace And Folder Shape Reporting

When the target is a folder, doctor classifies shape before computing drift
risk. The classification step prevents a workspace from being mistaken for one
giant skill and prevents plugin skills from losing namespace identity.

Detection order:

1. `simple_skill`: the target itself contains one atomic `SKILL.md` package.
2. `entry_skill_with_subskills`: the target contains a root `SKILL.md` plus
   nested `SKILL.md` files referenced by the root or known skill folders.
3. `multi_skill_workspace`: the target contains multiple sibling or nested
   skill packages without a plugin manifest tying them into one plugin.
4. `plugin_workspace`: the target has plugin structure, such as a plugin
   manifest and a `skills/` directory.
5. `non_skill_repository`: no plausible `SKILL.md` packages are found. Report
   shape and stop before expensive processing.

For `simple_skill`, doctor emits one package report.

For `entry_skill_with_subskills`, `multi_skill_workspace`, and
`plugin_workspace`, doctor emits:

- one aggregate workspace report;
- one package report per discovered `SKILL.md`;
- frontmatter discovery risk for every package;
- agent drift risk for every package;
- name/namespace collision checks;
- cross-skill reference checks;
- cycle checks where one skill references another package;
- aggregate recommendations that preserve the original folder shape.

Doctor must not flatten package names. Each package report should carry stable
identity fields:

```json
{
  "package_id": "claude-for-legal:privacy/cold-start-interview",
  "public_name": "cold-start-interview",
  "plugin_name": "claude-for-legal",
  "install_slug": "claude-for-legal__privacy__cold-start-interview",
  "path": "privacy/cold-start-interview/SKILL.md",
  "shape_role": "plugin_skill",
  "entrypoint": "SKILL.md"
}
```

Aggregate report shape:

```json
{
  "target": "./skills",
  "shape": {
    "kind": "multi_skill_workspace",
    "package_count": 12,
    "entry_packages": ["coding-standards"],
    "referenced_skill_packages": ["testing", "docs-style"],
    "non_skill_files_examined": 38
  },
  "workspace_agent_drift_risk": {
    "score": 68,
    "level": "high",
    "summary": "Workspace has many atomic skills with cross references and several weak discovery descriptions.",
    "conditions": [
      {
        "id": "workspace_cross_skill_reference_risk",
        "kind": "workspace_aggregate_risk",
        "level": "high",
        "basis_ids": ["skillspec_local_reliability_gap"],
        "claim_scope": "static_workspace_shape_risk",
        "threshold_source": "skillspec_policy_v0"
      }
    ]
  },
  "packages": [
    {
      "package_id": "coding-standards",
      "path": "coding-standards/SKILL.md",
      "frontmatter_discovery_risk": {},
      "agent_drift_risk": {}
    }
  ]
}
```

If the workspace is large, text output should summarize the highest-risk
packages and write the full JSON to disk. JSON output should include every
package unless the caller passes an explicit limit such as `--max-packages`.

## Contract Mitigation

Doctor distinguishes raw prose risk from structured contract mitigation.

Raw activation risk asks:

```text
How risky is the activated SKILL.md body if the agent follows only prose?
```

Contract mitigation asks:

```text
Does this package have a valid SkillSpec contract that moves load-bearing
behavior into routes, rules, dependencies, commands, tests, progress, and
alignment proof?
```

Current mitigation fields:

```json
{
  "raw_activation_risk": {
    "score": 18,
    "level": "low"
  },
  "contract_mitigation": {
    "present": true,
    "valid": true,
    "level": "strong",
    "routes": 20,
    "rules": 32,
    "commands": 77,
    "dependencies": 5,
    "tests": 41,
    "residual_risk_score": 0,
    "residual_risk_level": "low"
  }
}
```

Mitigation should not erase all risk. It should reduce risk only when the
contract is valid and contains meaningful checkable surfaces:

- routes;
- rules;
- command templates;
- dependencies;
- scenario tests;
- progress/alignment proof expectations.

An invalid `skill.spec.yml` is not mitigation. It should produce
`invalid_behavior_contract` and high risk until fixed.

## Risk Direction And Levels

`agent_drift_risk.score` is risk, not quality:

- `0-24`: low;
- `25-49`: medium;
- `50-74`: high;
- `75-100`: critical.

These bands are SkillSpec policy v0. They are not paper-derived
probabilities. JSON must say so:

```json
{
  "threshold_source": "skillspec_policy_v0"
}
```

The legacy `structural_score` can remain:

```text
structural_score = 100 - agent_drift_risk.score
```

Only do this when both scores use the same condition set. Otherwise keep
`structural_score` as a compatibility field and document that it is not the
inverse of drift risk.

## Condition Model

Each condition must be self-explaining and machine-readable:

```json
{
  "id": "buried_load_bearing_obligations",
  "kind": "position_risk",
  "level": "high",
  "score_delta": 16,
  "confidence": "medium",
  "measurement": {
    "late_obligation_count": 14,
    "middle_band_obligation_count": 9,
    "activation_body_tokens": 8340,
    "line_percent_threshold": 60
  },
  "evidence": [
    {
      "path": "SKILL.md",
      "line": 421,
      "text_preview": "Never install dependencies without asking first."
    }
  ],
  "basis_ids": ["lost_middle_position_effect"],
  "claim_scope": "directional_risk_not_probability",
  "threshold_source": "skillspec_policy_v0",
  "consequence": "Important safety or sequencing instructions may be missed when the agent reads the full body.",
  "recommended_action": "Promote late obligations into early route summaries, rules, forbids, elicitations, and tests."
}
```

## Condition Registry

Every condition has at least one external research basis or an explicit local
methodology basis. Conditions with only local basis must be marked
`experimental` and should not be presented as externally validated.

| Condition ID | Kind | Trigger | Basis IDs | Consequence | Recommendation |
| --- | --- | --- | --- | --- | --- |
| `activation_token_load` | `context_pressure` | Active `SKILL.md` load exceeds policy bands, e.g. >5k, >10k, >20k tokens. | `ruler_effective_context`, `tiktoken_token_accounting`, `skillsbench_focused_skills` | Agent spends more context on inactive or low-priority detail and may follow fewer active constraints. | Move examples/references/code into deferred files; keep active body as router plus critical rules. |
| `session_compaction_proximity` | `context_pressure` | `tokens_after_activation / context_window` approaches configured compaction threshold. | `ruler_effective_context`, `tiktoken_token_accounting`, `claude_token_counting`, `anthropic_context_management`, `governance_decay_compaction` | Loading near compaction may force summarization, omit safety constraints, or leave too little budget for proof. | Use source/query handles, defer references, pin governance constraints in SkillSpec, or start a fresh run. |
| `missing_or_malformed_frontmatter` | `discovery_risk` | Frontmatter is absent, cannot be parsed, or lacks a usable discovery field. | `claude_skill_frontmatter_discovery`, `skilldex_format_conformance`, `skill_metadata_supply_chain` | Skill may be slash-invocable but weak or invisible for automatic discovery. | Fix YAML frontmatter and provide a specific `description` with the main use case first. |
| `ambiguous_short_description` | `discovery_risk` | Description is short and lacks domain terms, action terms, or trigger phrases. | `claude_skill_frontmatter_discovery`, `skilldex_format_conformance`, `skill_metadata_supply_chain` | Harness has too little specific text to match natural user requests. | Rewrite description with domain, action, object, and trigger phrases. |
| `overbroad_description` | `discovery_risk` | Description uses generic verbs or wide scope without clear boundaries. | `claude_skill_frontmatter_discovery`, `skilldex_format_conformance`, `skill_metadata_supply_chain` | Skill may trigger for work it should not own. | Add boundaries: what it handles, what it does not, and when to ask. |
| `description_listing_budget_risk` | `discovery_risk` | Combined discovery text exceeds harness cap or crowded listing budget. | `claude_skill_frontmatter_discovery`, `tiktoken_token_accounting` | Keywords needed for discovery may be truncated or removed. | Put key trigger first, trim low-value wording, or reduce low-priority skills to name-only. |
| `manual_only_visibility` | `discovery_risk` | Frontmatter/settings disable automatic invocation or reduce listing visibility. | `claude_skill_frontmatter_discovery` | Automatic discovery may be intentionally unavailable. | Report as informational unless the goal requires automatic routing. |
| `middle_or_late_obligation_position` | `position_risk` | Load-bearing obligation appears after configured line/token percent. | `lost_middle_position_effect` | Important instructions may be less reliably used than instructions near beginning/end. | Promote obligations to early summaries or structured rules/tests. |
| `constraint_density` | `instruction_following_risk` | Many `must`, `never`, `ask`, `check`, `run`, numbered steps, or nested conditionals in active body. | `ifeval_verifiable_instructions`, `skillsbench_focused_skills` | Agent must satisfy many independent constraints from memory, and omissions may be invisible. | Convert constraints into rules, elicitations, checks, tests, and trace obligations. |
| `comprehensive_body_instead_of_focused_skill` | `skill_design_risk` | Broad documentation/reference material dominates active body. | `skillsbench_focused_skills` | A broad manual may underperform a focused task skill despite containing more information. | Split into focused skills or deferred references. |
| `implicit_tool_or_dependency_contract` | `execution_risk` | Active body mentions tools, commands, installs, APIs, auth, env vars, or packages without declared deps. | `agentboard_process_metrics`, `skillspec_local_reliability_gap`, `skillspec_local_contract_trace` | Agent may choose unavailable tools, skip preflight, or fail without degraded-proof explanation. | Add `deps.toml`, SkillSpec dependencies, command `requires`, and degraded proof policy. |
| `code_mixed_with_activation_instructions` | `execution_risk` | Fenced code/scripts are embedded in active prose without classification. | `skillsbench_focused_skills`, `skillspec_local_reliability_gap` | Agent may execute example code, ignore executable code, or confuse instructions with artifacts. | Move code to resources/code entries and label executable vs example vs reference. |
| `missing_behavior_contract` | `proof_gap` | No `skill.spec.yml` exists for a skill with operational prose. | `ifeval_verifiable_instructions`, `skillspec_local_contract_trace` | Route choices, forbids, tools, dependencies, and success criteria are not falsifiable. | Create a SkillSpec contract before relying on the skill for important workflows. |
| `missing_scenario_tests` | `proof_gap` | No scenario tests or equivalent verifier exists. | `ifeval_verifiable_instructions`, `skillsbench_focused_skills` | Author cannot prove the skill routes and constrains behavior as intended. | Add decision tests for route, matched rules, forbids, elicitations, and closures. |
| `missing_progress_or_trace_surface` | `proof_gap` | No trace/progress/evidence requirements for execution. | `agentboard_process_metrics`, `skillspec_local_contract_trace` | A final answer may claim success without proof that required steps happened. | Add progress requirements and alignment checks. |
| `workspace_shape_mismatch` | `shape_risk` | Multiple `SKILL.md` files, entry-with-subskills, plugin roots, or cross refs are passed as one skill. | `skillsbench_focused_skills`, `skillspec_local_reliability_gap` | Agent may flatten packages, lose namespace identity, or treat another skill as a resource. | Run `skillspec workspace map` and process atomic packages separately. |
| `workspace_cross_skill_reference_risk` | `workspace_aggregate_risk` | One skill package references another without explicit SkillSpec dependency edge. | `skillspec_local_reliability_gap`, `skillspec_local_contract_trace` | Agents may load referenced skill as loose resource, recurse into circular references, or miss upstream standards. | Preserve package identity and connect packages with explicit dependencies. |
| `workspace_name_collision_risk` | `workspace_aggregate_risk` | Multiple packages normalize to the same public name or install slug. | `claude_skill_frontmatter_discovery`, `skillspec_local_reliability_gap` | Wrong skill may be surfaced or installed over another skill. | Use namespace-preserving package ids and collision-resistant install slugs. |
| `unresolved_local_references` | `source_integrity_risk` | Markdown references point to missing local files. | `skillspec_local_contract_trace` | Active skill may instruct agent to load missing guidance or silently skip it. | Fix links or preserve missing files before import/install. |

## Condition Kinds

If `kind` is an enum, every value must have a basis category:

```json
{
  "condition_kind_registry": {
    "context_pressure": {
      "basis_ids": [
        "ruler_effective_context",
        "lost_middle_position_effect",
        "tiktoken_token_accounting",
        "claude_token_counting",
        "anthropic_context_management",
        "governance_decay_compaction"
      ]
    },
    "discovery_risk": {
      "basis_ids": [
        "claude_skill_frontmatter_discovery",
        "skilldex_format_conformance",
        "skill_metadata_supply_chain"
      ]
    },
    "position_risk": {
      "basis_ids": ["lost_middle_position_effect"]
    },
    "instruction_following_risk": {
      "basis_ids": ["ifeval_verifiable_instructions", "skillsbench_focused_skills"]
    },
    "skill_design_risk": {
      "basis_ids": ["skillsbench_focused_skills"]
    },
    "execution_risk": {
      "basis_ids": ["agentboard_process_metrics", "skillspec_local_contract_trace"]
    },
    "proof_gap": {
      "basis_ids": [
        "ifeval_verifiable_instructions",
        "agentboard_process_metrics",
        "skillsbench_focused_skills"
      ]
    },
    "shape_risk": {
      "basis_ids": ["skillsbench_focused_skills", "skillspec_local_reliability_gap"]
    },
    "workspace_aggregate_risk": {
      "basis_ids": [
        "claude_skill_frontmatter_discovery",
        "skillspec_local_reliability_gap",
        "skillspec_local_contract_trace"
      ]
    },
    "source_integrity_risk": {
      "basis_ids": ["skillspec_local_contract_trace"]
    }
  }
}
```

`level` is a policy enum:

```json
{
  "risk_level_registry": {
    "low": {"score_range": [0, 24], "threshold_source": "skillspec_policy_v0"},
    "medium": {"score_range": [25, 49], "threshold_source": "skillspec_policy_v0"},
    "high": {"score_range": [50, 74], "threshold_source": "skillspec_policy_v0"},
    "critical": {"score_range": [75, 100], "threshold_source": "skillspec_policy_v0"}
  }
}
```

The report must not say a paper proved these exact bands.

## Obligation Extraction

Doctor identifies load-bearing obligations in active prose. Start with static
extraction and improve later.

Signals:

- modal verbs: `must`, `should`, `never`, `always`, `only`, `do not`, `avoid`;
- execution verbs: `run`, `install`, `fetch`, `open`, `click`, `write`,
  `delete`, `deploy`, `publish`, `commit`, `push`;
- proof verbs: `check`, `verify`, `validate`, `test`, `record`, `report`;
- permission verbs: `ask`, `confirm`, `approve`, `permission`;
- sequence markers: numbered lists, `before`, `after`, `then`, `finally`;
- conditionals: `if`, `unless`, `when`, `except`.

Each obligation should carry position metadata:

```json
{
  "obligation_id": "obl_042",
  "kind": "forbid",
  "text": "Do not install dependencies without asking first.",
  "path": "SKILL.md",
  "line_start": 421,
  "token_start": 6140,
  "token_end": 6152,
  "body_line_percent": 72.4,
  "body_token_percent": 68.1,
  "position_band": "late",
  "basis_ids": ["lost_middle_position_effect"]
}
```

Position bands:

- `early`: 0-20%;
- `early_middle`: 20-40%;
- `middle`: 40-60%;
- `late_middle`: 60-80%;
- `late`: 80-100%.

Band names are SkillSpec policy. The reason position matters is backed by
`lost_middle_position_effect`.

## Scoring Model

The first version uses additive weighted scoring with caps. It is explainable
and easy to calibrate later.

```text
risk = min(100,
  activation_token_load
  + session_compaction_proximity
  + middle_or_late_obligation_position
  + constraint_density
  + implicit_tool_or_dependency_contract
  + missing_behavior_contract
  + missing_scenario_tests
  + missing_progress_or_trace_surface
)
```

Each condition produces:

- raw measurement;
- policy band;
- score delta;
- basis ids;
- threshold source.

Suggested initial caps:

| Group | Max Contribution |
| --- | ---: |
| context/token pressure | 25 |
| frontmatter/discovery risk | 20 |
| position risk | 20 |
| instruction/constraint density | 20 |
| execution ambiguity | 20 |
| proof gap | 25 |
| shape/source integrity/workspace risk | 20 |

The total still caps at 100.

## Human Report Shape

Text output should be short and operator-readable:

```text
skillspec doctor: ./my-skill
shape: simple_skill
analysis: full

Agent drift risk: high (74/100)
Structural score: 26/100

Why:
- frontmatter description is short and generic, so automatic discovery may be unreliable
- active skill load is 8,482 tokens; this is above the balanced policy target
- 14 required/forbidden obligations appear after 60% of the active body
- tool/API commands are mentioned, but dependencies are not declared
- no scenario tests or trace/progress proof surface was found

Likely consequence:
An agent may follow the broad task but skip a late safety gate, use an
undeclared tool, or claim completion without evidence.

Token pressure:
- startup discovery: 142 tokens
- on activation: 8,482 tokens
- current session: not provided
- selected context profile: 200,000 tokens from model profile registry
- compaction threshold: 90.0%
- activation would cross threshold if the current session is already at or above
  159,518 tokens

Recommended next step:
Run `skillspec source map`, then port the skill into a SkillSpec contract before
installing it for important workflows.
```

## JSON Report Shape

Top-level shape:

```json
{
  "target": "./my-skill",
  "shape": {"kind": "simple_skill"},
  "structural_score": 26,
  "frontmatter_discovery_risk": {},
  "agent_drift_risk": {
    "schema": "skillspec.doctor.agent_drift_risk.v0",
    "score": 74,
    "level": "high",
    "threshold_source": "skillspec_policy_v0",
    "summary": "High risk that an agent reading this skill as prose will skip late obligations, use undeclared tools, or report success without proof.",
    "recommended_mode": "port_to_skillspec_before_install",
    "conditions": [],
    "basis_registry": []
  },
  "raw_activation_risk": {},
  "contract_mitigation": {},
  "workspace_agent_drift_risk": null,
  "packages": []
}
```

Condition entries should be complete enough for CI gates and dashboards:

```json
{
  "id": "activation_token_load",
  "kind": "context_pressure",
  "level": "high",
  "score_delta": 18,
  "measurement": {
    "activation_body_tokens": 8340,
    "policy_target_tokens": 5000
  },
  "basis_ids": [
    "ruler_effective_context",
    "tiktoken_token_accounting",
    "skillsbench_focused_skills"
  ],
  "claim_scope": "directional_risk_not_probability",
  "threshold_source": "skillspec_policy_v0",
  "consequence": "The agent must carry a large active instruction body before doing task work.",
  "recommended_action": "Move examples, references, and detailed procedures into deferred files or structured SkillSpec entries."
}
```

## Implementation Plan

### Phase 1: Report Model

- Add `AgentDriftRiskReport`.
- Add `TokenAccountingReport`.
- Add `FrontmatterDiscoveryRiskReport`.
- Preserve current `DoctorReport` fields.
- Compute risk from existing doctor signals first.
- Add basis registry and condition registry.
- Keep approximate tokens if tiktoken is not available, but mark fallback
  clearly.

### Phase 2: Tokenization

- Add a Rust tokenizer dependency or internal adapter for OpenAI/Codex-style
  estimates.
- Add a Claude token-counting adapter. Prefer Anthropic's count-tokens endpoint
  when credentials are available; otherwise use a documented Claude estimate
  profile and mark `fallback_used: true`.
- Prefer model-to-tokenizer/context-window mapping when known.
- Support explicit `--encoding` and `--tokenizer-profile`.
- Record tokenizer source/version in JSON.
- Add tests with fixed text and expected token counts for selected encodings.

### Phase 3: Frontmatter Discovery Risk

- Parse `SKILL.md` frontmatter with structured YAML parsing, not regex.
- Preserve parse errors as `missing_or_malformed_frontmatter` evidence.
- Normalize common fields: `name`, `description`, and harness-specific fields.
- Compute description length, token count, trigger specificity, generic-term
  ratio, heading overlap, and listing-budget risk.
- Emit `frontmatter_discovery_risk` for every analyzed package.
- Add policy thresholds as `skillspec_policy_v0`.

### Phase 4: Obligation Extraction

- Extract modal, execution, permission, and proof obligations from active
  `SKILL.md`.
- Record line/token positions.
- Add position-band counts.
- Flag middle/late load-bearing obligations.

### Phase 5: Context And Compaction Inputs

- Add CLI flags for current session tokens, reserved output, compaction
  threshold, model, harness profile, tokenizer profile, and context-window
  override.
- Maintain a versioned model/harness profile registry.
- If session inputs are missing, report package-local risk and a hypothetical
  crossing threshold rather than actual current-session compaction risk.
- If inputs are present, compute `tokens_after_activation` and threshold margin.

### Phase 6: Folder Shape And Workspace Reports

- Run shape classification before risk scoring.
- For `simple_skill`, emit one package report.
- For workspace shapes, discover every `SKILL.md`, emit per-package reports,
  and include aggregate workspace risk.
- Preserve namespace identity for plugin and nested skills.
- Detect name collisions, install-slug collisions, cross-skill references,
  missing references, and circular package references.
- Keep non-skill repositories shape-only unless the caller explicitly asks for
  a broader code-repo scan.

### Phase 7: Calibration

- Collect real execution traces from prose skills and SkillSpec-backed ports.
- Compare doctor risk conditions against observed:
  - skipped phases;
  - skipped requirements;
  - forbidden action attempts;
  - missing dependency proof;
  - failed alignment;
  - final-response proof gaps.
- Tune weights and bands.

Calibration should produce `skillspec_policy_v1`. Until then, v0 bands are
heuristics with cited directional basis.

## Tests

Required tests:

- simple skill with small body -> low risk;
- large body over token threshold -> context condition;
- absent model -> dual Codex/OpenAI and Claude token estimate;
- malformed frontmatter -> `missing_or_malformed_frontmatter`;
- missing or empty `description` -> frontmatter discovery risk;
- short generic description -> `ambiguous_short_description`;
- broad description with no boundaries -> `overbroad_description`;
- long `description + when_to_use` near or above listing cap ->
  `description_listing_budget_risk`;
- late `must/never/ask/check` obligations -> position condition;
- active body with tools/commands but no deps -> implicit dependency condition;
- skill with `skill.spec.yml`, tests, deps -> lower proof-gap risk;
- simple skill folder -> one package risk report;
- multi-skill folder -> aggregate report plus per-`SKILL.md` package reports;
- plugin folder -> namespace-preserving aggregate report plus per-plugin-skill
  reports;
- non-skill repository -> shape-only result and no expensive package scoring;
- name/install-slug collision -> `workspace_name_collision_risk`;
- cross-skill package reference -> `workspace_cross_skill_reference_risk`;
- token CLI flags compute compaction margin;
- no current-session token input reports hypothetical crossing threshold;
- all conditions include non-empty `basis_ids`;
- all basis ids resolve to `basis_registry`;
- all policy thresholds include `threshold_source`.

## Standardization Requirements

For doctor to become a reliable standard:

1. The report must be deterministic for the same source tree and token options.
2. Every condition must include evidence, measurement, basis ids, and threshold
   source.
3. Research citations must be narrow and must not overclaim exact thresholds.
4. Policy thresholds must be versioned.
5. Token accounting must state tokenizer, encoding, and fallback status.
6. Static risk must be separate from observed runtime failure.
7. Frontmatter discovery risk must be reported separately from instruction-body
   drift risk.
8. Folder reports must preserve package identity and must not flatten plugin or
   nested skill names.
9. JSON must remain stable enough for third-party dashboards and CI gates.

## Open Questions

- Which Claude fallback estimate should be used when the Anthropic
  token-counting endpoint is unavailable?
- How should the model/harness profile registry be versioned and updated as
  model context windows change?
- Should `risk_level` bands be configurable per org?
- Should `session_compaction_proximity` be excluded from score when session
  inputs are absent, or scored as unknown risk?
- Should `workspace map` reuse the doctor package-risk model, or should doctor
  call the shared workspace scanner and remain the only risk-reporting command?
- Can doctor scores be benchmarked against SkillsBench-style tasks to calibrate
  v1 thresholds?
