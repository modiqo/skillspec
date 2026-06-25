# Workspace Authoring Graph

Workspace authoring exists for source roots that are bigger than one atomic
skill package. The core rule is simple: every folder containing one `SKILL.md`
is one package, and the parent root is a workspace that must be mapped before
fanout import.

This surface is deliberately separate from router indexing. Router indexing
answers "which installed skill should handle this task?" Workspace mapping
answers "what packages exist in this source tree, how are they named, what do
they reference, and what can be imported safely?"

## Command Flow

The expected authoring flow is:

```bash
skillspec workspace map <source-root> --out <build>/skillspec.workspace.yml
skillspec workspace validate <build>/skillspec.workspace.yml
skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build>
skillspec workspace converge <build>/skillspec.workspace.yml --build-root <workspace-build>
skillspec workspace compile <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex-skill
skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --dry-run
skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --apply-visibility
```

`map` and `validate` are the structure recon gate. `import` fans out one draft
package per atomic skill. `converge` checks generated drafts against the graph.
`compile` writes harness loaders. `install` preflights and then writes the
compiled package set into harness roots using manifest install slugs.

## Package Identity

Each package records:

- `package_id`: stable id derived from the path, such as
  `commercial-legal.skills.review`.
- `path`: source-root-relative package path.
- `public_name`: the skill name that compiled loaders and install collision
  checks use.
- `install_slug`: deterministic folder slug for installed workspace packages.
- `namespace` and `local_name` when the package lives inside a plugin-shaped
  root.

Non-plugin repositories keep legacy naming. Plugin-shaped repositories get a
namespace to avoid flattening collisions.

## Plugin Namespaces

A directory is treated as a plugin root when it has a `skills/` subdirectory and
at least one plugin marker:

- `.claude-plugin/plugin.json`
- `.mcp.json`
- `CLAUDE.md`

The namespace comes from `.claude-plugin/plugin.json` field `name` when present.
When that file is missing or has no usable name, the plugin folder slug is used.

For plugin packages:

- `namespace` is the plugin namespace, such as `commercial-legal`.
- `local_name` is the raw skill name inside that plugin, such as `review`.
- `public_name` is skill-safe and namespaced, such as
  `commercial-legal-review`.

This preserves plugin shape without requiring harness skill names to contain
colons. It also prevents repeated plugin-local names such as
`cold-start-interview`, `customize`, and `matter-workspace` from colliding.

## Reference Semantics

Workspace references are classified by kind:

- `file`: relative references such as `../coding-standards/SKILL.md`.
- `skill_invocation`: slash-command references such as `/cold-start-interview`
  or `/privacy-legal:use-case-triage`.

File references are hard dependencies and produce `depends_on` edges.

For plugin packages, slash-command references are workflow links. They are
resolved and recorded, but they do not create hard dependency edges. This avoids
false cycles when one skill tells the operator to run another skill later.

For non-plugin packages, legacy slash-command references still infer hard
dependencies. This preserves behavior for ordinary skills repositories that use
slash references as dependency shorthand.

## Invocation Resolution

Plugin slash-command resolution follows this order:

1. Explicit namespace, such as `/privacy-legal:use-case-triage`.
2. Same-plugin local name, such as `/cold-start-interview` from inside
   `commercial-legal`.
3. Global public name fallback, such as `/commercial-legal-review`.

Unqualified plugin-local aliases are not added to the global namespace. That is
the main protection against flattening collisions.

## Validation

`workspace validate` checks:

- manifest schema;
- source root exists;
- each package path is workspace-relative;
- each package contains exactly one `SKILL.md`;
- dependency ids resolve;
- packages do not depend on themselves;
- hard dependency graph is acyclic;
- install slugs are unique;
- hard cross-package references are covered by `depends_on`.

Duplicate public names are warnings during map/validate and become blockers
during workspace install planning. Plugin namespace mapping should make common
plugin-local duplicate names unique before install.

## Install Visibility

Workspace install uses manifest `install_slug` folders so packages from the same
workspace cannot overwrite ordinary single-skill installs accidentally. Install
is always planned first with `--dry-run`.

When `--apply-visibility` is used, the default `entry-implicit` policy keeps
entry packages visible and makes shared/helper/wrapper packages manual-only.
Other policies are explicit: `all-implicit`, `all-manual`, and `none`.

Router refresh remains separate runtime work. Workspace install does not rebuild
router indexes.

## Real Repo Smoke Tests

The plugin namespace behavior was validated against
`/Users/chetanconikee/tulving/claude-for-legal`:

- 151 packages discovered;
- 13 plugin namespaces discovered;
- repeated names resolved by namespaced public names;
- plugin slash-command references resolved without dependency cycles;
- workspace validation passed.

Legacy non-plugin behavior was validated against
`/Users/chetanconikee/tulving/skills`:

- 8 packages discovered;
- no plugin namespaces;
- existing hard dependency edges preserved;
- workspace validation passed.

## Critical Junctions

Keep these surfaces aligned whenever workspace behavior changes:

- `crates/skillspec-cli/src/cli/args.rs` for command shape and help text;
- `crates/skillspec-cli/src/features/workspace.rs` and submodules for graph,
  fanout, converge, compile, install, and visibility behavior;
- `spec/commandspec.md` for the formal command inventory;
- `docs/design/16-command-log.md` for the quick command table;
- `docs/README_DETAILED.md` and top-level `README.md` for user workflows;
- `skills/skillspec/skill.spec.yml` and generated `skills/skillspec/SKILL.md`
  for prompt-driven multiplexer behavior;
- CLI tests in `crates/skillspec-cli/tests/cli.rs`;
- smoke tests against a plugin-shaped repo and an ordinary multi-skill repo.
