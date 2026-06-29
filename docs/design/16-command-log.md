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
| `skillspec sensemake` | `<path>`, `--view index,summary,full`, `--json` | Teaches the shape of one spec without loading the full YAML. Index view is compact and hides exhaustive handle lists; use full view only when exact handles are needed. | `skillspec sensemake ./skill.spec.yml --view index` |
| `skillspec decide` | `<path>`, `--input <text>`, `--trace-dir <dir>` | Evaluates routing rules for a user task and emits the selected route, matched rules, forbids, elicitations, and after-success work as JSON. | `skillspec decide ./skill.spec.yml --input 'install this skill' --trace-dir .skillspec/traces` |
| `skillspec plan` | `<path>`, `--input <text>`, `--trace-dir <dir>`, `--json` | Lists the selected route's execution phases in order and writes a decision trace when `--trace-dir` is supplied. This is the pre-action phase-order view. | `skillspec plan ./skill.spec.yml --input 'port this skill' --trace-dir .skillspec/traces` |
| `skillspec run-loop` | `<path>`, `--input <text>` or `--resume <run-dir>`, `--guide agent,full`, `--view index,summary,full`, `--trace-dir <dir>`, `--phase <id>`, `--json` | Batches sensemake, decide, plan, and the first or requested action checklist in one spec load. With `--guide agent`, writes `guide-state.json` and `guide-summary.md`, then prints start/current/end anchors and the next allowed commands for compaction-safe resume. | `skillspec run-loop ./skill.spec.yml --input 'port this skill' --trace-dir .skillspec/traces --guide agent` |
| `skillspec act` | `<path>`, `--input <text>`, `--trace-dir <dir>`, `--run <run-dir>`, `--phase <id>`, `--json` | Expands the selected route and current phase into an OODA action checklist, including matched rules, allowed actions, forbids, transitions, handoffs, and the effective phase tool boundary. | `skillspec act ./skill.spec.yml --input 'port this skill' --run .skillspec/traces/run-123 --phase qa_and_proof` |
| `skillspec progress record` | `<run-dir>`, `<event>`, `[phase]`, `[requirement]`, `--id <id>`, `--status <status>`, `--evidence-kind <kind>`, `--evidence-ref <ref>`, `--source-skill <id>`, `--message <text>`, `--json` | Appends one structured event to `<run-dir>/execution.jsonl`. This is how the harness records phase, requirement, route, handoff, closure, and evidence proof. | `skillspec progress record .skillspec/traces/run-123 requirement-satisfied qa_and_proof validate_spec --evidence-kind command --evidence-ref validate.log` |
| `skillspec progress stats` | `<run-dir>`, `--workspace <name>`, `--workspace-stats-report <path>`, `--workspace-stats-json <path>`, token metric flags, `--agent-visible-tokens <n>`, `--artifact-tokens-preserved <n>`, `--avoided-tokens <n>`, `--metrics-source <source>`, `--phase <id>`, `--requirement <id>`, `--message <text>`, `--json` | Appends a `stats_collected` event with workspace/token metrics so `trace align --summary` can report measured consumption/savings or direct-run estimated output economy from `--summary` blocks. | `skillspec progress stats .skillspec/traces/run-123 --agent-visible-tokens 190 --artifact-tokens-preserved 96190 --avoided-tokens 96000 --metrics-source estimated` |
| `skillspec progress final-response` | `<run-dir>`, `--result`, `--evidence`, `--alignment`, `--token-savings`, `--phase <id>`, `--requirement <id>`, `--message <text>`, `--json` | Appends `final_response_sent` proof that the final answer includes result, evidence, alignment, and token math sections. | `skillspec progress final-response .skillspec/traces/run-123 --result --evidence --alignment --token-savings` |
| `skillspec progress show` | `<path>`, `--run <run-dir>`, `--json` | Reads the decision trace plus `execution.jsonl`, derives `progress.json`, and reports completed, current, blocked, and remaining phases plus open requirements. Treat as an internal gate check unless details are requested or needed for a blocker/failure. | `skillspec progress show ./skill.spec.yml --run .skillspec/traces/run-123` |
| `skillspec progress batch` | `<run-dir>`, `--file <jsonl-or-json-array>`, `--checkpoint <label>`, `--summary`, `--json`; `--events` remains a compatibility alias | Appends several structured progress/proof events to `execution.jsonl` in one foreground checkpoint and prints a compact `[checkpointing evidence...]` summary when `--summary` is used. Use after dry-run/planning, mutation, verification, route fulfillment, or before final alignment when successful routine proof rows would otherwise create a visible progress parade. | `skillspec progress batch .skillspec/traces/run-123 --file .skillspec/traces/run-123/final-proof.jsonl --checkpoint "checkpointing evidence" --summary` |
| `skillspec trace align` | `<path>`, `--decision-trace <run-dir>`, `--execution-trace <jsonl>`, `--summary`, `--proof-digest <path>`, `--json` | Replays the decision trace against the current spec and checks structured execution evidence for obligations. `--summary` prints only the completion-facing alignment/token block while writing full detail to `<run-dir>/alignment.json`; `--proof-digest` writes grouped missing proof so agents can batch final rows once. | `skillspec trace align ./skill.spec.yml --decision-trace .skillspec/traces/run-123 --execution-trace .skillspec/traces/run-123/execution.jsonl --summary --proof-digest .skillspec/traces/run-123/proof-digest.json` |

## Authoring And QA Commands

These commands help authors create, inspect, validate, test, compile, and install
SkillSpec-backed skills.

| Command | Args And Options | Explanation | Example |
| --- | --- | --- | --- |
| `skillspec validate` | `<path>` | Parses and validates a `skill.spec.yml` file against the typed grammar, parser checks, identifiers, and cross-references. | `skillspec validate examples/durable-executor/skill.spec.yml` |
| `skillspec test` | `<path>` | Runs scenario tests declared in the spec against the decision engine. | `skillspec test examples/durable-executor/skill.spec.yml` |
| `skillspec explain` | `<path>`, `--input <text>`, `--trace-dir <dir>` | Explains the routing decision for a task in human-facing form and optionally records decision trace events. | `skillspec explain ./skill.spec.yml --input 'browse gmail' --trace-dir .skillspec/traces` |
| `skillspec query` | `<path>`, `<handle>`, `--view index,summary,full`, `--json` | Retrieves one collection, item, or field path from the spec. Use handles such as `command:<id>.requires` or `test:<name>.expect` for progressive detail instead of reading the whole YAML. | `skillspec query ./skill.spec.yml command:validate_spec.requires --view summary` |
| `skillspec refs` | `<path>`, `<handle>`, `--view index,summary,full`, `--json` | Shows outgoing references for an item handle, such as a route's checks, a command's dependencies, a rule's preferred route, or a test's expected route/rules/elicitations. | `skillspec refs ./skill.spec.yml test:browse_selects_browser --view summary` |
| `skillspec source stage` | `<github-skill-uri>`, `--out <dir>`, `--no-detect-candidates`, `--json` | Stages a public GitHub repo URL, tree URL, blob-style folder URL, owner/repo shorthand, or owner/repo/path shorthand into a persistent sparse checkout before import. It parses repo, branch, and path; materializes the requested folder or candidate `SKILL.md` package folders; and prints `selected_source_path` when there is one candidate or `candidates[].source_path` when the user must choose. `tree/<branch>/...` is the canonical GitHub folder shape, but `blob/<branch>/...` is accepted when copied from the GitHub UI and the path resolves to a folder rather than `SKILL.md`. Use this before `doctor`, `source map`, `import-skill`, `workspace map`, or `port-one-shot` for URI imports. Do not use web search, raw GitHub URLs, or ad hoc sparse-checkout probing unless this command fails and the user approves troubleshooting. | `skillspec source stage https://github.com/anthropics/skills/tree/main/skills/pdf --out ./.skillspec/staged/pdf --json` |
| `skillspec source map` | `<path>`, `--out <dir>`, `--json` | Builds `source-map.json` and `source-map.md` from a prose skill file or folder using Markdown AST positions for normal files and a chunked heading/code/paragraph mapper for oversized Markdown. Use it before importing large or resource-heavy skills so the agent can query exact source spans instead of loading the whole source. For URI imports, use the local path returned by `skillspec source stage`. | `skillspec source map ./source-skill --out ./draft/.skillspec/source-map` |
| `skillspec source query` | `<source-map.json>`, `<handle>`, `--view index,summary,full`, `--json` | Retrieves source-map collections or exact nodes. Common handles are `files`, `nodes`, `dependencies`, `code`, `coverage`, `frontmatter:<file>`, `heading:<file>.<slug>`, and `code:<file>.<n>`. | `skillspec source query ./draft/.skillspec/source-map/source-map.json nodes --view index` |
| `skillspec source coverage` | `<source-map.json>`, `--json` | Summarizes mapped nodes, review-required classifications, and stale counts before semantic promotion. | `skillspec source coverage ./draft/.skillspec/source-map/source-map.json` |
| `skillspec source stale` | `<source-map.json>`, `--root <path>`, `--json` | Recomputes file hashes and exits non-zero when mapped source files changed or disappeared. Run before import, proof, or install. | `skillspec source stale ./draft/.skillspec/source-map/source-map.json --root ./source-skill` |
| `skillspec workspace map` | `<source-root>`, `--out <skillspec.workspace.yml>`, `--install-slug-policy workspace-path,local-name`, `--summary`, `--json` | Authoring-side structure recon for multi-skill repositories. Discovers folders with `SKILL.md`, detects plugin-shaped namespace roots (`skills/` plus `.claude-plugin/plugin.json`, `.mcp.json`, or `CLAUDE.md`), assigns package ids, skill-safe public names, and deterministic install slugs, scans Markdown for file and slash-command references, infers hard `depends_on` edges from file references, reports duplicate public names/install slugs, and writes a manifest plus markdown report. The default `workspace-path` policy is side-by-side/plugin-safe; use `local-name` only for replacement installs that must retire canonical existing folders such as `rote-setup`. Plugin slash-command links are workflow references, not hard dependency edges. This is not router indexing. `--summary` prints wall-clock and estimated token metrics while preserving full proof files. | `skillspec workspace map ./skills --out ./build/skillspec.workspace.yml --summary` |
| `skillspec workspace validate` | `<skillspec.workspace.yml>`, `--summary`, `--json` | Validates a workspace package graph before fanout import. Checks source root, package paths, exactly one `SKILL.md` per package, dependency references, self-dependencies, cycles, install slug uniqueness, and uncovered hard cross-package references. Duplicate public names are warnings until install planning. Plugin slash-command workflow references may cross packages without `depends_on`; file references still require dependency coverage. `--summary` prints wall-clock and estimated token metrics. | `skillspec workspace validate ./build/skillspec.workspace.yml --summary` |
| `skillspec workspace import` | `<skillspec.workspace.yml>`, `--out <build-root>`, `--summary`, `--json` | Runs fanout import for every package in a validated workspace graph. Dependency-ready packages in the same graph level may run in parallel; unchanged packages with intact artifacts are reused from `<build-root>/.skillspec/workspace-cache.json` and reported as `cached`. The command writes outputs under one mirrored build root, preserves successful package outputs when another package fails, and reports built, cached, failed, skipped, and blocked packages. It does not compile, install, or refresh router indexes. `--summary` keeps stdout compact and reports preserved artifact tokens plus cache hits/misses. | `skillspec workspace import ./build/skillspec.workspace.yml --out ./workspace-build --summary` |
| `skillspec workspace converge` | `<skillspec.workspace.yml>`, `--build-root <build-root>`, `--summary`, `--json` | Verifies the generated workspace build against the manifest before compile/install. Checks every package has a ready generated `skill.spec.yml` or explicit failure evidence, validates generated specs and package-local imports/resources, blocks dependents whose dependencies are not ready, and writes `workspace-converge.report.md`. `--summary` prints compact readiness counts plus wall-clock/token metrics. | `skillspec workspace converge ./build/skillspec.workspace.yml --build-root ./workspace-build --summary` |
| `skillspec workspace compile` | `<skillspec.workspace.yml>`, `--build-root <build-root>`, `--target codex-skill,claude-skill`, `--summary`, `--json` | Rechecks convergence, compiles ready package specs into generated `SKILL.md` loaders under the mirrored build root, blocks dependents whose dependencies did not compile, and writes `workspace-compile.report.md`. It does not install skills or refresh router indexes. `--summary` prints compact compile counts plus wall-clock/token metrics. | `skillspec workspace compile ./build/skillspec.workspace.yml --build-root ./workspace-build --target codex-skill --summary` |
| `skillspec workspace install` | `<skillspec.workspace.yml>`, `--build-root <build-root>`, `--target agents,codex,claude-local`, `--all-detected`, `--dry-run`, `--retire-existing`, `--install-slug-policy workspace-path,local-name`, `--visibility-policy entry-implicit,all-implicit,all-manual,none`, `--apply-visibility`, `--visibility-manifest <path>`, `--summary`, `--json` | Preflights a compiled workspace build and installs packages into harness roots using manifest `install_slug` folders, or an install-time slug policy override. Use `--install-slug-policy local-name --retire-existing` for upgrade/replacement installs where existing canonical folders must be backed up and replaced. It blocks missing compiled loaders, folder collisions, public-name collisions, duplicate effective slugs, and dependent packages whose dependencies cannot install; dry-run shows every planned write plus intended visibility. By default, entry packages remain implicit and shared/helper/wrapper packages are manual-only when `--apply-visibility` is used. Actual install writes `workspace-install.report.md`, `workspace-install.manifest.json`, and optionally a reversible workspace visibility manifest without refreshing router indexes. `--summary` prints compact install counts plus wall-clock/token metrics. After actual install, if the source root is inside a local Git checkout, the next steps recommend opening a PR with the generated contracts and proof artifacts only after restarting the harness and interacting with the installed SkillSpec-backed skills through the agent. | `skillspec workspace install ./build/skillspec.workspace.yml --build-root ./workspace-build --target codex --install-slug-policy local-name --retire-existing --dry-run --summary` |
| `skillspec doctor` | `<target>`, `--markdown`, `--html`, `--json` | Static shape gate plus agent follow-through risk diagnostic for local folders, public GitHub skill folders, and public GitHub repo URIs. Defaults to a formatted terminal report with "what this measures", current skill baseline, surface, findings, next actions, and basis summary. GitHub folder targets may use canonical `tree/<branch>/...` URLs or `blob/<branch>/...` URLs when the path resolves to a folder rather than `SKILL.md`. The human report treats risk as the headline metric so users do not misread the legacy structural score as a grade of domain quality; `--json` preserves the full machine report including `structural_score` and `score_model`. `--markdown` emits GitHub-flavored Markdown for run summaries and issue comments; `--html` emits a self-contained review page; `--json` emits the full machine report. These output modes are mutually exclusive. Reports `simple_skill`, `entry_skill_with_subskills`, `multi_skill_workspace`, `plugin_workspace`, or `non_skill_repository`. `simple_skill` receives full source-map structural scoring plus frontmatter discovery and agent drift risk. Multi-skill, entry-with-subskills, and plugin-shaped roots receive aggregate workspace risk plus one package report per `SKILL.md`, preserving plugin namespaces. Only `non_skill_repository` remains shape-only so doctor does not waste work on ordinary code repos. Remote targets are staged through a temporary partial sparse checkout and cleaned up after the report. | `skillspec doctor https://github.com/owner/repo --markdown` |
| `skillspec grammar sensemake` | `--view index,summary,porting,full`, `--json` | Teaches the embedded grammar artifact progressively. Use before importing or revising a spec so the harness does not infer grammar from memory. | `skillspec grammar sensemake --view porting` |
| `skillspec grammar checklist` | `--for <subject>`, `--json` | Shows the embedded coverage checklist for a semantic porting or review workflow. | `skillspec grammar checklist --for import-skill` |
| `skillspec grammar schema` | `--json` | Prints or summarizes the embedded JSON Schema used by grammar-aware harnesses and reviewers. | `skillspec grammar schema --json` |
| `skillspec imports check` | `<path>` | Validates declared local imports, sections, and dependency-first load order. | `skillspec imports check ./skill.spec.yml` |
| `skillspec deps check` | `<path>`, `--command <id>` | Checks declared dependencies for the whole spec or for one command. Local checks can pass or fail; harness-specific checks are reported as deferred. | `skillspec deps check ./skill.spec.yml --command validate_spec` |
| `skillspec compile` | `<path>`, `--target codex-skill,claude-skill,markdown` | Compiles a spec into harness guidance or a full Markdown rendering. Generated skill loaders point agents back to the colocated spec and runtime commands, and tell agents to verify/install the `skillspec` CLI because the trampoline depends on it for route, phase, progress, and alignment proof. | `skillspec compile ./skill.spec.yml --target codex-skill` |
| `skillspec import-skill` | `<path>`, `--out <path>`, `--source-map <path>` | Creates a mechanical draft `skill.spec.yml` from a local `SKILL.md` file or single skill folder after source-shape recon confirms one atomic package. Parent folders with multiple `SKILL.md` files, cross-skill references, or plugin markers should go through `skillspec workspace map` first so SkillSpec can identify atomic packages, plugin namespaces, hard dependency edges, workflow references, and name collisions. Existing reviewed `skill.spec.yml` files should be revised, not re-imported. For large or code-heavy sources, run `skillspec source map`, inspect `source coverage`, query `nodes`, `dependencies`, and `code`, then pass the fresh `source-map.json` with `--source-map`; the import refuses stale maps. The generated draft is scaffolding, not a finished semantic port. The original prose is preserved as `source/SKILL_md.old`, deliberately not as `SKILL.md` or Markdown. Fenced code is materialized under `resources/imported-code/` and referenced from the draft. The importer also writes `deps.toml`, declares it as a file dependency/artifact, and seeds it with inferred CLI plus Python/JavaScript/TypeScript package imports; the review pass must complete the ledger with source authority, local status, install risk, and degraded proof impact. A reviewed zero-dependency skill keeps `dependency_count = 0`; a byte-empty ledger is invalid. Do not delete dependency mentions to make QA pass. | `skillspec import-skill ./source-skill --out ./draft/skill.spec.yml --source-map ./draft/.skillspec/source-map/source-map.json` |
| `skillspec port-one-shot` | `<source>`, `--out <dir>`, `--target codex-skill,claude-skill,markdown`, `--prove`, `--force`, `--run-dir <dir>`, `--phase <id>`, `--requirement <id>`, `--json` | Bundles the safe single-skill porting ladder after source-shape recon confirms exactly one atomic prose package: embedded grammar/schema/checklist proof, source map, doctor, typed mechanical import, schema-derived shape crib, validate, imports check, deps check, scenario tests, compile, compact report, and optional direct-run estimated `progress stats`. The shape crib includes schema-sensitive forms such as quoted YAML strings and artifact executable refs. It rejects parent folders with multiple `SKILL.md` files. Use workspace flow for multi-skill or plugin-shaped roots and revision flow for existing reviewed `skill.spec.yml` files. Missing tests or dependency gaps are reported as `review_required`, not fake proof. When the source is inside a local Git checkout, successful summaries include PR guidance so the generated contract can be proposed back to the source skill repo after review, QA, install, harness restart, and a real agent interaction with the SkillSpec-backed skill. | `skillspec port-one-shot ./source-skill --out ./draft --target codex-skill --prove` |
| `skillspec synthesize-from-workspace` | `<workspace>`, `--out <folder>`, `--task <text>`, `--workspace-stats-report <path>`, `--workspace-log <path>`, `--workspace-meta <path>`, `--workspace-deps <path>`, `--log-last <n>`, `--observation-approved`, `--force`, `--json` | Rote-specific optional integration that creates a draft SkillSpec scaffold from durable rote workspace evidence. It refuses to write until the observed result and evidence summary have been shown and approved via `--observation-approved`. Required evidence is workspace stats, command log, and metadata; these can be collected live or supplied as explicit files when rote workspace lookup is unreliable. | `skillspec synthesize-from-workspace profile-enrichment --task 'use parallel web to enrich this profile' --out ./draft-profile-skill --observation-approved` |
| `skillspec trace compact` | `<run-dir>` | Rebuilds `trace.jsonl` and `summary.json` from append-only trace event files in a run directory. | `skillspec trace compact .skillspec/traces/run-123` |
| `skillspec install targets` | none | Lists detected harness skill roots, such as Codex, Agents, or Claude local skill directories. | `skillspec install targets` |
| `skillspec install skill` | `<folder>`, `--target agents,codex,claude-local`, `--all-detected`, `--dry-run`, `--name <name>`, `--force`, `--retire-existing` | Installs a generated skill folder containing `SKILL.md`, `skill.spec.yml`, and declared package-local files from imports, resources, code sources, and file dependencies into one or more detected harness roots. Use `--dry-run` before writing. When replacing an existing active prose skill, use `--retire-existing`: it backs up the old skill under `SKILLSPEC_HOME/backups/retired-skills` or `~/.skillspec/backups/retired-skills`, removes it from harness discovery, then installs the reviewed replacement at the same name. `--force` and `--retire-existing` are mutually exclusive. | `skillspec install skill examples/pdf --target agents --target codex --dry-run --retire-existing` |

## Harness Plugin Install Commands

These commands are not part of the `skillspec` CLI. They are the official
harness plugin install path for SkillSpec itself. Use them for public install
docs; use `skillspec install skill` for local development, generated skills, and
unreleased package testing.

| Harness | Command | Explanation |
| --- | --- | --- |
| Claude Code | `claude plugin marketplace add modiqo/skillspec --sparse .claude-plugin plugins/skillspec` | Adds the SkillSpec marketplace from the public repo. |
| Claude Code | `claude plugin install skillspec@skillspec` | Installs the `skillspec` plugin from that marketplace. |
| Claude Code | `claude plugin list`, then `claude plugin enable skillspec` only if disabled | Claude installs the plugin enabled by default in current builds; use `enable` only when list shows the plugin disabled. |
| Codex | `codex plugin marketplace add modiqo/skillspec --ref main --sparse .agents --sparse plugins/skillspec` | Adds the SkillSpec marketplace from the public repo. |
| Codex | `codex plugin add skillspec@skillspec` | Installs the `skillspec` plugin from that marketplace. Codex does not have a separate plugin enable command. |

## Binary And Crates.io Install Commands

These commands install the `skillspec` CLI itself. They are public
distribution commands, not SkillSpec runtime commands.

| Method | Command | Explanation |
| --- | --- | --- |
| Release binary | `curl -fsSL https://skillspec.sh/install.sh \| sh` | Downloads the latest published GitHub release asset for macOS or Linux x86_64, verifies the `.sha256`, and installs `skillspec` to `~/.local/bin` by default. |
| Pinned release binary | `curl -fsSL https://skillspec.sh/install.sh \| SKILLSPEC_VERSION=v0.1.0 sh` | Installs a specific release tag. `SKILLSPEC_INSTALL_DIR=/path/to/bin` changes the destination. |
| Crates.io | `cargo install skillspec` | Builds and installs the published crate from crates.io. Use when Rust is already installed or a prebuilt asset is unavailable for the platform. |
| Git main | `cargo install --git https://github.com/modiqo/skillspec --package skillspec --force` | Installs unreleased `main`; use for testing pending changes only. |
| Local checkout | `cargo install --path crates/skillspec-cli --force` | Installs from the current repo checkout for development and release verification. |

## Public Doctor Report Automation

These are repository automation surfaces, not `skillspec` CLI subcommands.

| Surface | Trigger | Explanation |
| --- | --- | --- |
| CI dogfood doctor | Push or pull request quality run | Runs `skillspec doctor skills/skillspec/`, prints the Markdown report to the GitHub job summary, and uploads text, Markdown, HTML, and JSON artifacts. |
| Doctor report request issue | `Doctor report request` issue form, `doctor-report` label, or `Doctor report:` title prefix | Validates a public `https://github.com/...` skill URL from the issue body or title, rejects private or unreadable repositories with local-run instructions, runs `skillspec doctor` for accepted public targets, writes the Markdown report to the Actions run summary, comments the rendered report on the issue, and uploads Markdown/HTML/JSON/text artifacts. |
| Public doctor Pages site | `https://skillspec.sh/` | Static GitHub Pages app that validates public GitHub skill URLs, opens a prefilled doctor-report issue request, lists prior public report issues, and renders the workflow's Markdown report comments without exposing a browser-side write token. |

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
| `skillspec route` | `--index <index-file-or-router-dir>`, `--query <text>`, `--top <n>`, `--execution-mode direct,durable`, `--json` | Scores candidate skills from the index and returns an authoritative decision: `use_skill`, `bypass`, or `ambiguous`. Managed `skill-router` packages are excluded from candidates so the dispatcher cannot select itself. `use_skill` includes a selected skill path; `bypass` and `ambiguous` leave `selected` null and include `bypass_reason` so the router continues normal behavior without forcing a candidate. Directory paths resolve to `skill-index.sqlite`. | `skillspec route --index ~/.skillspec/router --query 'extract text from a pdf' --json` |
| `skillspec skills audit` | `--roots <path>...`, `--json` | Audits routing metadata for overlong descriptions, vague descriptions, missing negative boundaries, and duplicate names. | `skillspec skills audit --roots ~/.agents/skills --json` |
| `skillspec skills set-visibility` | `<skill> <implicit,manual-only,name-only,off>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Sets one skill's conceptual visibility using native Codex/Claude controls and records a reversible manifest. | `skillspec skills set-visibility pdf manual-only --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec skills disable` | `<skill>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Convenience command for `set-visibility <skill> off`; off skills are excluded from router results when the manifest is used. | `skillspec skills disable legacy-skill --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec skills enable` | `<skill>`, `--roots <path>...`, `--manifest <path>`, `--dry-run`, `--json` | Convenience command for `set-visibility <skill> implicit`. | `skillspec skills enable pdf --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json` |
| `skillspec status` | `--roots <path>...`, `--json` | Read-only installation inventory: reports router and durable-executor installed/enabled/disabled state, supported and scanned roots, router index exists/stale/updated data, managed router guard hook state, and SkillSpec-backed versus legacy prose skills by name/path/visibility. Without `--roots`, scans router config roots when available, otherwise detected harness roots. | `skillspec status --json` |
| `skillspec visibility plan` | `--roots <path>...`, `--profile router-managed`, `--json` | Shows the native visibility changes router install would apply without editing files. | `skillspec visibility plan --roots ~/.agents/skills ~/.claude/skills --json` |
| `skillspec visibility apply` | `--roots <path>...`, `--profile router-managed`, `--manifest <path>`, `--dry-run`, `--json` | Applies native Codex `agents/openai.yaml`, Claude `skillOverrides`, and Claude `disable-model-invocation` frontmatter for shared `.agents` roots, then writes a rollback manifest. | `skillspec visibility apply --roots ~/.agents/skills --manifest ~/.skillspec/router/visibility-manifest.json --json` |
| `skillspec visibility restore` | `--manifest <path>`, `--dry-run`, `--json` | Restores exact file snapshots from a visibility manifest. It does not infer previous state. | `skillspec visibility restore --manifest ~/.skillspec/router/visibility-manifest.json --json` |
| `skillspec router install` | `--roots <path>...`, `--index <index-file-or-router-dir>`, `--manifest <path>`, `--router-name <name>`, `--dry-run`, `--json` | Installs the SkillSpec-backed `skill-router` skill into every configured root, enables router mode, installs managed Codex/Claude prompt guard hooks, makes routed skills explicit-only except an enabled durable-executor, builds the index, checks post-index preparedness, and writes router config with all managed router skill directories. The generated router loads a skill only when route decision is `use_skill`; `bypass` and `ambiguous` continue normal behavior. Directory paths resolve to `skill-index.sqlite`. | `skillspec router install --roots ~/.agents/skills --index ~/.skillspec/router --json` |
| `skillspec router enable` | `--dry-run`, `--json` | Re-enables an installed router, refreshes managed router skill files, installs managed prompt guard hooks, makes routed skills explicit-only except an enabled durable-executor, rebuilds the index from current roots, checks preparedness, writes `enabled: true`, and warns to restart harnesses. The generated router loads a skill only when route decision is `use_skill`; `bypass` and `ambiguous` continue normal behavior. | `skillspec router enable --json` |
| `skillspec router disable` | `--dry-run`, `--json` | Keeps router installed but disables router mode, removes only managed router guard hook entries, makes router explicit-only, restores routed skills to implicit/default visibility across recorded Codex and Claude roots, writes `enabled: false`, and warns to restart harnesses. | `skillspec router disable --json` |
| `skillspec router update` | `--backup-dir <path>`, `--dry-run`, `--json` | Reads the existing router config, backs up config, manifest, index, and managed router skill directories, rewrites the SkillSpec-backed router package in every recorded harness root, preserves enabled/disabled mode, refreshes managed guard hooks to match enabled state, reapplies matching visibility, rebuilds the index only when enabled, and warns that active harness sessions should be restarted. | `skillspec router update --json` |
| `skillspec router uninstall` | `--manifest <path>`, `--index <index-file-or-router-dir>`, `--keep-index`, `--dry-run`, `--json`; alias: `delete` | Restores visibility from the manifest, removes only managed router guard hook entries, removes every managed router skill marker directory recorded by router config, removes config, and optionally removes the index. Directory paths resolve to `skill-index.sqlite`. | `skillspec router uninstall --json` |
| `skillspec router guard` | `--config <path>`, `--hook`, `--json` | Verifies router-first readiness from router config. When enabled and stale/missing index state is detected, reapplies router-managed visibility and rebuilds the index before reporting `first_hop_ready`. `--hook` emits native `UserPromptSubmit` hook JSON: success injects compact router-ready context, and failure blocks with a repair command. | `skillspec router guard --json` |
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

For final closure and routine phase checkpoints, prefer
`skillspec progress batch --file <jsonl> --checkpoint "checkpointing evidence"
--summary` when several successful events need to be recorded together. It
keeps the ledger exact without making the user watch one command per
obligation. Surface individual rows only for failures, blockers, or proof gaps
the user must understand.

| Event | Meaning | Example |
| --- | --- | --- |
| `phase-started` | A declared phase has begun. | `skillspec progress record .skillspec/traces/run-123 phase-started qa_and_proof` |
| `requirement-started` | Work began for a phase requirement. | `skillspec progress record .skillspec/traces/run-123 requirement-started qa_and_proof validate_spec` |
| `requirement-satisfied` | A phase requirement has structured proof. | `skillspec progress record .skillspec/traces/run-123 requirement-satisfied qa_and_proof validate_spec --evidence-kind command --evidence-ref validate.log` |
| `requirement-failed` | A phase requirement failed. | `skillspec progress record .skillspec/traces/run-123 requirement-failed qa_and_proof test_spec --status fail` |
| `stats-collected` | Workspace or token metrics were recorded. Use `skillspec progress stats` so measured durable-executor stats or estimated direct-run summary metrics are populated for alignment. | `skillspec progress stats .skillspec/traces/run-123 --agent-visible-tokens 190 --artifact-tokens-preserved 96190 --avoided-tokens 96000 --metrics-source estimated` |
| `obligation-satisfied` | A route, forbid, elicitation, or other obligation has explicit proof. | `skillspec progress record .skillspec/traces/run-123 obligation-satisfied --id report_alignment_status --status pass` |
| `route-fulfilled` | The selected route was fulfilled. | `skillspec progress record .skillspec/traces/run-123 route-fulfilled --id prove_skill_value --status pass` |
| `route-check-completed` | A route-local check completed. | `skillspec progress record .skillspec/traces/run-123 route-check-completed --id qa_gate --status pass` |
| `after-success-completed` | A scheduled closure completed. | `skillspec progress record .skillspec/traces/run-123 after-success-completed --id trace_align --status pass` |
| `elicitation-answered` | A required elicitation was answered. | `skillspec progress record .skillspec/traces/run-123 elicitation-answered --id approve_scope --status pass` |
| `elicitation-waived` | A required elicitation was explicitly waived. | `skillspec progress record .skillspec/traces/run-123 elicitation-waived --id approve_scope --status pass` |
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
  `grammar`, `imports`, `deps`, `compile`, `import-skill`, `port-one-shot`,
  and `install`.
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
