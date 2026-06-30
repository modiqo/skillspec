use skillspec_core::error::{Error, Result};
use skillspec_core::model::SkillSpec;
use std::fs;
use std::path::Path;

const IMPORTED_SCAFFOLD_DESCRIPTION: &str = "Imported SkillSpec scaffold from SKILL.md";

#[derive(Clone, Debug)]
pub(crate) struct ReviewGate {
    blockers: Vec<String>,
}

impl ReviewGate {
    pub(crate) fn is_blocked(&self) -> bool {
        !self.blockers.is_empty()
    }

    pub(crate) fn message(&self) -> String {
        format!(
            "generated mechanical scaffold requires semantic promotion before converge, compile, or install: {}. Promote this package from source/SKILL_md.old and its source map by adding real activation/routes/rules/checks/tests/dependency decisions, then rerun workspace converge.",
            self.blockers.join("; ")
        )
    }
}

pub(crate) fn review_gate(output_dir: &Path, spec: &SkillSpec) -> Result<ReviewGate> {
    let mut blockers = Vec::new();
    if spec.id == "imported.skill" {
        blockers.push("generic imported.skill id".to_owned());
    }
    if spec.description.trim() == IMPORTED_SCAFFOLD_DESCRIPTION {
        blockers.push("importer scaffold description".to_owned());
    }
    if spec.routes.is_empty() {
        blockers.push("no executable routes".to_owned());
    }
    if spec.activation.is_none() && spec.applies_when.is_empty() && spec.rules.is_empty() {
        blockers.push("no activation, applies_when, or routing rules".to_owned());
    }
    if !spec.review_required.is_empty() {
        blockers.push("importer review_required notes remain".to_owned());
    }
    if spec
        .resources
        .values()
        .any(|resource| resource.path == "source/SKILL_md.old")
    {
        blockers.push("preserved prose source has not been promoted".to_owned());
    }

    if deps_toml_marks_import_scaffold(output_dir)? {
        blockers.push("deps.toml marks generated import scaffold as review_required".to_owned());
    }

    Ok(ReviewGate { blockers })
}

fn deps_toml_marks_import_scaffold(output_dir: &Path) -> Result<bool> {
    let path = output_dir.join("deps.toml");
    if !path.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(&path).map_err(|source| Error::Read { path, source })?;
    Ok(content
        .lines()
        .map(str::trim)
        .any(|line| line == "generated_by = \"skillspec import-skill\"")
        && content
            .lines()
            .map(str::trim)
            .any(|line| line == "review_required = true"))
}
