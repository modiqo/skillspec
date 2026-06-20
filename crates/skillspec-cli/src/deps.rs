use crate::error::{Error, Result};
use crate::model::{Dependency, DependencyKind, SkillSpec};
use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct DependencyCheckReport {
    pub ok: bool,
    pub dependencies: Vec<DependencyCheckResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DependencyCheckResult {
    pub id: String,
    pub kind: DependencyKind,
    pub status: DependencyStatus,
    pub check: DependencyCheckMethod,
    pub permission_required: bool,
    pub provisionable: bool,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyStatus {
    Present,
    Missing,
    Deferred,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyCheckMethod {
    PathLookup,
    FileExists,
    EnvPresent,
    HarnessRequired,
}

pub fn check(
    spec: &SkillSpec,
    spec_dir: &Path,
    command: Option<&str>,
) -> Result<DependencyCheckReport> {
    let dependency_ids = dependency_ids_for_command(spec, command)?;
    let dependencies = dependency_ids
        .iter()
        .filter_map(|id| spec.dependencies.get(id).map(|dependency| (id, dependency)))
        .map(|(id, dependency)| check_dependency(id, dependency, spec_dir))
        .collect::<Vec<_>>();
    let ok = dependencies
        .iter()
        .all(|dependency| dependency.status != DependencyStatus::Missing);

    Ok(DependencyCheckReport { ok, dependencies })
}

fn dependency_ids_for_command(spec: &SkillSpec, command: Option<&str>) -> Result<BTreeSet<String>> {
    let Some(command_id) = command else {
        return Ok(spec.dependencies.keys().cloned().collect());
    };
    let command = spec
        .commands
        .get(command_id)
        .ok_or_else(|| Error::UnknownReference {
            field: "deps.check.command",
            value: command_id.to_owned(),
        })?;

    Ok(command.requires.dependencies.iter().cloned().collect())
}

fn check_dependency(id: &str, dependency: &Dependency, spec_dir: &Path) -> DependencyCheckResult {
    let permission_required = dependency
        .permission
        .as_ref()
        .is_some_and(|permission| permission.required);
    let provisionable = dependency
        .provision
        .as_ref()
        .is_some_and(|provision| !provision.options.is_empty());

    let (status, check, message) = match dependency.kind {
        DependencyKind::Cli => check_cli(id, dependency),
        DependencyKind::File => check_file(id, dependency, spec_dir),
        DependencyKind::Env => check_env(id, dependency),
        DependencyKind::Package
        | DependencyKind::Service
        | DependencyKind::Adapter
        | DependencyKind::Browser => (
            DependencyStatus::Deferred,
            DependencyCheckMethod::HarnessRequired,
            "not locally checkable; requires harness-specific verification before use".to_owned(),
        ),
    };

    DependencyCheckResult {
        id: id.to_owned(),
        kind: dependency.kind.clone(),
        status,
        check,
        permission_required,
        provisionable,
        message,
    }
}

fn check_cli(
    id: &str,
    dependency: &Dependency,
) -> (DependencyStatus, DependencyCheckMethod, String) {
    let command = dependency
        .check
        .as_ref()
        .and_then(|check| check.command.as_deref())
        .or(dependency.command.as_deref())
        .unwrap_or(id);
    let program = command.split_whitespace().next().unwrap_or(command);

    if find_on_path(program).is_some() {
        (
            DependencyStatus::Present,
            DependencyCheckMethod::PathLookup,
            format!("{program} found on PATH"),
        )
    } else {
        (
            DependencyStatus::Missing,
            DependencyCheckMethod::PathLookup,
            format!("{program} not found on PATH"),
        )
    }
}

fn check_file(
    id: &str,
    dependency: &Dependency,
    spec_dir: &Path,
) -> (DependencyStatus, DependencyCheckMethod, String) {
    let path = dependency
        .check
        .as_ref()
        .and_then(|check| check.path.as_deref())
        .or(dependency.path.as_deref())
        .unwrap_or(id);
    let resolved_path = resolve_spec_path(spec_dir, path);

    if resolved_path.exists() {
        (
            DependencyStatus::Present,
            DependencyCheckMethod::FileExists,
            format!("{path} exists"),
        )
    } else {
        (
            DependencyStatus::Missing,
            DependencyCheckMethod::FileExists,
            format!("{path} does not exist"),
        )
    }
}

fn resolve_spec_path(spec_dir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        spec_dir.join(path)
    }
}

fn check_env(
    id: &str,
    dependency: &Dependency,
) -> (DependencyStatus, DependencyCheckMethod, String) {
    let env_name = dependency
        .check
        .as_ref()
        .and_then(|check| check.env.as_deref())
        .or(dependency.env.as_deref())
        .unwrap_or(id);

    if env::var_os(env_name).is_some() {
        (
            DependencyStatus::Present,
            DependencyCheckMethod::EnvPresent,
            format!("{env_name} is set"),
        )
    } else {
        (
            DependencyStatus::Missing,
            DependencyCheckMethod::EnvPresent,
            format!("{env_name} is not set"),
        )
    }
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn file_dependencies_resolve_relative_to_spec_directory() {
        let root = unique_temp_dir("skillspec-deps-relative");
        let spec_dir = root.join("skill");
        fs::create_dir_all(spec_dir.join("source")).unwrap();
        fs::write(spec_dir.join("source/requirements.txt"), "pillow\n").unwrap();

        let dependency = Dependency {
            kind: DependencyKind::File,
            description: None,
            command: None,
            path: Some("source/requirements.txt".to_owned()),
            env: None,
            check: None,
            permission: None,
            provision: None,
        };

        let (status, method, message) = check_file("requirements_txt", &dependency, &spec_dir);

        assert_eq!(status, DependencyStatus::Present);
        assert_eq!(method, DependencyCheckMethod::FileExists);
        assert_eq!(message, "source/requirements.txt exists");
    }

    #[test]
    fn package_dependencies_are_deferred_not_failed() {
        let yaml = r#"
schema: skillspec/v0
id: package.dependency
title: Package Dependency
description: Keeps package checks visible without failing local preflight.
routes:
  - id: local
    label: Local
dependencies:
  pypdf:
    kind: package
tests:
  - name: route assertion
    input: inspect pdf
    expect:
      route: local
"#;
        let spec = serde_yaml::from_str::<SkillSpec>(yaml).unwrap();
        crate::parser::validate_spec(&spec).unwrap();

        let report = check(&spec, Path::new("."), None).unwrap();

        assert!(report.ok);
        assert_eq!(report.dependencies[0].status, DependencyStatus::Deferred);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("{prefix}-{nanos}"))
    }
}
