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

/// The shape of a skill folder.
#[derive(Clone, Debug, Serialize)]
pub struct SurfaceMap {
    pub schema: &'static str,
    pub target: String,
    pub skills: Vec<SkillNode>,
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
            dir.strip_prefix(root)
                .unwrap_or(dir)
                .to_string_lossy()
                .to_string()
        })
        .map(|rel| if rel.is_empty() { ".".to_owned() } else { rel })
        .collect::<Vec<_>>();

    let owner = |path: &str| owning_package(path, &package_rel);
    let file_of_node = node_to_file(&map);

    // File -> file reference edges, and which files are referenced at all.
    let mut edges: Vec<(String, String)> = Vec::new();
    let mut referenced: BTreeSet<String> = BTreeSet::new();
    for reference in &map.references {
        let Some(target) = &reference.resolved_file else {
            continue;
        };
        // The source map joins reference paths without collapsing `..`, so a
        // cross-skill link resolves to `skills/a/../b/SKILL.md`. Normalize it
        // to match the clean file paths.
        let target = normalize_rel(target);
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

    // Cross-skill references, and the component graph.
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (source, target) in &edges {
        if let (Some(a), Some(b)) = (owner(source), owner(target)) {
            if a != b {
                let node = skills.get_mut(&a).expect("owner is a known package");
                if !node.references_skills.contains(&b) {
                    node.references_skills.push(b.clone());
                }
                adjacency.entry(a.clone()).or_default().insert(b.clone());
                adjacency.entry(b).or_default().insert(a);
            }
        }
    }

    let components = connected_components(&package_rel, &adjacency);
    let mut skills = skills.into_values().collect::<Vec<_>>();
    skills.sort_by(|a, b| a.package.cmp(&b.package));

    Ok(SurfaceMap {
        schema: SURFACE_MAP_SCHEMA,
        target: root.display().to_string(),
        skills,
        components,
    })
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

/// Render the surface map for a human.
pub fn render(map: &SurfaceMap) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "SkillSpec Boundary — Surface Map");
    let _ = writeln!(out, "================================");
    let resources: usize = map.skills.iter().map(|s| s.resources.len()).sum();
    let _ = writeln!(
        out,
        "Target: {}        Skills: {}   Resources: {}   Orphan files: {}",
        map.target,
        map.skills.len(),
        resources,
        map.orphan_count()
    );
    let _ = writeln!(out);

    for skill in &map.skills {
        let _ = writeln!(out, "{}", skill.package);
        if !skill.references_skills.is_empty() {
            let _ = writeln!(
                out,
                "  → references: {}",
                skill.references_skills.join(", ")
            );
        }
        if !skill.resources.is_empty() {
            let _ = writeln!(out, "  resources: {}", short_list(&skill.resources));
        }
        if !skill.orphans.is_empty() {
            let _ = writeln!(out, "  orphans:   {}", short_list(&skill.orphans));
        }
    }
    let _ = writeln!(out);

    let connected = map.components.iter().filter(|c| c.len() > 1).count();
    let independent = map.independent_count();
    let _ = writeln!(
        out,
        "Components: {connected} connected, {independent} independent — {} analysis path(s).",
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

fn short_list(items: &[String]) -> String {
    let shown = items.iter().take(4).cloned().collect::<Vec<_>>();
    let rest = items.len().saturating_sub(shown.len());
    if rest > 0 {
        format!("{} (+{rest} more)", shown.join(", "))
    } else {
        shown.join(", ")
    }
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
