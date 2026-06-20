use crate::error::{Error, Result};
use crate::imports;
use crate::model::{
    CodeSource, ConsumerRef, ExecutableRefKind, ImportLoad, ImportUse, ImportUseKind, ProducerRef,
    RecipeStep, ResourceUse, ResourceUseKind, RouteId, RuleId, SkillSpec,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_spec(path: &Path) -> Result<SkillSpec> {
    let spec = load_spec_unresolved(path)?;
    imports::validate(&spec, path)?;
    Ok(spec)
}

pub fn load_spec_unresolved(path: &Path) -> Result<SkillSpec> {
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let spec: SkillSpec = serde_yaml::from_str(&content).map_err(|source| Error::ParseYaml {
        path: path.to_path_buf(),
        source,
    })?;
    validate_spec(&spec)?;
    Ok(spec)
}

pub fn write_spec(path: &Path, spec: &SkillSpec) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let content = serde_yaml::to_string(spec).map_err(|source| Error::RenderYaml {
        path: PathBuf::from(path),
        source,
    })?;
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

pub fn validate_spec(spec: &SkillSpec) -> Result<()> {
    if spec.schema != "skillspec/v0" {
        return Err(Error::UnsupportedSchema {
            found: spec.schema.clone(),
        });
    }
    if spec.id.trim().is_empty() {
        return Err(Error::MissingField { field: "id" });
    }
    if spec.title.trim().is_empty() {
        return Err(Error::MissingField { field: "title" });
    }
    if spec.description.trim().is_empty() {
        return Err(Error::MissingField {
            field: "description",
        });
    }
    validate_identifier("id", &spec.id)?;
    validate_routes(spec)?;
    validate_elicitations(spec)?;
    validate_trace(spec)?;
    validate_dependencies(spec)?;
    validate_imports(spec)?;
    validate_resources(spec)?;
    validate_code(spec)?;
    validate_artifacts(spec)?;
    validate_recipes(spec)?;
    validate_rules(spec)?;
    validate_states(spec)?;
    validate_commands(spec)?;
    validate_tests(spec)?;
    Ok(())
}

fn validate_routes(spec: &SkillSpec) -> Result<()> {
    let mut seen = BTreeSet::new();
    for route in &spec.routes {
        validate_route_id("routes.id", &route.id)?;
        insert_unique("routes.id", &mut seen, &route.id.0)?;
    }
    Ok(())
}

fn validate_rules(spec: &SkillSpec) -> Result<()> {
    let route_ids = route_ids(spec);
    let action_ids = action_ids(spec);
    let elicitation_ids = elicitation_ids(spec);
    let mut seen = BTreeSet::new();

    for rule in &spec.rules {
        validate_rule_id("rules.id", &rule.id)?;
        insert_unique("rules.id", &mut seen, &rule.id.0)?;
        if let Some(route) = &rule.prefer {
            validate_known_route("rules.prefer", &route_ids, route)?;
        }
        for route in &rule.route_order {
            validate_known_route("rules.route_order", &route_ids, route)?;
        }
        for elicitation in &rule.elicit {
            validate_known_elicitation("rules.elicit", &elicitation_ids, elicitation)?;
        }
        for action in &rule.after_success {
            validate_known_action("rules.after_success", &action_ids, action)?;
        }
    }
    Ok(())
}

fn validate_states(spec: &SkillSpec) -> Result<()> {
    let state_ids = spec.states.keys().cloned().collect::<BTreeSet<_>>();
    let action_ids = action_ids(spec);
    let elicitation_ids = elicitation_ids(spec);

    for (state_id, state) in &spec.states {
        validate_identifier("states key", state_id)?;
        for action in &state.r#do {
            validate_known_action("states.do", &action_ids, action)?;
        }
        if let Some(next) = &state.next {
            validate_known_state("states.next", &state_ids, next)?;
        }
        if let Some(yes) = &state.yes {
            validate_known_state("states.yes", &state_ids, yes)?;
        }
        if let Some(no) = &state.no {
            validate_known_state("states.no", &state_ids, no)?;
        }
        if let Some(snippet) = &state.say {
            if !spec.snippets.contains_key(snippet) {
                return Err(Error::UnknownReference {
                    field: "states.say",
                    value: snippet.clone(),
                });
            }
        }
        if let Some(elicitation) = &state.ask {
            validate_known_elicitation("states.ask", &elicitation_ids, elicitation)?;
        }
    }
    Ok(())
}

fn validate_tests(spec: &SkillSpec) -> Result<()> {
    let route_ids = route_ids(spec);
    let action_ids = action_ids(spec);
    let elicitation_ids = elicitation_ids(spec);

    for test in &spec.tests {
        if !test.expect.has_assertions() {
            return Err(Error::MissingField {
                field: "tests.expect assertion",
            });
        }
        if let Some(route) = &test.expect.route {
            validate_known_route("tests.expect.route", &route_ids, route)?;
        }
        for route in &test.expect.route_order {
            validate_known_route("tests.expect.route_order", &route_ids, route)?;
        }
        for action in &test.expect.after_success {
            validate_known_action("tests.expect.after_success", &action_ids, action)?;
        }
        if let Some(actions) = &test.expect.after_success_exact {
            for action in actions {
                validate_known_action("tests.expect.after_success_exact", &action_ids, action)?;
            }
        }
        for action in &test.expect.not_after_success {
            validate_known_action("tests.expect.not_after_success", &action_ids, action)?;
        }
        for elicitation in &test.expect.elicit {
            validate_known_elicitation("tests.expect.elicit", &elicitation_ids, elicitation)?;
        }
        if let Some(elicitations) = &test.expect.elicit_exact {
            for elicitation in elicitations {
                validate_known_elicitation(
                    "tests.expect.elicit_exact",
                    &elicitation_ids,
                    elicitation,
                )?;
            }
        }
        for elicitation in &test.expect.not_elicit {
            validate_known_elicitation("tests.expect.not_elicit", &elicitation_ids, elicitation)?;
        }
        let rule_ids = rule_ids(spec);
        for rule in &test.expect.matched_rules {
            validate_known_rule("tests.expect.matched_rules", &rule_ids, rule)?;
        }
        if let Some(rules) = &test.expect.matched_rules_exact {
            for rule in rules {
                validate_known_rule("tests.expect.matched_rules_exact", &rule_ids, rule)?;
            }
        }
        for rule in &test.expect.not_matched_rules {
            validate_known_rule("tests.expect.not_matched_rules", &rule_ids, rule)?;
        }
    }
    Ok(())
}

fn validate_dependencies(spec: &SkillSpec) -> Result<()> {
    let elicitation_ids = elicitation_ids(spec);
    let mut seen = BTreeSet::new();

    for (id, dependency) in &spec.dependencies {
        validate_identifier("dependencies key", id)?;
        insert_unique("dependencies key", &mut seen, id)?;
        if dependency.command.as_deref().is_some_and(str::is_empty) {
            return Err(Error::MissingField {
                field: "dependencies.command",
            });
        }
        if dependency.path.as_deref().is_some_and(str::is_empty) {
            return Err(Error::MissingField {
                field: "dependencies.path",
            });
        }
        if dependency.env.as_deref().is_some_and(str::is_empty) {
            return Err(Error::MissingField {
                field: "dependencies.env",
            });
        }
        if let Some(check) = &dependency.check {
            if check.command.as_deref().is_some_and(str::is_empty) {
                return Err(Error::MissingField {
                    field: "dependencies.check.command",
                });
            }
            if check.path.as_deref().is_some_and(str::is_empty) {
                return Err(Error::MissingField {
                    field: "dependencies.check.path",
                });
            }
            if check.env.as_deref().is_some_and(str::is_empty) {
                return Err(Error::MissingField {
                    field: "dependencies.check.env",
                });
            }
        }
        if let Some(provision) = &dependency.provision {
            if let Some(elicitation) = &provision.elicit {
                validate_known_elicitation(
                    "dependencies.provision.elicit",
                    &elicitation_ids,
                    elicitation,
                )?;
            }
            let mut option_ids = BTreeSet::new();
            for option in &provision.options {
                validate_identifier("dependencies.provision.options.id", &option.id)?;
                insert_unique(
                    "dependencies.provision.options.id",
                    &mut option_ids,
                    &option.id,
                )?;
                if option.label.trim().is_empty() {
                    return Err(Error::MissingField {
                        field: "dependencies.provision.options.label",
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_resources(spec: &SkillSpec) -> Result<()> {
    let mut seen = BTreeSet::new();
    let references = resource_references(spec);

    for (id, resource) in &spec.resources {
        validate_identifier("resources key", id)?;
        insert_unique("resources key", &mut seen, id)?;
        if resource.path.trim().is_empty() {
            return Err(Error::MissingField {
                field: "resources.path",
            });
        }
        for use_ref in &resource.used_by {
            validate_resource_use(spec, use_ref)?;
        }
        if !references.contains_key(id) && resource.used_by.is_empty() {
            return Err(Error::UnknownReference {
                field: "resources.orphan",
                value: id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_imports(spec: &SkillSpec) -> Result<()> {
    let mut seen = BTreeSet::new();
    let import_ids = spec.imports.keys().cloned().collect::<BTreeSet<_>>();
    let references = import_references(spec);

    for (id, import) in &spec.imports {
        validate_identifier("imports key", id)?;
        insert_unique("imports key", &mut seen, id)?;
        if import.path.trim().is_empty() {
            return Err(Error::MissingField {
                field: "imports.path",
            });
        }
        if import.section.as_deref().is_some_and(str::is_empty) {
            return Err(Error::MissingField {
                field: "imports.section",
            });
        }
        for required_import in &import.requires.imports {
            validate_known_import("imports.requires.imports", &import_ids, required_import)?;
        }
        for use_ref in &import.used_by {
            validate_import_use(spec, use_ref)?;
        }
        if !references.contains_key(id)
            && import.used_by.is_empty()
            && import.load != ImportLoad::Always
        {
            return Err(Error::UnknownReference {
                field: "imports.orphan",
                value: id.clone(),
            });
        }
    }
    validate_import_cycles(spec)
}

fn validate_code(spec: &SkillSpec) -> Result<()> {
    let import_ids = spec.imports.keys().cloned().collect::<BTreeSet<_>>();
    let resource_ids = spec.resources.keys().cloned().collect::<BTreeSet<_>>();
    let dependency_ids = spec.dependencies.keys().cloned().collect::<BTreeSet<_>>();
    let artifact_ids = spec.artifacts.keys().cloned().collect::<BTreeSet<_>>();

    for (id, code) in &spec.code {
        validate_identifier("code key", id)?;
        if code.language.trim().is_empty() {
            return Err(Error::MissingField {
                field: "code.language",
            });
        }
        validate_code_source(&resource_ids, &code.source)?;
        if let Some(provenance) = &code.provenance {
            match (&provenance.resource, &provenance.import) {
                (Some(resource), None) => {
                    validate_known_resource("code.provenance.resource", &resource_ids, resource)?;
                }
                (None, Some(import)) => {
                    validate_known_import("code.provenance.import", &import_ids, import)?;
                }
                (None, None) => {
                    return Err(Error::MissingField {
                        field: "code.provenance.resource_or_import",
                    });
                }
                (Some(_), Some(_)) => {
                    return Err(Error::UnknownReference {
                        field: "code.provenance.resource_or_import",
                        value: "resource and import both set".to_owned(),
                    });
                }
            }
        }
        for dependency in &code.requires.dependencies {
            validate_known_dependency("code.requires.dependencies", &dependency_ids, dependency)?;
        }
        for import in &code.requires.imports {
            validate_known_import("code.requires.imports", &import_ids, import)?;
        }
        for resource in &code.requires.resources {
            validate_known_resource("code.requires.resources", &resource_ids, resource)?;
        }
        for artifact in &code.requires.artifacts {
            validate_known_artifact("code.requires.artifacts", &artifact_ids, artifact)?;
        }
        for artifact in code.inputs.iter().chain(code.outputs.iter()) {
            validate_known_artifact("code.inputs.outputs", &artifact_ids, artifact)?;
        }
    }
    Ok(())
}

fn validate_code_source(resource_ids: &BTreeSet<String>, source: &CodeSource) -> Result<()> {
    match source {
        CodeSource::Inline(inline_source) => {
            if inline_source.inline.trim().is_empty() {
                return Err(Error::MissingField {
                    field: "code.source.inline",
                });
            }
        }
        CodeSource::File(file) => {
            if file.file.trim().is_empty() {
                return Err(Error::MissingField {
                    field: "code.source.file",
                });
            }
            if let Some(resource) = &file.from_resource {
                validate_known_resource("code.source.from_resource", resource_ids, resource)?;
            }
        }
    }
    Ok(())
}

fn validate_artifacts(spec: &SkillSpec) -> Result<()> {
    for (id, artifact) in &spec.artifacts {
        validate_identifier("artifacts key", id)?;
        if artifact.path.as_deref().is_some_and(str::is_empty) {
            return Err(Error::MissingField {
                field: "artifacts.path",
            });
        }
        for producer in &artifact.produced_by {
            validate_executable_ref(spec, "artifacts.produced_by", producer)?;
        }
        for consumer in &artifact.consumed_by {
            validate_executable_ref(spec, "artifacts.consumed_by", consumer)?;
        }
    }
    Ok(())
}

fn validate_recipes(spec: &SkillSpec) -> Result<()> {
    let import_ids = spec.imports.keys().cloned().collect::<BTreeSet<_>>();
    let resource_ids = spec.resources.keys().cloned().collect::<BTreeSet<_>>();
    let dependency_ids = spec.dependencies.keys().cloned().collect::<BTreeSet<_>>();
    let artifact_ids = spec.artifacts.keys().cloned().collect::<BTreeSet<_>>();
    let command_ids = spec.commands.keys().cloned().collect::<BTreeSet<_>>();
    let code_ids = spec.code.keys().cloned().collect::<BTreeSet<_>>();
    let recipe_ids = spec.recipes.keys().cloned().collect::<BTreeSet<_>>();
    let elicitation_ids = elicitation_ids(spec);
    let refs = RecipeValidationRefs {
        import_ids: &import_ids,
        resource_ids: &resource_ids,
        command_ids: &command_ids,
        code_ids: &code_ids,
        recipe_ids: &recipe_ids,
        artifact_ids: &artifact_ids,
        elicitation_ids: &elicitation_ids,
    };

    for (id, recipe) in &spec.recipes {
        validate_identifier("recipes key", id)?;
        for resource in &recipe.requires.resources {
            validate_known_resource("recipes.requires.resources", &resource_ids, resource)?;
        }
        for import in &recipe.requires.imports {
            validate_known_import("recipes.requires.imports", &import_ids, import)?;
        }
        for dependency in &recipe.requires.dependencies {
            validate_known_dependency(
                "recipes.requires.dependencies",
                &dependency_ids,
                dependency,
            )?;
        }
        for artifact in &recipe.requires.artifacts {
            validate_known_artifact("recipes.requires.artifacts", &artifact_ids, artifact)?;
        }
        for step in &recipe.steps {
            validate_recipe_step(step, &refs)?;
        }
    }
    Ok(())
}

struct RecipeValidationRefs<'a> {
    import_ids: &'a BTreeSet<String>,
    resource_ids: &'a BTreeSet<String>,
    command_ids: &'a BTreeSet<String>,
    code_ids: &'a BTreeSet<String>,
    recipe_ids: &'a BTreeSet<String>,
    artifact_ids: &'a BTreeSet<String>,
    elicitation_ids: &'a BTreeSet<String>,
}

fn validate_recipe_step(step: &RecipeStep, refs: &RecipeValidationRefs<'_>) -> Result<()> {
    match step {
        RecipeStep::LoadImport(step) => validate_known_import(
            "recipes.steps.load_import",
            refs.import_ids,
            &step.load_import,
        ),
        RecipeStep::LoadResource(step) => validate_known_resource(
            "recipes.steps.load_resource",
            refs.resource_ids,
            &step.load_resource,
        ),
        RecipeStep::RunCommand(step) => validate_known_action(
            "recipes.steps.run_command",
            refs.command_ids,
            &step.run_command,
        ),
        RecipeStep::RunCode(step) => {
            validate_known_code("recipes.steps.run_code", refs.code_ids, &step.run_code)
        }
        RecipeStep::ProduceArtifact(step) => validate_known_artifact(
            "recipes.steps.produce_artifact",
            refs.artifact_ids,
            &step.produce_artifact,
        ),
        RecipeStep::ConsumeArtifact(step) => validate_known_artifact(
            "recipes.steps.consume_artifact",
            refs.artifact_ids,
            &step.consume_artifact,
        ),
        RecipeStep::Ask(step) => {
            validate_known_elicitation("recipes.steps.ask", refs.elicitation_ids, &step.ask)
        }
        RecipeStep::Branch(step) => {
            let branch = &step.branch;
            if branch.if_condition.trim().is_empty() {
                return Err(Error::MissingField {
                    field: "recipes.steps.branch.if",
                });
            }
            validate_branch_target(
                "recipes.steps.branch.then",
                refs.command_ids,
                refs.code_ids,
                refs.recipe_ids,
                &branch.then,
            )?;
            if let Some(otherwise) = &branch.otherwise {
                validate_branch_target(
                    "recipes.steps.branch.otherwise",
                    refs.command_ids,
                    refs.code_ids,
                    refs.recipe_ids,
                    otherwise,
                )?;
            }
            Ok(())
        }
        RecipeStep::Note(step) => {
            if step.note.trim().is_empty() {
                return Err(Error::MissingField {
                    field: "recipes.steps.note",
                });
            }
            Ok(())
        }
    }
}

fn validate_branch_target(
    field: &'static str,
    command_ids: &BTreeSet<String>,
    code_ids: &BTreeSet<String>,
    recipe_ids: &BTreeSet<String>,
    target: &str,
) -> Result<()> {
    if command_ids.contains(target) || code_ids.contains(target) || recipe_ids.contains(target) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: target.to_owned(),
        })
    }
}

fn validate_commands(spec: &SkillSpec) -> Result<()> {
    let dependency_ids = spec.dependencies.keys().cloned().collect::<BTreeSet<_>>();

    for (command_id, command) in &spec.commands {
        validate_identifier("commands key", command_id)?;
        for dependency in &command.requires.dependencies {
            validate_known_dependency(
                "commands.requires.dependencies",
                &dependency_ids,
                dependency,
            )?;
        }
    }
    Ok(())
}

fn validate_elicitations(spec: &SkillSpec) -> Result<()> {
    let route_ids = route_ids(spec);
    let state_ids = spec.states.keys().cloned().collect::<BTreeSet<_>>();

    for (id, elicitation) in &spec.elicitations {
        validate_identifier("elicitations key", id)?;
        if elicitation.question.trim().is_empty() {
            return Err(Error::MissingField {
                field: "elicitations.question",
            });
        }
        if elicitation.choices.is_empty() {
            return Err(Error::MissingField {
                field: "elicitations.choices",
            });
        }
        let mut choice_ids = BTreeSet::new();
        for choice in &elicitation.choices {
            validate_identifier("elicitations.choices.id", &choice.id)?;
            insert_unique("elicitations.choices.id", &mut choice_ids, &choice.id)?;
            if choice.label.trim().is_empty() {
                return Err(Error::MissingField {
                    field: "elicitations.choices.label",
                });
            }
            if let Some(route) = &choice.route {
                validate_known_route("elicitations.choices.route", &route_ids, route)?;
            }
            if let Some(next) = &choice.next {
                validate_known_state("elicitations.choices.next", &state_ids, next)?;
            }
        }
        if let Some(default) = &elicitation.default {
            if !choice_ids.contains(default) {
                return Err(Error::UnknownReference {
                    field: "elicitations.default",
                    value: default.clone(),
                });
            }
        }
        for condition in &elicitation.required_when {
            if let Some(route) = &condition.route {
                validate_known_route("elicitations.required_when.route", &route_ids, route)?;
            }
            if let Some(missing) = &condition.missing {
                validate_identifier("elicitations.required_when.missing", missing)?;
            }
        }
    }
    Ok(())
}

fn validate_trace(spec: &SkillSpec) -> Result<()> {
    let Some(trace) = &spec.trace else {
        return Ok(());
    };
    let mut seen = BTreeSet::new();
    for event in &trace.record {
        let value = format!("{event:?}");
        insert_unique("trace.record", &mut seen, &value)?;
    }
    Ok(())
}

fn route_ids(spec: &SkillSpec) -> BTreeSet<String> {
    spec.routes.iter().map(|route| route.id.0.clone()).collect()
}

fn rule_ids(spec: &SkillSpec) -> BTreeSet<String> {
    spec.rules.iter().map(|rule| rule.id.0.clone()).collect()
}

fn action_ids(spec: &SkillSpec) -> BTreeSet<String> {
    spec.commands
        .keys()
        .chain(spec.recipes.keys())
        .chain(spec.code.keys())
        .chain(spec.closures.keys())
        .cloned()
        .collect()
}

fn elicitation_ids(spec: &SkillSpec) -> BTreeSet<String> {
    spec.elicitations.keys().cloned().collect()
}

fn validate_known_route(
    field: &'static str,
    route_ids: &BTreeSet<String>,
    route: &RouteId,
) -> Result<()> {
    if route_ids.contains(&route.0) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: route.0.clone(),
        })
    }
}

fn validate_known_state(
    field: &'static str,
    state_ids: &BTreeSet<String>,
    state: &str,
) -> Result<()> {
    if state_ids.contains(state) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: state.to_owned(),
        })
    }
}

fn validate_known_action(
    field: &'static str,
    action_ids: &BTreeSet<String>,
    action: &str,
) -> Result<()> {
    if action_ids.contains(action) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: action.to_owned(),
        })
    }
}

fn validate_known_elicitation(
    field: &'static str,
    elicitation_ids: &BTreeSet<String>,
    elicitation: &str,
) -> Result<()> {
    if elicitation_ids.contains(elicitation) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: elicitation.to_owned(),
        })
    }
}

fn validate_known_rule(
    field: &'static str,
    rule_ids: &BTreeSet<String>,
    rule: &RuleId,
) -> Result<()> {
    if rule_ids.contains(&rule.0) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: rule.0.clone(),
        })
    }
}

fn validate_known_dependency(
    field: &'static str,
    dependency_ids: &BTreeSet<String>,
    dependency: &str,
) -> Result<()> {
    if dependency_ids.contains(dependency) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: dependency.to_owned(),
        })
    }
}

fn validate_known_import(
    field: &'static str,
    import_ids: &BTreeSet<String>,
    import: &str,
) -> Result<()> {
    if import_ids.contains(import) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: import.to_owned(),
        })
    }
}

fn validate_known_resource(
    field: &'static str,
    resource_ids: &BTreeSet<String>,
    resource: &str,
) -> Result<()> {
    if resource_ids.contains(resource) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: resource.to_owned(),
        })
    }
}

fn validate_known_code(field: &'static str, code_ids: &BTreeSet<String>, code: &str) -> Result<()> {
    if code_ids.contains(code) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: code.to_owned(),
        })
    }
}

fn validate_known_artifact(
    field: &'static str,
    artifact_ids: &BTreeSet<String>,
    artifact: &str,
) -> Result<()> {
    if artifact_ids.contains(artifact) {
        Ok(())
    } else {
        Err(Error::UnknownReference {
            field,
            value: artifact.to_owned(),
        })
    }
}

fn validate_executable_ref<T>(spec: &SkillSpec, field: &'static str, reference: &T) -> Result<()>
where
    T: ExecutableReference,
{
    match reference.kind() {
        ExecutableRefKind::Command => {
            let ids = spec.commands.keys().cloned().collect::<BTreeSet<_>>();
            validate_known_action(field, &ids, reference.id())
        }
        ExecutableRefKind::Code => {
            let ids = spec.code.keys().cloned().collect::<BTreeSet<_>>();
            validate_known_code(field, &ids, reference.id())
        }
        ExecutableRefKind::Recipe => {
            let ids = spec.recipes.keys().cloned().collect::<BTreeSet<_>>();
            validate_known_action(field, &ids, reference.id())
        }
    }
}

trait ExecutableReference {
    fn kind(&self) -> ExecutableRefKind;
    fn id(&self) -> &str;
}

impl ExecutableReference for ProducerRef {
    fn kind(&self) -> ExecutableRefKind {
        self.kind.clone()
    }

    fn id(&self) -> &str {
        &self.id
    }
}

impl ExecutableReference for ConsumerRef {
    fn kind(&self) -> ExecutableRefKind {
        self.kind.clone()
    }

    fn id(&self) -> &str {
        &self.id
    }
}

fn validate_import_use(spec: &SkillSpec, use_ref: &ImportUse) -> Result<()> {
    match use_ref.kind {
        ImportUseKind::Route => {
            let ids = route_ids(spec);
            validate_known_action("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::Rule => {
            let ids = spec.rules.iter().map(|rule| rule.id.0.clone()).collect();
            validate_known_action("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::State => {
            let ids = spec.states.keys().cloned().collect();
            validate_known_action("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::Elicitation => {
            let ids = elicitation_ids(spec);
            validate_known_elicitation("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::Dependency => {
            let ids = spec.dependencies.keys().cloned().collect();
            validate_known_dependency("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::Command => {
            let ids = spec.commands.keys().cloned().collect();
            validate_known_action("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::Code => {
            let ids = spec.code.keys().cloned().collect();
            validate_known_code("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::Artifact => {
            let ids = spec.artifacts.keys().cloned().collect();
            validate_known_artifact("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::Recipe => {
            let ids = spec.recipes.keys().cloned().collect();
            validate_known_action("imports.used_by", &ids, &use_ref.id)
        }
        ImportUseKind::Snippet => {
            let ids = spec.snippets.keys().cloned().collect();
            validate_known_action("imports.used_by", &ids, &use_ref.id)
        }
    }
}

fn validate_resource_use(spec: &SkillSpec, use_ref: &ResourceUse) -> Result<()> {
    match use_ref.kind {
        ResourceUseKind::Route => {
            let ids = route_ids(spec);
            validate_known_action("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::Rule => {
            let ids = spec.rules.iter().map(|rule| rule.id.0.clone()).collect();
            validate_known_action("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::State => {
            let ids = spec.states.keys().cloned().collect();
            validate_known_action("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::Elicitation => {
            let ids = elicitation_ids(spec);
            validate_known_elicitation("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::Dependency => {
            let ids = spec.dependencies.keys().cloned().collect();
            validate_known_dependency("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::Command => {
            let ids = spec.commands.keys().cloned().collect();
            validate_known_action("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::Code => {
            let ids = spec.code.keys().cloned().collect();
            validate_known_code("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::Artifact => {
            let ids = spec.artifacts.keys().cloned().collect();
            validate_known_artifact("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::Recipe => {
            let ids = spec.recipes.keys().cloned().collect();
            validate_known_action("resources.used_by", &ids, &use_ref.id)
        }
        ResourceUseKind::Snippet => {
            let ids = spec.snippets.keys().cloned().collect();
            validate_known_action("resources.used_by", &ids, &use_ref.id)
        }
    }
}

fn import_references(spec: &SkillSpec) -> BTreeMap<String, usize> {
    let mut references = BTreeMap::new();
    for import in spec.imports.values() {
        for required_import in &import.requires.imports {
            increment(&mut references, required_import);
        }
    }
    for code in spec.code.values() {
        for import in &code.requires.imports {
            increment(&mut references, import);
        }
    }
    for recipe in spec.recipes.values() {
        for import in &recipe.requires.imports {
            increment(&mut references, import);
        }
        for step in &recipe.steps {
            if let RecipeStep::LoadImport(step) = step {
                increment(&mut references, &step.load_import);
            }
        }
    }
    references
}

fn validate_import_cycles(spec: &SkillSpec) -> Result<()> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in spec.imports.keys() {
        validate_import_cycle_from(id, spec, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn validate_import_cycle_from(
    id: &str,
    spec: &SkillSpec,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<()> {
    if visited.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id.to_owned()) {
        return Err(Error::UnknownReference {
            field: "imports.requires.imports.cycle",
            value: id.to_owned(),
        });
    }
    if let Some(import) = spec.imports.get(id) {
        for child in &import.requires.imports {
            validate_import_cycle_from(child, spec, visiting, visited)?;
        }
    }
    visiting.remove(id);
    visited.insert(id.to_owned());
    Ok(())
}

fn resource_references(spec: &SkillSpec) -> BTreeMap<String, usize> {
    let mut references = BTreeMap::new();
    for code in spec.code.values() {
        if let CodeSource::File(file) = &code.source {
            if let Some(resource) = &file.from_resource {
                increment(&mut references, resource);
            }
        }
        for resource in &code.requires.resources {
            increment(&mut references, resource);
        }
    }
    for recipe in spec.recipes.values() {
        for resource in &recipe.requires.resources {
            increment(&mut references, resource);
        }
        for step in &recipe.steps {
            if let RecipeStep::LoadResource(step) = step {
                increment(&mut references, &step.load_resource);
            }
        }
    }
    references
}

fn increment(counts: &mut BTreeMap<String, usize>, id: &str) {
    *counts.entry(id.to_owned()).or_default() += 1;
}

fn insert_unique(field: &'static str, seen: &mut BTreeSet<String>, value: &str) -> Result<()> {
    if seen.insert(value.to_owned()) {
        Ok(())
    } else {
        Err(Error::DuplicateId {
            field,
            value: value.to_owned(),
        })
    }
}

fn validate_route_id(field: &'static str, route: &RouteId) -> Result<()> {
    validate_identifier(field, &route.0)
}

fn validate_rule_id(field: &'static str, rule: &RuleId) -> Result<()> {
    validate_identifier(field, &rule.0)
}

fn validate_identifier(field: &'static str, value: &str) -> Result<()> {
    let mut chars = value.chars();
    let valid = chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|char| {
            char.is_ascii_lowercase()
                || char.is_ascii_digit()
                || char == '_'
                || char == '-'
                || char == '.'
        });

    if valid {
        Ok(())
    } else {
        Err(Error::InvalidIdentifier {
            field,
            value: value.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_rule_fields_are_rejected() {
        let yaml = r#"
schema: skillspec/v0
id: typo.spec
title: Typo Spec
description: Demonstrates ignored fields.
routes:
  - id: local
    label: Local
rules:
  - id: typo_rule
    when:
      user_says_anny: ["run"]
    preferr: local
tests:
  - name: route assertion
    input: run this
    expect:
      route: local
"#;

        let error = serde_yaml::from_str::<SkillSpec>(yaml).unwrap_err();

        assert!(
            error.to_string().contains("unknown field"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn unknown_fields_are_rejected_across_typed_grammar() {
        let cases = [
            ("top_level", "unexpected: true"),
            ("entry", "entry:\n  prompt: Start\n  unexpected: true"),
            (
                "route",
                "routes:\n  - id: local\n    label: Local\n    unexpected: true",
            ),
            (
                "rule",
                "rules:\n  - id: local_rule\n    prefer: local\n    unexpected: true",
            ),
            (
                "predicate",
                "rules:\n  - id: local_rule\n    when:\n      user_says_anny: [run]",
            ),
            ("state", "states:\n  start:\n    nexxt: done"),
            (
                "elicitation",
                "elicitations:\n  mode:\n    question: Mode?\n    choices:\n      - id: fast\n        label: Fast\n    unexpected: true",
            ),
            (
                "elicitation_condition",
                "elicitations:\n  mode:\n    question: Mode?\n    required_when:\n      - route: local\n        unexpected: true\n    choices:\n      - id: fast\n        label: Fast",
            ),
            (
                "elicitation_choice",
                "elicitations:\n  mode:\n    question: Mode?\n    choices:\n      - id: fast\n        label: Fast\n        unexpected: true",
            ),
            (
                "trace",
                "trace:\n  mode: event_log\n  unexpected: true",
            ),
            (
                "dependency",
                "dependencies:\n  git:\n    kind: cli\n    unexpected: true",
            ),
            (
                "dependency_check",
                "dependencies:\n  git:\n    kind: cli\n    check:\n      command: git\n      unexpected: true",
            ),
            (
                "dependency_permission",
                "dependencies:\n  git:\n    kind: cli\n    permission:\n      required: true\n      unexpected: true",
            ),
            (
                "dependency_provision",
                "dependencies:\n  git:\n    kind: cli\n    provision:\n      unexpected: true",
            ),
            (
                "dependency_provision_option",
                "dependencies:\n  git:\n    kind: cli\n    provision:\n      options:\n        - id: install\n          label: Install\n          unexpected: true",
            ),
            (
                "import",
                "imports:\n  shared_rules:\n    path: ../INDEX.md\n    role: policy\n    load: always\n    unexpected: true",
            ),
            (
                "import_requires",
                "imports:\n  task_reference:\n    path: references/task.md\n    role: reference\n    requires:\n      imports: []\n      unexpected: true\n    used_by:\n      - kind: route\n        id: local",
            ),
            (
                "import_use",
                "imports:\n  task_reference:\n    path: references/task.md\n    role: reference\n    used_by:\n      - kind: route\n        id: local\n        unexpected: true",
            ),
            (
                "resource",
                "resources:\n  source:\n    path: SKILL.md\n    role: source_material\n    unexpected: true",
            ),
            (
                "resource_use",
                "resources:\n  source:\n    path: SKILL.md\n    role: source_material\n    used_by:\n      - kind: route\n        id: local\n        unexpected: true",
            ),
            (
                "command",
                "commands:\n  run:\n    template: echo ok\n    unexpected: true",
            ),
            (
                "command_requires",
                "commands:\n  run:\n    template: echo ok\n    requires:\n      dependencies: []\n      unexpected: true",
            ),
            (
                "code_block",
                "code:\n  sample:\n    language: text\n    kind: example\n    source:\n      inline: ok\n    unexpected: true",
            ),
            (
                "code_source_inline",
                "code:\n  sample:\n    language: text\n    kind: example\n    source:\n      inline: ok\n      unexpected: true",
            ),
            (
                "code_source_file",
                "code:\n  sample:\n    language: text\n    kind: example\n    source:\n      file: script.sh\n      unexpected: true",
            ),
            (
                "code_provenance",
                "code:\n  sample:\n    language: text\n    kind: example\n    source:\n      inline: ok\n    provenance:\n      resource: source\n      unexpected: true",
            ),
            (
                "code_requires",
                "code:\n  sample:\n    language: text\n    kind: example\n    source:\n      inline: ok\n    requires:\n      dependencies: []\n      unexpected: true",
            ),
            (
                "code_safety",
                "code:\n  sample:\n    language: text\n    kind: example\n    source:\n      inline: ok\n    safety:\n      writes_files: false\n      unexpected: true",
            ),
            (
                "artifact",
                "artifacts:\n  report:\n    kind: report\n    unexpected: true",
            ),
            (
                "producer_ref",
                "artifacts:\n  report:\n    kind: report\n    produced_by:\n      - kind: command\n        id: run\n        unexpected: true",
            ),
            (
                "consumer_ref",
                "artifacts:\n  report:\n    kind: report\n    consumed_by:\n      - kind: command\n        id: run\n        unexpected: true",
            ),
            (
                "recipe",
                "recipes:\n  main:\n    ordered: true\n    unexpected: true",
            ),
            (
                "recipe_requires",
                "recipes:\n  main:\n    requires:\n      dependencies: []\n      unexpected: true",
            ),
            (
                "recipe_step_load_resource",
                "recipes:\n  main:\n    steps:\n      - load_resource: source\n        unexpected: true",
            ),
            (
                "recipe_step_load_import",
                "recipes:\n  main:\n    steps:\n      - load_import: task_reference\n        unexpected: true",
            ),
            (
                "recipe_step_run_command",
                "recipes:\n  main:\n    steps:\n      - run_command: run\n        unexpected: true",
            ),
            (
                "recipe_step_run_code",
                "recipes:\n  main:\n    steps:\n      - run_code: sample\n        unexpected: true",
            ),
            (
                "recipe_step_produce_artifact",
                "recipes:\n  main:\n    steps:\n      - produce_artifact: report\n        unexpected: true",
            ),
            (
                "recipe_step_consume_artifact",
                "recipes:\n  main:\n    steps:\n      - consume_artifact: report\n        unexpected: true",
            ),
            (
                "recipe_step_ask",
                "recipes:\n  main:\n    steps:\n      - ask: mode\n        unexpected: true",
            ),
            (
                "recipe_step_branch",
                "recipes:\n  main:\n    steps:\n      - branch:\n          if: condition\n          then: main\n        unexpected: true",
            ),
            (
                "recipe_branch",
                "recipes:\n  main:\n    steps:\n      - branch:\n          if: condition\n          then: main\n          unexpected: true",
            ),
            (
                "recipe_step_note",
                "recipes:\n  main:\n    steps:\n      - note: remember this\n        unexpected: true",
            ),
            (
                "snippet",
                "snippets:\n  summary:\n    text: ok\n    unexpected: true",
            ),
            (
                "proof",
                "proof:\n  metrics: []\n  unexpected: true",
            ),
            (
                "scenario_test",
                "tests:\n  - name: route assertion\n    input: run\n    expect:\n      route: local\n    unexpected: true",
            ),
            (
                "expectation",
                "tests:\n  - name: route assertion\n    input: run\n    expect:\n      route: local\n      unexpected: true",
            ),
        ];

        for (name, body) in cases {
            let yaml = spec_with(body);
            assert!(
                serde_yaml::from_str::<SkillSpec>(&yaml).is_err(),
                "case {name} unexpectedly parsed"
            );
        }
    }

    #[test]
    fn explicit_extension_surfaces_accept_arbitrary_fields() {
        let yaml = spec_with(
            r#"
applies_when:
  - product_specific_hint:
      nested: true
routes:
  - id: local
    label: Local
rules:
  - id: local_rule
    prefer: local
    allow:
      product_specific_fallback:
        nested: true
elicitations:
  mode:
    question: Mode?
    choices:
      - id: fast
        label: Fast
        sets:
          product_specific_fact:
            nested: true
commands:
  run:
    template: echo ok
    success_when:
      product_specific_check:
        nested: true
artifacts:
  report:
    kind: report
    schema:
      product_specific_schema:
        nested: true
closures:
  product_specific_closure:
    nested: true
metadata:
  product_specific_metadata:
    nested: true
tests:
  - name: route assertion
    input: run
    expect:
      route: local
"#,
        );

        let spec = serde_yaml::from_str::<SkillSpec>(&yaml).unwrap();
        validate_spec(&spec).unwrap();
    }

    #[test]
    fn tests_must_have_at_least_one_assertion() {
        let yaml = r#"
schema: skillspec/v0
id: empty.expectation
title: Empty Expectation
description: Demonstrates empty expectation rejection.
routes:
  - id: local
    label: Local
tests:
  - name: empty expectation
    input: run this
    expect: {}
"#;
        let spec = serde_yaml::from_str::<SkillSpec>(yaml).unwrap();
        let error = validate_spec(&spec).unwrap_err();

        assert!(error.to_string().contains("tests.expect assertion"));
    }

    #[test]
    fn imports_must_be_referenced_unless_loaded_always() {
        let yaml = r#"
schema: skillspec/v0
id: imports.orphan
title: Import Orphan
description: Demonstrates orphan import rejection.
imports:
  task_reference:
    path: references/task.md
    role: reference
"#;
        let spec = serde_yaml::from_str::<SkillSpec>(yaml).unwrap();
        let error = validate_spec(&spec).unwrap_err();

        assert!(error.to_string().contains("imports.orphan"));
    }

    #[test]
    fn imports_detect_cycles() {
        let yaml = r#"
schema: skillspec/v0
id: imports.cycle
title: Import Cycle
description: Demonstrates import cycle rejection.
imports:
  one:
    path: one.md
    role: reference
    requires:
      imports: [two]
  two:
    path: two.md
    role: reference
    requires:
      imports: [one]
"#;
        let spec = serde_yaml::from_str::<SkillSpec>(yaml).unwrap();
        let error = validate_spec(&spec).unwrap_err();

        assert!(error.to_string().contains("imports.requires.imports.cycle"));
    }

    fn spec_with(body: &str) -> String {
        format!(
            r#"schema: skillspec/v0
id: typo.coverage
title: Typo Coverage
description: Tests strict grammar coverage.
{body}
"#
        )
    }
}
