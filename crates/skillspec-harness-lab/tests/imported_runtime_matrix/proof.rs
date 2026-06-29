use super::commands::{
    act_json, align_json, decide_json, latest_run_dir, plan_json, progress_batch_summary,
    progress_final_response, progress_stats,
};
use super::fixture::reviewed_import_fixture;
use skillspec_harness_lab::{assert_success, json_stdout, stdout, HarnessLabReportBuilder};

const INPUT: &str = "verify and install this skill";

pub fn decision_trace_without_execution_is_unproven(report: &mut HarnessLabReportBuilder) {
    let fixture = reviewed_import_fixture("runtime-unproven");
    let trace_dir = fixture.lab.root().join("traces");
    let plan_trace_dir = fixture.lab.root().join("plan-traces");

    let decision = decide_json(&fixture, INPUT, &trace_dir);
    let run_dir = latest_run_dir(&trace_dir);
    let plan = plan_json(&fixture, INPUT, &plan_trace_dir);
    assert_eq!(decision["route"], "verify_skill");
    assert_eq!(plan["selected_route"], "verify_skill");
    assert_eq!(plan["phases"].as_array().unwrap().len(), 2);

    let align = align_json(&fixture, &run_dir, false);
    assert_success(&align);
    let align_report = json_stdout(&align);
    assert_eq!(align_report["status"], "unproven");
    assert_eq!(align_report["summary"]["scope"], "decision_trace_only");
    assert_eq!(
        align_report["summary"]["execution_alignment"],
        "not_evaluated"
    );
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("decision_trace_without_execution_is_unproven");
    case.claim_pass("decide.route", "verify_skill", &decision["route"]);
    case.claim_pass(
        "plan.selected_route",
        "verify_skill",
        &plan["selected_route"],
    );
    case.claim_pass(
        "plan.phase_count",
        2,
        plan["phases"].as_array().unwrap().len(),
    );
    case.claim_pass("align.exit_success", true, align.status.success());
    case.claim_pass("align.status", "unproven", &align_report["status"]);
    case.claim_pass(
        "align.scope",
        "decision_trace_only",
        &align_report["summary"]["scope"],
    );
    case.claim_pass(
        "align.execution_alignment",
        "not_evaluated",
        &align_report["summary"]["execution_alignment"],
    );
    case.finish();
}

pub fn progress_batch_and_alignment_prove_reviewed_import(report: &mut HarnessLabReportBuilder) {
    let fixture = reviewed_import_fixture("runtime-aligned");
    let trace_dir = fixture.lab.root().join("traces");
    let plan_trace_dir = fixture.lab.root().join("plan-traces");
    let decision = decide_json(&fixture, INPUT, &trace_dir);
    let run_dir = latest_run_dir(&trace_dir);
    let plan = plan_json(&fixture, INPUT, &plan_trace_dir);
    let act = act_json(&fixture, INPUT, &run_dir);
    assert_eq!(decision["route"], "verify_skill");
    assert_eq!(plan["selected_route"], "verify_skill");
    assert_eq!(act["selected_route"], "verify_skill");
    assert_eq!(act["current_phase"]["id"], "assess");

    let events = fixture.lab.root().join("runtime-proof.jsonl");
    fixture.lab.write_file(
        &events,
        r#"{"event":"phase-started","phase":"assess","evidence":{"kind":"checklist","ref":"skillspec act"}}
{"event":"requirement-satisfied","phase":"assess","requirement":"doctor_report","status":"pass","evidence":{"kind":"command","ref":"skillspec doctor"}}
{"event":"phase-completed","phase":"assess","evidence":{"kind":"report","ref":"doctor.json"}}
{"event":"phase-started","phase":"install","evidence":{"kind":"checklist","ref":"skillspec act --phase install"}}
{"event":"requirement-satisfied","phase":"install","requirement":"installed_loader","status":"pass","evidence":{"kind":"command","ref":"skillspec install skill"}}
{"event":"phase-completed","phase":"install","evidence":{"kind":"directory","ref":"installed skill root"}}
{"event":"route-fulfilled","id":"verify_skill","status":"pass","evidence":{"kind":"trace","ref":"runtime route"}}
{"event":"route-check-completed","id":"run_validation","status":"pass","evidence":{"kind":"command","ref":"skillspec validate"}}
"#,
    );
    let batch = progress_batch_summary(&fixture, &run_dir, &events);
    assert_success(&batch);
    let batch_text = stdout(&batch);
    assert!(batch_text.contains("records: 8"));

    let stats = progress_stats(&fixture, &run_dir);
    assert_success(&stats);
    let stats_event = json_stdout(&stats);
    assert_eq!(stats_event["event"], "stats_collected");
    assert_eq!(stats_event["avoided_tokens"], 2280);

    let final_response = progress_final_response(&fixture, &run_dir);
    assert_success(&final_response);
    let final_event = json_stdout(&final_response);
    assert_eq!(final_event["event"], "final_response_sent");
    assert_eq!(final_event["included_alignment"], true);
    assert_eq!(final_event["included_token_savings"], true);

    let align = align_json(&fixture, &run_dir, true);
    assert_success(&align);
    let align_report = json_stdout(&align);
    assert_eq!(
        align_report["status"],
        "pass",
        "{}",
        serde_json::to_string_pretty(&align_report).unwrap()
    );
    assert_eq!(
        align_report["summary"]["completion"]["requirements"],
        "2/2 proven"
    );
    assert_eq!(align_report["summary"]["completion"]["alignment"], "pass");
    assert_eq!(
        align_report["summary"]["completion"]["forbidden_actions"],
        "no violations recorded"
    );
    assert!(align_report["summary"]["tokens"]["savings"]
        .as_str()
        .unwrap()
        .contains("2280 tokens kept out of chat"));
    assert!(run_dir.join("alignment.json").is_file());
    fixture.lab.assert_no_real_home_writes();

    let mut case = report.case("progress_batch_and_alignment_prove_reviewed_import");
    case.claim_pass("decide.route", "verify_skill", &decision["route"]);
    case.claim_pass(
        "plan.selected_route",
        "verify_skill",
        &plan["selected_route"],
    );
    case.claim_pass("act.current_phase", "assess", &act["current_phase"]["id"]);
    case.claim_pass("progress.batch.exit_success", true, batch.status.success());
    case.claim_pass(
        "progress.batch.records",
        true,
        batch_text.contains("records: 8"),
    );
    case.claim_pass(
        "progress.stats.avoided_tokens",
        2280,
        &stats_event["avoided_tokens"],
    );
    case.claim_pass(
        "progress.final_response.alignment",
        true,
        &final_event["included_alignment"],
    );
    case.claim_pass(
        "progress.final_response.token_savings",
        true,
        &final_event["included_token_savings"],
    );
    case.claim_pass("align.status", "pass", &align_report["status"]);
    case.claim_pass(
        "align.requirements",
        "2/2 proven",
        &align_report["summary"]["completion"]["requirements"],
    );
    case.claim_pass(
        "align.completion",
        "pass",
        &align_report["summary"]["completion"]["alignment"],
    );
    case.claim_pass(
        "align.report_written",
        true,
        run_dir.join("alignment.json").is_file(),
    );
    case.finish();
}
