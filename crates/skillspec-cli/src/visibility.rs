use crate::error::{Error, Result};
use crate::router::{self, SkillEntry, Visibility};
use serde::{Deserialize, Serialize};
use serde_json::Map as JsonMap;
use serde_yaml::Mapping as YamlMapping;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MANIFEST_SCHEMA: &str = "skillspec/visibility-manifest/v1";
pub(crate) const ROUTER_MANAGED_IMPLICIT_EXCEPTION: &str = "durable-executor";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityProfile {
    RouterManaged,
    Explicit,
}

impl VisibilityProfile {
    fn as_str(self) -> &'static str {
        match self {
            Self::RouterManaged => "router-managed",
            Self::Explicit => "explicit",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessKind {
    Codex,
    Claude,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HarnessFileTarget {
    CodexOpenai,
    ClaudeSettings,
    ClaudeFrontmatter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityChangeStatus {
    Planned,
    Applied,
    Restored,
}

#[derive(Clone, Debug)]
pub struct VisibilityPlanOptions {
    pub roots: Vec<PathBuf>,
    pub profile: VisibilityProfile,
}

#[derive(Clone, Debug)]
pub struct VisibilityApplyOptions {
    pub roots: Vec<PathBuf>,
    pub profile: VisibilityProfile,
    pub manifest: PathBuf,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct SetVisibilityOptions {
    pub roots: Vec<PathBuf>,
    pub skill: String,
    pub visibility: Visibility,
    pub manifest: PathBuf,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct VisibilityRestoreOptions {
    pub manifest: PathBuf,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct VisibilityPlanReport {
    pub profile: VisibilityProfile,
    pub roots: Vec<PathBuf>,
    pub changes: Vec<VisibilityChangeReport>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VisibilityApplyReport {
    pub profile: VisibilityProfile,
    pub manifest: PathBuf,
    pub dry_run: bool,
    pub changes: Vec<VisibilityChangeReport>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VisibilityRestoreReport {
    pub manifest: PathBuf,
    pub dry_run: bool,
    pub changes: Vec<VisibilityChangeReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VisibilityChangeReport {
    pub skill: String,
    pub skill_dir: PathBuf,
    pub harness: HarnessKind,
    pub before_visibility: Visibility,
    pub after_visibility: Visibility,
    pub status: VisibilityChangeStatus,
    pub files: Vec<PathBuf>,
    pub note: Option<String>,
}

#[derive(Clone, Debug)]
struct PreparedChange {
    report: VisibilityChangeReport,
    manifest: VisibilityManifestChange,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct VisibilityManifest {
    schema: String,
    created_at_unix: u64,
    profile: VisibilityProfile,
    changes: Vec<VisibilityManifestChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct VisibilityManifestChange {
    skill: String,
    skill_dir: PathBuf,
    skill_file: PathBuf,
    harness: HarnessKind,
    before_visibility: Visibility,
    after_visibility: Visibility,
    file_changes: Vec<FileSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileSnapshot {
    path: PathBuf,
    before_present: bool,
    before_content: Option<String>,
    after_present: bool,
    after_content: Option<String>,
}

pub fn plan(options: VisibilityPlanOptions) -> Result<VisibilityPlanReport> {
    let (entries, warnings) = scan(&options.roots)?;
    let changes = prepare_profile_changes(&entries, options.profile)?
        .into_iter()
        .map(|prepared| prepared.report)
        .collect();
    Ok(VisibilityPlanReport {
        profile: options.profile,
        roots: options.roots,
        changes,
        warnings,
    })
}

pub fn apply(options: VisibilityApplyOptions) -> Result<VisibilityApplyReport> {
    let (entries, warnings) = scan(&options.roots)?;
    let mut prepared = prepare_profile_changes(&entries, options.profile)?;
    for change in &mut prepared {
        change.report.status = if options.dry_run {
            VisibilityChangeStatus::Planned
        } else {
            VisibilityChangeStatus::Applied
        };
    }
    if !options.dry_run {
        write_prepared_changes(&prepared)?;
        write_manifest(&options.manifest, options.profile, &prepared)?;
    }
    Ok(VisibilityApplyReport {
        profile: options.profile,
        manifest: options.manifest,
        dry_run: options.dry_run,
        changes: prepared.into_iter().map(|change| change.report).collect(),
        warnings,
    })
}

pub fn set_visibility(options: SetVisibilityOptions) -> Result<VisibilityApplyReport> {
    let (entries, warnings) = scan(&options.roots)?;
    let matches = entries
        .iter()
        .filter(|entry| entry.name == options.skill)
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Err(Error::InvalidInput {
            message: format!("no skill named {:?} found in supplied roots", options.skill),
        });
    }

    let mut prepared = Vec::new();
    for entry in matches {
        prepared.extend(prepare_changes(
            entry,
            options.visibility,
            VisibilityProfile::Explicit,
        )?);
    }
    for change in &mut prepared {
        change.report.status = if options.dry_run {
            VisibilityChangeStatus::Planned
        } else {
            VisibilityChangeStatus::Applied
        };
    }
    if !options.dry_run {
        write_prepared_changes(&prepared)?;
        write_manifest(&options.manifest, VisibilityProfile::Explicit, &prepared)?;
    }
    Ok(VisibilityApplyReport {
        profile: VisibilityProfile::Explicit,
        manifest: options.manifest,
        dry_run: options.dry_run,
        changes: prepared.into_iter().map(|change| change.report).collect(),
        warnings,
    })
}

pub fn restore(options: VisibilityRestoreOptions) -> Result<VisibilityRestoreReport> {
    let manifest = read_manifest(&options.manifest)?;
    let mut reports = Vec::new();
    for change in &manifest.changes {
        if !options.dry_run {
            restore_file_snapshots(&change.file_changes)?;
        }
        reports.push(VisibilityChangeReport {
            skill: change.skill.clone(),
            skill_dir: change.skill_dir.clone(),
            harness: change.harness,
            before_visibility: change.after_visibility,
            after_visibility: change.before_visibility,
            status: if options.dry_run {
                VisibilityChangeStatus::Planned
            } else {
                VisibilityChangeStatus::Restored
            },
            files: change
                .file_changes
                .iter()
                .map(|snapshot| snapshot.path.clone())
                .collect(),
            note: None,
        });
    }
    Ok(VisibilityRestoreReport {
        manifest: options.manifest,
        dry_run: options.dry_run,
        changes: reports,
    })
}

pub fn render_plan(report: &VisibilityPlanReport) -> String {
    let mut output = String::new();
    output.push_str("Skill visibility plan\n\n");
    output.push_str(&format!("Profile: {}\n", report.profile.as_str()));
    output.push_str(&format!("Changes: {}\n", report.changes.len()));
    render_change_lines(&mut output, &report.changes);
    render_warnings(&mut output, &report.warnings);
    output
}

pub fn render_apply(report: &VisibilityApplyReport) -> String {
    let mut output = String::new();
    output.push_str("Skill visibility apply\n\n");
    output.push_str(&format!("Profile: {}\n", report.profile.as_str()));
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str(&format!("Changes: {}\n", report.changes.len()));
    render_change_lines(&mut output, &report.changes);
    render_warnings(&mut output, &report.warnings);
    output
}

pub fn render_restore(report: &VisibilityRestoreReport) -> String {
    let mut output = String::new();
    output.push_str("Skill visibility restore\n\n");
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str(&format!("Changes: {}\n", report.changes.len()));
    render_change_lines(&mut output, &report.changes);
    output
}

fn render_change_lines(output: &mut String, changes: &[VisibilityChangeReport]) {
    if changes.is_empty() {
        return;
    }
    output.push('\n');
    for change in changes {
        output.push_str(&format!(
            "- {} [{}]: {} -> {} ({:?})\n",
            change.skill,
            harness_label(change.harness),
            change.before_visibility.as_str(),
            change.after_visibility.as_str(),
            change.status
        ));
        if let Some(note) = &change.note {
            output.push_str(&format!("  note: {note}\n"));
        }
    }
}

fn render_warnings(output: &mut String, warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }
    output.push_str("\nWarnings:\n");
    for warning in warnings {
        output.push_str(&format!("- {warning}\n"));
    }
}

fn scan(roots: &[PathBuf]) -> Result<(Vec<SkillEntry>, Vec<String>)> {
    let mut warnings = Vec::new();
    let entries = router::scan_roots(roots, &mut warnings)?;
    Ok((entries, warnings))
}

fn prepare_profile_changes(
    entries: &[SkillEntry],
    profile: VisibilityProfile,
) -> Result<Vec<PreparedChange>> {
    let mut changes = Vec::new();
    for entry in entries {
        let target = match profile {
            VisibilityProfile::RouterManaged => {
                if entry.name == ROUTER_MANAGED_IMPLICIT_EXCEPTION {
                    Visibility::Implicit
                } else if entry.visibility == Visibility::Off {
                    Visibility::Off
                } else {
                    Visibility::ManualOnly
                }
            }
            VisibilityProfile::Explicit => entry.visibility,
        };
        changes.extend(prepare_changes(entry, target, profile)?);
    }
    Ok(changes)
}

fn prepare_changes(
    entry: &SkillEntry,
    target: Visibility,
    profile: VisibilityProfile,
) -> Result<Vec<PreparedChange>> {
    let mut changes = Vec::new();
    for target_file in harness_file_targets_for(entry) {
        if let Some(change) = prepare_change(entry, target, profile, target_file)? {
            changes.push(change);
        }
    }
    Ok(changes)
}

fn prepare_change(
    entry: &SkillEntry,
    target: Visibility,
    _profile: VisibilityProfile,
    target_file: HarnessFileTarget,
) -> Result<Option<PreparedChange>> {
    let (file_snapshot, note) = match target_file {
        HarnessFileTarget::CodexOpenai => prepare_codex_file(entry, target)?,
        HarnessFileTarget::ClaudeSettings => prepare_claude_settings_file(entry, target)?,
        HarnessFileTarget::ClaudeFrontmatter => prepare_claude_frontmatter_file(entry, target)?,
    };
    if !visibility_target_requires_manifest(target)
        && file_snapshot.before_present == file_snapshot.after_present
        && file_snapshot.before_content == file_snapshot.after_content
    {
        return Ok(None);
    }
    let harness = harness_kind(target_file);
    let files = vec![file_snapshot.path.clone()];
    Ok(Some(PreparedChange {
        report: VisibilityChangeReport {
            skill: entry.name.clone(),
            skill_dir: entry.skill_dir.clone(),
            harness,
            before_visibility: entry.visibility,
            after_visibility: target,
            status: VisibilityChangeStatus::Planned,
            files,
            note,
        },
        manifest: VisibilityManifestChange {
            skill: entry.name.clone(),
            skill_dir: entry.skill_dir.clone(),
            skill_file: entry.path.clone(),
            harness,
            before_visibility: entry.visibility,
            after_visibility: target,
            file_changes: vec![file_snapshot],
        },
    }))
}

fn harness_file_targets_for(entry: &SkillEntry) -> Vec<HarnessFileTarget> {
    if router::claude_settings_path(&entry.skill_dir).is_some() {
        return vec![HarnessFileTarget::ClaudeSettings];
    }

    let mut targets = vec![HarnessFileTarget::CodexOpenai];
    if has_ancestor_named(&entry.skill_dir, ".agents") {
        targets.push(HarnessFileTarget::ClaudeFrontmatter);
    }
    targets
}

fn harness_kind(target: HarnessFileTarget) -> HarnessKind {
    match target {
        HarnessFileTarget::CodexOpenai => HarnessKind::Codex,
        HarnessFileTarget::ClaudeSettings | HarnessFileTarget::ClaudeFrontmatter => {
            HarnessKind::Claude
        }
    }
}

fn visibility_target_requires_manifest(target: Visibility) -> bool {
    matches!(target, Visibility::NameOnly | Visibility::Off)
}

fn has_ancestor_named(path: &Path, name: &str) -> bool {
    path.ancestors()
        .any(|ancestor| ancestor.file_name().and_then(|part| part.to_str()) == Some(name))
}

fn prepare_codex_file(
    entry: &SkillEntry,
    target: Visibility,
) -> Result<(FileSnapshot, Option<String>)> {
    let path = entry.skill_dir.join("agents/openai.yaml");
    let before_content = read_optional(&path)?;
    if before_content.is_none() && target == Visibility::Implicit {
        return Ok((
            FileSnapshot {
                path,
                before_present: false,
                before_content: None,
                after_present: false,
                after_content: None,
            },
            None,
        ));
    }
    let allow_implicit = target == Visibility::Implicit;
    let after_content = render_openai_yaml(&path, before_content.as_deref(), allow_implicit)?;
    let note = match target {
        Visibility::NameOnly => Some(
            "Codex has no native name-only state; allow_implicit_invocation=false is used and the manifest preserves name-only for the router."
                .to_owned(),
        ),
        Visibility::Off => Some(
            "Codex has no native off state; allow_implicit_invocation=false is used and the manifest excludes the skill from router results."
                .to_owned(),
        ),
        _ => None,
    };
    Ok((
        FileSnapshot {
            path,
            before_present: before_content.is_some(),
            before_content,
            after_present: true,
            after_content: Some(after_content),
        },
        note,
    ))
}

fn prepare_claude_settings_file(
    entry: &SkillEntry,
    target: Visibility,
) -> Result<(FileSnapshot, Option<String>)> {
    let settings_path =
        router::claude_settings_path(&entry.skill_dir).ok_or_else(|| Error::InvalidInput {
            message: format!(
                "could not find .claude settings path for {}",
                entry.skill_dir.display()
            ),
        })?;
    let before_content = read_optional(&settings_path)?;
    let after_content = render_claude_settings(
        &settings_path,
        before_content.as_deref(),
        &entry.name,
        target,
    )?;
    Ok((
        FileSnapshot {
            path: settings_path,
            before_present: before_content.is_some(),
            before_content,
            after_present: true,
            after_content: Some(after_content),
        },
        None,
    ))
}

fn prepare_claude_frontmatter_file(
    entry: &SkillEntry,
    target: Visibility,
) -> Result<(FileSnapshot, Option<String>)> {
    let path = entry.path.clone();
    let before_content = Some(fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })?);
    if target == Visibility::Implicit
        && !claude_frontmatter_disables_invocation(&path, before_content.as_deref())?
    {
        return Ok((
            FileSnapshot {
                path,
                before_present: true,
                before_content: before_content.clone(),
                after_present: true,
                after_content: before_content,
            },
            None,
        ));
    }
    let after_content =
        render_claude_frontmatter(&path, before_content.as_deref().unwrap_or_default(), target)?;
    let note = match target {
        Visibility::NameOnly => Some(
            "Claude SKILL.md frontmatter has no native name-only state; disable-model-invocation=true is used and the manifest preserves name-only for the router."
                .to_owned(),
        ),
        Visibility::Off => Some(
            "Claude SKILL.md frontmatter has no native off state; disable-model-invocation=true is used and the manifest excludes the skill from router results."
                .to_owned(),
        ),
        _ => None,
    };
    Ok((
        FileSnapshot {
            path,
            before_present: true,
            before_content,
            after_present: true,
            after_content: Some(after_content),
        },
        note,
    ))
}

fn render_openai_yaml(path: &Path, before: Option<&str>, allow_implicit: bool) -> Result<String> {
    let value = match before {
        Some(text) if !text.trim().is_empty() => serde_yaml::from_str::<serde_yaml::Value>(text)
            .map_err(|source| Error::ParseYaml {
                path: path.to_path_buf(),
                source,
            })?,
        _ => serde_yaml::Value::Mapping(YamlMapping::new()),
    };
    let mut root = match value {
        serde_yaml::Value::Mapping(mapping) => mapping,
        _ => YamlMapping::new(),
    };
    let policy_key = serde_yaml::Value::String("policy".to_owned());
    let mut policy = match root.remove(&policy_key) {
        Some(serde_yaml::Value::Mapping(mapping)) => mapping,
        _ => YamlMapping::new(),
    };
    policy.insert(
        serde_yaml::Value::String("allow_implicit_invocation".to_owned()),
        serde_yaml::Value::Bool(allow_implicit),
    );
    root.insert(policy_key, serde_yaml::Value::Mapping(policy));
    serde_yaml::to_string(&serde_yaml::Value::Mapping(root)).map_err(|source| Error::RenderYaml {
        path: path.to_path_buf(),
        source,
    })
}

fn render_claude_settings(
    path: &Path,
    before: Option<&str>,
    skill: &str,
    target: Visibility,
) -> Result<String> {
    let mut value = match before {
        Some(text) if !text.trim().is_empty() => serde_json::from_str::<serde_json::Value>(text)
            .map_err(|source| Error::ParseJson {
                path: path.to_path_buf(),
                source,
            })?,
        _ => serde_json::Value::Object(JsonMap::new()),
    };
    if !value.is_object() {
        value = serde_json::Value::Object(JsonMap::new());
    }
    let root = value
        .as_object_mut()
        .expect("value was normalized to object");
    let overrides = root
        .entry("skillOverrides".to_owned())
        .or_insert_with(|| serde_json::Value::Object(JsonMap::new()));
    if !overrides.is_object() {
        *overrides = serde_json::Value::Object(JsonMap::new());
    }
    let overrides = overrides
        .as_object_mut()
        .expect("skillOverrides was normalized to object");
    overrides.insert(
        skill.to_owned(),
        serde_json::Value::String(claude_override_state(target).to_owned()),
    );
    serde_json::to_string_pretty(&value)
        .map(|json| format!("{json}\n"))
        .map_err(Error::RenderJson)
}

fn claude_override_state(target: Visibility) -> &'static str {
    match target {
        Visibility::Implicit => "on",
        Visibility::ManualOnly => "user-invocable-only",
        Visibility::NameOnly => "name-only",
        Visibility::Off => "off",
    }
}

fn claude_frontmatter_disables_invocation(path: &Path, before: Option<&str>) -> Result<bool> {
    let Some(before) = before else {
        return Ok(false);
    };
    let Some(rest) = before.strip_prefix("---") else {
        return Err(Error::InvalidInput {
            message: format!("missing YAML frontmatter in {}", path.display()),
        });
    };
    let Some((frontmatter, _body)) = rest.split_once("\n---") else {
        return Err(Error::InvalidInput {
            message: format!("unterminated YAML frontmatter in {}", path.display()),
        });
    };
    let value = serde_yaml::from_str::<serde_yaml::Value>(frontmatter).map_err(|source| {
        Error::ParseYaml {
            path: path.to_path_buf(),
            source,
        }
    })?;
    Ok(value
        .get("disable-model-invocation")
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false))
}

fn render_claude_frontmatter(path: &Path, before: &str, target: Visibility) -> Result<String> {
    let Some(rest) = before.strip_prefix("---") else {
        return Err(Error::InvalidInput {
            message: format!("missing YAML frontmatter in {}", path.display()),
        });
    };
    let Some((frontmatter, body)) = rest.split_once("\n---") else {
        return Err(Error::InvalidInput {
            message: format!("unterminated YAML frontmatter in {}", path.display()),
        });
    };
    let value = serde_yaml::from_str::<serde_yaml::Value>(frontmatter).map_err(|source| {
        Error::ParseYaml {
            path: path.to_path_buf(),
            source,
        }
    })?;
    let mut root = match value {
        serde_yaml::Value::Mapping(mapping) => mapping,
        _ => YamlMapping::new(),
    };
    root.insert(
        serde_yaml::Value::String("disable-model-invocation".to_owned()),
        serde_yaml::Value::Bool(target != Visibility::Implicit),
    );
    let mut rendered =
        serde_yaml::to_string(&serde_yaml::Value::Mapping(root)).map_err(|source| {
            Error::RenderYaml {
                path: path.to_path_buf(),
                source,
            }
        })?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(format!("---\n{rendered}---{body}"))
}

fn write_prepared_changes(changes: &[PreparedChange]) -> Result<()> {
    for change in changes {
        for snapshot in &change.manifest.file_changes {
            if snapshot.after_present {
                let content = snapshot.after_content.as_deref().unwrap_or_default();
                write_file(&snapshot.path, content)?;
            } else if snapshot.path.exists() {
                fs::remove_file(&snapshot.path).map_err(|source| Error::Write {
                    path: snapshot.path.clone(),
                    source,
                })?;
            }
        }
    }
    Ok(())
}

fn restore_file_snapshots(snapshots: &[FileSnapshot]) -> Result<()> {
    for snapshot in snapshots {
        if snapshot.before_present {
            let content = snapshot.before_content.as_deref().unwrap_or_default();
            write_file(&snapshot.path, content)?;
        } else if snapshot.path.exists() {
            fs::remove_file(&snapshot.path).map_err(|source| Error::Write {
                path: snapshot.path.clone(),
                source,
            })?;
        }
    }
    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<()> {
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

fn write_manifest(
    manifest_path: &Path,
    profile: VisibilityProfile,
    changes: &[PreparedChange],
) -> Result<()> {
    let mut manifest_changes = changes
        .iter()
        .map(|change| change.manifest.clone())
        .collect::<Vec<_>>();
    if manifest_path.is_file() {
        let existing = read_manifest(manifest_path)?;
        for existing_change in existing.changes {
            let replaced = manifest_changes
                .iter()
                .any(|change| same_manifest_target(change, &existing_change));
            if !replaced {
                manifest_changes.push(existing_change);
            }
        }
    }
    let manifest = VisibilityManifest {
        schema: MANIFEST_SCHEMA.to_owned(),
        created_at_unix: now_unix(),
        profile,
        changes: manifest_changes,
    };
    let json = serde_json::to_string_pretty(&manifest).map_err(Error::RenderJson)?;
    write_file(manifest_path, &format!("{json}\n"))
}

fn same_manifest_target(left: &VisibilityManifestChange, right: &VisibilityManifestChange) -> bool {
    left.harness == right.harness && router::same_path(&left.skill_dir, &right.skill_dir)
}

fn read_manifest(path: &Path) -> Result<VisibilityManifest> {
    let text = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let manifest: VisibilityManifest =
        serde_json::from_str(&text).map_err(|source| Error::ParseJson {
            path: path.to_path_buf(),
            source,
        })?;
    if manifest.schema != MANIFEST_SCHEMA {
        return Err(Error::InvalidInput {
            message: format!(
                "unsupported visibility manifest schema {:?}; expected {MANIFEST_SCHEMA}",
                manifest.schema
            ),
        });
    }
    Ok(manifest)
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn harness_label(harness: HarnessKind) -> &'static str {
    match harness {
        HarnessKind::Codex => "codex",
        HarnessKind::Claude => "claude",
    }
}
