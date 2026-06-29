use super::commands::{align_json, command_stdout, decide_run_dir};
use super::contract::write_execution_ledger;
use super::fixture::{rote_exec_fixture, PROOF_MARKER};
use skillspec_harness_lab::{assert_success, HarnessLabReportBuilder};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_LOCAL_ROTE: &str = "/Users/chetanconikee/.local/bin/rote";

pub fn copied_local_rote_runs_one_shot_process(report: &mut HarnessLabReportBuilder) {
    let fixture = rote_exec_fixture("durable-rote-live");
    let local_rote = local_rote_source();
    let copied_rote_dir = copy_local_rote_into_lab(&fixture.lab, &local_rote);
    let copied_rote = copied_rote_dir.join("rote");
    let source_rote_home = live_rote_home_source();
    assert!(
        source_rote_home.is_dir(),
        "live rote source home does not exist: {}; set SKILLSPEC_LIVE_ROTE_HOME",
        source_rote_home.display()
    );
    let copied_rote_home = copy_rote_home_into_lab(&fixture.lab, &source_rote_home);
    assert!(copied_rote_home.starts_with(fixture.lab.root()));
    let copied_workspaces_absent_before_init = !copied_rote_home.join("rote/workspaces").exists();
    assert!(copied_workspaces_absent_before_init);

    let version = run_rote(
        &copied_rote,
        &copied_rote_dir,
        &copied_rote_home,
        fixture.lab.home(),
        fixture.lab.root(),
    )
    .arg("--version")
    .output()
    .unwrap();
    assert_success(&version);
    let version_text = command_stdout(&version);
    assert!(version_text.starts_with("rote "));

    let workspace = unique_workspace_name();
    let init = run_rote(
        &copied_rote,
        &copied_rote_dir,
        &copied_rote_home,
        fixture.lab.home(),
        fixture.lab.root(),
    )
    .arg("init")
    .arg(&workspace)
    .arg("--seq")
    .arg("--force")
    .output()
    .unwrap();
    assert_success(&init);

    let workspace_dir = copied_rote_home.join("rote/workspaces").join(&workspace);
    assert!(
        workspace_dir.is_dir(),
        "expected rote workspace at {}",
        workspace_dir.display()
    );
    let workspace_sandbox = run_rote(
        &copied_rote,
        &copied_rote_dir,
        &copied_rote_home,
        fixture.lab.home(),
        fixture.lab.root(),
    )
    .arg("workspace")
    .arg("sandbox")
    .arg(&workspace)
    .arg("off")
    .output()
    .unwrap();
    assert_success(&workspace_sandbox);

    let exec = run_rote(
        &copied_rote,
        &copied_rote_dir,
        &copied_rote_home,
        fixture.lab.home(),
        &workspace_dir,
    )
    .arg("exec")
    .arg("--")
    .arg("printf")
    .arg(PROOF_MARKER)
    .output()
    .unwrap();
    assert_success(&exec);
    let exec_text = format!(
        "{}{}",
        String::from_utf8_lossy(&exec.stdout),
        String::from_utf8_lossy(&exec.stderr)
    );
    assert!(
        exec_text.contains(PROOF_MARKER),
        "expected rote exec output to contain marker, got:\n{exec_text}"
    );

    let trace_dir = fixture.lab.root().join("traces");
    let run_dir = decide_run_dir(&fixture, &trace_dir);
    let execution_trace = fixture.lab.root().join("execution.jsonl");
    write_execution_ledger(&fixture, &execution_trace, &workspace);
    let align = align_json(&fixture, &run_dir, &execution_trace);
    assert_eq!(align["status"], "pass");
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("copied_local_rote_runs_one_shot_process");
    case.claim_pass("local_rote.source_exists", true, local_rote.is_file());
    case.claim_pass("local_rote.copied", true, copied_rote.is_file());
    case.claim_pass("rote_home.source_exists", true, source_rote_home.is_dir());
    case.claim_pass("rote_home.copied", true, copied_rote_home.is_dir());
    case.claim_pass(
        "rote_home.no_preexisting_workspaces_copied",
        true,
        copied_workspaces_absent_before_init,
    );
    case.claim_pass(
        "rote_home.sandboxed",
        true,
        workspace_dir.starts_with(fixture.lab.root()),
    );
    case.claim_pass(
        "local_rote.version_prefix",
        true,
        version_text.starts_with("rote "),
    );
    case.claim_pass("rote.workspace_created", true, workspace_dir.is_dir());
    case.claim_pass(
        "rote.workspace_sandbox_off",
        true,
        workspace_sandbox.status.success(),
    );
    case.claim_pass("rote.exec_exit_success", true, exec.status.success());
    case.claim_pass(
        "rote.exec_contains_marker",
        true,
        exec_text.contains(PROOF_MARKER),
    );
    case.claim_pass("skillspec.align_status", "pass", &align["status"]);
    case.claim_pass(
        "skillspec.execution_alignment",
        "pass",
        &align["summary"]["execution_alignment"],
    );
    case.claim_pass(
        "skillspec.unproven_obligations",
        0,
        &align["summary"]["execution_obligations"]["unproven"],
    );
    case.claim_pass(
        "skillspec.rote_exec_proof_row",
        true,
        align["proof_rows"].as_array().unwrap().iter().any(|row| {
            row["requirement"] == "CLI work must be captured through rote exec"
                && row["status"] == "satisfied"
        }),
    );
    case.claim_pass(
        "rote_shell.skill_present",
        true,
        fixture
            .lab
            .agents_root()
            .join("rote-shell/SKILL.md")
            .is_file(),
    );
    case.claim_pass("proof.marker", PROOF_MARKER, PROOF_MARKER);
    case.finish();
}

fn local_rote_source() -> PathBuf {
    std::env::var_os("SKILLSPEC_LIVE_ROTE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LOCAL_ROTE))
}

fn live_rote_home_source() -> PathBuf {
    std::env::var_os("SKILLSPEC_LIVE_ROTE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .expect("HOME or SKILLSPEC_LIVE_ROTE_HOME must be set for live rote proof");
            home.join(".rote")
        })
}

fn copy_local_rote_into_lab(lab: &skillspec_harness_lab::HarnessLab, source: &Path) -> PathBuf {
    assert!(
        source.is_file(),
        "local rote binary is missing: {}; set SKILLSPEC_LIVE_ROTE",
        source.display()
    );
    let bin = lab.root().join("live-rote-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let destination = bin.join("rote");
    std::fs::copy(source, &destination).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&destination).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&destination, permissions).unwrap();
    }
    bin
}

fn copy_rote_home_into_lab(lab: &skillspec_harness_lab::HarnessLab, source: &Path) -> PathBuf {
    let destination = lab.root().join("live-rote-home");
    copy_rote_tree(source, &destination, source);
    destination
}

fn copy_rote_tree(source: &Path, destination: &Path, root: &Path) {
    if should_skip_rote_entry(source, root) {
        return;
    }
    let metadata = std::fs::symlink_metadata(source).unwrap();
    if metadata.is_dir() {
        std::fs::create_dir_all(destination).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let child_source = entry.path();
            let child_destination = destination.join(entry.file_name());
            copy_rote_tree(&child_source, &child_destination, root);
        }
    } else if metadata.file_type().is_symlink() {
        copy_symlink(source, destination);
    } else if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::copy(source, destination).unwrap();
    }
}

fn should_skip_rote_entry(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .any(|component| component.as_os_str() == "workspaces")
}

#[cfg(unix)]
fn copy_symlink(source: &Path, destination: &Path) {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let target = std::fs::read_link(source).unwrap();
    std::os::unix::fs::symlink(target, destination).unwrap();
}

#[cfg(not(unix))]
fn copy_symlink(source: &Path, destination: &Path) {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let target = std::fs::read_link(source).unwrap();
    std::fs::copy(target, destination).unwrap();
}

fn run_rote<'a>(
    copied_rote: &'a Path,
    copied_rote_dir: &'a Path,
    rote_home: &'a Path,
    fallback_home: &'a Path,
    current_dir: &'a Path,
) -> Command {
    let mut command = Command::new(copied_rote);
    command
        .current_dir(current_dir)
        .env("HOME", fallback_home)
        .env("ROTE_HOME", rote_home)
        .env("PATH", live_path(copied_rote_dir));
    command
}

fn live_path(copied_rote_dir: &Path) -> OsString {
    let mut paths = vec![copied_rote_dir.to_path_buf()];
    paths.extend(
        [
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
            "/opt/homebrew/bin",
        ]
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| path.is_dir()),
    );
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).unwrap()
}

fn unique_workspace_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("skillspec-durable-rote-exec-{}-{nanos}", std::process::id())
}
