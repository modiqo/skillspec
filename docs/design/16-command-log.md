# Command Log

This command log is the design-facing quick scan for the implemented
`skillspec` CLI surface.

It answers four questions for each command:

- What is the command?
- What are the important arguments and options?
- Why does it exist?
- What does a realistic invocation look like?

This document is not the formal command reference. The source of truth remains
the clap command tree in `crates/skillspec-cli/src/cli/args.rs` and the reference
inventory in `spec/commandspec.md`. If this table disagrees with the CLI help or
reference spec, this table is wrong.

## Runtime Loop Commands

These are the commands an agent or harness uses during a SkillSpec-backed run.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec sensemake` | `<path>`, `--view index,summary,full`, `--json` | Teaches the shape of one spec without loading the full YAML. Use it when the spec is unfamiliar and the agent needs section ids and navigation handles. | `skillspec sensemake ./skill.spec.yml --view index` |
| `skillspec decide` | `<path>`, `--input <text>`, `--trace-dir <dir>` | Evaluates routing rules for a user task and emits the selected route, matched rules, forbids, elicitations, and after-success work as JSON. | `skillspec decide ./skill.spec.yml --input 'install this skill' --trace-dir .skillspec/traces` |
| `skillspec plan` | `<path>`, `--input <text>`, `--trace-dir <dir>`, `--json` | Lists the selected route's execution phases in order and writes a decision trace when `--trace-dir` is supplied. This is the pre-action phase-order view. | `skillspec plan ./skill.spec.yml --input 'port this skill' --trace-dir .skillspec/traces` |
| `skillspec act` | `<path>`, `--input <text>`, `--trace-dir <dir>`, `--run <run-dir>`, `--phase <id>`, `--json` | Expands the selected route and current phase into an OODA action checklist, including matched rules, allowed actions, forbids, transitions, handoffs, and the effective phase tool boundary. | `skillspec act ./skill.spec.yml --input 'port this skill' --run .skillspec/traces/run-123 --phase qa_and_proof` |
| `skillspec progress record` | `<run-dir>`, `<event>`, `[phase]`, `[requirement]`, `--id <id>`, `--status <status>`, `--evidence-kind <kind>`, `--evidence-ref <ref>`, `--source-skill <id>`, `--message <text>`, `--json` | Appends one structured event to `<run-dir>/execution.jsonl`. This is how the harness records phase, requirement, route, handoff, closure, and evidence proof. | `skillspec progress record .skillspec/traces/run-123 requirement-satisfied qa_and_proof validate_spec --evidence-kind command --evidence-ref validate.log` |
| `skillspec progress stats` | `<run-dir>`, `--workspace <name>`, `--workspace-stats-report <path>`, `--workspace-stats-json <path>`, token metric flags, `--agent-visible-tokens <n>`, `--artifact-tokens-preserved <n>`, `--avoided-tokens <n>`, `--metrics-source <source>`, `--phase <id>`, `--requirement <id>`, `--message <text>`, `--json` | Appends a `stats_collected` event with workspace/token metrics so `trace align` can report measured consumption/savings or non-Rote estimated output economy from `--summary` blocks. | `skillspec progress stats .skillspec/traces/run-123 --agent-visible-tokens 190 --artifact-tokens-preserved 96190 --avoided-tokens 96000 --metrics-source estimated` |
| `skillspec progress final-response` | `<run-dir>`, `--result`, `--evidence`, `--alignment`, `--token-savings`, `--phase <id>`, `--requirement <id>`, `--message <text>`, `--json` | Appends `final_response_sent` proof that the final answer includes result, evidence, alignment, and token math sections. | `skillspec progress final-response .skillspec/traces/run-123 --result --evidence --alignment --token-savings` |
| `skillspec progress show` | `<path>`, `--run <run-dir>`, `--json` | Reads the decision trace plus `execution.jsonl`, derives `progress.json`, and reports completed, current, blocked, and remaining phases plus open requirements. | `skillspec progress show ./skill.spec.yml --run .skillspec/traces/run-123` |
| `skillspec trace align` | `<path>`, `--decision-trace <run-dir>`, `--execution-trace <jsonl>`, `--json` | Replays the decision trace against the current spec and checks structured execution evidence for obligations. Writes `<run-dir>/alignment.json`. | `skillspec trace align ./skill.spec.yml --decision-trace .skillspec/traces/run-123 --execution-trace .skillspec/traces/run-123/execution.jsonl` |

## Authoring And QA Commands

These commands help authors create, inspect, validate, test, compile, and install
SkillSpec-backed skills.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec validate` | `<path>` | Parses and validates a `skill.spec.yml` file against the typed grammar, parser checks, identifiers, and cross-references. | `skillspec validate examples/durable-executor/skill.spec.yml` |
| `skillspec test` | `<path>` | Runs scenario tests declared in the spec against the decision engine. | `skillspec test examples/durable-executor/skill.spec.yml` |
| `skillspec explain` | `<path>`, `--input <text>`, `--trace-dir <dir>` | Explains the routing decision for a task in human-facing form and optionally records decision trace events. | `skillspec explain ./skill.spec.yml --input 'browse gmail' --trace-dir .skillspec/traces` |
| `skillspec query` | `<path>`, `<handle>`, `--view index,summary,full`, `--json` | Retrieves one collection, item, or field path from the spec. Use it for progressive detail instead of reading the whole YAML. | `skillspec query ./skill.spec.yml command:validate_spec.requires --view summary` |
| `skillspec refs` | `<path>`, `<handle>`, `--view index,summary,full`, `--json` | Shows outgoing references for an item handle, such as a route's checks, a command's dependencies, or a rule's preferred route. | `skillspec refs ./skill.spec.yml route:local_skill_port --view summary` |
| `skillspec source map` | `<path>`, `--out <dir>`, `--json` | Builds `source-map.json` and `source-map.md` from a prose skill file or folder using Markdown AST positions. Use it before importing large or resource-heavy skills so the agent can query exact source spans instead of loading the whole source. | `skillspec source map ./source-skill --out ./draft/.skillspec/source-map` |
| `skillspec source query` | `<source-map.json>`, `<handle>`, `--view index,summary,full`, `--json` | Retrieves source-map collections or exact nodes. Common handles are `files`, `nodes`, `dependencies`, `code`, `coverage`, `frontmatter:<file>`, `heading:<file>.<slug>`, and `code:<file>.<n>`. | `skillspec source query ./draft/.skillspec/source-map/source-map.json nodes --view index` |
| `skillspec source coverage` | `<source-map.json>`, `--json` | Summarizes mapped nodes, review-required classifications, and stale counts before semantic promotion. | `skillspec source coverage ./draft/.skillspec/source-map/source-map.json` |
| `skillspec source stale` | `<source-map.json>`, `--root <path>`, `--json` | Recomputes file hashes and exits non-zero when mapped source files changed or disappeared. Run before import, proof, or install. | `skillspec source stale ./draft/.skillspec/source-map/source-map.json --root ./source-skill` |
| `skillspec workspace map` | `<source-root>`, `--out <skillspec.workspace.yml>`, `--summary`, `--json` | Authoring-side structure recon for multi-skill repositories. Discovers folders with `SKILL.md`, detects plugin-shaped namespace roots (`skills/` plus `.claude-plugin/plugin.json`, `.mcp.json`, or `CLAUDE.md`), assigns package ids, skill-safe public names, and deterministic install slugs, scans Markdown for file and slash-command references, infers hard `depends_on` edges from file references, reports duplicate public names/install slugs, and writes a manifest plus markdown report. Plugin slash-command links are workflow references, not hard dependency edges. This is not router indexing. `--summary` prints wall-clock and estimated token metrics while preserving full proof files. | `skillspec workspace map ./skills --out ./build/skillspec.workspace.yml --summary` |
| `skillspec workspace validate` | `<skillspec.workspace.yml>`, `--summary`, `--json` | Validates a workspace package graph before fanout import. Checks source root, package paths, exactly one `SKILL.md` per package, dependency references, self-dependencies, cycles, install slug uniqueness, and uncovered hard cross-package references. Duplicate public names are warnings until install planning. Plugin slash-command workflow references may cross packages without `depends_on`; file references still require dependency coverage. `--summary` prints wall-clock and estimated token metrics. | `skillspec workspace validate ./build/skillspec.workspace.yml --summary` |
| `skillspec workspace import` | `<skillspec.workspace.yml>`, `--out <build-root>`, `--summary`, `--json` | Runs fanout import for every package in a validated workspace graph. Processes packages in topological order through the existing single-package doctor, source map, and import-skill pipeline, writes outputs under one mirrored build root, preserves successful package outputs when another package fails, and reports built, failed, skipped, and blocked packages. It does not compile, install, or refresh router indexes. `--summary` keeps stdout compact and reports preserved artifact tokens. | `skillspec workspace import ./build/skillspec.workspace.yml --out ./workspace-build --summary` |
| `skillspec workspace converge` | `<skillspec.workspace.yml>`, `--build-root <build-root>`, `--summary`, `--json` | Verifies the generated workspace build against the manifest before compile/install. Checks every package has a ready generated `skill.spec.yml` or explicit failure evidence, validates generated specs and package-local imports/resources, blocks dependents whose dependencies are not ready, and writes `workspace-converge.report.md`. `--summary` prints compact readiness counts plus wall-clock/token metrics. | `skillspec workspace converge ./build/skillspec.workspace.yml --build-root ./workspace-build --summary` |
| `skillspec workspace compile` | `<skillspec.workspace.yml>`, `--build-root <build-root>`, `--target codex-skill,claude-skill`, `--summary`, `--json` | Rechecks convergence, compiles ready package specs into generated `SKILL.md` loaders under the mirrored build root, blocks dependents whose dependencies did not compile, and writes `workspace-compile.report.md`. It does not install skills or refresh router indexes. `--summary` prints compact compile counts plus wall-clock/token metrics. | `skillspec workspace compile ./build/skillspec.workspace.yml --build-root ./workspace-build --target codex-skill --summary` |
| `skillspec workspace install` | `<skillspec.workspace.yml>`, `--build-root <build-root>`, `--target agents,codex,claude-local`, `--all-detected`, `--dry-run`, `--retire-existing`, `--visibility-policy entry-implicit,all-implicit,all-manual,none`, `--apply-visibility`, `--visibility-manifest <path>`, `--summary`, `--json` | Preflights a compiled workspace build and installs packages into harness roots using manifest `install_slug` folders. It blocks missing compiled loaders, folder collisions, public-name collisions, and dependent packages whose dependencies cannot install; dry-run shows every planned write plus intended visibility. By default, entry packages remain implicit and shared/helper/wrapper packages are manual-only when `--apply-visibility` is used. Actual install writes `workspace-install.report.md`, `workspace-install.manifest.json`, and optionally a reversible workspace visibility manifest without refreshing router indexes. `--summary` prints compact install counts plus wall-clock/token metrics. | `skillspec workspace install ./build/skillspec.workspace.yml --build-root ./workspace-build --target codex --apply-visibility --summary` |
| `skillspec doctor` | `<target>`, `--json` | Static diagnostic for exactly one prose skill folder, local or public GitHub. Remote targets are staged through a temporary sparse checkout and cleaned up after the report. Parent folders with multiple `SKILL.md` files are rejected. Reports structural score, activation-loaded surface percentage, instruction-density and primacy-bias risk, code/instruction mixing, implicit dependency contracts, missing references, missing behavior contract, and missing proof/trace surface. It reads files only; dynamic behavior remains unproven until trace alignment. | `skillspec doctor https://github.com/anthropics/skills/tree/main/skills/pdf --json` |
| `skillspec grammar sensemake` | `--view index,summary,porting,full`, `--json` | Teaches the embedded grammar artifact progressively. Use before importing or revising a spec so the harness does not infer grammar from memory. | `skillspec grammar sensemake --view porting` |
| `skillspec grammar checklist` | `--for <subject>`, `--json` | Shows the embedded coverage checklist for a semantic porting or review workflow. | `skillspec grammar checklist --for import-skill` |
| `skillspec grammar schema` | `--json` | Prints or summarizes the embedded JSON Schema used by grammar-aware harnesses and reviewers. | `skillspec grammar schema --json` |
| `skillspec imports check` | `<path>` | Validates declared local imports, sections, and dependency-first load order. | `skillspec imports check ./skill.spec.yml` |
| `skillspec deps check` | `<path>`, `--command <id>` | Checks declared dependencies for the whole spec or for one command. Local checks can pass or fail; harness-specific checks are reported as deferred. | `skillspec deps check ./skill.spec.yml --command validate_spec` |
| `skillspec compile` | `<path>`, `--target codex-skill,claude-skill,markdown` | Compiles a spec into harness guidance or a full Markdown rendering. Generated skill loaders point agents back to the colocated spec and runtime commands. | `skillspec compile ./skill.spec.yml --target codex-skill` |
| `skillspec import-skill` | `<path>`, `--out <path>`, `--source-map <path>` | Creates a mechanical draft `skill.spec.yml` from a local `SKILL.md` file or single skill folder. Parent folders with multiple `SKILL.md` files are rejected; run `skillspec workspace map` first so SkillSpec can identify atomic packages, plugin namespaces, hard dependency edges, workflow references, and name collisions. For large or code-heavy sources, run `skillspec source map`, inspect `source coverage`, query `nodes`, `dependencies`, and `code`, then pass the fresh `source-map.json` with `--source-map`; the import refuses stale maps. The generated draft is scaffolding, not a finished semantic port. The original prose is preserved as `source/SKILL_md.old`, deliberately not as `SKILL.md` or Markdown. Fenced code is materialized under `resources/imported-code/` and referenced from the draft. The importer also writes `deps.toml`, declares it as a file dependency/artifact, and seeds it with inferred CLI plus Python/JavaScript/TypeScript package imports; the review pass must complete the ledger with source authority, local status, install risk, and degraded proof impact. A reviewed zero-dependency skill keeps `dependency_count = 0`; a byte-empty ledger is invalid. Do not delete dependency mentions to make QA pass. | `skillspec import-skill ./source-skill --out ./draft/skill.spec.yml --source-map ./draft/.skillspec/source-map/source-map.json` |
| `skillspec synthesize-from-workspace` | `<workspace>`, `--out <folder>`, `--task <text>`, `--workspace-stats-report <path>`, `--workspace-log <path>`, `--workspace-meta <path>`, `--workspace-deps <path>`, `--log-last <n>`, `--observation-approved`, `--force`, `--json` | Rote-specific optional integration that creates a draft SkillSpec scaffold from durable rote workspace evidence. It refuses to write until the observed result and evidence summary have been shown and approved via `--observation-approved`. Required evidence is workspace stats, command log, and metadata; these can be collected live or supplied as explicit files when rote workspace lookup is unreliable. | `skillspec synthesize-from-workspace profile-enrichment --task 'use parallel web to enrich this profile' --out ./draft-profile-skill --observation-approved` |
| `skillspec trace compact` | `<run-dir>` | Rebuilds `trace.jsonl` and `summary.json` from append-only trace event files in a run directory. | `skillspec trace compact .skillspec/traces/run-123` |
| `skillspec install targets` | none | Lists detected harness skill roots, such as Codex, Agents, or Claude local skill directories. | `skillspec install targets` |
| `skillspec install skill` | `<folder>`, `--target agents,codex,claude-local`, `--all-detected`, `--dry-run`, `--name <name>`, `--force`, `--retire-existing` | Installs a generated skill folder containing `SKILL.md`, `skill.spec.yml`, and declared package-local files from imports, resources, code sources, and file dependencies into one or more detected harness roots. Use `--dry-run` before writing. When replacing an existing active prose skill, use `--retire-existing`: it backs up the old skill under `SKILLSPEC_HOME/backups/retired-skills` or `~/.skillspec/backups/retired-skills`, removes it from harness discovery, then installs the reviewed replacement at the same name. `--force` and `--retire-existing` are mutually exclusive. | `skillspec install skill examples/pdf --target agents --target codex --dry-run --retire-existing` |

## Durable Executor Lifecycle Commands

These commands manage the optional SkillSpec-owned durable first-hop skill. They
are separate from ordinary `install skill` so update and delete can operate only
on recorded, managed durable-executor installs.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec durable-executor install` | `<source-folder>`, `--target agents,codex,claude-local`, `--all-detected`, `--dry-run`, `--force`, `--json` | Preflights that `rote` is on `PATH`, installs durable-executor from an explicit local source folder, writes a managed marker, records source and install dirs under `~/.skillspec/durable-executor/config.json`, and refreshes router visibility/index when router mode is configured. `--dry-run` reports the preflight without writing files. | `skillspec durable-executor install examples/durable-executor --target agents --json` |
| `skillspec durable-executor update` | `--source <source-folder>`, `--backup-dir <path>`, `--dry-run`, `--json` | Preflights that `rote` is on `PATH`, backs up durable config and every recorded managed durable-executor folder, rewrites marker-protected folders from the recorded source or `--source`, refreshes router state when configured, and warns to restart active harness sessions. It refuses an existing unmarked folder. | `skillspec durable-executor update --json` |
| `skillspec durable-executor enable` | `--dry-run`, `--json` | Keeps durable-executor installed, checks `rote` on `PATH`, and switches recorded installs back to implicit invocation across Codex and Claude visibility metadata; refreshes router state when router mode is configured and enabled. | `skillspec durable-executor enable --json` |
| `skillspec durable-executor disable` | `--dry-run`, `--json` | Keeps durable-executor installed but makes recorded installs explicit-only across Codex and Claude visibility metadata; refreshes router state when router mode is configured and enabled. | `skillspec durable-executor disable --json` |
| `skillspec durable-executor delete` | `--dry-run`, `--json`; alias: `uninstall` | Deletes only recorded durable-executor folders that contain the managed marker, removes durable config, and refreshes router state when configured. It refuses unmarked folders. | `skillspec durable-executor delete --json` |

## Skill Router Commands

These commands support large skill catalogs without putting every skill
description into the model context.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec index` | `--roots <path>...`, `--out <index-file-or-router-dir>`, `--visibility-manifest <path>`, `--json` | Builds the router-specific local SQLite skill catalog used by `skillspec route` and the optional skill-router. Directory paths resolve to `skill-index.sqlite`. This is runtime skill discovery, not source analysis, workspace recon, or skill import. Direct execution prints warnings that explain router state: no installed router config means standalone manual lookup only; disabled router mode means the index will not affect implicit skill selection until `skillspec router enable`; enabled router mode should usually use `skillspec router index refresh` so visibility controls and preparedness checks run too. | `skillspec index --roots ~/.agents/skills --out ~/.skillspec/router --json` |
| `skillspec route` | `--index <index-file-or-router-dir>`, `--query <text>`, `--top <n>`, `--execution-mode direct,durable`, `--json` | Scores candidate skills from the index and returns the selected skill path, candidates, confidence, visibility, and optional direct/durable elicitation hint. Directory paths resolve to `skill-index.sqlite`. | `skillspec route --index ~/.skillspec/router --query 'extract text from a pdf' --json` |
| `skillspec skills audit` | `--roots <path>...`, `--json` | Audits routing metadata for overlong descriptions, vague descriptions, missing negative boundaries, and duplicate names. | `skillspec skills audit --roots ~/.agents/skills --json` |
| `skillspec skills set-visibility` | `<skill> <implicit,manual-only,name-only,off>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Sets one skill's conceptual visibility using native Codex/Claude controls and records a reversible manifest. | `skillspec skills set-visibility pdf manual-only --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec skills disable` | `<skill>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Convenience command for `set-visibility <skill> off`; off skills are excluded from router results when the manifest is used. | `skillspec skills disable legacy-skill --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec skills enable` | `<skill>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Convenience command for `set-visibility <skill> implicit`. | `skillspec skills enable pdf --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec status` | `--roots <path>...`, `--json` | Read-only installation inventory: reports router and durable-executor installed/enabled/disabled state, supported and scanned roots, router index exists/stale/updated data, and SkillSpec-backed versus legacy prose skills by name/path/visibility. Without `--roots`, scans router config roots when available, otherwise detected harness roots. | `skillspec status --json` |
| `skillspec visibility plan` | `--roots <path>...`, `--profile router-managed`, `--json` | Shows the native visibility changes router install would apply without editing files. | `skillspec visibility plan --roots ~/.agents/skills ~/.claude/skills --json` |
| `skillspec visibility apply` | `--roots <path>...`, `--profile router-managed`, `--manifest <path>`, `--dry-run`, `--json` | Applies native Codex `agents/openai.yaml`, Claude `skillOverrides`, and Claude `disable-model-invocation` frontmatter for shared `.agents` roots, then writes a rollback manifest. | `skillspec visibility apply --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json --json` |
| `skillspec visibility restore` | `--manifest <path>`, `--dry-run`, `--json` | Restores exact file snapshots from a visibility manifest. It does not infer previous state. | `skillspec visibility restore --manifest ~/.skillspec/router/visibility-manifest.json --json` |
| `skillspec router install` | `--roots <path>...`, `--index <index-file-or-router-dir>`, `--manifest <path>`, `--router-name <name>`, `--dry-run`, `--json` | Installs the SkillSpec-backed `skill-router` skill into every configured root, enables router mode, makes router implicit, makes routed skills explicit-only except an enabled durable-executor, builds the index, checks post-index preparedness, and writes router config with all managed router skill directories for future hooks. Directory paths resolve to `skill-index.sqlite`. | `skillspec router install --roots ~/.agents/skills --index ~/.skillspec/router --json` |
| `skillspec router enable` | `--dry-run`, `--json` | Re-enables an installed router, refreshes managed router skill files, makes router implicit, makes routed skills explicit-only except an enabled durable-executor, rebuilds the index from current roots, checks preparedness, writes `enabled: true`, and warns to restart harnesses. | `skillspec router enable --json` |
| `skillspec router disable` | `--dry-run`, `--json` | Keeps router installed but disables router mode, makes router explicit-only, restores routed skills to implicit/default visibility across recorded Codex and Claude roots, writes `enabled: false`, and warns to restart harnesses. | `skillspec router disable --json` |
| `skillspec router update` | `--backup-dir <path>`, `--dry-run`, `--json` | Reads the existing router config, backs up config, manifest, index, and managed router skill directories, rewrites the SkillSpec-backed router package in every recorded harness root, preserves enabled/disabled mode, reapplies matching visibility, rebuilds the index only when enabled, and warns that active harness sessions should be restarted. | `skillspec router update --json` |
| `skillspec router uninstall` | `--manifest <path>`, `--index <index-file-or-router-dir>`, `--keep-index`, `--dry-run`, `--json`; alias: `delete` | Restores visibility from the manifest, removes every managed router skill marker directory recorded by router config, removes config, and optionally removes the index. Directory paths resolve to `skill-index.sqlite`. | `skillspec router uninstall --json` |
| `skillspec router index refresh` | `--roots <path>...`, `--index <index-file-or-router-dir>`, `--visibility-manifest <path>`, `--json` | Repairs router-managed state after skill additions, removals, or metadata changes. When router config is present and enabled, it reapplies explicit-only native controls across roots while preserving an enabled `durable-executor` as implicit, then rebuilds the index and reports preparedness plus pre-refresh advice. When disabled, it rebuilds the index without re-enabling visibility. Directory paths resolve to `skill-index.sqlite`. | `skillspec router index refresh --roots ~/.agents/skills --index ~/.skillspec/router --visibility-manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec router index status` | `--roots <path>...`, `--index <index-file-or-router-dir>`, `--visibility-manifest <path>`, `--json` | Compares the index against current roots and reports new, changed, missing, stale, and updated-at state. New and changed entries include whether they are prose-only or SkillSpec-backed; prose-only entries include `skillspec import-skill` conversion advice. Directory paths resolve to `skill-index.sqlite`. | `skillspec router index status --roots ~/.agents/skills --index ~/.skillspec/router --json` |

## Capability Bootstrap Commands

These commands manage local capability seeds used by durable bootstrap flows when
no domain SkillSpec exists yet.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec capability store` | none | Shows the local capability seed store path. Defaults under `~/.skillspec/capabilities/`, or under `SKILLSPEC_HOME` when set. | `skillspec capability store` |
| `skillspec capability add` | `<id>`, `--domain <domain>`, `--kind <kind>`, `--provides <capability>`, plus seed metadata flags | Creates or rewrites a local capability seed for a CLI, adapter, script, or other reusable substrate. | `skillspec capability add preferred-voice-cli --domain voice --kind cli --provides text_to_speech --command say` |
| `skillspec capability update` | `<id>`, `--domain <domain>`, patch flags such as `--add-provides`, `--remove-provides`, `--priority`, `--mark-failed` | Patches an existing seed without rewriting unspecified fields. Use this to adjust ranking, aliases, auth hints, risk flags, or verification state. | `skillspec capability update preferred-voice-cli --domain voice --priority 70 --add-provides speech_synthesis` |
| `skillspec capability list` | `--domain <domain>` | Lists local capability seeds, optionally scoped to one domain. | `skillspec capability list --domain voice` |
| `skillspec capability search` | `<capability>`, `--domain <domain>`, `--explain`, `--json`, `--local-only`, `--preferred-seed <id>` | Searches and ranks local seeds for a capability. It returns scores, reasons, risk gates, and ask policy when candidates are close. | `skillspec capability search text_to_speech --domain voice --explain --json` |
| `skillspec capability inspect` | `<id>`, `--domain <domain>`, `--json` | Reads one capability seed and reports its stored metadata. | `skillspec capability inspect preferred-voice-cli --domain voice --json` |
| `skillspec capability verify` | `<id>`, `--domain <domain>`, `--json` | Runs declared evidence checks for a seed and updates verification status. | `skillspec capability verify preferred-voice-cli --domain voice --json` |
| `skillspec capability prefer` | `<id>`, `--for <capability>`, `--domain <domain>`, `--priority <0-100>` | Updates preferred capability and priority metadata without editing durable-executor's spec. | `skillspec capability prefer preferred-voice-cli --domain voice --for text_to_speech --priority 90` |
| `skillspec capability remove` | `<id>`, `--domain <domain>` | Removes one local capability seed. | `skillspec capability remove preferred-voice-cli --domain voice` |
| `skillspec capability scan` | none | Scans for seed proposals. This is a discovery helper for local bootstrap work. | `skillspec capability scan` |

## Important Event Values

`skillspec progress record` accepts these event values:

| Event | Meaning | Example |
| --- | --- | --- |
| `phase-started` | A declared phase has begun. | `skillspec progress record .skillspec/traces/run-123 phase-started qa_and_proof` |
| `requirement-started` | Work began for a phase requirement. | `skillspec progress record .skillspec/traces/run-123 requirement-started qa_and_proof validate_spec` |
| `requirement-satisfied` | A phase requirement has structured proof. | `skillspec progress record .skillspec/traces/run-123 requirement-satisfied qa_and_proof validate_spec --evidence-kind command --evidence-ref validate.log` |
| `requirement-failed` | A phase requirement failed. | `skillspec progress record .skillspec/traces/run-123 requirement-failed qa_and_proof test_spec --status fail` |
| `stats-collected` | Workspace or token metrics were recorded. Use `skillspec progress stats` so measured Rote stats or estimated non-Rote summary metrics are populated for alignment. | `skillspec progress stats .skillspec/traces/run-123 --agent-visible-tokens 190 --artifact-tokens-preserved 96190 --avoided-tokens 96000 --metrics-source estimated` |
| `obligation-satisfied` | A route, forbid, elicitation, or other obligation has explicit proof. | `skillspec progress record .skillspec/traces/run-123 obligation-satisfied --id report_alignment_status --status pass` |
| `route-fulfilled` | The selected route was fulfilled. | `skillspec progress record .skillspec/traces/run-123 route-fulfilled --id prove_skill_value --status pass` |
| `after-success-completed` | A scheduled closure completed. | `skillspec progress record .skillspec/traces/run-123 after-success-completed --id trace_align --status pass` |
| `evidence-attached` | Extra evidence was attached to the run. | `skillspec progress record .skillspec/traces/run-123 evidence-attached --evidence-kind file --evidence-ref docs/design/16-command-log.md` |
| `handoff-started` | A declared handoff began. | `skillspec progress record .skillspec/traces/run-123 handoff-started browser_lookup` |
| `handoff-completed` | A declared handoff completed. | `skillspec progress record .skillspec/traces/run-123 handoff-completed browser_lookup --status pass` |
| `phase-completed` | A declared phase completed. | `skillspec progress record .skillspec/traces/run-123 phase-completed qa_and_proof --status pass` |
| `phase-blocked` | A declared phase cannot continue. | `skillspec progress record .skillspec/traces/run-123 phase-blocked install_skill --status blocked --message 'needs approval'` |

## Design Notes

The command surface has three layers:

- Runtime navigation and proof: `sensemake`, `decide`, `plan`, `act`,
  `progress`, and `trace align`.
- Authoring and QA: `validate`, `test`, `query`, `refs`, `source`, `workspace`,
  `grammar`, `imports`, `deps`, `compile`, `import-skill`, and `install`.
- Large skill discovery: `index`, `route`, `skills`, `visibility`, and
  `router`.
- Durable lifecycle and bootstrap: `durable-executor` and `capability`.

The CLI does not execute arbitrary task work. It renders contracts, validates
them, records progress evidence, and aligns traces. The surrounding harness
still owns actual tool execution, approvals, redaction, and substrate policy.

The earlier proposed namespace `skillspec skill port/install/prove/value` is not
documented here because it is not an implemented CLI namespace in the current
binary.

## Source Alignment

This doc is grounded in:

- `crates/skillspec-cli/src/cli/args.rs`, which defines the clap command tree;
- `crates/skillspec-cli/src/cli/dispatch.rs`, which dispatches parsed commands;
- `spec/commandspec.md`, which is the reference command inventory;
- `crates/skillspec-cli/src/execution/act.rs`,
  `crates/skillspec-cli/src/execution/progress.rs`,
  `crates/skillspec-cli/src/execution/align.rs`,
  `crates/skillspec-cli/src/execution/deps.rs`,
  `crates/skillspec-cli/src/spec/grammar.rs`,
  `crates/skillspec-cli/src/spec/imports.rs`,
  `crates/skillspec-cli/src/features/workspace.rs`,
  `crates/skillspec-cli/src/features/compiler.rs`,
  `crates/skillspec-cli/src/features/importer.rs`,
  `crates/skillspec-cli/src/features/capability.rs`,
  `crates/skillspec-cli/src/lifecycle/install.rs`,
  `crates/skillspec-cli/src/lifecycle/router.rs`,
  `crates/skillspec-cli/src/lifecycle/visibility.rs`,
  `crates/skillspec-cli/src/lifecycle/router_lifecycle.rs`, and
  `crates/skillspec-cli/src/lifecycle/durable_lifecycle.rs`, which implement
  the listed command behavior;
- `skillspec --help` and subcommand help output from the current local binary.
