use crate::{WorkspaceManifest, WorkspacePackage};
use skillspec_core::error::{Error, Result};
use skillspec_harness::install::{self, HarnessRoot};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug)]
pub(super) struct PluginPackageInstallPlan {
    pub(super) plugin_install_slug: String,
    pub(super) plugin_source_dir: PathBuf,
    pub(super) skill_relative_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(super) struct MultiSkillPackageInstallPlan {
    pub(super) workspace_install_slug: String,
    pub(super) workspace_source_dir: PathBuf,
    pub(super) package_relative_path: PathBuf,
}

pub(super) fn plugin_package_plan(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
) -> Option<PluginPackageInstallPlan> {
    let package_path = normalized_workspace_path(&package.path)?;
    for plugin_root in &manifest.source_shape.plugin_roots {
        let plugin_path = normalized_workspace_path(&plugin_root.path).unwrap_or_default();
        let skills_root = if plugin_path.as_os_str().is_empty() {
            PathBuf::from("skills")
        } else {
            plugin_path.join("skills")
        };
        let Ok(skill_relative_path) = package_path.strip_prefix(&skills_root) else {
            continue;
        };
        if skill_relative_path.as_os_str().is_empty() {
            continue;
        }
        return Some(PluginPackageInstallPlan {
            plugin_install_slug: plugin_install_slug(manifest, &plugin_path),
            plugin_source_dir: PathBuf::from(&manifest.source_root).join(&plugin_path),
            skill_relative_path: skill_relative_path.to_path_buf(),
        });
    }
    None
}

pub(super) fn multi_skill_package_plan(
    manifest: &WorkspaceManifest,
    package: &WorkspacePackage,
) -> Option<MultiSkillPackageInstallPlan> {
    Some(MultiSkillPackageInstallPlan {
        workspace_install_slug: manifest.workspace_slug.clone(),
        workspace_source_dir: PathBuf::from(&manifest.source_root),
        package_relative_path: normalized_workspace_path(&package.path)?,
    })
}

pub(super) fn normalized_workspace_path(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

pub(super) fn copy_multi_skill_parent_without_skill_packages(
    manifest: &WorkspaceManifest,
    source_dir: &Path,
    destination: &Path,
) -> Result<()> {
    let replacement_roots = multi_skill_replacement_roots(manifest);
    copy_parent_without_replacement_roots(source_dir, destination, &replacement_roots)?;
    for root in replacement_roots {
        fs::create_dir_all(destination.join(&root)).map_err(|source| Error::Write {
            path: destination.join(root),
            source,
        })?;
    }
    Ok(())
}

pub(super) fn copy_plugin_parent_without_skills(
    source_dir: &Path,
    destination: &Path,
) -> Result<()> {
    fs::create_dir_all(destination).map_err(|source| Error::Write {
        path: destination.to_path_buf(),
        source,
    })?;
    for entry in fs::read_dir(source_dir).map_err(|source| Error::Read {
        path: source_dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: source_dir.to_path_buf(),
            source,
        })?;
        if entry.file_name().to_string_lossy() == "skills" {
            continue;
        }
        let child_source = entry.path();
        let child_destination = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|source| Error::Read {
            path: child_source.clone(),
            source,
        })?;
        if file_type.is_dir() {
            copy_dir_all(&child_source, &child_destination)?;
        } else {
            if let Some(parent) = child_destination.parent() {
                fs::create_dir_all(parent).map_err(|source| Error::Write {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(&child_source, &child_destination).map_err(|source| Error::Write {
                path: child_destination,
                source,
            })?;
        }
    }
    fs::create_dir_all(destination.join("skills")).map_err(|source| Error::Write {
        path: destination.join("skills"),
        source,
    })?;
    Ok(())
}

pub(super) fn plugin_install_identity(root: &HarnessRoot, plugin_install_slug: &str) -> PathBuf {
    install::install_dir_identity(&root.path).join(plugin_install_slug)
}

pub(super) fn workspace_parent_install_identity(
    root: &HarnessRoot,
    workspace_install_slug: &str,
) -> PathBuf {
    install::install_dir_identity(&root.path).join(workspace_install_slug)
}

fn plugin_install_slug(manifest: &WorkspaceManifest, plugin_path: &Path) -> String {
    if plugin_path.as_os_str().is_empty() {
        return manifest.workspace_slug.clone();
    }
    path_slug(plugin_path)
}

fn path_slug(path: &Path) -> String {
    let slug = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str().map(crate::slugify),
            _ => None,
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("--");
    if slug.is_empty() {
        "plugin".to_owned()
    } else {
        slug
    }
}

fn multi_skill_replacement_roots(manifest: &WorkspaceManifest) -> BTreeSet<PathBuf> {
    let package_paths = manifest
        .packages
        .values()
        .filter_map(|package| normalized_workspace_path(&package.path))
        .collect::<Vec<_>>();
    let non_root_paths = package_paths
        .iter()
        .filter(|path| !path.as_os_str().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if !non_root_paths.is_empty()
        && non_root_paths
            .iter()
            .all(|path| path.components().next().is_some_and(is_skills_component))
    {
        return [PathBuf::from("skills")].into_iter().collect();
    }
    non_root_paths.into_iter().collect()
}

fn is_skills_component(component: Component<'_>) -> bool {
    matches!(component, Component::Normal(name) if name == "skills")
}

fn copy_parent_without_replacement_roots(
    source_dir: &Path,
    destination: &Path,
    replacement_roots: &BTreeSet<PathBuf>,
) -> Result<()> {
    fs::create_dir_all(destination).map_err(|source| Error::Write {
        path: destination.to_path_buf(),
        source,
    })?;
    copy_parent_children_without_replacement_roots(
        source_dir,
        source_dir,
        destination,
        replacement_roots,
    )
}

fn copy_parent_children_without_replacement_roots(
    root_source_dir: &Path,
    current_source_dir: &Path,
    current_destination: &Path,
    replacement_roots: &BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(current_source_dir).map_err(|source| Error::Read {
        path: current_source_dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: current_source_dir.to_path_buf(),
            source,
        })?;
        let child_source = entry.path();
        let relative = child_source
            .strip_prefix(root_source_dir)
            .unwrap_or(child_source.as_path());
        if replacement_roots.contains(relative) {
            continue;
        }
        let child_destination = current_destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|source| Error::Read {
            path: child_source.clone(),
            source,
        })?;
        if file_type.is_dir() {
            fs::create_dir_all(&child_destination).map_err(|source| Error::Write {
                path: child_destination.clone(),
                source,
            })?;
            copy_parent_children_without_replacement_roots(
                root_source_dir,
                &child_source,
                &child_destination,
                replacement_roots,
            )?;
        } else {
            if let Some(parent) = child_destination.parent() {
                fs::create_dir_all(parent).map_err(|source| Error::Write {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(&child_source, &child_destination).map_err(|source| Error::Write {
                path: child_destination,
                source,
            })?;
        }
    }
    Ok(())
}

fn copy_dir_all(source_dir: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(|source| Error::Write {
        path: destination.to_path_buf(),
        source,
    })?;
    for entry in fs::read_dir(source_dir).map_err(|source| Error::Read {
        path: source_dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: source_dir.to_path_buf(),
            source,
        })?;
        let child_source = entry.path();
        let child_destination = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|source| Error::Read {
            path: child_source.clone(),
            source,
        })?;
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
