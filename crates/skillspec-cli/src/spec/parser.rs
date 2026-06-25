mod validation;

pub use validation::validate_spec;

use crate::error::{Error, Result};
use crate::imports;
use crate::model::SkillSpec;
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_spec(path: &Path) -> Result<SkillSpec> {
    let spec = load_spec_unresolved(path)?;
    imports::validate(&spec, path)?;
    validation::validate_package_sidecars(&spec, path)?;
    Ok(spec)
}

pub fn load_spec_unresolved(path: &Path) -> Result<SkillSpec> {
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let spec: SkillSpec = serde_yaml::from_str(&content).map_err(|source| Error::ParseYaml {
        path: path.to_path_buf(),
        source,
    })?;
    validate_spec(&spec)?;
    Ok(spec)
}

pub fn write_spec(path: &Path, spec: &SkillSpec) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let content = serde_yaml::to_string(spec).map_err(|source| Error::RenderYaml {
        path: PathBuf::from(path),
        source,
    })?;
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}
