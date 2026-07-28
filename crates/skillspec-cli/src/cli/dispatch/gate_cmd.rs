//! `skillspec boundary gate` — a pre-install gate for skills and plugins.
//!
//! Skills ship in a git repository and are copied out of it on install, so the
//! repository can be assessed before anything lands on disk. The gate maps and
//! assesses the target, shows the shape and the risk, asks the user to proceed,
//! and runs the real install command only on approval. It is the human-driven
//! counterpart to the guard hook, which intercepts an agent-driven install at
//! PreToolUse.

use super::assess::{self, Outcome};
use skillspec::{error::Result, report};
use std::process::Command;

pub(super) fn run(path: String, then: Option<String>, assume_yes: bool) -> Result<()> {
    let assessment = assess::present(&path)?;
    match assess::confirm(&assessment, assume_yes)? {
        Outcome::Refused => {
            report::text(
                "This install has findings and no terminal is attached to confirm.\nRe-run with --yes to proceed, or install it manually after review.\n",
            )?;
            std::process::exit(2);
        }
        Outcome::Declined => {
            report::text("Declined. Nothing was installed.\n")?;
            std::process::exit(1);
        }
        Outcome::Approved => {}
    }

    if assessment.concerning && assume_yes {
        report::text("Proceeding despite findings (--yes).\n")?;
    }

    match then {
        Some(command) => {
            report::text(&format!("Running: {command}\n"))?;
            let status = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .status()
                .map_err(|source| skillspec::error::Error::InvalidInput {
                    message: format!("failed to run the install command: {source}"),
                })?;
            std::process::exit(status.code().unwrap_or(1));
        }
        None => report::text("Approved. Run your install command to proceed.\n"),
    }
}
