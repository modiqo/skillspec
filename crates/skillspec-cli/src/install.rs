use crate::error::{Error, Result};
use crate::model::{CodeSource, DependencyKind, SkillSpec};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessTarget {
    Agents,
    Codex,
    ClaudeLocal,
}

impl HarnessTarget {
    pub fn id(self) -> &'static str {
        match self {
            Self::Agents => "agents",
            Self::Codex => "codex",
            Self::ClaudeLocal => "claude-local",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Agents => "agent personal skills",
            Self::Codex => "Codex personal skills",
            Self::ClaudeLocal => "Claude repo-local skills",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HarnessRoot {
    pub target: HarnessTarget,
    pub id: &'static str,
    pub label: &'static str,
    pub path: PathBuf,
    pub detected: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct InstallReport {
    pub skill_name: String,
    pub dry_run: bool,
    pub installs: Vec<InstallTargetReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct InstallTargetReport {
    pub target: HarnessTarget,
    pub id: &'static str,
    pub path: PathBuf,
    pub existed: bool,
    pub retired_existing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<PathBuf>,
    pub status: InstallStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStatus {
    Planned,
    Installed,
}

pub fn detect_targets() -> Result<Vec<HarnessRoot>> {
    let home = home_dir()?;
    let mut roots = vec![
        root(HarnessTarget::Agents, home.join(".agents/skills")),
        root(HarnessTarget::Codex, home.join(".codex/skills")),
    ];
    if let Some(claude_root) = find_claude_skills_root()? {
        roots.push(root(HarnessTarget::ClaudeLocal, claude_root));
    }
    Ok(roots)
}

pub fn install_skill(
    skill_folder: &Path,
    targets: &[HarnessTarget],
    all_detected: bool,
    dry_run: bool,
    force: bool,
    retire_existing: bool,
    name: Option<&str>,
) -> Result<InstallReport> {
    install_skill_impl(
        skill_folder,
        targets,
        all_detected,
        dry_run,
        InstallBehavior {
            force,
            retire_existing,
            refresh_router: true,
        },
        name,
    )
}

pub fn install_skill_without_router_hook(
    skill_folder: &Path,
    targets: &[HarnessTarget],
    all_detected: bool,
    dry_run: bool,
    force: bool,
    retire_existing: bool,
    name: Option<&str>,
) -> Result<InstallReport> {
    install_skill_impl(
        skill_folder,
        targets,
        all_detected,
        dry_run,
        InstallBehavior {
            force,
            retire_existing,
            refresh_router: false,
        },
        name,
    )
}

#[derive(Clone, Copy, Debug)]
struct InstallBehavior {
    force: bool,
    retire_existing: bool,
    refresh_router: bool,
}

fn install_skill_impl(
    skill_folder: &Path,
    targets: &[HarnessTarget],
    all_detected: bool,
    dry_run: bool,
    behavior: InstallBehavior,
    name: Option<&str>,
) -> Result<InstallReport> {
    if behavior.force && behavior.retire_existing {
        return Err(Error::InvalidInput {
            message: "--force and --retire-existing are mutually exclusive; use --retire-existing to back up and remove the old active skill before install".to_owned(),
        });
    }
    let skill_name = match name {
        Some(name) => validate_skill_name(name)?,
        None => infer_skill_name(skill_folder)?,
    };
    validate_skill_folder(skill_folder)?;
    let spec = crate::parser::load_spec(&skill_folder.join("skill.spec.yml"))?;
    let support_files = declared_package_files(skill_folder, &spec)?;

    let target_roots = selected_roots(targets, all_detected)?;
    if target_roots.is_empty() {
        return Err(Error::InvalidInput {
            message: "no install targets selected; use --target or --all-detected".to_owned(),
        });
    }

    let mut pending = Vec::new();
    let mut backup_paths_by_identity: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    let backup_root = if behavior.retire_existing {
        Some(retired_backup_root()?)
    } else {
        None
    };
    for root in target_roots {
        let install_dir = root.path.join(&skill_name);
        let existed = install_dir.exists();
        if existed && !install_dir.is_dir() {
            return Err(Error::InvalidInput {
                message: format!(
                    "install target already exists and is not a directory: {}",
                    install_dir.display()
                ),
            });
        }
        let install_identity = install_identity(&install_dir);
        let backup_path = if existed && behavior.retire_existing {
            match backup_paths_by_identity.get(&install_identity) {
                Some(path) => Some(path.clone()),
                None => {
                    let path = retired_backup_path(
                        backup_root.as_ref().expect("backup root"),
                        root.target,
                        &skill_name,
                    );
                    backup_paths_by_identity.insert(install_identity.clone(), path.clone());
                    Some(path)
                }
            }
        } else {
            None
        };
        pending.push(PendingInstall {
            root,
            install_dir,
            install_identity,
            existed,
            backup_path,
        });
    }

    if !dry_run && !behavior.force && !behavior.retire_existing {
        for install in pending.iter().filter(|install| install.existed) {
            if !confirm_overwrite(&install.install_dir)? {
                return Err(Error::InvalidInput {
                    message: format!(
                        "install target already exists; overwrite declined: {}",
                        install.install_dir.display()
                    ),
                });
            }
        }
    }

    let mut installs = Vec::new();
    let mut retired_identities = BTreeSet::new();
    for install in pending {
        if !dry_run {
            if let Some(backup_path) = &install.backup_path {
                if retired_identities.insert(install.install_identity.clone()) {
                    retire_existing_install(&install.install_dir, backup_path)?;
                }
            }
            copy_skill_package(skill_folder, &install.install_dir, &support_files)?;
        }
        installs.push(InstallTargetReport {
            target: install.root.target,
            id: install.root.id,
            path: install.install_dir,
            existed: install.existed,
            retired_existing: install.existed && behavior.retire_existing,
            backup_path: install.backup_path,
            status: if dry_run {
                InstallStatus::Planned
            } else {
                InstallStatus::Installed
            },
        });
    }

    let report = InstallReport {
        skill_name,
        dry_run,
        installs,
    };
    if !dry_run && behavior.refresh_router {
        crate::router_lifecycle::after_skill_install()?;
    }
    Ok(report)
}

pub fn sync_skill_package(skill_folder: &Path, install_dir: &Path) -> Result<()> {
    validate_skill_folder(skill_folder)?;
    let spec = crate::parser::load_spec(&skill_folder.join("skill.spec.yml"))?;
    let support_files = declared_package_files(skill_folder, &spec)?;
    copy_skill_package(skill_folder, install_dir, &support_files)
}

struct PendingInstall {
    root: HarnessRoot,
    install_dir: PathBuf,
    install_identity: PathBuf,
    existed: bool,
    backup_path: Option<PathBuf>,
}

fn selected_roots(targets: &[HarnessTarget], all_detected: bool) -> Result<Vec<HarnessRoot>> {
    let detected = detect_targets()?;
    if all_detected {
        return Ok(detected
            .into_iter()
            .filter(|root| root.detected)
            .collect::<Vec<_>>());
    }

    let selected = targets.iter().copied().collect::<BTreeSet<_>>();
    let roots = detected
        .into_iter()
        .filter(|root| selected.contains(&root.target))
        .collect::<Vec<_>>();
    Ok(roots)
}

fn confirm_overwrite(path: &Path) -> Result<bool> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return Err(Error::InvalidInput {
            message: format!(
                "install target already exists: {}; rerun with --force to overwrite",
                path.display()
            ),
        });
    }

    let mut stderr = io::stderr().lock();
    write!(
        stderr,
        "install target already exists: {}. Overwrite? [y/N] ",
        path.display()
    )?;
    stderr.flush()?;

    let mut answer = String::new();
    stdin.read_line(&mut answer)?;
    Ok(is_yes(&answer))
}

fn is_yes(answer: &str) -> bool {
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn install_identity(install_dir: &Path) -> PathBuf {
    install_dir
        .canonicalize()
        .unwrap_or_else(|_| install_dir.to_path_buf())
}

fn retire_existing_install(install_dir: &Path, backup_path: &Path) -> Result<()> {
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    if backup_path.exists() {
        return Err(Error::InvalidInput {
            message: format!(
                "retired skill backup already exists: {}",
                backup_path.display()
            ),
        });
    }
    match fs::rename(install_dir, backup_path) {
        Ok(()) => Ok(()),
        Err(_rename_error) => {
            copy_dir_all(install_dir, backup_path)?;
            fs::remove_dir_all(install_dir).map_err(|source| Error::Write {
                path: install_dir.to_path_buf(),
                source,
            })?;
            Ok(())
        }
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(|source| Error::Write {
        path: destination.to_path_buf(),
        source,
    })?;
    for entry in fs::read_dir(source).map_err(|err| Error::Read {
        path: source.to_path_buf(),
        source: err,
    })? {
        let entry = entry.map_err(|err| Error::Read {
            path: source.to_path_buf(),
            source: err,
        })?;
        let file_type = entry.file_type().map_err(|source| Error::Read {
            path: entry.path(),
            source,
        })?;
        let child_source = entry.path();
        let child_destination = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&child_source, &child_destination)?;
        } else {
            fs::copy(&child_source, &child_destination).map_err(|source| Error::Write {
                path: child_destination,
                source,
            })?;
        }
    }
    Ok(())
}

fn retired_backup_root() -> Result<PathBuf> {
    Ok(skillspec_home()?
        .join("backups/retired-skills")
        .join(format!("retire-{}-{}", now_unix(), std::process::id())))
}

fn retired_backup_path(root: &Path, target: HarnessTarget, skill_name: &str) -> PathBuf {
    root.join(target.id()).join(skill_name)
}

fn skillspec_home() -> Result<PathBuf> {
    if let Some(path) = env::var_os("SKILLSPEC_HOME") {
        return Ok(PathBuf::from(path));
    }
    let Some(home) = env::var_os("HOME") else {
        return Err(Error::InvalidInput {
            message: "HOME is not set; set SKILLSPEC_HOME or HOME".to_owned(),
        });
    };
    Ok(PathBuf::from(home).join(".skillspec"))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn validate_skill_folder(skill_folder: &Path) -> Result<()> {
    if !skill_folder.is_dir() {
        return Err(Error::InvalidInput {
            message: format!("{} is not a skill folder", skill_folder.display()),
        });
    }
    for file_name in ["SKILL.md", "skill.spec.yml"] {
        let path = skill_folder.join(file_name);
        if !path.is_file() {
            return Err(Error::InvalidInput {
                message: format!("skill folder is missing {}", path.display()),
            });
        }
    }
    Ok(())
}

fn infer_skill_name(skill_folder: &Path) -> Result<String> {
    let Some(name) = skill_folder.file_name().and_then(|name| name.to_str()) else {
        return Err(Error::InvalidInput {
            message: format!("could not infer skill name from {}", skill_folder.display()),
        });
    };
    validate_skill_name(name)
}

fn validate_skill_name(name: &str) -> Result<String> {
    let valid = !name.is_empty()
        && name.chars().all(|char| {
            char.is_ascii_lowercase() || char.is_ascii_digit() || matches!(char, '-' | '_' | '.')
        });
    if valid {
        Ok(name.to_owned())
    } else {
        Err(Error::InvalidInput {
            message: format!(
                "invalid skill name {name:?}; use lowercase letters, digits, '-', '_', or '.'"
            ),
        })
    }
}

fn copy_file(skill_folder: &Path, install_dir: &Path, file_name: &str) -> Result<()> {
    let source = skill_folder.join(file_name);
    let destination = install_dir.join(file_name);
    if same_existing_file(&source, &destination) {
        return Ok(());
    }
    fs::copy(&source, &destination).map_err(|source| Error::Write {
        path: destination,
        source,
    })?;
    Ok(())
}

fn same_existing_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn copy_skill_package(
    skill_folder: &Path,
    install_dir: &Path,
    support_files: &BTreeSet<PathBuf>,
) -> Result<()> {
    fs::create_dir_all(install_dir).map_err(|source| Error::Write {
        path: install_dir.to_path_buf(),
        source,
    })?;
    copy_file(skill_folder, install_dir, "SKILL.md")?;
    copy_file(skill_folder, install_dir, "skill.spec.yml")?;
    for relative_path in support_files {
        copy_relative_file(skill_folder, install_dir, relative_path)?;
    }
    Ok(())
}

fn declared_package_files(skill_folder: &Path, spec: &SkillSpec) -> Result<BTreeSet<PathBuf>> {
    let mut paths = BTreeSet::new();

    for dependency in spec.dependencies.values() {
        if dependency.kind != DependencyKind::File {
            continue;
        }

        let Some(path) = dependency
            .check
            .as_ref()
            .and_then(|check| check.path.as_deref())
            .or(dependency.path.as_deref())
        else {
            continue;
        };

        insert_declared_package_file(skill_folder, &mut paths, Path::new(path))?;
    }

    for import in spec.imports.values() {
        insert_declared_package_file(skill_folder, &mut paths, Path::new(&import.path))?;
    }

    for resource in spec.resources.values() {
        insert_declared_package_file(skill_folder, &mut paths, Path::new(&resource.path))?;
    }

    for code in spec.code.values() {
        if let CodeSource::File(source) = &code.source {
            insert_declared_package_file(skill_folder, &mut paths, Path::new(&source.file))?;
        }
    }

    Ok(paths)
}

fn insert_declared_package_file(
    skill_folder: &Path,
    paths: &mut BTreeSet<PathBuf>,
    relative_path: &Path,
) -> Result<()> {
    let raw_path = relative_path.to_string_lossy();
    if relative_path.is_absolute() || raw_path.starts_with("~/") || raw_path.contains("://") {
        return Ok(());
    }
    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(Error::InvalidInput {
            message: format!(
                "declared package file path {} must stay within the skill folder",
                relative_path.display()
            ),
        });
    }
    if is_nested_skill_md(relative_path) {
        return Err(Error::InvalidInput {
            message: format!(
                "declared package file {} would create a nested discoverable SKILL.md; preserve original prose as source/SKILL_md.old or another non-discoverable, non-Markdown filename",
                relative_path.display()
            ),
        });
    }

    if !skill_folder.join(relative_path).is_file() {
        return Err(Error::InvalidInput {
            message: format!(
                "declared package file is missing: {}",
                skill_folder.join(relative_path).display()
            ),
        });
    }

    paths.insert(relative_path.to_path_buf());
    Ok(())
}

fn is_nested_skill_md(relative_path: &Path) -> bool {
    relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        && relative_path
            .components()
            .filter(|component| matches!(component, Component::Normal(_)))
            .count()
            > 1
}

fn copy_relative_file(skill_folder: &Path, install_dir: &Path, relative_path: &Path) -> Result<()> {
    let source = skill_folder.join(relative_path);
    let destination = install_dir.join(relative_path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::copy(&source, &destination).map_err(|source| Error::Write {
        path: destination,
        source,
    })?;
    Ok(())
}

fn root(target: HarnessTarget, path: PathBuf) -> HarnessRoot {
    HarnessRoot {
        target,
        id: target.id(),
        label: target.label(),
        detected: path.is_dir(),
        path,
    }
}

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::InvalidInput {
            message: "HOME is not set; cannot detect personal skill roots".to_owned(),
        })
}

fn find_claude_skills_root() -> Result<Option<PathBuf>> {
    let current_dir = env::current_dir().map_err(|source| Error::Read {
        path: PathBuf::from("."),
        source,
    })?;
    for ancestor in current_dir.ancestors() {
        let claude_dir = ancestor.join(".claude");
        if claude_dir.is_dir() {
            return Ok(Some(claude_dir.join("skills")));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::is_yes;

    #[test]
    fn confirmation_accepts_only_explicit_yes() {
        for value in ["y", "Y", "yes", "YES", " yes\n"] {
            assert!(is_yes(value), "{value:?} should confirm overwrite");
        }

        for value in ["", "n", "no", "true", "sure", " y please "] {
            assert!(!is_yes(value), "{value:?} should decline overwrite");
        }
    }
}
