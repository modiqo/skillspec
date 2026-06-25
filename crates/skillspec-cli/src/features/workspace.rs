use crate::error::{Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

mod compile;
mod converge;
mod import;
mod install;

pub use compile::{compile_workspace, render_compile_report, WorkspaceCompileReport};
pub use converge::{converge_workspace, render_converge_report, WorkspaceConvergeReport};
pub use import::{import_workspace, render_import_report, WorkspaceImportReport};
pub use install::{install_workspace, render_install_report, WorkspaceInstallReport};

pub const WORKSPACE_SCHEMA: &str = "skillspec/workspace/v0";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceManifest {
    pub schema: String,
    pub source_root: String,
    pub workspace_slug: String,
    pub output_root: String,
    pub packages: BTreeMap<String, WorkspacePackage>,
    #[serde(default)]
    pub references: Vec<WorkspaceReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspacePackage {
    pub package_id: String,
    pub path: String,
    pub kind: WorkspacePackageKind,
    pub entrypoint: String,
    pub public_name: String,
    pub install_slug: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePackageKind {
    Entry,
    Shared,
    Helper,
    Wrapper,
}

impl WorkspacePackageKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Shared => "shared",
            Self::Helper => "helper",
            Self::Wrapper => "wrapper",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceReference {
    pub from_package: String,
    pub source_path: String,
    pub line: usize,
    pub raw: String,
    pub resolved_path: String,
    #[serde(default)]
    pub target_package: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceMapReport {
    pub manifest_path: String,
    pub report_path: String,
    pub source_root: String,
    pub workspace_slug: String,
    pub package_count: usize,
    pub dependency_edges: Vec<WorkspaceDependencyEdge>,
    pub duplicate_public_names: Vec<WorkspaceDuplicate>,
    pub duplicate_install_slugs: Vec<WorkspaceDuplicate>,
    pub unresolved_references: Vec<WorkspaceReference>,
    pub references: Vec<WorkspaceReference>,
    pub next: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceDependencyEdge {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceDuplicate {
    pub value: String,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceValidationReport {
    pub ok: bool,
    pub manifest_path: String,
    pub package_count: usize,
    pub dependency_edges: Vec<WorkspaceDependencyEdge>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
struct SkillPackageSource {
    package_id: String,
    path: PathBuf,
    public_name: String,
    description: Option<String>,
    disable_model_invocation: bool,
}

pub fn guard_single_skill_source(path: &Path, command_name: &str) -> Result<()> {
    let source_root = source_root(path);
    let skill_files = discover_skill_files(&source_root)?;
    if skill_files.len() <= 1 {
        return Ok(());
    }

    Err(Error::InvalidInput {
        message: format!(
            "{command_name} expects one atomic skill package; found {} SKILL.md files under {}: {}. This is a workspace. Run `skillspec workspace map {} --out <build-dir>/skillspec.workspace.yml` first.",
            skill_files.len(),
            source_root.display(),
            display_paths(&skill_files),
            source_root.display()
        ),
    })
}

pub fn map_workspace(source_root: &Path, manifest_path: &Path) -> Result<WorkspaceMapReport> {
    let source_root = normalize_source_root(source_root)?;
    let packages = discover_packages(&source_root)?;
    if packages.is_empty() {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace map expected at least one SKILL.md under {}",
                source_root.display()
            ),
        });
    }

    let workspace_slug = workspace_slug(&source_root);
    let output_root = default_output_root(&source_root);
    let mut package_map = BTreeMap::new();
    for package in packages {
        let install_slug = format!("{workspace_slug}--{}", path_slug(&package.path));
        let kind = infer_package_kind(&package);
        package_map.insert(
            package.package_id.clone(),
            WorkspacePackage {
                package_id: package.package_id,
                path: path_to_string(&package.path),
                kind,
                entrypoint: "SKILL.md".to_owned(),
                public_name: package.public_name,
                install_slug,
                depends_on: Vec::new(),
            },
        );
    }

    let mut manifest = WorkspaceManifest {
        schema: WORKSPACE_SCHEMA.to_owned(),
        source_root: path_to_string(&source_root),
        workspace_slug,
        output_root: path_to_string(&output_root),
        packages: package_map,
        references: Vec::new(),
    };

    manifest.references = discover_references(&source_root, &manifest)?;
    infer_dependencies(&mut manifest);

    write_manifest(manifest_path, &manifest)?;
    let report_path = report_path_for(manifest_path);
    let report = map_report(manifest_path, &report_path, &manifest);
    write_text(&report_path, &render_map_report(&report, &manifest))?;
    Ok(report)
}

pub fn validate_workspace(manifest_path: &Path) -> Result<WorkspaceValidationReport> {
    let manifest = load_manifest(manifest_path)?;
    Ok(validate_manifest(manifest_path, &manifest))
}

pub fn load_manifest(path: &Path) -> Result<WorkspaceManifest> {
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_yaml::from_str(&content).map_err(|source| Error::ParseYaml {
        path: path.to_path_buf(),
        source,
    })
}

pub fn render_map_report(report: &WorkspaceMapReport, manifest: &WorkspaceManifest) -> String {
    let mut output = String::new();
    output.push_str("Workspace map\n\n");
    output.push_str(&format!("- source_root: {}\n", report.source_root));
    output.push_str(&format!("- workspace_slug: {}\n", report.workspace_slug));
    output.push_str(&format!("- packages: {}\n", report.package_count));
    output.push_str(&format!("- manifest: {}\n", report.manifest_path));
    output.push_str(&format!("- report: {}\n", report.report_path));
    output.push('\n');

    output.push_str("## Packages\n\n");
    for package in manifest.packages.values() {
        output.push_str(&format!(
            "- {} ({}) path={} public_name={} install_slug={}\n",
            package.package_id,
            package.kind.as_str(),
            package.path,
            package.public_name,
            package.install_slug
        ));
        if !package.depends_on.is_empty() {
            output.push_str(&format!(
                "  depends_on: {}\n",
                package.depends_on.join(", ")
            ));
        }
    }

    output.push_str("\n## Dependency Graph\n\n");
    if report.dependency_edges.is_empty() {
        output.push_str("- none inferred\n");
    } else {
        for edge in &report.dependency_edges {
            output.push_str(&format!("- {} -> {}\n", edge.from, edge.to));
        }
    }

    output.push_str("\n## Cross-Package References\n\n");
    let cross_refs = report
        .references
        .iter()
        .filter(|reference| reference.target_package.is_some())
        .collect::<Vec<_>>();
    if cross_refs.is_empty() {
        output.push_str("- none\n");
    } else {
        for reference in cross_refs {
            output.push_str(&format!(
                "- {}:{} {} -> {}\n",
                reference.source_path,
                reference.line,
                reference.raw,
                reference.target_package.as_deref().unwrap_or("unknown")
            ));
        }
    }

    if !report.unresolved_references.is_empty() {
        output.push_str("\n## Unresolved References\n\n");
        for reference in &report.unresolved_references {
            output.push_str(&format!(
                "- {}:{} {} resolved to {}\n",
                reference.source_path, reference.line, reference.raw, reference.resolved_path
            ));
        }
    }

    if !report.duplicate_public_names.is_empty() {
        output.push_str("\n## Duplicate Public Names\n\n");
        for duplicate in &report.duplicate_public_names {
            output.push_str(&format!(
                "- {}: {}\n",
                duplicate.value,
                duplicate.packages.join(", ")
            ));
        }
    }

    if !report.duplicate_install_slugs.is_empty() {
        output.push_str("\n## Duplicate Install Slugs\n\n");
        for duplicate in &report.duplicate_install_slugs {
            output.push_str(&format!(
                "- {}: {}\n",
                duplicate.value,
                duplicate.packages.join(", ")
            ));
        }
    }

    output.push_str("\n## Next\n\n");
    for next in &report.next {
        output.push_str(&format!("- {next}\n"));
    }
    output
}

pub fn render_validation_report(report: &WorkspaceValidationReport) -> String {
    let mut output = String::new();
    output.push_str("Workspace validate\n\n");
    output.push_str(&format!("- manifest: {}\n", report.manifest_path));
    output.push_str(&format!("- packages: {}\n", report.package_count));
    output.push_str(&format!(
        "- dependency_edges: {}\n",
        report.dependency_edges.len()
    ));
    output.push_str(&format!(
        "- status: {}\n",
        if report.ok { "ok" } else { "failed" }
    ));

    if !report.errors.is_empty() {
        output.push_str("\nErrors:\n");
        for error in &report.errors {
            output.push_str(&format!("- {error}\n"));
        }
    }

    if !report.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output
}

fn validate_manifest(path: &Path, manifest: &WorkspaceManifest) -> WorkspaceValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if manifest.schema != WORKSPACE_SCHEMA {
        errors.push(format!(
            "unsupported schema {:?}; expected {WORKSPACE_SCHEMA}",
            manifest.schema
        ));
    }

    let source_root = PathBuf::from(&manifest.source_root);
    if !source_root.is_dir() {
        errors.push(format!(
            "source_root is not a directory: {}",
            manifest.source_root
        ));
    }

    let mut package_ids = BTreeMap::<String, Vec<String>>::new();
    let mut paths = BTreeMap::<String, Vec<String>>::new();
    let mut install_slugs = BTreeMap::<String, Vec<String>>::new();
    let mut public_names = BTreeMap::<String, Vec<String>>::new();

    for (key, package) in &manifest.packages {
        package_ids
            .entry(package.package_id.clone())
            .or_default()
            .push(key.clone());
        paths
            .entry(package.path.clone())
            .or_default()
            .push(package.package_id.clone());
        install_slugs
            .entry(package.install_slug.clone())
            .or_default()
            .push(package.package_id.clone());
        public_names
            .entry(package.public_name.clone())
            .or_default()
            .push(package.package_id.clone());

        if &package.package_id != key {
            errors.push(format!(
                "package map key {key:?} does not match package_id {:?}",
                package.package_id
            ));
        }

        let Some(relative_package_path) = manifest_relative_path(&package.path) else {
            errors.push(format!(
                "package {} path must be a relative workspace path without parent components: {}",
                package.package_id, package.path
            ));
            continue;
        };

        let package_root = source_root.join(&relative_package_path);
        if !package_root.is_dir() {
            errors.push(format!(
                "package {} path is not a directory: {}",
                package.package_id,
                package_root.display()
            ));
        } else {
            match discover_skill_files(&package_root) {
                Ok(skill_files) if skill_files.len() == 1 => {}
                Ok(skill_files) => errors.push(format!(
                    "package {} must contain exactly one SKILL.md; found {} under {}: {}",
                    package.package_id,
                    skill_files.len(),
                    package_root.display(),
                    display_paths(&skill_files)
                )),
                Err(error) => errors.push(format!(
                    "failed to inspect package {}: {error}",
                    package.package_id
                )),
            }
        }

        for dependency in &package.depends_on {
            if dependency == &package.package_id {
                errors.push(format!("package {} depends on itself", package.package_id));
            }
            if !manifest.packages.contains_key(dependency) {
                errors.push(format!(
                    "package {} depends on unknown package {}",
                    package.package_id, dependency
                ));
            }
        }
    }

    push_duplicate_errors(&mut errors, "package_id", package_ids);
    push_duplicate_errors(&mut errors, "path", paths);
    push_duplicate_errors(&mut errors, "install_slug", install_slugs);
    push_duplicate_warnings(&mut warnings, "public_name", public_names);

    for reference in &manifest.references {
        let Some(target_package) = &reference.target_package else {
            warnings.push(format!(
                "unresolved reference {}:{} {} resolved to {}",
                reference.source_path, reference.line, reference.raw, reference.resolved_path
            ));
            continue;
        };
        let Some(package) = manifest.packages.get(&reference.from_package) else {
            errors.push(format!(
                "reference from unknown package {} at {}:{}",
                reference.from_package, reference.source_path, reference.line
            ));
            continue;
        };
        if target_package != &package.package_id && !package.depends_on.contains(target_package) {
            errors.push(format!(
                "uncovered cross-package reference: {}:{} {} targets {}, but {} does not depend_on it",
                reference.source_path,
                reference.line,
                reference.raw,
                target_package,
                package.package_id
            ));
        }
    }

    let cycles = dependency_cycles(manifest);
    for cycle in cycles {
        errors.push(format!("dependency cycle: {}", cycle.join(" -> ")));
    }

    let dependency_edges = dependency_edges(manifest);
    WorkspaceValidationReport {
        ok: errors.is_empty(),
        manifest_path: path_to_string(path),
        package_count: manifest.packages.len(),
        dependency_edges,
        errors,
        warnings,
    }
}

fn discover_packages(source_root: &Path) -> Result<Vec<SkillPackageSource>> {
    let mut skill_files = discover_skill_files(source_root)?;
    skill_files.sort();
    skill_files
        .into_iter()
        .map(|skill_path| package_from_skill_file(source_root, &skill_path))
        .collect()
}

fn package_from_skill_file(source_root: &Path, skill_path: &Path) -> Result<SkillPackageSource> {
    let package_root = skill_path.parent().unwrap_or(source_root);
    let relative_package = strip_prefix(package_root, source_root);
    let content = fs::read_to_string(skill_path).map_err(|source| Error::Read {
        path: skill_path.to_path_buf(),
        source,
    })?;
    let frontmatter = parse_frontmatter(skill_path, &content)?;
    let fallback_name = relative_package
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("skill")
        .to_owned();
    let public_name = frontmatter
        .get("name")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_owned())
        .unwrap_or(fallback_name);
    let description = frontmatter
        .get("description")
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let disable_model_invocation = frontmatter
        .get("disable-model-invocation")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let package_id = package_id_from_path(&relative_package);

    Ok(SkillPackageSource {
        package_id,
        path: relative_package,
        public_name,
        description,
        disable_model_invocation,
    })
}

fn discover_skill_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_skill_files(root, &mut files)?;
    Ok(files)
}

fn collect_skill_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if should_skip_dir(file_name) {
                continue;
            }
            collect_skill_files(&path, files)?;
        } else if file_name.eq_ignore_ascii_case("SKILL.md") {
            files.push(path);
        }
    }
    Ok(())
}

fn discover_references(
    source_root: &Path,
    manifest: &WorkspaceManifest,
) -> Result<Vec<WorkspaceReference>> {
    let reference_regex = Regex::new(r"\.\./[A-Za-z0-9_./-]+").expect("reference regex compiles");
    let slash_skill_regex =
        Regex::new(r"/[A-Za-z0-9][A-Za-z0-9_-]*").expect("slash skill regex compiles");
    let mut references = Vec::new();
    let package_paths = package_paths(manifest);
    let skill_name_targets = skill_name_targets(manifest);

    for package in manifest.packages.values() {
        let package_root = source_root.join(&package.path);
        let mut markdown_files = Vec::new();
        collect_package_markdown(&package_root, &package_root, &mut markdown_files)?;
        markdown_files.sort();
        for markdown_file in markdown_files {
            let relative_file = strip_prefix(&markdown_file, source_root);
            let content = fs::read_to_string(&markdown_file).map_err(|source| Error::Read {
                path: markdown_file.clone(),
                source,
            })?;
            let mut in_fence = false;
            for (index, line) in content.lines().enumerate() {
                if line.trim_start().starts_with("```") {
                    in_fence = !in_fence;
                    continue;
                }
                if in_fence {
                    continue;
                }
                for matched in reference_regex.find_iter(line) {
                    let raw = trim_reference(matched.as_str());
                    let Some(resolved) = resolve_relative_reference(&relative_file, raw) else {
                        continue;
                    };
                    let target_package = find_package_for_path(&package_paths, &resolved);
                    if target_package.as_deref() == Some(package.package_id.as_str()) {
                        continue;
                    }
                    references.push(WorkspaceReference {
                        from_package: package.package_id.clone(),
                        source_path: path_to_string(&relative_file),
                        line: index + 1,
                        raw: raw.to_owned(),
                        resolved_path: path_to_string(&resolved),
                        target_package,
                    });
                }
                for matched in slash_skill_regex.find_iter(line) {
                    if !is_standalone_skill_invocation(line, matched.start()) {
                        continue;
                    }
                    let raw = trim_reference(matched.as_str());
                    let Some((resolved_path, target_package)) =
                        find_package_for_skill_invocation(&skill_name_targets, raw)
                    else {
                        continue;
                    };
                    if target_package.as_deref() == Some(package.package_id.as_str()) {
                        continue;
                    }
                    references.push(WorkspaceReference {
                        from_package: package.package_id.clone(),
                        source_path: path_to_string(&relative_file),
                        line: index + 1,
                        raw: raw.to_owned(),
                        resolved_path,
                        target_package,
                    });
                }
            }
        }
    }

    references.sort_by(|left, right| {
        (&left.from_package, &left.source_path, left.line, &left.raw).cmp(&(
            &right.from_package,
            &right.source_path,
            right.line,
            &right.raw,
        ))
    });
    references.dedup_by(|left, right| {
        left.from_package == right.from_package
            && left.source_path == right.source_path
            && left.line == right.line
            && left.raw == right.raw
            && left.resolved_path == right.resolved_path
    });
    Ok(references)
}

fn collect_package_markdown(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if should_skip_dir(file_name) {
                continue;
            }
            if path != root && path.join("SKILL.md").is_file() {
                continue;
            }
            collect_package_markdown(root, &path, files)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn infer_dependencies(manifest: &mut WorkspaceManifest) {
    let mut dependencies = BTreeMap::<String, BTreeSet<String>>::new();
    for reference in &manifest.references {
        let Some(target) = &reference.target_package else {
            continue;
        };
        if target == &reference.from_package {
            continue;
        }
        dependencies
            .entry(reference.from_package.clone())
            .or_default()
            .insert(target.clone());
    }
    for (package_id, deps) in dependencies {
        if let Some(package) = manifest.packages.get_mut(&package_id) {
            package.depends_on = deps.into_iter().collect();
        }
    }
}

fn map_report(
    manifest_path: &Path,
    report_path: &Path,
    manifest: &WorkspaceManifest,
) -> WorkspaceMapReport {
    let duplicate_public_names = duplicates_by(manifest.packages.values(), |package| {
        package.public_name.clone()
    });
    let duplicate_install_slugs = duplicates_by(manifest.packages.values(), |package| {
        package.install_slug.clone()
    });
    let unresolved_references = manifest
        .references
        .iter()
        .filter(|reference| reference.target_package.is_none())
        .cloned()
        .collect::<Vec<_>>();

    WorkspaceMapReport {
        manifest_path: path_to_string(manifest_path),
        report_path: path_to_string(report_path),
        source_root: manifest.source_root.clone(),
        workspace_slug: manifest.workspace_slug.clone(),
        package_count: manifest.packages.len(),
        dependency_edges: dependency_edges(manifest),
        duplicate_public_names,
        duplicate_install_slugs,
        unresolved_references,
        references: manifest.references.clone(),
        next: vec![format!(
            "skillspec workspace validate {}",
            manifest_path.display()
        )],
    }
}

pub(super) fn dependency_edges(manifest: &WorkspaceManifest) -> Vec<WorkspaceDependencyEdge> {
    let mut edges = Vec::new();
    for package in manifest.packages.values() {
        for dependency in &package.depends_on {
            edges.push(WorkspaceDependencyEdge {
                from: package.package_id.clone(),
                to: dependency.clone(),
            });
        }
    }
    edges.sort_by(|left, right| (&left.from, &left.to).cmp(&(&right.from, &right.to)));
    edges
}

fn dependency_cycles(manifest: &WorkspaceManifest) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = BTreeSet::new();
    let mut stack = Vec::<String>::new();
    let mut in_stack = BTreeSet::new();
    for package_id in manifest.packages.keys() {
        visit_for_cycles(
            package_id,
            manifest,
            &mut visited,
            &mut stack,
            &mut in_stack,
            &mut cycles,
        );
    }
    cycles.sort();
    cycles.dedup();
    cycles
}

fn visit_for_cycles(
    package_id: &str,
    manifest: &WorkspaceManifest,
    visited: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
    in_stack: &mut BTreeSet<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    if in_stack.contains(package_id) {
        if let Some(index) = stack.iter().position(|id| id == package_id) {
            let mut cycle = stack[index..].to_vec();
            cycle.push(package_id.to_owned());
            cycles.push(cycle);
        }
        return;
    }
    if !visited.insert(package_id.to_owned()) {
        return;
    }
    in_stack.insert(package_id.to_owned());
    stack.push(package_id.to_owned());
    if let Some(package) = manifest.packages.get(package_id) {
        for dependency in &package.depends_on {
            if manifest.packages.contains_key(dependency) {
                visit_for_cycles(dependency, manifest, visited, stack, in_stack, cycles);
            }
        }
    }
    stack.pop();
    in_stack.remove(package_id);
}

fn duplicates_by<'a, F>(
    packages: impl Iterator<Item = &'a WorkspacePackage>,
    value: F,
) -> Vec<WorkspaceDuplicate>
where
    F: Fn(&WorkspacePackage) -> String,
{
    let mut groups = BTreeMap::<String, Vec<String>>::new();
    for package in packages {
        groups
            .entry(value(package))
            .or_default()
            .push(package.package_id.clone());
    }
    groups
        .into_iter()
        .filter_map(|(value, packages)| {
            (packages.len() > 1).then_some(WorkspaceDuplicate { value, packages })
        })
        .collect()
}

fn push_duplicate_errors(
    errors: &mut Vec<String>,
    field: &str,
    groups: BTreeMap<String, Vec<String>>,
) {
    for (value, packages) in groups {
        if packages.len() > 1 {
            errors.push(format!(
                "duplicate {field} {value:?}: {}",
                packages.join(", ")
            ));
        }
    }
}

fn push_duplicate_warnings(
    warnings: &mut Vec<String>,
    field: &str,
    groups: BTreeMap<String, Vec<String>>,
) {
    for (value, packages) in groups {
        if packages.len() > 1 {
            warnings.push(format!(
                "duplicate {field} {value:?}: {}",
                packages.join(", ")
            ));
        }
    }
}

fn package_paths(manifest: &WorkspaceManifest) -> Vec<(String, PathBuf)> {
    let mut paths = manifest
        .packages
        .values()
        .map(|package| (package.package_id.clone(), PathBuf::from(&package.path)))
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        right
            .1
            .components()
            .count()
            .cmp(&left.1.components().count())
            .then_with(|| left.0.cmp(&right.0))
    });
    paths
}

fn skill_name_targets(manifest: &WorkspaceManifest) -> BTreeMap<String, Vec<(String, PathBuf)>> {
    let mut targets = BTreeMap::<String, Vec<(String, PathBuf)>>::new();
    for package in manifest.packages.values() {
        push_skill_name_target(&mut targets, &package.public_name, package);
        if let Some(last_segment) = package.package_id.rsplit('.').next() {
            push_skill_name_target(&mut targets, last_segment, package);
        }
    }
    targets
}

fn push_skill_name_target(
    targets: &mut BTreeMap<String, Vec<(String, PathBuf)>>,
    name: &str,
    package: &WorkspacePackage,
) {
    let key = slugify(name);
    if key.is_empty() {
        return;
    }
    let entry = targets.entry(key).or_default();
    let value = (package.package_id.clone(), PathBuf::from(&package.path));
    if !entry.contains(&value) {
        entry.push(value);
    }
}

fn find_package_for_path(package_paths: &[(String, PathBuf)], target: &Path) -> Option<String> {
    package_paths
        .iter()
        .find(|(_, path)| path_is_prefix(path, target))
        .map(|(package_id, _)| package_id.clone())
}

fn find_package_for_skill_invocation(
    skill_name_targets: &BTreeMap<String, Vec<(String, PathBuf)>>,
    raw: &str,
) -> Option<(String, Option<String>)> {
    let name = raw.trim_start_matches('/');
    let key = slugify(name);
    let targets = skill_name_targets.get(&key)?;
    if targets.len() == 1 {
        let (package_id, package_path) = &targets[0];
        return Some((path_to_string(package_path), Some(package_id.clone())));
    }
    let packages = targets
        .iter()
        .map(|(package_id, _)| package_id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Some((format!("ambiguous skill name {raw}: {packages}"), None))
}

fn is_standalone_skill_invocation(line: &str, start: usize) -> bool {
    let Some(previous) = line[..start].chars().next_back() else {
        return true;
    };
    !(previous.is_ascii_alphanumeric() || matches!(previous, '.' | '/' | ':' | '_' | '-'))
}

fn path_is_prefix(prefix: &Path, path: &Path) -> bool {
    let prefix = prefix.components().collect::<Vec<_>>();
    let path = path.components().collect::<Vec<_>>();
    path.len() >= prefix.len()
        && path
            .iter()
            .zip(prefix.iter())
            .all(|(left, right)| left == right)
}

fn resolve_relative_reference(source_file: &Path, raw: &str) -> Option<PathBuf> {
    let parent = source_file.parent().unwrap_or_else(|| Path::new(""));
    normalize_relative_path(&parent.join(raw))
}

fn normalize_relative_path(path: &Path) -> Option<PathBuf> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => components.push(part.to_os_string()),
            Component::ParentDir => {
                components.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    let mut normalized = PathBuf::new();
    for component in components {
        normalized.push(component);
    }
    Some(normalized)
}

pub(super) fn manifest_relative_path(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if path.as_os_str().is_empty() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!normalized.as_os_str().is_empty()).then_some(normalized)
}

pub(super) fn output_package_dir(package: &WorkspacePackage, build_root: &Path) -> Result<PathBuf> {
    let relative = manifest_relative_path(&package.path).ok_or_else(|| Error::InvalidInput {
        message: format!(
            "package {} path must be a relative workspace path without parent components: {}",
            package.package_id, package.path
        ),
    })?;
    Ok(build_root.join(relative))
}

pub(super) fn topological_package_order(manifest: &WorkspaceManifest) -> Vec<String> {
    let mut dependents = BTreeMap::<String, BTreeSet<String>>::new();
    let mut remaining_dependency_counts = BTreeMap::<String, usize>::new();
    for package in manifest.packages.values() {
        remaining_dependency_counts.insert(package.package_id.clone(), package.depends_on.len());
        for dependency in &package.depends_on {
            dependents
                .entry(dependency.clone())
                .or_default()
                .insert(package.package_id.clone());
        }
    }

    let mut ready = remaining_dependency_counts
        .iter()
        .filter_map(|(package_id, count)| (*count == 0).then_some(package_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::new();
    while let Some(package_id) = ready.pop_first() {
        order.push(package_id.clone());
        if let Some(children) = dependents.get(&package_id) {
            for child in children {
                if let Some(count) = remaining_dependency_counts.get_mut(child) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
    }

    if order.len() != manifest.packages.len() {
        for package_id in manifest.packages.keys() {
            if !order.contains(package_id) {
                order.push(package_id.clone());
            }
        }
    }
    order
}

fn trim_reference(raw: &str) -> &str {
    raw.trim_end_matches(|ch: char| {
        matches!(
            ch,
            ')' | ']' | '}' | '"' | '\'' | '`' | ',' | '.' | ':' | ';'
        )
    })
}

fn parse_frontmatter(path: &Path, content: &str) -> Result<BTreeMap<String, serde_yaml::Value>> {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(rest) = normalized.strip_prefix("---") else {
        return Ok(BTreeMap::new());
    };
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"));
    let Some(rest) = rest else {
        return Ok(BTreeMap::new());
    };

    let mut yaml = String::new();
    for line in rest.lines() {
        if line.trim() == "---" {
            if yaml.trim().is_empty() {
                return Ok(BTreeMap::new());
            }
            return serde_yaml::from_str(&yaml).map_err(|source| Error::ParseYaml {
                path: path.to_path_buf(),
                source,
            });
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    Ok(BTreeMap::new())
}

fn infer_package_kind(package: &SkillPackageSource) -> WorkspacePackageKind {
    let name = package.public_name.to_ascii_lowercase();
    let description = package
        .description
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.contains("standards") || description.contains("standards package") {
        return WorkspacePackageKind::Shared;
    }
    if package.disable_model_invocation {
        return WorkspacePackageKind::Entry;
    }
    if description.contains("use when") || description.contains("test-driven") {
        return WorkspacePackageKind::Helper;
    }
    WorkspacePackageKind::Helper
}

fn package_id_from_path(path: &Path) -> String {
    let id = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str().map(slugify),
            _ => None,
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(".");
    if id.is_empty() {
        "skill".to_owned()
    } else {
        id
    }
}

fn path_slug(path: &Path) -> String {
    let slug = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str().map(slugify),
            _ => None,
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("--");
    if slug.is_empty() {
        "skill".to_owned()
    } else {
        slug
    }
}

fn workspace_slug(source_root: &Path) -> String {
    source_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(slugify)
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| short_hash(&path_to_string(source_root)))
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    slug.trim_matches('-').to_owned()
}

fn short_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn normalize_source_root(path: &Path) -> Result<PathBuf> {
    let source_root = source_root(path);
    if !source_root.is_dir() {
        return Err(Error::InvalidInput {
            message: format!(
                "workspace source root is not a directory: {}",
                path.display()
            ),
        });
    }
    Ok(source_root)
}

fn source_root(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

fn default_output_root(source_root: &Path) -> PathBuf {
    source_root.join(".skillspec/workspace-build")
}

fn strip_prefix(path: &Path, base: &Path) -> PathBuf {
    path.strip_prefix(base).unwrap_or(path).to_path_buf()
}

fn should_skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target" | "node_modules" | "vendor" | "dist" | "build"
        )
}

fn write_manifest(path: &Path, manifest: &WorkspaceManifest) -> Result<()> {
    let content = serde_yaml::to_string(manifest).map_err(|source| Error::RenderYaml {
        path: path.to_path_buf(),
        source,
    })?;
    write_text(path, &content)
}

pub(super) fn write_text(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn report_path_for(manifest_path: &Path) -> PathBuf {
    let file_name = manifest_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name}.report.md"))
        .unwrap_or_else(|| "skillspec.workspace.yml.report.md".to_owned());
    manifest_path.with_file_name(file_name)
}

fn display_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_relative_references() {
        let resolved = resolve_relative_reference(
            Path::new("code-review/SKILL.md"),
            "../coding-standards/SKILL.md",
        )
        .unwrap();
        assert_eq!(resolved, PathBuf::from("coding-standards/SKILL.md"));
    }

    #[test]
    fn path_slugs_use_double_dash_between_components() {
        assert_eq!(path_slug(Path::new("parentX/skill1")), "parentx--skill1");
    }
}
