# One-Shot Porting Workflow

The import path must avoid freehand YAML. The failure mode is predictable:
agents remember an older or imagined SkillSpec shape, patch a large YAML block,
then spend cycles fixing scalar/list, enum, trace, state, permission, and syntax
errors one at a time.

One-shot porting makes the safe path the default path.

## Goals

- derive YAML shape from the embedded grammar/schema before import;
- generate the draft with typed Rust structures, not handwritten YAML snippets;
- run source mapping, mechanical import, validation, imports, dependencies,
  scenario tests, and compile as one QA gate;
- record estimated non-Rote token metrics when a trace run is supplied;
- emit one compact report with artifact paths instead of dumping full proof.

## Non-Goals

- do not claim a mechanically imported skill is semantically complete;
- do not live-run imported runtime behavior unless runtime dependencies exist;
- do not hide dependency gaps by deleting dependency evidence;
- do not replace workspace map/import for parent folders with multiple skills.

## SOP

1. Grammar preflight
   - use embedded grammar sensemaking, checklist, and JSON Schema;
   - write those artifacts under the draft proof directory.

2. Source preflight
   - guard that the source is one atomic skill package;
   - run source map;
   - run doctor;
   - report file, code block, dependency, and review-required counts.

3. Mechanical draft
   - run the existing importer with the fresh source map;
   - materialize preserved source, imported files, code resources, and
     `deps.toml`;
   - write `skill.spec.yml` from typed Rust structs.

4. Shape crib
   - generate a known-valid representative YAML crib from the current typed
     model;
   - include route execution plan, scalar `prefer`, list `elicit`, state
     `do/next`, trace `mode|required|record`, dependency permission, safety
     enum, and scenario test shapes.

5. QA ladder
   - validate;
   - imports check;
   - deps check;
   - scenario tests;
   - compile to the requested target.

6. Metrics
   - compute compact-output and artifact-preservation estimates;
   - if `--run-dir` is supplied, append a `stats_collected` progress event with
     `metrics_source: estimated`;
   - never leave non-Rote port proof with token usage silently unrecorded when
     metrics were available.

## Command Shape

```bash
skillspec port-one-shot ./source-skill \
  --out ./draft-skill \
  --target codex-skill \
  --prove \
  --run-dir .skillspec/traces/run-123 \
  --phase import_skill \
  --requirement estimated_token_metrics
```

The command writes:

- `<out>/skill.spec.yml`
- `<out>/.skillspec/source-map/source-map.json`
- `<out>/.skillspec/source-map/source-map.md`
- `<out>/.skillspec/port/grammar-porting.md`
- `<out>/.skillspec/port/grammar-checklist.md`
- `<out>/.skillspec/port/schema.json`
- `<out>/.skillspec/port/shape-crib.yml`
- `<out>/.skillspec/port/doctor.json`
- `<out>/.skillspec/port/compiled.<target>.md`
- `<out>/.skillspec/port/port-one-shot.report.md`

## Failure Handling

Failures are grouped by gate instead of iterated one command at a time:

- `source`: invalid package shape, stale map, missing source;
- `draft`: importer or materialization failure;
- `validate`: grammar/schema/load failure;
- `imports`: missing import files or sections;
- `deps`: missing local dependencies or empty dependency ledger;
- `test`: scenario expectation mismatch;
- `compile`: target rendering failure;
- `stats`: progress metric recording failure.

The command should preserve partial artifacts and return non-zero on failed
required gates when `--prove` is set.

## First Implementation

The first implementation should be conservative:

- support local single-skill folders or `SKILL.md` files only;
- refuse multi-skill parent folders and direct users to `workspace map`;
- overwrite only with `--force`;
- use current importer scaffolding as the semantic baseline;
- mark semantic promotion as review-required in the report;
- record estimated token metrics only when a run directory is supplied.
