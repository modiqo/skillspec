use serde::Serialize;
use skillspec_core::error::{Error, Result};
use skillspec_core::{import_dependency_ledger, parser};
use std::fs;
use std::path::PathBuf;

mod command_inference;
mod evidence;
mod report;
mod spec_builder;

use command_inference::{dependency_ids, infer_observed_commands, skill_id, title_from_id};
use evidence::{collect_evidence, render_synthesis_approval_required, validate_evidence};
pub use report::render_report;
use spec_builder::build_spec;

#[derive(Debug)]
pub struct SynthesizeOptions {
    pub workspace: String,
    pub task: Option<String>,
    pub out: PathBuf,
    pub name: Option<String>,
    pub log_last: usize,
    pub workspace_stats_report: Option<PathBuf>,
    pub workspace_log: Option<PathBuf>,
    pub workspace_meta: Option<PathBuf>,
    pub workspace_deps: Option<PathBuf>,
    pub observation_approved: bool,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct SynthesisReport {
    pub out_dir: PathBuf,
    pub spec_path: PathBuf,
    pub deps_path: PathBuf,
    pub inferred_dependencies: Vec<String>,
    pub command_candidates: usize,
    pub review_required: Vec<String>,
}

pub fn synthesize_from_workspace(options: SynthesizeOptions) -> Result<SynthesisReport> {
    let workspace = options.workspace.trim().to_owned();
    if workspace.is_empty() {
        return Err(Error::InvalidInput {
            message: "synthesize-from-workspace requires a non-empty workspace name".to_owned(),
        });
    }
    if options.log_last == 0 {
        return Err(Error::InvalidInput {
            message: "--log-last must be greater than zero".to_owned(),
        });
    }

    let spec_path = options.out.join("skill.spec.yml");
    if spec_path.exists() && !options.force {
        return Err(Error::InvalidInput {
            message: format!(
                "{} already exists; rerun with --force to overwrite the synthesized scaffold",
                spec_path.display()
            ),
        });
    }

    let evidence = collect_evidence(&options, &workspace)?;
    validate_evidence(&workspace, &evidence)?;

    let task = options
        .task
        .as_deref()
        .unwrap_or("repeat observed workflow");
    let skill_id = skill_id(options.name.as_deref(), task);
    let title = title_from_id(&skill_id);
    let commands = infer_observed_commands(&evidence.log);

    if !options.observation_approved {
        return Err(Error::InvalidInput {
            message: render_synthesis_approval_required(&evidence, &commands),
        });
    }

    let spec = build_spec(&skill_id, &title, &commands);
    parser::validate_spec(&spec)?;

    fs::create_dir_all(&options.out).map_err(|source| Error::Write {
        path: options.out.clone(),
        source,
    })?;
    import_dependency_ledger::materialize_with_generator(
        &spec,
        &options.out,
        "skillspec synthesize-from-workspace",
    )?;
    parser::write_spec(&spec_path, &spec)?;

    Ok(SynthesisReport {
        out_dir: options.out.clone(),
        spec_path,
        deps_path: options.out.join(import_dependency_ledger::DEPS_TOML_PATH),
        inferred_dependencies: dependency_ids(&commands),
        command_candidates: commands.len(),
        review_required: spec.review_required.clone(),
    })
}
