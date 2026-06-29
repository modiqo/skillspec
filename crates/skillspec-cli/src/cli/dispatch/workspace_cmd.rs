use crate::cli::args::WorkspaceCommand;
use skillspec::{error::Result, install::HarnessTarget, report, workspace};
use std::time::Instant;

pub(super) fn run(command: WorkspaceCommand) -> Result<()> {
    match command {
        WorkspaceCommand::Map {
            source_root,
            out,
            install_slug_policy,
            json,
            summary,
        } => {
            let started = Instant::now();
            let workspace_report =
                workspace::map_workspace(&source_root, &out, install_slug_policy.into())?;
            let elapsed = started.elapsed();
            if json {
                report::json(&workspace_report)?;
            } else if summary {
                report::text(&workspace::render_map_summary(&workspace_report, elapsed))?;
            } else {
                let manifest = workspace::load_manifest(&out)?;
                report::text(&workspace::render_map_report(&workspace_report, &manifest))?;
            }
        }
        WorkspaceCommand::Validate {
            manifest,
            json,
            summary,
        } => {
            let started = Instant::now();
            let validation_report = workspace::validate_workspace(&manifest)?;
            let elapsed = started.elapsed();
            let ok = validation_report.ok;
            if json {
                report::json(&validation_report)?;
            } else if summary {
                report::text(&workspace::render_validation_summary(
                    &validation_report,
                    elapsed,
                ))?;
            } else {
                report::text(&workspace::render_validation_report(&validation_report))?;
            }
            if !ok {
                std::process::exit(1);
            }
        }
        WorkspaceCommand::Import {
            manifest,
            out,
            json,
            summary,
        } => {
            let started = Instant::now();
            let import_report = workspace::import_workspace(&manifest, &out)?;
            let elapsed = started.elapsed();
            let ok = import_report.ok;
            if json {
                report::json(&import_report)?;
            } else if summary {
                report::text(&workspace::render_import_summary(&import_report, elapsed))?;
            } else {
                report::text(&workspace::render_import_report(&import_report))?;
            }
            if !ok {
                std::process::exit(1);
            }
        }
        WorkspaceCommand::Converge {
            manifest,
            build_root,
            json,
            summary,
        } => {
            let started = Instant::now();
            let converge_report = workspace::converge_workspace(&manifest, &build_root)?;
            let elapsed = started.elapsed();
            let ok = converge_report.ok;
            if json {
                report::json(&converge_report)?;
            } else if summary {
                report::text(&workspace::render_converge_summary(
                    &converge_report,
                    elapsed,
                ))?;
            } else {
                report::text(&workspace::render_converge_report(&converge_report))?;
            }
            if !ok {
                std::process::exit(1);
            }
        }
        WorkspaceCommand::Compile {
            manifest,
            build_root,
            target,
            json,
            summary,
        } => {
            let started = Instant::now();
            let compile_report =
                workspace::compile_workspace(&manifest, &build_root, target.into())?;
            let elapsed = started.elapsed();
            let ok = compile_report.ok;
            if json {
                report::json(&compile_report)?;
            } else if summary {
                report::text(&workspace::render_compile_summary(&compile_report, elapsed))?;
            } else {
                report::text(&workspace::render_compile_report(&compile_report))?;
            }
            if !ok {
                std::process::exit(1);
            }
        }
        WorkspaceCommand::Install {
            manifest,
            build_root,
            target,
            all_detected,
            dry_run,
            retire_existing,
            install_slug_policy,
            visibility_policy,
            apply_visibility,
            visibility_manifest,
            json,
            summary,
        } => {
            let targets = target
                .into_iter()
                .map(HarnessTarget::from)
                .collect::<Vec<_>>();
            let started = Instant::now();
            let install_report =
                workspace::install_workspace(workspace::WorkspaceInstallRequest {
                    manifest_path: &manifest,
                    build_root: &build_root,
                    targets: &targets,
                    all_detected,
                    dry_run,
                    retire_existing,
                    install_slug_policy: install_slug_policy.map(Into::into),
                    visibility_policy: visibility_policy.into(),
                    apply_visibility,
                    visibility_manifest: visibility_manifest.as_deref(),
                })?;
            let elapsed = started.elapsed();
            let ok = install_report.ok;
            if json {
                report::json(&install_report)?;
            } else if summary {
                report::text(&workspace::render_install_summary(&install_report, elapsed))?;
            } else {
                report::text(&workspace::render_install_report(&install_report))?;
            }
            if !ok {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
