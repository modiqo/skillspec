//! Staging and install planning for a skill or plugin, so it can be assessed
//! before it lands on disk.
//!
//! A skill ships in a git repository and is copied out of it on install. That
//! makes the repository assessable *before* anything is placed, which is the
//! whole reason `skillspec pull` can gate an install: stage the source, run the
//! boundary assessment against the staged files, and only then place them.
//!
//! There are two ways a harness installs a skill, and this module plans for
//! both:
//!
//! - **Proxy** — a plugin-marketplace repo (one carrying
//!   `.claude-plugin/marketplace.json`) is installed through the harness CLI:
//!   `claude plugin marketplace add <owner/repo>` then `claude plugin install
//!   <plugin>@<marketplace>`. skillspec shells out to that CLI. It requires the
//!   binary to be present.
//! - **Place** — every other harness reads `SKILL.md` from a skills directory,
//!   so "install" is placing the skill folders there. skillspec does the copy
//!   itself. This is the universal path and needs no harness CLI.
//!
//! This module is pure: it stages, parses the manifest, enumerates skills, and
//! plans. Running the harness CLI is the caller's job; placing files is done
//! here because a file copy is deterministic and testable.

use serde::Deserialize;
use skillspec_core::error::{Error, Result};
use skillspec_source::remote::{self, TemporaryRemoteCheckout};
use std::path::{Path, PathBuf};

/// A staged source, resolved to local files that outlive the assessment.
///
/// For a remote source the temporary checkout is held for the lifetime of the
/// `Staged`, so the files are still present when they are placed. Dropping it
/// removes the checkout.
pub struct Staged {
    /// Directory the skills are enumerated from (a repo root, or a subpath).
    root: PathBuf,
    /// Directory the marketplace manifest would live in (the checkout root).
    manifest_root: PathBuf,
    origin: Origin,
    _temp: Option<TemporaryRemoteCheckout>,
}

/// Where a staged source came from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Origin {
    /// A path the user already has.
    Local(PathBuf),
    /// A remote git repository, with its `owner/repo` slug when derivable.
    Remote {
        repo_url: String,
        slug: Option<String>,
    },
}

/// A parsed plugin marketplace manifest.
#[derive(Clone, Debug, Deserialize)]
pub struct Marketplace {
    /// The marketplace name a `plugin install <plugin>@<name>` refers to.
    pub name: String,
    #[serde(default)]
    pub plugins: Vec<MarketplacePlugin>,
}

/// One plugin entry in a marketplace manifest.
#[derive(Clone, Debug, Deserialize)]
pub struct MarketplacePlugin {
    pub name: String,
    #[serde(default)]
    pub source: Option<String>,
}

impl Staged {
    /// The directory skills are enumerated from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    /// The `owner/repo` slug for a `marketplace add`, if the source is remote.
    pub fn marketplace_slug(&self) -> Option<&str> {
        match &self.origin {
            Origin::Remote { slug, .. } => slug.as_deref(),
            Origin::Local(_) => None,
        }
    }

    /// The plugin marketplace manifest, if this repo carries one.
    ///
    /// Both the Claude (`.claude-plugin/marketplace.json`) and the harness-
    /// neutral (`.agents/plugins/marketplace.json`) locations are read.
    pub fn marketplace(&self) -> Option<Marketplace> {
        for rel in [
            ".claude-plugin/marketplace.json",
            ".agents/plugins/marketplace.json",
        ] {
            let path = self.manifest_root.join(rel);
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(manifest) = serde_json::from_str::<Marketplace>(&text) {
                    return Some(manifest);
                }
            }
        }
        None
    }

    /// Is this a plugin-marketplace repo the harness CLI can install?
    pub fn is_plugin_marketplace(&self) -> bool {
        self.marketplace().is_some()
    }

    /// The skill package directories inside the staged tree.
    pub fn skills(&self) -> Result<Vec<PathBuf>> {
        crate::workspace::skill_package_dirs(&self.root)
    }
}

/// Stage a target for install: a local path as-is, or a remote repo cloned to a
/// temporary checkout that lives as long as the returned `Staged`.
///
/// The whole repository is fetched for a remote source (no sparse narrowing), so
/// the marketplace manifest and every skill are present for both assessment and
/// placement.
pub fn stage(target: &str) -> Result<Staged> {
    let local = Path::new(target);
    if local.exists() {
        return Ok(Staged {
            root: local.to_path_buf(),
            manifest_root: local.to_path_buf(),
            origin: Origin::Local(local.to_path_buf()),
            _temp: None,
        });
    }

    let Some(source) = remote::parse_target(target)? else {
        return Err(Error::InvalidInput {
            message: format!(
                "install target {target:?} does not exist locally and is not a supported remote git URL"
            ),
        });
    };
    let temp = remote::clone_remote_temp(&source, "skillspec-pull")?;
    let checkout = temp.checkout_dir().to_path_buf();
    let root = match &source.path {
        Some(path) => checkout.join(path),
        None => checkout.clone(),
    };
    if !root.exists() {
        return Err(Error::InvalidInput {
            message: format!(
                "remote path {} did not materialize from {}",
                source.path.as_deref().unwrap_or("."),
                source.repo_url
            ),
        });
    }
    Ok(Staged {
        root,
        manifest_root: checkout,
        origin: Origin::Remote {
            slug: slug_from_repo_url(&source.repo_url),
            repo_url: source.repo_url,
        },
        _temp: Some(temp),
    })
}

/// How an install should be carried out.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Method {
    /// Install through a harness CLI against a plugin marketplace.
    Proxy {
        /// The harness binary, e.g. `claude` or `codex`.
        cli: String,
        /// The `owner/repo` slug to register.
        slug: String,
        /// The marketplace name plugins install from.
        marketplace: String,
        /// The plugin names to install.
        plugins: Vec<String>,
    },
    /// Copy skill folders into a skills directory.
    Place { dest: PathBuf, skills: Vec<PathBuf> },
}

/// What the caller asked for, independent of what the source turns out to be.
#[derive(Clone, Debug, Default)]
pub struct PlanOptions {
    /// Force a specific harness (`claude`, `codex`, ...).
    pub harness: Option<String>,
    /// An explicit destination directory (forces file placement).
    pub into: Option<PathBuf>,
    /// Place into the project's skills dir rather than the user's home.
    pub project: bool,
    /// Restrict a proxy install to these plugin names.
    pub only: Vec<String>,
}

/// Decide how to install a staged source.
///
/// `cli_present` reports whether a harness binary is on `PATH`; it is a
/// parameter so the decision is testable without a real CLI.
pub fn plan(
    staged: &Staged,
    options: &PlanOptions,
    cli_present: &dyn Fn(&str) -> bool,
) -> Result<Method> {
    // An explicit destination is an unambiguous request to place files.
    if let Some(into) = &options.into {
        return Ok(Method::Place {
            dest: into.clone(),
            skills: staged.skills()?,
        });
    }

    // A named harness: proxy when it can, otherwise place into its skills dir.
    if let Some(harness) = &options.harness {
        if let Some(method) = proxy_for(staged, harness, cli_present, &options.only) {
            return Ok(method);
        }
        let dest = harness_skills_dir(harness, options.project).ok_or_else(|| Error::InvalidInput {
            message: format!(
                "harness {harness:?} has no known skills directory; pass --into <dir> to place skills explicitly"
            ),
        })?;
        return Ok(Method::Place {
            dest,
            skills: staged.skills()?,
        });
    }

    // No harness named: proxy through the first CLI that can install a
    // marketplace repo, else place into the default skills directory.
    for harness in ["claude", "codex"] {
        if let Some(method) = proxy_for(staged, harness, cli_present, &options.only) {
            return Ok(method);
        }
    }
    let dest =
        harness_skills_dir("claude", options.project).ok_or_else(|| Error::InvalidInput {
            message: "could not determine a default skills directory; pass --into <dir>".to_owned(),
        })?;
    Ok(Method::Place {
        dest,
        skills: staged.skills()?,
    })
}

/// A proxy install for `harness`, if the source is a marketplace repo, the CLI
/// is present, and the slug is known.
fn proxy_for(
    staged: &Staged,
    harness: &str,
    cli_present: &dyn Fn(&str) -> bool,
    only: &[String],
) -> Option<Method> {
    if !matches!(harness, "claude" | "codex") {
        return None;
    }
    let manifest = staged.marketplace()?;
    let slug = staged.marketplace_slug()?;
    if !cli_present(harness) {
        return None;
    }
    let plugins: Vec<String> = manifest
        .plugins
        .iter()
        .map(|plugin| plugin.name.clone())
        .filter(|name| only.is_empty() || only.iter().any(|wanted| wanted == name))
        .collect();
    if plugins.is_empty() {
        return None;
    }
    Some(Method::Proxy {
        cli: harness.to_owned(),
        slug: slug.to_owned(),
        marketplace: manifest.name,
        plugins,
    })
}

/// A known harness's skills directory, or `None` for one we cannot place into
/// confidently.
pub fn harness_skills_dir(harness: &str, project: bool) -> Option<PathBuf> {
    let (dir, home_sub) = match harness {
        "claude" => (".claude/skills", ".claude/skills"),
        "codex" => (".codex/skills", ".codex/skills"),
        _ => return None,
    };
    if project {
        Some(PathBuf::from(dir))
    } else {
        home_dir().map(|home| home.join(home_sub))
    }
}

/// Copy each skill folder into `dest`, returning the destinations written.
///
/// A skill folder is placed under its own basename. An existing folder of the
/// same name is replaced, so `place` is how both a fresh pull and an update
/// write their files.
pub fn place(skills: &[PathBuf], dest: &Path) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dest).map_err(|source| Error::InvalidInput {
        message: format!(
            "failed to create skills directory {}: {source}",
            dest.display()
        ),
    })?;
    let mut written = Vec::new();
    for skill in skills {
        let name = skill.file_name().ok_or_else(|| Error::InvalidInput {
            message: format!("skill path {} has no folder name", skill.display()),
        })?;
        let target = dest.join(name);
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(|source| Error::InvalidInput {
                message: format!("failed to replace {}: {source}", target.display()),
            })?;
        }
        copy_dir(skill, &target)?;
        written.push(target);
    }
    Ok(written)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).map_err(|source| Error::InvalidInput {
        message: format!("failed to create {}: {source}", to.display()),
    })?;
    let entries = std::fs::read_dir(from).map_err(|source| Error::InvalidInput {
        message: format!("failed to read {}: {source}", from.display()),
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::InvalidInput {
            message: format!("failed to read an entry of {}: {source}", from.display()),
        })?;
        let path = entry.path();
        let child = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &child)?;
        } else {
            std::fs::copy(&path, &child).map_err(|source| Error::InvalidInput {
                message: format!(
                    "failed to copy {} to {}: {source}",
                    path.display(),
                    child.display()
                ),
            })?;
        }
    }
    Ok(())
}

/// Derive an `owner/repo` slug from a clone URL, for a marketplace add.
fn slug_from_repo_url(repo_url: &str) -> Option<String> {
    let trimmed = repo_url.trim_end_matches('/').trim_end_matches(".git");
    // Take the last two path segments: host/owner/repo -> owner/repo.
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let without_userinfo = without_scheme
        .rsplit_once('@')
        .map(|(_, rest)| rest)
        .unwrap_or(without_scheme);
    // `host:owner/repo` (SSH) and `host/owner/repo` (HTTP) both split on '/'
    // once the ':' is normalized to a separator.
    let normalized = without_userinfo.replace(':', "/");
    let segments: Vec<&str> = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    match segments.as_slice() {
        [.., owner, repo] => Some(format!("{owner}/{repo}")),
        _ => None,
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_cli(_: &str) -> bool {
        false
    }
    fn has_cli(_: &str) -> bool {
        true
    }

    #[test]
    fn a_slug_is_derived_from_common_url_shapes() {
        assert_eq!(
            slug_from_repo_url("https://github.com/modiqo/rote-skills.git").as_deref(),
            Some("modiqo/rote-skills")
        );
        assert_eq!(
            slug_from_repo_url("https://github.com/modiqo/rote-skills").as_deref(),
            Some("modiqo/rote-skills")
        );
        assert_eq!(
            slug_from_repo_url("git@github.com:modiqo/rote-skills.git").as_deref(),
            Some("modiqo/rote-skills")
        );
    }

    #[test]
    fn an_explicit_into_always_places() {
        let staged = local_fixture();
        let options = PlanOptions {
            into: Some(PathBuf::from("/tmp/skills")),
            ..Default::default()
        };
        let method = plan(&staged, &options, &has_cli).expect("plan");
        assert!(matches!(method, Method::Place { dest, .. } if dest == Path::new("/tmp/skills")));
    }

    #[test]
    fn a_marketplace_repo_proxies_when_the_cli_is_present() {
        let staged = marketplace_fixture();
        let method = plan(&staged, &PlanOptions::default(), &has_cli).expect("plan");
        match method {
            Method::Proxy {
                cli,
                slug,
                marketplace,
                plugins,
            } => {
                assert_eq!(cli, "claude");
                assert_eq!(slug, "acme/skills");
                assert_eq!(marketplace, "acme-skills");
                assert_eq!(plugins, vec!["one".to_owned(), "two".to_owned()]);
            }
            other => panic!("expected a proxy, got {other:?}"),
        }
    }

    #[test]
    fn a_marketplace_repo_falls_back_to_placement_without_a_cli() {
        let staged = marketplace_fixture();
        let options = PlanOptions {
            project: true,
            ..Default::default()
        };
        let method = plan(&staged, &options, &no_cli).expect("plan");
        assert!(matches!(method, Method::Place { .. }));
    }

    #[test]
    fn only_narrows_the_proxied_plugins() {
        let staged = marketplace_fixture();
        let options = PlanOptions {
            only: vec!["two".to_owned()],
            ..Default::default()
        };
        let method = plan(&staged, &options, &has_cli).expect("plan");
        assert!(
            matches!(method, Method::Proxy { plugins, .. } if plugins == vec!["two".to_owned()])
        );
    }

    #[test]
    fn place_copies_each_skill_under_its_name() {
        let staged = local_fixture();
        let dest = staged.root.join("out");
        let written = place(&staged.skills().unwrap(), &dest).expect("place");
        assert_eq!(written.len(), 1);
        assert!(dest.join("skill-a").join("SKILL.md").exists());
    }

    // --- fixtures -----------------------------------------------------------

    fn local_fixture() -> Staged {
        let root = temp_tree();
        let skill = root.join("skill-a");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), "---\nname: a\n---\n# A\n").unwrap();
        Staged {
            root: root.clone(),
            manifest_root: root.clone(),
            origin: Origin::Local(root),
            _temp: None,
        }
    }

    fn marketplace_fixture() -> Staged {
        let root = temp_tree();
        let manifest_dir = root.join(".claude-plugin");
        std::fs::create_dir_all(&manifest_dir).unwrap();
        std::fs::write(
            manifest_dir.join("marketplace.json"),
            r#"{"name":"acme-skills","plugins":[{"name":"one"},{"name":"two"}]}"#,
        )
        .unwrap();
        let skill = root.join("skill-a");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), "---\nname: a\n---\n# A\n").unwrap();
        Staged {
            root: root.clone(),
            manifest_root: root.clone(),
            origin: Origin::Remote {
                repo_url: "https://github.com/acme/skills.git".to_owned(),
                slug: Some("acme/skills".to_owned()),
            },
            _temp: None,
        }
    }

    fn temp_tree() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "skillspec-install-test-{}-{}",
            std::process::id(),
            remote::unique_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }
}
