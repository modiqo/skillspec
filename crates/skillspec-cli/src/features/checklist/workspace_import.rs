use super::*;

#[derive(Clone, Debug)]
struct CurrentPackage {
    package_id: String,
    index: usize,
    output_dir: PathBuf,
    source_map: PathBuf,
    cursor: Option<usize>,
    remaining_source_blocks: Option<usize>,
}

pub(super) fn workspace_import_checklist(
    manifest_path: &Path,
    manifest: &workspace::WorkspaceManifest,
    build_root: Option<&Path>,
    stage: ChecklistStage,
) -> error::Result<ChecklistReport> {
    let shape = workspace_shape_kind(&manifest.source_shape.kind).to_owned();
    let order = package_order(manifest);
    let current = build_root.and_then(|root| current_package(manifest, root, &order));
    let package_count = manifest.packages.len();
    let mut blockers = Vec::new();
    if matches!(stage, ChecklistStage::Loop | ChecklistStage::Exit) && build_root.is_none() {
        blockers.push("--build-root is required for loop or exit import checklists".to_owned());
    }
    let status = if !blockers.is_empty() {
        ChecklistStatus::Blocked
    } else if matches!(stage, ChecklistStage::Loop) && build_root.is_some() && current.is_none() {
        ChecklistStatus::Complete
    } else {
        ChecklistStatus::Ready
    };
    let steps =
        workspace_import_steps(manifest_path, manifest, build_root, stage, current.as_ref());
    Ok(ChecklistReport {
        schema: CHECKLIST_SCHEMA,
        kind: ChecklistKind::Import,
        stage,
        status,
        entity: ChecklistEntity {
            target: manifest_path.display().to_string(),
            root: Some(manifest.source_root.clone()),
            shape: Some(shape.clone()),
            manifest: Some(manifest_path.display().to_string()),
            build_root: build_root.map(|path| path.display().to_string()),
            ..ChecklistEntity::default()
        },
        activation_policy: activation_policy_for_workspace_shape(&manifest.source_shape.kind)
            .to_owned(),
        position: ChecklistPosition {
            package_index: current.as_ref().map(|current| current.index),
            package_count: Some(package_count),
            remaining_packages: current
                .as_ref()
                .map(|current| package_count.saturating_sub(current.index)),
            package_id: current.as_ref().map(|current| current.package_id.clone()),
            cursor: current.as_ref().and_then(|current| current.cursor),
            remaining_source_blocks: current
                .as_ref()
                .and_then(|current| current.remaining_source_blocks),
            ..ChecklistPosition::default()
        },
        forbid: common_forbids_for_shape(&shape),
        next_command: steps
            .first()
            .and_then(|step| step.commands.first())
            .cloned(),
        blockers,
        steps,
    })
}

fn current_package(
    manifest: &workspace::WorkspaceManifest,
    build_root: &Path,
    order: &[String],
) -> Option<CurrentPackage> {
    for (offset, package_id) in order.iter().enumerate() {
        let package = manifest.packages.get(package_id)?;
        let output_dir = output_package_dir(package, build_root)?;
        let promotion = output_dir.join(".skillspec/workspace-promotion.json");
        if promotion.is_file() {
            continue;
        }
        let source_map_path = output_dir.join(".skillspec/source-map/source-map.json");
        let lens = source_map::lens(&source_map_path, 1, 1).ok();
        return Some(CurrentPackage {
            package_id: package_id.clone(),
            index: offset + 1,
            output_dir,
            source_map: source_map_path,
            cursor: lens.as_ref().map(|lens| lens.cursor),
            remaining_source_blocks: lens.as_ref().map(|lens| lens.total),
        });
    }
    None
}

fn workspace_import_steps(
    manifest_path: &Path,
    _manifest: &workspace::WorkspaceManifest,
    build_root: Option<&Path>,
    stage: ChecklistStage,
    current: Option<&CurrentPackage>,
) -> Vec<ChecklistStep> {
    let manifest_arg = shell_arg_path(manifest_path);
    match stage {
        ChecklistStage::Entry => vec![ChecklistStep {
            id: "workspace_import_entry".to_owned(),
            description: "Validate workspace shape before fanout import.".to_owned(),
            directive: "Confirm every SKILL.md package is represented exactly once with preserved path, frontmatter-derived name, namespace, dependency edges, and activation policy before importing generated drafts.".to_owned(),
            commands: vec![
                format!("skillspec workspace validate {manifest_arg} --summary"),
                format!("skillspec workspace import {manifest_arg} --out <workspace-build> --summary"),
            ],
            repeat: ChecklistRepeat {
                for_each: Some("workspace_manifest_package".to_owned()),
                until: Some("every manifest package has a package-local draft output".to_owned()),
                ..ChecklistRepeat::default()
            },
            requires: vec![
                "manifest_package_paths_unique".to_owned(),
                "frontmatter_names_preserved".to_owned(),
                "cross_skill_references_classified".to_owned(),
            ],
            blocks: vec![
                "missing_package_for_skill_file".to_owned(),
                "duplicate_install_slug".to_owned(),
                "uncovered_cross_skill_reference".to_owned(),
            ],
            forbid: vec![
                "import_workspace_root_as_single_skill".to_owned(),
                "use_port_one_shot_for_workspace_root".to_owned(),
                "flatten_plugin_shape".to_owned(),
            ],
            evidence: vec![manifest_path.display().to_string()],
        }],
        ChecklistStage::Loop => {
            let Some(current) = current else {
                return vec![ChecklistStep {
                    id: "workspace_package_loop_complete_or_blocked".to_owned(),
                    description: "No unpromoted package was found in the build root.".to_owned(),
                    directive: "If all packages have current workspace-promotion proof, continue to the exit checklist. If a package is failed or missing, repair only that package and rerun this loop. Ask the user only for approvals, inaccessible source, credentials, or policy waivers.".to_owned(),
                    commands: build_root
                        .map(|root| {
                            vec![format!(
                                "skillspec import checklist {} --build-root {} --stage exit",
                                manifest_arg,
                                shell_arg_path(root)
                            )]
                        })
                        .unwrap_or_default(),
                    repeat: ChecklistRepeat {
                        until: Some("all packages have workspace-promotion proof or a real blocker is reported".to_owned()),
                        ..ChecklistRepeat::default()
                    },
                    requires: Vec::new(),
                    blocks: vec!["missing_build_root".to_owned()],
                    forbid: common_bulk_forbids(),
                    evidence: Vec::new(),
                }];
            };
            vec![ChecklistStep {
                id: "promote_workspace_package".to_owned(),
                description: format!(
                    "Promote package {} from its own source map and source blocks.",
                    current.package_id
                ),
                directive: "Review one source lens block at a time. Convert conditional language into rules or route phases, state-machine language into states/phases/closures, dependencies into deps.toml, durable files into resources/code, and workflow promises into tests or closure checks. Validate the package before advancing.".to_owned(),
                commands: vec![
                    format!(
                        "skillspec source lens {} --cursor {}",
                        shell_arg_path(&current.source_map),
                        current.cursor.unwrap_or(1)
                    ),
                    format!(
                        "skillspec validate {}",
                        shell_arg_path(&current.output_dir.join("skill.spec.yml"))
                    ),
                    format!(
                        "skillspec deps check {}",
                        shell_arg_path(&current.output_dir.join("skill.spec.yml"))
                    ),
                    format!(
                        "skillspec test {}",
                        shell_arg_path(&current.output_dir.join("skill.spec.yml"))
                    ),
                ],
                repeat: ChecklistRepeat {
                    outer: Some("for_each_package_in_manifest_order".to_owned()),
                    inner: Some("for_each_source_lens_block".to_owned()),
                    until: Some("all package source blocks have compatible source-obligation coverage and workspace-promotion proof".to_owned()),
                    cursor_field: Some("next_cursor".to_owned()),
                    ..ChecklistRepeat::default()
                },
                requires: vec![
                    "source_hash_recorded".to_owned(),
                    "target_kind_matches_obligation".to_owned(),
                    "dependency_ledger_reviewed".to_owned(),
                    "workspace_promotion_proof_written".to_owned(),
                ],
                blocks: vec![
                    "unmapped_source_obligation".to_owned(),
                    "conditional_left_as_prose".to_owned(),
                    "state_machine_left_as_summary".to_owned(),
                    "dependency_ledger_unreviewed".to_owned(),
                ],
                forbid: common_bulk_forbids(),
                evidence: vec![
                    current
                        .output_dir
                        .join(".skillspec/workspace-promotion.json")
                        .display()
                        .to_string(),
                    current.output_dir.join("deps.toml").display().to_string(),
                ],
            }]
        }
        ChecklistStage::Exit => {
            let build_arg = build_root
                .map(shell_arg_path)
                .unwrap_or_else(|| "<workspace-build>".to_owned());
            vec![ChecklistStep {
                id: "workspace_import_exit".to_owned(),
                description: "Converge, compile, dry-run install, and install only reviewed packages.".to_owned(),
                directive: "Run the workspace gates in order. If a scaffold or missing promotion proof appears, return to `skillspec import checklist ... --stage loop` and keep promoting packages. Ask the user only for dependency approval, retirement approval, inaccessible source, credentials, policy waivers, or other external intervention.".to_owned(),
                commands: vec![
                    format!(
                        "skillspec workspace converge {manifest_arg} --build-root {build_arg} --summary"
                    ),
                    format!(
                        "skillspec workspace compile {manifest_arg} --build-root {build_arg} --target codex-skill --summary"
                    ),
                    format!(
                        "skillspec workspace install {manifest_arg} --build-root {build_arg} --target codex --retire-existing --dry-run --summary"
                    ),
                    format!(
                        "skillspec workspace install {manifest_arg} --build-root {build_arg} --target codex --retire-existing --summary"
                    ),
                ],
                repeat: ChecklistRepeat {
                    until: Some("converge, compile, dry-run install, approved retirement, and install reports pass without scaffold blockers".to_owned()),
                    ..ChecklistRepeat::default()
                },
                requires: vec![
                    "workspace_converge_report".to_owned(),
                    "workspace_compile_report".to_owned(),
                    "workspace_install_dry_run_report".to_owned(),
                    "retire_existing_approved_for_replacements".to_owned(),
                ],
                blocks: vec![
                    "unpromoted_package_scaffold".to_owned(),
                    "missing_workspace_promotion_proof".to_owned(),
                    "dependency_not_ready".to_owned(),
                    "install_target_collision_without_retirement_approval".to_owned(),
                ],
                forbid: vec![
                    "install_generated_scaffold".to_owned(),
                    "install_without_workspace_dry_run".to_owned(),
                    "refresh_router_during_workspace_install".to_owned(),
                ],
                evidence: vec![
                    build_root
                        .map(|root| root.join("workspace-converge.report.md").display().to_string())
                        .unwrap_or_else(|| "<workspace-build>/workspace-converge.report.md".to_owned()),
                    build_root
                        .map(|root| root.join("workspace-install.manifest.json").display().to_string())
                        .unwrap_or_else(|| "<workspace-build>/workspace-install.manifest.json".to_owned()),
                ],
            }]
        }
    }
}
