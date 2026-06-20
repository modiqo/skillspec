use crate::error::{Error, Result};
use crate::model::{RouteId, RuleId, SkillSpec};
use std::collections::BTreeSet;
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
    validate_rules(spec)?;
    validate_states(spec)?;
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

fn route_ids(spec: &SkillSpec) -> BTreeSet<String> {
    spec.routes.iter().map(|route| route.id.0.clone()).collect()
}

fn action_ids(spec: &SkillSpec) -> BTreeSet<String> {
    spec.commands
        .keys()
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
