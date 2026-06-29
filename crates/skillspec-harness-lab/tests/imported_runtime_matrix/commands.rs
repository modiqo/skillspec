use super::fixture::ReviewedImportFixture;
use skillspec_harness_lab::{assert_success, json_stdout};
use std::path::{Path, PathBuf};
use std::process::Output;

pub fn compile_loader(fixture: &ReviewedImportFixture) -> Output {
    fixture
        .lab
        .command()
        .arg("compile")
        .arg(&fixture.spec_path)
        .arg("--target")
        .arg("codex-skill")
        .output()
        .unwrap()
}

pub fn write_compiled_loader(fixture: &ReviewedImportFixture) -> String {
    let compile = compile_loader(fixture);
    assert_success(&compile);
    let loader = String::from_utf8_lossy(&compile.stdout).into_owned();
    fixture
        .lab
        .write_file(&fixture.package_dir.join("SKILL.md"), &loader);
    loader
}

pub fn install_skill_all_detected(fixture: &ReviewedImportFixture, name: &str) -> Output {
    fixture
        .lab
        .command_in_project()
        .arg("install")
        .arg("skill")
        .arg(&fixture.package_dir)
        .arg("--name")
        .arg(name)
        .arg("--all-detected")
        .output()
        .unwrap()
}

pub fn install_skill_target(
    fixture: &ReviewedImportFixture,
    name: &str,
    target: &str,
    retire_existing: bool,
) -> Output {
    let mut command = fixture.lab.command_in_project();
    command
        .arg("install")
        .arg("skill")
        .arg(&fixture.package_dir)
        .arg("--name")
        .arg(name)
        .arg("--target")
        .arg(target);
    if retire_existing {
        command.arg("--retire-existing");
    }
    command.output().unwrap()
}

pub fn plan_json(
    fixture: &ReviewedImportFixture,
    input: &str,
    trace_dir: &Path,
) -> serde_json::Value {
    let output = fixture
        .lab
        .command()
        .arg("plan")
        .arg(&fixture.spec_path)
        .arg("--input")
        .arg(input)
        .arg("--trace-dir")
        .arg(trace_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn decide_json(
    fixture: &ReviewedImportFixture,
    input: &str,
    trace_dir: &Path,
) -> serde_json::Value {
    let output = fixture
        .lab
        .command()
        .arg("decide")
        .arg(&fixture.spec_path)
        .arg("--input")
        .arg(input)
        .arg("--trace-dir")
        .arg(trace_dir)
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn act_json(fixture: &ReviewedImportFixture, input: &str, run_dir: &Path) -> serde_json::Value {
    let output = fixture
        .lab
        .command()
        .arg("act")
        .arg(&fixture.spec_path)
        .arg("--input")
        .arg(input)
        .arg("--run")
        .arg(run_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn align_json(fixture: &ReviewedImportFixture, run_dir: &Path, execution: bool) -> Output {
    let mut command = fixture.lab.command();
    command
        .arg("trace")
        .arg("align")
        .arg(&fixture.spec_path)
        .arg("--decision-trace")
        .arg(run_dir);
    if execution {
        command
            .arg("--execution-trace")
            .arg(run_dir.join("execution.jsonl"));
    }
    command.arg("--json");
    command.output().unwrap()
}

pub fn latest_run_dir(trace_dir: &Path) -> PathBuf {
    let mut dirs = std::fs::read_dir(trace_dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .collect::<Vec<_>>();
    dirs.sort();
    dirs.pop().expect("expected trace run directory")
}

pub fn progress_batch_summary(
    fixture: &ReviewedImportFixture,
    run_dir: &Path,
    events: &Path,
) -> Output {
    fixture
        .lab
        .command()
        .arg("progress")
        .arg("batch")
        .arg(run_dir)
        .arg("--file")
        .arg(events)
        .arg("--checkpoint")
        .arg("runtime proof")
        .arg("--summary")
        .output()
        .unwrap()
}

pub fn progress_stats(fixture: &ReviewedImportFixture, run_dir: &Path) -> Output {
    fixture
        .lab
        .command()
        .arg("progress")
        .arg("stats")
        .arg(run_dir)
        .arg("--agent-visible-tokens")
        .arg("120")
        .arg("--artifact-tokens-preserved")
        .arg("2400")
        .arg("--avoided-tokens")
        .arg("2280")
        .arg("--metrics-source")
        .arg("estimated")
        .output()
        .unwrap()
}

pub fn progress_final_response(fixture: &ReviewedImportFixture, run_dir: &Path) -> Output {
    fixture
        .lab
        .command()
        .arg("progress")
        .arg("final-response")
        .arg(run_dir)
        .arg("--phase")
        .arg("install")
        .arg("--requirement")
        .arg("installed_loader")
        .arg("--result")
        .arg("--evidence")
        .arg("--alignment")
        .arg("--token-savings")
        .output()
        .unwrap()
}
