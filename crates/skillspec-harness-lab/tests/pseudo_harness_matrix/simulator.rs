use super::commands::{file_contains, guard_hook_json, route_json};
use super::fixture::PseudoHarnessFixture;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SimulatedPrompt {
    pub events: Vec<Value>,
    pub route: Value,
    pub loaded_skill: Option<LoadedSkill>,
    pub catalog: Catalog,
}

#[derive(Clone, Debug)]
pub struct LoadedSkill {
    pub name: String,
    pub path: PathBuf,
    pub trampoline: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub implicit: Vec<String>,
    pub manual_only: Vec<String>,
}

pub fn simulate_prompt(fixture: &PseudoHarnessFixture, query: &str) -> SimulatedPrompt {
    let mut events = Vec::new();
    events.push(json!({
        "event": "lab_started",
        "home": fixture.lab.normalize_path(fixture.lab.home()),
    }));
    events.push(json!({
        "event": "roots_detected",
        "roots": normalized_roots(fixture),
    }));

    let hook = guard_hook_json(fixture);
    let hook_context = hook.to_string();
    events.push(json!({
        "event": "hook_invoked",
        "first_hop_ready": hook_context.contains("first_hop_ready=true"),
    }));

    let catalog = catalog(fixture);
    events.push(json!({
        "event": "catalog_built",
        "implicit": catalog.implicit,
        "manual_only": catalog.manual_only,
    }));

    let route = route_json(fixture, query);
    events.push(json!({
        "event": "route_decision",
        "decision": route["decision"],
        "selected": route["selected"]["name"],
        "reason": route["decision_reason"],
    }));

    let loaded_skill = loaded_skill(fixture, &route);
    events.push(json!({
        "event": "domain_skill_loaded",
        "loaded": loaded_skill.is_some(),
        "name": loaded_skill.as_ref().map(|skill| skill.name.as_str()),
    }));
    if let Some(skill) = &loaded_skill {
        events.push(json!({
            "event": "trampoline_checked",
            "name": skill.name,
            "trampoline": skill.trampoline,
            "path": fixture.lab.normalize_path(&skill.path),
        }));
    }

    SimulatedPrompt {
        events,
        route,
        loaded_skill,
        catalog,
    }
}

pub fn event_position(events: &[Value], event_name: &str) -> usize {
    events
        .iter()
        .position(|event| event["event"] == event_name)
        .unwrap_or_else(|| panic!("missing simulator event: {event_name}"))
}

pub fn event_bool(events: &[Value], event_name: &str, key: &str) -> bool {
    events
        .iter()
        .find(|event| event["event"] == event_name)
        .and_then(|event| event[key].as_bool())
        .unwrap_or(false)
}

fn normalized_roots(fixture: &PseudoHarnessFixture) -> Vec<String> {
    [
        fixture.lab.agents_root(),
        fixture.lab.codex_root(),
        fixture.lab.claude_root(),
    ]
    .iter()
    .map(|root| fixture.lab.normalize_path(root))
    .collect()
}

fn catalog(fixture: &PseudoHarnessFixture) -> Catalog {
    let mut catalog = Catalog::default();
    for root in [
        fixture.lab.agents_root(),
        fixture.lab.codex_root(),
        fixture.lab.claude_root(),
    ] {
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if !path.join("SKILL.md").is_file() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if is_manual_only(&path) {
                catalog.manual_only.push(name);
            } else {
                catalog.implicit.push(name);
            }
        }
    }
    catalog.implicit.sort();
    catalog.manual_only.sort();
    catalog
}

fn is_manual_only(skill_dir: &Path) -> bool {
    file_contains(skill_dir.join("SKILL.md"), "disable-model-invocation: true")
        || file_contains(
            skill_dir.join("agents/openai.yaml"),
            "allow_implicit_invocation: false",
        )
}

fn loaded_skill(fixture: &PseudoHarnessFixture, route: &Value) -> Option<LoadedSkill> {
    if route["decision"] != "use_skill" {
        return None;
    }
    let name = route["selected"]["name"].as_str()?;
    if name == "skill-router" {
        return None;
    }
    let path = route["selected"]["path"]
        .as_str()
        .map(PathBuf::from)
        .or_else(|| find_skill(fixture, name))?;
    let skill_dir = if path.file_name().and_then(|value| value.to_str()) == Some("SKILL.md") {
        path.parent().unwrap_or(&path)
    } else {
        &path
    };
    let trampoline = skill_dir.join("skill.spec.yml").is_file()
        && file_contains(skill_dir.join("SKILL.md"), "SkillSpec");
    Some(LoadedSkill {
        name: name.to_owned(),
        path,
        trampoline,
    })
}

fn find_skill(fixture: &PseudoHarnessFixture, name: &str) -> Option<PathBuf> {
    [
        fixture.lab.agents_root(),
        fixture.lab.codex_root(),
        fixture.lab.claude_root(),
    ]
    .iter()
    .map(|root| root.join(name))
    .find(|path| path.join("SKILL.md").is_file())
}
