use super::commands::{install_skill_all_detected, install_skill_target, write_compiled_loader};
use super::fixture::reviewed_import_fixture;
use skillspec_harness_lab::{assert_success, json_stdout, HarnessLabReportBuilder};
use std::path::PathBuf;

pub fn compiled_loader_installs_into_all_detected_roots(report: &mut HarnessLabReportBuilder) {
    let fixture = reviewed_import_fixture("runtime-install-all");
    let loader = write_compiled_loader(&fixture);
    assert!(loader.contains("skillspec run-loop <skill_dir>/skill.spec.yml"));

    let install = install_skill_all_detected(&fixture, "reviewed-runtime");
    assert_success(&install);
    let install_report = json_stdout(&install);
    assert_eq!(install_report["skill_name"], "reviewed-runtime");
    assert_eq!(install_report["dry_run"], false);
    assert_eq!(install_report["installs"].as_array().unwrap().len(), 3);

    for root in [
        fixture.lab.agents_root(),
        fixture.lab.codex_root(),
        fixture.lab.claude_root(),
    ] {
        assert!(root.join("reviewed-runtime/SKILL.md").is_file());
        assert!(root.join("reviewed-runtime/skill.spec.yml").is_file());
        assert!(root.join("reviewed-runtime/deps.toml").is_file());
        assert!(root.join("reviewed-runtime/source/SKILL_md.old").is_file());
        let installed_loader =
            std::fs::read_to_string(root.join("reviewed-runtime/SKILL.md")).unwrap();
        assert!(installed_loader.contains("skillspec run-loop <skill_dir>/skill.spec.yml"));
    }
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("compiled_loader_installs_into_all_detected_roots");
    case.claim_pass(
        "compile.loader_contains_run_loop",
        true,
        loader.contains("skillspec run-loop <skill_dir>/skill.spec.yml"),
    );
    case.claim_pass("install.exit_success", true, install.status.success());
    case.claim_pass(
        "install.skill_name",
        "reviewed-runtime",
        &install_report["skill_name"],
    );
    case.claim_pass(
        "install.target_count",
        3,
        install_report["installs"].as_array().unwrap().len(),
    );
    for (id, root) in [
        ("agents", fixture.lab.agents_root()),
        ("codex", fixture.lab.codex_root()),
        ("claude-local", fixture.lab.claude_root()),
    ] {
        case.claim_pass(
            format!("install.{id}.loader"),
            true,
            root.join("reviewed-runtime/SKILL.md").is_file(),
        );
        case.claim_pass(
            format!("install.{id}.spec"),
            true,
            root.join("reviewed-runtime/skill.spec.yml").is_file(),
        );
        case.claim_pass(
            format!("install.{id}.preserved_source"),
            true,
            root.join("reviewed-runtime/source/SKILL_md.old").is_file(),
        );
    }
    case.finish();
}

pub fn retire_existing_replaces_prose_skill(report: &mut HarnessLabReportBuilder) {
    let fixture = reviewed_import_fixture("runtime-retire-existing");
    let loader = write_compiled_loader(&fixture);
    assert!(loader.contains("skillspec run-loop <skill_dir>/skill.spec.yml"));

    let install_dir = fixture.lab.agents_root().join("reviewed-runtime");
    fixture
        .lab
        .write_file(&install_dir.join("SKILL.md"), "# Old Skill\n");
    fixture
        .lab
        .write_file(&install_dir.join("stale.txt"), "stale\n");

    let install = install_skill_target(&fixture, "reviewed-runtime", "agents", true);
    assert_success(&install);
    let install_report = json_stdout(&install);
    assert_eq!(install_report["installs"][0]["status"], "installed");
    assert_eq!(install_report["installs"][0]["retired_existing"], true);
    let backup_path = PathBuf::from(
        install_report["installs"][0]["backup_path"]
            .as_str()
            .unwrap(),
    );
    assert!(backup_path.join("SKILL.md").is_file());
    assert!(backup_path.join("stale.txt").is_file());
    assert_eq!(
        std::fs::read_to_string(backup_path.join("SKILL.md")).unwrap(),
        "# Old Skill\n"
    );
    assert!(!install_dir.join("stale.txt").exists());
    let installed_loader = std::fs::read_to_string(install_dir.join("SKILL.md")).unwrap();
    assert!(installed_loader.contains("skillspec run-loop <skill_dir>/skill.spec.yml"));
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("retire_existing_replaces_prose_skill");
    case.claim_pass("install.exit_success", true, install.status.success());
    case.claim_pass(
        "install.status",
        "installed",
        &install_report["installs"][0]["status"],
    );
    case.claim_pass(
        "install.retired_existing",
        true,
        &install_report["installs"][0]["retired_existing"],
    );
    case.claim_pass(
        "install.backup_skill_md",
        true,
        backup_path.join("SKILL.md").is_file(),
    );
    case.claim_pass(
        "install.backup_stale_file",
        true,
        backup_path.join("stale.txt").is_file(),
    );
    case.claim_pass(
        "install.stale_removed",
        false,
        install_dir.join("stale.txt").exists(),
    );
    case.claim_pass(
        "install.loader_replaced",
        true,
        installed_loader.contains("skillspec run-loop <skill_dir>/skill.spec.yml"),
    );
    case.finish();
}
