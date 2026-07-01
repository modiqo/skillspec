use skillspec_harness_lab::HarnessLab;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub struct DurableFixture {
    pub lab: HarnessLab,
    pub source: PathBuf,
    pub fake_rote_path: OsString,
    pub no_rote_path: OsString,
}

pub fn durable_fixture(name: &str, suffix: &str) -> DurableFixture {
    let lab = HarnessLab::new(name);
    let source = lab.root().join("durable-source");
    write_durable_source(&lab, &source, suffix);
    let fake_rote_path = write_fake_rote(&lab);
    let no_rote_path = path_without_rote(&lab);
    DurableFixture {
        lab,
        source,
        fake_rote_path,
        no_rote_path,
    }
}

pub fn write_durable_source(lab: &HarnessLab, path: &Path, description_suffix: &str) {
    lab.write_file(
        &path.join("SKILL.md"),
        &format!(
            r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment. {description_suffix}
---
# Durable Executor
"#,
        ),
    );
    lab.write_file(
        &path.join("skill.spec.yml"),
        r#"schema: skillspec/v0
id: durable.executor
title: Durable Executor
description: Durable executor fixture.
routes:
  - id: durable
    label: Durable
"#,
    );
}

pub fn write_plain_skill(lab: &HarnessLab, root: &Path, name: &str) {
    let skill = format!(
        r#"---
name: {name}
description: Use {name} for controlled durable harness lab routing.
---
# {name}
"#,
    );
    lab.write_skill(root, name, &skill, None);
}

fn write_fake_rote(lab: &HarnessLab) -> OsString {
    let bin_dir = lab.root().join("fake-rote-bin");
    #[cfg(unix)]
    write_executable(&bin_dir.join("rote"), "#!/bin/sh\nexit 0\n");
    #[cfg(windows)]
    lab.write_file(&bin_dir.join("rote.cmd"), "@echo off\r\nexit /B 0\r\n");

    let mut paths = vec![bin_dir];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).unwrap()
}

fn path_without_rote(lab: &HarnessLab) -> OsString {
    let shim_dir = lab.root().join("toolchain-without-rote");
    write_toolchain_shims(&shim_dir);
    let mut paths = vec![shim_dir];
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
        .filter(|path| path.is_dir() && !has_executable_rote(path)),
    );
    std::env::join_paths(paths).unwrap()
}

fn write_toolchain_shims(shim_dir: &Path) {
    std::fs::create_dir_all(shim_dir).unwrap();
    for command in ["cargo", "rustc", "rustup"] {
        let Some(source) = find_on_current_path(command) else {
            continue;
        };
        let destination = shim_dir.join(command);
        if destination.exists() {
            continue;
        }
        link_or_copy(&source, &destination);
    }
}

fn find_on_current_path(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|directory| {
            command_candidates(command)
                .into_iter()
                .map(move |candidate| directory.join(candidate))
        })
        .find(|candidate| is_executable_file(candidate))
}

#[cfg(unix)]
fn link_or_copy(source: &Path, destination: &Path) {
    std::os::unix::fs::symlink(source, destination)
        .or_else(|_| std::fs::copy(source, destination).map(|_| ()))
        .unwrap();
}

#[cfg(not(unix))]
fn link_or_copy(source: &Path, destination: &Path) {
    std::fs::copy(source, destination).unwrap();
}

fn has_executable_rote(path: &Path) -> bool {
    command_candidates("rote")
        .into_iter()
        .map(|candidate| path.join(candidate))
        .any(|candidate| is_executable_file(&candidate))
}

fn command_candidates(command: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        vec![
            command.to_owned(),
            format!("{command}.exe"),
            format!("{command}.bat"),
            format!("{command}.cmd"),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![command.to_owned()]
    }
}

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;

    lab_write_parent(path);
    std::fs::write(path, content).unwrap();
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(unix)]
fn lab_write_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
}
