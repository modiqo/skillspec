# Router Mode

Router mode is the answer to skill explosion. It lets a workspace keep many
skills installed without loading every skill description into the prompt as an
implicit candidate.

## Context Burden Reduced

Router mode moves skill discovery out of the prompt and into a local catalog:

- installed skills remain on disk;
- native implicit discovery is narrowed;
- route ranking happens against `skill-index.sqlite`;
- the prompt receives selected skill handles and candidates, not every skill
  description.

## 1. The Problem

Without router mode, more skills means more context pressure and more ambiguous
native skill selection.

```mermaid
flowchart LR
    A[many installed skills] --> B[large native skill inventory]
    B --> C[context burden]
    B --> D[ambiguous implicit selection]
    C --> E[missed or noisy routing]
    D --> E
```

Review check:

- The problem is not that skills are bad.
- The problem is uncontrolled implicit discovery at scale.

## 2. Router Install Changes Visibility

Router install creates a managed router skill, applies native visibility
controls, builds an index, and records a reversible manifest.

```mermaid
flowchart LR
    A[skill roots] --> B[router install]
    B --> C[skill-router skill]
    B --> D[visibility manifest]
    B --> E[skill index]
    B --> F[router config]
    B --> G[explicit-only skills]
```

Grounded command:

```sh
skillspec router install \
  --roots <skill-root>... \
  --index <router-index>
```

Review check:

- Router skill is generated in each configured root.
- Router config records managed roots and router skill dirs.
- Visibility is manifest-backed for restore.
- `durable-executor` remains implicit only when already installed.

## 3. Runtime Routing Uses The Index

The router does not execute domain work. It ranks candidates from the local index
and returns the selected skill path plus candidates and confidence.

```mermaid
flowchart LR
    A[user task] --> B[skillspec route]
    C[skill index] --> B
    B --> D[selected skill]
    B --> E[candidates]
    B --> F[confidence]
    D --> G[harness loads selected skill explicitly]
```

Grounded command:

```sh
skillspec route \
  --index <router-index> \
  --query '<user task>' \
  --json
```

Review check:

- Router chooses; it does not perform the selected skill's task.
- The selected skill still owns its own SkillSpec or prose contract.
- Durable execution remains a separate execution policy.

## 4. Out-Of-Band Skills Are Repaired

Skills can be added outside `skillspec install skill`. Router mode detects and
repairs that drift.

```mermaid
flowchart LR
    A[out-of-band skill added] --> B[index status]
    B --> C[new or changed skill]
    C --> D[prose-only advice]
    C --> E[SkillSpec-backed direct index]
    B --> F[index refresh]
    F --> G[visibility reapplied]
    F --> H[index rebuilt]
```

Grounded commands:

```sh
skillspec router index status --roots <skill-root>... --index <router-index> --json
skillspec router index refresh --roots <skill-root>... --index <router-index> --json
```

Review check:

- Status is read-only.
- Refresh reapplies explicit-only controls.
- Prose-only skills are indexed but receive conversion advice.
- Missing skills are reported as drift instead of silently ignored.

## 5. Router Has Its Own Lifecycle

Router mode is managed state, not a loose folder copy.

```mermaid
flowchart LR
    A[install] --> B[config + marker + manifest + index]
    B --> C[update with backup]
    B --> D[delete/uninstall]
    C --> E[restart harness warning]
    D --> F[restore visibility]
```

Grounded commands:

```sh
skillspec router update --json
skillspec router delete --json
```

Review check:

- Update starts from saved router config.
- Delete removes only generated router skills with the managed marker.
- Active harness sessions should restart after mutation.

## What This Workflow Does Not Do

- It does not execute the selected skill's work.
- It does not silently install durable-executor.
- It does not delete ordinary skills.
- It does not make hidden skills unavailable for explicit invocation.

## Mental Model

Router mode turns a growing skill library into an explicit catalog. It reduces
context burden by moving discovery out of the prompt and into a local index.
