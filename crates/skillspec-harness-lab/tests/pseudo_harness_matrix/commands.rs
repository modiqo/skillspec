use super::fixture::PseudoHarnessFixture;
use serde_json::Value;
use skillspec_harness_lab::{assert_success, json_stdout};
use std::path::Path;
use std::process::Output;

pub fn install_router_json(fixture: &PseudoHarnessFixture) -> Value {
    let output = install_router(fixture);
    assert_success(&output);
    json_stdout(&output)
}

pub fn guard_hook_json(fixture: &PseudoHarnessFixture) -> Value {
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

pub fn guard_json(fixture: &PseudoHarnessFixture) -> Value {
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

pub fn route_json(fixture: &PseudoHarnessFixture, query: &str) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("route")
        .arg("--index")
        .arg(&fixture.index)
        .arg("--query")
        .arg(query)
        .arg("--current-harness")
        .arg("claude-local")
        .arg("--current-root")
        .arg(fixture.lab.claude_root())
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn file_contains(path: impl AsRef<Path>, needle: &str) -> bool {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .contains(needle)
}

fn install_router(fixture: &PseudoHarnessFixture) -> Output {
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
