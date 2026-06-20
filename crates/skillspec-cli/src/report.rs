use crate::decision::{Decision, TestRun};
use crate::error::Result;
use crate::model::SkillSpec;
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
    text(&format!(
        "ok: imported {} into {} with {} review note(s)",
        path.display(),
        out.display(),
        spec.review_required.len()
    ))
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

pub fn compile_markdown(spec: &SkillSpec) -> String {
    let mut output = String::new();
    output.push_str("---\n");
    output.push_str(&format!("name: {}\n", spec.id));
    output.push_str(&format!("description: {:?}\n", spec.description));
    output.push_str("---\n\n");
    output.push_str(&format!("# {}\n\n", spec.title));
    output.push_str("Follow the companion `skill.spec.yml` for structured routing, guards, state progression, command templates, and scenario tests.\n\n");
    output.push_str("## Policy Summary\n\n");
    output.push_str(&format!("- routes: {}\n", spec.routes.len()));
    output.push_str(&format!("- rules: {}\n", spec.rules.len()));
    output.push_str(&format!("- states: {}\n", spec.states.len()));
    output.push_str(&format!("- commands: {}\n", spec.commands.len()));
    output.push_str(&format!("- tests: {}\n", spec.tests.len()));
    output
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

pub fn error(error: crate::error::Error) {
    let _ = writeln!(io::stderr().lock(), "error: {error}");
}
