use crate::align::{AlignCheckStatus, AlignProofStatus, AlignReport, AlignStatus};
use crate::decision::{Decision, TestRun};
use crate::trace::TraceWriteResult;
use skillspec_core::error::Result;
use skillspec_core::model::SkillSpec;
use std::io::{self, Write};
use std::path::Path;

pub fn validation_ok(path: &Path, spec: &SkillSpec) -> Result<()> {
    text(&format!(
        "ok: {} is {} ({})",
        path.display(),
        spec.id,
        spec.schema
    ))
}

pub fn import_ok(path: &Path, out: &Path, spec: &SkillSpec) -> Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(
        stdout,
        "ok: imported {} into {} with {} review note(s)",
        path.display(),
        out.display(),
        spec.review_required.len()
    )?;
    writeln!(
        stdout,
        "next: run `skillspec grammar sensemake --view porting` before semantic review"
    )?;
    writeln!(
        stdout,
        "next: run `skillspec sensemake {} --view index` to inspect draft coverage",
        out.display()
    )?;
    writeln!(
        stdout,
        "next: run `skillspec grammar checklist --for import-skill` and fill the coverage matrix before install"
    )?;
    writeln!(
        stdout,
        "deps ledger: wrote deps.toml beside the draft; zero dependency entries are allowed, but a byte-empty ledger is not"
    )?;
    writeln!(
        stdout,
        "next: inspect deps.toml and complete dependency authority, local status, install risk, and degraded proof impact before proof/install"
    )?;
    if spec.imports.is_empty() {
        writeln!(stdout, "imports: none inferred")?;
    } else {
        let imports = spec
            .imports
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(stdout, "imports: inferred {imports}")?;
        writeln!(
            stdout,
            "next: run `skillspec imports check {}` and review import load paths before install",
            out.display()
        )?;
    }
    if spec.dependencies.is_empty() {
        writeln!(stdout, "deps: none inferred")?;
    } else {
        let deps = spec
            .dependencies
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(stdout, "deps: inferred {deps}")?;
        writeln!(
            stdout,
            "next: run `skillspec deps check {}` and review permissions/provisioning before install",
            out.display()
        )?;
    }
    Ok(())
}

pub fn test_result(result: &TestRun) -> Result<()> {
    let total = result.passed.len() + result.failed.len();
    let mut stdout = io::stdout().lock();
    writeln!(
        stdout,
        "skillspec test: {}/{} passed",
        result.passed.len(),
        total
    )?;
    for failure in &result.failed {
        writeln!(stdout, "FAIL {}", failure.name)?;
        for reason in &failure.failures {
            writeln!(stdout, "  - {reason}")?;
        }
    }
    Ok(())
}

pub fn explain(decision: &Decision) -> Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "input: {}", decision.input)?;
    if let Some(route) = &decision.route {
        writeln!(stdout, "route: {}", route.0)?;
    } else {
        writeln!(stdout, "route: <none>")?;
    }
    if !decision.route_order.is_empty() {
        let route_order = decision
            .route_order
            .iter()
            .map(|route| route.0.as_str())
            .collect::<Vec<_>>()
            .join(" -> ");
        writeln!(stdout, "route_order: {route_order}")?;
    }
    if !decision.forbid.is_empty() {
        writeln!(stdout, "forbid: {}", decision.forbid.join(", "))?;
    }
    if !decision.elicit.is_empty() {
        writeln!(stdout, "elicit: {}", decision.elicit.join(", "))?;
    }
    if !decision.after_success.is_empty() {
        writeln!(
            stdout,
            "after_success: {}",
            decision.after_success.join(", ")
        )?;
    }
    if !decision.matched_rules.is_empty() {
        writeln!(stdout, "matched_rules:")?;
        for matched in &decision.matched_rules {
            match &matched.reason {
                Some(reason) => writeln!(stdout, "  - {}: {}", matched.id.0, reason)?,
                None => writeln!(stdout, "  - {}", matched.id.0)?,
            }
        }
    }
    Ok(())
}

pub fn json<T: serde::Serialize>(value: &T) -> Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(())
}

pub fn text(value: &str) -> Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{value}")?;
    Ok(())
}

pub fn trace_written(trace: &TraceWriteResult) -> Result<()> {
    let mut stderr = io::stderr().lock();
    writeln!(
        stderr,
        "trace: wrote {} event(s) to {}",
        trace.event_count,
        trace.run_dir.display()
    )?;
    writeln!(stderr, "trace: jsonl {}", trace.trace_jsonl.display())?;
    writeln!(stderr, "trace: summary {}", trace.summary_json.display())?;
    Ok(())
}

pub fn alignment_written(path: &Path) -> Result<()> {
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "alignment: wrote {}", path.display())?;
    Ok(())
}

pub fn proof_digest_written(path: &Path) -> Result<()> {
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "proof_digest: wrote {}", path.display())?;
    Ok(())
}

pub fn align(report: &AlignReport) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    let execution_not_evaluated = matches!(
        report.summary.execution_alignment,
        crate::align::AlignLayerStatus::NotEvaluated
    );
    writeln!(
        stdout,
        "alignment: decision={}, execution={}",
        align_layer_status_name(report.summary.decision_alignment),
        align_layer_status_name(report.summary.execution_alignment)
    )?;
    writeln!(stdout, "scope: {}", align_scope_name(report.summary.scope))?;
    writeln!(stdout, "status: {}", align_status_name(report.status))?;
    writeln!(stdout, "spec: {}", report.spec)?;
    writeln!(stdout, "decision_trace: {}", report.decision_trace)?;
    if !report.execution_traces.is_empty() {
        writeln!(
            stdout,
            "execution_traces: {}",
            report.execution_traces.join(", ")
        )?;
    }
    writeln!(stdout, "summary: {}", report.summary.conclusion)?;
    writeln!(stdout, "meaning: {}", report.summary.status_meaning)?;
    writeln!(stdout, "alignment_summary:")?;
    writeln!(
        stdout,
        "  Decision replay: {}",
        report.summary.completion.decision_replay
    )?;
    writeln!(
        stdout,
        "  Phase order: {}",
        report.summary.completion.phase_order
    )?;
    writeln!(
        stdout,
        "  Requirements: {}",
        report.summary.completion.requirements
    )?;
    for item in &report.summary.completion.missing_proof {
        writeln!(stdout, "  Missing proof: {item}")?;
    }
    writeln!(
        stdout,
        "  Forbidden actions: {}",
        report.summary.completion.forbidden_actions
    )?;
    writeln!(
        stdout,
        "  Alignment: {}",
        report.summary.completion.alignment
    )?;
    writeln!(stdout, "token_usage:")?;
    writeln!(
        stdout,
        "  Token consumption: {}",
        report.summary.tokens.consumption
    )?;
    writeln!(stdout, "  Token savings: {}", report.summary.tokens.savings)?;
    if !report.summary.tokens.evidence.is_empty() {
        writeln!(
            stdout,
            "  Token evidence: {}",
            report.summary.tokens.evidence.join(", ")
        )?;
    }
    if !report.summary.layers.is_empty() {
        writeln!(stdout, "model:")?;
        for layer in &report.summary.layers {
            writeln!(
                stdout,
                "  - {}: {}",
                align_layer_name(layer.id),
                layer.measures
            )?;
            writeln!(stdout, "    result: {}", layer.interpretation)?;
        }
    }
    if let Some(route) = &report.summary.selected_route {
        write!(stdout, "decision: route {route}")?;
        if let Some(basis) = &report.summary.route_selection_basis {
            write!(stdout, " via {basis}")?;
        }
        if let Some(rule) = &report.summary.route_selection_rule {
            write!(stdout, " ({rule})")?;
        }
        writeln!(stdout)?;
    }
    if !report.summary.matched_rules.is_empty() {
        writeln!(
            stdout,
            "matched_rules: {}",
            report.summary.matched_rules.join(", ")
        )?;
    }
    writeln!(
        stdout,
        "proof: decision checks {} pass, {} fail, {} unproven ({} total)",
        report.summary.decision_checks.pass,
        report.summary.decision_checks.fail,
        report.summary.decision_checks.unproven,
        report.summary.decision_checks.total
    )?;
    if execution_not_evaluated {
        writeln!(
            stdout,
            "proof: execution obligations not evaluated because no execution trace was supplied ({} obligation(s) require execution evidence)",
            report.summary.execution_obligations.total
        )?;
    } else {
        writeln!(
            stdout,
            "proof: execution obligations {} pass, {} fail, {} unproven ({} total)",
            report.summary.execution_obligations.pass,
            report.summary.execution_obligations.fail,
            report.summary.execution_obligations.unproven,
            report.summary.execution_obligations.total
        )?;
    }
    if !report.summary.unproven_obligation_kinds.is_empty() {
        let groups = report
            .summary
            .unproven_obligation_kinds
            .iter()
            .map(|count| {
                format!(
                    "{}={}/{}",
                    obligation_kind_name(count.kind),
                    count.unproven,
                    count.total
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let label = if execution_not_evaluated {
            "execution_requirements_by_kind"
        } else {
            "unproven_by_kind"
        };
        writeln!(stdout, "{label}: {groups}")?;
    }
    if !report.summary.evidence_gaps.is_empty() {
        let label = if execution_not_evaluated {
            "evidence_needed_for_execution_trace"
        } else {
            "evidence_gaps"
        };
        writeln!(stdout, "{label}:")?;
        for gap in &report.summary.evidence_gaps {
            match gap.obligation_kind {
                Some(kind) => writeln!(
                    stdout,
                    "  - {} {} ({}) from {}: {}",
                    evidence_gap_kind_name(gap.kind),
                    gap.id,
                    obligation_kind_name(kind),
                    gap.source,
                    gap.needed
                )?,
                None => writeln!(
                    stdout,
                    "  - {} {} from {}: {}",
                    evidence_gap_kind_name(gap.kind),
                    gap.id,
                    gap.source,
                    gap.needed
                )?,
            }
        }
    }
    if !report.proof_rows.is_empty() {
        let label = if execution_not_evaluated {
            "execution_evidence_needed"
        } else {
            "alignment_evidence"
        };
        writeln!(stdout, "{label}:")?;
        for row in &report.proof_rows {
            writeln!(stdout, "  - requirement: {}", row.requirement)?;
            writeln!(stdout, "    obligation: {}", row.obligation)?;
            writeln!(stdout, "    expected: {}", row.expected_evidence)?;
            writeln!(stdout, "    observed: {}", row.observed_evidence)?;
            let status = if execution_not_evaluated && row.status == AlignProofStatus::Unproven {
                "not_evaluated"
            } else {
                proof_status_name(row.status)
            };
            writeln!(stdout, "    status: {status}")?;
            writeln!(stdout, "    explanation: {}", row.explanation)?;
        }
    }
    writeln!(stdout, "checks:")?;
    for check in &report.checks {
        writeln!(
            stdout,
            "  - {}: {} ({})",
            check.id,
            check_status_name(check.status),
            check.message
        )?;
    }
    if !report.obligations.is_empty() {
        let label = if execution_not_evaluated {
            "execution_obligations_not_evaluated"
        } else {
            "obligations"
        };
        writeln!(stdout, "{label}:")?;
        for obligation in &report.obligations {
            let status =
                if execution_not_evaluated && obligation.status == AlignCheckStatus::Unproven {
                    "not_evaluated"
                } else {
                    check_status_name(obligation.status)
                };
            writeln!(
                stdout,
                "  - {}: {} ({})",
                obligation.id, status, obligation.message
            )?;
        }
    }
    Ok(())
}

pub fn align_summary(
    report: &AlignReport,
    alignment_report: &Path,
    proof_digest: Option<&Path>,
) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "alignment_summary:")?;
    writeln!(
        stdout,
        "  Decision replay: {}",
        report.summary.completion.decision_replay
    )?;
    writeln!(
        stdout,
        "  Phase order: {}",
        report.summary.completion.phase_order
    )?;
    writeln!(
        stdout,
        "  Requirements: {}",
        report.summary.completion.requirements
    )?;
    for item in &report.summary.completion.missing_proof {
        writeln!(stdout, "  Missing proof: {item}")?;
    }
    writeln!(
        stdout,
        "  Forbidden actions: {}",
        report.summary.completion.forbidden_actions
    )?;
    writeln!(
        stdout,
        "  Alignment: {}",
        report.summary.completion.alignment
    )?;
    writeln!(stdout, "summary_meaning:")?;
    writeln!(
        stdout,
        "  Decision replay: replays the current spec against the captured input; pass means routing is reproducible."
    )?;
    writeln!(
        stdout,
        "  Execution proof: checks execution.jsonl for structured evidence; partial or unproven means evidence is missing or incomplete, not that decision replay failed."
    )?;
    writeln!(stdout, "token_usage:")?;
    writeln!(
        stdout,
        "  Token consumption: {}",
        report.summary.tokens.consumption
    )?;
    writeln!(stdout, "  Token savings: {}", report.summary.tokens.savings)?;
    if !report.summary.tokens.evidence.is_empty() {
        writeln!(
            stdout,
            "  Token evidence: {}",
            report.summary.tokens.evidence.join(", ")
        )?;
    }
    writeln!(stdout, "alignment_report: {}", alignment_report.display())?;
    if let Some(path) = proof_digest {
        writeln!(stdout, "proof_digest: {}", path.display())?;
    }
    Ok(())
}

fn check_status_name(status: AlignCheckStatus) -> &'static str {
    match status {
        AlignCheckStatus::Pass => "pass",
        AlignCheckStatus::Fail => "fail",
        AlignCheckStatus::Unproven => "unproven",
    }
}

fn align_status_name(status: AlignStatus) -> &'static str {
    match status {
        AlignStatus::Pass => "pass",
        AlignStatus::Fail => "fail",
        AlignStatus::Unproven => "unproven",
    }
}

fn proof_status_name(status: AlignProofStatus) -> &'static str {
    match status {
        AlignProofStatus::Satisfied => "satisfied",
        AlignProofStatus::PartiallySatisfied => "partially_satisfied",
        AlignProofStatus::Violated => "violated",
        AlignProofStatus::Unproven => "unproven",
    }
}

fn align_layer_name(kind: crate::align::AlignLayerKind) -> &'static str {
    match kind {
        crate::align::AlignLayerKind::DecisionReplay => "decision_replay",
        crate::align::AlignLayerKind::ExecutionProof => "execution_proof",
    }
}

fn align_scope_name(scope: crate::align::AlignScope) -> &'static str {
    match scope {
        crate::align::AlignScope::DecisionTraceOnly => "decision_trace_only",
        crate::align::AlignScope::DecisionAndExecutionTrace => "decision_and_execution_trace",
    }
}

fn align_layer_status_name(status: crate::align::AlignLayerStatus) -> &'static str {
    match status {
        crate::align::AlignLayerStatus::Pass => "pass",
        crate::align::AlignLayerStatus::Fail => "fail",
        crate::align::AlignLayerStatus::Incomplete => "incomplete",
        crate::align::AlignLayerStatus::NotEvaluated => "not_evaluated",
    }
}

fn evidence_gap_kind_name(kind: crate::align::AlignEvidenceGapKind) -> &'static str {
    match kind {
        crate::align::AlignEvidenceGapKind::DecisionTrace => "decision_trace",
        crate::align::AlignEvidenceGapKind::ExecutionObligation => "execution_obligation",
    }
}

fn obligation_kind_name(kind: crate::align::AlignObligationKind) -> &'static str {
    match kind {
        crate::align::AlignObligationKind::Route => "route",
        crate::align::AlignObligationKind::RouteCheck => "route_check",
        crate::align::AlignObligationKind::Forbid => "forbid",
        crate::align::AlignObligationKind::Elicitation => "elicitation",
        crate::align::AlignObligationKind::AfterSuccess => "after_success",
        crate::align::AlignObligationKind::UserRequirement => "user_requirement",
    }
}

pub fn error(error: skillspec_core::error::Error) {
    let _ = writeln!(io::stderr().lock(), "error: {error}");
}
