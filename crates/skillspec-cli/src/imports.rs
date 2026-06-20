use crate::error::{Error, Result};
use crate::model::{ImportLoad, ImportRole, SkillSpec};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct ImportCheckReport {
    pub ok: bool,
    pub spec: String,
    pub imports: Vec<ImportCheckEntry>,
    pub load_order: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportCheckEntry {
    pub id: String,
    pub path: String,
    pub resolved_path: Option<String>,
    pub role: String,
    pub load: String,
    pub section: Option<String>,
    pub section_found: Option<bool>,
    pub requires: Vec<String>,
    pub status: ImportCheckStatus,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportCheckStatus {
    Ok,
    InvalidPath,
    MissingFile,
    MissingSection,
}

pub fn check(spec: &SkillSpec, spec_path: &Path) -> ImportCheckReport {
    let spec_dir = spec_path.parent().unwrap_or_else(|| Path::new("."));
    let imports = spec
        .imports
        .iter()
        .map(|(id, import)| {
            let mut entry = ImportCheckEntry {
                id: id.clone(),
                path: import.path.clone(),
                resolved_path: None,
                role: import_role_name(&import.role).to_owned(),
                load: import_load_name(&import.load).to_owned(),
                section: import.section.clone(),
                section_found: None,
                requires: import.requires.imports.clone(),
                status: ImportCheckStatus::Ok,
                error: None,
            };

            let import_path = Path::new(&import.path);
            if import_path.is_absolute() || import.path.contains("://") {
                entry.status = ImportCheckStatus::InvalidPath;
                entry.error = Some("import path must be local and relative".to_owned());
                return entry;
            }

            let resolved = spec_dir.join(import_path);
            entry.resolved_path = Some(display_path(&resolved));
            match fs::read_to_string(&resolved) {
                Ok(content) => {
                    if let Some(section) = &import.section {
                        let section_found = markdown_has_section(&content, section);
                        entry.section_found = Some(section_found);
                        if !section_found {
                            entry.status = ImportCheckStatus::MissingSection;
                            entry.error = Some(format!("section {section:?} not found"));
                        }
                    }
                }
                Err(error) => {
                    entry.status = ImportCheckStatus::MissingFile;
                    entry.error = Some(error.to_string());
                }
            }

            entry
        })
        .collect::<Vec<_>>();

    ImportCheckReport {
        ok: imports
            .iter()
            .all(|entry| matches!(entry.status, ImportCheckStatus::Ok)),
        spec: display_path(spec_path),
        imports,
        load_order: topological_load_order(spec),
    }
}

pub fn validate(spec: &SkillSpec, spec_path: &Path) -> Result<()> {
    let report = check(spec, spec_path);
    if report.ok {
        return Ok(());
    }

    let failures = report
        .imports
        .iter()
        .filter(|entry| !matches!(entry.status, ImportCheckStatus::Ok))
        .map(|entry| {
            format!(
                "{}: {}{}",
                entry.id,
                import_check_status_name(&entry.status),
                entry
                    .error
                    .as_ref()
                    .map(|error| format!(" ({error})"))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    Err(Error::InvalidInput {
        message: format!("import validation failed: {failures}"),
    })
}

fn topological_load_order(spec: &SkillSpec) -> Vec<String> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for id in spec.imports.keys() {
        visit(id, spec, &mut visiting, &mut visited, &mut order);
    }
    order
}

fn visit(
    id: &str,
    spec: &SkillSpec,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) {
    if visited.contains(id) || !visiting.insert(id.to_owned()) {
        return;
    }
    if let Some(import) = spec.imports.get(id) {
        for child in &import.requires.imports {
            visit(child, spec, visiting, visited, order);
        }
    }
    visiting.remove(id);
    visited.insert(id.to_owned());
    order.push(id.to_owned());
}

fn markdown_has_section(content: &str, section: &str) -> bool {
    let expected = normalize_heading(section);
    content.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with('#') && normalize_heading(trimmed.trim_start_matches('#')) == expected
    })
}

fn normalize_heading(heading: &str) -> String {
    heading.trim().trim_matches('#').trim().to_ascii_lowercase()
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn import_role_name(role: &ImportRole) -> &'static str {
    match role {
        ImportRole::Policy => "policy",
        ImportRole::Reference => "reference",
        ImportRole::Procedure => "procedure",
        ImportRole::Example => "example",
        ImportRole::Skill => "skill",
    }
}

fn import_load_name(load: &ImportLoad) -> &'static str {
    match load {
        ImportLoad::Always => "always",
        ImportLoad::OnDemand => "on_demand",
    }
}

fn import_check_status_name(status: &ImportCheckStatus) -> &'static str {
    match status {
        ImportCheckStatus::Ok => "ok",
        ImportCheckStatus::InvalidPath => "invalid_path",
        ImportCheckStatus::MissingFile => "missing_file",
        ImportCheckStatus::MissingSection => "missing_section",
    }
}
