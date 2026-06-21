use crate::error::{Error, Result};
use crate::model::{CommandRequires, Predicate, RecipeStep, RouteId, SkillSpec};
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt::Write;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    Index,
    Summary,
    Full,
}

#[derive(Clone, Debug, Serialize)]
pub struct SensemakeReport {
    pub spec_id: String,
    pub title: String,
    pub spec_path: String,
    pub view: View,
    pub sections: Vec<SectionMap>,
    pub navigation: Vec<NavigationHint>,
    pub escalation: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SectionMap {
    pub name: &'static str,
    pub role: &'static str,
    pub count: usize,
    pub ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NavigationHint {
    pub intent: &'static str,
    pub command: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryReport {
    pub spec_id: String,
    pub spec_path: String,
    pub handle: String,
    pub view: View,
    pub target: QueryTarget,
    pub value: Value,
    pub query_hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryTarget {
    pub kind: String,
    pub id: Option<String>,
    pub field_path: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RefsReport {
    pub spec_id: String,
    pub spec_path: String,
    pub handle: String,
    pub view: View,
    pub target: QueryTarget,
    pub outgoing: Vec<ReferenceEdge>,
    pub query_hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReferenceEdge {
    pub field: String,
    pub kind: String,
    pub ids: Vec<String>,
}

pub fn sensemake(spec: &SkillSpec, path: &Path, view: View) -> SensemakeReport {
    let spec_path = path.display().to_string();
    SensemakeReport {
        spec_id: spec.id.clone(),
        title: spec.title.clone(),
        spec_path: spec_path.clone(),
        view,
        sections: sections(spec, view),
        navigation: navigation(&spec_path),
        escalation: vec![
            "start with sensemake --view index only when unfamiliar".to_owned(),
            "use decide for task routing".to_owned(),
            "use query/refs for matched ids instead of reading the whole YAML".to_owned(),
            "escalate index -> summary -> full only when needed".to_owned(),
        ],
    }
}

pub fn query(spec: &SkillSpec, path: &Path, handle: &str, view: View) -> Result<QueryReport> {
    let parsed = ParsedHandle::parse(handle)?;
    let selected = select_value(spec, &parsed, view)?;
    let query_hints = query_hints(path, &parsed);
    Ok(QueryReport {
        spec_id: spec.id.clone(),
        spec_path: path.display().to_string(),
        handle: handle.to_owned(),
        view,
        target: QueryTarget {
            kind: parsed.kind.clone(),
            id: parsed.id.clone(),
            field_path: parsed.field_path.clone(),
        },
        value: selected,
        query_hints,
    })
}

pub fn refs(spec: &SkillSpec, path: &Path, handle: &str, view: View) -> Result<RefsReport> {
    let parsed = ParsedHandle::parse(handle)?;
    if parsed.id.is_none() {
        return Err(Error::InvalidInput {
            message: format!("refs requires an item handle such as rule:<id>, got {handle:?}"),
        });
    }
    let outgoing = outgoing_refs(spec, &parsed)?;
    let query_hints = refs_hints(path, &parsed, &outgoing);
    Ok(RefsReport {
        spec_id: spec.id.clone(),
        spec_path: path.display().to_string(),
        handle: handle.to_owned(),
        view,
        target: QueryTarget {
            kind: parsed.kind.clone(),
            id: parsed.id.clone(),
            field_path: parsed.field_path.clone(),
        },
        outgoing,
        query_hints,
    })
}

pub fn render_sensemake(report: &SensemakeReport) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "SkillSpec map: {} ({})",
        report.title, report.spec_id
    )
    .unwrap();
    writeln!(output, "spec: {}", report.spec_path).unwrap();
    writeln!(output).unwrap();
    writeln!(output, "Grammar shape:").unwrap();
    for section in &report.sections {
        writeln!(
            output,
            "- {}: {} ({})",
            section.name, section.role, section.count
        )
        .unwrap();
    }
    writeln!(output).unwrap();
    writeln!(output, "Query handles:").unwrap();
    for section in &report.sections {
        let ids = if section.ids.is_empty() {
            "<none>".to_owned()
        } else {
            section.ids.join(", ")
        };
        writeln!(output, "- {}: {}", section.name, ids).unwrap();
    }
    writeln!(output).unwrap();
    writeln!(output, "Navigation:").unwrap();
    for hint in &report.navigation {
        writeln!(output, "- {}: `{}`", hint.intent, hint.command).unwrap();
    }
    writeln!(output).unwrap();
    writeln!(output, "Progressive use:").unwrap();
    for item in &report.escalation {
        writeln!(output, "- {item}").unwrap();
    }
    output
}

pub fn render_query(report: &QueryReport) -> String {
    let mut output = String::new();
    render_target_header(&mut output, &report.target, &report.handle, report.view);
    writeln!(output, "value:").unwrap();
    render_value_lines(&mut output, &report.value);
    if !report.query_hints.is_empty() {
        writeln!(output, "query_hints:").unwrap();
        for hint in &report.query_hints {
            writeln!(output, "- `{hint}`").unwrap();
        }
    }
    output
}

pub fn render_refs(report: &RefsReport) -> String {
    let mut output = String::new();
    render_target_header(&mut output, &report.target, &report.handle, report.view);
    if report.outgoing.is_empty() {
        writeln!(output, "refs: <none>").unwrap();
    } else {
        writeln!(output, "refs:").unwrap();
        for edge in &report.outgoing {
            writeln!(
                output,
                "- {} -> {}: {}",
                edge.field,
                edge.kind,
                edge.ids.join(", ")
            )
            .unwrap();
        }
    }
    if !report.query_hints.is_empty() {
        writeln!(output, "query_hints:").unwrap();
        for hint in &report.query_hints {
            writeln!(output, "- `{hint}`").unwrap();
        }
    }
    output
}

fn sections(spec: &SkillSpec, view: View) -> Vec<SectionMap> {
    let mut maps = vec![
        SectionMap {
            name: "routes",
            role: "strategy choices",
            count: spec.routes.len(),
            ids: spec.routes.iter().map(|route| route.id.0.clone()).collect(),
        },
        SectionMap {
            name: "rules",
            role: "steering logic",
            count: spec.rules.len(),
            ids: spec.rules.iter().map(|rule| rule.id.0.clone()).collect(),
        },
        SectionMap {
            name: "states",
            role: "lifecycle phases",
            count: spec.states.len(),
            ids: spec.states.keys().cloned().collect(),
        },
        SectionMap {
            name: "dependencies",
            role: "static substrate",
            count: spec.dependencies.len(),
            ids: spec.dependencies.keys().cloned().collect(),
        },
        SectionMap {
            name: "commands",
            role: "executable templates",
            count: spec.commands.len(),
            ids: spec.commands.keys().cloned().collect(),
        },
        SectionMap {
            name: "recipes",
            role: "ordered procedures",
            count: spec.recipes.len(),
            ids: spec.recipes.keys().cloned().collect(),
        },
        SectionMap {
            name: "closures",
            role: "completion obligations",
            count: spec.closures.len(),
            ids: spec.closures.keys().cloned().collect(),
        },
        SectionMap {
            name: "tests",
            role: "behavior checks",
            count: spec.tests.len(),
            ids: spec.tests.iter().map(|test| test.name.clone()).collect(),
        },
    ];
    if view != View::Index {
        maps.extend([
            SectionMap {
                name: "trace",
                role: "decision evidence",
                count: usize::from(spec.trace.is_some()),
                ids: spec
                    .trace
                    .as_ref()
                    .map(|_| vec!["trace".to_owned()])
                    .unwrap_or_default(),
            },
            SectionMap {
                name: "proof",
                role: "verification metrics",
                count: usize::from(spec.proof.is_some()),
                ids: spec
                    .proof
                    .as_ref()
                    .map(|_| vec!["proof".to_owned()])
                    .unwrap_or_default(),
            },
            SectionMap {
                name: "imports",
                role: "lazy external context",
                count: spec.imports.len(),
                ids: spec.imports.keys().cloned().collect(),
            },
            SectionMap {
                name: "resources",
                role: "local reference material",
                count: spec.resources.len(),
                ids: spec.resources.keys().cloned().collect(),
            },
        ]);
    }
    maps
}

fn navigation(spec_path: &str) -> Vec<NavigationHint> {
    vec![
        NavigationHint {
            intent: "orient",
            command: format!("skillspec sensemake {spec_path} --view index"),
        },
        NavigationHint {
            intent: "task routing",
            command: format!("skillspec decide {spec_path} --input '<task>'"),
        },
        NavigationHint {
            intent: "inspect active rule",
            command: format!("skillspec query {spec_path} rule:<id> --view summary"),
        },
        NavigationHint {
            intent: "inspect outgoing refs",
            command: format!("skillspec refs {spec_path} rule:<id> --view summary"),
        },
        NavigationHint {
            intent: "inspect command readiness",
            command: format!("skillspec query {spec_path} command:<id>.requires"),
        },
        NavigationHint {
            intent: "check command dependencies",
            command: format!("skillspec deps check {spec_path} --command <id>"),
        },
        NavigationHint {
            intent: "inspect lifecycle",
            command: format!("skillspec query {spec_path} state:<id> --view summary"),
        },
        NavigationHint {
            intent: "prove completion",
            command: format!("skillspec trace align {spec_path} --decision-trace <run_dir>"),
        },
    ]
}

#[derive(Clone, Debug)]
struct ParsedHandle {
    kind: String,
    id: Option<String>,
    field_path: Vec<String>,
}

impl ParsedHandle {
    fn parse(handle: &str) -> Result<Self> {
        let handle = handle.trim();
        if handle.is_empty() {
            return Err(Error::InvalidInput {
                message: "query handle cannot be empty".to_owned(),
            });
        }
        let (base, field_path) = match handle.split_once('.') {
            Some((base, fields)) => (
                base,
                fields
                    .split('.')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect(),
            ),
            None => (handle, Vec::new()),
        };
        if let Some((kind, id)) = base.split_once(':') {
            if id.is_empty() {
                return Err(Error::InvalidInput {
                    message: format!("query handle {handle:?} is missing an id"),
                });
            }
            return Ok(Self {
                kind: kind.to_owned(),
                id: Some(id.to_owned()),
                field_path,
            });
        }
        Ok(Self {
            kind: base.to_owned(),
            id: None,
            field_path,
        })
    }
}

fn select_value(spec: &SkillSpec, parsed: &ParsedHandle, view: View) -> Result<Value> {
    let base_view = if parsed.field_path.is_empty() {
        view
    } else {
        View::Full
    };
    let mut value = match (parsed.kind.as_str(), parsed.id.as_deref()) {
        ("routes", None) => collection(
            spec.routes
                .iter()
                .map(|route| route.id.0.as_str())
                .collect(),
            spec.routes.iter().map(route_summary).collect(),
            &spec.routes,
            base_view,
        )?,
        ("rules", None) => collection(
            spec.rules.iter().map(|rule| rule.id.0.as_str()).collect(),
            spec.rules.iter().map(rule_summary).collect(),
            &spec.rules,
            base_view,
        )?,
        ("states", None) => map_collection(&spec.states, base_view)?,
        ("dependencies", None) => map_collection(&spec.dependencies, base_view)?,
        ("commands", None) => map_collection_summary(&spec.commands, base_view, command_summary)?,
        ("recipes", None) => map_collection(&spec.recipes, base_view)?,
        ("closures", None) => map_collection(&spec.closures, base_view)?,
        ("tests", None) => collection(
            spec.tests.iter().map(|test| test.name.as_str()).collect(),
            spec.tests
                .iter()
                .map(|test| json!({"name": test.name, "input": test.input}))
                .collect(),
            &spec.tests,
            base_view,
        )?,
        ("trace", None) => option_value(&spec.trace, base_view)?,
        ("proof", None) => option_value(&spec.proof, base_view)?,
        ("imports", None) => map_collection(&spec.imports, base_view)?,
        ("resources", None) => map_collection(&spec.resources, base_view)?,
        ("code", None) => map_collection(&spec.code, base_view)?,
        ("artifacts", None) => map_collection(&spec.artifacts, base_view)?,
        ("snippets", None) => map_collection(&spec.snippets, base_view)?,
        ("elicitations", None) => map_collection(&spec.elicitations, base_view)?,
        ("route", Some(id)) => item(
            id,
            spec.routes
                .iter()
                .find(|route| route.id.0 == id)
                .map(|route| (route_summary(route), route)),
            base_view,
            "route",
        )?,
        ("rule", Some(id)) => item(
            id,
            spec.rules
                .iter()
                .find(|rule| rule.id.0 == id)
                .map(|rule| (rule_summary(rule), rule)),
            base_view,
            "rule",
        )?,
        ("state", Some(id)) => map_item(id, &spec.states, base_view, "state")?,
        ("dependency", Some(id)) => map_item(id, &spec.dependencies, base_view, "dependency")?,
        ("command", Some(id)) => item(
            id,
            spec.commands
                .get(id)
                .map(|command| (command_summary(id, command), command)),
            base_view,
            "command",
        )?,
        ("recipe", Some(id)) => map_item(id, &spec.recipes, base_view, "recipe")?,
        ("closure", Some(id)) => map_item(id, &spec.closures, base_view, "closure")?,
        ("import", Some(id)) => map_item(id, &spec.imports, base_view, "import")?,
        ("resource", Some(id)) => map_item(id, &spec.resources, base_view, "resource")?,
        ("code", Some(id)) => map_item(id, &spec.code, base_view, "code")?,
        ("artifact", Some(id)) => map_item(id, &spec.artifacts, base_view, "artifact")?,
        ("snippet", Some(id)) => map_item(id, &spec.snippets, base_view, "snippet")?,
        ("elicitation", Some(id)) => map_item(id, &spec.elicitations, base_view, "elicitation")?,
        _ => {
            return Err(Error::InvalidInput {
                message: format!("unknown query handle {kind}", kind = parsed.kind),
            });
        }
    };
    if !parsed.field_path.is_empty() {
        value = project_field(value, &parsed.field_path, &parsed.kind)?;
    }
    Ok(value)
}

fn collection<T: Serialize>(
    ids: Vec<&str>,
    summaries: Vec<Value>,
    full: &T,
    view: View,
) -> Result<Value> {
    match view {
        View::Index => Ok(json!(ids)),
        View::Summary => Ok(Value::Array(summaries)),
        View::Full => Ok(serde_json::to_value(full)?),
    }
}

fn map_collection<T: Serialize>(map: &impl MapLike<T>, view: View) -> Result<Value> {
    match view {
        View::Index => Ok(json!(map.keys())),
        View::Summary => Ok(json!(map.keys())),
        View::Full => Ok(serde_json::to_value(map.as_value())?),
    }
}

fn map_collection_summary<T: Serialize>(
    map: &impl MapLike<T>,
    view: View,
    summarize: fn(&str, &T) -> Value,
) -> Result<Value> {
    match view {
        View::Index => Ok(json!(map.keys())),
        View::Summary => Ok(Value::Array(
            map.entries()
                .into_iter()
                .map(|(id, value)| summarize(id, value))
                .collect(),
        )),
        View::Full => Ok(serde_json::to_value(map.as_value())?),
    }
}

fn option_value<T: Serialize>(option: &Option<T>, view: View) -> Result<Value> {
    match (option, view) {
        (Some(value), View::Full) => Ok(serde_json::to_value(value)?),
        (Some(_), _) => Ok(json!(["present"])),
        (None, _) => Ok(json!([])),
    }
}

fn item<T: Serialize>(
    id: &str,
    item: Option<(Value, &T)>,
    view: View,
    kind: &str,
) -> Result<Value> {
    let Some((summary, full)) = item else {
        return Err(Error::InvalidInput {
            message: format!("unknown {kind} id {id:?}"),
        });
    };
    match view {
        View::Index => Ok(json!({
            "id": id,
            "fields": fields_for(full)?,
        })),
        View::Summary => Ok(summary),
        View::Full => Ok(serde_json::to_value(full)?),
    }
}

fn map_item<T: Serialize>(
    id: &str,
    map: &impl MapLike<T>,
    view: View,
    kind: &str,
) -> Result<Value> {
    item(
        id,
        map.get(id)
            .map(|value| (summary_for_value(id, value), value)),
        view,
        kind,
    )
}

trait MapLike<T: Serialize>: Serialize {
    fn keys(&self) -> Vec<String>;
    fn entries(&self) -> Vec<(&str, &T)>;
    fn get(&self, id: &str) -> Option<&T>;
    fn as_value(&self) -> &Self;
}

impl<T: Serialize> MapLike<T> for std::collections::BTreeMap<String, T> {
    fn keys(&self) -> Vec<String> {
        self.keys().cloned().collect()
    }

    fn entries(&self) -> Vec<(&str, &T)> {
        self.iter()
            .map(|(key, value)| (key.as_str(), value))
            .collect()
    }

    fn get(&self, id: &str) -> Option<&T> {
        std::collections::BTreeMap::get(self, id)
    }

    fn as_value(&self) -> &Self {
        self
    }
}

fn fields_for<T: Serialize>(value: &T) -> Result<Vec<String>> {
    match serde_json::to_value(value)? {
        Value::Object(object) => Ok(object.keys().cloned().collect()),
        _ => Ok(Vec::new()),
    }
}

fn summary_for_value<T: Serialize>(id: &str, value: &T) -> Value {
    match serde_json::to_value(value) {
        Ok(Value::Object(mut object)) => {
            object.insert("id".to_owned(), Value::String(id.to_owned()));
            Value::Object(object)
        }
        Ok(value) => json!({"id": id, "value": value}),
        Err(_) => json!({"id": id}),
    }
}

fn project_field(mut value: Value, path: &[String], kind: &str) -> Result<Value> {
    for part in path {
        match value {
            Value::Object(mut object) => {
                value = object.remove(part).ok_or_else(|| Error::InvalidInput {
                    message: format!("unknown field {part:?} on {kind}"),
                })?;
            }
            _ => {
                return Err(Error::InvalidInput {
                    message: format!("{kind} does not contain field {part:?}"),
                });
            }
        }
    }
    Ok(value)
}

fn route_summary(route: &crate::model::Route) -> Value {
    json!({
        "id": route.id.0,
        "label": route.label,
        "rank": route.rank,
        "description": route.description,
        "checks": route.checks,
        "handoff": route.handoff,
        "execution_plan": route.execution_plan,
    })
}

fn rule_summary(rule: &crate::model::Rule) -> Value {
    json!({
        "id": rule.id.0,
        "fields": non_empty_rule_fields(rule),
        "when": predicate_summary(&rule.when),
        "prefer": route_id(rule.prefer.as_ref()),
        "route_order": route_ids(&rule.route_order),
        "forbids": rule.forbid,
        "allows": rule.allow.keys().cloned().collect::<Vec<_>>(),
        "elicit": rule.elicit,
        "after_success": rule.after_success,
        "reason": rule.reason,
    })
}

fn command_summary(id: &str, command: &crate::model::CommandTemplate) -> Value {
    json!({
        "id": id,
        "description": command.description,
        "safety": command.safety,
        "requires": requires_summary(&command.requires),
        "parse_fields": command.parse.keys().cloned().collect::<Vec<_>>(),
        "success_when": command.success_when.keys().cloned().collect::<Vec<_>>(),
    })
}

fn predicate_summary(predicate: &Predicate) -> Value {
    json!({
        "user_says_any": predicate.user_says_any,
        "user_says_all_groups": predicate.user_says_all_groups,
        "task_recurrence_likely": predicate.task_recurrence_likely,
        "domain_object_task": predicate.domain_object_task,
        "interactive_prompt_likely": predicate.interactive_prompt_likely,
        "command_likely_long_running": predicate.command_likely_long_running,
    })
}

fn requires_summary(requires: &CommandRequires) -> Value {
    json!({
        "dependencies": requires.dependencies,
        "files": requires.files,
        "env": requires.env,
        "auth": requires.auth,
    })
}

fn route_id(route: Option<&RouteId>) -> Option<String> {
    route.map(|route| route.0.clone())
}

fn route_ids(routes: &[RouteId]) -> Vec<String> {
    routes.iter().map(|route| route.0.clone()).collect()
}

fn non_empty_rule_fields(rule: &crate::model::Rule) -> Vec<&'static str> {
    let mut fields = vec!["when"];
    if rule.prefer.is_some() {
        fields.push("prefer");
    }
    if !rule.route_order.is_empty() {
        fields.push("route_order");
    }
    if !rule.forbid.is_empty() {
        fields.push("forbid");
    }
    if !rule.allow.is_empty() {
        fields.push("allow");
    }
    if !rule.elicit.is_empty() {
        fields.push("elicit");
    }
    if !rule.after_success.is_empty() {
        fields.push("after_success");
    }
    if rule.reason.is_some() {
        fields.push("reason");
    }
    fields
}

fn outgoing_refs(spec: &SkillSpec, parsed: &ParsedHandle) -> Result<Vec<ReferenceEdge>> {
    let id = parsed.id.as_deref().unwrap_or_default();
    match parsed.kind.as_str() {
        "rule" => {
            let rule = spec
                .rules
                .iter()
                .find(|rule| rule.id.0 == id)
                .ok_or_else(|| Error::InvalidInput {
                    message: format!("unknown rule id {id:?}"),
                })?;
            let mut edges = Vec::new();
            if let Some(route) = &rule.prefer {
                edges.push(edge("prefer", "route", vec![route.0.clone()]));
            }
            if !rule.route_order.is_empty() {
                edges.push(edge("route_order", "route", route_ids(&rule.route_order)));
            }
            if !rule.forbid.is_empty() {
                edges.push(edge("forbid", "forbid", rule.forbid.clone()));
            }
            if !rule.elicit.is_empty() {
                edges.push(edge("elicit", "elicitation", rule.elicit.clone()));
            }
            if !rule.after_success.is_empty() {
                edges.push(edge(
                    "after_success",
                    "command_or_recipe_or_state",
                    rule.after_success.clone(),
                ));
            }
            Ok(edges)
        }
        "command" => {
            let command = spec.commands.get(id).ok_or_else(|| Error::InvalidInput {
                message: format!("unknown command id {id:?}"),
            })?;
            let mut edges = Vec::new();
            if !command.requires.dependencies.is_empty() {
                edges.push(edge(
                    "requires.dependencies",
                    "dependency",
                    command.requires.dependencies.clone(),
                ));
            }
            if !command.requires.files.is_empty() {
                edges.push(edge(
                    "requires.files",
                    "file",
                    command.requires.files.clone(),
                ));
            }
            if !command.requires.env.is_empty() {
                edges.push(edge("requires.env", "env", command.requires.env.clone()));
            }
            if !command.requires.auth.is_empty() {
                edges.push(edge("requires.auth", "auth", command.requires.auth.clone()));
            }
            Ok(edges)
        }
        "state" => {
            let state = spec.states.get(id).ok_or_else(|| Error::InvalidInput {
                message: format!("unknown state id {id:?}"),
            })?;
            let mut edges = Vec::new();
            for (field, value) in [
                ("next", &state.next),
                ("yes", &state.yes),
                ("no", &state.no),
            ] {
                if let Some(value) = value {
                    edges.push(edge(field, "state", vec![value.clone()]));
                }
            }
            Ok(edges)
        }
        "recipe" => {
            let recipe = spec.recipes.get(id).ok_or_else(|| Error::InvalidInput {
                message: format!("unknown recipe id {id:?}"),
            })?;
            let mut edges = Vec::new();
            if !recipe.requires.imports.is_empty() {
                edges.push(edge("requires.imports", "import", recipe.requires.imports.clone()));
            }
            if !recipe.requires.resources.is_empty() {
                edges.push(edge(
                    "requires.resources",
                    "resource",
                    recipe.requires.resources.clone(),
                ));
            }
            if !recipe.requires.dependencies.is_empty() {
                edges.push(edge(
                    "requires.dependencies",
                    "dependency",
                    recipe.requires.dependencies.clone(),
                ));
            }
            if !recipe.requires.artifacts.is_empty() {
                edges.push(edge(
                    "requires.artifacts",
                    "artifact",
                    recipe.requires.artifacts.clone(),
                ));
            }
            for step in &recipe.steps {
                if let Some(edge) = recipe_step_edge(step) {
                    edges.push(edge);
                }
            }
            Ok(edges)
        }
        "route" => {
            let route = spec
                .routes
                .iter()
                .find(|route| route.id.0 == id)
                .ok_or_else(|| Error::InvalidInput {
                    message: format!("unknown route id {id:?}"),
                })?;
            let mut edges = Vec::new();
            if !route.checks.is_empty() {
                edges.push(edge("checks", "check", route.checks.clone()));
            }
            if let Some(handoff) = &route.handoff {
                edges.push(edge(
                    "handoff.to_skill",
                    "skill",
                    vec![handoff.to_skill.clone()],
                ));
            }
            if let Some(plan) = &route.execution_plan {
                let owner_skills = plan
                    .phases
                    .iter()
                    .map(|phase| phase.owner_skill.clone())
                    .collect::<Vec<_>>();
                if !owner_skills.is_empty() {
                    edges.push(edge("execution_plan.owner_skill", "skill", owner_skills));
                }
                let phase_routes = plan
                    .phases
                    .iter()
                    .filter_map(|phase| phase.route.as_ref().map(|route| route.0.clone()))
                    .collect::<Vec<_>>();
                if !phase_routes.is_empty() {
                    edges.push(edge("execution_plan.route", "route", phase_routes));
                }
                let handoff_targets = plan
                    .phases
                    .iter()
                    .filter_map(|phase| phase.handoff.as_ref().map(|handoff| handoff.to_skill.clone()))
                    .collect::<Vec<_>>();
                if !handoff_targets.is_empty() {
                    edges.push(edge(
                        "execution_plan.handoff.to_skill",
                        "skill",
                        handoff_targets,
                    ));
                }
            }
            Ok(edges)
        }
        _ => Err(Error::InvalidInput {
            message: format!(
                "refs supports route:<id>, rule:<id>, state:<id>, command:<id>, and recipe:<id>; got {kind}",
                kind = parsed.kind
            ),
        }),
    }
}

fn edge(field: &str, kind: &str, ids: Vec<String>) -> ReferenceEdge {
    ReferenceEdge {
        field: field.to_owned(),
        kind: kind.to_owned(),
        ids,
    }
}

fn recipe_step_edge(step: &RecipeStep) -> Option<ReferenceEdge> {
    match step {
        RecipeStep::LoadImport(step) => Some(edge(
            "steps.load_import",
            "import",
            vec![step.load_import.clone()],
        )),
        RecipeStep::LoadResource(step) => Some(edge(
            "steps.load_resource",
            "resource",
            vec![step.load_resource.clone()],
        )),
        RecipeStep::RunCommand(step) => Some(edge(
            "steps.run_command",
            "command",
            vec![step.run_command.clone()],
        )),
        RecipeStep::RunCode(step) => {
            Some(edge("steps.run_code", "code", vec![step.run_code.clone()]))
        }
        RecipeStep::ProduceArtifact(step) => Some(edge(
            "steps.produce_artifact",
            "artifact",
            vec![step.produce_artifact.clone()],
        )),
        RecipeStep::ConsumeArtifact(step) => Some(edge(
            "steps.consume_artifact",
            "artifact",
            vec![step.consume_artifact.clone()],
        )),
        RecipeStep::Ask(step) => Some(edge("steps.ask", "elicitation", vec![step.ask.clone()])),
        RecipeStep::Branch(step) => {
            let mut ids = vec![step.branch.then.clone()];
            if let Some(otherwise) = &step.branch.otherwise {
                ids.push(otherwise.clone());
            }
            Some(edge("steps.branch", "state", ids))
        }
        RecipeStep::Note(_) => None,
    }
}

fn query_hints(path: &Path, parsed: &ParsedHandle) -> Vec<String> {
    let path = path.display();
    let Some(id) = parsed.id.as_deref() else {
        return vec![format!(
            "skillspec query {path} {} --view summary",
            parsed.kind
        )];
    };
    match parsed.kind.as_str() {
        "rule" => vec![
            format!("skillspec query {path} rule:{id}.forbid"),
            format!("skillspec query {path} rule:{id}.after_success"),
            format!("skillspec refs {path} rule:{id} --view summary"),
        ],
        "command" => vec![
            format!("skillspec query {path} command:{id}.requires"),
            format!("skillspec deps check {path} --command {id}"),
            format!("skillspec refs {path} command:{id} --view summary"),
        ],
        "state" => vec![
            format!("skillspec query {path} state:{id}.next"),
            format!("skillspec refs {path} state:{id} --view summary"),
        ],
        "route" => vec![format!("skillspec query {path} route:{id}.checks")],
        _ => Vec::new(),
    }
}

fn refs_hints(path: &Path, parsed: &ParsedHandle, outgoing: &[ReferenceEdge]) -> Vec<String> {
    let path = path.display();
    let mut hints = Vec::new();
    for edge in outgoing {
        for id in &edge.ids {
            let command = match edge.kind.as_str() {
                "route" => Some(format!("skillspec query {path} route:{id} --view summary")),
                "dependency" => Some(format!(
                    "skillspec query {path} dependency:{id} --view summary"
                )),
                "elicitation" => Some(format!(
                    "skillspec query {path} elicitation:{id} --view summary"
                )),
                "command" => Some(format!(
                    "skillspec query {path} command:{id} --view summary"
                )),
                "state" => Some(format!("skillspec query {path} state:{id} --view summary")),
                "import" => Some(format!("skillspec query {path} import:{id} --view summary")),
                "resource" => Some(format!(
                    "skillspec query {path} resource:{id} --view summary"
                )),
                "artifact" => Some(format!(
                    "skillspec query {path} artifact:{id} --view summary"
                )),
                "code" => Some(format!("skillspec query {path} code:{id} --view summary")),
                _ => None,
            };
            if let Some(command) = command {
                hints.push(command);
            }
        }
    }
    if hints.is_empty() {
        if let Some(id) = parsed.id.as_deref() {
            hints.push(format!(
                "skillspec query {path} {}:{id} --view full",
                parsed.kind
            ));
        }
    }
    hints
}

fn render_target_header(output: &mut String, target: &QueryTarget, handle: &str, view: View) {
    writeln!(output, "handle: {handle}").unwrap();
    writeln!(output, "target: {}", target_name(target)).unwrap();
    writeln!(output, "view: {}", view_name(view)).unwrap();
    if !target.field_path.is_empty() {
        writeln!(output, "field_path: {}", target.field_path.join(".")).unwrap();
    }
}

fn target_name(target: &QueryTarget) -> String {
    match &target.id {
        Some(id) => format!("{}:{}", target.kind, id),
        None => target.kind.clone(),
    }
}

fn view_name(view: View) -> &'static str {
    match view {
        View::Index => "index",
        View::Summary => "summary",
        View::Full => "full",
    }
}

fn render_value_lines(output: &mut String, value: &Value) {
    match value {
        Value::Array(values) if values.iter().all(Value::is_string) => {
            let values = values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                output,
                "- {}",
                if values.is_empty() { "<none>" } else { &values }
            )
            .unwrap();
        }
        Value::Object(object) => {
            for (key, value) in object {
                writeln!(output, "- {key}: {}", compact_value(value)).unwrap();
            }
        }
        _ => {
            writeln!(output, "- {}", compact_value(value)).unwrap();
        }
    }
}

fn compact_value(value: &Value) -> String {
    match value {
        Value::Null => "<none>".to_owned(),
        Value::String(value) => value.clone(),
        Value::Array(values) => {
            if values.is_empty() {
                "<none>".to_owned()
            } else if values.iter().all(Value::is_string) {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                serde_json::to_string(value).unwrap_or_else(|_| "<unrenderable>".to_owned())
            }
        }
        Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "<unrenderable>".to_owned())
        }
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
    }
}
