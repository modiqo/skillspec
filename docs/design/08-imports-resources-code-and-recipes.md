# Imports, Resources, Code, And Recipes

SkillSpec separates extra material by role. This prevents a skill package from
turning every nearby file, code block, example, and procedure into active
instructions.

The separation is:

- `imports`: active instruction material the harness may deliberately load.
- `resources`: provenance or supporting material.
- `code`: preserved executable or example code blocks.
- `commands`: command templates.
- `artifacts`: declared produced or consumed outputs.
- `recipes`: ordered procedures that compose imports, resources, commands, code,
  artifacts, questions, branches, and notes.

The design principle is explicit activation. A file should influence behavior
only when the spec declares how it is connected to the active contract.

## Imports

Imports are runtime-loadable instruction material. They are for material the
agent may need to read as guidance while satisfying a task.

The current import model includes:

- `path`;
- `role`;
- `description`;
- `section`;
- `load`;
- `requires.imports`;
- `used_by`;
- `load_when`.

Import roles are:

- `policy`;
- `reference`;
- `procedure`;
- `example`;
- `skill`.

Import load modes are:

- `always`;
- `on_demand`.

`on_demand` is the default.

An import is not inheritance. It does not merge another SkillSpec into the
current spec. It points to local instruction material that a harness can load
deliberately.

## Import Validation

The parser and import checker enforce several constraints:

- import ids are validated identifiers;
- `path` must be non-empty;
- `section`, when present, must be non-empty;
- `requires.imports` must name known imports;
- `used_by` entries must reference known targets for their kind;
- import dependency cycles are rejected;
- an import is rejected as an orphan unless it is referenced, explicitly
  `used_by`, or marked `load: always`;
- `skillspec imports check` rejects absolute paths and URL-like paths containing
  `://`;
- `skillspec imports check` resolves paths relative to the spec directory;
- `skillspec imports check` checks file existence;
- `skillspec imports check` checks declared Markdown sections;
- `skillspec imports check` reports dependency-first load order.

These checks make import loading reviewable. They do not inject import content
into the model automatically. The loader or harness still has to read the
declared import at the right time.

## Resources

Resources are supporting material. They are for provenance, examples, assets,
source material, required procedures, scripts, and references that explain where
the contract came from or what it depends on.

The current resource model includes:

- `path`;
- `role`;
- `description`;
- `used_by`;
- `load_when`.

Resource roles are:

- `source_material`;
- `reference`;
- `required_procedure`;
- `example`;
- `script`;
- `asset`.

The parser validates resource ids, non-empty paths, `used_by` references, and
orphan resources. A resource must be referenced or explicitly `used_by`.

The difference between import and resource is behavioral:

- an import is active guidance that may be loaded during execution;
- a resource is provenance or support unless another structured section uses it.

If a Markdown document contains instructions the agent must follow, declare it
as an import. If it explains why the contract exists or where code came from,
declare it as a resource.

## Code

`code` preserves executable or example code in a structured form.

The current code model includes:

- `language`;
- `kind`;
- `source`;
- `provenance`;
- `purpose`;
- `requires`;
- `inputs`;
- `outputs`;
- `safety`;
- `use_when`.

Code kinds are:

- `example`;
- `runnable_script`;
- `probe`;
- `transform`;
- `validator`;
- `troubleshooting`;
- `reference`.

Code source can be inline or file-backed. File-backed code can point at a
resource, fence index, heading, or hash. Code provenance can record the resource
or import that produced the block, fence index, heading, and source line range.

Code requirements can reference:

- `dependencies`;
- `imports`;
- `resources`;
- `artifacts`.

Code safety currently records:

- `mutates_input`;
- `writes_files`;
- `network`;
- `notes`.

Imported fenced code is not automatically safe to run. Importing a code block
preserves knowledge and provenance. A harness should run it only when the active
contract, dependencies, safety fields, and approval policy allow it.

## Commands

Commands are command templates. They are distinct from code blocks because a
command template is intended to describe a shell or CLI invocation surface.

A command can declare:

- `description`;
- `template`;
- `safety`;
- `requires`;
- `parse`;
- `success_when`.

Command requirements can include dependencies, files, environment variables, and
auth requirements. `skillspec deps check <spec> --command <id>` narrows
dependency checks to one command's declared dependencies.

A command template is not permission. It is a declared execution surface for the
harness to inspect and approve.

## Artifacts

Artifacts make dataflow explicit.

The current artifact model includes:

- `kind`;
- `description`;
- `path`;
- `schema`;
- `produced_by`;
- `consumed_by`.

Artifact kinds are:

- `file`;
- `directory`;
- `json`;
- `text`;
- `image`;
- `pdf`;
- `transcript`;
- `report`.

Producer and consumer references can point to commands, code, or recipes. The
parser validates those references.

Artifacts are useful when a skill should prove that a command, code block, or
recipe produced a specific output and that another step consumed it.

## Recipes

Recipes are ordered or unordered procedures that compose other contract
elements.

The current recipe model includes:

- `description`;
- `ordered`;
- `requires`;
- `steps`.

Recipe requirements can reference:

- `imports`;
- `resources`;
- `dependencies`;
- `artifacts`.

Recipe steps can:

- `load_import`;
- `load_resource`;
- `run_command`;
- `run_code`;
- `produce_artifact`;
- `consume_artifact`;
- `ask`;
- `branch`;
- `note`.

The parser validates recipe requirements and step targets. Branch targets may
point to a command, code block, or recipe. A branch must have a non-empty `if`
condition, and `note` steps must have non-empty text.

Recipes are not executed by the v0 CLI. They make procedural intent explicit so
an agent or harness can execute through policy, preserve evidence, and report
which steps are proven.

## Importer Behavior

The prose importer uses these structures conservatively.

It can:

- preserve Markdown documents as imports or resources;
- extract command blocks into command templates;
- extract fenced code blocks into `code`;
- attach code provenance to resources or imports;
- infer simple CLI dependencies from commands and runnable code languages;
- create review notes.

It does not decide which prose paragraphs are correct rules, which code blocks
are safe to run, or which recipes are complete. Those decisions must be reviewed
and promoted into the spec deliberately.

## Design Guidance

Use `imports` for material the agent should read as active guidance.

Use `resources` for source material, examples, references, scripts, and assets
that support the contract but are not active instructions by default.

Use `code` to preserve executable or example logic with provenance, requirements,
inputs, outputs, and safety notes.

Use `commands` for reusable CLI invocation templates.

Use `artifacts` when output and dataflow matter.

Use `recipes` when the skill needs an ordered procedure, but keep execution
policy in the harness.

Do not rely on folder proximity. If a file matters, connect it to the contract.

Do not run imported code just because it exists. Connect it to a route, rule,
command, recipe, dependency, safety declaration, and evidence expectation.

## Source Alignment

This doc is grounded in:

- `spec/imports.md`, which defines imports as explicit local runtime-loadable
  instruction material;
- `spec/grammar.md`, which distinguishes imports, resources, code, artifacts,
  commands, and recipes;
- `crates/skillspec-cli/src/spec/model.rs`, which defines the typed structures and
  enums for imports, resources, code, artifacts, commands, and recipes;
- `crates/skillspec-cli/src/spec/parser/validation.rs`, which validates references and orphaned
  imports/resources;
- `crates/skillspec-cli/src/spec/imports.rs`, which validates local import paths,
  Markdown sections, and load order;
- `crates/skillspec-cli/src/features/importer.rs`, which scaffolds imports, resources,
  code, commands, inferred dependencies, and review notes from prose skills.
