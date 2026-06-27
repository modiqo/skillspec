use super::render;
use super::types::{GuideReport, PriorGuideState};
use crate::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_prior(run_dir: &Path) -> Result<Option<PriorGuideState>> {
    let path = guide_state_path(run_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|source| Error::ParseJson { path, source })
}

pub fn write(run_dir: &Path, report: &GuideReport) -> Result<()> {
    fs::create_dir_all(run_dir).map_err(|source| Error::Write {
        path: run_dir.to_path_buf(),
        source,
    })?;
    let state_path = guide_state_path(run_dir);
    let state_content = serde_json::to_vec_pretty(report)?;
    fs::write(&state_path, state_content).map_err(|source| Error::Write {
        path: state_path,
        source,
    })?;

    let summary_path = guide_summary_path(run_dir);
    fs::write(&summary_path, render::render_summary_markdown(report)).map_err(|source| {
        Error::Write {
            path: summary_path,
            source,
        }
    })?;
    Ok(())
}

pub fn guide_state_path(run_dir: &Path) -> PathBuf {
    run_dir.join("guide-state.json")
}

pub fn guide_summary_path(run_dir: &Path) -> PathBuf {
    run_dir.join("guide-summary.md")
}
