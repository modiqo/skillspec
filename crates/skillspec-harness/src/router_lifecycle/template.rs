use crate::router::RouteHarness;
use std::path::Path;

pub(super) fn render_router_spec(
    router_name: &str,
    index: &Path,
    current_harness: Option<RouteHarness>,
    current_root: Option<&Path>,
) -> String {
    let router_skill = yaml_single_quote(router_name);
    let index_path = shell_single_quote(&index.display().to_string());
    let route_context = route_context_args(current_harness, current_root);
    let route_command = yaml_single_quote(&format!(
        "skillspec route --index {index_path} --query \"<user task>\"{route_context} --top 5 --json"
    ));
    let index_status_command = yaml_single_quote(&format!(
        "skillspec router index status --roots <skill-root>... --index {index_path} --visibility-manifest <manifest> --json"
    ));
    let lifecycle_plan_command = yaml_single_quote(&format!(
        "skillspec router install --roots <skill-root>... --index {index_path} --manifest <manifest> --dry-run --json"
    ));

    format!(
        r#"schema: skillspec/v0
id: skill.router
title: Skill Router
description: Use for every user request when SkillSpec router mode is enabled. First check the local SkillSpec router index, load a selected skill only when the route decision is use_skill, and continue with normal agent behavior when the decision is bypass or ambiguous. The installed SKILL.md is only the native loader; this SkillSpec is the router contract.

activation:
  summary: Use first for every request when router mode is enabled. Route through the local skill index, load a selected skill only when the route decision is use_skill, and otherwise continue with normal agent behavior.
  keywords:
    - every request
    - any request
    - user request
    - tell me
    - explain
    - enlighten
    - what is
    - help with
    - primary skill discovery
    - router first hop
    - skill router
    - route to a skill
    - choose the right skill
    - choose from installed skills
    - select local skill
    - find matching skill
    - many skills installed
    - skill descriptions shortened
    - install router
    - uninstall router
    - enable router
    - disable router
    - refresh skill index
    - out-of-band skill
    - prose skill added
    - skill visibility
    - disable implicit invocation
    - allow_implicit_invocation
    - disable-model-invocation
    - skillOverrides
  priority: broad_router

applies_when:
  - user_intent:
      - handle every user request through the router before skill-specific work
      - route any request that may be handled by an installed local skill
      - decide whether route output is use_skill, bypass, or ambiguous before loading a skill
      - answer a question that may mention an installed skill by name or topic
      - explain, describe, inspect, or use a named local skill
      - choose the correct skill before loading skill-specific instructions
      - select a skill from a large local skill library
      - route a request to the best SkillSpec or SKILL.md package
      - reduce native skill discovery context pressure
      - install or uninstall the SkillSpec router
      - enable or disable router mode without deleting the router package
      - make skills explicit-only, manual-only, implicit, or off
      - refresh a skill index after skill additions or removals
      - detect or repair skills added outside the SkillSpec install flow

entry:
  prompt: If selected implicitly, treat this as the first hop for every request in managed skill roots. For ordinary requests with prompt-hook context first_hop_ready=true, use the SKILL.md fast path; run the route query once, include current harness/root context when available, do not run index status, and do not load this full SkillSpec. Load this SkillSpec for router lifecycle, repair, visibility, guard, index status, index refresh, or missing/failed guard context. Route before reading or searching for domain skill material, then load only when route JSON says decision is use_skill and selected is non-null. Duplicate physical roots collapse to one logical skill before matching; harness/root context only chooses the installed copy. If route JSON says bypass or ambiguous, do not load candidate skills; continue with normal agent behavior.
  decision_required: true
  tool_boundary:
    default: deny
    allow:
      - skillspec_cli
      - local_skill_files
      - local_router_index
      - local_visibility_manifest
    permission_required_for:
      - any_unlisted_tool
      - mutating_visibility_files
      - installing_router_skill
      - deleting_router_skill
      - deleting_router_index

routes:
  - id: route_from_index
    label: Route from the skill index
    rank: 10
    description: Fast dispatch through the local SQLite index. Guard owns freshness; this route only decides whether a candidate skill should run or whether normal agent behavior should continue.
    execution_plan:
      mode: ordered
      phases:
        - id: route_query
          owner_skill: {router_skill}
          description: Query the router index once, including any active SQLite router policy profile, inspect decision, selected, bypass_reason, policy, and candidates, and load a skill only when decision is use_skill.
          requires:
            - run_route_query
        - id: match_gate
          owner_skill: {router_skill}
          description: If decision is use_skill and selected is non-null, load that selected skill explicitly. If decision is bypass, continue normal agent behavior. If decision is ambiguous, do not silently load a candidate; ask only when the user explicitly wanted skill selection, otherwise continue normal behavior.
          requires:
            - apply_route_decision
        - id: execution_mode_elicitation
          owner_skill: {router_skill}
          description: Ask direct versus durable execution only when route output requests it for a use_skill decision and the user has not already chosen.
          requires:
            - ask_direct_or_durable_when_needed

  - id: manage_router_lifecycle
    label: Install, update, refresh, or uninstall router
    rank: 20
    description: Install the router skill into every managed root, manage guard hooks, enable or disable router mode, back up and update recorded router installs, apply visibility, build and verify the index, refresh out-of-band additions, verify guard readiness, or uninstall and restore visibility from the manifest.
    execution_plan:
      mode: ordered
      phases:
        - id: plan_lifecycle_change
          owner_skill: {router_skill}
          description: Show the lifecycle operation, affected roots, index path, manifest path, and restore behavior before mutation.
          requires:
            - show_router_lifecycle_plan
        - id: apply_lifecycle_change
          owner_skill: {router_skill}
          description: Run router install, enable, disable, update, uninstall, guard, index refresh, or index status commands. Install/enable/update manage prompt guard hooks. If install reports that the index path is a legacy router SQLite file blocking the config directory, rerun install with --force only after accepting migration to skill-index.sqlite. Disable removes managed guard hooks, makes the router explicit-only, and restores routed skills to implicit/default without deleting router files.
          requires:
            - run_router_lifecycle_command
        - id: verify_lifecycle_change
          owner_skill: {router_skill}
          description: Verify router skill files, manifest, config, harness_hooks, first_hop_ready, preparedness.ready, and index status after the lifecycle change.
          requires:
            - verify_router_lifecycle_result

  - id: manage_visibility
    label: Manage native skill visibility
    rank: 30
    description: Plan, apply, restore, or explicitly set Codex and Claude native visibility controls.
    execution_plan:
      mode: ordered
      phases:
        - id: visibility_plan
          owner_skill: {router_skill}
          description: Preview native Codex and Claude visibility changes before editing files.
          requires:
            - run_visibility_plan
        - id: visibility_apply_or_restore
          owner_skill: {router_skill}
          description: Apply or restore visibility from a reversible manifest.
          requires:
            - run_visibility_apply_or_restore
        - id: visibility_verify
          owner_skill: {router_skill}
          description: Verify native metadata and router manifest state.

rules:
  - id: route_queries_use_index
    when:
      user_says_any:
        - route to a skill
        - choose the right skill
        - find matching skill
        - many skills installed
        - skill descriptions shortened
    prefer: route_from_index
    reason: Discovery should use the local index instead of loading every skill description into context.

  - id: lifecycle_requests_use_router_commands
    when:
      user_says_any:
        - install router
        - uninstall router
        - enable router
        - disable router
        - router index
        - refresh index
        - index status
        - out-of-band skill
        - prose skill added
    prefer: manage_router_lifecycle
    reason: Router lifecycle changes must write a manifest, config, and index through dedicated commands.

  - id: visibility_requests_use_native_controls
    when:
      user_says_any:
        - set visibility
        - explicit-only
        - manual-only
        - name-only
        - implicit invocation
        - disable implicit
        - skillOverrides
        - disable-model-invocation
        - allow_implicit_invocation
    prefer: manage_visibility
    reason: Visibility changes should use Codex and Claude native controls with a manifest-backed restore path.

elicitations:
  execution_mode_direct_or_durable:
    question: Do you want direct execution or durable execution for this selected skill?
    choices:
      - id: direct
        label: Direct
        sets:
          execution_mode: direct
      - id: durable
        label: Durable
        sets:
          execution_mode: durable
    default: direct

commands:
  inspect_router_index_status:
    description: Compare the router index against current skill roots and report advice for out-of-band prose or SkillSpec-backed skills.
    template: {index_status_command}
    safety: local_read

  run_route_query:
    description: Route the user request to candidate skills from the index. Duplicate physical roots collapse to one logical skill before matching; optional current harness/root context only chooses the installed copy. The JSON decision is authoritative; use_skill loads selected, bypass continues normal behavior, and ambiguous must not silently load a candidate. The router is provider-neutral and does not hardcode execution substrates; selected skills and durable execution own their own tool policy.
    template: {route_command}
    safety: local_read

  apply_route_decision:
    description: Apply route JSON without inventing a match. Load selected only for decision use_skill. For bypass, continue normal agent behavior. For ambiguous, ask only when the user explicitly requested skill choice; otherwise continue normal behavior.
    template: 'inspect route JSON fields: decision, selected, bypass_reason, candidates'
    safety: local_read

  show_router_lifecycle_plan:
    description: Preview router lifecycle changes before writing files.
    template: {lifecycle_plan_command}
    safety: local_read

  run_router_lifecycle_command:
    description: Apply the requested router lifecycle command. Use router install --force only to migrate an accepted legacy SQLite index file into the router config directory; use guard to verify first_hop_ready and repair visibility/index drift; use enable to reapply explicit invocation controls, install guard hooks, and rebuild the index after router mode was disabled; use disable to remove managed guard hooks.
    template: 'skillspec router install [--force]|enable|disable|update|uninstall|guard|index refresh|index status'
    safety: local_write

  run_visibility_plan:
    description: Preview native Codex and Claude visibility changes.
    template: 'skillspec visibility plan --roots <skill-root>... --json'
    safety: local_read

  run_visibility_apply_or_restore:
    description: Apply or restore native visibility controls using a manifest.
    template: 'skillspec visibility apply|restore --manifest <manifest>'
    safety: local_write

tests:
  - name: route query uses index
    input: choose the right skill from many skills installed
    expect:
      route: route_from_index
      matched_rules:
        - route_queries_use_index

  - name: bypass does not force skill
    input: tell me what you changed in this repository
    expect:
      route: route_from_index

  - name: lifecycle uses router commands
    input: enable router and refresh index
    expect:
      route: manage_router_lifecycle
      matched_rules:
        - lifecycle_requests_use_router_commands

  - name: visibility uses native controls
    input: set a noisy skill to explicit-only with allow_implicit_invocation false
    expect:
      route: manage_visibility
      matched_rules:
        - visibility_requests_use_native_controls
"#
    )
}

fn route_context_args(
    current_harness: Option<RouteHarness>,
    current_root: Option<&Path>,
) -> String {
    let mut args = String::new();
    if let Some(harness) = current_harness {
        args.push_str(&format!(" --current-harness {}", harness.as_str()));
    }
    if let Some(root) = current_root {
        args.push_str(&format!(
            " --current-root {}",
            shell_single_quote(&root.display().to_string())
        ));
    }
    args
}

pub(super) fn yaml_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(super) fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}
