//! Reading a source package into effect observations.
//!
//! Orchestrates the per-language extractors over the source map, and decides
//! each file's [`Reach`] - the property that separates an effect a reviewer
//! would encounter by reading the documentation from one they would not.

pub mod markdown;
pub mod pyargs;
pub mod python;
pub mod shell;
pub mod tsjs;

use crate::bounds::{Budget, SkipReason};
use crate::effect::{EffectObservation, EffectOrigin, Reach};
use skillspec_core::error::{Error, Result};
use skillspec_source::source_map::{SourceFileKind, SourceFileLoadStatus, SourceMap};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Extensions handed to the shell extractor.
const SHELL_EXTENSIONS: &[&str] = &["sh", "bash", "zsh", "ksh", "command"];

/// Extensions handed to the Python extractor.
const PYTHON_EXTENSIONS: &[&str] = &["py"];

/// Extensions handed to the TS/JS host-effect extractor.
///
/// `.tsx`/`.jsx` are excluded: those are browser component files, where a
/// `fetch` runs in the page sandbox rather than on the host.
const TSJS_EXTENSIONS: &[&str] = &["ts", "js", "mjs", "cjs", "mts", "cts"];

/// Everything read out of one package, before deduplication.
pub struct Extraction {
    pub observations: Vec<EffectObservation>,
    pub skill_path: String,
    pub extractors: Vec<&'static str>,
    /// Concealment findings from every scanned text file.
    pub concealment: Vec<crate::concealment::Concealment>,
    /// The skill body text, for directive detection.
    pub skill_body: String,
    /// The 1-based line the skill body starts on, past the frontmatter.
    pub skill_body_line: usize,
    /// The skill's activation description, for activation overbreadth.
    pub activation_description: Option<String>,
}

/// Extract every effect observable in a mapped source package.
pub fn run(map: &SourceMap, source_root: &Path, budget: &mut Budget) -> Result<Extraction> {
    let skill_file_id = skill_file(map)?;
    let skill_path = map
        .files
        .iter()
        .find(|file| file.id == skill_file_id)
        .map(|file| file.path.clone())
        .unwrap_or_default();
    let referenced = referenced_paths(map);

    let mut observations = Vec::new();
    let mut extractors = BTreeSet::new();
    let mut concealment = Vec::new();
    let mut skill_body = String::new();
    let mut skill_body_line = 1;
    let mut activation_description = None;

    for file in &map.files {
        if file.load_status != SourceFileLoadStatus::Loaded {
            if file.load_status == SourceFileLoadStatus::BinaryPreserved {
                budget.skip(&file.path, SkipReason::Binary);
            }
            continue;
        }
        if !budget.admit(&file.path, file.bytes) {
            continue;
        }

        let absolute = source_root.join(&file.path);
        let Ok(content) = read_text(&absolute) else {
            budget.skip(&file.path, SkipReason::Binary);
            continue;
        };
        let reach = reach_for(&file.path, &skill_path, &referenced);

        // Concealment can hide in any text file, not only the skill body.
        concealment.extend(crate::concealment::scan(&content, &file.path, 1));

        if file.kind == SourceFileKind::Markdown && file.id == skill_file_id {
            let (body, body_line) = body_after_frontmatter(&content);
            skill_body = body.to_owned();
            skill_body_line = body_line;
            activation_description = frontmatter_description(&content);
        }

        match file.kind {
            SourceFileKind::Markdown => {
                extractors.insert("markdown");
                observations.extend(markdown::extract(
                    map, &file.id, &file.path, &content, reach,
                ));
            }
            SourceFileKind::Code if is_shell_file(&file.path) => {
                extractors.insert("shell");
                observations.extend(shell::extract(
                    &content,
                    shell::ShellContext {
                        path: &file.path,
                        origin: EffectOrigin::ScriptFile,
                        reach,
                        first_line: 1,
                    },
                ));
            }
            SourceFileKind::Code if is_python_file(&file.path) => {
                extractors.insert("python");
                observations.extend(python::extract(
                    &content,
                    python::PythonContext {
                        path: &file.path,
                        origin: EffectOrigin::ScriptFile,
                        reach,
                        first_line: 1,
                    },
                ));
            }
            SourceFileKind::Code if is_tsjs_file(&file.path) => {
                extractors.insert("tsjs");
                observations.extend(tsjs::extract(
                    &content,
                    tsjs::TsJsContext {
                        path: &file.path,
                        origin: EffectOrigin::ScriptFile,
                        reach,
                        first_line: 1,
                    },
                ));
            }
            _ => {}
        }
    }

    Ok(Extraction {
        observations,
        skill_path,
        extractors: extractors.into_iter().collect(),
        concealment,
        skill_body,
        skill_body_line,
        activation_description,
    })
}

/// The skill body past its YAML frontmatter, and the line it starts on.
fn body_after_frontmatter(content: &str) -> (&str, usize) {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return (content, 1);
    }
    let mut consumed = 1usize;
    for line in lines {
        consumed += 1;
        if line.trim() == "---" {
            // Byte offset just past this line.
            let mut offset = 0;
            for (index, raw) in content.lines().enumerate() {
                offset += raw.len() + 1;
                if index + 1 == consumed {
                    break;
                }
            }
            return (content.get(offset..).unwrap_or(""), consumed + 1);
        }
    }
    (content, 1)
}

/// The `description:` field from a skill's YAML frontmatter.
fn frontmatter_description(content: &str) -> Option<String> {
    let mut in_frontmatter = false;
    for line in content.lines() {
        if line.trim() == "---" {
            if in_frontmatter {
                break;
            }
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if let Some(value) = line.trim().strip_prefix("description:") {
                return Some(
                    value
                        .trim()
                        .trim_matches(|ch| ch == '"' || ch == '\'')
                        .to_owned(),
                );
            }
        }
    }
    None
}

/// The single `SKILL.md` this package is built around.
fn skill_file(map: &SourceMap) -> Result<String> {
    let mut candidates = map
        .files
        .iter()
        .filter(|file| {
            Path::new(&file.path)
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(Error::InvalidInput {
            message: "skillspec boundary expects a skill package containing SKILL.md".to_owned(),
        });
    }
    // Shallowest wins, so an entry skill is preferred over a nested subskill.
    candidates.sort_by_key(|file| (file.path.matches('/').count(), file.path.clone()));
    Ok(candidates[0].id.clone())
}

/// Package-relative paths reachable by following the Markdown.
fn referenced_paths(map: &SourceMap) -> BTreeSet<&str> {
    map.references
        .iter()
        .filter_map(|reference| reference.resolved_file.as_deref())
        .collect()
}

/// Where a file sits relative to what a reader would encounter.
///
/// `Unmapped` is the notable one: the file ships in the package and nothing in
/// the documentation reaches it, so following the skill's own text would never
/// show it to a reviewer. That is a fact about reviewability, not about intent,
/// and the report presents it as such.
fn reach_for(path: &str, skill_path: &str, referenced: &BTreeSet<&str>) -> Reach {
    if path == skill_path {
        Reach::Activation
    } else if referenced.contains(path) {
        Reach::Deferred
    } else {
        Reach::Unmapped
    }
}

fn is_shell_file(path: &str) -> bool {
    has_extension(path, SHELL_EXTENSIONS)
}

fn is_python_file(path: &str) -> bool {
    has_extension(path, PYTHON_EXTENSIONS)
}

fn is_tsjs_file(path: &str) -> bool {
    has_extension(path, TSJS_EXTENSIONS)
}

fn has_extension(path: &str, extensions: &[&str]) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| extensions.contains(&extension.as_str()))
}

fn read_text(path: &PathBuf) -> Result<String> {
    fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::reach_for;
    use crate::effect::Reach;
    use std::collections::BTreeSet;

    #[test]
    fn the_skill_file_itself_is_activation_reach() {
        let referenced = BTreeSet::new();
        assert_eq!(
            reach_for("SKILL.md", "SKILL.md", &referenced),
            Reach::Activation
        );
    }

    #[test]
    fn a_referenced_file_is_deferred() {
        let referenced = BTreeSet::from(["scripts/build.sh"]);
        assert_eq!(
            reach_for("scripts/build.sh", "SKILL.md", &referenced),
            Reach::Deferred
        );
    }

    #[test]
    fn a_file_nothing_references_is_unmapped() {
        // Present in the package, absent from anything a reviewer reaches by
        // following the documentation.
        let referenced = BTreeSet::from(["scripts/build.sh"]);
        assert_eq!(
            reach_for("scripts/collect.sh", "SKILL.md", &referenced),
            Reach::Unmapped
        );
    }

    #[test]
    fn shell_files_are_recognized_by_extension() {
        assert!(super::is_shell_file("scripts/run.sh"));
        assert!(super::is_shell_file("scripts/RUN.BASH"));
        assert!(!super::is_shell_file("scripts/run.py"));
        assert!(!super::is_shell_file("README"));
    }

    #[test]
    fn python_files_are_recognized_by_extension() {
        assert!(super::is_python_file("scripts/tool.py"));
        assert!(super::is_python_file("build.PY"));
        assert!(!super::is_python_file("scripts/run.sh"));
    }

    #[test]
    fn server_tsjs_files_are_recognized_but_browser_components_are_not() {
        assert!(super::is_tsjs_file("scripts/tool.ts"));
        assert!(super::is_tsjs_file("build.mjs"));
        assert!(super::is_tsjs_file("index.js"));
        // Browser component files are excluded: their fetch is sandboxed.
        assert!(!super::is_tsjs_file("src/App.tsx"));
        assert!(!super::is_tsjs_file("src/Button.jsx"));
    }
}
