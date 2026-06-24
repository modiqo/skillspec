use crate::error::{Error, Result};
use crate::import_dependency_ledger;
use crate::model::{
    Artifact, ArtifactKind, CodeBlock, CodeFileSource, CodeInlineSource, CodeKind, CodeProvenance,
    CodeRequires, CodeSafety, CodeSource, CommandRequires, CommandTemplate, Dependency,
    DependencyCheck, DependencyKind, Import, ImportLoad, ImportRequires, ImportRole, ImportUse,
    ImportUseKind, Resource, ResourceRole, ResourceUse, ResourceUseKind, SkillSpec, Snippet,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub fn import_skill(path: &Path) -> Result<SkillSpec> {
    let source = SkillSource::read(path)?;
    let analysis = SkillAnalysis::from_source(&source);
    let mut dependencies = dependencies_from_analysis(&analysis);
    dependencies.insert(
        "dependency_ledger".to_owned(),
        Dependency {
            kind: DependencyKind::File,
            description: Some(
                "Generated dependency ledger for imported package evidence.".to_owned(),
            ),
            command: None,
            path: Some("deps.toml".to_owned()),
            env: None,
            check: Some(DependencyCheck {
                command: None,
                path: Some("deps.toml".to_owned()),
                env: None,
            }),
            permission: None,
            provision: None,
        },
    );
    let commands = commands_from_blocks(&analysis.command_blocks);
    let (imports, resources, code) = imports_resources_and_code(&analysis);
    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        "dependency_ledger".to_owned(),
        Artifact {
            kind: ArtifactKind::File,
            description: Some(
                "Generated dependency ledger preserving dependency evidence from imported skill material."
                    .to_owned(),
            ),
            path: Some("deps.toml".to_owned()),
            schema: None,
            produced_by: Vec::new(),
            consumed_by: Vec::new(),
        },
    );

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
        activation: None,
        applies_when: Vec::new(),
        entry: None,
        routes: Vec::new(),
        rules: Vec::new(),
        states: BTreeMap::new(),
        elicitations: BTreeMap::new(),
        trace: None,
        dependencies,
        imports,
        resources,
        code,
        artifacts,
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
            "Review deps.toml and preserve required, optional, script import, and inferred dependency evidence before proof or install."
                .to_owned(),
            "Add scenario tests before trusting this structured skill.".to_owned(),
        ],
        metadata,
    })
}

pub fn import_skill_for_output(path: &Path, out: &Path) -> Result<SkillSpec> {
    let mut spec = import_skill(path)?;
    let source_root = source_root(path);
    let out_dir = out.parent().unwrap_or_else(|| Path::new("."));
    for import in spec.imports.values_mut() {
        let source_path = source_root.join(&import.path);
        if let Some(relative) = relative_path(&source_path, out_dir) {
            import.path = relative.display().to_string();
        }
    }
    import_dependency_ledger::materialize(&spec, out_dir)?;
    materialize_inline_code_resources(&mut spec, out_dir)?;
    Ok(spec)
}

#[derive(Debug)]
struct CodeMaterialization {
    code_id: String,
    resource_id: String,
    relative_path: PathBuf,
    text: String,
    role: ResourceRole,
    fence_index: Option<u32>,
    heading: Option<String>,
}

fn materialize_inline_code_resources(spec: &mut SkillSpec, out_dir: &Path) -> Result<()> {
    let materializations = spec
        .code
        .iter()
        .filter_map(|(id, block)| {
            let CodeSource::Inline(source) = &block.source else {
                return None;
            };
            let extension = code_file_extension(&block.language);
            let file_name = format!("{}.{}", file_stem(id), extension);
            let relative_path = PathBuf::from("resources")
                .join("imported-code")
                .join(file_name);
            Some(CodeMaterialization {
                code_id: id.clone(),
                resource_id: format!("{id}_file"),
                relative_path,
                text: source.inline.clone(),
                role: code_resource_role(&block.kind),
                fence_index: block
                    .provenance
                    .as_ref()
                    .and_then(|provenance| provenance.fence_index),
                heading: block
                    .provenance
                    .as_ref()
                    .and_then(|provenance| provenance.heading.clone()),
            })
        })
        .collect::<Vec<_>>();

    for materialization in materializations {
        let destination = out_dir.join(&materialization.relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|source| Error::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&destination, &materialization.text).map_err(|source| Error::Write {
            path: destination,
            source,
        })?;

        let file = path_to_spec_string(&materialization.relative_path);
        spec.resources.insert(
            materialization.resource_id.clone(),
            Resource {
                path: file.clone(),
                role: materialization.role,
                description: Some(format!(
                    "Imported fenced code materialized from code block {}.",
                    materialization.code_id
                )),
                used_by: vec![ResourceUse {
                    kind: ResourceUseKind::Code,
                    id: materialization.code_id.clone(),
                }],
                load_when: Vec::new(),
            },
        );

        if let Some(block) = spec.code.get_mut(&materialization.code_id) {
            block.source = CodeSource::File(CodeFileSource {
                file,
                from_resource: Some(materialization.resource_id.clone()),
                fence_index: materialization.fence_index,
                heading: materialization.heading,
                sha256: None,
            });
            if !block
                .requires
                .resources
                .iter()
                .any(|id| id == &materialization.resource_id)
            {
                block
                    .requires
                    .resources
                    .push(materialization.resource_id.clone());
            }
        }
    }

    Ok(())
}

fn code_resource_role(kind: &CodeKind) -> ResourceRole {
    match kind {
        CodeKind::RunnableScript | CodeKind::Probe | CodeKind::Transform | CodeKind::Validator => {
            ResourceRole::Script
        }
        CodeKind::Example | CodeKind::Troubleshooting | CodeKind::Reference => {
            ResourceRole::Example
        }
    }
}

fn code_file_extension(language: &str) -> &'static str {
    match language.trim().to_ascii_lowercase().as_str() {
        "bash" | "sh" | "shell" | "zsh" => "sh",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "json" => "json",
        "yaml" | "yml" => "yml",
        "markdown" | "md" => "md",
        _ => "txt",
    }
}

fn file_stem(id: &str) -> String {
    id.chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || matches!(char, '-' | '_') {
                char
            } else {
                '_'
            }
        })
        .collect()
}

fn path_to_spec_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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

fn relative_path(target: &Path, base: &Path) -> Option<PathBuf> {
    let target = target.canonicalize().ok()?;
    let base = base.canonicalize().ok()?;
    let target_components = target.components().collect::<Vec<_>>();
    let base_components = base.components().collect::<Vec<_>>();
    if target_components.first() != base_components.first() {
        return None;
    }

    let mut common = 0;
    while common < target_components.len()
        && common < base_components.len()
        && target_components[common] == base_components[common]
    {
        common += 1;
    }

    let mut relative = PathBuf::new();
    for component in &base_components[common..] {
        if matches!(component, Component::Normal(_)) {
            relative.push("..");
        }
    }
    for component in &target_components[common..] {
        relative.push(component.as_os_str());
    }
    Some(relative)
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
                        resources: Vec::new(),
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

fn imports_resources_and_code(
    analysis: &SkillAnalysis,
) -> (
    BTreeMap<String, Import>,
    BTreeMap<String, Resource>,
    BTreeMap<String, CodeBlock>,
) {
    let mut imports = BTreeMap::new();
    let mut resources = BTreeMap::new();
    let mut code = BTreeMap::new();
    let code_ids_by_resource = code_ids_by_resource(&analysis.code_blocks);
    let import_document_ids = analysis
        .documents
        .iter()
        .filter(|document| document.is_import_candidate())
        .map(|document| document.resource_id.clone())
        .collect::<BTreeSet<_>>();

    for document in &analysis.documents {
        if document.is_import_candidate() {
            let used_by = code_ids_by_resource
                .get(&document.resource_id)
                .into_iter()
                .flatten()
                .map(|id| ImportUse {
                    kind: ImportUseKind::Code,
                    id: id.clone(),
                })
                .collect::<Vec<_>>();
            imports.insert(
                document.resource_id.clone(),
                Import {
                    path: document.relative_path.display().to_string(),
                    role: import_role(&document.relative_path),
                    description: Some(format!(
                        "Imported runtime guidance from {}.",
                        document.relative_path.display()
                    )),
                    section: None,
                    load: ImportLoad::OnDemand,
                    requires: ImportRequires::default(),
                    used_by,
                    load_when: vec![
                        "Load when the active route, rule, recipe, or code path needs this guidance."
                            .to_owned(),
                    ],
                },
            );
            continue;
        }

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
                    resource: (!import_document_ids.contains(&block.resource_id))
                        .then(|| block.resource_id.clone()),
                    import: import_document_ids
                        .contains(&block.resource_id)
                        .then(|| block.resource_id.clone()),
                    fence_index: Some(block.fence_index),
                    heading: block.heading.clone(),
                    line_start: Some(block.line_start),
                    line_end: Some(block.line_end),
                }),
                purpose: Some("Imported fenced code block; review before execution.".to_owned()),
                requires: CodeRequires {
                    dependencies: runtime_dependencies(&block.language),
                    imports: if import_document_ids.contains(&block.resource_id) {
                        vec![block.resource_id.clone()]
                    } else {
                        Vec::new()
                    },
                    resources: if import_document_ids.contains(&block.resource_id) {
                        Vec::new()
                    } else {
                        vec![block.resource_id.clone()]
                    },
                    artifacts: Vec::new(),
                },
                inputs: Vec::new(),
                outputs: Vec::new(),
                safety: CodeSafety::default(),
                use_when: Vec::new(),
            },
        );
    }

    (imports, resources, code)
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

    fn is_import_candidate(&self) -> bool {
        self.relative_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| !name.eq_ignore_ascii_case("SKILL.md"))
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

fn import_role(path: &Path) -> ImportRole {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match file_name.as_str() {
        "skill.md" => ImportRole::Skill,
        "procedure.md" | "procedures.md" | "forms.md" => ImportRole::Procedure,
        "example.md" | "examples.md" => ImportRole::Example,
        _ => ImportRole::Reference,
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
