//! Effect-surface enumeration and least-privilege boundary proposals.
//!
//! This crate answers one question about a skill package:
//!
//! ```text
//! If an agent executed this skill, what could it reach, and what is the
//! smallest permission set that still lets the skill work?
//! ```
//!
//! It enumerates rather than classifies. Where a scanner asks whether a given
//! call is malicious - a question that cannot be answered from source text - an
//! enumerator asks only what the package touches, including the ordinary parts.
//! The consequence is the failure mode: a classifier that misses something fails
//! open, whereas an enumerator that misses something fails closed *provided the
//! emitted policy is enforced by a substrate with deny-by-default semantics*.
//! That proviso is load-bearing and is stated wherever the property is claimed.
//!
//! SkillSpec does not enforce anything by itself. A boundary proposal is a
//! policy artifact for a harness permission system, a guard hook, or a network
//! policy to apply.
//!
//! Reports never claim intent. They state what a package can reach and where the
//! evidence is.
//!
//! Design: `docs/design/security/`.
//!
//! This crate is an implementation boundary used by the workspace. It is not a
//! stable Rust API.

pub mod bounds;
pub mod dedupe;
pub mod effect;
pub mod emit;
pub mod extract;
pub mod normalize;
pub mod proposal;
pub mod render;
pub mod sanitize;
pub mod surface;

pub use bounds::{Bounds, Budget};
pub use dedupe::Effect;
pub use effect::{
    Confidence, EffectClass, EffectEvidence, EffectObservation, EffectOrigin, EffectTarget,
    PathClass, Reach, TargetResolution,
};
pub use emit::{emit, EmitTarget};
pub use proposal::{compile, Proposal};
pub use render::render;
pub use sanitize::Preview;
pub use surface::{EffectSurface, EXTRACTOR_VERSION};

use skillspec_core::error::{Error, Result};
use skillspec_source::remote;
use std::path::Path;

/// Enumerate the effect surface of a local folder or a public GitHub target.
///
/// A remote target is staged into a temporary checkout, analyzed, and the
/// checkout is removed when the staging guard drops. Nothing in the package is
/// executed at any point, which is what makes it safe to point this at a
/// repository nobody has reviewed.
pub fn analyze_target(target: &str) -> Result<EffectSurface> {
    let local = Path::new(target);
    if local.exists() {
        let mut surface = analyze(local)?;
        surface.target = target.to_owned();
        return Ok(surface);
    }

    if looks_like_local_target(target) {
        let cwd = std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "<unknown>".to_owned());
        return Err(Error::InvalidInput {
            message: format!(
                "boundary target {target:?} does not exist locally from {cwd}; a path written as ./x, ../x, /x, or ~/x must exist before analysis runs"
            ),
        });
    }

    let Some(source) = remote::parse_target(target)? else {
        return Err(Error::InvalidInput {
            message: format!(
                "boundary target {target:?} does not exist locally; remote analysis supports public GitHub repo or skill-folder URLs such as https://github.com/<owner>/<repo> and https://github.com/<owner>/<repo>/tree/<branch>/<path>"
            ),
        });
    };

    let staged = remote::clone_remote_temp(&source, "skillspec-boundary")?;
    let package_root = match &source.path {
        Some(path) => {
            remote::set_sparse_path(staged.checkout_dir(), path)?;
            staged.checkout_dir().join(path)
        }
        None => staged.checkout_dir().to_path_buf(),
    };
    if !package_root.exists() {
        return Err(Error::InvalidInput {
            message: format!(
                "remote path {} did not materialize from {}",
                source.path.as_deref().unwrap_or("."),
                source.repo_url
            ),
        });
    }

    let mut surface = analyze(&package_root)?;
    surface.target = target.to_owned();
    surface.source_kind = "remote_github".to_owned();
    surface.staged_from = Some(source.repo_url);
    Ok(surface)
}

/// Whether a target was written as a path and so must exist locally.
///
/// Without this, a mistyped local path falls through to the remote parser and
/// the user gets an error about GitHub URLs for a directory they meant to name.
fn looks_like_local_target(target: &str) -> bool {
    let trimmed = target.trim();
    trimmed.starts_with('.')
        || trimmed.starts_with('/')
        || trimmed.starts_with('~')
        || trimmed.ends_with('/')
        || trimmed.ends_with('\\')
}

/// Enumerate the effect surface of a skill package on disk.
///
/// Reads the package; never executes anything in it.
pub fn analyze(path: &Path) -> Result<EffectSurface> {
    analyze_with(path, Bounds::default())
}

/// [`analyze`] with explicit resource limits.
pub fn analyze_with(path: &Path, bounds: Bounds) -> Result<EffectSurface> {
    let map = skillspec_source::source_map::build(path)?;
    let source_root = Path::new(&map.source_root).to_path_buf();
    let mut budget = Budget::new(bounds);
    let extraction = extract::run(&map, &source_root, &mut budget)?;
    let effects = dedupe::merge(extraction.observations);

    let analysis = surface::AnalysisReport {
        extractor_version: EXTRACTOR_VERSION.to_owned(),
        extractors: extraction
            .extractors
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
        extraction_mode: "lexical",
        files_analyzed: budget.files_read(),
        truncated: budget.truncated(),
        files_skipped: budget.skipped().to_vec(),
    };

    Ok(EffectSurface::new(
        path.display().to_string(),
        extraction.skill_path,
        analysis,
        effects,
    ))
}
