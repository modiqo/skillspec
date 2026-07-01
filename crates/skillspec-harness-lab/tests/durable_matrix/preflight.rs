use super::commands::{
    assert_failed_with, disable_json, enable_missing_rote, install_agents_json,
    install_agents_missing_rote,
};
use super::fixture::durable_fixture;
use skillspec_harness_lab::{json_stdout, HarnessLabReportBuilder};

pub fn durable_install_and_enable_require_rote(report: &mut HarnessLabReportBuilder) {
    let missing_install = durable_fixture("durable-missing-rote-install", "missing rote");
    let install = install_agents_missing_rote(&missing_install, false);
    assert_failed_with(&install, "requires `rote` on PATH");
    let install_no_managed_dir = !missing_install
        .lab
        .agents_root()
        .join("durable-executor")
        .exists();
    let install_no_config = !missing_install
        .lab
        .skillspec_home()
        .join("durable-executor/config.json")
        .exists();
    assert!(install_no_managed_dir);
    assert!(install_no_config);

    let dry_run = install_agents_missing_rote(&missing_install, true);
    assert!(dry_run.status.success());
    let dry_run_report = json_stdout(&dry_run);
    assert_eq!(dry_run_report["rote_preflight"]["present"], false);
    assert!(!missing_install
        .lab
        .agents_root()
        .join("durable-executor")
        .exists());

    let missing_enable = durable_fixture("durable-missing-rote-enable", "enable missing rote");
    let installed = install_agents_json(&missing_enable);
    assert_eq!(installed["managed_installs"][0]["status"], "installed");
    let disable = disable_json(&missing_enable);
    assert_eq!(disable["enabled"], false);
    let enable = enable_missing_rote(&missing_enable);
    assert_failed_with(&enable, "requires `rote` on PATH");
    assert!(std::fs::read_to_string(
        missing_enable
            .lab
            .agents_root()
            .join("durable-executor/agents/openai.yaml"),
    )
    .unwrap()
    .contains("allow_implicit_invocation: false"));
    missing_install.lab.assert_no_real_home_writes();
    missing_enable.lab.assert_no_real_home_writes();

    let mut case = report.case("durable_install_and_enable_require_rote");
    case.claim_pass("install.exit_success", false, install.status.success());
    case.claim_pass(
        "install.stderr_rote",
        true,
        String::from_utf8_lossy(&install.stderr).contains("requires `rote` on PATH"),
    );
    case.claim_pass("install.no_managed_dir_write", true, install_no_managed_dir);
    case.claim_pass("install.no_config_write", true, install_no_config);
    case.claim_pass("dry_run.exit_success", true, dry_run.status.success());
    case.claim_pass(
        "dry_run.rote_present",
        false,
        &dry_run_report["rote_preflight"]["present"],
    );
    case.claim_pass("enable.exit_success", false, enable.status.success());
    case.claim_pass(
        "enable.stderr_rote",
        true,
        String::from_utf8_lossy(&enable.stderr).contains("requires `rote` on PATH"),
    );
    case.claim_pass(
        "enable.visibility_remains_manual",
        true,
        std::fs::read_to_string(
            missing_enable
                .lab
                .agents_root()
                .join("durable-executor/agents/openai.yaml"),
        )
        .unwrap()
        .contains("allow_implicit_invocation: false"),
    );
    case.finish();
}
