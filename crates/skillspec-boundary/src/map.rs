//! The surface map: the shape of a skill folder before any analysis.
//!
//! A phase-0 orientation artifact. Built from the source map alone - no effect
//! extraction - it answers "what is here and how does it connect" so a reviewer
//! knows the shape before reading per-skill reports, and it surfaces the two
//! pieces of topology the security analysis cares about most:
//!
//! - **Cross-skill references**: a skill that links into another skill is the
//!   propagation surface. One that reads or writes another skill's files is
//!   caught as an effect; one that merely references it is shown here.
//! - **Orphan files**: files nothing in the documentation reaches. They ship in
//!   the package, are analyzed, and are invisible to a reader following the
//!   `SKILL.md` - the natural place to hide a payload or a directive.
//!
//! The connected-component view (which skills form a group, which stand alone)
//! is useful for orientation, and can later drive parallel analysis of
//! independent skills, but the analysis is fast enough that orientation is the
//! point here, not scheduling.

use serde::Serialize;
use skillspec_core::error::Result;
use skillspec_source::source_map::{self, SourceMap};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Schema id for a serialized surface map.
pub const SURFACE_MAP_SCHEMA: &str = "skillspec.boundary.surface_map.v0";

/// One skill in the map.
#[derive(Clone, Debug, Serialize)]
pub struct SkillNode {
    /// Package directory, relative to the map root.
    pub package: String,
    /// Files this skill owns that its documentation reaches.
    pub resources: Vec<String>,
    /// Other skills this skill references.
    pub references_skills: Vec<String>,
    /// Files this skill owns that nothing references.
    pub orphans: Vec<String>,
}

/// A document outside any skill that indexes or coordinates skills, e.g. a root
/// `README.md` that links every skill in the collection.
#[derive(Clone, Debug, Serialize)]
pub struct EntryDoc {
    pub path: String,
    pub references_skills: Vec<String>,
}

/// The shape of a skill folder.
#[derive(Clone, Debug, Serialize)]
pub struct SurfaceMap {
    pub schema: &'static str,
    pub target: String,
    pub skills: Vec<SkillNode>,
    /// Root-level documents that index the skills below them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entry_docs: Vec<EntryDoc>,
    /// Connected groups of skills; a single-skill group is independent.
    pub components: Vec<Vec<String>>,
}

impl SurfaceMap {
    /// Files nothing references, across every skill: the hiding spots.
    pub fn orphan_count(&self) -> usize {
        self.skills.iter().map(|skill| skill.orphans.len()).sum()
    }

    /// Independent skills, i.e. single-skill components: the parallel paths.
    pub fn independent_count(&self) -> usize {
        self.components.iter().filter(|c| c.len() == 1).count()
    }
}

/// Build the surface map of a folder of skills.
pub fn build(root: &Path) -> Result<SurfaceMap> {
    let map = source_map::build(root)?;
    let packages = crate::workspace::skill_package_dirs(root)?;
    let package_rel = packages
        .iter()
        .map(|dir| {
            // Forward-slashed to match the source map's file paths, which are
            // always `/`-separated; otherwise `owning_package` never matches on
            // Windows and every file falls out of its skill. No-op on Unix.
            dir.strip_prefix(root)
                .unwrap_or(dir)
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/")
        })
        .map(|rel| if rel.is_empty() { ".".to_owned() } else { rel })
        .collect::<Vec<_>>();

    let owner = |path: &str| owning_package(path, &package_rel);
    let file_of_node = node_to_file(&map);
    let all_files: BTreeSet<&str> = map.files.iter().map(|file| file.path.as_str()).collect();

    // File -> file reference edges, and which files are referenced at all.
    let mut edges: Vec<(String, String)> = Vec::new();
    let mut referenced: BTreeSet<String> = BTreeSet::new();
    for reference in &map.references {
        let Some(target) = resolve_reference(reference, &all_files) else {
            continue;
        };
        referenced.insert(target.clone());
        if let Some(source_file) = file_of_node.get(reference.source.as_str()) {
            edges.push(((*source_file).to_owned(), target));
        }
    }

    // Per-skill resources, orphans, and cross-skill references.
    let mut skills: BTreeMap<String, SkillNode> = package_rel
        .iter()
        .map(|package| {
            (
                package.clone(),
                SkillNode {
                    package: package.clone(),
                    resources: Vec::new(),
                    references_skills: Vec::new(),
                    orphans: Vec::new(),
                },
            )
        })
        .collect();

    for file in &map.files {
        let Some(package) = owner(&file.path) else {
            continue;
        };
        if is_skill_md(&file.path) {
            continue;
        }
        let node = skills.get_mut(&package).expect("owner is a known package");
        if referenced.contains(&file.path) {
            node.resources.push(file.path.clone());
        } else {
            node.orphans.push(file.path.clone());
        }
    }

    // Cross-skill references, the entry-document index, and the component graph.
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut entry: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (source, target) in &edges {
        let target_owner = owner(target);
        match (owner(source), target_owner) {
            (Some(a), Some(b)) if a != b => {
                let node = skills.get_mut(&a).expect("owner is a known package");
                if !node.references_skills.contains(&b) {
                    node.references_skills.push(b.clone());
                }
                adjacency.entry(a.clone()).or_default().insert(b.clone());
                adjacency.entry(b).or_default().insert(a);
            }
            // A reference from a file that belongs to no skill, pointing into a
            // skill: a root manifest indexing the collection.
            (None, Some(b)) => {
                entry.entry(source.clone()).or_default().insert(b);
            }
            _ => {}
        }
    }
    let entry_docs = entry
        .into_iter()
        .map(|(path, skills)| EntryDoc {
            path,
            references_skills: skills.into_iter().collect(),
        })
        .collect::<Vec<_>>();

    let components = connected_components(&package_rel, &adjacency);
    let mut skills = skills.into_values().collect::<Vec<_>>();
    skills.sort_by(|a, b| a.package.cmp(&b.package));

    Ok(SurfaceMap {
        schema: SURFACE_MAP_SCHEMA,
        target: root.display().to_string(),
        skills,
        entry_docs,
        components,
    })
}

/// Resolve one reference to a package-relative file path, if it points at a
/// file that exists.
///
/// The source map resolves a link relative to its own file's directory. When
/// that fails - a common shape in skill collections is `skills/other/SKILL.md`
/// written relative to the repository root, not the linking skill - fall back
/// to interpreting the raw target from the root. The fallback is only accepted
/// when the resulting path names a file that actually exists, so a real
/// cross-skill link resolves while a dangling one is still ignored: this
/// resolves references, it does not invent them.
fn resolve_reference(
    reference: &skillspec_source::source_map::SourceReferenceRecord,
    all_files: &BTreeSet<&str>,
) -> Option<String> {
    use skillspec_source::source_map::SourceReferenceKind;
    if let Some(resolved) = &reference.resolved_file {
        let normalized = normalize_rel(resolved);
        if all_files.contains(normalized.as_str()) {
            return Some(normalized);
        }
    }
    if reference.target_kind == SourceReferenceKind::LocalFile {
        let root_relative = normalize_rel(reference.target.split(['#', '?']).next()?);
        if !root_relative.is_empty() && all_files.contains(root_relative.as_str()) {
            return Some(root_relative);
        }
    }
    None
}

/// The package that owns a file: the longest package path that prefixes it.
fn owning_package(path: &str, packages: &[String]) -> Option<String> {
    packages
        .iter()
        .filter(|package| {
            *package == "." || path == package.as_str() || path.starts_with(&format!("{package}/"))
        })
        .max_by_key(|package| if *package == "." { 0 } else { package.len() })
        .cloned()
}

/// Map a node id to the path of the file that contains it.
///
/// A node records its file by id, so this resolves through the file table.
fn node_to_file(map: &SourceMap) -> BTreeMap<&str, &str> {
    let file_path: BTreeMap<&str, &str> = map
        .files
        .iter()
        .map(|file| (file.id.as_str(), file.path.as_str()))
        .collect();
    map.nodes
        .iter()
        .filter_map(|node| {
            file_path
                .get(node.file.as_str())
                .map(|path| (node.id.as_str(), *path))
        })
        .collect()
}

/// Collapse `.` and `..` in a repo-relative path lexically.
fn normalize_rel(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if matches!(parts.last(), Some(last) if *last != "..") {
                    parts.pop();
                } else {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn is_skill_md(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("SKILL.md"))
}

/// Group skills into connected components over the cross-reference graph.
fn connected_components(
    packages: &[String],
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<Vec<String>> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut components = Vec::new();
    for start in packages {
        if seen.contains(start.as_str()) {
            continue;
        }
        let mut stack = vec![start.as_str()];
        let mut group = Vec::new();
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            group.push(node.to_owned());
            if let Some(neighbors) = adjacency.get(node) {
                for neighbor in neighbors {
                    if !seen.contains(neighbor.as_str()) {
                        stack.push(neighbor.as_str());
                    }
                }
            }
        }
        group.sort();
        components.push(group);
    }
    components
}

/// Render the surface map as a tree.
///
/// Uses `termtree` for the box-drawing connectors so the tree layout is a
/// dependency's job, not hand-rolled. Each skill's resources and orphans hang
/// under it, orphans marked, with file paths shown relative to the skill.
pub fn render(map: &SurfaceMap) -> String {
    use crate::style::{self, Style};
    use std::fmt::Write;
    use termtree::Tree;

    let color = style::colors_enabled();
    let resources: usize = map.skills.iter().map(|s| s.resources.len()).sum();
    let root_label = format!(
        "{}   ({} skills · {resources} resources · {} orphans)",
        map.target,
        map.skills.len(),
        map.orphan_count()
    );
    let mut root = Tree::new(root_label);

    for doc in &map.entry_docs {
        root.push(Tree::new(format!(
            "{} {} → indexes {} skill(s)",
            style::paint("[entry]", Style::Accent, color),
            doc.path,
            doc.references_skills.len()
        )));
    }

    for skill in &map.skills {
        let mut node = Tree::new(skill.package.clone());
        if !skill.references_skills.is_empty() {
            node.push(Tree::new(format!(
                "→ references: {}",
                skill.references_skills.join(", ")
            )));
        }
        for resource in &skill.resources {
            node.push(Tree::new(relative_to(resource, &skill.package)));
        }
        for orphan in &skill.orphans {
            node.push(Tree::new(format!(
                "{} {}",
                style::paint("(orphan)", Style::Warn, color),
                relative_to(orphan, &skill.package)
            )));
        }
        root.push(node);
    }

    let mut out = String::new();
    let _ = write!(out, "{root}");

    let connected = map.components.iter().filter(|c| c.len() > 1).count();
    let _ = writeln!(
        out,
        "\nComponents: {connected} connected, {} independent — {} analysis path(s).",
        map.independent_count(),
        map.components.len()
    );
    if map.orphan_count() > 0 {
        let _ = writeln!(
            out,
            "Orphan files ship in the package but nothing in the documentation reaches them;\nthey are analyzed and are the natural place to hide a payload or directive."
        );
    }
    out
}

/// A file path shown relative to its skill package, for a compact tree leaf.
fn relative_to(path: &str, package: &str) -> String {
    path.strip_prefix(&format!("{package}/"))
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::build;
    use std::fs;

    fn workspace(name: &str) -> std::path::PathBuf {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/map-tests")
            .join(format!("{name}{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        base
    }

    fn write(root: &std::path::Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    #[test]
    fn independent_skills_are_separate_components() {
        let root = workspace("indep");
        write(
            &root,
            "skills/a/SKILL.md",
            "---\nname: a\ndescription: x.\n---\n# A\n",
        );
        write(
            &root,
            "skills/b/SKILL.md",
            "---\nname: b\ndescription: y.\n---\n# B\n",
        );
        let map = build(&root).unwrap();
        assert_eq!(map.skills.len(), 2);
        assert_eq!(map.components.len(), 2);
        assert_eq!(map.independent_count(), 2);
    }

    #[test]
    fn a_cross_skill_reference_connects_two_skills() {
        let root = workspace("cross");
        write(
            &root,
            "skills/a/SKILL.md",
            "---\nname: a\ndescription: x.\n---\n# A\nSee [b](../b/SKILL.md).\n",
        );
        write(
            &root,
            "skills/b/SKILL.md",
            "---\nname: b\ndescription: y.\n---\n# B\n",
        );
        let map = build(&root).unwrap();
        let a = map
            .skills
            .iter()
            .find(|s| s.package.ends_with("a"))
            .unwrap();
        assert!(a.references_skills.iter().any(|r| r.ends_with("b")));
        assert!(map.components.iter().any(|c| c.len() == 2));
    }

    #[test]
    fn a_repo_root_relative_cross_skill_reference_resolves() {
        // jakubkrehel shape: a skill links to `skills/b/SKILL.md` relative to
        // the repo root, not to its own directory.
        let root = workspace("rootrel");
        write(
            &root,
            "skills/a/SKILL.md",
            "---
name: a
description: x.
---
# A
See [b](skills/b/SKILL.md).
",
        );
        write(
            &root,
            "skills/b/SKILL.md",
            "---
name: b
description: y.
---
# B
",
        );
        let map = build(&root).unwrap();
        let a = map
            .skills
            .iter()
            .find(|s| s.package.ends_with("a"))
            .unwrap();
        assert!(
            a.references_skills.iter().any(|r| r.ends_with("b")),
            "root-relative cross-skill link should resolve"
        );
        assert!(map.components.iter().any(|c| c.len() == 2));
    }

    #[test]
    fn a_root_manifest_that_indexes_skills_is_an_entry_document() {
        // jakubkrehel shape: a root README links every skill below it.
        let root = workspace("manifest");
        write(
            &root,
            "README.md",
            "# Collection
- [a](skills/a/SKILL.md)
- [b](skills/b/SKILL.md)
",
        );
        write(
            &root,
            "skills/a/SKILL.md",
            "---
name: a
description: x.
---
# A
",
        );
        write(
            &root,
            "skills/b/SKILL.md",
            "---
name: b
description: y.
---
# B
",
        );
        let map = build(&root).unwrap();
        let readme = map
            .entry_docs
            .iter()
            .find(|d| d.path.ends_with("README.md"))
            .unwrap();
        assert_eq!(readme.references_skills.len(), 2);
    }

    #[test]
    fn a_dangling_reference_is_not_invented_as_an_edge() {
        let root = workspace("dangling");
        write(
            &root,
            "skills/a/SKILL.md",
            "---
name: a
description: x.
---
# A
See [gone](skills/does-not-exist/SKILL.md).
",
        );
        write(
            &root,
            "skills/b/SKILL.md",
            "---
name: b
description: y.
---
# B
",
        );
        let map = build(&root).unwrap();
        let a = map
            .skills
            .iter()
            .find(|s| s.package.ends_with("a"))
            .unwrap();
        assert!(
            a.references_skills.is_empty(),
            "a dangling link is not an edge"
        );
    }

    #[test]
    fn an_unreferenced_file_is_an_orphan() {
        let root = workspace("orphan");
        write(
            &root,
            "skills/a/SKILL.md",
            "---\nname: a\ndescription: x.\n---\n# A\nSee [style](style.md).\n",
        );
        write(&root, "skills/a/style.md", "# Style\n");
        write(
            &root,
            "skills/a/scripts/hidden.sh",
            "cat ~/.aws/credentials\n",
        );
        let map = build(&root).unwrap();
        let a = &map.skills[0];
        assert!(a.resources.iter().any(|r| r.ends_with("style.md")));
        assert!(a.orphans.iter().any(|o| o.ends_with("hidden.sh")));
        assert_eq!(map.orphan_count(), 1);
    }
}
