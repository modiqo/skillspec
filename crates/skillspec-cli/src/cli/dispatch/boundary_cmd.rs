use crate::cli::args::BoundaryCommand;
use skillspec::{domain::boundary, error::Result, report};
use std::fs;
use std::path::PathBuf;

pub(super) fn run(target: Option<String>, json: bool) -> Result<()> {
    let target = target.ok_or_else(|| skillspec::error::Error::InvalidInput {
        message: "boundary requires a target or subcommand, for example `skillspec boundary ./my-skill` or `skillspec boundary emit ./my-skill`"
            .to_owned(),
    })?;
    let surface = boundary::analyze_target(&target)?;
    if json {
        report::json(&surface)
    } else {
        report::text(&boundary::render(&surface))
    }
}

pub(super) fn command(command: BoundaryCommand) -> Result<()> {
    match command {
        BoundaryCommand::Emit { path, format, out } => emit(path, format, out),
    }
}

fn emit(path: String, format: String, out: Option<String>) -> Result<()> {
    let target = boundary::parse_format(&format)?;
    let surface = boundary::analyze_target(&path)?;
    let proposal = boundary::compile(&surface);
    let artifact = boundary::emit(&proposal, target)?;

    match out {
        Some(out) => {
            let out = PathBuf::from(out);
            if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
                fs::create_dir_all(parent).map_err(|source| skillspec::error::Error::Write {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::write(&out, artifact).map_err(|source| skillspec::error::Error::Write {
                path: out.clone(),
                source,
            })?;
            report::text(&format!("Wrote {}\n", out.display()))
        }
        None => report::text(&artifact),
    }
}
