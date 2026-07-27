//! The effect model: what a skill could reach if an agent executed it.
//!
//! An *effect* is one host-visible action a skill could cause. An
//! [`EffectObservation`] is one evidence-backed sighting of such an action;
//! several observations of the same thing collapse into one effect during
//! deduplication.
//!
//! Two independent axes describe how sure the analysis is:
//!
//! - [`Confidence`] - how sure we are the effect exists at all.
//! - [`TargetResolution`] - how completely the target could be determined.
//!
//! They are genuinely independent. `curl "$URL"` is a [`Confidence::High`]
//! network effect with [`TargetResolution::Dynamic`] resolution.
//!
//! See `docs/design/security/36-skill-effect-surface.md`.

use crate::sanitize::Preview;
use serde::Serialize;
use std::fmt;

/// The kind of host-visible action an effect represents.
///
/// Deliberately small: each variant must map onto something a real permission
/// system can express, because these become grants in a boundary proposal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    /// Sends data off-host. The exfiltration channel.
    NetEgress,
    /// Brings remote content into the run. The untrusted-input channel.
    NetFetch,
    /// Reads a path.
    FsRead,
    /// Writes, creates, or deletes a path.
    FsWrite,
    /// Invokes a binary or subprocess.
    ProcExec,
    /// Reads an environment variable.
    EnvRead,
    /// Calls a harness or MCP tool.
    ToolInvoke,
    /// Writes agent-controlling state that outlives the run.
    AgentConfig,
    /// Installs a dependency.
    PkgInstall,
}

impl EffectClass {
    /// Stable token used as the left half of a grant label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NetEgress => "net.egress",
            Self::NetFetch => "net.fetch",
            Self::FsRead => "fs.read",
            Self::FsWrite => "fs.write",
            Self::ProcExec => "proc.exec",
            Self::EnvRead => "env.read",
            Self::ToolInvoke => "tool.invoke",
            Self::AgentConfig => "agent.config",
            Self::PkgInstall => "pkg.install",
        }
    }
}

impl fmt::Display for EffectClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What an effect acts on, normalized so two sightings can be compared.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectTarget {
    /// A network host.
    Host {
        host: String,
        scheme: Option<String>,
        port: Option<u16>,
        /// Host is a bare IP address; never collapses into a wildcard grant.
        ip_literal: bool,
        /// Host carries non-ASCII characters, which may be a homoglyph.
        non_ascii: bool,
    },
    /// A filesystem path, carrying the class that drives boundary generation.
    Path { pattern: String, class: PathClass },
    /// An executable, reduced to its basename.
    Binary {
        name: String,
        /// Reached through `sudo` or an equivalent elevation wrapper.
        privileged: bool,
    },
    /// An environment variable.
    EnvVar {
        name: String,
        /// Name matches a credential-shaped pattern.
        credential_like: bool,
    },
    /// A harness or MCP tool.
    Tool { id: String },
    /// A package from a dependency ecosystem.
    Package {
        ecosystem: String,
        name: String,
        pinned: bool,
    },
}

impl EffectTarget {
    /// The right half of a grant label.
    ///
    /// Aggregation happens here, at the level each target kind defines: hosts
    /// grant exactly, paths grant by class, binaries grant by basename. Host
    /// grants never widen to a parent domain; widening is the usual way a
    /// least-privilege policy quietly becomes a permissive one, and the cost of
    /// refusing is a slightly longer list.
    pub fn grant_token(&self) -> String {
        match self {
            Self::Host { host, .. } => host.clone(),
            Self::Path { class, .. } => class.as_str().to_owned(),
            Self::Binary { name, .. } => name.clone(),
            Self::EnvVar { name, .. } => name.clone(),
            Self::Tool { id } => id.clone(),
            Self::Package {
                ecosystem, name, ..
            } => format!("{ecosystem}/{name}"),
        }
    }

    /// The path class, when this target is a path.
    pub fn path_class(&self) -> Option<PathClass> {
        match self {
            Self::Path { class, .. } => Some(*class),
            _ => None,
        }
    }
}

/// Sensitivity classification for a filesystem path.
///
/// The class, not the literal path, drives boundary generation. A literal-path
/// allow-list is unreadable and breaks on the first legitimate variation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathClass {
    /// Credentials and key material.
    Secret,
    /// Agent-controlling state that outlives the run.
    AgentConfig,
    /// A skill package other than the one being analyzed.
    SkillPackage,
    /// Shell startup files.
    ShellInit,
    /// Version-control configuration and hooks.
    VcsConfig,
    /// Inside the current project.
    Workspace,
    /// Elsewhere under the user's home directory.
    UserHome,
    /// System locations.
    System,
    /// Temporary directories.
    Temp,
    /// Normalization could not place the path.
    Unknown,
}

impl PathClass {
    /// Stable token used in grant labels.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Secret => "secret",
            Self::AgentConfig => "agent_config",
            Self::SkillPackage => "skill_package",
            Self::ShellInit => "shell_init",
            Self::VcsConfig => "vcs_config",
            Self::Workspace => "workspace",
            Self::UserHome => "user_home",
            Self::System => "system",
            Self::Temp => "temp",
            Self::Unknown => "unknown",
        }
    }

    /// Classes that never become a silent allow grant.
    ///
    /// This is the single place the rule lives. The proposal compiler, the
    /// report ordering, and the drift classifier all call it rather than
    /// re-listing variants.
    ///
    /// `Unknown` is included deliberately. Path normalization is lexical - it
    /// never touches the filesystem, so a report cannot depend on the analyst's
    /// machine - which means it fails on exactly the forms an evasion takes:
    /// absolute home paths on an unfamiliar layout, traversal through a
    /// symlink, platform-specific variable syntax. Treating unplaceable paths as
    /// ordinary would make this the one point where uncertainty resolves toward
    /// permission.
    pub fn is_sensitive(self) -> bool {
        matches!(
            self,
            Self::Secret
                | Self::AgentConfig
                | Self::SkillPackage
                | Self::ShellInit
                | Self::VcsConfig
                | Self::Unknown
        )
    }
}

/// How completely an effect's target could be determined from source text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetResolution {
    /// Fully determined: `curl https://api.github.com/user`.
    Literal,
    /// Determined except for interpolated segments that do not change the
    /// grant: `curl https://api.github.com/repos/$OWNER/$REPO`.
    Templated,
    /// Computed at runtime: `curl "$ENDPOINT"`.
    Dynamic,
    /// An effect is present but no target could be attributed.
    Unknown,
}

impl TargetResolution {
    /// Whether this resolution can become a grant.
    ///
    /// Dynamic and unknown targets are reported as unresolved effects rather
    /// than guessed at. They are the primary input to human review.
    pub fn is_grantable(self) -> bool {
        matches!(self, Self::Literal | Self::Templated)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::Templated => "templated",
            Self::Dynamic => "dynamic",
            Self::Unknown => "unknown",
        }
    }
}

/// Where in the package an effect was found.
///
/// Ordering is by declaration, most visible first, so [`Ord`] agrees with
/// [`Reach::most_visible`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach {
    /// In the `SKILL.md` body, loaded into context at activation.
    Activation,
    /// In a file the Markdown references.
    Deferred,
    /// In a package file that nothing in the Markdown references.
    Unmapped,
}

impl Reach {
    /// The more visible of two reaches.
    ///
    /// Merging sightings must keep the most visible one, so an effect present
    /// both in an unmapped script and in the documented body is not filed as
    /// though it were only reachable through an undocumented file.
    ///
    /// Think in visibility, not breadth: `Activation` is the *minimum* under the
    /// derived ordering, and the inverted reading is an easy defect to ship.
    pub fn most_visible(self, other: Self) -> Self {
        self.min(other)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Activation => "activation",
            Self::Deferred => "deferred",
            Self::Unmapped => "unmapped",
        }
    }
}

/// Which part of a package an observation was read from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOrigin {
    Frontmatter,
    MarkdownProse,
    MarkdownCommandExample,
    MarkdownCodeBlock,
    ScriptFile,
    Manifest,
}

/// How sure the extractor is that an effect exists at all.
///
/// Ordered most confident first, matching [`crate::effect::Reach`] and the
/// severity conventions elsewhere in the workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Syntactically unambiguous: a URL literal, an `argv[0]`.
    High,
    /// Pattern-derived: a path mentioned in prose.
    Medium,
    /// Weak signal retained because omission fails open.
    Low,
}

impl Confidence {
    /// The stronger of two confidences.
    pub fn strongest(self, other: Self) -> Self {
        self.min(other)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Where an observation was seen, precise enough for a reviewer to find it.
///
/// `text_preview` is a [`Preview`], so it cannot carry unsanitized package
/// text.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct EffectEvidence {
    /// Package-relative path.
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Source-map node id, when the observation came from Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    pub text_preview: Preview,
}

impl EffectEvidence {
    /// Build evidence, sanitizing `raw` on the way in.
    pub fn new(path: impl Into<String>, line: Option<usize>, raw: &str) -> Self {
        Self {
            path: path.into(),
            line,
            node: None,
            text_preview: Preview::of(raw),
        }
    }

    /// Attach the source-map node this observation came from.
    pub fn with_node(mut self, node: impl Into<String>) -> Self {
        self.node = Some(node.into());
        self
    }
}

/// One evidence-backed sighting of an effect.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectObservation {
    pub class: EffectClass,
    pub target: EffectTarget,
    pub resolution: TargetResolution,
    pub origin: EffectOrigin,
    pub reach: Reach,
    pub confidence: Confidence,
    pub evidence: EffectEvidence,
}

impl EffectObservation {
    /// The key two observations must agree on to merge into one effect.
    pub fn merge_key(&self) -> (EffectClass, &EffectTarget, TargetResolution) {
        (self.class, &self.target, self.resolution)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Confidence, EffectClass, EffectEvidence, EffectTarget, PathClass, Reach, TargetResolution,
    };

    #[test]
    fn reach_keeps_the_most_visible_sighting() {
        // Regression guard for the inverted-ordering trap: an effect seen both
        // in an unmapped script and in the documented body must merge to
        // activation, not to unmapped.
        assert_eq!(
            Reach::Unmapped.most_visible(Reach::Activation),
            Reach::Activation
        );
        assert_eq!(
            Reach::Activation.most_visible(Reach::Unmapped),
            Reach::Activation
        );
        assert_eq!(
            Reach::Deferred.most_visible(Reach::Unmapped),
            Reach::Deferred
        );
        assert_eq!(
            Reach::Activation.most_visible(Reach::Activation),
            Reach::Activation
        );
    }

    #[test]
    fn reach_ordering_places_activation_first() {
        let mut reaches = [Reach::Unmapped, Reach::Activation, Reach::Deferred];
        reaches.sort();
        assert_eq!(
            reaches,
            [Reach::Activation, Reach::Deferred, Reach::Unmapped]
        );
    }

    #[test]
    fn confidence_keeps_the_strongest_sighting() {
        assert_eq!(
            Confidence::Low.strongest(Confidence::High),
            Confidence::High
        );
        assert_eq!(
            Confidence::Medium.strongest(Confidence::Low),
            Confidence::Medium
        );
    }

    #[test]
    fn unknown_paths_are_sensitive() {
        // Uncertainty resolves toward review, not toward permission. This is
        // the one assertion that keeps the fail-closed property honest.
        assert!(PathClass::Unknown.is_sensitive());
    }

    #[test]
    fn sensitive_classes_are_exactly_the_documented_six() {
        let sensitive = [
            PathClass::Secret,
            PathClass::AgentConfig,
            PathClass::SkillPackage,
            PathClass::ShellInit,
            PathClass::VcsConfig,
            PathClass::Unknown,
        ];
        let ordinary = [
            PathClass::Workspace,
            PathClass::UserHome,
            PathClass::System,
            PathClass::Temp,
        ];
        assert!(sensitive.iter().all(|class| class.is_sensitive()));
        assert!(!ordinary.iter().any(|class| class.is_sensitive()));
    }

    #[test]
    fn only_literal_and_templated_targets_can_be_granted() {
        assert!(TargetResolution::Literal.is_grantable());
        assert!(TargetResolution::Templated.is_grantable());
        assert!(!TargetResolution::Dynamic.is_grantable());
        assert!(!TargetResolution::Unknown.is_grantable());
    }

    #[test]
    fn host_grants_do_not_widen_to_a_parent_domain() {
        let api = EffectTarget::Host {
            host: "api.github.com".to_owned(),
            scheme: Some("https".to_owned()),
            port: None,
            ip_literal: false,
            non_ascii: false,
        };
        let raw = EffectTarget::Host {
            host: "raw.githubusercontent.com".to_owned(),
            scheme: Some("https".to_owned()),
            port: None,
            ip_literal: false,
            non_ascii: false,
        };
        assert_eq!(api.grant_token(), "api.github.com");
        assert_ne!(api.grant_token(), raw.grant_token());
    }

    #[test]
    fn path_grants_aggregate_to_their_class() {
        let target = EffectTarget::Path {
            pattern: "~/.aws/credentials".to_owned(),
            class: PathClass::Secret,
        };
        assert_eq!(target.grant_token(), "secret");
        assert_eq!(target.path_class(), Some(PathClass::Secret));
    }

    #[test]
    fn grant_labels_compose_class_and_target() {
        let target = EffectTarget::Binary {
            name: "git".to_owned(),
            privileged: false,
        };
        assert_eq!(
            format!("{}:{}", EffectClass::ProcExec, target.grant_token()),
            "proc.exec:git"
        );
    }

    #[test]
    fn evidence_sanitizes_quoted_text_on_construction() {
        // There is no way to build evidence holding raw package text.
        let evidence = EffectEvidence::new("SKILL.md", Some(12), "run\u{200b}`this`");
        assert!(!evidence.text_preview.as_str().contains('\u{200b}'));
        assert!(!evidence.text_preview.as_str().contains('`'));
    }
}
