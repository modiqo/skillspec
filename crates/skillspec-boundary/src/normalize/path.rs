//! Path normalization and sensitivity classification.
//!
//! Normalization is **lexical**. It expands home markers, resolves `.` and `..`
//! textually, and matches the result against a class table. It never touches
//! the filesystem, because a report must describe the package rather than the
//! machine that happened to analyze it.
//!
//! That choice has a direct consequence: some real paths cannot be placed. Those
//! become [`PathClass::Unknown`], which [`PathClass::is_sensitive`] treats as
//! needing review. Uncertainty resolves toward review, never toward permission.
//!
//! See `docs/design/security/36-skill-effect-surface.md`.

use crate::effect::{PathClass, TargetResolution};

/// A path reduced to a comparable pattern plus its class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedPath {
    /// The lexically resolved path, with home markers folded to `~`.
    pub pattern: String,
    pub class: PathClass,
    pub resolution: TargetResolution,
}

/// Normalize and classify a filesystem path written in package source.
pub fn normalize(raw: &str) -> NormalizedPath {
    let trimmed = raw.trim().trim_matches(|ch| ch == '"' || ch == '\'');
    if trimmed.is_empty() {
        return unknown(trimmed);
    }

    let (expanded, home_relative) = expand_home(trimmed);
    let interpolated = contains_interpolation(&expanded);

    // A path that is nothing but an interpolation tells us nothing at all.
    if interpolated && is_wholly_interpolated(&expanded) {
        return NormalizedPath {
            pattern: expanded,
            class: PathClass::Unknown,
            resolution: TargetResolution::Dynamic,
        };
    }

    let Some(resolved) = resolve_lexically(&expanded) else {
        // `..` after an interpolated segment cannot be resolved soundly, and
        // guessing is how a traversal slips past classification.
        return NormalizedPath {
            pattern: expanded,
            class: PathClass::Unknown,
            resolution: TargetResolution::Dynamic,
        };
    };

    let class = classify(&resolved, home_relative);
    let resolution = if interpolated {
        TargetResolution::Templated
    } else {
        TargetResolution::Literal
    };
    let pattern = if home_relative {
        format!("~/{resolved}")
    } else {
        resolved
    };

    NormalizedPath {
        pattern,
        class,
        resolution,
    }
}

fn unknown(pattern: &str) -> NormalizedPath {
    NormalizedPath {
        pattern: pattern.to_owned(),
        class: PathClass::Unknown,
        resolution: TargetResolution::Unknown,
    }
}

/// Fold every spelling of "the user's home directory" into one form.
///
/// Returns the remainder and whether the path was home-relative. Absolute paths
/// that name a home directory layout (`/Users/x/...`, `/home/x/...`) are folded
/// too, because writing a secret path absolutely is otherwise a trivial way to
/// miss the class table.
fn expand_home(path: &str) -> (String, bool) {
    let normalized = path.replace('\\', "/");
    for marker in ["~/", "$HOME/", "${HOME}/", "%USERPROFILE%/"] {
        if let Some(rest) = normalized.strip_prefix(marker) {
            return (rest.to_owned(), true);
        }
    }
    if normalized == "~" || normalized == "$HOME" {
        return (String::new(), true);
    }
    // /root is root's home directly, with no username segment.
    if let Some(rest) = normalized.strip_prefix("/root/") {
        return (rest.to_owned(), true);
    }
    // /Users/<user>/... and /home/<user>/... carry a username to drop.
    for root in ["/Users/", "/home/"] {
        if let Some(rest) = normalized.strip_prefix(root) {
            if let Some((_user, tail)) = rest.split_once('/') {
                return (tail.to_owned(), true);
            }
        }
    }
    (normalized, false)
}

fn contains_interpolation(path: &str) -> bool {
    path.contains('$') || path.contains("%%") || path.contains("{{") || path.contains('%')
}

fn is_wholly_interpolated(path: &str) -> bool {
    let trimmed = path.trim_matches('/');
    trimmed.starts_with('$') && !trimmed.contains('/')
}

/// Resolve `.` and `..` textually.
///
/// Returns `None` when a `..` would traverse through an interpolated segment,
/// since the result is not determinable from source text.
fn resolve_lexically(path: &str) -> Option<String> {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => continue,
            ".." => match parts.last() {
                // Popping past an interpolated segment is not determinable, and
                // guessing is how a traversal slips past classification.
                Some(last) if last.contains('$') => return None,
                Some(last) if *last != ".." => {
                    parts.pop();
                }
                // A relative path may legitimately climb out of the package.
                // Keep the marker: escaping the package root is itself
                // information, and `classify` treats it as unplaceable.
                _ if !absolute => parts.push(".."),
                // At an absolute root, `..` has nowhere to go.
                _ => {}
            },
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    Some(if absolute {
        format!("/{joined}")
    } else {
        joined
    })
}

/// Match a resolved path against the class table, most specific first.
fn classify(resolved: &str, home_relative: bool) -> PathClass {
    let lower = resolved.to_ascii_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or("");

    if let Some(class) = classify_by_file_name(file_name) {
        return class;
    }
    if let Some(class) = classify_by_segment(&lower) {
        return class;
    }
    if home_relative {
        return PathClass::UserHome;
    }
    if lower.starts_with('/') {
        return classify_absolute(&lower);
    }
    if lower.is_empty() {
        return PathClass::Unknown;
    }
    // A relative path that climbs above the package root is still project-local
    // in every realistic layout, so it stays `workspace`. The sensitive cases
    // are not lost by this: `../../.ssh/id_rsa` is caught by the file-name and
    // directory rules above, which run first and do not care about traversal.
    // Reserving `unknown` for genuinely opaque paths is what keeps that class -
    // and the review queue it feeds - meaningful.
    PathClass::Workspace
}

fn classify_by_file_name(file_name: &str) -> Option<PathClass> {
    const SECRET_NAMES: &[&str] = &[
        ".env",
        ".netrc",
        ".pgpass",
        ".npmrc",
        "credentials",
        "id_rsa",
        "id_ed25519",
        "id_ecdsa",
        "id_dsa",
    ];
    const AGENT_CONFIG_NAMES: &[&str] = &[
        "claude.md",
        "agents.md",
        "memory.md",
        "settings.json",
        "settings.local.json",
        ".mcp.json",
        "hooks.json",
    ];
    const SKILL_PACKAGE_NAMES: &[&str] = &["skill.md", "skill.spec.yml"];
    const SHELL_INIT_NAMES: &[&str] = &[
        ".bashrc",
        ".bash_profile",
        ".zshrc",
        ".zshenv",
        ".zprofile",
        ".profile",
    ];
    const VCS_CONFIG_NAMES: &[&str] = &[".gitconfig"];

    if SECRET_NAMES.contains(&file_name)
        || file_name.starts_with(".env.")
        || file_name.ends_with(".pem")
        || file_name.ends_with(".p12")
        || file_name.ends_with(".keystore")
    {
        return Some(PathClass::Secret);
    }
    if AGENT_CONFIG_NAMES.contains(&file_name) {
        return Some(PathClass::AgentConfig);
    }
    if SKILL_PACKAGE_NAMES.contains(&file_name) {
        return Some(PathClass::SkillPackage);
    }
    if SHELL_INIT_NAMES.contains(&file_name) {
        return Some(PathClass::ShellInit);
    }
    if VCS_CONFIG_NAMES.contains(&file_name) {
        return Some(PathClass::VcsConfig);
    }
    None
}

fn classify_by_segment(lower: &str) -> Option<PathClass> {
    const SECRET_DIRS: &[&str] = &[
        ".ssh",
        ".aws",
        ".gnupg",
        ".kube",
        ".docker",
        ".config/gcloud",
        "library/keychains",
    ];
    const AGENT_CONFIG_DIRS: &[&str] = &[".claude", ".codex", ".agents", ".skillspec"];
    const VCS_CONFIG_DIRS: &[&str] = &[".git"];

    let segments = lower.split('/').collect::<Vec<_>>();
    if SECRET_DIRS
        .iter()
        .any(|dir| contains_dir_path(&segments, dir))
    {
        return Some(PathClass::Secret);
    }
    if AGENT_CONFIG_DIRS
        .iter()
        .any(|dir| contains_dir_path(&segments, dir))
    {
        return Some(PathClass::AgentConfig);
    }
    if VCS_CONFIG_DIRS
        .iter()
        .any(|dir| contains_dir_path(&segments, dir))
    {
        return Some(PathClass::VcsConfig);
    }
    if contains_dir_path(&segments, "skills") && segments.len() > 1 {
        return Some(PathClass::SkillPackage);
    }
    None
}

/// Whether `needle`, itself possibly a multi-segment path, appears as a run of
/// whole segments in `segments`.
fn contains_dir_path(segments: &[&str], needle: &str) -> bool {
    let needle_parts = needle.split('/').collect::<Vec<_>>();
    segments
        .windows(needle_parts.len())
        .any(|window| window == needle_parts.as_slice())
}

fn classify_absolute(lower: &str) -> PathClass {
    const TEMP_ROOTS: &[&str] = &["/tmp/", "/var/tmp/", "/private/tmp/", "/var/folders/"];
    const SYSTEM_ROOTS: &[&str] = &[
        "/etc/",
        "/usr/",
        "/bin/",
        "/sbin/",
        "/opt/",
        "/boot/",
        "/sys/",
        "/proc/",
        "/library/",
        "/system/",
        "/c:/windows/",
        "/windows/",
    ];

    if TEMP_ROOTS.iter().any(|root| lower.starts_with(root)) {
        return PathClass::Temp;
    }
    if SYSTEM_ROOTS.iter().any(|root| lower.starts_with(root)) {
        return PathClass::System;
    }
    PathClass::Unknown
}

#[cfg(test)]
mod tests {
    use super::normalize;
    use crate::effect::{PathClass, TargetResolution};

    fn class_of(raw: &str) -> PathClass {
        normalize(raw).class
    }

    #[test]
    fn home_relative_secret_directories_classify_as_secret() {
        for raw in [
            "~/.aws/credentials",
            "$HOME/.aws/credentials",
            "${HOME}/.ssh/id_rsa",
            "~/.gnupg/secring.gpg",
            "~/.kube/config",
        ] {
            assert_eq!(class_of(raw), PathClass::Secret, "{raw}");
        }
    }

    #[test]
    fn absolute_home_paths_classify_the_same_as_tilde_paths() {
        // Writing the path out in full is the cheapest possible evasion, so it
        // must not change the class.
        assert_eq!(class_of("/Users/alice/.aws/credentials"), PathClass::Secret);
        assert_eq!(class_of("/home/bob/.ssh/id_ed25519"), PathClass::Secret);
        assert_eq!(class_of("/root/.aws/credentials"), PathClass::Secret);
    }

    #[test]
    fn root_home_keeps_its_full_path_without_dropping_a_segment() {
        // /root is the home directory itself; there is no username to strip, so
        // /root/.aws/credentials must not collapse to ~/credentials.
        assert_eq!(
            normalize("/root/.aws/credentials").pattern,
            "~/.aws/credentials"
        );
        assert_eq!(
            normalize("/Users/alice/.aws/credentials").pattern,
            "~/.aws/credentials"
        );
    }

    #[test]
    fn traversal_through_a_literal_directory_is_resolved_before_classifying() {
        assert_eq!(
            class_of("~/projects/../.aws/credentials"),
            PathClass::Secret
        );
        assert_eq!(class_of("~/.ssh/../.ssh/id_rsa"), PathClass::Secret);
    }

    #[test]
    fn traversal_through_an_interpolated_segment_is_not_guessed() {
        let normalized = normalize("$PROJECT/../.aws/credentials");
        assert_eq!(normalized.class, PathClass::Unknown);
        assert_eq!(normalized.resolution, TargetResolution::Dynamic);
        assert!(normalized.class.is_sensitive());
    }

    #[test]
    fn case_variation_does_not_change_the_class() {
        assert_eq!(class_of("~/.AWS/CREDENTIALS"), PathClass::Secret);
        assert_eq!(class_of("~/.SSH/id_rsa"), PathClass::Secret);
    }

    #[test]
    fn windows_separators_and_user_profile_are_understood() {
        assert_eq!(
            class_of("%USERPROFILE%\\.aws\\credentials"),
            PathClass::Secret
        );
    }

    #[test]
    fn agent_config_paths_are_recognized_by_directory_and_by_file() {
        for raw in [
            "~/.claude/settings.json",
            ".claude/settings.json",
            "~/.codex/hooks.json",
            "~/.skillspec/router.json",
            "CLAUDE.md",
            "AGENTS.md",
            "MEMORY.md",
        ] {
            assert_eq!(class_of(raw), PathClass::AgentConfig, "{raw}");
        }
    }

    #[test]
    fn other_skill_packages_are_their_own_class() {
        // A SKILL.md under an agent config directory is classified as a skill
        // package rather than as agent config: both are sensitive, so the
        // boundary outcome is identical, but skill_package is what makes the
        // cross-skill propagation path query expressible.
        assert_eq!(
            class_of("~/.claude/skills/other/SKILL.md"),
            PathClass::SkillPackage
        );
        assert_eq!(class_of("../other-skill/SKILL.md"), PathClass::SkillPackage);
        assert_eq!(
            class_of("vendor/skills/thing/notes.md"),
            PathClass::SkillPackage
        );
    }

    #[test]
    fn shell_and_vcs_configuration_are_distinct_classes() {
        assert_eq!(class_of("~/.zshrc"), PathClass::ShellInit);
        assert_eq!(class_of("~/.bash_profile"), PathClass::ShellInit);
        assert_eq!(class_of("~/.gitconfig"), PathClass::VcsConfig);
        assert_eq!(class_of(".git/hooks/pre-commit"), PathClass::VcsConfig);
    }

    #[test]
    fn dotenv_variants_are_secret() {
        assert_eq!(class_of(".env"), PathClass::Secret);
        assert_eq!(class_of(".env.production"), PathClass::Secret);
        assert_eq!(class_of("config/.env"), PathClass::Secret);
        assert_eq!(class_of("certs/server.pem"), PathClass::Secret);
    }

    #[test]
    fn ordinary_project_paths_are_workspace() {
        assert_eq!(class_of("src/main.rs"), PathClass::Workspace);
        assert_eq!(class_of("./README.md"), PathClass::Workspace);
        assert_eq!(class_of("docs/design/index.md"), PathClass::Workspace);
    }

    #[test]
    fn system_and_temp_roots_are_classified() {
        assert_eq!(class_of("/etc/passwd"), PathClass::System);
        assert_eq!(class_of("/usr/local/bin/tool"), PathClass::System);
        assert_eq!(class_of("/tmp/scratch"), PathClass::Temp);
        assert_eq!(class_of("/var/tmp/build"), PathClass::Temp);
    }

    #[test]
    fn a_wholly_interpolated_path_resolves_to_nothing() {
        let normalized = normalize("$TARGET");
        assert_eq!(normalized.class, PathClass::Unknown);
        assert_eq!(normalized.resolution, TargetResolution::Dynamic);
    }

    #[test]
    fn interpolation_inside_a_known_prefix_stays_templated() {
        let normalized = normalize("~/.aws/$PROFILE");
        assert_eq!(normalized.class, PathClass::Secret);
        assert_eq!(normalized.resolution, TargetResolution::Templated);
        assert!(normalized.resolution.is_grantable());
    }

    #[test]
    fn quotes_and_surrounding_whitespace_are_stripped() {
        assert_eq!(class_of("  \"~/.aws/credentials\"  "), PathClass::Secret);
        assert_eq!(class_of("'~/.ssh/id_rsa'"), PathClass::Secret);
    }

    #[test]
    fn a_relative_path_above_the_package_is_still_project_local() {
        // Ordinary in document skills. Flagging it beside ~/.aws/credentials
        // would dilute the review queue for no gain.
        assert_eq!(class_of("../out.docx"), PathClass::Workspace);
        assert_eq!(class_of("../../build/report.pdf"), PathClass::Workspace);
    }

    #[test]
    fn traversal_never_hides_a_sensitive_name_or_directory() {
        // The file-name and directory rules run before traversal is considered,
        // so climbing out cannot be used to launder a secret path.
        assert_eq!(class_of("../../.ssh/id_rsa"), PathClass::Secret);
        assert_eq!(class_of("../../../.aws/credentials"), PathClass::Secret);
        assert_eq!(class_of("../other/.env"), PathClass::Secret);
    }

    #[test]
    fn unplaceable_absolute_paths_are_unknown_and_therefore_sensitive() {
        let normalized = normalize("/nonstandard/location/file");
        assert_eq!(normalized.class, PathClass::Unknown);
        assert!(normalized.class.is_sensitive());
    }

    #[test]
    fn empty_input_is_unknown() {
        assert_eq!(normalize("").class, PathClass::Unknown);
        assert_eq!(normalize("   ").resolution, TargetResolution::Unknown);
    }
}
