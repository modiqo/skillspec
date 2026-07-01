use crate::paths::repo_root;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub(crate) struct LabEnvironment<'a> {
    pub(crate) home: &'a Path,
    pub(crate) skillspec_home: &'a Path,
    pub(crate) xdg_config_home: &'a Path,
    pub(crate) xdg_cache_home: &'a Path,
    pub(crate) xdg_data_home: &'a Path,
}

pub(crate) fn skillspec_command(current_dir: &Path, env: LabEnvironment<'_>) -> Command {
    let mut command = if let Some(binary) = std::env::var_os("SKILLSPEC_BIN") {
        Command::new(binary)
    } else {
        let mut cargo = Command::new("cargo");
        cargo
            .arg("run")
            .arg("--locked")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(repo_root().join("Cargo.toml"))
            .arg("-p")
            .arg("skillspec")
            .arg("--");
        cargo
    };

    command
        .current_dir(current_dir)
        .env("HOME", env.home)
        .env("SKILLSPEC_HOME", env.skillspec_home)
        .env("XDG_CONFIG_HOME", env.xdg_config_home)
        .env("XDG_CACHE_HOME", env.xdg_cache_home)
        .env("XDG_DATA_HOME", env.xdg_data_home)
        .env("SKILLSPEC_HARNESS_LAB", OsStr::new("1"));
    command
}
