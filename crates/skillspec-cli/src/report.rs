use crate::align::{AlignCheckStatus, AlignReport, AlignStatus};
use crate::decision::{Decision, TestRun};
use crate::error::Result;
use crate::model::SkillSpec;
use crate::trace::TraceWriteResult;
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

pub fn align(report: &AlignReport) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "status: {}", align_status_name(report.status))?;
    writeln!(stdout, "spec: {}", report.spec)?;
    writeln!(stdout, "decision_trace: {}", report.decision_trace)?;
    writeln!(stdout, "summary: {}", report.summary.conclusion)?;
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
    writeln!(
        stdout,
        "proof: execution obligations {} pass, {} fail, {} unproven ({} total)",
        report.summary.execution_obligations.pass,
        report.summary.execution_obligations.fail,
        report.summary.execution_obligations.unproven,
        report.summary.execution_obligations.total
    )?;
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
        writeln!(stdout, "unproven_by_kind: {groups}")?;
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
        writeln!(stdout, "obligations:")?;
        for obligation in &report.obligations {
            writeln!(
                stdout,
                "  - {}: {} ({})",
                obligation.id,
                check_status_name(obligation.status),
                obligation.message
            )?;
        }
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

fn obligation_kind_name(kind: crate::align::AlignObligationKind) -> &'static str {
    match kind {
        crate::align::AlignObligationKind::Route => "route",
        crate::align::AlignObligationKind::RouteCheck => "route_check",
        crate::align::AlignObligationKind::Forbid => "forbid",
        crate::align::AlignObligationKind::Elicitation => "elicitation",
        crate::align::AlignObligationKind::AfterSuccess => "after_success",
    }
}

pub fn error(error: crate::error::Error) {
    let _ = writeln!(io::stderr().lock(), "error: {error}");
}
