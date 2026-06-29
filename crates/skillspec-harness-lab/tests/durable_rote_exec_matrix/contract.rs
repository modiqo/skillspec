use super::commands::{act_json, align_json, decide_run_dir, plan_json};
use super::fixture::{rote_exec_fixture, PROOF_MARKER};
use skillspec_harness_lab::HarnessLabReportBuilder;

pub fn durable_executor_selects_rote_exec_contract(report: &mut HarnessLabReportBuilder) {
    let fixture = rote_exec_fixture("durable-rote-contract");
    let trace_dir = fixture.lab.root().join("traces");
    let plan = plan_json(&fixture, &trace_dir);
    let run_dir = super::fixture::latest_run_dir(&trace_dir);
    let act = act_json(&fixture, &run_dir);

    assert_eq!(plan["selected_route"], "one_shot_process");
    assert_eq!(act["selected_route"], "one_shot_process");
    assert_eq!(
        act["route_selection"]["rule_id"],
        "cli_invocations_use_rote_exec"
    );
    assert!(act["tool_boundary"]["allow"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "rote_exec"));
    assert!(act["forbidden"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "direct_cli_without_rote_exec"));
    assert!(act["matched_rules"]
        .as_array()
        .unwrap()
        .iter()
        .any(|rule| rule["id"] == "durable_result_uses_rote_exec"));
    assert!(fixture
        .lab
        .agents_root()
        .join("rote-shell/SKILL.md")
        .is_file());
    assert!(
        std::fs::read_to_string(fixture.lab.agents_root().join("rote-shell/SKILL.md"))
            .unwrap()
            .contains("rote exec --")
    );
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("durable_executor_selects_rote_exec_contract");
    case.claim_pass(
        "plan.selected_route",
        "one_shot_process",
        &plan["selected_route"],
    );
    case.claim_pass(
        "act.selected_route",
        "one_shot_process",
        &act["selected_route"],
    );
    case.claim_pass(
        "act.route_rule",
        "cli_invocations_use_rote_exec",
        &act["route_selection"]["rule_id"],
    );
    case.claim_pass(
        "act.allows_rote_exec",
        true,
        act["tool_boundary"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "rote_exec"),
    );
    case.claim_pass(
        "act.forbids_direct_cli",
        true,
        act["forbidden"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "direct_cli_without_rote_exec"),
    );
    case.claim_pass(
        "act.matches_durable_result_rule",
        true,
        act["matched_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule["id"] == "durable_result_uses_rote_exec"),
    );
    case.claim_pass(
        "rote_shell.skill_present",
        true,
        fixture
            .lab
            .agents_root()
            .join("rote-shell/SKILL.md")
            .is_file(),
    );
    case.claim_pass(
        "rote_shell.teaches_rote_exec",
        true,
        std::fs::read_to_string(fixture.lab.agents_root().join("rote-shell/SKILL.md"))
            .unwrap()
            .contains("rote exec --"),
    );
    case.finish();
}

pub fn alignment_accepts_rote_exec_process_evidence(report: &mut HarnessLabReportBuilder) {
    let fixture = rote_exec_fixture("durable-rote-alignment");
    let trace_dir = fixture.lab.root().join("traces");
    let run_dir = decide_run_dir(&fixture, &trace_dir);
    let execution_trace = fixture.lab.root().join("execution.jsonl");
    write_execution_ledger(&fixture, &execution_trace, "skillspec-durable-rote-exec");

    let align = align_json(&fixture, &run_dir, &execution_trace);
    assert_eq!(align["status"], "pass");
    assert_eq!(align["summary"]["decision_alignment"], "pass");
    assert_eq!(align["summary"]["execution_alignment"], "pass");
    assert_eq!(align["summary"]["execution_obligations"]["unproven"], 0);
    assert!(align["proof_rows"].as_array().unwrap().iter().any(|row| {
        row["requirement"] == "CLI work must be captured through rote exec"
            && row["status"] == "satisfied"
            && row["observed_evidence"]
                .as_str()
                .unwrap()
                .contains("printf")
    }));
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("alignment_accepts_rote_exec_process_evidence");
    case.claim_pass("align.status", "pass", &align["status"]);
    case.claim_pass(
        "align.decision_alignment",
        "pass",
        &align["summary"]["decision_alignment"],
    );
    case.claim_pass(
        "align.execution_alignment",
        "pass",
        &align["summary"]["execution_alignment"],
    );
    case.claim_pass(
        "align.unproven_obligations",
        0,
        &align["summary"]["execution_obligations"]["unproven"],
    );
    case.claim_pass(
        "align.rote_exec_proof_row",
        true,
        align["proof_rows"].as_array().unwrap().iter().any(|row| {
            row["requirement"] == "CLI work must be captured through rote exec"
                && row["status"] == "satisfied"
        }),
    );
    case.claim_pass(
        "align.command_redacted_to_basename",
        true,
        align["proof_rows"].as_array().unwrap().iter().any(|row| {
            row["observed_evidence"]
                .as_str()
                .unwrap_or_default()
                .contains("command(s) printf ran with arguments redacted")
        }),
    );
    case.claim_pass("execution.marker_command", PROOF_MARKER, PROOF_MARKER);
    case.finish();
}

pub fn write_execution_ledger(
    fixture: &super::fixture::RoteExecFixture,
    path: &std::path::Path,
    workspace: &str,
) {
    fixture.lab.write_file(
        path,
        &format!(
            r#"{{"event":"workspace_created","workspace":"{workspace}","anonymous":false}}
{{"event":"elicitation_waived","id":"durable_or_direct","status":"pass","evidence":{{"kind":"test_policy","ref":"durable mode explicitly selected"}}}}
{{"event":"process_started","workspace":"{workspace}","command":"printf {PROOF_MARKER}","executor":"rote_exec","operation_kind":"one_shot_process","exit_code":0,"stdout_captured":true,"stderr_captured":true,"response_id":"@1"}}
{{"event":"workspace_trace_collected","workspace":"{workspace}"}}
{{"event":"stats_collected","workspace":"{workspace}","response_tokens_cached":128,"query_result_tokens":8,"reduction_percent":93.75}}
{{"event":"after_success_completed","id":"record_stats_collected_event","status":"pass","evidence":{{"kind":"event","ref":"stats_collected"}}}}
{{"event":"after_success_completed","id":"run_skillspec_trace_alignment","status":"pass","evidence":{{"kind":"command","ref":"skillspec trace align"}}}}
{{"event":"obligation_satisfied","id":"final_summary_without_alignment_summary","status":"pass","evidence":{{"kind":"no_violation","ref":"final response included alignment"}}}}
{{"event":"obligation_satisfied","id":"final_summary_without_token_usage","status":"pass","evidence":{{"kind":"no_violation","ref":"final response included token usage"}}}}
{{"event":"obligation_satisfied","id":"ad_hoc_redirect_for_evidence","status":"pass","evidence":{{"kind":"no_violation","ref":"rote exec captured stdout"}}}}
{{"event":"final_response_sent","included_result":true,"included_alignment":true,"included_evidence":true,"included_token_savings":true}}
"#,
        ),
    );
}
