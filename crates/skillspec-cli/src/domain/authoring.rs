use crate::{
    capability, compiler, deps, error, grammar, importer, imports, model, parser, port_one_shot,
    remote_source, router, router_lifecycle, source_map, workspace, workspace_synthesizer,
};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

pub use capability::{AddOptions, PreferOptions, SearchOptions, UpdateOptions, VerificationStatus};
pub use port_one_shot::PortOneShotOptions;
pub use router::{IndexOptions, RouteOptions};
pub use workspace_synthesizer::SynthesizeOptions;

pub struct PortOneShotOutput {
    pub report: port_one_shot::PortOneShotReport,
    pub rendered: String,
}

pub fn compile(path: &Path, target: compiler::Target) -> error::Result<String> {
    let spec = parser::load_spec(path)?;
    Ok(compiler::compile(&spec, target))
}

pub fn import_skill(
    path: &Path,
    out: &Path,
    source_map_path: Option<&Path>,
) -> error::Result<model::SkillSpec> {
    workspace::guard_single_skill_source(path, "skillspec import-skill")?;
    if let Some(source_map_path) = source_map_path {
        let source_root = source_map::source_root_for(path);
        let stale_report = source_map::stale(source_map_path, Some(&source_root))?;
        if !stale_report.ok {
            return Err(error::Error::InvalidInput {
                message: format!(
                    "source map {} is stale for {}; rerun `skillspec source map {} --out <map-dir>` before import",
                    source_map_path.display(),
                    source_root.display(),
                    path.display()
                ),
            });
        }
    }
    let imported = importer::import_skill_for_output(path, out)?;
    parser::write_spec(out, &imported)?;
    Ok(imported)
}

pub fn port_one_shot(
    options: port_one_shot::PortOneShotOptions,
) -> error::Result<PortOneShotOutput> {
    let started = Instant::now();
    let mut report = port_one_shot::run(options)?;
    let elapsed = started.elapsed();
    let preview = port_one_shot::render_summary(&report, elapsed);
    port_one_shot::record_estimated_stats(&mut report, elapsed, preview.len() as u64)?;
    let rendered = port_one_shot::render_summary(&report, elapsed);
    port_one_shot::write_report(&report, &rendered)?;
    Ok(PortOneShotOutput { report, rendered })
}

pub fn synthesize_from_workspace(
    options: workspace_synthesizer::SynthesizeOptions,
) -> error::Result<workspace_synthesizer::SynthesisReport> {
    workspace_synthesizer::synthesize_from_workspace(options)
}

pub fn render_synthesis(report: &workspace_synthesizer::SynthesisReport) -> String {
    workspace_synthesizer::render_report(report)
}

pub fn index(options: router::IndexOptions) -> error::Result<router::IndexReport> {
    let mut report = router::index(options)?;
    report
        .warnings
        .extend(router_lifecycle::direct_index_warnings());
    Ok(report)
}

pub fn render_index(report: &router::IndexReport) -> String {
    router::render_index(report)
}

pub fn route(options: router::RouteOptions) -> error::Result<router::RouteReport> {
    router::route(options)
}

pub fn render_route(report: &router::RouteReport) -> String {
    router::render_route(report)
}

pub fn stage_remote_source(
    uri: &str,
    out: Option<&Path>,
    detect_candidates: bool,
) -> error::Result<remote_source::RemoteStageReport> {
    remote_source::stage_remote_source(uri, out, detect_candidates)
}

pub fn render_stage_report(report: &remote_source::RemoteStageReport) -> String {
    remote_source::render_stage_report(report)
}

pub fn create_source_map(
    path: &Path,
    out: &Path,
) -> error::Result<source_map::SourceMapWriteReport> {
    source_map::create_source_map(path, out)
}

pub fn create_source_map_from_source(
    source: &str,
    out: &Path,
) -> error::Result<source_map::SourceMapWriteReport> {
    let local_path = PathBuf::from(source);
    if local_path.exists() || looks_like_explicit_local_path(source) {
        return create_source_map(&local_path, out);
    }

    let Some(_) = remote_source::parse_target(source)? else {
        return create_source_map(&local_path, out);
    };

    let stage_root = remote_stage_root_for(out);
    let stage_report = remote_source::stage_remote_source(source, Some(&stage_root), true)?;
    let source_path = selected_remote_source_path(&stage_report, out)?;
    let mut report = create_source_map(Path::new(&source_path), out)?;
    report.staged_from = Some(stage_report.target);
    report.staged_checkout = Some(stage_report.checkout_dir);
    report.source_path = Some(source_path);
    Ok(report)
}

fn looks_like_explicit_local_path(source: &str) -> bool {
    source.starts_with('.')
        || source.starts_with('/')
        || source.starts_with('~')
        || source.starts_with(std::path::MAIN_SEPARATOR)
}

fn remote_stage_root_for(out: &Path) -> PathBuf {
    let base = out
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("staged");
    base.join(format!("source-map-{}", unique_nanos()))
}

fn unique_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn selected_remote_source_path(
    report: &remote_source::RemoteStageReport,
    out: &Path,
) -> error::Result<String> {
    if let Some(path) = &report.selected_source_path {
        return Ok(path.clone());
    }

    if report.candidates.is_empty() {
        return Err(error::Error::InvalidInput {
            message: format!(
                "remote source {} did not contain a SKILL.md candidate; run `skillspec source stage {} --out <staging-root> --json` to inspect the checkout",
                report.target, report.target
            ),
        });
    }

    let candidates = report
        .candidates
        .iter()
        .map(|candidate| format!("- {}", candidate.source_path))
        .collect::<Vec<_>>()
        .join("\n");
    Err(error::Error::InvalidInput {
        message: format!(
            "remote source {} has multiple SKILL.md candidates; choose one source_path and rerun `skillspec source map <source_path> --out {}`:\n{}",
            report.target,
            out.display(),
            candidates
        ),
    })
}

pub fn render_source_map_write(report: &source_map::SourceMapWriteReport) -> String {
    source_map::render_write_report(report)
}

pub fn query_source_map(
    map: &Path,
    handle: &str,
    view: source_map::SourceView,
) -> error::Result<serde_json::Value> {
    source_map::query(map, handle, view)
}

pub fn render_source_query(value: &serde_json::Value) -> String {
    source_map::render_query(value)
}

pub fn source_coverage(map: &Path) -> error::Result<source_map::SourceCoverage> {
    Ok(source_map::load(map)?.coverage)
}

pub fn render_source_coverage(coverage: &source_map::SourceCoverage) -> String {
    source_map::render_coverage(coverage)
}

pub fn stale_source_map(
    map: &Path,
    root: Option<&Path>,
) -> error::Result<source_map::SourceStaleReport> {
    source_map::stale(map, root)
}

pub fn render_source_stale(report: &source_map::SourceStaleReport) -> String {
    source_map::render_stale(report)
}

pub fn check_deps(
    path: &Path,
    command: Option<&str>,
) -> error::Result<deps::DependencyCheckReport> {
    let spec = parser::load_spec(path)?;
    let spec_dir = path.parent().unwrap_or_else(|| Path::new("."));
    deps::check(&spec, spec_dir, command)
}

pub fn check_imports(path: &Path) -> error::Result<imports::ImportCheckReport> {
    let spec = parser::load_spec_unresolved(path)?;
    Ok(imports::check(&spec, path))
}

pub fn grammar_sensemake(view: grammar::GrammarView) -> grammar::GrammarSenseReport {
    grammar::sensemake(view)
}

pub fn render_grammar_sensemake(report: &grammar::GrammarSenseReport) -> String {
    grammar::render_sensemake(report)
}

pub fn grammar_checklist(subject: grammar::ChecklistSubject) -> grammar::GrammarChecklistReport {
    grammar::checklist(subject)
}

pub fn render_grammar_checklist(report: &grammar::GrammarChecklistReport) -> String {
    grammar::render_checklist(report)
}

pub fn grammar_schema_json() -> error::Result<serde_json::Value> {
    grammar::schema_json()
}

pub fn render_grammar_schema_summary() -> String {
    grammar::render_schema_summary()
}

pub fn capability_store() -> error::Result<capability::StoreReport> {
    capability::store()
}

pub fn capability_add(
    options: capability::AddOptions,
) -> error::Result<capability::SeedWriteReport> {
    capability::add(options)
}

pub fn capability_update(
    options: capability::UpdateOptions,
) -> error::Result<capability::SeedWriteReport> {
    capability::update(options)
}

pub fn capability_list(domain: Option<&str>) -> error::Result<capability::SeedListReport> {
    capability::list(domain)
}

pub fn capability_search(
    options: capability::SearchOptions,
) -> error::Result<capability::SearchReport> {
    capability::search(options)
}

pub fn capability_inspect(
    id: &str,
    domain: Option<&str>,
) -> error::Result<capability::SeedInspectReport> {
    capability::inspect(id, domain)
}

pub fn capability_verify(
    id: &str,
    domain: Option<&str>,
) -> error::Result<capability::VerifyReport> {
    capability::verify(id, domain)
}

pub fn capability_prefer(
    options: capability::PreferOptions,
) -> error::Result<capability::SeedWriteReport> {
    capability::prefer(options)
}

pub fn capability_remove(
    id: &str,
    domain: Option<&str>,
) -> error::Result<capability::RemoveReport> {
    capability::remove(id, domain)
}

pub fn capability_scan() -> error::Result<capability::ScanReport> {
    capability::scan()
}
