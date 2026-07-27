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
pub mod concealment;
pub mod dedupe;
pub mod directive;
pub mod drift;
pub mod effect;
pub mod emit;
pub mod extract;
pub mod guard;
pub mod normalize;
pub mod proposal;
pub mod render;
pub mod sanitize;
pub mod surface;

pub use bounds::{Bounds, Budget};
pub use concealment::Concealment;
pub use dedupe::Effect;
pub use directive::Directive;
pub use drift::{diff, DriftReport};
pub use effect::{
    Confidence, EffectClass, EffectEvidence, EffectObservation, EffectOrigin, EffectTarget,
    PathClass, Reach, TargetResolution,
};
pub use emit::{emit, EmitTarget};
pub use guard::{GuardPolicy, GuardStore, Mode};
pub use proposal::{compile, Proposal};
pub use render::render;
pub use sanitize::Preview;
pub use surface::{EffectSurface, EXTRACTOR_VERSION};

use skillspec_core::error::{Error, Result};
use skillspec_source::remote;
use std::path::Path;

/// Analyze `target`, and the same package at a prior git revision, and diff them.
///
/// The target must be a local path inside a git working tree. The prior revision
/// is materialized in a detached worktree that is removed afterwards; neither
/// the working tree nor the index is disturbed, and nothing is executed.
pub fn diff_against(target: &str, git_ref: &str) -> Result<DriftReport> {
    let head = analyze_target(target)?;
    let worktree = remote::worktree_at_ref(Path::new(target), git_ref)?;
    let base = analyze(worktree.package_dir())?;
    Ok(diff(&base, &head, git_ref, "working tree"))
}

/// The disposition of a `check` run, mapped to a process exit code.
///
/// The codes are a contract with CI. Incompleteness (code 2) is deliberately
/// distinct from findings (code 1): "the tool could not fully determine the
/// surface" is a different fact from "the tool determined it and it is
/// concerning", and a job may treat them differently.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckOutcome {
    /// No finding at or above the threshold.
    Clean = 0,
    /// A finding at or above the threshold.
    Findings = 1,
    /// The proposal is incomplete because the surface was not fully determined.
    Incomplete = 2,
}

impl CheckOutcome {
    pub fn exit_code(self) -> i32 {
        self as i32
    }
}

/// What `check` evaluated the target against.
pub enum CheckMode {
    /// First contact: the whole surface is reviewed against the threshold.
    Absolute,
    /// Update: only what changed since `git_ref` is gated.
    Against(String),
}

/// Evaluate a target for a CI gate.
///
/// Absolute review is the rule for first contact; drift is the rule for an
/// update. A skill hostile from its first commit shows no drift, and an attacker
/// controls the baseline history, so gating a first install on drift would be
/// backwards.
pub fn check(
    target: &str,
    mode: CheckMode,
    fail_on_incomplete: bool,
) -> Result<(CheckOutcome, String)> {
    match mode {
        CheckMode::Absolute => {
            let surface = analyze_target(target)?;
            let proposal = compile(&surface);
            // Concealment and directives are concerning in their own right: they
            // operate inside permissions the skill already has, so a boundary
            // does not cover them and the gate must.
            let concerning = !surface.summary.sensitive_path_classes.is_empty()
                || !surface.concealment.is_empty()
                || !surface.directives.is_empty()
                || surface
                    .all()
                    .any(|effect| effect.class == EffectClass::NetEgress);
            let mut report = render(&surface);
            if !proposal.complete && fail_on_incomplete {
                report
                    .push_str("\nThe surface is incomplete; failing on the incompleteness gate.\n");
                return Ok((CheckOutcome::Incomplete, report));
            }
            let outcome = if concerning {
                CheckOutcome::Findings
            } else {
                CheckOutcome::Clean
            };
            Ok((outcome, report))
        }
        CheckMode::Against(git_ref) => {
            let drift = diff_against(target, &git_ref)?;
            let report = drift::render(&drift);
            if !drift.comparable {
                return Ok((CheckOutcome::Incomplete, report));
            }
            let outcome = if drift.requires_review() {
                CheckOutcome::Findings
            } else {
                CheckOutcome::Clean
            };
            Ok((outcome, report))
        }
    }
}

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

/// Decode concealment payloads from a local target and write them to `out`.
///
/// Local targets only: a remote checkout is gone by the time this runs, and
/// revealing a payload is a deliberate act on files the user already has. The
/// decoded text never touches stdout, JSON, or a report.
pub fn reveal_payloads(target: &str, out: &str) -> Result<()> {
    let root = Path::new(target);
    if !root.exists() {
        return Err(Error::InvalidInput {
            message: format!(
                "--reveal works on a local skill folder; {target:?} is not a local path"
            ),
        });
    }
    let mut revealed = String::new();
    collect_reveal(root, root, &mut revealed)?;
    if revealed.is_empty() {
        revealed.push_str("# No decodable concealment payloads were found.\n");
    }
    std::fs::write(out, revealed).map_err(|source| Error::Write {
        path: Path::new(out).to_path_buf(),
        source,
    })?;
    Ok(())
}

fn collect_reveal(root: &Path, dir: &Path, out: &mut String) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }
            collect_reveal(root, &path, out)?;
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            out.push_str(&concealment::reveal(&text, &relative));
        }
    }
    Ok(())
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

    let directives = directive::scan(
        &extraction.skill_body,
        &extraction.skill_path,
        extraction.skill_body_line,
        extraction.activation_description.as_deref(),
    );
    let concealment = extraction.concealment.clone();

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

    let mut surface = EffectSurface::new(
        path.display().to_string(),
        extraction.skill_path,
        analysis,
        effects,
    );
    surface.concealment = concealment;
    surface.directives = directives;
    Ok(surface)
}
