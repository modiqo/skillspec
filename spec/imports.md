# Imports

Imports are v0's runtime-loadable instruction layer. They are for Markdown or
text that the harness should deliberately read while executing a skill, such as
shared operating rules, branch references, procedures, examples, or another
skill document.

Resources remain provenance and supporting material. A source file, asset,
script, or evidence document should stay a resource unless the agent is expected
to load it as active guidance during the run.

## Shape

```yaml
imports:
  shared_operating_rules:
    path: ../INDEX.md
    role: policy
    section: Shared operating rules
    load: always
    description: Shared rules every onboard skill must obey.

  flow_search_and_run:
    path: references/flow-search-and-run.md
    role: procedure
    load: on_demand
    used_by:
      - kind: recipe
        id: run_existing_flow

  flow_authoring:
    path: references/flow-authoring.md
    role: procedure
    requires:
      imports:
        - typescript_transformations
    used_by:
      - kind: route
        id: author_flow

  typescript_transformations:
    path: references/typescript-transformations.md
    role: reference
    used_by:
      - kind: recipe
        id: author_flow
```

`load` defaults to `on_demand`. Use `always` only for small, load-bearing
policy that must be in context before any task action.

## Resolution

Import paths resolve relative to the directory that contains `skill.spec.yml`.
That rule holds even for nested imports. Nested imports are links between import
ids, not nested path scopes.

```text
skill/
  skill.spec.yml
  SKILL.md
  references/task-routing.md
plugin/
  INDEX.md
```

From `skill/skill.spec.yml`:

```yaml
imports:
  shared_rules:
    path: ../plugin/INDEX.md
    role: policy
    load: always
  task_routing:
    path: references/task-routing.md
    role: reference
```

Do not expand `~`, environment variables, shell substitutions, or Markdown links
inside import paths. Harnesses may reject paths that escape an allowed package
root, cross a symlink boundary, point at an absolute path, or use a URL.

Use the CLI to validate static import resolution:

```bash
skillspec imports check skill.spec.yml
```

The command validates local relative paths, Markdown sections, explicit nesting,
and dependency-first load order. It does not inject imported content into an
agent context; runtime loading remains the harness responsibility.

## Sections

`section` narrows a Markdown import to one heading and its children. A loader
should find the named heading and read until the next heading at the same or
higher level.

```yaml
imports:
  shared_rules:
    path: ../INDEX.md
    role: policy
    section: Shared operating rules
    load: always
```

If the section is missing, fail closed with a clear missing-import error. Do not
silently load the whole file or a nearby heading.

## Nesting

`requires.imports` is the only v0 nesting mechanism. It forms a directed
acyclic graph of import ids. All imports live in the main spec's top-level
`imports` map; nested imports are references between those ids, not declarations
inside imported Markdown files.

Depth is arbitrary:

```yaml
imports:
  a:
    path: a.md
    role: procedure
    requires:
      imports: [b]
  b:
    path: references/b.md
    role: reference
    requires:
      imports: [c]
  c:
    path: ../shared/c.md
    role: policy
    used_by:
      - kind: recipe
        id: use_a
```

All three paths resolve from the directory containing `skill.spec.yml`, even
when `b` is loaded because `a` requires it.

When a harness loads an import:

1. Resolve the import id in the current spec's top-level `imports` map.
2. Load every id in `requires.imports` first, in listed order.
3. Then load the requested import.
4. Skip duplicate loads during one decision run unless the harness intentionally
   needs a fresh read.

Markdown links inside imported files are just prose links. A human or agent may
choose to follow them, resolving them relative to that Markdown file, but
following them is outside SkillSpec import semantics unless the target is also
declared as an import.

Cycles are invalid:

```yaml
imports:
  a:
    path: a.md
    role: reference
    requires:
      imports: [b]
  b:
    path: b.md
    role: reference
    requires:
      imports: [a]
```

## Import Or Resource

Use an import when:

- the agent must read the file as active task guidance
- the file is loaded only for a route, rule, recipe, or code path
- the file is shared policy such as plugin-level operating rules
- the file is a branch-specific reference like `references/task-routing.md`

Use a resource when:

- the file is source provenance from a prose skill
- the file is an asset, fixture, script, example output, or evidence document
- the file supports code provenance but should not be loaded as guidance
- preserving the file matters, but loading it is not part of runtime behavior

Code provenance can point at either a `resource` or an `import`. Fenced code
extracted from runtime guidance should use `provenance.import`; fenced code
extracted from preserved source material should use `provenance.resource`.
