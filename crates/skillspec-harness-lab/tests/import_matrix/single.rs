use super::helpers::{
    deps_check_json, import_skill_output, import_skill_with_source_map_output, importable_skill_md,
    read_file, source_map_output, source_mapped_skill_md, validate_output,
};
use skillspec_harness_lab::{
    assert_failure, assert_success, json_stdout, stderr, stdout, HarnessLab,
    HarnessLabReportBuilder,
};

pub fn import_skill_rejects_missing_path(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("import-missing-path");
    let source = lab.root().join("missing-skill");
    let out = lab.root().join("draft").join("skill.spec.yml");

    let output = import_skill_output(&lab, &source, &out);
    assert_failure(&output);
    let stderr = stderr(&output);
    assert!(stderr.contains("No such file") || stderr.contains("failed to read"));
    assert!(!out.exists());
    lab.assert_no_real_home_writes();

    let mut case = report.case("import_skill_rejects_missing_path");
    case.claim_pass("import.exit_success", false, output.status.success());
    case.claim_pass(
        "import.stderr.missing_path",
        true,
        stderr.contains("No such file") || stderr.contains("failed to read"),
    );
    case.claim_pass("import.out_absent", true, !out.exists());
    case.finish();
}

pub fn import_skill_imports_single_skill_folder(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("import-single-folder");
    let source_root = lab.root().join("source");
    let skill_dir = lab.write_skill(&source_root, "review-skill", &importable_skill_md(), None);
    lab.write_file(
        &skill_dir.join("reference.md"),
        "# Reference\n\nNever skip referenced local files.\n",
    );
    let out = lab.root().join("draft").join("skill.spec.yml");

    let output = import_skill_output(&lab, &skill_dir, &out);
    assert_success(&output);
    let import_stdout = stdout(&output);
    assert!(import_stdout.contains("review note"));
    assert!(import_stdout.contains("deps ledger: wrote deps.toml"));
    assert!(out.is_file());
    assert!(out.parent().unwrap().join("source/SKILL_md.old").is_file());
    assert!(!out.parent().unwrap().join("source/SKILL.md").exists());
    assert!(skill_dir.join("SKILL.md").is_file());

    let spec = read_file(&out);
    assert!(spec.contains("review_required"));
    assert!(spec.contains("imports:"));
    assert!(spec.contains("dependency_ledger"));
    assert!(spec.contains("path: source/SKILL_md.old"));
    assert_success(&validate_output(&lab, &out));
    let deps = deps_check_json(&lab, &out);
    assert!(deps["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| dependency["id"] == "dependency_ledger"
            && dependency["status"] == "present"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("import_skill_imports_single_skill_folder");
    case.claim_pass("import.exit_success", true, output.status.success());
    case.claim_pass("import.out_file", true, out.is_file());
    case.claim_pass(
        "import.preserved_source",
        true,
        out.parent().unwrap().join("source/SKILL_md.old").is_file(),
    );
    case.claim_pass(
        "import.no_discoverable_source_skill",
        false,
        out.parent().unwrap().join("source/SKILL.md").exists(),
    );
    case.claim_pass(
        "import.source_unchanged",
        true,
        skill_dir.join("SKILL.md").is_file(),
    );
    case.claim_pass(
        "import.spec.review_required",
        true,
        spec.contains("review_required"),
    );
    case.claim_pass("import.spec.imports", true, spec.contains("imports:"));
    case.claim_pass(
        "import.deps.ledger_present",
        true,
        deps["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|dependency| {
                dependency["id"] == "dependency_ledger" && dependency["status"] == "present"
            }),
    );
    case.finish();
}

pub fn import_skill_imports_direct_skill_md_file(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("import-direct-skill-md");
    let source_root = lab.root().join("source");
    let skill_dir = lab.write_skill(&source_root, "direct-skill", &importable_skill_md(), None);
    let source = skill_dir.join("SKILL.md");
    let out = lab.root().join("draft").join("skill.spec.yml");

    let output = import_skill_output(&lab, &source, &out);
    assert_success(&output);
    assert!(out.is_file());
    assert_success(&validate_output(&lab, &out));
    let spec = read_file(&out);
    assert!(spec.contains("source_kind: file"));
    assert!(spec.contains("path: source/SKILL_md.old"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("import_skill_imports_direct_skill_md_file");
    case.claim_pass("import.exit_success", true, output.status.success());
    case.claim_pass("import.out_file", true, out.is_file());
    case.claim_pass(
        "import.spec.source_kind_file",
        true,
        spec.contains("source_kind: file"),
    );
    case.claim_pass(
        "import.preserved_source",
        true,
        out.parent().unwrap().join("source/SKILL_md.old").is_file(),
    );
    case.finish();
}

pub fn import_skill_imports_direct_markdown_file_as_file_source(
    report: &mut HarnessLabReportBuilder,
) {
    let lab = HarnessLab::new("import-direct-markdown-file");
    let source = lab.root().join("README.md");
    lab.write_file(
        &source,
        "# Standalone Markdown Skill Draft\n\nAlways review before trusting this draft.\n",
    );
    let out = lab.root().join("draft").join("skill.spec.yml");

    let output = import_skill_output(&lab, &source, &out);
    assert_success(&output);
    assert!(out.is_file());
    assert_success(&validate_output(&lab, &out));
    let spec = read_file(&out);
    assert!(spec.contains("source_kind: file"));
    assert!(!out.parent().unwrap().join("source/SKILL_md.old").exists());
    lab.assert_no_real_home_writes();

    let mut case = report.case("import_skill_imports_direct_markdown_file_as_file_source");
    case.claim_pass("import.exit_success", true, output.status.success());
    case.claim_pass("import.out_file", true, out.is_file());
    case.claim_pass(
        "import.spec.source_kind_file",
        true,
        spec.contains("source_kind: file"),
    );
    case.claim_pass(
        "import.no_preserved_skill_source",
        false,
        out.parent().unwrap().join("source/SKILL_md.old").exists(),
    );
    case.finish();
}

pub fn import_skill_scaffolds_empty_skill_for_review(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("import-empty-skill");
    let source = lab.root().join("empty-skill");
    lab.write_file(&source.join("SKILL.md"), "");
    let out = lab.root().join("draft").join("skill.spec.yml");

    let output = import_skill_output(&lab, &source, &out);
    assert_success(&output);
    assert!(out.is_file());
    assert_success(&validate_output(&lab, &out));
    let spec = read_file(&out);
    assert!(spec.contains("review_required"));
    assert!(spec.contains("dependency_ledger"));
    assert!(spec.contains("path: source/SKILL_md.old"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("import_skill_scaffolds_empty_skill_for_review");
    case.claim_pass("import.exit_success", true, output.status.success());
    case.claim_pass("import.out_file", true, out.is_file());
    case.claim_pass(
        "import.spec.review_required",
        true,
        spec.contains("review_required"),
    );
    case.claim_pass(
        "import.spec.dependency_ledger",
        true,
        spec.contains("dependency_ledger"),
    );
    case.finish();
}

pub fn import_skill_scaffolds_malformed_frontmatter_for_review(
    report: &mut HarnessLabReportBuilder,
) {
    let lab = HarnessLab::new("import-malformed-frontmatter");
    let source = lab.root().join("bad-frontmatter");
    lab.write_file(
        &source.join("SKILL.md"),
        "---\nname: bad\ndescription: Bad: unquoted colon\n---\n# Bad\n\nAlways review this draft.\n",
    );
    let out = lab.root().join("draft").join("skill.spec.yml");

    let output = import_skill_output(&lab, &source, &out);
    assert_success(&output);
    assert!(out.is_file());
    assert_success(&validate_output(&lab, &out));
    let spec = read_file(&out);
    assert!(spec.contains("title: Bad"));
    assert!(spec.contains("review_required"));
    lab.assert_no_real_home_writes();

    let mut case = report.case("import_skill_scaffolds_malformed_frontmatter_for_review");
    case.claim_pass("import.exit_success", true, output.status.success());
    case.claim_pass("import.out_file", true, out.is_file());
    case.claim_pass(
        "import.spec.title_from_heading",
        true,
        spec.contains("title: Bad"),
    );
    case.claim_pass(
        "import.spec.review_required",
        true,
        spec.contains("review_required"),
    );
    case.finish();
}

pub fn import_skill_rejects_parent_folder_with_multiple_skills(
    report: &mut HarnessLabReportBuilder,
) {
    let lab = HarnessLab::new("import-multi-skill-reject");
    let source = lab.root().join("skills");
    lab.write_file(
        &source.join("pdf").join("SKILL.md"),
        "---\nname: pdf\ndescription: PDF skill.\n---\n# PDF\n",
    );
    lab.write_file(
        &source.join("csv").join("SKILL.md"),
        "---\nname: csv\ndescription: CSV skill.\n---\n# CSV\n",
    );
    let out = lab.root().join("draft").join("skill.spec.yml");

    let output = import_skill_output(&lab, &source, &out);
    assert_failure(&output);
    let stderr = stderr(&output);
    assert!(stderr.contains("expects one atomic skill package"));
    assert!(stderr.contains("skillspec workspace map"));
    assert!(!out.exists());
    lab.assert_no_real_home_writes();

    let mut case = report.case("import_skill_rejects_parent_folder_with_multiple_skills");
    case.claim_pass("import.exit_success", false, output.status.success());
    case.claim_pass(
        "import.stderr.atomic_package",
        true,
        stderr.contains("expects one atomic skill package"),
    );
    case.claim_pass(
        "import.stderr.workspace_map_guidance",
        true,
        stderr.contains("skillspec workspace map"),
    );
    case.claim_pass("import.out_absent", true, !out.exists());
    case.finish();
}

pub fn import_skill_rejects_stale_source_map(report: &mut HarnessLabReportBuilder) {
    let lab = HarnessLab::new("import-stale-source-map");
    let source_root = lab.root().join("source");
    let skill_dir = lab.write_skill(
        &source_root,
        "mapped-skill",
        &source_mapped_skill_md("Always inspect dependencies before proof."),
        None,
    );
    let map_dir = lab.root().join("source-map");
    let map = source_map_output(&lab, &skill_dir, &map_dir);
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["files"], 1);
    let map_path = map_dir.join("source-map.json");
    assert!(map_path.is_file());

    lab.write_file(
        &skill_dir.join("SKILL.md"),
        &source_mapped_skill_md("Changed after source map."),
    );
    let out = lab.root().join("draft").join("skill.spec.yml");
    let output = import_skill_with_source_map_output(&lab, &skill_dir, &out, &map_path);
    assert_failure(&output);
    let stderr = stderr(&output);
    assert!(stderr.contains("source map"));
    assert!(stderr.contains("stale"));
    assert!(!out.exists());
    lab.assert_no_real_home_writes();

    let mut case = report.case("import_skill_rejects_stale_source_map");
    case.claim_pass("source_map.exit_success", true, map.status.success());
    case.claim_pass("source_map.files", 1, &map_report["files"]);
    case.claim_pass("import.exit_success", false, output.status.success());
    case.claim_pass(
        "import.stderr.source_map",
        true,
        stderr.contains("source map"),
    );
    case.claim_pass("import.stderr.stale", true, stderr.contains("stale"));
    case.claim_pass("import.out_absent", true, !out.exists());
    case.finish();
}
