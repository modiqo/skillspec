use skillspec_core::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn guard_single_skill_source(path: &Path, command_name: &str) -> Result<()> {
    let source_root = source_root(path);
    let skill_files = discover_skill_files(&source_root)?;
    if skill_files.len() <= 1 {
        return Ok(());
    }

    Err(Error::InvalidInput {
        message: format!(
            "{command_name} expects one atomic skill package; found {} SKILL.md files under {}: {}. This is a workspace. Run `skillspec workspace map {} --out <build-dir>/skillspec.workspace.yml` first.",
            skill_files.len(),
            source_root.display(),
            display_paths(&skill_files),
            source_root.display()
        ),
    })
}

fn source_root(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

fn discover_skill_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_skill_files(root, &mut files)?;
    Ok(files)
}

fn collect_skill_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if should_skip_dir(file_name) {
                continue;
            }
            collect_skill_files(&path, files)?;
        } else if file_name.eq_ignore_ascii_case("SKILL.md") {
            files.push(path);
        }
    }
    Ok(())
}

fn should_skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target" | "node_modules" | "vendor" | "dist" | "build"
        )
}

fn display_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
