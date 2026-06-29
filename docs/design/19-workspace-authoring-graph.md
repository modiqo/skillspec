# Workspace Authoring Graph

Workspace authoring exists for source roots that are bigger than one atomic
skill package. The core rule is simple: every folder containing one `SKILL.md`
is one package, and the parent root is a workspace that must be mapped before
fanout import.

This surface is deliberately separate from router indexing. Router indexing
answers "which installed skill should handle this task?" Workspace mapping
answers "what packages exist in this source tree, how are they named, what do
they reference, and what can be imported safely?"

## Operator Decision Gate

Agents should classify the selected source before choosing an import command.
This avoids treating `port-one-shot` as a universal shortcut.

| Observed source | Route | Command path |
| --- | --- | --- |
| Exactly one `SKILL.md` below the selected root, no plugin markers, no reviewed `skill.spec.yml` to revise | Single atomic skill | `skillspec port-one-shot <source> --out <draft> --target codex-skill --prove` |
| More than one `SKILL.md`, shared standards packages, or relative references into sibling skill folders | Ordinary multi-skill workspace | `skillspec workspace map`, `workspace validate`, `workspace import`, `workspace converge`, `workspace compile` |
| A root with `skills/` plus `.claude-plugin/plugin.json`, `.mcp.json`, or `CLAUDE.md` | Plugin-shaped workspace | Workspace flow with plugin namespace preservation |
| A reviewed `skill.spec.yml` already exists and the request is to improve it | Existing SkillSpec revision | `skillspec grammar sensemake`, `skillspec sensemake`, precise `query`/`refs`, then validate/test/compile |

The source shape wins over command convenience. If a user asks for one-shot on a
parent folder that contains multiple skills or plugin boundaries, the correct
response is to map the workspace and explain why. If a user asks to import a
folder that already has a reviewed `skill.spec.yml`, the correct response is to
revise that spec instead of generating a new scaffold.

## Compatibility Boundary

The workspace flow must not change the simple single-skill path.

A normal skill folder such as:

```text
pdf/
  SKILL.md
  references/
  scripts/
```

is still one atomic skill package. It continues to use the ordinary commands:

```bash
skillspec source map ./pdf --out ./pdf/.skillspec/source-map/source-map.json
skillspec import-skill ./pdf --out ./pdf/skill.spec.yml --source-map ./pdf/.skillspec/source-map/source-map.json
skillspec compile ./pdf/skill.spec.yml --target codex-skill > ./pdf/SKILL.md
skillspec install skill ./pdf --target codex --dry-run
```

`import-skill` calls a workspace guard before importing. The guard counts
`SKILL.md` files under the source root:

- zero or one `SKILL.md`: proceed to command-specific single-skill validation;
- more than one `SKILL.md`: stop and tell the operator to run
  `skillspec workspace map`.

That boundary is intentional. Workspace code is for structure recon, fanout,
converge, and grouped install. It should not reinterpret a plain folder with
one `SKILL.md` and resources.

## Detected Shapes

SkillSpec recognizes three practical source shapes.

### Single Atomic Skill

Example:

```text
source-root/
  SKILL.md
  references/
  resources/
  scripts/
```

Detection:

- exactly one `SKILL.md` exists below the selected source root;
- no workspace manifest is required;
- all sibling folders are treated as package-local resources for that one skill.

Activation:

- `import-skill` creates one `skill.spec.yml`;
- `compile` creates one harness-facing `SKILL.md` loader;
- `install skill` installs one generated skill folder into the selected harness
  root;
- router indexing, if desired, happens later through the router/index commands.

### Ordinary Multi-Skill Workspace

Example:

```text
skills/
  coding-standards/
    SKILL.md
  code-review/
    SKILL.md
    review.md          # may reference ../coding-standards/SKILL.md
  test-driven-fix/
    SKILL.md           # may invoke /code-review
```

Detection:

- more than one `SKILL.md` exists below the selected source root;
- no plugin marker is needed;
- every folder containing `SKILL.md` becomes one package;
- package names keep legacy non-plugin behavior.

Activation:

- `workspace map` creates one manifest entry per package;
- relative file references such as `../coding-standards/SKILL.md` become hard
  dependency edges;
- non-plugin slash-command references such as `/code-review` continue to infer
  hard dependencies for legacy repositories;
- `workspace import` fans out one draft `skill.spec.yml` per package in
  dependency order;
- `workspace converge` checks that generated drafts still line up with the
  workspace graph;
- `workspace compile` creates one loader per ready package;
- `workspace install` installs the compiled package set using deterministic
  install slugs so the workspace cannot accidentally overwrite a separately
  installed single skill with the same public name.

### Plugin-Shaped Workspace

Example:

```text
legal-plugins/
  commercial-legal/
    .claude-plugin/
      plugin.json      # { "name": "commercial-legal" }
    skills/
      review/
        SKILL.md
      cold-start-interview/
        SKILL.md
  privacy-legal/
    .claude-plugin/
      plugin.json      # { "name": "privacy-legal" }
    skills/
      review/
        SKILL.md
      cold-start-interview/
        SKILL.md
```

Detection:

- a directory is a plugin root when it has a `skills/` subdirectory plus
  `.claude-plugin/plugin.json`, `.mcp.json`, or `CLAUDE.md`;
- packages below `<plugin-root>/skills/` inherit that plugin namespace;
- the namespace comes from `.claude-plugin/plugin.json` field `name` when
  present, otherwise from the plugin folder name;
- repeated local names are allowed because public names are namespaced.

Activation:

- `commercial-legal/skills/review/SKILL.md` becomes public skill
  `commercial-legal-review`;
- `privacy-legal/skills/review/SKILL.md` becomes public skill
  `privacy-legal-review`;
- `/cold-start-interview` inside `commercial-legal` resolves to
  `commercial-legal-cold-start-interview`;
- `/privacy-legal:cold-start-interview` resolves across plugins;
- plugin slash-command references are recorded as workflow references, not hard
  dependency edges, because they often mean "run this other plugin skill later"
  rather than "load this package before compiling me";
- relative file references are still hard dependencies.

This protects plugin boundaries without requiring installed harness skill names
to contain colons.

## Shape Detection Workflow

`skillspec workspace map <source-root>` performs structure recon in this order.

1. Normalize the source root.

   The selected path is resolved into the workspace source root used in the
   manifest. All package paths are recorded relative to that root.

2. Discover plugin roots.

   The mapper recursively scans directories, skipping ignored build and VCS
   folders. A directory is recorded as a plugin root when it has `skills/` and
   at least one supported plugin marker:

   - `.claude-plugin/plugin.json`
   - `.mcp.json`
   - `CLAUDE.md`

   Plugin roots are sorted deepest first so package classification chooses the
   most specific plugin root if nested plugin-shaped folders ever appear.

3. Discover atomic skill packages.

   The mapper recursively finds every `SKILL.md`. Each parent directory becomes
   one package. The mapper does not merge neighboring skills and does not treat a
   parent folder as a skill just because it contains children.

4. Classify each package.

   For each `SKILL.md`, SkillSpec reads frontmatter and records:

   - package root path relative to the workspace;
   - `package_id`, derived from the relative path;
   - raw public name from frontmatter `name`, falling back to the folder name;
   - package kind, currently inferred as `shared`, `entry`, or `helper`;
   - namespace and local name when the package lives under a plugin root.

5. Assign install identity.

   Every package receives a deterministic `install_slug`. The default policy is
   `workspace-path`: workspace slug plus relative package path. This keeps
   side-by-side workspace installs and plugin-shaped packages from flattening
   into the same harness folder.

   Replacement/upgrade flows can choose `local-name`:

   ```bash
   skillspec workspace map <source-root> \
     --out <build>/skillspec.workspace.yml \
     --install-slug-policy local-name
   ```

   or override an existing manifest during install:

   ```bash
   skillspec workspace install <build>/skillspec.workspace.yml \
     --build-root <workspace-build> \
     --target codex \
     --install-slug-policy local-name \
     --retire-existing \
     --dry-run
   ```

   Use `local-name` only when the intent is to replace canonical installed
   skills, for example retiring `rote-setup` with a generated `rote-setup`
   package. Validation rejects duplicate `install_slug` values before install,
   so plugin workspaces with repeated local names must keep `workspace-path` or
   manually choose unique manifest slugs.

6. Build the skill invocation index.

   Non-plugin packages are indexed globally by public name and path tail.
   Plugin packages are indexed globally only by namespaced public name, and
   indexed locally by `(namespace, local_name)`.

   This means:

   - `/code-review` can resolve globally in an ordinary workspace;
   - `/commercial-legal-review` can resolve globally in a plugin workspace;
   - `/review` inside `commercial-legal` resolves only inside that plugin;
   - `/privacy-legal:review` resolves explicitly across plugin namespaces;
   - duplicate unqualified plugin-local names do not pollute the global name
     space.

7. Scan package Markdown for references.

   The mapper scans package-local Markdown files, excluding nested child skill
   packages and fenced code blocks.

   It records two reference kinds:

   - `file`: relative Markdown references such as
     `../coding-standards/SKILL.md`;
   - `skill_invocation`: slash-command references such as `/code-review` or
     `/privacy-legal:review`.

8. Infer hard dependencies.

   Dependency edges are inferred from references that must be ready before the
   current package can be safely imported, converged, compiled, or installed.

   - file reference across packages: hard dependency;
   - non-plugin slash-command reference: hard dependency for legacy behavior;
   - plugin slash-command reference: workflow reference only;
   - self-reference: ignored.

9. Write the workspace manifest and map report.

   The manifest becomes the stable input for `validate`, `import`, `converge`,
   `compile`, and `install`. The map report is the human-readable explanation of
   what was discovered: packages, plugin namespaces, references, dependency
   edges, duplicate names, duplicate install slugs, and unresolved references.

## Activation Workflow

Activation means "what becomes visible or usable to the agent harness after the
source shape has been analyzed and compiled."

For a single atomic skill:

1. the source folder is imported into one `skill.spec.yml`;
2. the spec is compiled into one small `SKILL.md` loader;
3. the generated folder is installed as one harness skill;
4. any additional resources remain package-local beside that skill.

For an ordinary multi-skill workspace:

1. `workspace map` decides the package graph;
2. `workspace validate` blocks broken paths, cycles, self-dependencies, and
   install slug collisions;
3. `workspace import` creates one generated package folder per atomic source
   skill;
4. packages are processed in dependency order, so shared packages such as
   `coding-standards` are handled before dependents;
5. `workspace converge` checks that all generated specs exist and that
   dependents are not released while dependencies are missing or failed;
6. `workspace compile` creates one harness loader per ready package;
7. `workspace install` installs the grouped package set using manifest
   `install_slug` folders.

For a plugin-shaped workspace:

1. plugin roots define namespaces;
2. every package below each plugin's `skills/` folder gets a namespaced public
   name;
3. plugin-local invocations resolve inside their own namespace first;
4. explicit namespace invocations can cross plugin boundaries;
5. plugin workflow links are preserved as references for reports and alignment
   without turning the whole plugin library into one cyclic dependency graph;
6. install uses namespaced public names plus `workspace-path` install slugs by
   default; replacement installs may opt into `local-name` only when duplicate
   install slugs are not present.

Workspace install can optionally apply visibility. With the default
`entry-implicit` policy, entry packages stay implicitly visible and shared,
helper, or wrapper packages become manual-only. Other policies are explicit:
`all-implicit`, `all-manual`, and `none`.

Router activation is a separate step. Installing a workspace writes skills into
harness roots; it does not rebuild the router index.

## Command Flow

The expected authoring flow is:

```bash
skillspec workspace map <source-root> --out <build>/skillspec.workspace.yml --summary
skillspec workspace validate <build>/skillspec.workspace.yml --summary
skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build> --summary
skillspec workspace converge <build>/skillspec.workspace.yml --build-root <workspace-build> --summary
skillspec workspace compile <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex-skill --summary
skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --dry-run --summary
skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --apply-visibility --summary
```

`map` and `validate` are the structure recon gate. `import` fans out one draft
package per atomic skill. Dependency-ready packages in the same graph level may
import in parallel, and unchanged packages with intact proof artifacts are
reused from `<workspace-build>/.skillspec/workspace-cache.json` and reported as
`cached`. `converge` checks generated drafts against the graph.
`--summary` keeps agent-facing output compact by printing wall-clock and
estimated token metrics while preserving full reports and package evidence on
disk.
Those estimates are direct-run output-economy metrics: agent-visible summary
tokens versus artifact tokens kept out of chat. Durable-executor runs add a
separate measured token-accounting layer when workspace stats are available.
When a run needs these estimates in `trace align --summary`, record the summary values
with `skillspec progress stats <run-dir> --agent-visible-tokens <n>
--artifact-tokens-preserved <n> --avoided-tokens <n> --metrics-source
estimated` before alignment.
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

- `crates/skillspec-cli/src/cli/args/` for command shape and help text;
- `crates/skillspec-cli/src/features/workspace.rs` and submodules for graph,
  fanout, converge, compile, install, and visibility behavior;
- `spec/commandspec.md` for the formal command inventory;
- `docs/design/16-command-log.md` for the quick command table;
- `docs/README_DETAILED.md` and top-level `README.md` for user workflows;
- `skills/skillspec/skill.spec.yml` and generated `skills/skillspec/SKILL.md`
  for prompt-driven multiplexer behavior;
- CLI tests in `crates/skillspec-cli/tests/cli/`;
- smoke tests against a plugin-shaped repo and an ordinary multi-skill repo.
