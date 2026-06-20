use crate::error::{Error, Result};
use crate::model::{DependencyKind, SkillSpec};
use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

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

#[derive(Debug, Serialize)]
pub struct InstallReport {
    pub skill_name: String,
    pub dry_run: bool,
    pub installs: Vec<InstallTargetReport>,
}

#[derive(Debug, Serialize)]
pub struct InstallTargetReport {
    pub target: HarnessTarget,
    pub id: &'static str,
    pub path: PathBuf,
    pub status: InstallStatus,
}

#[derive(Debug, Serialize)]
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
    name: Option<&str>,
) -> Result<InstallReport> {
    let skill_name = match name {
        Some(name) => validate_skill_name(name)?,
        None => infer_skill_name(skill_folder)?,
    };
    validate_skill_folder(skill_folder)?;
    let spec = crate::parser::load_spec(&skill_folder.join("skill.spec.yml"))?;
    let support_files = declared_relative_file_dependencies(skill_folder, &spec)?;

    let target_roots = selected_roots(targets, all_detected)?;
    if target_roots.is_empty() {
        return Err(Error::InvalidInput {
            message: "no install targets selected; use --target or --all-detected".to_owned(),
        });
    }

    let mut installs = Vec::new();
    for root in target_roots {
        let install_dir = root.path.join(&skill_name);
        if !dry_run {
            fs::create_dir_all(&install_dir).map_err(|source| Error::Write {
                path: install_dir.clone(),
                source,
            })?;
            copy_file(skill_folder, &install_dir, "SKILL.md")?;
            copy_file(skill_folder, &install_dir, "skill.spec.yml")?;
            for relative_path in &support_files {
                copy_relative_file(skill_folder, &install_dir, relative_path)?;
            }
        }
        installs.push(InstallTargetReport {
            target: root.target,
            id: root.id,
            path: install_dir,
            status: if dry_run {
                InstallStatus::Planned
            } else {
                InstallStatus::Installed
            },
        });
    }

    Ok(InstallReport {
        skill_name,
        dry_run,
        installs,
    })
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
    fs::copy(&source, &destination).map_err(|source| Error::Write {
        path: destination,
        source,
    })?;
    Ok(())
}

fn declared_relative_file_dependencies(
    skill_folder: &Path,
    spec: &SkillSpec,
) -> Result<BTreeSet<PathBuf>> {
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

        let relative_path = Path::new(path);
        if relative_path.is_absolute() {
            continue;
        }
        if relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        }) {
            return Err(Error::InvalidInput {
                message: format!(
                    "file dependency path {} must stay within the skill folder",
                    relative_path.display()
                ),
            });
        }

        if !skill_folder.join(relative_path).is_file() {
            return Err(Error::InvalidInput {
                message: format!(
                    "declared file dependency is missing: {}",
                    skill_folder.join(relative_path).display()
                ),
            });
        }

        paths.insert(relative_path.to_path_buf());
    }
    Ok(paths)
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
