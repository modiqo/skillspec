use crate::error::{Error, Result};
use crate::source_map::{
    self, SourceClassificationKind, SourceFileKind, SourceFileLoadStatus, SourceMap,
    SourceReferenceKind,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const LARGE_BODY_LINES: usize = 500;
const LARGE_BODY_TOKENS: usize = 5_000;
const PRIMACY_LINE_PERCENT: usize = 60;

#[derive(Clone, Debug, Serialize)]
pub struct DoctorReport {
    pub target: String,
    pub source_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_from: Option<String>,
    pub verdict: String,
    pub structural_score: u8,
    pub large_surface_percentage: u8,
    pub surface: SurfaceReport,
    pub counts: DoctorCounts,
    pub issues: Vec<DoctorIssue>,
    pub basis: Vec<DoctorBasis>,
    pub suggested_next_steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SurfaceReport {
    pub frontmatter_bytes: usize,
    pub frontmatter_lines: usize,
    pub activation_bytes: usize,
    pub activation_lines: usize,
    pub activation_estimated_tokens: usize,
    pub deferred_bytes: usize,
    pub deferred_files: usize,
    pub unmapped_files: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorCounts {
    pub markdown_files: usize,
    pub code_files: usize,
    pub manifest_files: usize,
    pub code_blocks_in_skill: usize,
    pub unlabeled_code_blocks_in_skill: usize,
    pub modal_obligations: usize,
    pub late_modal_obligations: usize,
    pub numbered_steps: usize,
    pub dependency_mentions: usize,
    pub missing_local_references: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorIssue {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub evidence: String,
    pub basis: Vec<String>,
    pub remediation: String,
    pub score_penalty: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorBasis {
    pub id: String,
    pub source: String,
    pub claim: String,
}

struct SkillBody {
    path: PathBuf,
    file_id: String,
    body: String,
    frontmatter: String,
    body_start_line: usize,
}

#[derive(Debug)]
struct RemoteSkillSource {
    repo_url: String,
    branch: Option<String>,
    path: String,
}

struct StagedRemote {
    root: PathBuf,
    skill_path: PathBuf,
}

impl Drop for StagedRemote {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn inspect_target(target: &str) -> Result<DoctorReport> {
    let target_path = Path::new(target);
    if target_path.exists() {
        let mut report = inspect(target_path)?;
        report.target = target.to_owned();
        return Ok(report);
    }

    let Some(remote) = remote_from_target(target)? else {
        return Err(Error::InvalidInput {
            message: format!(
                "doctor target {target:?} does not exist locally; remote doctor currently supports public GitHub single skill folder URLs such as https://github.com/<owner>/<repo>/tree/<branch>/<path>"
            ),
        });
    };
    let staged = stage_remote(&remote)?;
    let mut report = match inspect(&staged.skill_path) {
        Ok(report) => report,
        Err(error) => return Err(rewrite_remote_error(error, &staged.skill_path, target)),
    };
    rewrite_remote_locations(&mut report, &staged.skill_path, target);
    report.target = target.to_owned();
    report.source_kind = "remote_github".to_owned();
    report.staged_from = Some(remote.repo_url);
    Ok(report)
}

pub fn inspect(path: &Path) -> Result<DoctorReport> {
    let map = source_map::build(path)?;
    let source_root = PathBuf::from(&map.source_root);
    let skill = load_skill_body(&map, &source_root)?;
    let surface = surface_report(&map, &skill);
    let counts = counts(&map, &skill);
    let has_skill_spec = source_root.join("skill.spec.yml").exists();
    let has_deps_toml = source_root.join("deps.toml").exists();
    let has_tests = source_root.join(".skillspec").exists() || has_skill_spec;
    let mut issues = Vec::new();

    let large_surface_percentage = percentage(
        surface.activation_bytes,
        surface.frontmatter_bytes + surface.activation_bytes + surface.deferred_bytes,
    );

    if surface.activation_lines > LARGE_BODY_LINES
        || surface.activation_estimated_tokens > LARGE_BODY_TOKENS
    {
        let penalty = if surface.activation_lines > LARGE_BODY_LINES * 2
            || surface.activation_estimated_tokens > LARGE_BODY_TOKENS * 2
        {
            24
        } else {
            16
        };
        issues.push(with_location(
            issue(
                "large_activation_body",
                "high",
                "Large activation-loaded instruction body",
                format!(
                    "SKILL.md activation body is {} lines / approximately {} tokens; this exceeds the {} line or {} token guidance used by skill authoring practice.",
                    surface.activation_lines,
                    surface.activation_estimated_tokens,
                    LARGE_BODY_LINES,
                    LARGE_BODY_TOKENS
                ),
                vec![
                    "reliability_gap_instruction_density",
                    "contract_trace_activation_adherence_enforcement",
                ],
                "Split detail into referenced files, then use `skillspec source map` and `skillspec import-skill` so load-bearing behavior can be reviewed progressively.",
                penalty,
            ),
            format!("{}:{}", skill.path.display(), skill.body_start_line),
        ));
    }

    if large_surface_percentage >= 75 {
        issues.push(with_location(
            issue(
                "large_activation_surface",
                "high",
                "Most package text loads at activation",
                format!(
                    "{}% of loaded text surface is in SKILL.md activation body; little is deferred behind task-specific references.",
                    large_surface_percentage
                ),
                vec![
                    "reliability_gap_metadata_context_pressure",
                    "reliability_gap_instruction_density",
                ],
                "Move long references, examples, and code into referenced files and keep the activation body as a compact router.",
                18,
            ),
            skill.path.display().to_string(),
        ));
    } else if large_surface_percentage >= 50 {
        issues.push(with_location(
            issue(
                "medium_activation_surface",
                "medium",
                "Activation surface is still broad",
                format!(
                    "{}% of loaded text surface is in SKILL.md activation body.",
                    large_surface_percentage
                ),
                vec!["reliability_gap_instruction_density"],
                "Review whether examples and procedural detail can be deferred into references.",
                8,
            ),
            skill.path.display().to_string(),
        ));
    }

    if counts.modal_obligations >= 12 || counts.numbered_steps >= 12 {
        issues.push(with_location(
            issue(
                "instruction_density",
                "high",
                "Dense load-bearing prose",
                format!(
                    "Found {} modal obligation spans and {} numbered steps in the activation body.",
                    counts.modal_obligations, counts.numbered_steps
                ),
                vec![
                    "reliability_gap_instruction_density",
                    "contract_trace_activation_adherence_enforcement",
                ],
                "Promote route choices, forbids, elicitations, dependencies, and tests into `skill.spec.yml` so they can be checked instead of remembered.",
                14,
            ),
            skill.path.display().to_string(),
        ));
    }

    if counts.late_modal_obligations > 0 {
        issues.push(with_location(
            issue(
                "primacy_bias_late_obligations",
                "medium",
                "Late load-bearing instructions are exposed to primacy bias",
                format!(
                    "{} modal obligation span(s) appear after the first {}% of the activation body.",
                    counts.late_modal_obligations, PRIMACY_LINE_PERCENT
                ),
                vec!["reliability_gap_instruction_density"],
                "Move late obligations into earlier route/rule summaries or structured checks; do not rely on a model remembering buried instructions.",
                10,
            ),
            skill.path.display().to_string(),
        ));
    }

    if counts.code_blocks_in_skill > 0 {
        let code_bytes = skill_code_bytes(&map, &skill.file_id);
        let code_percent = percentage(code_bytes, surface.activation_bytes);
        issues.push(with_location(
            issue(
                "code_mixed_with_activation_instructions",
                "medium",
                "Code is mixed into the activation instruction body",
                format!(
                    "Found {} fenced code block(s) in SKILL.md, accounting for about {}% of activation bytes.",
                    counts.code_blocks_in_skill, code_percent
                ),
                vec![
                    "reliability_gap_no_execution_guarantees",
                    "contract_trace_static_well_formedness",
                ],
                "Move executable code into scripts/resources or structured `code` entries and state whether snippets are executable, examples, or reference material.",
                12,
            ),
            skill.path.display().to_string(),
        ));
    }

    if counts.unlabeled_code_blocks_in_skill > 0 {
        issues.push(with_location(
            issue(
                "unlabeled_code_fences",
                "medium",
                "Code fence language is ambiguous",
                format!(
                    "{} code fence(s) in SKILL.md omit a language label.",
                    counts.unlabeled_code_blocks_in_skill
                ),
                vec!["reliability_gap_no_execution_guarantees"],
                "Label code fences and classify each one as executable code, command example, or non-executable reference.",
                6,
            ),
            skill.path.display().to_string(),
        ));
    }

    if operational_prose(&skill.body) && !has_skill_spec {
        issues.push(with_location(
            issue(
                "ambiguous_execution_substrate",
                "high",
                "Operational prose lacks a structured execution contract",
                "The activation body tells the model to use/run/create/fetch/click/install or similar, but there is no SkillSpec route, tool boundary, command template, or trace vocabulary.".to_owned(),
                vec![
                    "contract_trace_activation_adherence_enforcement",
                    "contract_trace_unproven_verdict",
                ],
                "Add a SkillSpec contract with routes, phase tool boundaries, commands, scenario tests, and trace requirements.",
                18,
            ),
            skill.path.display().to_string(),
        ));
    }

    if (counts.dependency_mentions > 0
        || counts.code_files > 0
        || counts.manifest_files > 0
        || counts.code_blocks_in_skill > 0)
        && !has_deps_toml
    {
        issues.push(with_location(
            issue(
                "implicit_dependency_contract",
                "high",
                "Dependencies are implicit",
                format!(
                    "Detected {} dependency mention(s), {} code file(s), {} manifest file(s), and {} code block(s), but no deps.toml ledger.",
                    counts.dependency_mentions,
                    counts.code_files,
                    counts.manifest_files,
                    counts.code_blocks_in_skill
                ),
                vec!["reliability_gap_implicit_environment_contract"],
                "Create `deps.toml` and preserve dependency authority, local status, install risk, and degraded proof impact before proof or install.",
                16,
            ),
            source_root.display().to_string(),
        ));
    }

    if counts.missing_local_references > 0 {
        issues.push(with_location(
            issue(
                "missing_referenced_files",
                "medium",
                "Referenced local files are missing",
                format!(
                    "{} local Markdown reference(s) did not resolve to files in the skill package.",
                    counts.missing_local_references
                ),
                vec!["contract_trace_static_well_formedness"],
                "Fix broken links or preserve the missing files before import, install, or release.",
                8,
            ),
            skill.path.display().to_string(),
        ));
    }

    if surface.unmapped_files > 0 {
        issues.push(with_location(
            issue(
                "unmapped_package_surface",
                "medium",
                "Package files are present but not clearly reachable",
                format!(
                    "{} non-SKILL file(s) are present without an explicit local reference from Markdown.",
                    surface.unmapped_files
                ),
                vec!["contract_trace_static_well_formedness"],
                "Declare package-local files as imports, resources, code sources, artifacts, or dependency ledgers during SkillSpec porting.",
                8,
            ),
            source_root.display().to_string(),
        ));
    }

    if !has_skill_spec {
        issues.push(with_location(
            issue(
                "missing_behavior_contract",
                "high",
                "No machine-checkable behavior contract",
                "No skill.spec.yml was found, so route choices, forbids, tool boundaries, dependency checks, scenario tests, and trace expectations are not falsifiable.".to_owned(),
                vec![
                    "reliability_gap_unfilled_requirement",
                    "contract_trace_behavioral_contract",
                ],
                "Run `skillspec source map`, then `skillspec import-skill`, and complete the generated contract before install or proof.",
                20,
            ),
            source_root.display().to_string(),
        ));
    }

    if !has_tests {
        issues.push(with_location(
            issue(
                "missing_trace_proof_surface",
                "medium",
                "Runtime success would be unproven",
                "No trace/progress/test surface was found, so a successful run would not prove which obligations actually executed.".to_owned(),
                vec!["contract_trace_unproven_verdict"],
                "Add scenario tests and trace/progress requirements; report `unproven` when evidence is absent instead of treating no error as success.",
                10,
            ),
            source_root.display().to_string(),
        ));
    }

    issues.sort_by_key(|issue| severity_rank(&issue.severity));
    let penalty = issues
        .iter()
        .map(|issue| usize::from(issue.score_penalty))
        .sum::<usize>()
        .min(100);
    let structural_score = u8::try_from(100usize.saturating_sub(penalty)).unwrap_or(0);
    let verdict = verdict(structural_score);

    Ok(DoctorReport {
        target: source_root.display().to_string(),
        source_kind: "local".to_owned(),
        staged_from: None,
        verdict,
        structural_score,
        large_surface_percentage,
        surface,
        counts,
        issues,
        basis: basis(),
        suggested_next_steps: next_steps(has_skill_spec, has_deps_toml),
    })
}

pub fn render(report: &DoctorReport) -> String {
    let mut output = String::new();
    output.push_str(&format!("skillspec doctor: {}\n", report.target));
    output.push_str(&format!("source_kind: {}\n", report.source_kind));
    if let Some(staged_from) = &report.staged_from {
        output.push_str(&format!("staged_from: {staged_from}\n"));
    }
    output.push_str(&format!("verdict: {}\n", report.verdict));
    output.push_str(&format!(
        "structural_score: {}/100\n",
        report.structural_score
    ));
    output.push_str(&format!(
        "large_surface: {}% activation-loaded\n\n",
        report.large_surface_percentage
    ));
    output.push_str("surface:\n");
    output.push_str(&format!(
        "- frontmatter: {} line(s), {} byte(s)\n",
        report.surface.frontmatter_lines, report.surface.frontmatter_bytes
    ));
    output.push_str(&format!(
        "- activation: {} line(s), {} byte(s), ~{} token(s)\n",
        report.surface.activation_lines,
        report.surface.activation_bytes,
        report.surface.activation_estimated_tokens
    ));
    output.push_str(&format!(
        "- deferred: {} file(s), {} byte(s)\n",
        report.surface.deferred_files, report.surface.deferred_bytes
    ));
    output.push_str(&format!(
        "- unmapped package files: {}\n\n",
        report.surface.unmapped_files
    ));
    output.push_str("counts:\n");
    output.push_str(&format!(
        "- modal obligations: {} (late: {})\n",
        report.counts.modal_obligations, report.counts.late_modal_obligations
    ));
    output.push_str(&format!(
        "- numbered steps: {}\n",
        report.counts.numbered_steps
    ));
    output.push_str(&format!(
        "- code blocks in SKILL.md: {} (unlabeled: {})\n",
        report.counts.code_blocks_in_skill, report.counts.unlabeled_code_blocks_in_skill
    ));
    output.push_str(&format!(
        "- dependency mentions: {}\n",
        report.counts.dependency_mentions
    ));
    output.push_str(&format!(
        "- missing local references: {}\n\n",
        report.counts.missing_local_references
    ));

    if report.issues.is_empty() {
        output.push_str("issues: none detected by static structure\n");
    } else {
        output.push_str("issues:\n");
        for issue in &report.issues {
            output.push_str(&format!(
                "- [{}] {}: {}\n",
                issue.severity, issue.id, issue.title
            ));
            output.push_str(&format!("  evidence: {}\n", issue.evidence));
            output.push_str(&format!("  basis: {}\n", issue.basis.join(", ")));
            output.push_str(&format!("  remediation: {}\n", issue.remediation));
        }
    }

    output.push_str("\nbasis:\n");
    for basis in &report.basis {
        output.push_str(&format!(
            "- {}: {} ({})\n",
            basis.id, basis.claim, basis.source
        ));
    }
    output.push_str("\nnext:\n");
    for step in &report.suggested_next_steps {
        output.push_str(&format!("- {step}\n"));
    }
    trim_trailing_newline(&mut output);
    output
}

fn remote_from_target(target: &str) -> Result<Option<RemoteSkillSource>> {
    let trimmed = target.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let github_path = without_scheme
        .strip_prefix("github.com/")
        .or_else(|| trimmed.strip_prefix("github:"));
    let Some(github_path) = github_path else {
        if looks_like_github_shorthand(trimmed) {
            return github_shorthand(trimmed).map(Some);
        }
        return Ok(None);
    };
    let parts = github_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 3 {
        return Err(Error::InvalidInput {
            message: "remote doctor requires a GitHub owner/repo/path single skill folder target"
                .to_owned(),
        });
    }
    let owner = parts[0];
    let repo = parts[1].trim_end_matches(".git");
    if parts.get(2) == Some(&"blob") {
        return Err(Error::InvalidInput {
            message: "remote doctor expects a skill folder URL, not a blob/SKILL.md URL".to_owned(),
        });
    }
    let (branch, path_parts) = if parts.get(2) == Some(&"tree") {
        if parts.len() < 5 {
            return Err(Error::InvalidInput {
                message: "GitHub tree URL must include a branch and a single skill folder path"
                    .to_owned(),
            });
        }
        (Some(parts[3].to_owned()), &parts[4..])
    } else {
        (None, &parts[2..])
    };
    let path = path_parts.join("/");
    if path.is_empty() || is_skill_file_path(&path) {
        return Err(Error::InvalidInput {
            message: "remote doctor expects a single skill folder path, not a parent repo or SKILL.md blob"
                .to_owned(),
        });
    }
    Ok(Some(RemoteSkillSource {
        repo_url: format!("https://github.com/{owner}/{repo}.git"),
        branch,
        path,
    }))
}

fn looks_like_github_shorthand(target: &str) -> bool {
    let parts = target
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    parts.len() >= 3
        && !target.starts_with('/')
        && !target.starts_with('.')
        && !target.contains("://")
        && parts[0]
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        && parts[1]
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn github_shorthand(target: &str) -> Result<RemoteSkillSource> {
    let parts = target
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let path = parts[2..].join("/");
    if path.is_empty() || is_skill_file_path(&path) {
        return Err(Error::InvalidInput {
            message: "remote doctor shorthand expects owner/repo/<single-skill-folder>".to_owned(),
        });
    }
    Ok(RemoteSkillSource {
        repo_url: format!("https://github.com/{}/{}.git", parts[0], parts[1]),
        branch: None,
        path,
    })
}

fn is_skill_file_path(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
}

fn rewrite_remote_locations(report: &mut DoctorReport, staged_skill_path: &Path, target: &str) {
    let Some(prefix) = staged_skill_path.to_str() else {
        return;
    };
    let target = target.trim_end_matches('/');
    for issue in &mut report.issues {
        if let Some(location) = &mut issue.location {
            if let Some(suffix) = location.strip_prefix(prefix) {
                *location = format!("{target}{suffix}");
            }
        }
    }
}

fn rewrite_remote_error(error: Error, staged_skill_path: &Path, target: &str) -> Error {
    let Error::InvalidInput { message } = error else {
        return error;
    };
    let Some(prefix) = staged_skill_path.to_str() else {
        return Error::InvalidInput { message };
    };
    Error::InvalidInput {
        message: message.replace(prefix, target.trim_end_matches('/')),
    }
}

fn stage_remote(remote: &RemoteSkillSource) -> Result<StagedRemote> {
    let root = std::env::temp_dir().join(format!(
        "skillspec-doctor-{}-{}",
        std::process::id(),
        unique_nanos()
    ));
    let clone_dir = root.join("repo");
    fs::create_dir_all(&root).map_err(|source| Error::Write {
        path: root.clone(),
        source,
    })?;

    let mut clone = Command::new("git");
    clone
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--filter=blob:none")
        .arg("--sparse");
    if let Some(branch) = &remote.branch {
        clone.arg("--branch").arg(branch);
    }
    clone.arg(&remote.repo_url).arg(&clone_dir);
    run_git(clone, "clone remote skill repository")?;

    let mut sparse = Command::new("git");
    sparse
        .arg("-C")
        .arg(&clone_dir)
        .arg("sparse-checkout")
        .arg("set")
        .arg(&remote.path);
    run_git(sparse, "sparse-checkout remote skill folder")?;

    let skill_path = clone_dir.join(&remote.path);
    if !skill_path.exists() {
        return Err(Error::InvalidInput {
            message: format!(
                "remote path {} did not materialize a skill folder from {}",
                remote.path, remote.repo_url
            ),
        });
    }
    Ok(StagedRemote { root, skill_path })
}

fn run_git(mut command: Command, action: &str) -> Result<()> {
    let output = command.output().map_err(|source| Error::InvalidInput {
        message: format!("failed to run git for {action}: {source}"),
    })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr)
        .lines()
        .take(8)
        .collect::<Vec<_>>()
        .join("\n");
    Err(Error::InvalidInput {
        message: format!("git failed to {action}: {stderr}"),
    })
}

fn unique_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn load_skill_body(map: &SourceMap, source_root: &Path) -> Result<SkillBody> {
    let skill_files = map
        .files
        .iter()
        .filter(|file| {
            Path::new(&file.path)
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        })
        .collect::<Vec<_>>();
    if skill_files.is_empty() {
        return Err(Error::InvalidInput {
            message: format!(
                "skillspec doctor expected a prose skill folder or SKILL.md file under {}",
                source_root.display()
            ),
        });
    }
    if skill_files.len() > 1 {
        let paths = skill_files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(Error::InvalidInput {
            message: format!(
                "skillspec doctor requires exactly one SKILL.md; found {} under {}: {}",
                skill_files.len(),
                source_root.display(),
                paths
            ),
        });
    }
    let file = skill_files[0];
    let path = source_root.join(&file.path);
    let content = fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })?;
    let frontmatter_node = map
        .nodes
        .iter()
        .find(|node| node.file == file.id && node.kind == "frontmatter");
    let (frontmatter, body, body_start_line) = match frontmatter_node {
        Some(node) => {
            let end = node.byte_range.map(|range| range[1]).unwrap_or(0);
            let line = node.line_range.map(|range| range[1] + 1).unwrap_or(1);
            (
                content.get(..end).unwrap_or("").to_owned(),
                content.get(end..).unwrap_or("").to_owned(),
                line,
            )
        }
        None => (String::new(), content.clone(), 1),
    };
    Ok(SkillBody {
        path,
        file_id: file.id.clone(),
        body,
        frontmatter,
        body_start_line,
    })
}

fn surface_report(map: &SourceMap, skill: &SkillBody) -> SurfaceReport {
    let referenced = map
        .references
        .iter()
        .filter_map(|reference| reference.resolved_file.as_deref())
        .collect::<BTreeSet<_>>();
    let frontmatter_bytes = skill.frontmatter.len();
    let frontmatter_lines = skill.frontmatter.lines().count();
    let activation_bytes = skill.body.len();
    let activation_lines = skill.body.lines().count();
    let activation_estimated_tokens = estimate_tokens(&skill.body);
    let deferred_files = map
        .files
        .iter()
        .filter(|file| file.id != skill.file_id && file.load_status == SourceFileLoadStatus::Loaded)
        .count();
    let deferred_bytes = map
        .files
        .iter()
        .filter(|file| file.id != skill.file_id && file.load_status == SourceFileLoadStatus::Loaded)
        .map(|file| file.bytes)
        .sum::<usize>();
    let unmapped_files = map
        .files
        .iter()
        .filter(|file| file.id != skill.file_id)
        .filter(|file| {
            file.kind == SourceFileKind::Code
                || file.kind == SourceFileKind::Manifest
                || file.kind == SourceFileKind::Other
        })
        .filter(|file| !referenced.contains(file.path.as_str()))
        .count();
    SurfaceReport {
        frontmatter_bytes,
        frontmatter_lines,
        activation_bytes,
        activation_lines,
        activation_estimated_tokens,
        deferred_bytes,
        deferred_files,
        unmapped_files,
    }
}

fn counts(map: &SourceMap, skill: &SkillBody) -> DoctorCounts {
    let markdown_files = map
        .files
        .iter()
        .filter(|file| file.kind == SourceFileKind::Markdown)
        .count();
    let code_files = map
        .files
        .iter()
        .filter(|file| file.kind == SourceFileKind::Code)
        .count();
    let manifest_files = map
        .files
        .iter()
        .filter(|file| file.kind == SourceFileKind::Manifest)
        .count();
    let code_nodes = map
        .nodes
        .iter()
        .filter(|node| node.file == skill.file_id && node.kind == "code")
        .collect::<Vec<_>>();
    let code_blocks_in_skill = code_nodes.len();
    let unlabeled_code_blocks_in_skill = code_nodes
        .iter()
        .filter(|node| {
            node.language
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        })
        .count();
    let modal_nodes = map
        .classifications
        .iter()
        .filter(|class| class.kind == SourceClassificationKind::ModalObligation)
        .filter_map(|class| map.nodes.iter().find(|node| node.id == class.target))
        .filter(|node| node.file == skill.file_id)
        .collect::<Vec<_>>();
    let modal_obligations = modal_nodes.len();
    let late_line =
        skill.body_start_line + (skill.body.lines().count() * PRIMACY_LINE_PERCENT / 100);
    let late_modal_obligations = modal_nodes
        .iter()
        .filter(|node| {
            node.line_range
                .map(|range| range[0] >= late_line)
                .unwrap_or(false)
        })
        .count();
    let numbered_steps = skill
        .body
        .lines()
        .filter(|line| looks_like_numbered_step(line))
        .count();
    let dependency_mentions = map
        .classifications
        .iter()
        .filter(|class| class.kind == SourceClassificationKind::DependencyMention)
        .count();
    let missing_local_references = map
        .references
        .iter()
        .filter(|reference| reference.target_kind == SourceReferenceKind::LocalFile)
        .filter(|reference| reference.resolved_file.is_none())
        .count();
    DoctorCounts {
        markdown_files,
        code_files,
        manifest_files,
        code_blocks_in_skill,
        unlabeled_code_blocks_in_skill,
        modal_obligations,
        late_modal_obligations,
        numbered_steps,
        dependency_mentions,
        missing_local_references,
    }
}

fn skill_code_bytes(map: &SourceMap, skill_file_id: &str) -> usize {
    map.nodes
        .iter()
        .filter(|node| node.file == skill_file_id && node.kind == "code")
        .filter_map(|node| {
            node.byte_range
                .map(|range| range[1].saturating_sub(range[0]))
        })
        .sum()
}

fn looks_like_numbered_step(line: &str) -> bool {
    let trimmed = line.trim_start();
    let digits = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();
    digits > 0 && trimmed.chars().nth(digits) == Some('.')
}

fn operational_prose(body: &str) -> bool {
    let lowered = body.to_ascii_lowercase();
    [
        " run ",
        " execute ",
        " use ",
        " call ",
        " fetch ",
        " open ",
        " click ",
        " install ",
        " create ",
        " generate ",
        " browse ",
        " shell ",
        " python ",
        " bash ",
        " api ",
    ]
    .iter()
    .any(|signal| lowered.contains(signal))
}

fn estimate_tokens(text: &str) -> usize {
    let by_bytes = text.len().div_ceil(4);
    let by_words = text.split_whitespace().count();
    by_bytes.max(by_words)
}

fn percentage(numerator: usize, denominator: usize) -> u8 {
    if denominator == 0 {
        return 0;
    }
    let value = ((numerator as f64 / denominator as f64) * 100.0).round();
    u8::try_from(value as usize).unwrap_or(100).min(100)
}

fn issue(
    id: &str,
    severity: &str,
    title: &str,
    evidence: String,
    basis: Vec<&str>,
    remediation: &str,
    score_penalty: u8,
) -> DoctorIssue {
    DoctorIssue {
        id: id.to_owned(),
        severity: severity.to_owned(),
        title: title.to_owned(),
        evidence,
        basis: basis.into_iter().map(str::to_owned).collect(),
        remediation: remediation.to_owned(),
        score_penalty,
        location: None,
    }
}

fn with_location(mut issue: DoctorIssue, location: String) -> DoctorIssue {
    issue.location = Some(location);
    issue
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

fn verdict(score: u8) -> String {
    match score {
        80..=100 => "low reliability debt".to_owned(),
        60..=79 => "medium reliability debt".to_owned(),
        40..=59 => "high reliability debt".to_owned(),
        _ => "critical reliability debt".to_owned(),
    }
}

fn basis() -> Vec<DoctorBasis> {
    vec![
        DoctorBasis {
            id: "reliability_gap_instruction_density".to_owned(),
            source: "docs/00-skills-reliability-gap.md §3.1".to_owned(),
            claim: "Large, dense activation bodies increase dropped-step risk and later instructions suffer primacy bias.".to_owned(),
        },
        DoctorBasis {
            id: "reliability_gap_metadata_context_pressure".to_owned(),
            source: "docs/00-skills-reliability-gap.md §3.2".to_owned(),
            claim: "Skill frontmatter metadata and activated bodies consume context; broad surfaces degrade routing and adherence.".to_owned(),
        },
        DoctorBasis {
            id: "reliability_gap_implicit_environment_contract".to_owned(),
            source: "docs/00-skills-reliability-gap.md §3.3".to_owned(),
            claim: "Scripts and snippets carry dependency contracts whether or not the skill declares them.".to_owned(),
        },
        DoctorBasis {
            id: "reliability_gap_no_execution_guarantees".to_owned(),
            source: "docs/00-skills-reliability-gap.md §3.5".to_owned(),
            claim: "Prose instructions and embedded snippets are guidance unless moved into checkable or enforced surfaces.".to_owned(),
        },
        DoctorBasis {
            id: "reliability_gap_unfilled_requirement".to_owned(),
            source: "docs/00-skills-reliability-gap.md §5".to_owned(),
            claim: "Reliable skills need a portable, machine-checkable account of intended behavior, dependencies, and proof.".to_owned(),
        },
        DoctorBasis {
            id: "contract_trace_activation_adherence_enforcement".to_owned(),
            source: "docs/08-contract-trace-methodology.md §3.1".to_owned(),
            claim: "Activation, adherence, and enforcement are separate gates; static prose mostly influences adherence, not enforcement.".to_owned(),
        },
        DoctorBasis {
            id: "contract_trace_behavioral_contract".to_owned(),
            source: "docs/08-contract-trace-methodology.md §4.1".to_owned(),
            claim: "A behavioral contract makes steering, dependencies, forbids, and tests statically checkable.".to_owned(),
        },
        DoctorBasis {
            id: "contract_trace_static_well_formedness".to_owned(),
            source: "docs/08-contract-trace-methodology.md §4.1".to_owned(),
            claim: "Reference closure, reachability, and typed structure are static pre-filters before execution.".to_owned(),
        },
        DoctorBasis {
            id: "contract_trace_unproven_verdict".to_owned(),
            source: "docs/08-contract-trace-methodology.md §4.3".to_owned(),
            claim: "Missing trace evidence should be reported as unproven, not inferred as success.".to_owned(),
        },
    ]
}

fn next_steps(has_skill_spec: bool, has_deps_toml: bool) -> Vec<String> {
    let mut steps = Vec::new();
    steps.push("Run `skillspec source map <skill> --out <dir>` to inspect exact source handles before conversion.".to_owned());
    if !has_skill_spec {
        steps.push("Run `skillspec import-skill <skill> --out <skill>/skill.spec.yml --source-map <dir>/source-map.json` and review the scaffold.".to_owned());
    }
    if !has_deps_toml {
        steps.push("Create or complete `deps.toml`; preserve dependency authority, local status, install risk, and degraded proof impact.".to_owned());
    }
    steps.push("Promote operational prose into routes, rules, forbids, command templates, scenario tests, and trace/progress proof obligations.".to_owned());
    steps
}

fn trim_trailing_newline(output: &mut String) {
    while output.ends_with('\n') {
        output.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::{remote_from_target, rewrite_remote_error};
    use crate::error::Error;
    use std::path::Path;

    #[test]
    fn parses_github_tree_skill_folder_url() {
        let remote =
            remote_from_target("https://github.com/anthropics/skills/tree/main/skills/pdf")
                .unwrap()
                .unwrap();
        assert_eq!(remote.repo_url, "https://github.com/anthropics/skills.git");
        assert_eq!(remote.branch.as_deref(), Some("main"));
        assert_eq!(remote.path, "skills/pdf");
    }

    #[test]
    fn parses_github_owner_repo_path_shorthand() {
        let remote = remote_from_target("anthropics/skills/skills/pdf")
            .unwrap()
            .unwrap();
        assert_eq!(remote.repo_url, "https://github.com/anthropics/skills.git");
        assert_eq!(remote.branch, None);
        assert_eq!(remote.path, "skills/pdf");
    }

    #[test]
    fn rejects_github_blob_urls_for_remote_doctor() {
        let error = remote_from_target(
            "https://github.com/anthropics/skills/blob/main/skills/pdf/SKILL.md",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("not a blob"));
    }

    #[test]
    fn rejects_remote_skill_md_file_shorthand() {
        let error = remote_from_target("anthropics/skills/skills/pdf/SKILL.md")
            .unwrap_err()
            .to_string();
        assert!(error.contains("single-skill-folder"));
    }

    #[test]
    fn rewrites_remote_error_locations() {
        let error = rewrite_remote_error(
            Error::InvalidInput {
                message: "found multiple SKILL.md under /tmp/skillspec-doctor/repo/skills"
                    .to_owned(),
            },
            Path::new("/tmp/skillspec-doctor/repo/skills"),
            "https://github.com/owner/repo/tree/main/skills",
        )
        .to_string();
        assert!(error.contains("https://github.com/owner/repo/tree/main/skills"));
        assert!(!error.contains("/tmp/skillspec-doctor"));
    }
}
