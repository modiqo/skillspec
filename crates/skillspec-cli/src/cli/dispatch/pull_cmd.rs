//! `skillspec pull` and `skillspec update` — assess a skill or plugin, then
//! install it through whichever harness applies.
//!
//! This is the proxy front-end over the boundary engine. A skill ships in a git
//! repository; `pull` stages it, runs the same assessment `gate` does, and — on
//! approval — installs it: through the harness CLI for a plugin marketplace, or
//! by placing the skill folders into a skills directory for every other harness.
//! Because assessment is *part of* the install verb, it is not bypassable the
//! way a raw `claude plugin install` is.
//!
//! `update` re-pulls a source and, when it already lives in the destination,
//! shows what capability changed before replacing it.

use super::assess::{self, Outcome};
use skillspec::boundary::install::{self, Method, PlanOptions, Staged};
use skillspec::{boundary, error::Result, report};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Options shared by `pull` and `update`.
pub(super) struct Options {
    pub harness: Option<String>,
    pub into: Option<PathBuf>,
    pub project: bool,
    pub only: Vec<String>,
    pub yes: bool,
}

pub(super) fn pull(source: String, options: Options) -> Result<()> {
    let staged = install::stage(&source)?;
    confirm(&staged, options.yes)?;
    install_staged(&staged, &options, None)
}

pub(super) fn update(source: String, options: Options) -> Result<()> {
    let staged = install::stage(&source)?;
    // Show what changed versus any already-installed copy before touching it.
    let dest = placement_dest(&options);
    confirm(&staged, options.yes)?;
    install_staged(&staged, &options, dest.as_deref())
}

/// Assess the staged source and gate on the outcome. Returns on approval; exits
/// the process on a declined or non-interactive-refused install.
fn confirm(staged: &Staged, assume_yes: bool) -> Result<()> {
    let root = staged.root().to_string_lossy().into_owned();
    let assessment = assess::present(&root)?;
    match assess::confirm(&assessment, assume_yes)? {
        Outcome::Approved => {
            if assessment.concerning && assume_yes {
                report::text("Proceeding despite findings (--yes).\n")?;
            }
            Ok(())
        }
        Outcome::Declined => {
            report::text("Declined. Nothing was installed.\n")?;
            std::process::exit(1);
        }
        Outcome::Refused => {
            report::text(
                "This install has findings and no terminal is attached to confirm.\nRe-run with --yes to proceed, or install it manually after review.\n",
            )?;
            std::process::exit(2);
        }
    }
}

/// The destination `update` would place into, so it can diff against it first.
fn placement_dest(options: &Options) -> Option<PathBuf> {
    options.into.clone().or_else(|| {
        let harness = options.harness.as_deref().unwrap_or("claude");
        install::harness_skills_dir(harness, options.project)
    })
}

fn install_staged(staged: &Staged, options: &Options, drift_against: Option<&Path>) -> Result<()> {
    let plan_options = PlanOptions {
        harness: options.harness.clone(),
        into: options.into.clone(),
        project: options.project,
        only: options.only.clone(),
    };
    let method = install::plan(staged, &plan_options, &on_path)?;
    match method {
        Method::Proxy {
            cli,
            slug,
            marketplace,
            plugins,
        } => proxy_install(&cli, &slug, &marketplace, &plugins),
        Method::Place { dest, skills } => {
            if let Some(installed) = drift_against {
                report_drift(&skills, installed)?;
            }
            place_install(&skills, &dest)
        }
    }
}

fn proxy_install(cli: &str, slug: &str, marketplace: &str, plugins: &[String]) -> Result<()> {
    report::text(&format!(
        "Installing through {cli}: marketplace {slug}, plugin(s) {}\n",
        plugins.join(", ")
    ))?;
    run_cli(cli, &["plugin", "marketplace", "add", slug])?;
    for plugin in plugins {
        let spec = format!("{plugin}@{marketplace}");
        run_cli(cli, &["plugin", "install", &spec])?;
    }
    report::text("Installed.\n")
}

fn place_install(skills: &[PathBuf], dest: &Path) -> Result<()> {
    let written = install::place(skills, dest)?;
    report::text(&format!(
        "Placed {} skill(s) into {}:\n{}\n",
        written.len(),
        dest.display(),
        written
            .iter()
            .map(|path| format!(
                "  {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

/// Show capability drift for any staged skill that already exists at the
/// destination, so an update makes a growing surface visible before replacing.
fn report_drift(skills: &[PathBuf], installed_root: &Path) -> Result<()> {
    for skill in skills {
        let Some(name) = skill.file_name() else {
            continue;
        };
        let installed = installed_root.join(name);
        if !installed.join("SKILL.md").exists() {
            continue;
        }
        let (Ok(before), Ok(after)) = (boundary::analyze(&installed), boundary::analyze(skill))
        else {
            continue;
        };
        let report = boundary::drift::diff(&before, &after, "installed", "update");
        if report.requires_review() {
            report::text(&format!(
                "\nUpdate to {} changes its capability surface:\n{}\n",
                name.to_string_lossy(),
                boundary::drift::render(&report)
            ))?;
        }
    }
    Ok(())
}

fn run_cli(cli: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cli).args(args).status().map_err(|source| {
        skillspec::error::Error::InvalidInput {
            message: format!("failed to run `{cli} {}`: {source}", args.join(" ")),
        }
    })?;
    if !status.success() {
        return Err(skillspec::error::Error::InvalidInput {
            message: format!(
                "`{cli} {}` exited with {}",
                args.join(" "),
                status.code().unwrap_or(-1)
            ),
        });
    }
    Ok(())
}

/// Whether a binary is on `PATH`. A lookup, not an execution — the gate must
/// never run the harness CLI just to test for it.
fn on_path(binary: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(binary);
        candidate.is_file() || candidate.with_extension("exe").is_file()
    })
}
