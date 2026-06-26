mod validation;

pub use validation::validate_spec;

use crate::error::{Error, Result};
use crate::imports;
use crate::model::SkillSpec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SPEC_CACHE_SCHEMA: &str = "skillspec/spec-cache/v1";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SpecCacheFile {
    schema: String,
    cli_version: String,
    path: String,
    source_hash: String,
    spec: SkillSpec,
}

pub fn load_spec(path: &Path) -> Result<SkillSpec> {
    let spec = load_spec_unresolved(path)?;
    imports::validate(&spec, path)?;
    validation::validate_package_sidecars(&spec, path)?;
    Ok(spec)
}

pub fn load_spec_unresolved(path: &Path) -> Result<SkillSpec> {
    let content = fs::read(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let source_hash = sha256_hex(&content);
    if let Some(spec) = load_cached_spec(path, &source_hash) {
        validate_spec(&spec)?;
        return Ok(spec);
    }
    let content = String::from_utf8(content).map_err(|source| Error::InvalidInput {
        message: format!("skill spec {} is not valid UTF-8: {source}", path.display()),
    })?;
    let spec: SkillSpec = serde_yaml::from_str(&content).map_err(|source| Error::ParseYaml {
        path: path.to_path_buf(),
        source,
    })?;
    validate_spec(&spec)?;
    store_cached_spec(path, &source_hash, &spec);
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

fn load_cached_spec(path: &Path, source_hash: &str) -> Option<SkillSpec> {
    let cache_path = spec_cache_path(path)?;
    let content = fs::read_to_string(cache_path).ok()?;
    let cache: SpecCacheFile = serde_json::from_str(&content).ok()?;
    let canonical = canonical_path_string(path);
    (cache.schema == SPEC_CACHE_SCHEMA
        && cache.cli_version == CLI_VERSION
        && cache.source_hash == source_hash
        && cache.path == canonical)
        .then_some(cache.spec)
}

fn store_cached_spec(path: &Path, source_hash: &str, spec: &SkillSpec) {
    let Some(cache_path) = spec_cache_path(path) else {
        return;
    };
    let Some(parent) = cache_path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let cache = SpecCacheFile {
        schema: SPEC_CACHE_SCHEMA.to_owned(),
        cli_version: CLI_VERSION.to_owned(),
        path: canonical_path_string(path),
        source_hash: source_hash.to_owned(),
        spec: spec.clone(),
    };
    let Ok(content) = serde_json::to_string(&cache) else {
        return;
    };
    let _ = fs::write(cache_path, content);
}

fn spec_cache_path(path: &Path) -> Option<PathBuf> {
    let base = if path.is_dir() { path } else { path.parent()? };
    Some(base.join(".skillspec/cache/spec-cache.json"))
}

fn canonical_path_string(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
