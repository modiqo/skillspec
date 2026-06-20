use crate::error::{Error, Result};
use crate::model::{
    CodeSource, ConsumerRef, ExecutableRefKind, ProducerRef, RecipeStep, ResourceUse,
    ResourceUseKind, RouteId, RuleId, SkillSpec,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_spec(path: &Path) -> Result<SkillSpec> {
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
        if let Some(route) = &test.expect.route {
            validate_known_route("tests.expect.route", &route_ids, route)?;
        }
        for route in &test.expect.route_order {
            validate_known_route("tests.expect.route_order", &route_ids, route)?;
        }
        for action in &test.expect.after_success {
            validate_known_action("tests.expect.after_success", &action_ids, action)?;
        }
        for elicitation in &test.expect.elicit {
            validate_known_elicitation("tests.expect.elicit", &elicitation_ids, elicitation)?;
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

fn validate_code(spec: &SkillSpec) -> Result<()> {
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
            validate_known_resource(
                "code.provenance.resource",
                &resource_ids,
                &provenance.resource,
            )?;
        }
        for dependency in &code.requires.dependencies {
            validate_known_dependency("code.requires.dependencies", &dependency_ids, dependency)?;
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
        CodeSource::Inline { inline } => {
            if inline.trim().is_empty() {
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
    let resource_ids = spec.resources.keys().cloned().collect::<BTreeSet<_>>();
    let dependency_ids = spec.dependencies.keys().cloned().collect::<BTreeSet<_>>();
    let artifact_ids = spec.artifacts.keys().cloned().collect::<BTreeSet<_>>();
    let command_ids = spec.commands.keys().cloned().collect::<BTreeSet<_>>();
    let code_ids = spec.code.keys().cloned().collect::<BTreeSet<_>>();
    let elicitation_ids = elicitation_ids(spec);

    for (id, recipe) in &spec.recipes {
        validate_identifier("recipes key", id)?;
        for resource in &recipe.requires.resources {
            validate_known_resource("recipes.requires.resources", &resource_ids, resource)?;
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
            validate_recipe_step(
                step,
                &resource_ids,
                &command_ids,
                &code_ids,
                &spec.recipes.keys().cloned().collect(),
                &artifact_ids,
                &elicitation_ids,
            )?;
        }
    }
    Ok(())
}

fn validate_recipe_step(
    step: &RecipeStep,
    resource_ids: &BTreeSet<String>,
    command_ids: &BTreeSet<String>,
    code_ids: &BTreeSet<String>,
    recipe_ids: &BTreeSet<String>,
    artifact_ids: &BTreeSet<String>,
    elicitation_ids: &BTreeSet<String>,
) -> Result<()> {
    match step {
        RecipeStep::LoadResource { load_resource } => {
            validate_known_resource("recipes.steps.load_resource", resource_ids, load_resource)
        }
        RecipeStep::RunCommand { run_command } => {
            validate_known_action("recipes.steps.run_command", command_ids, run_command)
        }
        RecipeStep::RunCode { run_code } => {
            validate_known_code("recipes.steps.run_code", code_ids, run_code)
        }
        RecipeStep::ProduceArtifact { produce_artifact } => validate_known_artifact(
            "recipes.steps.produce_artifact",
            artifact_ids,
            produce_artifact,
        ),
        RecipeStep::ConsumeArtifact { consume_artifact } => validate_known_artifact(
            "recipes.steps.consume_artifact",
            artifact_ids,
            consume_artifact,
        ),
        RecipeStep::Ask { ask } => {
            validate_known_elicitation("recipes.steps.ask", elicitation_ids, ask)
        }
        RecipeStep::Branch { branch } => {
            if branch.if_condition.trim().is_empty() {
                return Err(Error::MissingField {
                    field: "recipes.steps.branch.if",
                });
            }
            validate_branch_target(
                "recipes.steps.branch.then",
                command_ids,
                code_ids,
                recipe_ids,
                &branch.then,
            )?;
            if let Some(otherwise) = &branch.otherwise {
                validate_branch_target(
                    "recipes.steps.branch.otherwise",
                    command_ids,
                    code_ids,
                    recipe_ids,
                    otherwise,
                )?;
            }
            Ok(())
        }
        RecipeStep::Note { note } => {
            if note.trim().is_empty() {
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
            if let RecipeStep::LoadResource { load_resource } = step {
                increment(&mut references, load_resource);
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
