use super::*;

pub(super) fn doctor_steps(
    target: &str,
    report: &doctor::DoctorReport,
    stage: ChecklistStage,
) -> Vec<ChecklistStep> {
    let shape = report.shape.kind.as_str();
    match (shape, stage) {
        ("simple_skill", ChecklistStage::Entry) => vec![single_skill_entry_step(target)],
        ("simple_skill", ChecklistStage::Loop) => vec![single_skill_loop_step(target)],
        ("simple_skill", ChecklistStage::Exit) => vec![single_skill_exit_step()],
        ("plugin_workspace", ChecklistStage::Entry) => vec![plugin_entry_step(target)],
        ("plugin_workspace", ChecklistStage::Loop) => vec![plugin_loop_step()],
        ("plugin_workspace", ChecklistStage::Exit) => vec![plugin_exit_step()],
        ("multi_skill_workspace" | "entry_skill_with_subskills", ChecklistStage::Entry) => {
            vec![multi_skill_entry_step(target)]
        }
        ("multi_skill_workspace" | "entry_skill_with_subskills", ChecklistStage::Loop) => {
            vec![multi_skill_loop_step()]
        }
        ("multi_skill_workspace" | "entry_skill_with_subskills", ChecklistStage::Exit) => {
            vec![multi_skill_exit_step()]
        }
        _ => vec![ChecklistStep {
            id: "unsupported_source_shape".to_owned(),
            description: format!("Source shape {shape:?} is not importable as a SkillSpec skill."),
            directive: "Stop import planning and choose a real skill source folder, multi-skill workspace, or plugin-shaped workspace.".to_owned(),
            commands: Vec::new(),
            repeat: ChecklistRepeat {
                until: Some("an importable source shape is selected".to_owned()),
                ..ChecklistRepeat::default()
            },
            requires: Vec::new(),
            blocks: vec!["non_skill_repository".to_owned()],
            forbid: vec!["import_non_skill_repository_as_skill".to_owned()],
            evidence: vec![report.shape.root.clone()],
        }],
    }
}

fn single_skill_entry_step(target: &str) -> ChecklistStep {
    ChecklistStep {
        id: "single_skill_entry".to_owned(),
        description: "Confirm the selected folder is one atomic source skill.".to_owned(),
        directive: "Run doctor and source mapping before import. If more than one SKILL.md or a plugin marker is discovered, switch to the matching workspace template.".to_owned(),
        commands: vec![
            format!("skillspec doctor {} --json", shell_arg(target)),
            format!(
                "skillspec source map {} --out <draft>/.skillspec/source-map",
                shell_arg(target)
            ),
            "skillspec source coverage <draft>/.skillspec/source-map/source-map.json".to_owned(),
            format!(
                "skillspec source stale <draft>/.skillspec/source-map/source-map.json --root {}",
                shell_arg(target)
            ),
        ],
        repeat: ChecklistRepeat {
            for_each: Some("discovered_skill_file".to_owned()),
            until: Some("exactly_one_skill_file_verified".to_owned()),
            ..ChecklistRepeat::default()
        },
        requires: vec!["one_SKILL_md".to_owned(), "source_map_fresh".to_owned()],
        blocks: vec![
            "multiple_skill_files".to_owned(),
            "plugin_marker_detected".to_owned(),
            "source_map_stale".to_owned(),
        ],
        forbid: vec!["import_workspace_root_as_single_skill".to_owned()],
        evidence: vec!["<draft>/.skillspec/source-map/source-map.json".to_owned()],
    }
}

fn single_skill_loop_step(target: &str) -> ChecklistStep {
    ChecklistStep {
        id: "single_skill_source_loop".to_owned(),
        description: "Review each source block for the one package.".to_owned(),
        directive: "Use source lens one cursor at a time. Port each block into structural SkillSpec constructs and validate after each meaningful edit.".to_owned(),
        commands: vec![
            "skillspec source lens <draft>/.skillspec/source-map/source-map.json --cursor <n>".to_owned(),
            "skillspec validate <draft>/skill.spec.yml".to_owned(),
            "skillspec deps check <draft>/skill.spec.yml".to_owned(),
            "skillspec test <draft>/skill.spec.yml".to_owned(),
        ],
        repeat: ChecklistRepeat {
            for_each: Some(target.to_owned()),
            inner: Some("source_lens_block".to_owned()),
            until: Some("all source blocks are represented structurally or explicitly marked review_required".to_owned()),
            cursor_field: Some("next_cursor".to_owned()),
            ..ChecklistRepeat::default()
        },
        requires: vec![
            "activation_and_routes_source_backed".to_owned(),
            "conditional_rules_preserved".to_owned(),
            "dependencies_typed_and_reviewed".to_owned(),
            "tests_cover_source_promises".to_owned(),
        ],
        blocks: vec![
            "thin_or_placeholder_scaffold".to_owned(),
            "conditional_rule_left_as_prose".to_owned(),
            "unreviewed_dependency_ledger".to_owned(),
        ],
        forbid: common_bulk_forbids(),
        evidence: vec![
            "<draft>/skill.spec.yml".to_owned(),
            "<draft>/deps.toml".to_owned(),
        ],
    }
}

fn single_skill_exit_step() -> ChecklistStep {
    ChecklistStep {
        id: "single_skill_exit".to_owned(),
        description: "Compile and dry-run install only after review is complete.".to_owned(),
        directive: "Run validation, imports, dependency, tests, compile, and install dry-run before any harness mutation.".to_owned(),
        commands: vec![
            "skillspec imports check <draft>/skill.spec.yml".to_owned(),
            "skillspec deps check <draft>/skill.spec.yml".to_owned(),
            "skillspec test <draft>/skill.spec.yml".to_owned(),
            "skillspec compile <draft>/skill.spec.yml --target codex-skill --out <draft>/compiled".to_owned(),
            "skillspec install skill <draft>/compiled --target codex --dry-run --retire-existing".to_owned(),
        ],
        repeat: ChecklistRepeat {
            until: Some("qa_and_dry_run_pass_without_scaffold_or_dependency_blockers".to_owned()),
            ..ChecklistRepeat::default()
        },
        requires: vec![
            "schema_valid".to_owned(),
            "imports_valid".to_owned(),
            "deps_reviewed".to_owned(),
            "tests_pass".to_owned(),
            "dry_run_reviewed".to_owned(),
        ],
        blocks: vec![
            "validation_failed".to_owned(),
            "install_collision_without_retirement_approval".to_owned(),
        ],
        forbid: vec![
            "install_without_dry_run".to_owned(),
            "install_generated_scaffold".to_owned(),
        ],
        evidence: vec![
            "<draft>/compiled/SKILL.md".to_owned(),
            "install dry-run report".to_owned(),
        ],
    }
}

fn multi_skill_entry_step(target: &str) -> ChecklistStep {
    workspace_entry_step(
        "multi_skill_entry",
        "Map the parent folder as a workspace before importing.",
        target,
        "preserve every source folder and create one activation entry package plus support packages as needed",
    )
}

fn multi_skill_loop_step() -> ChecklistStep {
    ChecklistStep {
        id: "multi_skill_workspace_loop".to_owned(),
        description: "Promote each workspace package independently.".to_owned(),
        directive: "Process packages in manifest order or dependency-ready batches. For each package, review its own source/SKILL_md.old and source lens blocks before writing promotion proof.".to_owned(),
        commands: vec![
            "skillspec import checklist <build>/skillspec.workspace.yml --build-root <workspace-build> --stage loop --json".to_owned(),
        ],
        repeat: ChecklistRepeat {
            outer: Some("for_each_package_in_manifest_order".to_owned()),
            inner: Some("for_each_source_lens_block".to_owned()),
            until: Some("every package has workspace-promotion proof and no scaffold blockers".to_owned()),
            ..ChecklistRepeat::default()
        },
        requires: vec!["package_local_review".to_owned(), "workspace_promotion_proof".to_owned()],
        blocks: vec!["bulk_generated_promotion".to_owned(), "missing_package_review".to_owned()],
        forbid: common_bulk_forbids(),
        evidence: vec!["<workspace-build>/<package>/.skillspec/workspace-promotion.json".to_owned()],
    }
}

fn multi_skill_exit_step() -> ChecklistStep {
    workspace_exit_step("multi_skill_workspace_exit")
}

fn plugin_entry_step(target: &str) -> ChecklistStep {
    workspace_entry_step(
        "plugin_entry",
        "Map plugin-shaped source without flattening plugin parent folders.",
        target,
        "preserve plugin metadata, shared files, parent folders, frontmatter, and skills/ relative paths",
    )
}

fn plugin_loop_step() -> ChecklistStep {
    ChecklistStep {
        id: "plugin_workspace_loop".to_owned(),
        description: "Promote plugin packages while preserving namespace and folder shape.".to_owned(),
        directive: "Review one plugin package at a time. Keep package-local proof, do not copy old SKILL.md as the install result, and do not flatten plugin paths into top-level skill folders.".to_owned(),
        commands: vec![
            "skillspec import checklist <build>/skillspec.workspace.yml --build-root <workspace-build> --stage loop --json".to_owned(),
        ],
        repeat: ChecklistRepeat {
            outer: Some("for_each_plugin_package_in_manifest_order".to_owned()),
            inner: Some("for_each_source_lens_block".to_owned()),
            until: Some("all plugin packages are semantically promoted and shape-preserving install proof exists".to_owned()),
            ..ChecklistRepeat::default()
        },
        requires: vec![
            "plugin_namespace_preserved".to_owned(),
            "plugin_parent_shape_preserved".to_owned(),
            "package_promotion_proof".to_owned(),
        ],
        blocks: vec!["flattened_plugin_install".to_owned(), "raw_skill_copy_substitution".to_owned()],
        forbid: common_bulk_forbids(),
        evidence: vec![
            "skillspec.workspace.yml".to_owned(),
            "workspace-install.manifest.json".to_owned(),
        ],
    }
}

fn plugin_exit_step() -> ChecklistStep {
    workspace_exit_step("plugin_workspace_exit")
}

fn workspace_entry_step(
    id: &str,
    description: &str,
    target: &str,
    activation_policy: &str,
) -> ChecklistStep {
    ChecklistStep {
        id: id.to_owned(),
        description: description.to_owned(),
        directive: format!(
            "Run workspace map and validate before import; activation policy is to {activation_policy}."
        ),
        commands: vec![
            format!(
                "skillspec workspace map {} --out <build>/skillspec.workspace.yml --summary",
                shell_arg(target)
            ),
            "skillspec workspace validate <build>/skillspec.workspace.yml --summary".to_owned(),
            "skillspec workspace import <build>/skillspec.workspace.yml --out <workspace-build> --summary".to_owned(),
        ],
        repeat: ChecklistRepeat {
            for_each: Some("discovered_SKILL_md".to_owned()),
            until: Some("manifest package count equals doctor skill file count and shape is preserved".to_owned()),
            ..ChecklistRepeat::default()
        },
        requires: vec![
            "workspace_shape_confirmed".to_owned(),
            "package_paths_preserved".to_owned(),
            "frontmatter_names_preserved".to_owned(),
            "source_map_per_package".to_owned(),
        ],
        blocks: vec![
            "representative_subset_import".to_owned(),
            "flattened_install_slug_for_workspace".to_owned(),
            "unresolved_cross_skill_reference".to_owned(),
        ],
        forbid: vec![
            "run_import_skill_on_parent_folder".to_owned(),
            "use_bulk_yaml_rewrite".to_owned(),
            "install_scaffolded_drafts".to_owned(),
        ],
        evidence: vec![
            "<build>/skillspec.workspace.yml".to_owned(),
            "<workspace-build>/workspace-import.report.md".to_owned(),
        ],
    }
}

fn workspace_exit_step(id: &str) -> ChecklistStep {
    ChecklistStep {
        id: id.to_owned(),
        description: "Finish workspace only after every package is promoted.".to_owned(),
        directive: "Run converge, compile, dry-run install, and approved install. Treat scaffold blockers as the next package-loop work, not as final response permission.".to_owned(),
        commands: vec![
            "skillspec workspace converge <build>/skillspec.workspace.yml --build-root <workspace-build> --summary".to_owned(),
            "skillspec workspace compile <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex-skill --summary".to_owned(),
            "skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --dry-run --summary".to_owned(),
            "skillspec workspace install <build>/skillspec.workspace.yml --build-root <workspace-build> --target codex --retire-existing --summary".to_owned(),
        ],
        repeat: ChecklistRepeat {
            until: Some("all installable packages are compiled and dry-run plus install reports pass".to_owned()),
            ..ChecklistRepeat::default()
        },
        requires: vec![
            "workspace_converge_passed".to_owned(),
            "workspace_compile_passed".to_owned(),
            "dry_run_reviewed".to_owned(),
            "retirement_approved_if_needed".to_owned(),
        ],
        blocks: vec![
            "unpromoted_scaffold_package".to_owned(),
            "missing_compiled_loader".to_owned(),
            "install_collision_without_approval".to_owned(),
        ],
        forbid: vec![
            "install_without_compile".to_owned(),
            "install_without_dry_run".to_owned(),
            "copy_raw_skills_as_substitute".to_owned(),
        ],
        evidence: vec![
            "<workspace-build>/workspace-converge.report.md".to_owned(),
            "<workspace-build>/workspace-compile.report.md".to_owned(),
            "<workspace-build>/workspace-install.manifest.json".to_owned(),
        ],
    }
}
