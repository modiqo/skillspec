use super::fixture::RouterFixture;
use serde_json::Value;
use skillspec_harness_lab::{assert_success, json_stdout, HarnessLab};
use std::path::Path;
use std::process::Output;

pub fn install_router(fixture: &RouterFixture) -> Output {
    fixture
        .lab
        .command_in_project()
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(fixture.lab.agents_root())
        .arg(fixture.lab.codex_root())
        .arg(fixture.lab.claude_root())
        .arg("--index")
        .arg(&fixture.index)
        .arg("--manifest")
        .arg(&fixture.manifest)
        .arg("--json")
        .output()
        .unwrap()
}

pub fn install_router_json(fixture: &RouterFixture) -> Value {
    let output = install_router(fixture);
    assert_success(&output);
    json_stdout(&output)
}

pub fn guard_json(fixture: &RouterFixture) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("router")
        .arg("guard")
        .arg("--config")
        .arg(&fixture.config)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn guard_hook_json(fixture: &RouterFixture) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("router")
        .arg("guard")
        .arg("--config")
        .arg(&fixture.config)
        .arg("--hook")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn index_status_json(fixture: &RouterFixture) -> Value {
    let output = router_index_command(fixture, "status").output().unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn disable_router_json(fixture: &RouterFixture) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("router")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn enable_router_json(fixture: &RouterFixture) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("router")
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn uninstall_router_json(fixture: &RouterFixture) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("router")
        .arg("uninstall")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn route_json(lab: &HarnessLab, index: &Path, query: &str) -> Value {
    let output = lab
        .command_in_project()
        .arg("route")
        .arg("--index")
        .arg(index)
        .arg("--query")
        .arg(query)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

fn router_index_command(fixture: &RouterFixture, subcommand: &str) -> std::process::Command {
    let mut command = fixture.lab.command_in_project();
    command
        .arg("router")
        .arg("index")
        .arg(subcommand)
        .arg("--roots")
        .arg(fixture.lab.agents_root())
        .arg(fixture.lab.codex_root())
        .arg(fixture.lab.claude_root())
        .arg("--index")
        .arg(&fixture.index)
        .arg("--visibility-manifest")
        .arg(&fixture.manifest)
        .arg("--json");
    command
}

pub fn has_hook_command(path: &Path, needle: &str) -> bool {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let value = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
    value
        .get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|hooks| hooks.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|group| group.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|hook| hook.get("command").and_then(Value::as_str))
        .any(|command| command.contains(needle))
}

pub fn file_contains(path: &Path, needle: &str) -> bool {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .contains(needle)
}
