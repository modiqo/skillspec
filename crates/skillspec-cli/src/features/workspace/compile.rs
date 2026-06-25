use super::{
    dependency_edges, load_manifest, output_package_dir, path_to_string, topological_package_order,
    validate_workspace, write_text, WorkspaceDependencyEdge, WorkspacePackage, WorkspaceReference,
};
use crate::compiler::{self, Target};
use crate::error::{Error, Result};
use crate::parser;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceCompileReport {
    pub ok: bool,
    pub manifest_path: String,
    pub build_root: String,
    pub report_path: String,
    pub target: String,
    pub package_count: usize,
    pub compiled: Vec<String>,
    pub failed: Vec<String>,
    pub blocked: Vec<String>,
    pub missing: Vec<String>,
    pub skipped: Vec<String>,
    pub validation_warnings: Vec<String>,
    pub cross_package_references: Vec<WorkspaceReference>,
    pub dependency_edges: Vec<WorkspaceDependencyEdge>,
    pub packages: Vec<WorkspaceCompilePackageReport>,
    pub next: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceCompilePackageReport {
    pub package_id: String,
    pub status: WorkspaceCompileStatus,
    pub output_dir: String,
    pub spec_path: String,
    pub loader_path: String,
    pub converge_status: Option<String>,
    pub dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceCompileStatus {
    Compiled,
    Failed,
    Blocked,
    Missing,
    Skipped,
}

impl WorkspaceCompileStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Compiled => "compiled",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Missing => "missing",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug)]
struct ConvergePackageEvidence {
    status: super::converge::WorkspaceConvergeStatus,
    message: Option<String>,
}

pub fn compile_workspace(
    manifest_path: &Path,
    build_root: &Path,
    target: Target,
) -> Result<WorkspaceCompileReport> {
    if matches!(target, Target::Markdown) {
        return Err(Error::InvalidInput {
            message:
                "workspace compile writes SKILL.md loaders and supports codex-skill or claude-skill targets; use ordinary `skillspec compile` for markdown"
                    .to_owned(),
        });
    }

    let validation = validate_workspace(manifest_path)?;
    if !validation.ok {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace compile requires a valid manifest; run `skillspec workspace validate {}` first. Errors: {}",
                manifest_path.display(),
                validation.errors.join("; ")
            ),
        });
    }

    let manifest = load_manifest(manifest_path)?;
    let validation_warnings = validation.warnings;
    let cross_package_references = manifest
        .references
        .iter()
        .filter(|reference| reference.target_package.is_some())
        .cloned()
        .collect::<Vec<_>>();
    if !build_root.is_dir() {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace compile build root is not a directory: {}",
                build_root.display()
            ),
        });
    }

    let converge_report = super::converge_workspace(manifest_path, build_root)?;
    let converge_statuses = converge_report
        .packages
        .into_iter()
        .map(|package| {
            (
                package.package_id,
                ConvergePackageEvidence {
                    status: package.status,
                    message: package.message,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut package_reports = Vec::new();
    let mut statuses = BTreeMap::<String, WorkspaceCompileStatus>::new();
    for package_id in topological_package_order(&manifest) {
        let package = manifest
            .packages
            .get(&package_id)
            .expect("topological order only includes known packages");
        let blocked_by = package
            .depends_on
            .iter()
            .filter(|dependency| {
                statuses.get(*dependency) != Some(&WorkspaceCompileStatus::Compiled)
            })
            .cloned()
            .collect::<Vec<_>>();

        let package_report = if blocked_by.is_empty() {
            compile_one_package(
                package,
                build_root,
                target,
                converge_statuses.get(&package_id),
            )?
        } else {
            blocked_package_report(
                package,
                build_root,
                converge_statuses.get(&package_id),
                &blocked_by,
            )?
        };
        statuses.insert(package.package_id.clone(), package_report.status.clone());
        package_reports.push(package_report);
    }

    let compiled = package_ids_by_status(&package_reports, WorkspaceCompileStatus::Compiled);
    let failed = package_ids_by_status(&package_reports, WorkspaceCompileStatus::Failed);
    let blocked = package_ids_by_status(&package_reports, WorkspaceCompileStatus::Blocked);
    let missing = package_ids_by_status(&package_reports, WorkspaceCompileStatus::Missing);
    let skipped = package_ids_by_status(&package_reports, WorkspaceCompileStatus::Skipped);
    let report_path = build_root.join("workspace-compile.report.md");

    let report = WorkspaceCompileReport {
        ok: failed.is_empty() && blocked.is_empty() && missing.is_empty(),
        manifest_path: path_to_string(manifest_path),
        build_root: path_to_string(build_root),
        report_path: path_to_string(&report_path),
        target: target_name(target).to_owned(),
        package_count: manifest.packages.len(),
        compiled,
        failed,
        blocked,
        missing,
        skipped,
        validation_warnings,
        cross_package_references,
        dependency_edges: dependency_edges(&manifest),
        packages: package_reports,
        next: vec![
            "review compiled package loaders before install; workspace install is Phase 7"
                .to_owned(),
        ],
    };
    write_text(&report_path, &render_compile_report(&report))?;
    Ok(report)
}

pub fn render_compile_report(report: &WorkspaceCompileReport) -> String {
    let mut output = String::new();
    output.push_str("Workspace compile\n\n");
    output.push_str(&format!("- manifest: {}\n", report.manifest_path));
    output.push_str(&format!("- build_root: {}\n", report.build_root));
    output.push_str(&format!("- target: {}\n", report.target));
    output.push_str(&format!("- packages: {}\n", report.package_count));
    output.push_str(&format!(
        "- status: {}\n",
        if report.ok { "ok" } else { "failed" }
    ));
    output.push_str(&format!("- report: {}\n", report.report_path));
    output.push('\n');

    push_id_list(&mut output, "Compiled", &report.compiled);
    push_id_list(&mut output, "Failed", &report.failed);
    push_id_list(&mut output, "Blocked", &report.blocked);
    push_id_list(&mut output, "Missing", &report.missing);
    push_id_list(&mut output, "Skipped", &report.skipped);

    output.push_str("\n## Graph Proof\n\n");
    output.push_str("- manifest validation: passed\n");
    output.push_str("- cycles checked: passed\n");
    output.push_str("- dependency references checked: passed\n");
    output.push_str("- install slug collision check: passed\n");
    output.push_str(&format!(
        "- cross-package references: {}\n",
        report.cross_package_references.len()
    ));
    output.push_str(&format!(
        "- compiled package loaders: {}/{}\n",
        report.compiled.len(),
        report.package_count
    ));
    if !report.validation_warnings.is_empty() {
        output.push_str("- validation warnings:\n");
        for warning in &report.validation_warnings {
            output.push_str(&format!("  - {warning}\n"));
        }
    }

    output.push_str("\n## Packages\n\n");
    for package in &report.packages {
        output.push_str(&format!(
            "- {}: {} -> {}\n",
            package.package_id,
            package.status.as_str(),
            package.loader_path
        ));
        if let Some(converge_status) = &package.converge_status {
            output.push_str(&format!("  converge_status: {converge_status}\n"));
        }
        if let Some(message) = &package.message {
            output.push_str(&format!("  message: {message}\n"));
        }
    }

    output.push_str("\n## Dependency Graph\n\n");
    if report.dependency_edges.is_empty() {
        output.push_str("- none\n");
    } else {
        for edge in &report.dependency_edges {
            output.push_str(&format!("- {} -> {}\n", edge.from, edge.to));
        }
    }

    output.push_str("\n## Next\n\n");
    if report.ok {
        for next in &report.next {
            output.push_str(&format!("- {next}\n"));
        }
    } else {
        output.push_str("- fix failed, blocked, or missing packages, rerun workspace converge, then rerun workspace compile\n");
    }
    output
}

fn compile_one_package(
    package: &WorkspacePackage,
    build_root: &Path,
    target: Target,
    converge_status: Option<&ConvergePackageEvidence>,
) -> Result<WorkspaceCompilePackageReport> {
    let output_dir = output_package_dir(package, build_root)?;
    let spec_path = output_dir.join("skill.spec.yml");
    let loader_path = output_dir.join("SKILL.md");
    let converge_status_value = converge_status.map(|status| status.status.clone());

    let (status, message) = match converge_status.map(|status| &status.status) {
        Some(super::converge::WorkspaceConvergeStatus::Ready) => {
            if !spec_path.is_file() {
                (
                    WorkspaceCompileStatus::Missing,
                    Some(format!("missing generated spec {}", spec_path.display())),
                )
            } else {
                match parser::load_spec(&spec_path) {
                    Ok(spec) => {
                        let compiled =
                            compiler::compile_with_skill_name(&spec, target, &package.public_name);
                        write_text(&loader_path, &compiled)?;
                        (WorkspaceCompileStatus::Compiled, None)
                    }
                    Err(error) => (WorkspaceCompileStatus::Failed, Some(error.to_string())),
                }
            }
        }
        Some(super::converge::WorkspaceConvergeStatus::Failed) => (
            WorkspaceCompileStatus::Failed,
            converge_status
                .and_then(|status| status.message.clone())
                .or_else(|| Some("workspace converge reported this package as failed".to_owned())),
        ),
        Some(super::converge::WorkspaceConvergeStatus::Blocked) => (
            WorkspaceCompileStatus::Blocked,
            converge_status
                .and_then(|status| status.message.clone())
                .or_else(|| Some("workspace converge reported this package as blocked".to_owned())),
        ),
        Some(super::converge::WorkspaceConvergeStatus::Missing) => (
            WorkspaceCompileStatus::Missing,
            converge_status
                .and_then(|status| status.message.clone())
                .or_else(|| Some("workspace converge reported this package as missing".to_owned())),
        ),
        None => (
            WorkspaceCompileStatus::Missing,
            Some("workspace converge did not report this package".to_owned()),
        ),
    };

    Ok(WorkspaceCompilePackageReport {
        package_id: package.package_id.clone(),
        status,
        output_dir: path_to_string(&output_dir),
        spec_path: path_to_string(&spec_path),
        loader_path: path_to_string(&loader_path),
        converge_status: converge_status_value
            .map(|status| converge_status_name(&status).to_owned()),
        dependencies: package.depends_on.clone(),
        message,
    })
}

fn blocked_package_report(
    package: &WorkspacePackage,
    build_root: &Path,
    converge_status: Option<&ConvergePackageEvidence>,
    blocked_by: &[String],
) -> Result<WorkspaceCompilePackageReport> {
    let output_dir = output_package_dir(package, build_root)?;
    let spec_path = output_dir.join("skill.spec.yml");
    let loader_path = output_dir.join("SKILL.md");
    Ok(WorkspaceCompilePackageReport {
        package_id: package.package_id.clone(),
        status: WorkspaceCompileStatus::Blocked,
        output_dir: path_to_string(&output_dir),
        spec_path: path_to_string(&spec_path),
        loader_path: path_to_string(&loader_path),
        converge_status: converge_status
            .map(|status| converge_status_name(&status.status).to_owned()),
        dependencies: package.depends_on.clone(),
        message: Some(format!(
            "blocked because dependencies did not compile: {}",
            blocked_by.join(", ")
        )),
    })
}

fn package_ids_by_status(
    packages: &[WorkspaceCompilePackageReport],
    status: WorkspaceCompileStatus,
) -> Vec<String> {
    packages
        .iter()
        .filter_map(|package| (package.status == status).then_some(package.package_id.clone()))
        .collect()
}

fn push_id_list(output: &mut String, title: &str, ids: &[String]) {
    output.push_str(&format!("## {title}\n\n"));
    if ids.is_empty() {
        output.push_str("- none\n");
    } else {
        for id in ids {
            output.push_str(&format!("- {id}\n"));
        }
    }
    output.push('\n');
}

fn target_name(target: Target) -> &'static str {
    match target {
        Target::CodexSkill => "codex-skill",
        Target::ClaudeSkill => "claude-skill",
        Target::Markdown => "markdown",
    }
}

fn converge_status_name(status: &super::converge::WorkspaceConvergeStatus) -> &'static str {
    match status {
        super::converge::WorkspaceConvergeStatus::Ready => "ready",
        super::converge::WorkspaceConvergeStatus::Failed => "failed",
        super::converge::WorkspaceConvergeStatus::Blocked => "blocked",
        super::converge::WorkspaceConvergeStatus::Missing => "missing",
    }
}
