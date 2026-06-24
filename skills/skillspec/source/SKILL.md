---
name: skillspec
description: Use as the SkillSpec post-install setup and skill-authoring multiplexer: import an existing prose SKILL.md from a local folder or public URI, install router mode, optionally install durable-executor, create a SkillSpec skill from an observed durable rote workspace, revise an existing SkillSpec, prove value, compile, and optionally install.
---

# skillspec

Use this skill as the main prompt entry point after SkillSpec is installed. It
routes SkillSpec setup and skill-authoring requests without making the user
remember several separate prompt skills.

The main modes are:

- import an existing prose `SKILL.md` from a local folder or public URI
- install or refresh router mode
- optionally install durable-executor from an existing skill or approved source
- create a SkillSpec skill from an observed durable rote workspace
- revise an existing `skill.spec.yml`
- prove value, compile, and optionally install a reviewed SkillSpec-backed skill

## Post-Install Multiplexer

Use `/skillspec` as the prompt-level setup surface:

```text
/skillspec import /Users/me/.agents/skills/durable-executor
/skillspec import https://github.com/anthropics/skills/tree/main/skills/pdf
/skillspec install router
/skillspec install durable-executor from /path/or/public-uri
/skillspec observe durable workspace <workspace> and create a spec skill
```

Router install must write the visible router skill into the selected roots and
must not require a separate router root. In router mode, durable-executor is the
managed implicit exception when present.

durable-executor is optional. If it is already present in the selected roots,
verify it and keep it implicit. If it is missing, ask for a source path or URI,
or report that durable first-hop is unavailable.

Observed-workspace skill creation starts from a named rote workspace. Inspect
workspace stats, command logs, metadata, files, outputs, dependencies, errors,
and token evidence. Synthesize a reviewed `skill.spec.yml` from observed facts,
mark inferred behavior in the coverage matrix, and do not replay mutating
actions without explicit approval.

The source skill may be:

- a local `SKILL.md`
- a local skill folder
- a public GitHub repository
- a public GitHub repository path that contains one or more skills

The job is not a blind conversion. The job is to create a real SkillSpec:

```text
Keep the prose. Structure the decisions.
```

The deterministic `skillspec import-skill` command is only a first-pass helper.
The harness must stage the source, read the folder, reason over the resource
graph, revise the draft, test it, and prove the resulting spec.

## Source Contract

Accept these source forms:

- local file: `/path/to/SKILL.md`
- local folder: `/path/to/skill-folder`
- public repo shorthand: `owner/repo`
- public repo URL: `https://github.com/owner/repo`
- public repo path: `owner/repo/path/to/skill`
- public GitHub tree URL:
  `https://github.com/owner/repo/tree/main/path/to/skill`

Remote sources must be staged locally before porting. Do not install directly
from a remote checkout.

Use a temporary staging directory:

```bash
mkdir -p /tmp/skillspec-port
git clone --depth 1 <public-repo-url> /tmp/skillspec-port/<repo-name>
```

If the source is a repo and the user did not provide a skill path, find
candidate `SKILL.md` files and ask which one to port.

The porting path is always:

```text
source skill folder
  -> local staging folder with resources intact
  -> mechanical draft skill.spec.yml
  -> semantic reviewed skill.spec.yml
  -> validation and scenario tests
  -> compiled SKILL.md
  -> optional harness install
```

## Creation Procedure

1. Run the SkillSpec CLI capability preflight before porting:

   ```bash
   skillspec --help
   skillspec source map --help
   skillspec source query --help
   skillspec source coverage --help
   skillspec source stale --help
   skillspec grammar sensemake --help
   skillspec grammar checklist --help
   skillspec grammar schema --help
   skillspec import-skill --help
   skillspec validate --help
   skillspec imports check --help
   skillspec test --help
   skillspec compile --help
   skillspec deps --help
   skillspec deps check --help
   ```

   Required capabilities are `source map`, `source query`, `source coverage`,
   `source stale`, `grammar sensemake`, `grammar checklist`, `grammar schema`,
   `import-skill`, `validate`, `imports check`, `test`, `compile`, and `deps
   check`.

   If `imports check` or `deps check` is unavailable, continue only in degraded
   draft mode:

   - infer and write imports into the spec
   - infer and write dependencies into the spec
   - mark import validation as `review_required`
   - mark dependency verification as `review_required`
   - report the inferred dependency surface explicitly
   - do not claim dependency presence/absence
   - do not install or release the generated skill
   - tell the user to upgrade the SkillSpec CLI before install/release

   If `source map`, `source query`, `source coverage`, `source stale`,
   `grammar sensemake`, `grammar checklist`, `import-skill`, `validate`,
   `test`, or `compile` is unavailable, stop and ask the user to upgrade the
   SkillSpec CLI before porting.

2. Resolve the source to one local skill folder. A single `SKILL.md` is allowed,
   but a folder is preferred because referenced files, scripts, assets, and
   examples are part of the skill.
3. If the source is remote, download or clone it into the staging directory.
   Preserve relative paths. Do not copy only `SKILL.md` unless the source truly
   has no sibling resources.
4. Inventory and map the staged folder before importing:

   - Markdown imports/resources: `SKILL.md`, `reference.md`, `forms.md`,
     examples, and other linked docs
   - scripts and assets
   - fenced code blocks and their languages
   - shell command blocks
   - explicit file paths, env vars, packages, CLIs, services, browser/session
     assumptions
   - required ordering language such as "first", "before", "then", "after",
     "must complete in order", and "if/otherwise"

   ```bash
   skillspec source map path/to/skill-folder --out <draft-dir>/.skillspec/source-map
   skillspec source coverage <draft-dir>/.skillspec/source-map/source-map.json
   skillspec source query <draft-dir>/.skillspec/source-map/source-map.json nodes --view index
   skillspec source query <draft-dir>/.skillspec/source-map/source-map.json dependencies --view summary
   skillspec source query <draft-dir>/.skillspec/source-map/source-map.json code --view summary
   skillspec source stale <draft-dir>/.skillspec/source-map/source-map.json --root path/to/skill-folder
   ```

5. Use the source map as the progressive reader. Query exact source handles with
   `--view full` when a heading, code block, dependency, local reference, or
   modal obligation needs semantic promotion. Do not load a large source file
   wholesale when a source-map handle can recover the exact span. For small
   sources, a full `SKILL.md` read is acceptable only after the map confirms the
   file is bounded and has no sibling resources that affect routing, commands,
   code, dependencies, or recipes.
6. Teach the harness the current grammar before importing or editing the spec:

   ```bash
   skillspec grammar sensemake --view index
   skillspec grammar sensemake --view porting
   ```

   Use this output as the active grammar map. Do not infer the grammar from
   memory, Rust source, old examples, or generic YAML habits.

7. Run the mechanical extractor for a draft:

   ```bash
   skillspec import-skill path/to/skill-folder --out skill.spec.yml --source-map <draft-dir>/.skillspec/source-map/source-map.json
   ```

8. Sensemake the draft and load the import checklist before semantic review:

   ```bash
   skillspec sensemake skill.spec.yml --view index
   skillspec grammar checklist --for import-skill
   ```

   Fill or update a coverage matrix before changing routes, rules,
   dependencies, imports, resources, recipes, states, closures, tests, or trace
   fields:

   ```text
   prose_span | obligation | skillspec_construct | confidence | status | review_note
   ```

   The coverage matrix is the anti-bullshit layer. It should say what came
   directly from prose, what was inferred, what is proven, and what still needs
   human review.

9. Immediately validate imports and tell the user which dependencies were
   inferred before asking for approval to install or run anything:

   ```bash
   skillspec imports check skill.spec.yml
   skillspec deps check skill.spec.yml
   ```

   Summarize import status plus dependency ids, status, permission requirements,
   and provision options in plain language. Do not hide this in the final
   report.

10. Treat the draft as scaffolding, not truth.
11. Extract routes from strategy choices. Examples:

   - adapter/API
   - CLI/process
   - browser
   - PTY
   - background job
   - local file inspection
   - remembered/reused route
   - human approval

12. Extract rules from decision language: "always", "never", "prefer",
   "unless", "when", "before", "after", "ask", "do not", and "must".
13. Extract elicitations from places where the old skill would ask the user to
   choose, approve, connect, install, authenticate, attach, or continue.
14. Extract runtime-loadable Markdown into `imports`, and source provenance or
   supporting material into `resources`. Use imports for shared policy,
   branch-specific references, required procedures, examples, or other files
   the harness should deliberately load during a run. Use resources for
   evidence, scripts, assets, fixtures, and source material that should be
   preserved but not loaded as active guidance. Every on-demand import and every
   resource must be connected to a route, rule, command, code block, artifact,
   recipe, snippet, or explicit review note. Do not leave orphaned imports or
   resources.

15. Extract fenced snippets into `code` with:

   - `language`
   - `kind`: `example`, `runnable_script`, `probe`, `transform`, `validator`,
     `troubleshooting`, or `reference`
   - `source`: inline or extracted file
   - `provenance`: resource or import id, fence index, heading, and line span
     when known
   - `purpose`
   - `requires.dependencies`, `requires.imports`, `requires.resources`, and
     `requires.artifacts`
   - `inputs`, `outputs`, and `safety`

   Do not execute a snippet merely because it was imported. Preserve first,
   classify second, promote intentionally.

16. Extract named files and data products into `artifacts`. Examples:

   - input PDFs
   - field report JSON
   - generated images
   - filled PDFs
   - log/transcript/report files

   Link artifacts through `produced_by` and `consumed_by` where the source
   material makes that relationship clear.

17. Extract ordered procedures into `recipes`. Use recipes when a resource says
   work must happen in order, a probe determines a branch, or intermediate
   artifacts control the next step.

   A good recipe binds:

   - required imports
   - required resources
   - dependencies
   - artifacts
   - `load_import`
   - `load_resource`
   - `run_code`
   - `run_command`
   - `ask`
   - `branch`
   - artifact produce/consume steps

18. Extract commands into `commands` with:

   - `template`
   - `description`
   - `safety`
   - `requires.dependencies` for declared tools, files, env vars, services,
     adapters, browsers, or packages

19. Extract required tools, files, env vars, services, adapters, browsers, and
   packages into top-level `dependencies`. Do not leave required commands such
   as `curl`, `sed`, `gh`, `rote`, `python`, or `cargo` as prose.
20. For every dependency, add the best available check:

    - CLI: `kind: cli`, `command`, and `check.command`
    - file: `kind: file`, `path`, and `check.path`
    - env var: `kind: env`, `env`, and `check.env`
    - service/adapter/browser/package: declare the kind and note that the
      harness must perform the check

19. Add `permission` when using the dependency needs approval or special care.
20. Add `provision` when the skill can offer install/connect choices. Provision
    must point to an elicitation; never silently install missing tools.

21. Extract lifecycle phases into `states`. States should point to command,
   snippet, elicitation, or closure ids. Do not hide paragraphs in `states.do`.
22. Extract stable product language into `snippets`.
23. Extract post-task obligations into `closures`. Examples:

   - collect evidence
   - compute cost
   - ask whether to remember
   - ask whether to share
   - write a digest
   - run release QA

24. Add `trace` when the spec steers a harness:

    ```yaml
    trace:
      mode: event_log
      required: true
      record:
        - input_received
        - spec_loaded
        - rule_evaluated
        - rule_matched
        - route_selected
        - elicitation_requested
        - outcome_recorded
    ```

25. Add durable-executor compatibility when the skill may run commands, call
    APIs, invoke provider CLIs, write files, use adapters, preserve evidence,
    or participate in future recall. This is an agent-mediated contract, not a
    runtime engine.

    - Add `activation.summary` so generated trampoline frontmatter states what
      the skill is before the full spec is loaded.
    - For durable meta-router skills, use a summary like:
      `Universal durable-work meta-router and CLI/API/shell substrate with trace, alignment, evidence capture, and future recall.`
    - For domain skills, use a domain summary like:
      `Universal browser/web automation router with trace and alignment benefits.`
    - Do not create or maintain a central domain registry. Domain skills
      advertise their own activation metadata; the agent selects the matching
      installed skill from harness metadata.
    - If the skill receives a durable handoff packet, preserve `workspace`,
      `trace_dir`, `return_to`, `branch_id`, `execution_policy`, and
      `evidence_context`.
    - If no durable handoff packet is present and the task asks for remembered
      evidence, future recall, trace, alignment, reuse, or durable execution,
      route through `durable-executor` before domain work unless the user
      explicitly asks for direct/no-rote execution.
    - Domain skills own domain interpretation and validation only. Any CLI,
      shell, local process, package command, API fallback, or provider command
      must use the durable execution substrate, normally a rote adapter or
      `rote exec --`.
    - When domain work completes, the skill should produce a return packet with
      status, selected route, skill metadata, artifacts, evidence handles,
      blockers, and trace paths, then hand back to `return_to` for final durable
      closure.
    - For parallel branches, keep one top-level workspace and use branch-scoped
      `branch_id`, trace paths, evidence labels, and artifact directories.

26. Add scenario tests for every important decision, especially old-skill
    failure modes.
27. Add `review_required` for any uncertain judgment. Do not bury uncertainty
    in comments.
28. Validate, test, and check dependencies:

    ```bash
    skillspec validate skill.spec.yml
    skillspec imports check skill.spec.yml
    skillspec test skill.spec.yml
    skillspec deps check skill.spec.yml
    skillspec deps check skill.spec.yml --command '<command-id>'
    skillspec explain skill.spec.yml --input '<representative request>'
    ```

29. Before asking to install the generated skill, show the dependency summary
    again and ask the user to approve the dependency surface. Approval should
    cover:

    - required CLIs/files/env vars/services/adapters/browser/package managers
    - permission-sensitive dependencies
    - provision/install options
    - any missing dependencies that leave the skill draft-only

30. Compile only after the spec is valid:

    ```bash
    skillspec compile skill.spec.yml --target codex-skill
    skillspec compile skill.spec.yml --target claude-skill
    ```

31. If the user asks to install, create a clean generated skill folder:

    ```text
    <skill-name>/
      SKILL.md
      skill.spec.yml
    ```

    Then install it through `skillspec install` so harness roots are detected
    and the spec is validated before files are written.

## Harness Install Targets

Do not install by default. Ask or wait for explicit instruction.

Supported destinations:

- Codex/Codex-style personal skill:
  `skillspec install skill <skill-folder> --target agents` or
  `skillspec install skill <skill-folder> --target codex`
- Claude repo skill:
  `skillspec install skill <skill-folder> --target claude-local`
- Hermes or another harness:
  ask for the target skill root

Install the compiled `SKILL.md` and the reviewed `skill.spec.yml` together.
The generated skill should point agents to the local `skill.spec.yml`.
Use `skillspec install targets` to show detected harness roots, and use
`--dry-run` before writing when the user is still reviewing the install plan.

## Remote Source Staging

For a GitHub tree URL or `owner/repo/path` source, stage the full requested
folder locally before importing. Prefer sparse checkout when possible:

```bash
mkdir -p /tmp/skillspec-port
git clone --depth 1 --filter=blob:none --sparse https://github.com/<owner>/<repo> /tmp/skillspec-port/<repo>
git -C /tmp/skillspec-port/<repo> sparse-checkout set <path/to/skill>
```

If sparse checkout is unavailable, download the files into a temp folder while
preserving relative paths. After staging, import the local folder, not the URL.

Do not install packages, run scripts, or execute imported snippets during
staging. Staging is read-only source acquisition.

## SkillSpec CLI Capability Preflight

The `skillspec` skill requires a recent `skillspec` CLI. Check the available
surface before doing meaningful work:

```bash
skillspec --help
skillspec source map --help
skillspec source query --help
skillspec source coverage --help
skillspec source stale --help
skillspec import-skill --help
skillspec validate --help
skillspec imports check --help
skillspec test --help
skillspec compile --help
skillspec deps --help
skillspec deps check --help
```

If `imports check` or `deps check` is missing but import/validate/test/compile
exist, continue only as a draft port. In draft mode:

- keep runtime-loadable Markdown extraction first-class in `imports`
- keep dependency extraction first-class in `dependencies`
- mark import validation as `review_required` when `imports check` is missing
- mark dependency verification as `review_required`
- report inferred dependencies without local present/missing claims
- do not install package managers, CLIs, services, adapters, or generated
  skills
- tell the user to upgrade the CLI before release or installation

If `source map`, `source query`, `source coverage`, `source stale`,
`import-skill`, `validate`, `test`, or `compile` is missing, stop and ask the
user to upgrade the SkillSpec CLI before porting.

When working from this repository during development, prefer the checked-out
binary if the installed binary is stale:

```bash
cargo build
./target/debug/skillspec imports check skill.spec.yml
./target/debug/skillspec deps check skill.spec.yml
```

## Semantic Promotion

Mechanical import preserves evidence; semantic promotion creates the useful
SkillSpec.

Promote code when:

- the source text says it is required
- a recipe needs it as a probe, transform, or validator
- it produces or consumes a named artifact
- it has clear dependencies and safety

Keep code as an example when:

- it illustrates a library but is not required
- it lacks inputs/outputs
- it appears in troubleshooting or reference-only sections

Create recipes when:

- a resource has ordered instructions
- a step must run before another step
- a probe decides the branch
- an intermediate artifact is inspected before continuing

Create artifacts when:

- a file or JSON object is produced for later use
- the source names an output path
- validation depends on a generated file
- the user-facing result is a file/report/transcript/image/PDF

Add `review_required` when the harness cannot confidently classify a snippet,
artifact, or recipe edge.

## Rule Extraction

Rules should be short, testable steering decisions.

Use rules for:

- route choice
- route order
- forbidden substitutions
- narrow allowed fallbacks
- required elicitation
- post-success obligations

Good:

```yaml
rules:
  - id: browser_words_handoff_to_browse
    when:
      user_says_any:
        - browse
        - click
        - snapshot
    prefer: browser
    forbid:
      - native_search_as_answer
    elicit: browser_mode
```

Weak:

```yaml
rules:
  - id: browser
    reason: Use the browser when it seems appropriate and be careful.
```

## Elicitation Extraction

Make elicitation first-class when a user choice changes the route, risk, auth
surface, browser mode, install scope, or destructive action.

Good:

```yaml
elicitations:
  browser_mode:
    question: How should I access the browser state?
    choices:
      - id: attach_existing
        label: Attach to active browser
      - id: new_headed
        label: Start visible browser
      - id: new_headless
        label: Start headless browser
```

Do not leave important choices as prose like "ask the user what to do."

## Command Extraction

Every command template needs a safety class:

- `read_only`
- `local_read`
- `local_write`
- `network_read`
- `network_write`
- `browser_attach`
- `credential_request`
- `destructive`

If a command depends on a tool, file, auth state, or environment variable,
record that requirement through `dependencies` and `commands.<id>.requires`.
If the current grammar cannot express it, add `review_required`.

## Dependency Extraction

Use top-level `dependencies` for anything a command or route assumes is
available.

Good:

```yaml
dependencies:
  gh:
    kind: cli
    command: gh
    check:
      command: gh
    permission:
      required: true
      reason: GitHub CLI may use authenticated network access.
      safety: network_read
    provision:
      elicit: install_scope
      options:
        - id: user_global
          label: Install with a user package manager
          command: brew install gh
          safety: local_write

commands:
  list_prs:
    template: gh pr list
    safety: network_read
    requires:
      dependencies:
        - gh
```

Weak:

```yaml
commands:
  list_prs:
    template: gh pr list
    description: Requires gh to be installed somehow.
```

The creator must preserve install choices as elicitation. It must not tell a
harness to silently install global tools.

## Dependency Approval

Before validation/install approval, tell the user what the old skill appears to
depend on. Use this shape:

```text
Inferred dependency surface:
- gh: present, network/auth-sensitive, provisionable via user_global
- rote: present, required for workspace evidence
- deps.toml: generated scaffold present, review required before proof/install
- no dependencies: keep dependency_count = 0 in deps.toml; do not leave the file byte-empty

I will not install or connect anything unless you approve the provision option.
```

If `skillspec deps check` reports a dependency as missing, do not proceed as if
the port is production-ready. Either add provision choices, leave the generated
skill draft-only, or ask the user how to handle it.

## Code-Heavy Skill Porting

For skills with referenced Markdown files and code snippets, such as PDF,
spreadsheet, browser, data-processing, or build-system skills, do this extra
pass after mechanical import:

1. Build an import/resource map:

   ```text
   SKILL.md -> entry/source material
   reference.md -> import when loaded as active guidance, resource when only provenance
   forms.md / guide.md / workflow.md -> procedure import when the source says to load/follow it
   scripts/* -> script resources or file dependencies
   assets/* -> asset resources
   ```

2. Build a code map:

   ```text
   code id -> language -> source import/resource -> heading -> purpose -> deps -> inputs -> outputs -> safety
   ```

3. Classify snippets:

   - `probe`: decides what route or branch to take
   - `transform`: converts input artifacts into output artifacts
   - `validator`: proves a previous step worked
   - `runnable_script`: command-like code intended to run as-is
   - `example`: illustrative code that should not run automatically
   - `troubleshooting`: only used after failure
   - `reference`: retained for lookup

4. Create artifacts for every intermediate or final file the procedure relies
   on.
5. Create recipes for ordered procedures and branch points.
6. Add tests that prove the route picks the recipe, not a generic answer.
7. Add review notes for every snippet whose role is uncertain.

Do not flatten referenced files into a giant prose snippet. The whole point is
to preserve provenance as resources, load active guidance as imports, and move
control logic into recipes and rules.

## Example: Porting A GitHub Skill Folder

For a public GitHub skill folder:

```bash
mkdir -p /tmp/skillspec-port
git clone --depth 1 --filter=blob:none --sparse https://github.com/anthropics/skills /tmp/skillspec-port/anthropics-skills
git -C /tmp/skillspec-port/anthropics-skills sparse-checkout set skills/pdf

skillspec grammar sensemake --view porting
skillspec source map /tmp/skillspec-port/anthropics-skills/skills/pdf \
  --out /tmp/anthropic-pdf-source-map
skillspec source coverage /tmp/anthropic-pdf-source-map/source-map.json
skillspec source query /tmp/anthropic-pdf-source-map/source-map.json nodes --view index
skillspec source query /tmp/anthropic-pdf-source-map/source-map.json dependencies --view summary
skillspec source stale /tmp/anthropic-pdf-source-map/source-map.json \
  --root /tmp/skillspec-port/anthropics-skills/skills/pdf
skillspec import-skill /tmp/skillspec-port/anthropics-skills/skills/pdf \
  --out /tmp/anthropic-pdf.skill.spec.yml \
  --source-map /tmp/anthropic-pdf-source-map/source-map.json

skillspec sensemake /tmp/anthropic-pdf.skill.spec.yml --view index
skillspec grammar checklist --for import-skill
skillspec validate /tmp/anthropic-pdf.skill.spec.yml
skillspec imports check /tmp/anthropic-pdf.skill.spec.yml
skillspec deps check /tmp/anthropic-pdf.skill.spec.yml
```

Then inspect the draft:

```bash
rg -n "^imports:|^resources:|^code:|^commands:|^dependencies:|provenance:|fence_index:" /tmp/anthropic-pdf.skill.spec.yml
```

If the source contains a procedural file such as `forms.md`, revise the draft
into a reviewed spec with:

- `imports.forms.role: procedure` when the harness must load and follow it
- `resources.forms.role: required_procedure` only when it is preserved as
  provenance rather than loaded as runtime guidance
- code blocks classified as probes/transforms/validators/examples
- artifacts for field reports, rendered pages, generated PDFs, and validation
  output
- a recipe that `load_import`s the procedure, runs the probe, branches, and
  validates the final artifact
- dependency/provision elicitations for missing CLIs or packages

## Revising Existing SkillSpecs

When the user asks to revise an existing SkillSpec-backed skill, do not patch
YAML from memory. Start with the grammar and the current spec map:

```bash
skillspec grammar sensemake --view porting
skillspec sensemake path/to/skill.spec.yml --view index
skillspec grammar checklist --for import-skill
```

Then inspect only the active handles needed for the requested change:

```bash
skillspec query path/to/skill.spec.yml route:<id> --view summary
skillspec refs path/to/skill.spec.yml route:<id> --view summary
skillspec query path/to/skill.spec.yml rule:<id> --view summary
```

Before editing, update the coverage matrix row for the changed obligation.
After editing, run `validate`, `imports check`, `deps check`, `test`, a
realistic `decide`, and `trace align`. Report whether alignment is `pass`,
`unproven`, or `fail`; do not convert unproven execution evidence into a pass.

## Test Extraction

Every meaningful route rule should have at least one scenario test.

Prioritize tests for harness drift:

- browser request answered with native web search
- adapter setup attempted before browser fallback
- shell output summarized from scrollback instead of typed evidence
- long-running process run in a blocking foreground path
- release or publish run without dry-run or explicit approval
- dependency-dependent flow marked released without dependency evidence
- user choice skipped when the spec required elicitation

## Done Definition

A created SkillSpec is ready for serious testing when:

- the source was mapped with `skillspec source map`, source coverage and
  dependency/code summaries were inspected, and exact source spans were queried
  with `--view full` where needed
- the source was staged locally if remote
- `skillspec grammar sensemake --view porting` was run
- `skillspec grammar checklist --for import-skill` was used to fill or update
  a coverage matrix
- `skillspec validate skill.spec.yml` passes
- `skillspec imports check skill.spec.yml` passes
- `skillspec test skill.spec.yml` passes
- `skillspec deps check skill.spec.yml` has been run, and any missing
  dependency is represented by provision choices or review notes
- the user has seen the dependency surface before approving install
- `skillspec explain` gives expected routes for realistic inputs
- the generated Codex/Claude skill is smaller than the old prose skill
- the generated skill keeps the reviewed `skill.spec.yml` beside it
- all uncertain mappings are explicit in `review_required`
