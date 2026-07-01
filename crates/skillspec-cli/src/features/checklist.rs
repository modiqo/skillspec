mod run;
mod shape_steps;
mod workspace_import;

use crate::{doctor, error, guide, source_map, workspace};
use serde::Serialize;
use skillspec_core::error::Error;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const CHECKLIST_SCHEMA: &str = "skillspec/checklist/v0";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistKind {
    Doctor,
    Import,
    Run,
}

impl ChecklistKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::Import => "import",
            Self::Run => "run",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistStage {
    Entry,
    Loop,
    Exit,
}

impl ChecklistStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Loop => "loop",
            Self::Exit => "exit",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistStatus {
    Ready,
    Blocked,
    Complete,
    Partial,
}

impl ChecklistStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Complete => "complete",
            Self::Partial => "partial",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ChecklistReport {
    pub schema: &'static str,
    pub kind: ChecklistKind,
    pub stage: ChecklistStage,
    pub status: ChecklistStatus,
    pub entity: ChecklistEntity,
    pub activation_policy: String,
    pub position: ChecklistPosition,
    pub steps: Vec<ChecklistStep>,
    pub forbid: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ChecklistEntity {
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ChecklistPosition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_packages: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_source_blocks: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_phases: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChecklistStep {
    pub id: String,
    pub description: String,
    pub directive: String,
    pub commands: Vec<String>,
    pub repeat: ChecklistRepeat,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbid: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ChecklistRepeat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_field: Option<String>,
}

pub fn doctor_checklist(target: &str, stage: ChecklistStage) -> error::Result<ChecklistReport> {
    let report = doctor::inspect_target(target)?;
    let shape = report.shape.kind.clone();
    let status = if shape == "non_skill_repository" {
        ChecklistStatus::Blocked
    } else {
        ChecklistStatus::Ready
    };
    let blockers = (shape == "non_skill_repository")
        .then(|| "target is not shaped like a SkillSpec-importable skill source".to_owned())
        .into_iter()
        .collect::<Vec<_>>();
    let steps = shape_steps::doctor_steps(target, &report, stage);
    let next_command = steps
        .first()
        .and_then(|step| step.commands.first())
        .cloned()
        .or_else(|| Some(report.shape.recommended_command.clone()));
    Ok(ChecklistReport {
        schema: CHECKLIST_SCHEMA,
        kind: ChecklistKind::Doctor,
        stage,
        status,
        entity: ChecklistEntity {
            target: target.to_owned(),
            root: Some(report.shape.root.clone()),
            shape: Some(shape.clone()),
            ..ChecklistEntity::default()
        },
        activation_policy: activation_policy_for_doctor_shape(&shape).to_owned(),
        position: ChecklistPosition {
            package_count: Some(report.shape.skill_files.len()),
            remaining_packages: Some(report.shape.skill_files.len()),
            ..ChecklistPosition::default()
        },
        steps,
        forbid: common_forbids_for_shape(&shape),
        next_command,
        blockers,
    })
}

pub fn import_checklist(
    target: &str,
    build_root: Option<&Path>,
    stage: ChecklistStage,
) -> error::Result<ChecklistReport> {
    let target_path = Path::new(target);
    if target_path.is_file() {
        if let Ok(manifest) = workspace::load_manifest(target_path) {
            return workspace_import::workspace_import_checklist(
                target_path,
                &manifest,
                build_root,
                stage,
            );
        }
    }
    source_import_checklist(target, stage)
}

pub fn run_checklist(target: &Path, stage: ChecklistStage) -> error::Result<ChecklistReport> {
    if target.is_dir() {
        let guide_state = target.join("guide-state.json");
        if !guide_state.is_file() {
            return Ok(run::blocked_run_dir_checklist(target, stage));
        }
        let content = fs::read_to_string(&guide_state).map_err(|source| Error::Read {
            path: guide_state.clone(),
            source,
        })?;
        let guide_report: guide::GuideReport =
            serde_json::from_str(&content).map_err(|source| Error::ParseJson {
                path: guide_state,
                source,
            })?;
        return Ok(run::run_guide_checklist(target, &guide_report, stage));
    }
    if target.is_file() {
        return Ok(run::run_spec_checklist(target, stage));
    }
    Err(Error::InvalidInput {
        message: format!(
            "run checklist target must be a skill.spec.yml file or guided run directory: {}",
            target.display()
        ),
    })
}

pub fn render(report: &ChecklistReport) -> String {
    let mut output = String::new();
    output.push_str("SkillSpec checklist\n\n");
    output.push_str(&format!("- kind: {}\n", report.kind.as_str()));
    output.push_str(&format!("- stage: {}\n", report.stage.as_str()));
    output.push_str(&format!("- status: {}\n", report.status.as_str()));
    output.push_str(&format!("- target: {}\n", report.entity.target));
    if let Some(shape) = &report.entity.shape {
        output.push_str(&format!("- shape: {shape}\n"));
    }
    output.push_str(&format!(
        "- activation_policy: {}\n",
        report.activation_policy
    ));
    if let Some(package_id) = &report.position.package_id {
        output.push_str(&format!(
            "- package: {} ({}/{}, remaining {})\n",
            package_id,
            report.position.package_index.unwrap_or(0),
            report.position.package_count.unwrap_or(0),
            report.position.remaining_packages.unwrap_or(0)
        ));
    }
    if let Some(phase) = &report.position.current_phase {
        output.push_str(&format!("- current_phase: {phase}\n"));
    }
    if !report.blockers.is_empty() {
        output.push_str("\nBlockers:\n");
        for blocker in &report.blockers {
            output.push_str(&format!("- {blocker}\n"));
        }
    }
    output.push_str("\nSteps:\n");
    for step in &report.steps {
        output.push_str(&format!("- {}: {}\n", step.id, step.description));
        output.push_str(&format!("  directive: {}\n", step.directive));
        if let Some(until) = &step.repeat.until {
            output.push_str(&format!("  repeat_until: {until}\n"));
        }
        if !step.commands.is_empty() {
            output.push_str("  commands:\n");
            for command in &step.commands {
                output.push_str(&format!("    - {command}\n"));
            }
        }
        if !step.forbid.is_empty() {
            output.push_str("  forbid:\n");
            for item in &step.forbid {
                output.push_str(&format!("    - {item}\n"));
            }
        }
    }
    if !report.forbid.is_empty() {
        output.push_str("\nGlobal forbids:\n");
        for item in &report.forbid {
            output.push_str(&format!("- {item}\n"));
        }
    }
    if let Some(next) = &report.next_command {
        output.push_str(&format!("\nNext command:\n{next}\n"));
    }
    output
}

fn source_import_checklist(target: &str, stage: ChecklistStage) -> error::Result<ChecklistReport> {
    let doctor = doctor::inspect_target(target)?;
    let shape = doctor.shape.kind.clone();
    let mut steps = shape_steps::doctor_steps(target, &doctor, stage);
    steps.push(ChecklistStep {
        id: "preserve_source_and_frontmatter".to_owned(),
        description: "Preserve original source and frontmatter before semantic edits.".to_owned(),
        directive: "Ensure every package keeps source/SKILL_md.old, frontmatter-derived metadata, sibling resources, dependency evidence, and source-map hashes.".to_owned(),
        commands: match shape.as_str() {
            "simple_skill" => vec![
                format!("skillspec source map {} --out <draft>/.skillspec/source-map", shell_arg(target)),
                "skillspec import-skill <source> --out <draft>/skill.spec.yml --source-map <draft>/.skillspec/source-map/source-map.json".to_owned(),
            ],
            "multi_skill_workspace" | "entry_skill_with_subskills" | "plugin_workspace" => vec![
                format!("skillspec workspace map {} --out <build>/skillspec.workspace.yml --summary", shell_arg(target)),
                "skillspec workspace validate <build>/skillspec.workspace.yml --summary".to_owned(),
                "skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build> --summary".to_owned(),
            ],
            _ => Vec::new(),
        },
        repeat: ChecklistRepeat {
            for_each: Some("source_package".to_owned()),
            until: Some("every source package has preserved source, frontmatter, resources, and dependency ledger".to_owned()),
            ..ChecklistRepeat::default()
        },
        requires: vec![
            "source/SKILL_md.old".to_owned(),
            "frontmatter_preserved".to_owned(),
            "deps.toml_initialized".to_owned(),
        ],
        blocks: vec![
            "drop_original_skill_source".to_owned(),
            "drop_source_frontmatter".to_owned(),
            "dependency_ledger_missing".to_owned(),
        ],
        forbid: vec![
            "treat_imported_scaffold_as_finished".to_owned(),
            "install_generated_scaffold".to_owned(),
        ],
        evidence: vec![
            "<package>/source/SKILL_md.old".to_owned(),
            "<package>/deps.toml".to_owned(),
        ],
    });
    Ok(ChecklistReport {
        schema: CHECKLIST_SCHEMA,
        kind: ChecklistKind::Import,
        stage,
        status: if shape == "non_skill_repository" {
            ChecklistStatus::Blocked
        } else {
            ChecklistStatus::Ready
        },
        entity: ChecklistEntity {
            target: target.to_owned(),
            root: Some(doctor.shape.root.clone()),
            shape: Some(shape.clone()),
            ..ChecklistEntity::default()
        },
        activation_policy: activation_policy_for_doctor_shape(&shape).to_owned(),
        position: ChecklistPosition {
            package_count: Some(doctor.shape.skill_files.len()),
            remaining_packages: Some(doctor.shape.skill_files.len()),
            ..ChecklistPosition::default()
        },
        forbid: common_forbids_for_shape(&shape),
        next_command: steps
            .first()
            .and_then(|step| step.commands.first())
            .cloned(),
        blockers: if shape == "non_skill_repository" {
            vec!["target is not shaped like an importable skill source".to_owned()]
        } else {
            Vec::new()
        },
        steps,
    })
}

fn activation_policy_for_doctor_shape(shape: &str) -> &'static str {
    match shape {
        "simple_skill" => "single_activation_skill",
        "plugin_workspace" => "preserve_plugin_activation",
        "multi_skill_workspace" | "entry_skill_with_subskills" => "single_workspace_activation",
        _ => "no_activation",
    }
}

fn activation_policy_for_workspace_shape(
    shape: &workspace::WorkspaceSourceShapeKind,
) -> &'static str {
    match shape {
        workspace::WorkspaceSourceShapeKind::SingleSkill => "single_activation_skill",
        workspace::WorkspaceSourceShapeKind::MultiSkillWorkspace => "single_workspace_activation",
        workspace::WorkspaceSourceShapeKind::PluginWorkspace => "preserve_plugin_activation",
        workspace::WorkspaceSourceShapeKind::Unknown => "unknown_activation",
    }
}

fn workspace_shape_kind(shape: &workspace::WorkspaceSourceShapeKind) -> &'static str {
    match shape {
        workspace::WorkspaceSourceShapeKind::Unknown => "unknown",
        workspace::WorkspaceSourceShapeKind::SingleSkill => "single_skill",
        workspace::WorkspaceSourceShapeKind::MultiSkillWorkspace => "multi_skill_workspace",
        workspace::WorkspaceSourceShapeKind::PluginWorkspace => "plugin_workspace",
    }
}

fn common_forbids_for_shape(shape: &str) -> Vec<String> {
    let mut forbids = common_bulk_forbids();
    forbids.extend([
        "treat_imported_scaffold_as_finished".to_owned(),
        "install_generated_scaffold".to_owned(),
        "drop_source_frontmatter".to_owned(),
        "drop_original_skill_source".to_owned(),
        "leave_conditional_workflows_as_prose".to_owned(),
        "omit_state_machine_or_phase_logic".to_owned(),
        "delete_dependency_mentions_to_pass_validation".to_owned(),
        "claim_full_port_without_source_obligation_coverage".to_owned(),
    ]);
    if shape == "plugin_workspace" || shape == "plugin_shape" {
        forbids.extend([
            "flatten_plugin_shape".to_owned(),
            "drop_plugin_manifest".to_owned(),
            "drop_plugin_namespace".to_owned(),
        ]);
    }
    if shape == "multi_skill_workspace" || shape == "entry_skill_with_subskills" {
        forbids.push("collapse_multi_skill_workspace_into_single_skill".to_owned());
    }
    forbids.sort();
    forbids.dedup();
    forbids
}

fn common_bulk_forbids() -> Vec<String> {
    vec![
        "bulk_rewrite_skill_specs".to_owned(),
        "bulk_promote_scaffolds".to_owned(),
        "generate_all_packages_from_one_template".to_owned(),
        "review_one_representative_package_only".to_owned(),
        "apply_ruby_yaml_generator_across_packages".to_owned(),
        "apply_python_yaml_generator_across_packages".to_owned(),
        "copy_one_package_semantics_to_siblings".to_owned(),
    ]
}

fn package_order(manifest: &workspace::WorkspaceManifest) -> Vec<String> {
    let mut dependents = BTreeMap::<String, BTreeSet<String>>::new();
    let mut remaining_dependency_counts = BTreeMap::<String, usize>::new();
    for package in manifest.packages.values() {
        remaining_dependency_counts.insert(package.package_id.clone(), package.depends_on.len());
        for dependency in &package.depends_on {
            dependents
                .entry(dependency.clone())
                .or_default()
                .insert(package.package_id.clone());
        }
    }
    let mut ready = remaining_dependency_counts
        .iter()
        .filter_map(|(package_id, count)| (*count == 0).then_some(package_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::new();
    while let Some(package_id) = ready.pop_first() {
        order.push(package_id.clone());
        if let Some(children) = dependents.get(&package_id) {
            for child in children {
                if let Some(count) = remaining_dependency_counts.get_mut(child) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
    }
    for package_id in manifest.packages.keys() {
        if !order.iter().any(|item| item == package_id) {
            order.push(package_id.clone());
        }
    }
    order
}

fn output_package_dir(package: &workspace::WorkspacePackage, build_root: &Path) -> Option<PathBuf> {
    let relative = normalized_relative_path(&package.path)?;
    Some(build_root.join(relative))
}

fn normalized_relative_path(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if path.as_os_str().is_empty() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!normalized.as_os_str().is_empty()).then_some(normalized)
}

fn shell_arg_path(path: &Path) -> String {
    shell_arg(&path.display().to_string())
}

fn shell_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':' | '='))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
