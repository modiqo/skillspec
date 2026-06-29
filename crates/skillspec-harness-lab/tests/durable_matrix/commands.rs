use super::fixture::DurableFixture;
use serde_json::Value;
use skillspec_harness_lab::{assert_failure, assert_success, json_stdout, stderr};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Output;

pub fn install_agents_json(fixture: &DurableFixture) -> Value {
    let output = durable_command(fixture, Some(&fixture.fake_rote_path))
        .arg("install")
        .arg(&fixture.source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn install_agents_codex_json(fixture: &DurableFixture) -> Value {
    let output = durable_command(fixture, Some(&fixture.fake_rote_path))
        .arg("install")
        .arg(&fixture.source)
        .arg("--target")
        .arg("agents")
        .arg("--target")
        .arg("codex")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn install_agents_missing_rote(fixture: &DurableFixture, dry_run: bool) -> Output {
    let mut command = direct_durable_command(fixture, &fixture.no_rote_path);
    command
        .arg("install")
        .arg(&fixture.source)
        .arg("--target")
        .arg("agents")
        .arg("--json");
    if dry_run {
        command.arg("--dry-run");
    }
    command.output().unwrap()
}

pub fn update_json(fixture: &DurableFixture) -> Value {
    let output = durable_command(fixture, Some(&fixture.fake_rote_path))
        .arg("update")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn update_output(fixture: &DurableFixture) -> Output {
    durable_command(fixture, Some(&fixture.fake_rote_path))
        .arg("update")
        .arg("--json")
        .output()
        .unwrap()
}

pub fn delete_json(fixture: &DurableFixture) -> Value {
    let output = delete_output(fixture);
    assert_success(&output);
    json_stdout(&output)
}

pub fn delete_output(fixture: &DurableFixture) -> Output {
    durable_command(fixture, None)
        .arg("delete")
        .arg("--json")
        .output()
        .unwrap()
}

pub fn disable_json(fixture: &DurableFixture) -> Value {
    let output = durable_command(fixture, None)
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn enable_json(fixture: &DurableFixture) -> Value {
    let output = enable_output_with_path(fixture, Some(&fixture.fake_rote_path));
    assert_success(&output);
    json_stdout(&output)
}

pub fn enable_missing_rote(fixture: &DurableFixture) -> Output {
    direct_durable_command(fixture, &fixture.no_rote_path)
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap()
}

pub fn install_router_json(fixture: &DurableFixture) -> Value {
    let index = fixture
        .lab
        .skillspec_home()
        .join("router/skill-index.sqlite");
    let output = fixture
        .lab
        .command_in_project()
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(fixture.lab.agents_root())
        .arg("--index")
        .arg(index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn router_status_json(fixture: &DurableFixture) -> Value {
    let index = fixture
        .lab
        .skillspec_home()
        .join("router/skill-index.sqlite");
    let output = fixture
        .lab
        .command_in_project()
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(fixture.lab.agents_root())
        .arg("--index")
        .arg(index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn assert_failed_with(output: &Output, text: &str) {
    assert_failure(output);
    assert!(
        stderr(output).contains(text),
        "expected stderr to contain {text:?}, got:\n{}",
        stderr(output)
    );
}

pub fn file_contains(path: impl AsRef<std::path::Path>, needle: &str) -> bool {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .contains(needle)
}

fn durable_command(fixture: &DurableFixture, path: Option<&OsStr>) -> std::process::Command {
    let mut command = fixture.lab.command_in_project();
    command.arg("durable-executor");
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command
}

fn enable_output_with_path(fixture: &DurableFixture, path: Option<&OsStr>) -> Output {
    durable_command(fixture, path)
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap()
}

fn direct_durable_command(fixture: &DurableFixture, path: &OsStr) -> std::process::Command {
    let binary = direct_skillspec_binary(fixture);
    let mut command = std::process::Command::new(binary);
    command
        .current_dir(fixture.lab.project())
        .env("HOME", fixture.lab.home())
        .env("SKILLSPEC_HOME", fixture.lab.skillspec_home())
        .env("PATH", path)
        .arg("durable-executor");
    command
}

fn direct_skillspec_binary(fixture: &DurableFixture) -> PathBuf {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("harness lab crate lives under crates/")
        .to_path_buf();
    let binary = repo_root
        .join("target")
        .join("debug")
        .join(format!("skillspec{}", std::env::consts::EXE_SUFFIX));
    if !binary.is_file() {
        let output = fixture
            .lab
            .command_in_project()
            .arg("--version")
            .output()
            .unwrap();
        assert_success(&output);
    }
    assert!(
        binary.is_file(),
        "expected built skillspec binary at {}",
        binary.display()
    );
    binary
}
