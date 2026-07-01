# Shape-Specific Checklist Generation

Status: implemented initial CLI surface
Owner: SkillSpec
Target reader: implementers, reviewers, agent operators, and harness authors

## One-Line Thesis

Checklist commands should turn source shape and current processing state into a
concrete, executable playbook. The activated trampoline should not carry import
policy in prose. It should ask the CLI for a shape-specific checklist and follow
the returned steps until the checklist is complete or blocked.

## Problem

SkillSpec has two related failure modes when importing existing prose skills:

1. The trampoline skill accumulates load-bearing instructions that the agent must
   remember across a long import.
2. Workspace imports can look successful after a mechanical fanout even though
   each generated package is still a scaffold.

The second failure is especially dangerous for multi-skill and plugin-shaped
sources. Each `SKILL.md` may contain its own activation rules, conditional
workflow branches, state-machine language, dependencies, output formats,
handoff rules, examples, configuration paths, and closure checks. A bulk script
that rewrites every YAML file from one template may make validation pass while
dropping the actual behavior.

The fix is not more prose in `SKILL.md`. The fix is generated, entity-specific
checklists that:

- classify the selected source shape;
- preserve the original shape, frontmatter, source files, and namespace;
- name exactly which package, file, or source block is next;
- include exact SkillSpec commands and directives;
- repeat until source-specific coverage is complete;
- fail deterministically when bulk or shallow import behavior appears.

## Command Surface

The implemented command family is:

```bash
skillspec doctor checklist <source> [--stage entry|loop|exit] [--json]
skillspec import checklist <source-or-workspace> [--build-root <dir>] [--stage entry|loop|exit] [--json]
skillspec run checklist <spec-or-run-dir> [--stage entry|loop|exit] [--json]
```

The commands do not replace `doctor`, `source map`, `workspace map`,
`workspace import`, `source lens`, `validate`, `deps check`, `test`, `converge`,
`compile`, `install`, `run-loop`, `progress`, or `trace align`. They generate
the next concrete checklist over those commands.

The checklist command output should be the operating cue. The full source maps,
workspace reports, manifests, promotion proof, and traces remain on disk.

## Generation Model

Checklist generation has four layers:

```text
common invariant template
  + shape-specific template
  + inspected entity facts
  + current stage/cursor/proof state
  = concrete checklist
```

The common invariant template defines what must always be true for safe import
or execution. The shape-specific template defines what the source shape means
for package identity, activation, graph edges, plugin namespaces, and install
layout. The inspected entity facts are discovered from the actual folder,
manifest, source map, build root, or run directory. The current stage decides
whether the checklist is an entry gate, loop gate, or exit gate.

The resulting checklist must not be generic advice. For a concrete source it
should include concrete package ids, paths, source hashes, cursor positions,
remaining counts, exact commands, exact blockers, and required evidence paths.

## Checklist Envelope

Every checklist should have a stable machine-readable shape:

```json
{
  "schema": "skillspec/checklist/v0",
  "kind": "import",
  "stage": "loop",
  "status": "blocked",
  "entity": {
    "root": "/tmp/privacy-legal",
    "shape": "plugin_workspace",
    "manifest": "/tmp/build/skillspec.workspace.yml",
    "build_root": "/tmp/workspace-build"
  },
  "activation_policy": "preserve_plugin_activation",
  "position": {
    "package_index": 3,
    "package_count": 9,
    "remaining_packages": 6,
    "cursor": 4,
    "remaining_source_blocks": 12
  },
  "steps": [],
  "forbid": [],
  "next_command": "skillspec source lens /tmp/workspace-build/source-map/source-map.json --package data-retention --cursor 4"
}
```

The status values should be:

- `ready`: the next command or directive can run.
- `blocked`: the checklist found a condition that must be fixed before
  advancing. It is terminal only when the fix requires user approval,
  credentials, inaccessible source, a policy waiver, or another external state
  change. Mechanical scaffolds, missing promotion proof, unreviewed dependency
  ledgers, and package-local QA failures are continuation gates; the agent must
  return to the named loop command and keep working.
- `complete`: the current stage has no remaining open steps.
- `partial`: the user intentionally accepted a degraded or deferred proof state.

A blocked checklist should return a non-zero exit code when used in normal CLI
mode. JSON mode should still print the blocked report.

## Checklist Step Shape

Each step should include a step description and the accompanying commands or
directives. A step is not just a pass/fail row.

```json
{
  "id": "promote_source_block",
  "description": "Review one source block and map its obligations into structural SkillSpec constructs.",
  "directive": "Read this lens block. Convert conditional language into rules or route phases, dependencies into deps.toml, durable source files into resources or code, and workflow promises into tests or closure checks.",
  "commands": [
    "skillspec source lens <source-map.json> --package <package-id> --cursor <cursor>",
    "skillspec validate <package>/skill.spec.yml",
    "skillspec deps check <package>/skill.spec.yml",
    "skillspec test <package>/skill.spec.yml"
  ],
  "repeat": {
    "for_each": "source_lens_block",
    "until": "source_lens_complete",
    "cursor_field": "next_cursor"
  },
  "requires": [
    "source_hash_recorded",
    "target_kind_matches_obligation",
    "dependency_ledger_reviewed"
  ],
  "blocks": [
    "unmapped_source_obligation",
    "conditional_left_as_prose",
    "state_machine_left_as_summary",
    "dependency_ledger_unreviewed"
  ],
  "forbid": [
    "bulk_yaml_rewrite",
    "representative_package_review",
    "install_generated_scaffold"
  ],
  "evidence": [
    "<package>/.skillspec/workspace-promotion.json",
    "<package>/deps.toml"
  ]
}
```

The important field is `repeat`. It tells the agent exactly what loop it is in:
for each source file, for each parsed block, for each package in manifest
order, for each phase, or until no open requirements remain.

## Common Invariant Template

Every doctor, import, and run checklist should inherit these invariant checks.

### Source Fidelity

Description: The imported behavior must come from the selected source, not from
memory, prior ports, or a representative sibling package.

Directives:

- Stage remote sources locally before import.
- Map source files before semantic promotion.
- Preserve each original `SKILL.md` as source evidence.
- Check stale source hashes before import or promotion.
- Use exact source-map handles for large files or referenced files.

Forbids:

- `consult_existing_ports_without_user_request`
- `consult_repo_history_without_user_request`
- `consult_memory_or_prior_examples_without_user_request`
- `fetch_raw_github_skill_md_for_import`
- `web_search_for_remote_source`
- `claim_source_review_without_source_hashes`

### No Blind Bulk Promotion

Description: Semantic promotion must be package-local and source-backed.

Directives:

- Process packages in manifest order or dependency-ready batches only when
  outputs stay package-scoped.
- For each package, review its own source map, original source, frontmatter,
  dependencies, references, and generated scaffold.
- For each source lens block, map obligations into compatible SkillSpec
  construct kinds.
- Record package-local proof before advancing.

Forbids:

- `bulk_rewrite_skill_specs`
- `bulk_promote_scaffolds`
- `generate_all_packages_from_one_template`
- `review_one_representative_package_only`
- `apply_ruby_yaml_generator_across_packages`
- `apply_python_yaml_generator_across_packages`
- `copy_one_package_semantics_to_siblings`
- `treat_imported_scaffold_as_finished`

Bulk scripting is allowed only for non-semantic mechanical sync, such as copying
already-reviewed canonical files to their packaged mirrors. It is not allowed
for semantic promotion across source skills.

### Original Source And Frontmatter Preservation

Description: Source identity is part of the port. A valid import keeps the
source material and frontmatter available for review.

Directives:

- Preserve frontmatter fields in package metadata or a source metadata section.
- Preserve the original `SKILL.md` text under a deterministic package-local
  path such as `source/SKILL_md.old`.
- Preserve sibling resources, scripts, templates, examples, and referenced
  files as package-local resources or code artifacts.
- Keep plugin and workspace package paths stable enough for proof review.

Forbids:

- `drop_source_frontmatter`
- `drop_original_skill_source`
- `flatten_referenced_files_into_prompt_blob`
- `leave_orphaned_resources`
- `delete_dependency_mentions_to_pass_validation`

### Structural Behavior Mapping

Description: Source behavior must become typed SkillSpec constructs.

Directives:

- Activation language maps to activation, route, route_order, visibility, or
  entry package policy.
- Conditional language such as if, when, unless, only if, except, or provided
  that maps to rules, phase gates, state transitions, or explicit elicitations.
- State-machine or lifecycle language maps to states, route phases, jumps, or
  closure checks.
- Dependency mentions map to `deps.toml` and dependency declarations with
  authority, risk, local status, install candidate, and degraded proof impact.
- External commands and scripts map to commands, code, resources, recipes, or
  artifacts with provenance.
- Output templates and handoff rules map to artifacts, recipes, closures, or
  tests.

Forbids:

- `leave_conditional_workflows_as_prose`
- `omit_state_machine_or_phase_logic`
- `state_machine_as_summary`
- `conditional_behavior_as_comment`
- `claim_full_port_without_source_obligation_coverage`
- `mutate_dependency_set_to_pass_qa`

### QA And Install Gates

Description: A package cannot advance to install until structural proof passes.

Directives:

- Run validate, imports check, dependency check, and tests per package.
- Run workspace converge before workspace compile.
- If converge or compile reports unpromoted scaffolds, return to the generated
  `skillspec import checklist <manifest> --build-root <workspace-build>
  --stage loop --json` command and process the named package until the checklist
  reports complete or a true user-intervention blocker is reached.
- Run workspace compile before workspace install.
- Run workspace install dry-run before mutation.
- Use approved `--retire-existing` for replacement installs so old active
  skills are backed up and removed instead of remaining discoverable.

Forbids:

- `install_generated_scaffold`
- `compile_or_install_during_workspace_import`
- `install_without_workspace_dry_run`
- `leave_old_and_new_skill_discoverable_without_user_choice`
- `refresh_router_during_workspace_install`

## Shape Template: Single SKILL Folder

Shape id: `single_skill_folder`

Detection:

- Exactly one `SKILL.md` exists under the selected source root.
- No plugin marker controls the selected root.
- No workspace manifest is required.

Activation policy: `single_activation_skill`

Preservation policy:

- Keep the source folder as one package.
- Preserve the original `SKILL.md` as package-local source evidence.
- Preserve source frontmatter as package metadata.
- Preserve sibling resources as package-local resources or code.

Entry checklist:

```json
{
  "step": "single_skill_entry",
  "description": "Confirm the selected folder is one atomic source skill.",
  "directive": "Run doctor and source mapping before import. If more than one SKILL.md or a plugin marker is discovered, switch to the matching workspace template.",
  "commands": [
    "skillspec doctor <source> --json",
    "skillspec source map <source> --out <draft>/.skillspec/source-map",
    "skillspec source coverage <draft>/.skillspec/source-map/source-map.json",
    "skillspec source stale <draft>/.skillspec/source-map/source-map.json --root <source>"
  ],
  "repeat": {
    "for_each": "discovered_skill_file",
    "until": "exactly_one_skill_file_verified"
  },
  "blocks": [
    "multiple_skill_files",
    "plugin_marker_detected",
    "source_map_stale"
  ]
}
```

Loop checklist:

```json
{
  "step": "single_skill_source_loop",
  "description": "Review each source block for the one package.",
  "directive": "Use source lens one cursor at a time. Port each block into structural SkillSpec constructs and validate after each meaningful edit.",
  "commands": [
    "skillspec source lens <draft>/.skillspec/source-map/source-map.json --cursor <cursor>",
    "skillspec import-skill <source> --out <draft>/skill.spec.yml --source-map <draft>/.skillspec/source-map/source-map.json",
    "skillspec validate <draft>/skill.spec.yml",
    "skillspec deps check <draft>/skill.spec.yml",
    "skillspec test <draft>/skill.spec.yml"
  ],
  "repeat": {
    "for_each": "source_lens_block",
    "until": "all lens blocks have source-obligation coverage"
  }
}
```

Exit checklist:

```json
{
  "step": "single_skill_exit",
  "description": "Prove the reviewed single package can compile and install.",
  "directive": "Run final QA, compile the reviewed package, dry-run install, then install only after approval.",
  "commands": [
    "skillspec validate <draft>/skill.spec.yml",
    "skillspec deps check <draft>/skill.spec.yml",
    "skillspec test <draft>/skill.spec.yml",
    "skillspec compile <draft>/skill.spec.yml --target codex-skill",
    "skillspec install skill <draft> --target codex --dry-run --retire-existing",
    "skillspec install skill <draft> --target codex --retire-existing"
  ],
  "blocks": [
    "unreviewed_dependency_ledger",
    "unmapped_source_obligation",
    "missing_tests",
    "install_target_collision_without_retirement_approval"
  ]
}
```

## Shape Template: Multi-Skill Folder

Shape id: `multi_skill_folder`

Detection:

- More than one folder below the selected root contains `SKILL.md`.
- Cross-skill references may appear as relative paths, slash-command mentions,
  shared standards packages, or named workflow handoffs.
- No plugin root marker controls the selected root.

Activation policy: `single_workspace_activation`

Preservation policy:

- Each folder containing `SKILL.md` remains a package.
- Each package keeps its own frontmatter and original source.
- Shared folders, standards packages, and referenced files become explicit
  packages, resources, or hard dependency edges.
- The compiled install exposes one user-facing activation skill for the
  ordinary workspace unless a source package is explicitly proven to be an
  independent entrypoint and the user approves exposing it.
- Support packages install as support/manual-only packages or remain
  package-local behind the activation skill, depending on harness capability.

Why one activation skill:

Ordinary multi-skill folders are usually a library or workflow bundle, not a
plugin. If every child folder remains independently discoverable by default,
the harness can route users into helper skills without the workspace context.
The generated activation skill is the entrypoint that knows the workspace graph,
available support packages, handoffs, and shared standards.

Entry checklist:

```json
{
  "step": "multi_skill_entry",
  "description": "Map the source root as one ordinary workspace with one package per SKILL.md folder.",
  "directive": "Run workspace map and validate. Confirm every discovered SKILL.md appears exactly once, every frontmatter name is preserved, and cross-folder references become graph edges or workflow references.",
  "commands": [
    "skillspec doctor <source-root> --json",
    "skillspec workspace map <source-root> --out <build>/skillspec.workspace.yml --summary",
    "skillspec workspace validate <build>/skillspec.workspace.yml --summary"
  ],
  "repeat": {
    "for_each": "discovered_skill_folder",
    "until": "every folder is represented exactly once in the workspace manifest"
  },
  "blocks": [
    "missing_package_for_skill_file",
    "duplicate_install_slug",
    "uncovered_cross_skill_reference",
    "plugin_marker_detected"
  ]
}
```

Loop checklist:

```json
{
  "step": "multi_skill_package_loop",
  "description": "Promote each package from its own source in graph order.",
  "directive": "For the current package, review its own source lens blocks, dependencies, frontmatter, resources, and references. Do not copy semantics from a sibling package. Preserve package-local proof before moving to the next package.",
  "commands": [
    "skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build> --summary",
    "skillspec source lens <workspace-build>/<package>/.skillspec/source-map/source-map.json --cursor <cursor>",
    "skillspec validate <workspace-build>/<package>/skill.spec.yml",
    "skillspec deps check <workspace-build>/<package>/skill.spec.yml",
    "skillspec test <workspace-build>/<package>/skill.spec.yml"
  ],
  "repeat": {
    "outer": "for_each_package_in_manifest_order",
    "inner": "for_each_source_lens_block",
    "until": "all packages have source-obligation coverage and no scaffold markers"
  },
  "forbid": [
    "bulk_promote_scaffolds",
    "review_one_representative_package_only",
    "generate_all_packages_from_one_template"
  ]
}
```

Activation checklist:

```json
{
  "step": "multi_skill_activation",
  "description": "Create or verify one workspace activation package.",
  "directive": "Ensure the ordinary workspace has one user-facing activation path that can reach support packages through structural routes, phases, handoffs, dependencies, or recipes. Support packages must not become default visible entrypoints unless explicitly proven and approved.",
  "commands": [
    "skillspec workspace converge <build>/skillspec.workspace.yml --build-root <workspace-build> --summary",
    "skillspec workspace compile <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex-skill --summary"
  ],
  "blocks": [
    "multiple_default_activation_skills",
    "missing_workspace_activation_skill",
    "support_package_visible_without_policy",
    "cross_skill_reference_not_reachable_from_activation"
  ]
}
```

Exit checklist:

```json
{
  "step": "multi_skill_exit",
  "description": "Install the reviewed workspace after dry-run proof.",
  "directive": "Converge, compile, dry-run install, review visibility and retirement, then install. The dry-run must preserve the source parent folder and write compiled packages back at their original relative paths instead of flattened top-level folders. Router refresh remains separate.",
  "commands": [
    "skillspec workspace converge <build>/skillspec.workspace.yml --build-root <workspace-build> --summary",
    "skillspec workspace compile <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex-skill --summary",
    "skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --retire-existing --dry-run --summary",
    "skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --retire-existing --summary"
  ],
  "blocks": [
    "unpromoted_package_scaffold",
    "missing_workspace_promotion_proof",
    "dependency_not_ready",
    "install_target_collision_without_retirement_approval",
    "flatten_workspace_shape"
  ]
}
```

## Shape Template: Plugin Shape

Shape id: `plugin_shape`

Detection:

- A plugin root marker is present: a plugin-named metadata folder with a
  supported manifest, such as `.agent-plugin/marketplace.json`,
  `.codex-plugin/plugin.json`, `.claude-plugin/plugin.json`, or another
  supported plugin manifest. Compatibility markers such as `.mcp.json` and
  `CLAUDE.md` are also recognized.
- Skills live under the plugin's declared skill folder, normally `skills/`.
- Repeated local skill names may be valid because the plugin namespace
  disambiguates them.

Activation policy: `preserve_plugin_activation`

Preservation policy:

- Preserve the plugin root folder shape.
- Preserve plugin manifest metadata and namespace.
- Preserve each skill folder, frontmatter, original source, resources, and
  slash-command bindings.
- Preserve repeated local names by namespace rather than flattening them into
  local-name installs.
- Do not create an ordinary multi-skill activation wrapper unless the plugin
  source itself declares that shape.

Why plugin activation is different:

A plugin is already an activation boundary. Flattening plugin skills into an
ordinary workspace loses namespace, command bindings, plugin metadata, and
harness activation semantics. The checklist must keep the plugin install shape
so the harness can activate the plugin as a plugin.

Entry checklist:

```json
{
  "step": "plugin_entry",
  "description": "Map the source root as a plugin-shaped workspace.",
  "directive": "Detect plugin roots, read plugin manifests, preserve namespaces, and map every plugin skill folder without flattening local names.",
  "commands": [
    "skillspec doctor <plugin-root> --json",
    "skillspec workspace map <plugin-root> --out <build>/skillspec.workspace.yml --summary",
    "skillspec workspace validate <build>/skillspec.workspace.yml --summary"
  ],
  "repeat": {
    "for_each": "plugin_skill_folder",
    "until": "every plugin skill folder is represented with namespace and frontmatter"
  },
  "blocks": [
    "missing_plugin_manifest",
    "namespace_not_preserved",
    "skill_folder_flattened",
    "duplicate_local_name_without_namespace"
  ]
}
```

Loop checklist:

```json
{
  "step": "plugin_package_loop",
  "description": "Promote each plugin skill package without losing plugin semantics.",
  "directive": "Review one plugin package at a time. Preserve frontmatter, slash-command behavior, resources, dependency evidence, state machines, handoff rules, and output templates. Classify evidence before creating hard dependencies; treat plugin slash references as workflow references unless a hard file dependency is found.",
  "commands": [
    "skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build> --summary",
    "skillspec source lens <workspace-build>/<plugin-package>/.skillspec/source-map/source-map.json --cursor <cursor>",
    "skillspec validate <workspace-build>/<plugin-package>/skill.spec.yml",
    "skillspec deps check <workspace-build>/<plugin-package>/skill.spec.yml",
    "skillspec test <workspace-build>/<plugin-package>/skill.spec.yml"
  ],
  "repeat": {
    "outer": "for_each_plugin_package_in_manifest_order",
    "inner": "for_each_source_lens_block",
    "until": "all plugin packages are reviewed and plugin namespace proof is complete"
  },
  "forbid": [
    "flatten_plugin_shape",
    "drop_plugin_manifest",
    "drop_plugin_namespace",
    "bulk_yaml_rewrite"
  ]
}
```

Exit checklist:

```json
{
  "step": "plugin_exit",
  "description": "Compile and install the plugin-shaped workspace without flattening activation.",
  "directive": "Converge, compile, and dry-run install with plugin-compatible install slugs and visibility. The dry-run must show preserved namespace, plugin activation material, parent-folder preservation, and retirement behavior for preexisting plugin skills.",
  "commands": [
    "skillspec workspace converge <build>/skillspec.workspace.yml --build-root <workspace-build> --summary",
    "skillspec workspace compile <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex-skill --summary",
    "skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --retire-existing --dry-run --summary",
    "skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --retire-existing --summary"
  ],
  "blocks": [
    "plugin_manifest_not_packaged",
    "namespace_lost",
    "plugin_skill_flattened_to_local_name",
    "slash_command_binding_unmapped",
    "unpromoted_package_scaffold"
  ]
}
```

## Doctor Checklist

`skillspec doctor checklist` is the entry classifier. It should not import,
compile, install, or create proof ledgers. It should inspect the selected source
and print the shape-specific next plan.

Entry directives:

- Count `SKILL.md` files.
- Detect plugin markers.
- Read frontmatter.
- Identify sibling resources, scripts, references, and dependency signals.
- Emit shape, activation policy, package candidates, and the next allowed
  command path.

Loop directives:

- For each discovered `SKILL.md`, prove it belongs to exactly one package.
- For each plugin root, prove namespace and manifest association.
- For each cross-reference, classify it as a hard dependency, workflow
  reference, external reference, or unresolved reference.

Exit directives:

- If shape is single, emit source-map/import commands for one package.
- If shape is multi-skill, emit workspace map/validate/import commands and the
  single workspace activation policy.
- If shape is plugin-shaped, emit workspace map/validate/import commands and
  the preserve-plugin-activation policy.

## Import Checklist

`skillspec import checklist` owns the long import loop. It should be callable
before import, during package promotion, and before compile/install.

Entry directives:

- Verify the doctor shape decision.
- Verify source map or workspace manifest exists.
- Verify original source preservation path.
- Verify frontmatter preservation plan.
- Verify dependency ledger exists for every generated package.
- Verify package order and current package cursor.

Loop directives:

- Repeat for each package in manifest or graph order.
- Inside each package, repeat for each source lens block.
- Run validate/deps/test after each package-level promotion.
- Require package-local promotion proof with source hashes and target kinds.
- Keep package status as scaffold until all required source obligations are
  covered.

Exit directives:

- Run workspace converge.
- Run workspace compile.
- Run install dry-run.
- Verify retirement plan for preexisting active skills.
- Block final install when any installable package remains a scaffold.

## Run Checklist

`skillspec run checklist` owns execution after a SkillSpec is selected. It
should be generated from a spec and run directory rather than from source shape.

Entry directives:

- Show selected route, matched rules, route selection basis, current phase, and
  open requirements.
- Show active forbids and required elicitations.
- Show allowed commands for the current phase.

Loop directives:

- Repeat until no open phase requirement remains.
- At each phase boundary, checkpoint successful evidence once with a compact
  progress command.
- Record individual progress rows only for failures, blockers, or explicit
  debugging.
- Do not manufacture proof after the fact.

Exit directives:

- Run quiet alignment.
- Report pass, partial, or fail with exact missing proof.
- Include trace path, proof-digest path, alignment path, and token metrics when
  available.

## Concrete Generated Example: Multi-Skill Folder

For this source:

```text
skills/
  intake/
    SKILL.md
  analysis/
    SKILL.md
    templates/report.md
  handoff/
    SKILL.md
```

The checklist should name the entity:

```json
{
  "shape": "multi_skill_folder",
  "activation_policy": "single_workspace_activation",
  "packages": [
    {"id": "intake", "path": "intake", "frontmatter_name": "intake"},
    {"id": "analysis", "path": "analysis", "frontmatter_name": "analysis"},
    {"id": "handoff", "path": "handoff", "frontmatter_name": "handoff"}
  ],
  "entry_package": "workspace-activation",
  "support_packages": ["intake", "analysis", "handoff"],
  "must_preserve": [
    "source/SKILL_md.old",
    "frontmatter",
    "resources",
    "cross_refs"
  ],
  "next": {
    "package_index": 2,
    "package_count": 3,
    "package_id": "analysis",
    "cursor": 5,
    "command": "skillspec source lens /tmp/build/analysis/.skillspec/source-map/source-map.json --cursor 5"
  }
}
```

## Concrete Generated Example: Plugin Shape

For this source:

```text
privacy-legal/
  .claude-plugin/
    plugin.json
  skills/
    data-retention/
      SKILL.md
    breach-response/
      SKILL.md
```

The checklist should preserve plugin activation:

```json
{
  "shape": "plugin_shape",
  "activation_policy": "preserve_plugin_activation",
  "plugin": {
    "root": "privacy-legal",
    "manifest": ".agent-plugin/marketplace.json",
    "namespace": "privacy-legal"
  },
  "packages": [
    {
      "id": "privacy-legal.data-retention",
      "path": "skills/data-retention",
      "frontmatter_name": "data-retention"
    },
    {
      "id": "privacy-legal.breach-response",
      "path": "skills/breach-response",
      "frontmatter_name": "breach-response"
    }
  ],
  "must_preserve": [
    "plugin_manifest",
    "namespace",
    "skill_frontmatter",
    "folder_shape",
    "slash_command_bindings",
    "plugin_parent_non_skills_files",
    "compiled_packages_replace_source_skills_tree"
  ],
  "forbid": [
    "flatten_plugin_shape",
    "install_slug_policy_local_name_without_explicit_review",
    "install_plugin_skills_as_top_level_flattened_folders",
    "drop_plugin_parent_files_when_installing_compiled_skills"
  ]
}
```

## Trampoline Integration

The SkillSpec trampoline should shrink to three responsibilities:

1. Find the colocated `skill.spec.yml`.
2. Start or resume `skillspec run-loop`.
3. When the current route is doctor/import/run work, call the matching
   checklist command and follow its returned steps.

The trampoline should not restate the full source-shape matrix, package
promotion loop, dependency ledger policy, install gate, or proof policy. Those
belong in generated checklists and the structured spec.

## Acceptance Criteria

A checklist implementation is acceptable when these are true:

- `doctor checklist` emits a different concrete checklist for single-skill,
  multi-skill, and plugin-shaped sources.
- `import checklist` names the exact next package, source cursor, remaining
  count, commands, forbids, and evidence paths.
- `run checklist` names the exact current route/phase/requirements and does not
  require the trampoline to remember route internals.
- A generated scaffold cannot pass the exit checklist.
- A package cannot pass by inheriting proof from another package.
- Conditional and state-machine source language is blocked unless represented
  structurally.
- Dependency mentions cannot be deleted to make validation pass.
- Multi-skill and plugin-shaped sources cannot be flattened into ordinary
  top-level per-package install folders.
- Multi-skill and plugin-shaped installs must preserve the parent folder, copy
  non-skill source files from the reference parent, replace the source skill
  package subtree with compiled SkillSpec-backed packages, and leave auditable
  path evidence in the install dry-run/report.
- Ordinary multi-skill folders produce one workspace activation path unless the
  source and user explicitly approve additional entrypoints.
- Replacement installs retire preexisting active skills after approval instead
  of leaving duplicate old and new skills discoverable.

## Implementation Notes

This document defines the desired contract and the initial command surface now
implemented in the CLI. Detailed cues should continue moving out of the
trampoline and into generated checklist output as the checklist engine grows.

Implementation surfaces:

- CLI args and dispatch for `doctor checklist`, `import checklist`, and
  `run checklist`;
- source-shape and source-map data from `crates/skillspec-doctor`;
- workspace manifest and readiness data from `crates/skillspec-workspace`;
- route and progress state from `crates/skillspec-runtime`;
- command inventory and docs in `spec/commandspec.md`;
- trampoline docs and skills in `skills/skillspec/` and plugin mirrors.
