use crate::error::{Error, Result};
use crate::model::{SkillSpec, Snippet};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn import_skill(path: &Path) -> Result<SkillSpec> {
    let content = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let analysis = SkillAnalysis::from_markdown(&content);

    let mut snippets = BTreeMap::new();
    snippets.insert(
        "source_summary".to_owned(),
        Snippet {
            text: analysis.summary(),
        },
    );

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "source".to_owned(),
        serde_yaml::Value::String(path.display().to_string()),
    );
    metadata.insert(
        "heading_count".to_owned(),
        serde_yaml::Value::Number(analysis.headings.len().into()),
    );
    metadata.insert(
        "command_block_count".to_owned(),
        serde_yaml::Value::Number(analysis.command_blocks.len().into()),
    );
    metadata.insert(
        "strong_directive_count".to_owned(),
        serde_yaml::Value::Number(analysis.directives.len().into()),
    );

    Ok(SkillSpec {
        schema: "skillspec/v0".to_owned(),
        id: "imported.skill".to_owned(),
        title: analysis
            .title
            .unwrap_or_else(|| "imported skill".to_owned()),
        description: "Imported SkillSpec scaffold from SKILL.md".to_owned(),
        applies_when: Vec::new(),
        entry: None,
        routes: Vec::new(),
        rules: Vec::new(),
        states: BTreeMap::new(),
        commands: BTreeMap::new(),
        snippets,
        closures: BTreeMap::new(),
        proof: None,
        tests: Vec::new(),
        review_required: vec![
            "Review extracted headings and convert decision-heavy prose into rules.".to_owned(),
            "Review command blocks and decide which should become command templates.".to_owned(),
            "Add scenario tests before trusting this structured skill.".to_owned(),
        ],
        metadata,
    })
}

#[derive(Debug)]
struct SkillAnalysis {
    title: Option<String>,
    headings: Vec<String>,
    command_blocks: Vec<String>,
    directives: Vec<String>,
}

impl SkillAnalysis {
    fn from_markdown(content: &str) -> Self {
        let mut title = None;
        let mut headings = Vec::new();
        let mut command_blocks = Vec::new();
        let mut directives = Vec::new();
        let mut in_code = false;
        let mut current_code = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("```") {
                if in_code {
                    command_blocks.push(current_code.join("\n"));
                    current_code.clear();
                    in_code = false;
                } else {
                    in_code = true;
                }
                continue;
            }

            if in_code {
                current_code.push(line.to_owned());
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("# ") {
                title.get_or_insert_with(|| heading.to_owned());
                headings.push(heading.to_owned());
            } else if trimmed.starts_with('#') {
                headings.push(trimmed.trim_start_matches('#').trim().to_owned());
            }

            let lower = trimmed.to_lowercase();
            if lower.contains("must")
                || lower.contains("never")
                || lower.contains("always")
                || lower.contains("do not")
                || lower.contains("prefer")
                || lower.contains("forbid")
            {
                directives.push(trimmed.to_owned());
            }
        }

        Self {
            title,
            headings,
            command_blocks,
            directives,
        }
    }

    fn summary(&self) -> String {
        format!(
            "Imported {} headings, {} command blocks, and {} strong directives.",
            self.headings.len(),
            self.command_blocks.len(),
            self.directives.len()
        )
    }
}
