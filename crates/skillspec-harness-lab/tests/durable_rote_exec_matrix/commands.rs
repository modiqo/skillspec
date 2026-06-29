use super::fixture::{latest_run_dir, RoteExecFixture};
use serde_json::Value;
use skillspec_harness_lab::{assert_success, json_stdout, stdout};
use std::path::{Path, PathBuf};
use std::process::Output;

pub fn plan_json(fixture: &RoteExecFixture, trace_dir: &Path) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("plan")
        .arg(&fixture.spec)
        .arg("--input")
        .arg(fixture.input)
        .arg("--trace-dir")
        .arg(trace_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn act_json(fixture: &RoteExecFixture, run_dir: &Path) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("act")
        .arg(&fixture.spec)
        .arg("--input")
        .arg(fixture.input)
        .arg("--run")
        .arg(run_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn decide_run_dir(fixture: &RoteExecFixture, trace_dir: &Path) -> PathBuf {
    let output = fixture
        .lab
        .command_in_project()
        .arg("decide")
        .arg(&fixture.spec)
        .arg("--input")
        .arg(fixture.input)
        .arg("--trace-dir")
        .arg(trace_dir)
        .output()
        .unwrap();
    assert_success(&output);
    latest_run_dir(trace_dir)
}

pub fn align_json(fixture: &RoteExecFixture, run_dir: &Path, execution_trace: &Path) -> Value {
    let output = fixture
        .lab
        .command_in_project()
        .arg("trace")
        .arg("align")
        .arg(&fixture.spec)
        .arg("--decision-trace")
        .arg(run_dir)
        .arg("--execution-trace")
        .arg(execution_trace)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn command_stdout(output: &Output) -> String {
    stdout(output)
}
