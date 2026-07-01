use super::commands::{
    assert_failed_with, delete_json, delete_output, file_contains, install_agents_json,
    update_json, update_output,
};
use super::fixture::{durable_fixture, write_durable_source};
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn durable_install_update_delete_managed_dirs(report: &mut HarnessLabReportBuilder) {
    let fixture = durable_fixture("durable-lifecycle", "initial");
    let install = install_agents_json(&fixture);
    let install_dir = fixture.lab.agents_root().join("durable-executor");
    assert_eq!(install["skill_name"], "durable-executor");
    assert_eq!(install["rote_preflight"]["present"], true);
    assert_eq!(install["managed_installs"][0]["status"], "installed");
    assert!(install_dir.join("SKILL.md").is_file());
    assert!(install_dir
        .join(".skillspec-durable-executor-managed")
        .is_file());
    let install_skill_md_present = install_dir.join("SKILL.md").is_file();
    let install_marker_present = install_dir
        .join(".skillspec-durable-executor-managed")
        .is_file();
    let install_config_present = fixture
        .lab
        .skillspec_home()
        .join("durable-executor/config.json")
        .is_file();
    assert!(install_config_present);

    write_durable_source(&fixture.lab, &fixture.source, "updated");
    let update = update_json(&fixture);
    assert_eq!(update["rote_preflight"]["present"], true);
    assert_eq!(update["managed_installs"][0]["status"], "updated");
    assert!(update["backup"]["path"].as_str().is_some());
    let update_contains_updated = file_contains(install_dir.join("SKILL.md"), "updated");
    assert!(update_contains_updated);

    let delete = delete_json(&fixture);
    assert_eq!(delete["managed_installs"][0]["status"], "removed");
    assert_eq!(delete["config_removed"], true);
    let delete_install_dir_removed = !install_dir.exists();
    assert!(delete_install_dir_removed);
    assert!(!fixture
        .lab
        .skillspec_home()
        .join("durable-executor/config.json")
        .exists());
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("durable_install_update_delete_managed_dirs");
    case.claim_pass(
        "install.skill_name",
        "durable-executor",
        &install["skill_name"],
    );
    case.claim_pass(
        "install.rote_present",
        true,
        &install["rote_preflight"]["present"],
    );
    case.claim_pass(
        "install.status",
        "installed",
        &install["managed_installs"][0]["status"],
    );
    case.claim_pass("install.skill_md", true, install_skill_md_present);
    case.claim_pass("install.marker", true, install_marker_present);
    case.claim_pass("install.config", true, install_config_present);
    case.claim_pass(
        "update.status",
        "updated",
        &update["managed_installs"][0]["status"],
    );
    case.claim_pass(
        "update.backup_path_present",
        true,
        update["backup"]["path"].is_string(),
    );
    case.claim_pass(
        "update.skill_md_contains_updated",
        true,
        update_contains_updated,
    );
    case.claim_pass(
        "delete.status",
        "removed",
        &delete["managed_installs"][0]["status"],
    );
    case.claim_pass("delete.config_removed", true, &delete["config_removed"]);
    case.claim_pass(
        "delete.install_dir_removed",
        true,
        delete_install_dir_removed,
    );
    case.finish();
}

pub fn durable_refuses_update_and_delete_without_marker(report: &mut HarnessLabReportBuilder) {
    let fixture = durable_fixture("durable-marker-guard", "marker guard");
    let install = install_agents_json(&fixture);
    assert_eq!(install["managed_installs"][0]["status"], "installed");
    let install_dir = fixture.lab.agents_root().join("durable-executor");
    let marker = install_dir.join(".skillspec-durable-executor-managed");
    std::fs::remove_file(&marker).unwrap();

    let unsafe_update = update_output(&fixture);
    assert_failed_with(&unsafe_update, "managed marker");
    assert!(install_dir.exists());

    fixture
        .lab
        .write_file(&marker, "schema: skillspec/durable-executor-managed/v1\n");
    std::fs::remove_file(&marker).unwrap();
    let unsafe_delete = delete_output(&fixture);
    assert_failed_with(&unsafe_delete, "managed marker");
    assert!(install_dir.exists());
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("durable_refuses_update_and_delete_without_marker");
    case.claim_pass("update.exit_success", false, unsafe_update.status.success());
    case.claim_pass(
        "update.stderr_marker",
        true,
        String::from_utf8_lossy(&unsafe_update.stderr).contains("managed marker"),
    );
    case.claim_pass("update.install_dir_preserved", true, install_dir.exists());
    case.claim_pass("delete.exit_success", false, unsafe_delete.status.success());
    case.claim_pass(
        "delete.stderr_marker",
        true,
        String::from_utf8_lossy(&unsafe_delete.stderr).contains("managed marker"),
    );
    case.claim_pass("delete.install_dir_preserved", true, install_dir.exists());
    case.finish();
}
