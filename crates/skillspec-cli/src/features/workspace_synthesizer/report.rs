use super::SynthesisReport;

pub fn render_report(report: &SynthesisReport) -> String {
    let deps = if report.inferred_dependencies.is_empty() {
        "none inferred".to_owned()
    } else {
        report.inferred_dependencies.join(", ")
    };
    format!(
        "SkillSpec CLI synthesis\n\nSpec: {}\nDeps: {}\nCommand candidates: {}\nInferred dependencies: {}\n\nNext: review the typed input contract and dependency ledger, then run `skillspec validate {}`, `skillspec deps check {}`, and `skillspec test {}`.\n",
        report.spec_path.display(),
        report.deps_path.display(),
        report.command_candidates,
        deps,
        report.spec_path.display(),
        report.spec_path.display(),
        report.spec_path.display()
    )
}
