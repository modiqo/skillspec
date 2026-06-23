use crate::model::{
    Artifact, ArtifactKind, CodeBlock, CodeKind, CodeSource, CommandRequires, CommandTemplate,
    Dependency, DependencyKind, Elicitation, ElicitationChoice, ExecutionPlanMode, HandoffBoundary,
    Import, ImportLoad, ImportRole, ImportUse, ImportUseKind, Predicate, Recipe, RecipeStep,
    Resource, ResourceRole, ResourceUse, ResourceUseKind, Route, Rule, SafetyClass, ScenarioTest,
    SkillSpec, State, TraceEventKind,
};
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Clone, Copy, Debug)]
pub enum Target {
    CodexSkill,
    ClaudeSkill,
    Markdown,
}

pub fn compile(spec: &SkillSpec, target: Target) -> String {
    let mut output = String::new();
    write_frontmatter(&mut output, spec, target);
    if matches!(target, Target::CodexSkill | Target::ClaudeSkill) {
        write_loader_skill(&mut output, spec);
        trim_trailing_blank_lines(&mut output);
        return output;
    }

    write_overview(&mut output, spec, target);
    write_runtime_contract(&mut output);
    write_authoring_contract(&mut output);
    write_entry(&mut output, spec);
    write_activation(&mut output, spec);
    write_routes(&mut output, spec);
    write_rules(&mut output, spec);
    write_elicitations(&mut output, spec);
    write_trace(&mut output, spec);
    write_dependencies(&mut output, spec);
    write_imports(&mut output, spec);
    write_resources(&mut output, spec);
    write_code(&mut output, spec);
    write_artifacts(&mut output, spec);
    write_recipes(&mut output, spec);
    write_states(&mut output, spec);
    write_commands(&mut output, spec);
    write_snippets(&mut output, spec);
    write_closures(&mut output, spec);
    write_tests(&mut output, spec);
    write_proof(&mut output, spec);
    write_review_required(&mut output, spec);
    write_runtime_commands(&mut output);
    trim_trailing_blank_lines(&mut output);
    output
}

fn trim_trailing_blank_lines(output: &mut String) {
    let trimmed_len = output.trim_end_matches(['\n', '\r', ' ', '\t']).len();
    output.truncate(trimmed_len);
    output.push('\n');
}

fn write_loader_skill(output: &mut String, spec: &SkillSpec) {
    let _ = writeln!(output, "# {}", spec.title);
    output.push('\n');
    let _ = writeln!(output, "{}", spec.description);
    output.push('\n');
    write_entry_gate(output, spec);
    output.push_str("This skill is a thin loader for the colocated `skill.spec.yml`. The spec is the source of truth for routes, rules, dependencies, imports, resources, recipes, tests, and trace requirements. Do not treat the spec as background prose; treat it as the execution contract for this task.\n\n");
    output.push_str("## Runtime Contract\n\n");
    output.push_str(
        "1. Load `./skill.spec.yml` from this skill folder before taking task actions.\n",
    );
    output.push_str("2. When the `skillspec` CLI is available and the spec shape is unfamiliar, run `skillspec sensemake ./skill.spec.yml --view index` to learn the section roles, counts, query handles, and navigation grammar without dumping the full YAML.\n");
    output
        .push_str("3. Then create the ordered phase plan and current-route action checklist:\n\n");
    output.push_str("   ```bash\n");
    output.push_str("   skillspec plan ./skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n");
    output.push_str("   skillspec act ./skill.spec.yml --input='<user task>' --run <run_dir> --phase <phase-id>\n");
    output.push_str("   ```\n\n");
    output.push_str("4. Strip skill invocation prefixes such as `/my-skill`, `$my-skill`, or `/durable-executor-spec` before passing `--input`.\n");
    output.push_str("5. Preserve the emitted trace `run_dir`.\n");
    output.push_str(
        "6. Read the full phase plan and action checklist before using tools. Treat them as the active execution SOP, not as advice. The `PHASE TOOL BOUNDARY - HARD` section is the permission boundary for the next action.\n",
    );
    output.push_str("7. For each execution phase, run `skillspec act ./skill.spec.yml --input='<user task>' --run <run_dir> --phase <phase-id>` before acting, record phase progress in `<run_dir>/execution.jsonl`, then run `skillspec progress show ./skill.spec.yml --run <run_dir>` to see completed, current, blocked, and remaining phases.\n");
    output.push_str("8. Pull active details with `skillspec query ./skill.spec.yml <handle> --view summary` and relationship edges with `skillspec refs ./skill.spec.yml <handle> --view summary`. Prefer precise handles such as `rule:<id>`, `rule:<id>.forbid`, `command:<id>.requires`, and `state:<id>.next` over reading the whole spec.\n");
    output.push_str("9. Before every substrate/tool call, apply the phase tool boundary and checklist allow/deny questions. Any unlisted tool, data source, execution substrate, provider, adapter, CLI, browser mode, API, or skill requires explicit user permission before use. The selected route and matched rules override lower-level skill defaults and generic tool preferences.\n");
    output.push_str("10. When the CLI is available after a trace exists, run `skillspec trace align ./skill.spec.yml --decision-trace <run_dir>` and, when structured action evidence exists, add `--execution-trace <run_dir>/execution.jsonl`. The command writes `<run_dir>/alignment.json`; report the alignment status, meaning, model layers, evidence gaps, user-facing proof rows, summary, and trace path.\n");
    output.push_str("11. If `skillspec plan`, `skillspec act`, or `skillspec progress` is unavailable, fall back to `skillspec decide`, then manually construct the same ordered phase checklist and progress notes before using tools. If the CLI is unavailable, read `skill.spec.yml` directly and apply the same contract manually. Do not expand this loader into a second source of truth.\n\n");
    write_authoring_contract(output);
    write_durable_handoff_contract(output);
    output.push_str("## How To Execute The Structure\n\n");
    output.push_str("Before the first task action, use `skillspec plan` and `skillspec act` to convert the decision output and relevant spec sections into an ordered phase plan plus a current-route OODA checklist:\n\n");
    output.push_str("- `route`: the selected route is the strategy to use. If no route is selected, stop and ask for the missing task shape instead of inventing a fallback.\n");
    output.push_str("- execution plan: if the selected route has `execution_plan`, execute its phases in order before using any tool outside the current phase. A later handoff phase does not license skipping an earlier shell or adapter phase. If a phase declares `jumps`, take the first matching jump condition and continue at the named phase.\n");
    output.push_str("- phase tool boundary: `skillspec act` renders the effective `tool_boundary` inherited from entry, route, and phase. Treat it as hard. If a needed tool or substrate is not listed, stop and ask permission before using it.\n");
    output.push_str("- route handoff: if the selected route has `handoff`, treat it as a hard execution boundary. Follow the handoff target and boundary before using tools from the current skill; `stop_current_skill` means do not continue current-skill execution except to pass the declared context.\n");
    output.push_str("- `matched_rules`: these are active obligations, not explanatory decoration. Use each rule's `reason`, `prefer`, `forbid`, `elicit`, and `after_success` fields to constrain the next action.\n");
    output.push_str("- `forbid`: forbids are hard negative constraints on behavior. They block substitutions even when a convenient tool is available. If a forbidden action seems necessary, stop and ask for explicit user approval or a different route; do not silently do it.\n");
    output.push_str("- user constraints: carry explicit user instructions into the same checklist. The spec adds structure; it does not erase the user's constraints.\n");
    output.push_str("- `elicit`: ask the required question before irreversible work, side effects, installs, auth steps, or broad exploration.\n");
    output.push_str("- `dependencies`: prove readiness for the active route, command, recipe, or code block before using it. Prefer command-scoped checks such as `skillspec deps check ./skill.spec.yml --command <id>` when a command id is known.\n");
    output.push_str("- dependency evidence: a missing environment variable only proves that variable is absent; it does not prove that auth, API keys, browser sessions, keychains, vaults, or CLI-native credentials are absent. When auth can live outside env, prove readiness with the declared command, adapter, browser, or dependency check instead of grepping env.\n");
    output.push_str("- `imports` and `resources`: load only the items required by the active route/rule/recipe/code, plus anything marked `always`.\n");
    output.push_str("- `commands`, `recipes`, and `code`: use declared templates and ordered steps as the allowed execution surface. Check their `requires` fields first, preserve outputs as evidence, and do not replace them with unrelated tools unless the active contract allows that substitution.\n");
    output.push_str("- `after_success` and closures: these are completion obligations. Do them before the final response, or report why they remain unproven.\n\n");
    output.push_str("Repeat the checklist before every tool call. If a lower-level skill or generic tool default conflicts with the selected route, follow the selected route. If the next tool is forbidden, stop and report that the SkillSpec blocks it.\n\n");
    output.push_str("If every allowed route is blocked by missing dependencies, auth, permissions, or a forbid, report the blocker and ask how to proceed. Do not switch to native search, raw shell, browser automation, direct API calls, or installs just because they are available in the harness.\n\n");
    output.push_str("## Quick Commands\n\n");
    output.push_str("```bash\n");
    output.push_str("skillspec sensemake ./skill.spec.yml --view index\n");
    output.push_str("skillspec plan ./skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n");
    output.push_str("skillspec act ./skill.spec.yml --input='<user task>' --run \"${PWD}/.skillspec/traces/<run-id>\" --phase <phase-id>\n");
    output.push_str("skillspec progress record \"${PWD}/.skillspec/traces/<run-id>\" phase-completed <phase-id> --evidence-kind rote_response --evidence-ref <ref>\n");
    output.push_str("skillspec progress stats \"${PWD}/.skillspec/traces/<run-id>\" --workspace <rote-workspace> --workspace-stats-report \"${PWD}/.skillspec/traces/<run-id>/workspace-stats.txt\" --phase <phase-id> --requirement <stats-requirement-id>\n");
    output.push_str("skillspec progress final-response \"${PWD}/.skillspec/traces/<run-id>\" --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings\n");
    output.push_str(
        "skillspec progress show ./skill.spec.yml --run \"${PWD}/.skillspec/traces/<run-id>\"\n",
    );
    output.push_str("skillspec validate ./skill.spec.yml\n");
    output.push_str("skillspec imports check ./skill.spec.yml\n");
    output.push_str("skillspec test ./skill.spec.yml\n");
    output.push_str("skillspec deps check ./skill.spec.yml\n");
    output.push_str("skillspec query ./skill.spec.yml rule:<id> --view summary\n");
    output.push_str("skillspec refs ./skill.spec.yml rule:<id> --view summary\n");
    output.push_str("skillspec query ./skill.spec.yml command:<id>.requires\n");
    output.push_str("skillspec decide ./skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n");
    output.push_str("skillspec explain ./skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n");
    output.push_str("skillspec trace align ./skill.spec.yml --decision-trace \"${PWD}/.skillspec/traces/<run-id>\" --execution-trace \"${PWD}/.skillspec/traces/<run-id>/execution.jsonl\"\n");
    output.push_str("```\n\n");
    output.push_str("## Completion Report\n\n");
    output.push_str("When reporting completion, always include the selected route, the SkillSpec trace `run_dir`, the persisted `<run_dir>/alignment.json`, and the compact `skillspec trace align` completion summary. Do not report a bare `unproven`; if alignment is incomplete, use `Alignment: partial` plus specific `Missing proof` rows from the align output. Command proof must name only the command basename, never raw args.\n\n");
    output.push_str("Always include token usage. For successful rote-backed runs, collect `rote workspace stats <workspace>` into a report file and run `skillspec progress stats <run_dir> --workspace <workspace> --workspace-stats-report <file> --phase <phase-id> --requirement <stats-requirement-id>` before alignment; missing `stats_collected` evidence is a workflow bug, not a normal omission. Draft the final response with Result, Evidence, Alignment summary, Token usage, and SkillSpec sections, run `skillspec progress final-response <run_dir> --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings`, then rerun `skillspec trace align` and report that final alignment. If stats truly cannot be collected, write `Token consumption: not recorded` and `Token savings: not recorded`; do not invent savings. When query-reduction stats exist, state the cached response tokens, extracted query-result tokens, saved-token delta, and reduction percentage. When rote workspace stats exist, include measured context-window/API tokens and explain that full evidence is outside the prompt in the workspace and can be retrieved by id/file instead of reloaded into context.\n\n");
    output.push_str("Minimum final response shape:\n\n");
    output.push_str("- `Result`: answer the user's task directly.\n");
    output.push_str("- `Evidence`: workspace name plus important response ids/files the user can query later.\n");
    output.push_str("- `Alignment summary`: include `Decision replay`, `Phase order`, `Requirements`, one or more `Missing proof` rows, `Forbidden actions`, and `Alignment` exactly as reported by `skillspec trace align`.\n");
    output.push_str("- `Token usage`: include `Token consumption` and `Token savings` exactly as reported by `skillspec trace align`; say `not recorded` when absent.\n");
    output.push_str("- `SkillSpec`: selected route, trace run directory, align status, status meaning, and proof rows that map request/spec obligations to observed evidence. Never let this replace the Result, Evidence, Alignment summary, or Token usage sections.\n\n");
    if !spec.routes.is_empty() {
        output.push_str("## Route Hints\n\n");
        let mut routes = spec.routes.iter().collect::<Vec<_>>();
        routes.sort_by_key(|route| route.rank.unwrap_or(i64::MAX));
        for route in routes {
            let _ = writeln!(output, "- `{}`: {}", route.id.0, route.label);
        }
    }
}

fn write_durable_handoff_contract(output: &mut String) {
    output.push_str("## Durable Handoff Contract\n\n");
    output.push_str("This skill participates in agent-mediated durable execution. There is no runtime handoff engine: the agent reads the active SkillSpec contracts, carries the handoff packet in context, and preserves the declared evidence.\n\n");
    output.push_str("- If a durable handoff packet is present, preserve its `workspace`, `trace_dir`, `return_to`, `branch_id`, and `execution_policy` fields.\n");
    output.push_str("- If no durable handoff packet is present and the task asks for remembered evidence, future recall, reuse, trace, alignment, or durable execution, route through `durable-executor` first unless the user explicitly requests direct/no-rote execution.\n");
    output.push_str("- If `durable_context.active` is true, do not route the whole task back to `durable-executor`; use `durable-executor` only as the execution substrate and then return to `return_to`.\n");
    output.push_str("- This skill owns its domain interpretation and validation. `durable-executor` owns workspace, trace, evidence, command substrate, final alignment, token-savings, and recall/crystallization closure when it initiated the handoff.\n");
    output.push_str("- Any CLI, shell command, local process, package command, API fallback, or provider command must use the durable execution substrate, normally a rote adapter or `rote exec --`, unless the active spec or user explicitly allows direct execution.\n");
    output.push_str("- On completion, emit a return packet with status, selected route, skill metadata, artifacts, evidence handles, blockers, and trace paths, then hand back to `return_to` for final closure.\n");
    output.push_str("- For parallel work, keep one top-level workspace but use branch-scoped `branch_id`, trace paths, evidence labels, and artifact directories.\n\n");
}

fn write_authoring_contract(output: &mut String) {
    output.push_str(
        "## Authoring And Revision Contract

",
    );
    output.push_str(
        "When importing, creating, revising, or extending this SkillSpec-backed skill, use the embedded grammar teacher before editing `skill.spec.yml`:

",
    );
    output.push_str(
        "```bash
",
    );
    output.push_str(
        "skillspec grammar sensemake --view index
",
    );
    output.push_str(
        "skillspec grammar sensemake --view porting
",
    );
    output.push_str(
        "skillspec grammar checklist --for import-skill
",
    );
    output.push_str(
        "```

",
    );
    output.push_str("- Treat the checklist as the review gate for semantic edits: activation, routes, rules, elicitations, imports/resources, commands/deps, procedures, tests, proof, and contract quality.
");
    output.push_str("- Fill or update a coverage matrix with `prose_span | obligation | skillspec_construct | confidence | status | review_note` before installing or releasing a changed skill.
");
    output.push_str("- Use `skillspec grammar schema --json` when a harness needs the exact embedded JSON schema.
");
    output.push_str("- Do not patch YAML by memory when the binary can teach the current grammar. Run the grammar commands again after CLI upgrades or when a spec shape is unfamiliar.

");
}

fn write_imports(output: &mut String, spec: &SkillSpec) {
    if spec.imports.is_empty() {
        return;
    }
    output.push_str("## Imports\n\n");
    output.push_str("Imports are runtime-loadable instruction material. Resolve import paths relative to the `skill.spec.yml` file; load `always` imports before task actions and load `on_demand` imports only when their route, rule, recipe, code, or nested import reference is active.\n\n");
    for (id, import) in &spec.imports {
        write_import(output, id, import);
    }
}

fn write_import(output: &mut String, id: &str, import: &Import) {
    let _ = writeln!(output, "### `{id}`");
    let _ = writeln!(output, "- path: `{}`", import.path);
    let _ = writeln!(output, "- role: `{}`", import_role_name(import));
    let _ = writeln!(output, "- load: `{}`", import_load_name(&import.load));
    if let Some(section) = &import.section {
        let _ = writeln!(output, "- section: {section}");
    }
    if let Some(description) = &import.description {
        let _ = writeln!(output, "- description: {description}");
    }
    if !import.requires.imports.is_empty() {
        let _ = writeln!(
            output,
            "- requires.imports: {}",
            code_list(&import.requires.imports)
        );
    }
    if !import.used_by.is_empty() {
        output.push_str("- used_by:\n");
        for use_ref in &import.used_by {
            let _ = writeln!(
                output,
                "  - {}: `{}`",
                import_use_kind_name(use_ref),
                use_ref.id
            );
        }
    }
    if !import.load_when.is_empty() {
        let _ = writeln!(output, "- load_when: {}", code_list(&import.load_when));
    }
    output.push('\n');
}

fn write_resources(output: &mut String, spec: &SkillSpec) {
    if spec.resources.is_empty() {
        return;
    }
    output.push_str("## Resources\n\n");
    output.push_str("Resources are source material and provenance, not hidden control flow. Use structured routes, rules, code, commands, and recipes for behavior.\n\n");
    for (id, resource) in &spec.resources {
        write_resource(output, id, resource);
    }
}

fn write_resource(output: &mut String, id: &str, resource: &Resource) {
    let _ = writeln!(output, "### `{id}`");
    let _ = writeln!(output, "- path: `{}`", resource.path);
    let _ = writeln!(output, "- role: `{}`", resource_role_name(resource));
    if let Some(description) = &resource.description {
        let _ = writeln!(output, "- description: {description}");
    }
    if !resource.used_by.is_empty() {
        output.push_str("- used_by:\n");
        for use_ref in &resource.used_by {
            let _ = writeln!(
                output,
                "  - {}: `{}`",
                resource_use_kind_name(use_ref),
                use_ref.id
            );
        }
    }
    if !resource.load_when.is_empty() {
        let _ = writeln!(output, "- load_when: {}", code_list(&resource.load_when));
    }
    output.push('\n');
}

fn write_code(output: &mut String, spec: &SkillSpec) {
    if spec.code.is_empty() {
        return;
    }
    output.push_str("## Code Blocks\n\n");
    output.push_str("Code blocks preserve executable knowledge from source skills. Review safety, dependencies, inputs, and outputs before running or promoting code into a recipe.\n\n");
    for (id, code) in &spec.code {
        write_code_block(output, id, code);
    }
}

fn write_code_block(output: &mut String, id: &str, code: &CodeBlock) {
    let _ = writeln!(output, "### `{id}`");
    let _ = writeln!(output, "- language: `{}`", code.language);
    let _ = writeln!(output, "- kind: `{}`", code_kind_name(code));
    if let Some(purpose) = &code.purpose {
        let _ = writeln!(output, "- purpose: {purpose}");
    }
    if let Some(provenance) = &code.provenance {
        if let Some(resource) = &provenance.resource {
            let _ = writeln!(output, "- provenance resource: `{resource}`");
        }
        if let Some(import) = &provenance.import {
            let _ = writeln!(output, "- provenance import: `{import}`");
        }
        if let Some(fence_index) = provenance.fence_index {
            let _ = writeln!(output, "  - fence_index: {fence_index}");
        }
        if let Some(heading) = &provenance.heading {
            let _ = writeln!(output, "  - heading: {heading}");
        }
    }
    if !code.requires.dependencies.is_empty()
        || !code.requires.imports.is_empty()
        || !code.requires.resources.is_empty()
        || !code.requires.artifacts.is_empty()
    {
        output.push_str("- requires:\n");
        if !code.requires.dependencies.is_empty() {
            let _ = writeln!(
                output,
                "  - dependencies: {}",
                code_list(&code.requires.dependencies)
            );
        }
        if !code.requires.imports.is_empty() {
            let _ = writeln!(output, "  - imports: {}", code_list(&code.requires.imports));
        }
        if !code.requires.resources.is_empty() {
            let _ = writeln!(
                output,
                "  - resources: {}",
                code_list(&code.requires.resources)
            );
        }
        if !code.requires.artifacts.is_empty() {
            let _ = writeln!(
                output,
                "  - artifacts: {}",
                code_list(&code.requires.artifacts)
            );
        }
    }
    if !code.inputs.is_empty() {
        let _ = writeln!(output, "- inputs: {}", code_list(&code.inputs));
    }
    if !code.outputs.is_empty() {
        let _ = writeln!(output, "- outputs: {}", code_list(&code.outputs));
    }
    output.push_str("- source:\n\n");
    match &code.source {
        CodeSource::Inline(inline_source) => {
            let _ = writeln!(output, "```{}", code.language);
            output.push_str(&inline_source.inline);
            output.push_str("\n```\n");
        }
        CodeSource::File(file) => {
            let _ = writeln!(output, "`{}`\n", file.file);
        }
    }
    output.push('\n');
}

fn write_artifacts(output: &mut String, spec: &SkillSpec) {
    if spec.artifacts.is_empty() {
        return;
    }
    output.push_str("## Artifacts\n\n");
    output.push_str("Artifacts describe named files or data products that code, commands, and recipes consume or produce.\n\n");
    for (id, artifact) in &spec.artifacts {
        write_artifact(output, id, artifact);
    }
}

fn write_artifact(output: &mut String, id: &str, artifact: &Artifact) {
    let _ = writeln!(output, "### `{id}`");
    let _ = writeln!(output, "- kind: `{}`", artifact_kind_name(artifact));
    if let Some(description) = &artifact.description {
        let _ = writeln!(output, "- description: {description}");
    }
    if let Some(path) = &artifact.path {
        let _ = writeln!(output, "- path: `{path}`");
    }
    output.push('\n');
}

fn write_recipes(output: &mut String, spec: &SkillSpec) {
    if spec.recipes.is_empty() {
        return;
    }
    output.push_str("## Recipes\n\n");
    output.push_str("Recipes are ordered procedures with explicit import, resource, dependency, code, command, elicitation, and artifact references.\n\n");
    for (id, recipe) in &spec.recipes {
        write_recipe(output, id, recipe);
    }
}

fn write_recipe(output: &mut String, id: &str, recipe: &Recipe) {
    let _ = writeln!(output, "### `{id}`");
    if let Some(description) = &recipe.description {
        let _ = writeln!(output, "- description: {description}");
    }
    let _ = writeln!(output, "- ordered: {}", recipe.ordered);
    if !recipe.steps.is_empty() {
        output.push_str("- steps:\n");
        for step in &recipe.steps {
            write_recipe_step(output, step);
        }
    }
    output.push('\n');
}

fn write_recipe_step(output: &mut String, step: &RecipeStep) {
    match step {
        RecipeStep::LoadImport(step) => {
            let _ = writeln!(output, "  - load_import: `{}`", step.load_import);
        }
        RecipeStep::LoadResource(step) => {
            let _ = writeln!(output, "  - load_resource: `{}`", step.load_resource);
        }
        RecipeStep::RunCommand(step) => {
            let _ = writeln!(output, "  - run_command: `{}`", step.run_command);
        }
        RecipeStep::RunCode(step) => {
            let _ = writeln!(output, "  - run_code: `{}`", step.run_code);
        }
        RecipeStep::ProduceArtifact(step) => {
            let _ = writeln!(output, "  - produce_artifact: `{}`", step.produce_artifact);
        }
        RecipeStep::ConsumeArtifact(step) => {
            let _ = writeln!(output, "  - consume_artifact: `{}`", step.consume_artifact);
        }
        RecipeStep::Ask(step) => {
            let _ = writeln!(output, "  - ask: `{}`", step.ask);
        }
        RecipeStep::Branch(step) => {
            let _ = writeln!(
                output,
                "  - branch: if `{}` then `{}`",
                step.branch.if_condition, step.branch.then
            );
            if let Some(otherwise) = &step.branch.otherwise {
                let _ = writeln!(output, "    otherwise `{otherwise}`");
            }
        }
        RecipeStep::Note(step) => {
            let _ = writeln!(output, "  - note: {}", step.note);
        }
    }
}

fn write_dependencies(output: &mut String, spec: &SkillSpec) {
    if spec.dependencies.is_empty() {
        return;
    }
    output.push_str("## Dependencies\n\n");
    output.push_str("Check declared dependencies before using commands that require them. Missing dependencies must be handled through the declared provision or elicitation path; do not silently install global tools.\n\n");
    for (id, dependency) in &spec.dependencies {
        write_dependency(output, id, dependency);
    }
}

fn write_dependency(output: &mut String, id: &str, dependency: &Dependency) {
    let _ = writeln!(output, "### `{id}`");
    let _ = writeln!(output, "- kind: `{}`", dependency_kind_name(dependency));
    if let Some(description) = &dependency.description {
        let _ = writeln!(output, "- description: {description}");
    }
    if let Some(command) = &dependency.command {
        let _ = writeln!(output, "- command: `{command}`");
    }
    if let Some(path) = &dependency.path {
        let _ = writeln!(output, "- path: `{path}`");
    }
    if let Some(env) = &dependency.env {
        let _ = writeln!(output, "- env: `{env}`");
    }
    if let Some(check) = &dependency.check {
        output.push_str("- check:\n");
        if let Some(command) = &check.command {
            let _ = writeln!(output, "  - command: `{command}`");
        }
        if let Some(path) = &check.path {
            let _ = writeln!(output, "  - path: `{path}`");
        }
        if let Some(env) = &check.env {
            let _ = writeln!(output, "  - env: `{env}`");
        }
    }
    if let Some(permission) = &dependency.permission {
        output.push_str("- permission:\n");
        let _ = writeln!(output, "  - required: {}", permission.required);
        if let Some(reason) = &permission.reason {
            let _ = writeln!(output, "  - reason: {reason}");
        }
        if let Some(safety) = &permission.safety {
            let _ = writeln!(output, "  - safety: `{}`", safety_name(safety));
        }
    }
    if let Some(provision) = &dependency.provision {
        output.push_str("- provision:\n");
        if let Some(elicitation) = &provision.elicit {
            let _ = writeln!(output, "  - elicit: `{elicitation}`");
        }
        if !provision.options.is_empty() {
            output.push_str("  - options:\n");
            for option in &provision.options {
                let _ = writeln!(output, "    - `{}`: {}", option.id, option.label);
                if let Some(description) = &option.description {
                    let _ = writeln!(output, "      description: {description}");
                }
                if let Some(command) = &option.command {
                    let _ = writeln!(output, "      command: `{command}`");
                }
                if let Some(safety) = &option.safety {
                    let _ = writeln!(output, "      safety: `{}`", safety_name(safety));
                }
            }
        }
    }
    output.push('\n');
}

fn write_frontmatter(output: &mut String, spec: &SkillSpec, target: Target) {
    match target {
        Target::CodexSkill | Target::ClaudeSkill => {
            let _ = writeln!(output, "---");
            let _ = writeln!(output, "name: {}", skill_name(&spec.id));
            let _ = writeln!(output, "description: {:?}", selection_description(spec));
            let _ = writeln!(output, "---");
            output.push('\n');
        }
        Target::Markdown => {}
    }
}

fn selection_description(spec: &SkillSpec) -> String {
    let mut intents = Vec::new();
    for applies_when in &spec.applies_when {
        collect_user_intents(applies_when, &mut intents);
    }

    let activation_summary = spec
        .activation
        .as_ref()
        .map(|activation| activation.summary.trim().trim_end_matches('.'))
        .filter(|summary| !summary.is_empty());

    let mut capabilities = Vec::new();
    let cli_intent_surface = intents.iter().any(|intent| {
        let intent = intent.to_ascii_lowercase();
        intent.contains("cli")
            || intent.contains("shell")
            || intent.contains("command")
            || intent.contains("process")
            || intent.contains("terminal")
    }) || spec.routes.iter().any(|route| {
        let route = format!(
            "{} {}",
            route.label,
            route.description.as_deref().unwrap_or_default()
        )
        .to_ascii_lowercase();
        route.contains("cli")
            || route.contains("shell")
            || route.contains("command")
            || route.contains("process")
            || route.contains("terminal")
    });
    let cli_command_surface = spec
        .commands
        .keys()
        .any(|id| id.contains("exec") || id.contains("process") || id.contains("pty"));
    if cli_intent_surface && cli_command_surface {
        push_unique(&mut capabilities, "CLI and shell commands");
    }
    if spec
        .dependencies
        .values()
        .any(|dependency| dependency.kind == DependencyKind::Adapter)
    {
        push_unique(
            &mut capabilities,
            "APIs, MCP/rote adapters, and service connectors",
        );
    }
    if spec
        .dependencies
        .values()
        .any(|dependency| dependency.kind == DependencyKind::Browser)
    {
        push_unique(&mut capabilities, "browser handoff and page evidence");
    }
    if spec
        .dependencies
        .values()
        .any(|dependency| dependency.kind == DependencyKind::Service)
    {
        push_unique(&mut capabilities, "external services");
    }
    if spec
        .commands
        .keys()
        .any(|id| id.contains("exec") || id.contains("process"))
    {
        push_unique(&mut capabilities, "process capture");
    }
    if spec
        .commands
        .keys()
        .any(|id| id.contains("stream") || id.contains("follow"))
    {
        push_unique(&mut capabilities, "logs and streams");
    }
    if spec.commands.keys().any(|id| id.contains("pty")) {
        push_unique(&mut capabilities, "PTY and terminal-sensitive prompts");
    }
    if spec
        .commands
        .keys()
        .any(|id| id.contains("deps") || id.contains("dependency"))
    {
        push_unique(&mut capabilities, "dependency checks");
    }
    if spec
        .closures
        .keys()
        .any(|id| id.contains("crystallize") || id.contains("crystallization"))
    {
        push_unique(&mut capabilities, "flow crystallization and replay");
    }

    let mut parts = Vec::new();
    if let Some(summary) = activation_summary {
        parts.push(summary.to_owned());
    }
    if let Some(activation) = &spec.activation {
        if !activation.keywords.is_empty() {
            parts.push(format!(
                "Use for {}",
                sentence_list(
                    &activation
                        .keywords
                        .iter()
                        .take(12)
                        .cloned()
                        .collect::<Vec<_>>()
                )
            ));
        }
    }
    if !intents.is_empty() {
        parts.push(format!(
            "Use when the task needs to {}",
            sentence_list(&intents.into_iter().take(7).collect::<Vec<_>>())
        ));
    } else {
        parts.push(format!(
            "Use for {}",
            lower_first(spec.description.trim_end_matches('.'))
        ));
    }
    if !capabilities.is_empty() {
        parts.push(format!(
            "Handles {}",
            sentence_list(&capabilities.into_iter().take(8).collect::<Vec<_>>())
        ));
    }
    if spec
        .entry
        .as_ref()
        .is_some_and(|entry| entry.decision_required)
    {
        parts.push(
            "Requires `skillspec decide` before substrate tools or overlapping low-level skills"
                .to_owned(),
        );
    }
    parts.push(
        "Preserves evidence with SkillSpec routes, forbids, dependencies, traces, and token-savings reports"
            .to_owned(),
    );
    shorten_description(&parts.join(". "))
}

fn collect_user_intents(value: &serde_yaml::Value, intents: &mut Vec<String>) {
    let serde_yaml::Value::Mapping(mapping) = value else {
        return;
    };
    let Some(user_intent) = mapping.get(serde_yaml::Value::String("user_intent".to_owned())) else {
        return;
    };
    match user_intent {
        serde_yaml::Value::Sequence(values) => {
            for value in values {
                if let Some(value) = value.as_str() {
                    push_unique(intents, value);
                }
            }
        }
        serde_yaml::Value::String(value) => push_unique(intents, value),
        _ => {}
    }
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if value.is_empty() || values.iter().any(|existing| existing == value) {
        return;
    }
    values.push(value.to_owned());
}

fn sentence_list(values: &[String]) -> String {
    match values {
        [] => String::new(),
        [only] => only.clone(),
        [head @ .., last] => format!("{} and {}", head.join(", "), last),
    }
}

fn shorten_description(value: &str) -> String {
    const LIMIT: usize = 900;
    if value.len() <= LIMIT {
        return value.to_owned();
    }
    let mut shortened = value[..LIMIT].trim_end().to_owned();
    if let Some(index) = shortened.rfind(['.', ',', ';']) {
        shortened.truncate(index);
    }
    shortened.trim_end().trim_end_matches('.').to_owned()
}

fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut lowered = first.to_lowercase().collect::<String>();
    lowered.push_str(chars.as_str());
    lowered
}

fn write_overview(output: &mut String, spec: &SkillSpec, target: Target) {
    let _ = writeln!(output, "# {}", spec.title);
    output.push('\n');
    let _ = writeln!(output, "{}", spec.description);
    output.push('\n');
    match target {
        Target::CodexSkill | Target::ClaudeSkill => {
            output.push_str(
                "This skill was generated from a SkillSpec. Use this document as the loaded harness guidance, and treat the referenced structured decisions as the behavioral contract.\n\n",
            );
        }
        Target::Markdown => {
            output.push_str(
                "This document is a complete Markdown rendering of the SkillSpec behavioral contract.\n\n",
            );
        }
    }
}

fn write_runtime_contract(output: &mut String) {
    output.push_str("## Runtime Contract\n\n");
    output.push_str("- Read this generated skill for orientation and immediate rules.\n");
    output.push_str("- Treat routes, rules, states, commands, tests, and review notes below as authoritative.\n");
    output.push_str("- Route `execution_plan` entries are ordered hard obligations. Execute phase 1 before phase 2; do not jump to a later handoff just because that substrate is available. Phase `jumps` are the only declared conditional exits from the default order.\n");
    output.push_str("- Route `handoff` entries are hard execution boundaries, not prose. If a selected route has `handoff.boundary: stop_current_skill`, stop current-skill execution except to pass the declared context to the target skill.\n");
    output.push_str("- Rules beat prose when there is tension.\n");
    output.push_str("- `forbid` entries are hard negative steering, not suggestions.\n");
    output.push_str("- `elicit` entries require bounded user questions before guessing.\n");
    output.push_str("- Use the scenario tests as examples of expected behavior.\n");
    output.push_str("- When unfamiliar with the spec shape, run `skillspec sensemake <skill-folder>/skill.spec.yml --view index` to get section roles, counts, ids, and query commands without consuming the whole spec.\n");
    output.push_str("- When the `skillspec` CLI is available, run `skillspec plan <skill-folder>/skill.spec.yml --input='<task>' --trace-dir \"${PWD}/.skillspec/traces\"`, preserve the printed `run_dir`, then run `skillspec act <skill-folder>/skill.spec.yml --input='<task>' --run <run_dir> --phase <phase-id>` before substrate tools. Treat the phase plan and current-route checklist as the active execution SOP.\n");
    output.push_str("- The `skillspec act` checklist is an OODA loop for the selected route: observe the task and trace, orient with matched rules, current phase, and `PHASE TOOL BOUNDARY - HARD`, decide the next allowed action, act with evidence capture, then repeat before the next tool call.\n");
    output.push_str("- The selected route and matched rules override lower-level skill defaults and generic tool preferences. If a lower-level skill suggests a forbidden tool, stop and follow the SkillSpec route.\n");
    output.push_str("- For each execution phase, run `skillspec act <skill-folder>/skill.spec.yml --input='<task>' --run <run_dir> --phase <phase-id>` before acting, obey the rendered phase tool boundary, record phase progress in `<run_dir>/execution.jsonl`, then run `skillspec progress show <skill-folder>/skill.spec.yml --run <run_dir>` to see completed, current, blocked, and remaining phases.\n");
    output.push_str("- If `skillspec plan`, `skillspec act`, or `skillspec progress` is unavailable, fall back to `skillspec decide` plus a manually constructed allow/deny checklist and progress notes before tool use. Prefer `skillspec explain` for human-facing route rationale.\n");
    output.push_str("- After `skillspec act`, inspect matched rules and active execution surfaces with `skillspec query <skill-folder>/skill.spec.yml <handle> --view summary` and `skillspec refs <skill-folder>/skill.spec.yml <handle> --view summary` instead of ad hoc YAML queries.\n");
    output.push_str("- Escalate query detail from `--view index` to `--view summary` to `--view full` only when the smaller view cannot answer the decision.\n");
    output.push_str("- When invoking `skillspec plan`, `skillspec act`, or `skillspec decide`, pass only the user's task text. Strip skill invocation prefixes such as `/durable-executor-spec`, `$durable-executor-spec`, or `/my-skill` before setting `--input`.\n");
    output.push_str("- Prefer `--input='<task text>'` in shell examples so `$skill-name` text is not expanded by the shell.\n");
    output.push_str("- Resolve `skill.spec.yml` relative to this `SKILL.md` folder, not the process working directory.\n");
    output.push_str("- Always pass `--trace-dir`; use `${PWD}/.skillspec/traces` unless the user or harness provides a run-specific trace directory.\n");
    output.push_str("- After `skillspec plan` or `skillspec act` prints trace lines, keep the emitted `run_dir` and mention it when reporting how the decision was made.\n");
    output.push_str("- When the CLI is available, run `skillspec trace align <skill-folder>/skill.spec.yml --decision-trace <run_dir>` and add `--execution-trace <run_dir>/execution.jsonl` when structured action evidence exists. This writes `<run_dir>/alignment.json`. Include the compact alignment summary, status meaning, decision-replay and execution-proof layer results, evidence gaps, user-facing proof rows, and any failed or partial checks in the completion report. Do not report a bare `unproven`; use `Alignment: partial` plus specific `Missing proof` rows.\n");
    output.push_str("- Always include token usage in the completion report. For successful rote-backed runs, collect `rote workspace stats <workspace>` into a report file and run `skillspec progress stats <run_dir> --workspace <workspace> --workspace-stats-report <file> --phase <phase-id> --requirement <stats-requirement-id>` before alignment; missing `stats_collected` evidence is a workflow bug, not a normal omission. Draft the final response with Result, Evidence, Alignment summary, Token usage, and SkillSpec sections, run `skillspec progress final-response <run_dir> --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings`, then rerun `skillspec trace align` and report that final alignment. Use `Token consumption` and `Token savings` from `skillspec trace align`; if stats truly cannot be collected, say `not recorded`. When query-reduction stats exist, report cached response tokens reduced to query-result tokens, saved-token delta, and reduction percentage instead of calling cached tokens consumed prompt tokens. When rote workspace evidence or stats exist, name the workspace and response ids/files, describe the workspace as a retrievable context file system, report measured context-window/API tokens when available, and explain crystallized/remembered reuse as avoiding full evidence reloads. Do not invent replay savings.\n");
    output.push_str("- Alignment proof rows may mention command basenames such as `gh` or `git`, but must not include raw command arguments because args may contain private data.\n\n");
    output.push_str("Minimum final response shape:\n\n");
    output.push_str("- `Result`: answer the user's task directly.\n");
    output.push_str("- `Evidence`: workspace name plus important response ids/files the user can query later.\n");
    output.push_str("- `Alignment summary`: include `Decision replay`, `Phase order`, `Requirements`, one or more `Missing proof` rows, `Forbidden actions`, and `Alignment` exactly as reported by `skillspec trace align`.\n");
    output.push_str("- `Token usage`: include `Token consumption` and `Token savings` exactly as reported by `skillspec trace align`; say `not recorded` when absent.\n");
    output.push_str("- `SkillSpec`: selected route, trace run directory, align status, status meaning, and proof rows that map request/spec obligations to observed evidence. Never let this replace the Result, Evidence, Alignment summary, or Token usage sections.\n\n");
}

fn write_entry(output: &mut String, spec: &SkillSpec) {
    if let Some(entry) = &spec.entry {
        output.push_str("## Entry\n\n");
        let _ = writeln!(output, "Prompt: {}", entry.prompt);
        output.push('\n');
    }
}

fn write_activation(output: &mut String, spec: &SkillSpec) {
    if spec.activation.is_none() && spec.applies_when.is_empty() {
        return;
    }
    output.push_str("## Activation\n\n");
    if let Some(activation) = &spec.activation {
        let _ = writeln!(output, "- summary: {}", activation.summary);
        if !activation.keywords.is_empty() {
            let _ = writeln!(output, "- keywords: {}", activation.keywords.join(", "));
        }
        if let Some(priority) = &activation.priority {
            let _ = writeln!(output, "- priority: {priority}");
        }
        output.push('\n');
    }
    if spec.applies_when.is_empty() {
        return;
    }
    output.push_str("### Applies When\n\n");
    for hint in &spec.applies_when {
        write_yaml_block(output, hint);
    }
    output.push('\n');
}

fn write_entry_gate(output: &mut String, spec: &SkillSpec) {
    let Some(entry) = &spec.entry else {
        return;
    };
    if !entry.decision_required
        && entry.supersedes_skills.is_empty()
        && entry.forbid_before_decision.is_empty()
    {
        return;
    }

    output.push_str("## Entry Gate\n\n");
    if entry.decision_required {
        output.push_str("- Before any task action, run `skillspec plan ./skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"`, preserve the printed `run_dir`, then run `skillspec act ./skill.spec.yml --input='<user task>' --run <run_dir> --phase <phase-id>`, and read the ordered phase plan plus current-route action checklist.\n");
        output.push_str("- Until that plan and checklist are read, the only allowed actions are loading this `SKILL.md`, loading the colocated `skill.spec.yml`, and running SkillSpec navigation or decision commands for this spec.\n");
        output.push_str("- The selected route and matched rules in the checklist override lower-level skill defaults. If a tool is forbidden, stop and report that the SkillSpec blocks it.\n");
        output.push_str("- After each phase action, record structured progress in `<run_dir>/execution.jsonl` and run `skillspec progress show ./skill.spec.yml --run <run_dir>` before moving to the next phase.\n");
    }
    if !entry.supersedes_skills.is_empty() {
        let _ = writeln!(
            output,
            "- This SkillSpec supersedes overlapping lower-level skill instructions: {}.",
            entry.supersedes_skills.join(", ")
        );
    }
    if !entry.forbid_before_decision.is_empty() {
        let _ = writeln!(
            output,
            "- Forbidden before the decision: {}.",
            entry.forbid_before_decision.join(", ")
        );
    }
    output.push('\n');
}

fn write_routes(output: &mut String, spec: &SkillSpec) {
    if spec.routes.is_empty() {
        return;
    }
    output.push_str("## Routes\n\n");
    output.push_str(
        "Try lower-rank routes first unless matching rules override the route or route order.\n\n",
    );
    let mut routes = spec.routes.iter().collect::<Vec<_>>();
    routes.sort_by_key(|route| route.rank.unwrap_or(i64::MAX));
    for route in routes {
        write_route(output, route);
    }
}

fn write_route(output: &mut String, route: &Route) {
    let rank = route
        .rank
        .map(|rank| rank.to_string())
        .unwrap_or_else(|| "unranked".to_owned());
    let _ = writeln!(output, "### `{}`", route.id.0);
    let _ = writeln!(output, "- label: {}", route.label);
    let _ = writeln!(output, "- rank: {rank}");
    if let Some(description) = &route.description {
        let _ = writeln!(output, "- description: {description}");
    }
    if !route.checks.is_empty() {
        let _ = writeln!(output, "- checks: {}", route.checks.join(", "));
    }
    if let Some(handoff) = &route.handoff {
        let _ = writeln!(output, "- handoff:");
        let _ = writeln!(output, "  - to_skill: `{}`", handoff.to_skill);
        let _ = writeln!(
            output,
            "  - boundary: `{}`",
            handoff_boundary_name(&handoff.boundary)
        );
        if !handoff.pass_context.is_empty() {
            let _ = writeln!(
                output,
                "  - pass_context: {}",
                handoff.pass_context.join(", ")
            );
        }
        if !handoff.forbid.is_empty() {
            let _ = writeln!(output, "  - forbid: {}", handoff.forbid.join(", "));
        }
        if let Some(reason) = &handoff.reason {
            let _ = writeln!(output, "  - reason: {reason}");
        }
    }
    if let Some(plan) = &route.execution_plan {
        let _ = writeln!(
            output,
            "- execution_plan: {}",
            execution_plan_mode_name(&plan.mode)
        );
        if let Some(reason) = &plan.reason {
            let _ = writeln!(output, "  - reason: {reason}");
        }
        for phase in &plan.phases {
            let _ = writeln!(output, "  - phase `{}`:", phase.id);
            let _ = writeln!(output, "    - owner_skill: `{}`", phase.owner_skill);
            if let Some(route) = &phase.route {
                let _ = writeln!(output, "    - route: `{}`", route.0);
            }
            if let Some(description) = &phase.description {
                let _ = writeln!(output, "    - description: {description}");
            }
            if !phase.requires.is_empty() {
                let _ = writeln!(output, "    - requires: {}", phase.requires.join(", "));
            }
            if !phase.checks.is_empty() {
                let _ = writeln!(output, "    - checks: {}", phase.checks.join(", "));
            }
            if !phase.forbid.is_empty() {
                let _ = writeln!(output, "    - forbid: {}", phase.forbid.join(", "));
            }
            if let Some(handoff) = &phase.handoff {
                let _ = writeln!(output, "    - handoff.to_skill: `{}`", handoff.to_skill);
                let _ = writeln!(
                    output,
                    "    - handoff.boundary: `{}`",
                    handoff_boundary_name(&handoff.boundary)
                );
            }
            if !phase.jumps.is_empty() {
                let _ = writeln!(output, "    - jumps:");
                for jump in &phase.jumps {
                    let _ = writeln!(
                        output,
                        "      - when `{}` -> phase `{}`",
                        jump.when, jump.to_phase
                    );
                    if let Some(reason) = &jump.reason {
                        let _ = writeln!(output, "        reason: {reason}");
                    }
                }
            }
        }
    }
    output.push('\n');
}

fn execution_plan_mode_name(mode: &ExecutionPlanMode) -> &'static str {
    match mode {
        ExecutionPlanMode::Ordered => "ordered",
    }
}

fn handoff_boundary_name(boundary: &HandoffBoundary) -> &'static str {
    match boundary {
        HandoffBoundary::StopCurrentSkill => "stop_current_skill",
        HandoffBoundary::ResumeAfterHandoff => "resume_after_handoff",
    }
}

fn write_rules(output: &mut String, spec: &SkillSpec) {
    if spec.rules.is_empty() {
        return;
    }
    output.push_str("## Rules\n\n");
    output.push_str("Evaluate rules in order. A matching rule may choose a route, replace route order, forbid substitutions, allow narrow fallbacks, request bounded elicitation, and schedule post-success actions.\n\n");
    for rule in &spec.rules {
        write_rule(output, rule);
    }
}

fn write_rule(output: &mut String, rule: &Rule) {
    let _ = writeln!(output, "### `{}`", rule.id.0);
    write_predicate(output, &rule.when);
    if let Some(route) = &rule.prefer {
        let _ = writeln!(output, "- prefer: `{}`", route.0);
    }
    if !rule.route_order.is_empty() {
        let _ = writeln!(
            output,
            "- route_order: {}",
            rule.route_order
                .iter()
                .map(|route| format!("`{}`", route.0))
                .collect::<Vec<_>>()
                .join(" -> ")
        );
    }
    if !rule.forbid.is_empty() {
        let _ = writeln!(output, "- forbid: {}", code_list(&rule.forbid));
    }
    if !rule.allow.is_empty() {
        output.push_str("- allow:\n");
        write_yaml_map(output, &rule.allow, 2);
    }
    if !rule.elicit.is_empty() {
        let _ = writeln!(output, "- elicit: {}", code_list(&rule.elicit));
    }
    if !rule.after_success.is_empty() {
        let _ = writeln!(
            output,
            "- after_success: {}",
            code_list(&rule.after_success)
        );
    }
    if let Some(reason) = &rule.reason {
        let _ = writeln!(output, "- reason: {reason}");
    }
    output.push('\n');
}

fn write_elicitations(output: &mut String, spec: &SkillSpec) {
    if spec.elicitations.is_empty() {
        return;
    }
    output.push_str("## Elicitations\n\n");
    output.push_str("Use elicitations for bounded, high-signal questions. Do not replace them with open-ended questioning or silent guessing.\n\n");
    for (id, elicitation) in &spec.elicitations {
        write_elicitation(output, id, elicitation);
    }
}

fn write_elicitation(output: &mut String, id: &str, elicitation: &Elicitation) {
    let _ = writeln!(output, "### `{id}`");
    let _ = writeln!(output, "- question: {}", elicitation.question);
    if let Some(default) = &elicitation.default {
        let _ = writeln!(output, "- default: `{default}`");
    }
    if let Some(max_choices) = elicitation.max_choices {
        let _ = writeln!(output, "- max_choices: {max_choices}");
    }
    if !elicitation.required_when.is_empty() {
        output.push_str("- required_when:\n");
        for condition in &elicitation.required_when {
            if let Some(route) = &condition.route {
                let _ = writeln!(output, "  - route: `{}`", route.0);
            }
            if let Some(missing) = &condition.missing {
                let _ = writeln!(output, "  - missing: `{missing}`");
            }
            if let Some(predicate) = &condition.predicate {
                output.push_str("  - predicate:\n");
                write_predicate(output, predicate);
            }
        }
    }
    output.push_str("- choices:\n");
    for choice in &elicitation.choices {
        write_elicitation_choice(output, choice);
    }
    output.push('\n');
}

fn write_elicitation_choice(output: &mut String, choice: &ElicitationChoice) {
    let _ = writeln!(output, "  - `{}`: {}", choice.id, choice.label);
    if let Some(description) = &choice.description {
        let _ = writeln!(output, "    description: {description}");
    }
    if !choice.sets.is_empty() {
        output.push_str("    sets:\n");
        write_yaml_map(output, &choice.sets, 6);
    }
    if let Some(route) = &choice.route {
        let _ = writeln!(output, "    route: `{}`", route.0);
    }
    if let Some(next) = &choice.next {
        let _ = writeln!(output, "    next: `{next}`");
    }
    if let Some(safety) = &choice.safety {
        let _ = writeln!(output, "    safety: `{}`", safety_name(safety));
    }
}

fn write_trace(output: &mut String, spec: &SkillSpec) {
    let Some(trace) = &spec.trace else {
        return;
    };
    output.push_str("## Decision Trace\n\n");
    output.push_str("When the `skillspec` CLI or a compatible harness evaluates this spec, record the decision path as append-only events. Rules trigger decisions; the evaluator writes the trace.\n\n");
    output.push_str("- mode: `event_log`\n");
    let _ = writeln!(output, "- required: {}", trace.required);
    if !trace.record.is_empty() {
        let events = trace
            .record
            .iter()
            .map(|event| format!("`{}`", trace_event_name(event)))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(output, "- record: {events}");
    } else {
        output.push_str("- record: all v0 decision events\n");
    }
    output.push('\n');
}

fn write_predicate(output: &mut String, predicate: &Predicate) {
    output.push_str("- when:\n");
    if !predicate.user_says_any.is_empty() {
        let _ = writeln!(
            output,
            "  - user_says_any: {}",
            quoted_list(&predicate.user_says_any)
        );
    }
    if !predicate.user_says_all_groups.is_empty() {
        let groups = predicate
            .user_says_all_groups
            .iter()
            .map(|group| format!("[{}]", quoted_list(group)))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(output, "  - user_says_all_groups: {groups}");
    }
    if let Some(value) = predicate.task_recurrence_likely {
        let _ = writeln!(output, "  - task_recurrence_likely: {value}");
    }
    if let Some(value) = predicate.domain_object_task {
        let _ = writeln!(output, "  - domain_object_task: {value}");
    }
    if let Some(value) = predicate.interactive_prompt_likely {
        let _ = writeln!(output, "  - interactive_prompt_likely: {value}");
    }
    if let Some(value) = predicate.command_likely_long_running {
        let _ = writeln!(output, "  - command_likely_long_running: {value}");
    }
}

fn write_states(output: &mut String, spec: &SkillSpec) {
    if spec.states.is_empty() {
        return;
    }
    output.push_str("## State Machine\n\n");
    output.push_str("Use states as lifecycle guidance. State actions must reference commands or closures; snippets supply user-facing copy.\n\n");
    for (id, state) in &spec.states {
        write_state(output, id, state);
    }
}

fn write_state(output: &mut String, id: &str, state: &State) {
    let _ = writeln!(output, "### `{id}`");
    if !state.r#do.is_empty() {
        let _ = writeln!(output, "- do: {}", code_list(&state.r#do));
    }
    if let Some(say) = &state.say {
        let _ = writeln!(output, "- say: `{say}`");
    }
    if let Some(ask) = &state.ask {
        let _ = writeln!(output, "- ask: `{ask}`");
    }
    if let Some(next) = &state.next {
        let _ = writeln!(output, "- next: `{next}`");
    }
    if let Some(yes) = &state.yes {
        let _ = writeln!(output, "- yes: `{yes}`");
    }
    if let Some(no) = &state.no {
        let _ = writeln!(output, "- no: `{no}`");
    }
    output.push('\n');
}

fn write_commands(output: &mut String, spec: &SkillSpec) {
    if spec.commands.is_empty() {
        return;
    }
    output.push_str("## Command Templates\n\n");
    output.push_str("Command templates are examples and contracts, not automatic permission. Apply the safety class and the harness approval policy before executing.\n\n");
    for (id, command) in &spec.commands {
        write_command(output, id, command);
    }
}

fn write_command(output: &mut String, id: &str, command: &CommandTemplate) {
    let _ = writeln!(output, "### `{id}`");
    if let Some(description) = &command.description {
        let _ = writeln!(output, "- description: {description}");
    }
    if let Some(safety) = &command.safety {
        let _ = writeln!(output, "- safety: `{}`", safety_name(safety));
    }
    output.push_str("- template:\n\n");
    output.push_str("```bash\n");
    output.push_str(&command.template);
    output.push_str("\n```\n");
    if !command.requires.is_empty() {
        output.push_str("- requires:\n");
        write_command_requires(output, &command.requires);
    }
    if !command.parse.is_empty() {
        output.push_str("- parse:\n");
        for (key, value) in &command.parse {
            let _ = writeln!(output, "  - `{key}`: `{value}`");
        }
    }
    if !command.success_when.is_empty() {
        output.push_str("- success_when:\n");
        write_yaml_map(output, &command.success_when, 2);
    }
    output.push('\n');
}

fn write_command_requires(output: &mut String, requires: &CommandRequires) {
    if !requires.dependencies.is_empty() {
        let _ = writeln!(
            output,
            "  - dependencies: {}",
            code_list(&requires.dependencies)
        );
    }
    if !requires.files.is_empty() {
        let _ = writeln!(output, "  - files: {}", code_list(&requires.files));
    }
    if !requires.env.is_empty() {
        let _ = writeln!(output, "  - env: {}", code_list(&requires.env));
    }
    if !requires.auth.is_empty() {
        let _ = writeln!(output, "  - auth: {}", code_list(&requires.auth));
    }
}

fn write_snippets(output: &mut String, spec: &SkillSpec) {
    if spec.snippets.is_empty() {
        return;
    }
    output.push_str("## Snippets\n\n");
    for (id, snippet) in &spec.snippets {
        let _ = writeln!(output, "### `{id}`");
        output.push_str(snippet.text.trim());
        output.push_str("\n\n");
    }
}

fn write_closures(output: &mut String, spec: &SkillSpec) {
    if spec.closures.is_empty() {
        return;
    }
    output.push_str("## Closures\n\n");
    output.push_str("Closures are post-task obligations or named lifecycle actions. Run them when referenced by states or `after_success`.\n\n");
    for (id, closure) in &spec.closures {
        let _ = writeln!(output, "### `{id}`");
        write_yaml_block(output, closure);
        output.push('\n');
    }
}

fn write_tests(output: &mut String, spec: &SkillSpec) {
    if spec.tests.is_empty() {
        return;
    }
    output.push_str("## Scenario Tests\n\n");
    output.push_str("Use these as behavioral examples. The agent should make the same routing and guardrail choices for equivalent tasks.\n\n");
    for test in &spec.tests {
        write_test(output, test);
    }
}

fn write_test(output: &mut String, test: &ScenarioTest) {
    let _ = writeln!(output, "### {}", test.name);
    let _ = writeln!(output, "- input: {:?}", test.input);
    if let Some(route) = &test.expect.route {
        let _ = writeln!(output, "- expect route: `{}`", route.0);
    }
    if !test.expect.route_order.is_empty() {
        let _ = writeln!(
            output,
            "- expect route_order: {}",
            test.expect
                .route_order
                .iter()
                .map(|route| format!("`{}`", route.0))
                .collect::<Vec<_>>()
                .join(" -> ")
        );
    }
    if !test.expect.forbid.is_empty() {
        let _ = writeln!(
            output,
            "- expect forbid: {}",
            code_list(&test.expect.forbid)
        );
    }
    if !test.expect.elicit.is_empty() {
        let _ = writeln!(
            output,
            "- expect elicit: {}",
            code_list(&test.expect.elicit)
        );
    }
    if !test.expect.after_success.is_empty() {
        let _ = writeln!(
            output,
            "- expect after_success: {}",
            code_list(&test.expect.after_success)
        );
    }
    output.push('\n');
}

fn write_proof(output: &mut String, spec: &SkillSpec) {
    let Some(proof) = &spec.proof else {
        return;
    };
    if proof.metrics.is_empty() {
        return;
    }
    output.push_str("## Proof Metrics\n\n");
    for metric in &proof.metrics {
        let _ = writeln!(output, "- `{metric}`");
    }
    output.push('\n');
}

fn write_review_required(output: &mut String, spec: &SkillSpec) {
    if spec.review_required.is_empty() {
        return;
    }
    output.push_str("## Review Required\n\n");
    for note in &spec.review_required {
        let _ = writeln!(output, "- {note}");
    }
    output.push('\n');
}

fn write_runtime_commands(output: &mut String) {
    output.push_str("## SkillSpec CLI Commands\n\n");
    output.push_str("Use these commands when the `skillspec` CLI is available. Replace `<skill-folder>` with the folder containing this generated `SKILL.md`. The default trace location is `${PWD}/.skillspec/traces`, where `PWD` is the task working directory.\n\n");
    output.push_str("```bash\n");
    output.push_str("skillspec sensemake <skill-folder>/skill.spec.yml --view index\n");
    output.push_str("skillspec validate <skill-folder>/skill.spec.yml\n");
    output.push_str("skillspec imports check <skill-folder>/skill.spec.yml\n");
    output.push_str("skillspec test <skill-folder>/skill.spec.yml\n");
    output.push_str("skillspec deps check <skill-folder>/skill.spec.yml\n");
    output.push_str("skillspec deps check <skill-folder>/skill.spec.yml --command <command-id>\n");
    output
        .push_str("skillspec query <skill-folder>/skill.spec.yml rule:<rule-id> --view summary\n");
    output.push_str("skillspec refs <skill-folder>/skill.spec.yml rule:<rule-id> --view summary\n");
    output
        .push_str("skillspec query <skill-folder>/skill.spec.yml command:<command-id>.requires\n");
    output.push_str(
        "skillspec plan <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n",
    );
    output.push_str(
        "skillspec act <skill-folder>/skill.spec.yml --input='<user task>' --run \"${PWD}/.skillspec/traces/<run-id>\" --phase <phase-id>\n",
    );
    output.push_str(
        "skillspec progress record \"${PWD}/.skillspec/traces/<run-id>\" phase-completed <phase-id> --evidence-kind rote_response --evidence-ref <ref>\n",
    );
    output.push_str("skillspec progress stats \"${PWD}/.skillspec/traces/<run-id>\" --workspace <rote-workspace> --workspace-stats-report \"${PWD}/.skillspec/traces/<run-id>/workspace-stats.txt\" --phase <phase-id> --requirement <stats-requirement-id>\n");
    output.push_str("skillspec progress final-response \"${PWD}/.skillspec/traces/<run-id>\" --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings\n");
    output.push_str(
        "skillspec progress show <skill-folder>/skill.spec.yml --run \"${PWD}/.skillspec/traces/<run-id>\"\n",
    );
    output.push_str(
        "skillspec decide <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n",
    );
    output.push_str("skillspec explain <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n");
    output.push_str("skillspec trace compact \"${PWD}/.skillspec/traces/<run-id>\"\n");
    output.push_str("skillspec trace align <skill-folder>/skill.spec.yml --decision-trace \"${PWD}/.skillspec/traces/<run-id>\" --execution-trace \"${PWD}/.skillspec/traces/<run-id>/execution.jsonl\"\n");
    output.push_str("```\n");
}

fn write_yaml_block(output: &mut String, value: &serde_yaml::Value) {
    output.push_str("```yaml\n");
    for line in yaml_lines(value) {
        let _ = writeln!(output, "{line}");
    }
    output.push_str("```\n");
}

fn write_yaml_map(output: &mut String, map: &BTreeMap<String, serde_yaml::Value>, indent: usize) {
    let prefix = " ".repeat(indent);
    for (key, value) in map {
        match scalar_yaml(value) {
            Some(value) => {
                let _ = writeln!(output, "{prefix}- `{key}`: {value}");
            }
            None => {
                let _ = writeln!(output, "{prefix}- `{key}`:");
                for line in yaml_lines(value) {
                    let _ = writeln!(output, "{prefix}  {line}");
                }
            }
        }
    }
}

fn yaml_lines(value: &serde_yaml::Value) -> Vec<String> {
    let rendered = serde_yaml::to_string(value).unwrap_or_else(|_| format!("{value:?}"));
    rendered
        .lines()
        .filter(|line| !line.trim().is_empty() && *line != "---")
        .map(|line| line.trim_end().to_owned())
        .collect()
}

fn scalar_yaml(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Bool(value) => Some(value.to_string()),
        serde_yaml::Value::Number(value) => Some(value.to_string()),
        serde_yaml::Value::String(value) => Some(format!("{value:?}")),
        serde_yaml::Value::Null => Some("null".to_owned()),
        serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Tagged(_) => None,
    }
}

fn code_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn quoted_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn safety_name(safety: &SafetyClass) -> &'static str {
    match safety {
        SafetyClass::ReadOnly => "read_only",
        SafetyClass::LocalRead => "local_read",
        SafetyClass::LocalWrite => "local_write",
        SafetyClass::NetworkRead => "network_read",
        SafetyClass::NetworkWrite => "network_write",
        SafetyClass::BrowserAttach => "browser_attach",
        SafetyClass::CredentialRequest => "credential_request",
        SafetyClass::Destructive => "destructive",
    }
}

fn dependency_kind_name(dependency: &Dependency) -> &'static str {
    match dependency.kind {
        DependencyKind::Cli => "cli",
        DependencyKind::Package => "package",
        DependencyKind::File => "file",
        DependencyKind::Env => "env",
        DependencyKind::Service => "service",
        DependencyKind::Adapter => "adapter",
        DependencyKind::Browser => "browser",
    }
}

fn resource_role_name(resource: &Resource) -> &'static str {
    match &resource.role {
        ResourceRole::SourceMaterial => "source_material",
        ResourceRole::Reference => "reference",
        ResourceRole::RequiredProcedure => "required_procedure",
        ResourceRole::Example => "example",
        ResourceRole::Script => "script",
        ResourceRole::Asset => "asset",
    }
}

fn import_role_name(import: &Import) -> &'static str {
    match &import.role {
        ImportRole::Policy => "policy",
        ImportRole::Reference => "reference",
        ImportRole::Procedure => "procedure",
        ImportRole::Example => "example",
        ImportRole::Skill => "skill",
    }
}

fn import_load_name(load: &ImportLoad) -> &'static str {
    match load {
        ImportLoad::Always => "always",
        ImportLoad::OnDemand => "on_demand",
    }
}

fn import_use_kind_name(use_ref: &ImportUse) -> &'static str {
    match &use_ref.kind {
        ImportUseKind::Route => "route",
        ImportUseKind::Rule => "rule",
        ImportUseKind::State => "state",
        ImportUseKind::Elicitation => "elicitation",
        ImportUseKind::Dependency => "dependency",
        ImportUseKind::Command => "command",
        ImportUseKind::Code => "code",
        ImportUseKind::Artifact => "artifact",
        ImportUseKind::Recipe => "recipe",
        ImportUseKind::Snippet => "snippet",
    }
}

fn resource_use_kind_name(use_ref: &ResourceUse) -> &'static str {
    match &use_ref.kind {
        ResourceUseKind::Route => "route",
        ResourceUseKind::Rule => "rule",
        ResourceUseKind::State => "state",
        ResourceUseKind::Elicitation => "elicitation",
        ResourceUseKind::Dependency => "dependency",
        ResourceUseKind::Command => "command",
        ResourceUseKind::Code => "code",
        ResourceUseKind::Artifact => "artifact",
        ResourceUseKind::Recipe => "recipe",
        ResourceUseKind::Snippet => "snippet",
    }
}

fn code_kind_name(code: &CodeBlock) -> &'static str {
    match &code.kind {
        CodeKind::Example => "example",
        CodeKind::RunnableScript => "runnable_script",
        CodeKind::Probe => "probe",
        CodeKind::Transform => "transform",
        CodeKind::Validator => "validator",
        CodeKind::Troubleshooting => "troubleshooting",
        CodeKind::Reference => "reference",
    }
}

fn artifact_kind_name(artifact: &Artifact) -> &'static str {
    match &artifact.kind {
        ArtifactKind::File => "file",
        ArtifactKind::Directory => "directory",
        ArtifactKind::Json => "json",
        ArtifactKind::Text => "text",
        ArtifactKind::Image => "image",
        ArtifactKind::Pdf => "pdf",
        ArtifactKind::Transcript => "transcript",
        ArtifactKind::Report => "report",
    }
}

fn trace_event_name(event: &TraceEventKind) -> &'static str {
    match event {
        TraceEventKind::InputReceived => "input_received",
        TraceEventKind::SpecLoaded => "spec_loaded",
        TraceEventKind::RuleEvaluated => "rule_evaluated",
        TraceEventKind::RuleMatched => "rule_matched",
        TraceEventKind::RouteSelected => "route_selected",
        TraceEventKind::RouteOrderSet => "route_order_set",
        TraceEventKind::ForbidAdded => "forbid_added",
        TraceEventKind::AllowAdded => "allow_added",
        TraceEventKind::ElicitationRequested => "elicitation_requested",
        TraceEventKind::AfterSuccessScheduled => "after_success_scheduled",
        TraceEventKind::OutcomeRecorded => "outcome_recorded",
    }
}

fn skill_name(id: &str) -> String {
    let mut name = String::new();
    let mut last_was_dash = false;
    for char in id.chars() {
        let next = if char.is_ascii_alphanumeric() {
            last_was_dash = false;
            Some(char.to_ascii_lowercase())
        } else if !last_was_dash {
            last_was_dash = true;
            Some('-')
        } else {
            None
        };
        if let Some(char) = next {
            name.push(char);
        }
    }
    name.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_skill_targets_emit_minimal_loader() {
        let spec = SkillSpec {
            schema: "skillspec/v0".to_owned(),
            id: "generic.code_review".to_owned(),
            title: "Code Review".to_owned(),
            description: "Review code changes.".to_owned(),
            activation: None,
            applies_when: Vec::new(),
            entry: None,
            routes: vec![Route {
                id: crate::model::RouteId("current_pr".to_owned()),
                label: "Review current pull request".to_owned(),
                rank: Some(10),
                description: None,
                checks: Vec::new(),
                handoff: None,
                execution_plan: None,
                tool_boundary: None,
            }],
            rules: Vec::new(),
            states: BTreeMap::new(),
            elicitations: BTreeMap::new(),
            trace: None,
            dependencies: BTreeMap::new(),
            imports: BTreeMap::new(),
            resources: BTreeMap::new(),
            code: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            recipes: BTreeMap::new(),
            commands: BTreeMap::new(),
            snippets: BTreeMap::new(),
            closures: BTreeMap::new(),
            proof: None,
            tests: Vec::new(),
            review_required: Vec::new(),
            metadata: BTreeMap::new(),
        };

        let output = compile(&spec, Target::ClaudeSkill);

        assert!(output.contains("thin loader"));
        assert!(output.contains("skill.spec.yml"));
        assert!(output.contains("--trace-dir"));
        assert!(output.contains("trace align"));
        assert!(output.contains("Completion Report"));
        assert!(output.contains("Authoring And Revision Contract"));
        assert!(output.contains("skillspec grammar sensemake --view porting"));
        assert!(output.contains("skillspec grammar checklist --for import-skill"));
        assert!(output.contains("coverage matrix"));
        assert!(output.contains("run_dir"));
        assert!(output.contains("status meaning"));
        assert!(output.contains("Alignment summary"));
        assert!(output.contains("Token usage"));
        assert!(output.contains("Token consumption"));
        assert!(output.contains("evidence gaps"));
        assert!(output.contains("skillspec act ./skill.spec.yml"));
        assert!(output.contains("active execution SOP"));
        assert!(output.contains("The selected route and matched rules"));
        assert!(output.contains("forbids are hard negative constraints"));
        assert!(
            output.contains("The spec adds structure; it does not erase the user's constraints")
        );
        assert!(
            output.contains("a missing environment variable only proves that variable is absent")
        );
        assert!(output.contains("do not replace them with unrelated tools"));
        assert!(!output.contains("## Rules"));
        assert!(!output.contains("## Dependencies"));
    }
}
