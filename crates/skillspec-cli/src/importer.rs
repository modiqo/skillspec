use crate::error::{Error, Result};
use crate::model::{
    CodeBlock, CodeInlineSource, CodeKind, CodeProvenance, CodeRequires, CodeSafety, CodeSource,
    CommandRequires, CommandTemplate, Dependency, DependencyCheck, DependencyKind, Resource,
    ResourceRole, ResourceUse, ResourceUseKind, SkillSpec, Snippet,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn import_skill(path: &Path) -> Result<SkillSpec> {
    let source = SkillSource::read(path)?;
    let analysis = SkillAnalysis::from_source(&source);
    let dependencies = dependencies_from_analysis(&analysis);
    let commands = commands_from_blocks(&analysis.command_blocks);
    let (resources, code) = resources_and_code(&analysis);

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
        "source_kind".to_owned(),
        serde_yaml::Value::String(source.kind().to_owned()),
    );
    metadata.insert(
        "resource_count".to_owned(),
        serde_yaml::Value::Number(analysis.documents.len().into()),
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
        "code_block_count".to_owned(),
        serde_yaml::Value::Number(analysis.code_blocks.len().into()),
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
        elicitations: BTreeMap::new(),
        trace: None,
        dependencies,
        resources,
        code,
        artifacts: BTreeMap::new(),
        recipes: BTreeMap::new(),
        commands,
        snippets,
        closures: BTreeMap::new(),
        proof: None,
        tests: Vec::new(),
        review_required: vec![
            "Review extracted headings and convert decision-heavy prose into rules.".to_owned(),
            "Review command blocks and decide which should become command templates.".to_owned(),
            "Review extracted resources and code snippets; promote only intentional snippets into runnable recipes."
                .to_owned(),
            "Review inferred command dependencies and add permission/provision choices where needed."
                .to_owned(),
            "Add scenario tests before trusting this structured skill.".to_owned(),
        ],
        metadata,
    })
}

fn commands_from_blocks(command_blocks: &[String]) -> BTreeMap<String, CommandTemplate> {
    command_blocks
        .iter()
        .enumerate()
        .map(|(index, block)| {
            let dependencies = command_dependencies(block);
            (
                format!("command_block_{}", index + 1),
                CommandTemplate {
                    description: Some("Imported command block; review before use.".to_owned()),
                    template: block.trim().to_owned(),
                    safety: None,
                    requires: CommandRequires {
                        dependencies,
                        files: Vec::new(),
                        env: Vec::new(),
                        auth: Vec::new(),
                    },
                    parse: BTreeMap::new(),
                    success_when: BTreeMap::new(),
                },
            )
        })
        .collect()
}

fn resources_and_code(
    analysis: &SkillAnalysis,
) -> (BTreeMap<String, Resource>, BTreeMap<String, CodeBlock>) {
    let mut resources = BTreeMap::new();
    let mut code = BTreeMap::new();
    let code_ids_by_resource = code_ids_by_resource(&analysis.code_blocks);

    for document in &analysis.documents {
        let used_by = code_ids_by_resource
            .get(&document.resource_id)
            .into_iter()
            .flatten()
            .map(|id| ResourceUse {
                kind: ResourceUseKind::Code,
                id: id.clone(),
            })
            .chain(std::iter::once(ResourceUse {
                kind: ResourceUseKind::Snippet,
                id: "source_summary".to_owned(),
            }))
            .collect::<Vec<_>>();
        resources.insert(
            document.resource_id.clone(),
            Resource {
                path: document.relative_path.display().to_string(),
                role: document.role.clone(),
                description: Some(format!(
                    "Imported source material from {}.",
                    document.relative_path.display()
                )),
                used_by,
                load_when: Vec::new(),
            },
        );
    }

    for block in &analysis.code_blocks {
        code.insert(
            block.id.clone(),
            CodeBlock {
                language: block.language.clone(),
                kind: classify_code(&block.language),
                source: CodeSource::Inline(CodeInlineSource {
                    inline: block.text.clone(),
                }),
                provenance: Some(CodeProvenance {
                    resource: block.resource_id.clone(),
                    fence_index: Some(block.fence_index),
                    heading: block.heading.clone(),
                    line_start: Some(block.line_start),
                    line_end: Some(block.line_end),
                }),
                purpose: Some("Imported fenced code block; review before execution.".to_owned()),
                requires: CodeRequires {
                    dependencies: runtime_dependencies(&block.language),
                    resources: vec![block.resource_id.clone()],
                    artifacts: Vec::new(),
                },
                inputs: Vec::new(),
                outputs: Vec::new(),
                safety: CodeSafety::default(),
                use_when: Vec::new(),
            },
        );
    }

    (resources, code)
}

fn code_ids_by_resource(code_blocks: &[ImportedCodeBlock]) -> BTreeMap<String, Vec<String>> {
    let mut by_resource = BTreeMap::<String, Vec<String>>::new();
    for block in code_blocks {
        by_resource
            .entry(block.resource_id.clone())
            .or_default()
            .push(block.id.clone());
    }
    by_resource
}

fn classify_code(language: &str) -> CodeKind {
    if is_command_language(language) || is_runnable_language(language) {
        CodeKind::RunnableScript
    } else {
        CodeKind::Example
    }
}

fn is_runnable_language(language: &str) -> bool {
    matches!(
        language.trim().to_ascii_lowercase().as_str(),
        "python" | "py"
    )
}

fn dependencies_from_analysis(analysis: &SkillAnalysis) -> BTreeMap<String, Dependency> {
    let command_dependencies = analysis
        .command_blocks
        .iter()
        .flat_map(|block| command_dependencies(block))
        .chain(
            analysis
                .code_blocks
                .iter()
                .flat_map(|block| runtime_dependencies(&block.language)),
        )
        .collect::<BTreeSet<_>>();

    command_dependencies
        .into_iter()
        .map(|command| dependency_for_command(&command))
        .collect()
}

fn dependency_for_command(command: &str) -> (String, Dependency) {
    (
        command.to_owned(),
        Dependency {
            kind: DependencyKind::Cli,
            description: Some(format!(
                "Inferred CLI dependency from imported skill material: {command}"
            )),
            command: Some(command.to_owned()),
            path: None,
            env: None,
            check: Some(DependencyCheck {
                command: Some(command.to_owned()),
                path: None,
                env: None,
            }),
            permission: None,
            provision: None,
        },
    )
}

fn runtime_dependencies(language: &str) -> Vec<String> {
    match language.trim().to_ascii_lowercase().as_str() {
        "python" | "py" => vec!["python3".to_owned()],
        "javascript" | "js" => vec!["node".to_owned()],
        "typescript" | "ts" => vec!["deno".to_owned()],
        _ => Vec::new(),
    }
}

fn command_dependencies(block: &str) -> Vec<String> {
    let mut commands = BTreeSet::new();
    for line in block.lines() {
        for segment in split_shell_segments(line) {
            if let Some(command) = leading_command(segment) {
                commands.insert(command);
            }
        }
    }
    commands.into_iter().collect()
}

fn split_shell_segments(line: &str) -> Vec<&str> {
    line.split(['|', ';'])
        .flat_map(|segment| segment.split("&&"))
        .flat_map(|segment| segment.split("||"))
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn leading_command(segment: &str) -> Option<String> {
    let trimmed = segment
        .trim()
        .trim_start_matches('$')
        .trim_start_matches('>')
        .trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let mut tokens = trimmed.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        let token = token
            .trim_matches(|char: char| char == '(' || char == ')' || char == '{' || char == '}');
        if token.is_empty() {
            continue;
        }
        if token == "sudo" || token == "env" || token == "time" {
            continue;
        }
        if token.contains('=') && !token.starts_with('/') {
            continue;
        }
        if ignored_shell_word(token) {
            return None;
        }
        let command = token.rsplit('/').next().unwrap_or(token);
        if valid_command_name(command) {
            return Some(command.to_owned());
        }
        tokens.peek()?;
    }
    None
}

fn ignored_shell_word(token: &str) -> bool {
    matches!(
        token,
        "cd" | "export"
            | "alias"
            | "unalias"
            | "set"
            | "unset"
            | "source"
            | "."
            | "eval"
            | "if"
            | "then"
            | "else"
            | "fi"
            | "for"
            | "do"
            | "done"
            | "while"
            | "case"
            | "esac"
    )
}

fn valid_command_name(command: &str) -> bool {
    command
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && command
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '_' | '-' | '.' | '+'))
}

#[derive(Debug)]
struct SkillAnalysis {
    title: Option<String>,
    documents: Vec<SourceDocument>,
    headings: Vec<String>,
    code_blocks: Vec<ImportedCodeBlock>,
    command_blocks: Vec<String>,
    directives: Vec<String>,
}

impl SkillAnalysis {
    fn from_source(source: &SkillSource) -> Self {
        let mut title = None;
        let mut headings = Vec::new();
        let mut code_blocks = Vec::new();
        let mut command_blocks = Vec::new();
        let mut directives = Vec::new();

        for document in &source.documents {
            let document_analysis = DocumentAnalysis::from_document(document);
            title = title.or(document_analysis.title);
            headings.extend(document_analysis.headings);
            directives.extend(document_analysis.directives);
            for block in document_analysis.code_blocks {
                if is_command_language(&block.language) {
                    command_blocks.push(block.text.clone());
                }
                code_blocks.push(block);
            }
        }

        Self {
            title,
            documents: source.documents.clone(),
            headings,
            code_blocks,
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

#[derive(Clone, Debug)]
struct SkillSource {
    documents: Vec<SourceDocument>,
    is_directory: bool,
}

impl SkillSource {
    fn read(path: &Path) -> Result<Self> {
        if path.is_dir() {
            let mut markdown = Vec::new();
            collect_markdown(path, path, &mut markdown)?;
            markdown.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
            Ok(Self {
                documents: markdown,
                is_directory: true,
            })
        } else {
            let content = fs::read_to_string(path).map_err(|source| Error::Read {
                path: path.to_path_buf(),
                source,
            })?;
            let file_name = path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("SKILL.md"));
            Ok(Self {
                documents: vec![SourceDocument::new(file_name, content)],
                is_directory: false,
            })
        }
    }

    fn kind(&self) -> &'static str {
        if self.is_directory {
            "folder"
        } else {
            "file"
        }
    }
}

#[derive(Clone, Debug)]
struct SourceDocument {
    relative_path: PathBuf,
    resource_id: String,
    role: ResourceRole,
    content: String,
}

impl SourceDocument {
    fn new(relative_path: PathBuf, content: String) -> Self {
        let resource_id = resource_id_from_path(&relative_path);
        let role = resource_role(&relative_path);
        Self {
            relative_path,
            resource_id,
            role,
            content,
        }
    }
}

#[derive(Debug)]
struct DocumentAnalysis {
    title: Option<String>,
    headings: Vec<String>,
    code_blocks: Vec<ImportedCodeBlock>,
    directives: Vec<String>,
}

impl DocumentAnalysis {
    fn from_document(document: &SourceDocument) -> Self {
        let mut title = None;
        let mut headings = Vec::new();
        let mut code_blocks = Vec::new();
        let mut directives = Vec::new();
        let mut in_code = false;
        let mut current_code = Vec::new();
        let mut current_language = String::new();
        let mut current_heading = None::<String>;
        let mut code_start_line = 0_u32;
        let mut fence_index = 0_u32;

        for (line_index, line) in document.content.lines().enumerate() {
            let line_number = (line_index + 1) as u32;
            let trimmed = line.trim();
            if let Some(info) = trimmed.strip_prefix("```") {
                if in_code {
                    fence_index += 1;
                    let text = current_code.join("\n");
                    let id = code_id(&document.resource_id, fence_index);
                    code_blocks.push(ImportedCodeBlock {
                        id,
                        resource_id: document.resource_id.clone(),
                        language: normalize_language(&current_language),
                        text,
                        heading: current_heading.clone(),
                        fence_index,
                        line_start: code_start_line,
                        line_end: line_number,
                    });
                    current_code.clear();
                    current_language.clear();
                    in_code = false;
                } else {
                    current_language = info.split_whitespace().next().unwrap_or("").to_owned();
                    code_start_line = line_number + 1;
                    in_code = true;
                }
                continue;
            }

            if in_code {
                current_code.push(line.to_owned());
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("# ") {
                let heading = heading.to_owned();
                title.get_or_insert_with(|| heading.clone());
                current_heading = Some(heading.clone());
                headings.push(heading);
            } else if trimmed.starts_with('#') {
                let heading = trimmed.trim_start_matches('#').trim().to_owned();
                current_heading = Some(heading.clone());
                headings.push(heading);
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
            code_blocks,
            directives,
        }
    }
}

#[derive(Debug)]
struct ImportedCodeBlock {
    id: String,
    resource_id: String,
    language: String,
    text: String,
    heading: Option<String>,
    fence_index: u32,
    line_start: u32,
    line_end: u32,
}

fn collect_markdown(root: &Path, dir: &Path, documents: &mut Vec<SourceDocument>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with('.') || matches!(name.as_ref(), "target" | "node_modules") {
            continue;
        }
        if path.is_dir() {
            collect_markdown(root, &path, documents)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            let content = fs::read_to_string(&path).map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;
            let relative_path = path
                .strip_prefix(root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.clone());
            documents.push(SourceDocument::new(relative_path, content));
        }
    }
    Ok(())
}

fn resource_id_from_path(path: &Path) -> String {
    let without_extension = path.with_extension("");
    sanitize_identifier(&without_extension.display().to_string())
}

fn resource_role(path: &Path) -> ResourceRole {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match file_name.as_str() {
        "skill.md" => ResourceRole::SourceMaterial,
        "reference.md" | "references.md" => ResourceRole::Reference,
        "forms.md" | "procedure.md" | "procedures.md" => ResourceRole::RequiredProcedure,
        "example.md" | "examples.md" => ResourceRole::Example,
        _ => ResourceRole::SourceMaterial,
    }
}

fn code_id(resource_id: &str, fence_index: u32) -> String {
    format!("{resource_id}_code_{fence_index}")
}

fn normalize_language(language: &str) -> String {
    let language = language.trim().to_ascii_lowercase();
    if language.is_empty() {
        "text".to_owned()
    } else {
        language
    }
}

fn is_command_language(language: &str) -> bool {
    matches!(
        language.trim().to_ascii_lowercase().as_str(),
        "" | "sh" | "shell" | "bash" | "zsh" | "console" | "terminal"
    )
}

fn sanitize_identifier(input: &str) -> String {
    let mut output = String::new();
    for char in input.chars() {
        if char.is_ascii_alphanumeric() {
            output.push(char.to_ascii_lowercase());
        } else if matches!(char, '_' | '-' | '.' | '/') {
            output.push('_');
        }
    }
    let trimmed = output.trim_matches('_').to_owned();
    if trimmed
        .chars()
        .next()
        .is_some_and(|char| char.is_ascii_lowercase())
    {
        trimmed
    } else {
        format!("resource_{trimmed}")
    }
}
