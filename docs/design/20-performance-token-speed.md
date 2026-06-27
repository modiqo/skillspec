# Performance, Token Economy, And Incremental Processing

SkillSpec has two different performance goals.

1. Reduce wall-clock time for import, compile, install, and runtime planning.
2. Reduce model-facing tokens without hiding the proof needed to trust a run.

These goals overlap, but they are not the same. A compact `--summary` output can
save tokens while the CLI still performs the same local work. A cache hit can
save wall-clock time while preserving the same proof files on disk. This design
keeps those claims separate in metrics and docs.

## Current Baseline

The current workspace flow already keeps agent-facing output compact:

```bash
skillspec workspace map ./skills --out ./build/skillspec.workspace.yml --summary
skillspec workspace import ./build/skillspec.workspace.yml --out ./workspace-build --summary
skillspec workspace converge ./build/skillspec.workspace.yml --build-root ./workspace-build --summary
skillspec workspace compile ./build/skillspec.workspace.yml --build-root ./workspace-build --target codex-skill --summary
skillspec workspace install ./build/skillspec.workspace.yml --build-root ./workspace-build --target codex --dry-run --summary
```

Those summaries report:

- wall-clock time;
- estimated agent-visible tokens;
- estimated artifact tokens preserved on disk;
- estimated avoided tokens;
- cache hit/miss counts when a command uses cache evidence.

The estimates are direct-run output-economy metrics. They are not model API
token measurements. Durable-executor workspace stats remain the measured token
path when they exist.

## Design Targets

### 1. Persistent Loaded-Spec Cache

Repeated commands such as `sensemake`, `decide`, `plan`, `act`, `query`, `refs`,
`deps check`, `imports check`, and `compile` often load the same
`skill.spec.yml` across separate CLI invocations. The parser should support a
small persistent cache keyed by:

- CLI cache schema and binary version;
- canonical spec path;
- spec file hash;
- sidecar dependency surface needed for package validation.

The cache stores the parsed `SkillSpec` as JSON under the package-local cache
root:

```text
<skill-dir>/.skillspec/cache/spec-cache.json
```

The cache must be conservative:

- parse and validate normally on cache miss;
- reuse only when the source hash and cache schema match;
- re-run import and package-sidecar validation before returning the spec;
- never skip validation for externally referenced imports/resources.

This saves YAML parse/deserialization work across common command loops without
changing correctness boundaries.

### 2. Command Batching For Common Loops

Agents frequently run the same sequence:

```bash
skillspec sensemake ./skill.spec.yml --view index
skillspec decide ./skill.spec.yml --input "..."
skillspec plan ./skill.spec.yml --input "..."
skillspec act ./skill.spec.yml --input "..." --phase <phase>
```

Running each command as a fresh process repeats argument parsing, spec loading,
validation, and rendering setup. SkillSpec should use a batching command for the
common planning loop. The preferred agent-facing form is guided, because it also
persists resume anchors and prints only the current gate:

```bash
skillspec run-loop <skill.spec.yml> \
  --input "<task>" \
  --trace-dir .skillspec/traces \
  --guide agent
```

The command should load the spec once, then emit a compact report with:

- sensemake navigation;
- decision result;
- plan;
- first actionable phase or current resume gate;
- guide-state and guide-summary files for compaction recovery;
- report paths and metrics.

The command is a convenience wrapper, not a new runtime. It must not execute
tools or mutate external systems. It only batches existing read/planning
commands so agents spend fewer CLI calls and fewer tokens.

### 3. Incremental Workspace Cache

Workspace import is the largest local hot path because every package currently
runs doctor, source map, importer, validation, and report writing on every run.

The build root should contain an incremental cache:

```text
<build-root>/.skillspec/workspace-cache.json
```

Cache keys must include:

- cache schema and CLI version;
- workspace manifest hash;
- command options;
- package source file hashes;
- generated spec hash;
- output artifact paths.

When an unchanged package has all required output artifacts present,
`workspace import` can skip doctor/source-map/import work and report:

```yaml
status: cached
proof:
  cached: true
  reason: source_hash_unchanged
```

The summary metrics must report `cache_hits` and `cache_misses`. Cache hits
should still be visible in package reports and should still preserve proof paths.

### 4. Parallel Fanout With Sequential Gates

Workspace packages form a dependency graph. Packages in the same topological
level can be imported in parallel when:

- all hard dependencies are already built or cached;
- outputs are package-local under the build root;
- no package writes shared files other than the final workspace report/cache.

Sequential gates remain mandatory for:

- workspace map;
- workspace validate;
- dependency failure propagation;
- final workspace report/cache writes;
- compile/install phases that depend on prior readiness.

The importer should process independent ready packages concurrently using a
bounded worker count derived from available parallelism. Reports must remain
deterministic by sorting package results before rendering.

### 5. Markdown AST And Source Reuse

The source-map builder already uses Markdown AST parsing, but the import flow
still reads and analyzes source material more than once:

- source map reads files, hashes them, and parses Markdown;
- importer reads Markdown again to synthesize and materialize the draft.

The optimized path should let the importer reuse source-map evidence when a
fresh source map is available. For very large Markdown files, the source map
should keep compact node previews and handles in agent-facing output while full
content remains on disk. Full-file AST parsing is acceptable for exact
correctness in v0, but the command surface should avoid printing full parsed
trees unless `--view full` or `--json` is explicitly requested.

## Implementation Phases

### Phase 1: Metrics And Design Alignment

- Keep `--summary` outputs compact.
- Add cache hit/miss fields to summaries.
- Document that direct-run token values are estimated output economy.

### Phase 2: Persistent Spec Cache

- Add a parser cache module.
- Use it in spec-loading commands.
- Keep validation conservative by validating sidecars after cache load.

### Phase 3: Workspace Incremental Cache

- Add `workspace-cache.json` under the build root.
- Hash package source files and manifest content.
- Skip unchanged packages with intact artifacts.
- Surface cached packages in JSON, reports, and summaries.

### Phase 4: Parallel Workspace Import

- Process independent package levels in parallel.
- Keep deterministic report ordering.
- Keep dependency-bound and mutating phases sequential.

### Phase 5: Batch Planning Loop

- Add `skillspec run-loop`.
- Load the spec once.
- Produce sensemake, decision, plan, first phase act output, and metrics in one
  compact report.

### Phase 6: Source Reuse

- Let import use a fresh source map where possible.
- Avoid repeated source reads for package metadata already present in source-map
  evidence.
- Preserve full proof files on disk while keeping stdout compact.

## Non-Goals

- Do not make SkillSpec a general task executor.
- Do not hide failed proof behind a cache hit.
- Do not parallelize install writes unless collision and visibility semantics are
  proven safe.
- Do not claim measured token savings from estimated summary output.
- Do not change the simple single-skill path except by making repeated loads
  cheaper.

## QA Gate

Every performance change must prove:

- unchanged behavior for simple single-skill import/compile/install;
- deterministic workspace reports;
- cache invalidation when source files change;
- cache bypass when required artifacts are missing;
- correct dependency blocking when an upstream package fails;
- command help and command log updates for any new command or flag;
- preflight with `cargo fmt`, `cargo check`, focused tests, and at least one
  real workspace smoke test when feasible.
