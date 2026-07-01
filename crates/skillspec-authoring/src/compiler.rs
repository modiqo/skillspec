use skillspec_core::model::{
    Artifact, ArtifactKind, CodeBlock, CodeKind, CodeSource, CommandRequires, CommandTemplate,
    Dependency, DependencyKind, Elicitation, ElicitationChoice, ExecutionPlanMode, HandoffBoundary,
    Import, ImportLoad, ImportRole, ImportUse, ImportUseKind, Predicate, Recipe, RecipeStep,
    Resource, ResourceRole, ResourceUse, ResourceUseKind, Route, Rule, SafetyClass, ScenarioTest,
    SkillSpec, State, TraceEventKind,
};
use std::collections::BTreeMap;
use std::fmt::Write;

mod contracts;
mod selection;

use contracts::{
    write_authoring_contract, write_harness_presentation_contract, write_runtime_contract,
};
use selection::write_frontmatter;

#[derive(Clone, Copy, Debug)]
pub enum Target {
    CodexSkill,
    ClaudeSkill,
    Markdown,
}

pub fn compile(spec: &SkillSpec, target: Target) -> String {
    compile_with_name(spec, target, None)
}

pub fn compile_with_skill_name(spec: &SkillSpec, target: Target, skill_name: &str) -> String {
    compile_with_name(spec, target, Some(skill_name))
}

fn compile_with_name(spec: &SkillSpec, target: Target, skill_name: Option<&str>) -> String {
    let mut output = String::new();
    write_frontmatter(&mut output, spec, target, skill_name);
    if matches!(target, Target::CodexSkill | Target::ClaudeSkill) {
        write_loader_skill(&mut output, spec);
        trim_trailing_blank_lines(&mut output);
        return output;
    }

    write_overview(&mut output, spec, target);
    write_runtime_contract(&mut output);
    write_harness_presentation_contract(&mut output);
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
    output.push_str("Use the directory that contains this loaded `SKILL.md` as `<skill_dir>`.\n");
    output.push_str("The SkillSpec contract is `<skill_dir>/skill.spec.yml`; do not assume the user's current working directory contains the spec.\n\n");
    output.push_str("Start the SkillSpec guide with the user's task:\n\n");
    output.push_str("`skillspec run-loop <skill_dir>/skill.spec.yml --input '<user task>' --trace-dir \"${PWD}/.skillspec/traces\" --guide agent`\n\n");
    output.push_str("Resume an existing guided run:\n\n");
    output.push_str(
        "`skillspec run-loop <skill_dir>/skill.spec.yml --resume <run_dir> --guide agent`\n\n",
    );
    output.push_str("Follow the printed current gate. The selected route, matched rules, forbids, allowed commands, open requirements, resume command, and end proof from the CLI guide are authoritative.\n\n");
    output.push_str("Use `skillspec query` and `skillspec refs` only for handles named by the guide. Do not read the full spec unless the guide, a blocker, or the user asks for it.\n\n");
    output.push_str("Before the final response, follow the guide's end anchor: record final-response evidence, run the printed `skillspec trace align ... --summary` command as the completion summary source, and report result, evidence, alignment summary, token usage, selected route, and run directory.\n\n");
    output.push_str("If the `skillspec` CLI is not installed, report that this skill requires SkillSpec and ask the user to install it before continuing:\n\n");
    output.push_str("```bash\n");
    output.push_str(
        "curl -fsSL https://raw.githubusercontent.com/modiqo/skillspec/main/install.sh | sh\n",
    );
    output.push_str("# or, with Rust installed:\n");
    output.push_str("cargo install skillspec\n");
    output.push_str("```\n\n");
    output.push_str("If the user declines or installation is impossible, read `<skill_dir>/skill.spec.yml` directly and manually follow the same route, rule, phase, dependency, forbid, proof, and completion contract. Report that CLI guidance was unavailable and alignment proof is partial.\n");
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
        "skillspec progress record \"${PWD}/.skillspec/traces/<run-id>\" phase-completed <phase-id> --evidence-kind <kind> --evidence-ref <ref>\n",
    );
    output.push_str("skillspec progress stats \"${PWD}/.skillspec/traces/<run-id>\" --workspace <workspace> --workspace-stats-report \"${PWD}/.skillspec/traces/<run-id>/workspace-stats.txt\" --phase <phase-id> --requirement <stats-requirement-id>\n");
    output.push_str("skillspec progress stats \"${PWD}/.skillspec/traces/<run-id>\" --agent-visible-tokens <n> --artifact-tokens-preserved <n> --avoided-tokens <n> --metrics-source estimated --phase <phase-id> --requirement <stats-requirement-id>\n");
    output.push_str("skillspec progress final-response \"${PWD}/.skillspec/traces/<run-id>\" --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings\n");
    output.push_str("skillspec progress batch \"${PWD}/.skillspec/traces/<run-id>\" --file \"${PWD}/.skillspec/traces/<run-id>/final-proof.jsonl\" --checkpoint \"checkpointing evidence\" --summary\n");
    output.push_str(
        "skillspec progress show <skill-folder>/skill.spec.yml --run \"${PWD}/.skillspec/traces/<run-id>\"\n",
    );
    output.push_str(
        "skillspec decide <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n",
    );
    output.push_str("skillspec explain <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n");
    output.push_str("skillspec trace compact \"${PWD}/.skillspec/traces/<run-id>\"\n");
    output.push_str("skillspec trace align <skill-folder>/skill.spec.yml --decision-trace \"${PWD}/.skillspec/traces/<run-id>\" --execution-trace \"${PWD}/.skillspec/traces/<run-id>/execution.jsonl\" --summary --proof-digest \"${PWD}/.skillspec/traces/<run-id>/proof-digest.json\"\n");
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
                id: skillspec_core::model::RouteId("current_pr".to_owned()),
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

        assert!(output.contains("Use the directory that contains this loaded `SKILL.md`"));
        assert!(output.contains("skill.spec.yml"));
        assert!(output.contains("skillspec run-loop <skill_dir>/skill.spec.yml"));
        assert!(output.contains("--guide agent"));
        assert!(output.contains("--trace-dir"));
        assert!(output.contains("--resume <run_dir>"));
        assert!(output.contains("trace align"));
        assert!(output.contains("alignment summary"));
        assert!(output.contains("token usage"));
        assert!(output.contains("trace align ... --summary"));
        assert!(output.contains(
            "curl -fsSL https://raw.githubusercontent.com/modiqo/skillspec/main/install.sh | sh"
        ));
        assert!(output.lines().count() < 60);
        assert!(!output.contains("## Runtime Contract"));
        assert!(!output.contains("## Completion Report"));
        assert!(!output.contains("## Authoring And Revision Contract"));
        assert!(!output.contains("## Durable Handoff Contract"));
        assert!(!output.contains("skillspec act ./skill.spec.yml"));
        assert!(!output.contains("## Rules"));
        assert!(!output.contains("## Dependencies"));
    }
}
