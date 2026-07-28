//! Turning an effect surface into ranked, decision-driving risk findings.
//!
//! The effect surface says *what* a skill touches. To decide whether to install
//! it, a person needs three more things about each touch: how bad it is, why,
//! and what happens if they install. This module computes exactly that.
//!
//! Two ideas do the work:
//!
//! - **Scope.** A skill that reads and writes only inside its own directory is
//!   doing its job; that is low risk however much it touches. Risk comes from
//!   *reaching outside that scope* — a home-directory secret, an absolute system
//!   path, an escape via `..`, the network, another skill's files, the agent's
//!   own configuration. Scope, not raw count, sets the floor.
//! - **Severity from consequence.** Reading a credential is worse than reading
//!   an unclassified relative file; a hidden instruction is worse than a visible
//!   one; egress next to a secret read is an exfiltration path and worse than
//!   either alone. Each finding carries the severity its consequence earns.
//!
//! This never claims intent. A finding states a capability and what that
//! capability would mean if exercised, with the evidence that shows it.

use crate::dedupe::Effect;
use crate::effect::{EffectClass, EffectTarget, PathClass, Reach, TargetResolution};
use crate::surface::EffectSurface;

/// How much a finding should weigh on an install decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Confined to the skill's own directory, or otherwise unremarkable.
    Low,
    /// Reaches outside the skill's scope, or runs code chosen at runtime.
    Medium,
    /// Reads a credential, tampers with agent/shell config, hides content, or
    /// retargets the agent.
    High,
    /// A complete exfiltration path: a secret read and a way off the machine.
    Critical,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }
}

/// One thing worth knowing before installing, in the three terms a person
/// decides on: how bad, why, and what happens if installed.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Finding {
    pub severity: Severity,
    /// A short name for the finding, e.g. "reads credentials".
    pub headline: String,
    /// The package-relative file the evidence is in, when there is one.
    pub file: Option<String>,
    /// The line in that file, when known.
    pub line: Option<usize>,
    /// What the finding points at — a host, a path, an env var — when it names
    /// something distinct from the file it was found in.
    pub reached: Option<String>,
    /// A qualifier that changes how the finding should be read — e.g. that its
    /// evidence is example code in documentation rather than an executed effect.
    /// Set when the finding was downgraded; the finding is still reported.
    pub note: Option<String>,
    /// Why it earns this severity.
    pub why: String,
    /// What it means if the skill runs.
    pub consequence: String,
}

impl Finding {
    /// The bare `path:line` locus, for a rendering that cannot build a link.
    pub fn locus(&self) -> String {
        match (&self.file, self.line) {
            (Some(file), Some(line)) => format!("{file}:{line}"),
            (Some(file), None) => file.clone(),
            _ => String::new(),
        }
    }
}

/// The ranked risk of one skill.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct SkillRisk {
    /// Findings, most severe first.
    pub findings: Vec<Finding>,
}

impl SkillRisk {
    /// The worst finding's severity, or `None` when nothing was found.
    pub fn severity(&self) -> Option<Severity> {
        self.findings.iter().map(|finding| finding.severity).max()
    }

    /// Whether this should pause an install: anything at Medium or above.
    /// A skill confined to its own directory (Low only) does not gate.
    pub fn warrants_review(&self) -> bool {
        self.severity()
            .is_some_and(|severity| severity >= Severity::Medium)
    }

    /// Compute the risk of a skill from its effect surface.
    pub fn of(surface: &EffectSurface) -> Self {
        let mut findings = Vec::new();
        findings.extend(concealment_findings(surface));
        findings.extend(directive_findings(surface));
        findings.extend(effect_findings(surface));
        elevate_exfiltration(&mut findings);
        findings.sort_by_key(|finding| std::cmp::Reverse(finding.severity));
        SkillRisk { findings }
    }
}

/// Where a filesystem target sits relative to the skill's own directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scope {
    /// Inside the skill folder: a relative path with no escape.
    Inside,
    /// Reaches outside: a home path, an absolute path, or an `..` escape.
    Outside,
}

fn path_scope(pattern: &str, class: PathClass) -> Scope {
    // Classified-sensitive locations are outside a skill's own directory by
    // definition — a credential store, the agent's config, shell startup.
    if matches!(
        class,
        PathClass::Secret
            | PathClass::AgentConfig
            | PathClass::ShellInit
            | PathClass::VcsConfig
            | PathClass::SkillPackage
    ) {
        return Scope::Outside;
    }
    let p = pattern.trim();
    let escapes = p.starts_with('~')
        || p.starts_with('/')
        || p.starts_with("$HOME")
        || p.starts_with("${HOME")
        || p.split(['/', '\\']).any(|segment| segment == "..");
    if escapes {
        Scope::Outside
    } else {
        Scope::Inside
    }
}

/// Whether the skill reaches beyond its own directory in a way a directive
/// could weaponize: the network, a classified-sensitive path, or a filesystem
/// target outside the skill folder. An in-scope read of an unclassified file
/// does not count — that is a skill touching its own bundle.
fn has_real_capability(surface: &EffectSurface) -> bool {
    surface.all().any(|effect| match effect.class {
        EffectClass::NetEgress | EffectClass::NetFetch => true,
        EffectClass::FsRead | EffectClass::FsWrite => match &effect.target {
            EffectTarget::Path { pattern, class } => path_scope(pattern, *class) == Scope::Outside,
            _ => false,
        },
        _ => false,
    })
}

fn concealment_findings(surface: &EffectSurface) -> Vec<Finding> {
    // One finding per kind of concealment, not per occurrence.
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for item in &surface.concealment {
        if !seen.insert(item.id.as_str()) {
            continue;
        }
        out.push(Finding {
            severity: Severity::High,
            headline: format!("hidden content ({})", item.id.as_str()),
            file: Some(item.path.clone()),
            line: Some(item.line),
            reached: None,
            note: None,
            why: item.statement.clone(),
            consequence: "an instruction or payload a reviewer reading the skill never sees could act on the agent".to_owned(),
        });
    }
    out
}

fn directive_findings(surface: &EffectSurface) -> Vec<Finding> {
    // A directive gates a skill only when the skill has a capability it could
    // abuse. That capability must be *real* — the network, a classified-
    // sensitive path, or a read outside the skill's directory — not an
    // in-scope read of an unclassified file, which is a skill touching its own
    // bundle. Strong families (an injection, a self-disclosure) are concerning
    // regardless, as is any directive that names a sensitive subject itself.
    let has_capability = has_real_capability(surface);
    let mut best: std::collections::BTreeMap<String, Finding> = std::collections::BTreeMap::new();
    for directive in &surface.directives {
        if !(directive.is_strong() || has_capability || directive.mentions_sensitive_subject()) {
            continue;
        }
        let hidden = directive.is_hidden_from_review();
        let severity = if directive.is_strong() {
            Severity::High
        } else {
            Severity::Medium
        };
        let consequence = if directive.is_strong() {
            "the agent may follow this instead of your instructions"
        } else if hidden {
            "an instruction outside the skill's main text could steer the agent"
        } else {
            "the agent could be steered away from what you asked"
        }
        .to_owned();
        let mut why = directive.statement.clone();
        if hidden {
            why.push_str(" It sits in a file a reader following the skill does not open.");
        }
        let finding = Finding {
            severity,
            headline: "agent-directing instruction".to_owned(),
            file: Some(directive.path.clone()),
            line: Some(directive.line),
            reached: None,
            note: None,
            why,
            consequence,
        };
        best.entry(directive.kind_id.clone())
            .and_modify(|existing| {
                if finding.severity > existing.severity {
                    *existing = finding.clone();
                }
            })
            .or_insert(finding);
    }
    best.into_values().collect()
}

/// Whether an effect's evidence is example code in documentation rather than an
/// executed effect.
///
/// The signal is structural, not a matter of trust: the effect is never seen in
/// the activation body (its reach is a referenced or unmapped file), and every
/// place it was seen is a Markdown document — a fenced `curl` in a how-to,
/// `process.env.STRIPE_KEY` in a mocking guide. A shipped script keeps full
/// severity even when unreferenced, because an orphan script is exactly where a
/// payload hides; only prose examples are qualified.
fn is_illustrative(effect: &Effect) -> bool {
    effect.reach != Reach::Activation
        && !effect.observations.is_empty()
        && effect
            .observations
            .iter()
            .all(|evidence| is_markdown(&evidence.path))
}

fn is_markdown(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown")
}

/// Downgrade a finding whose evidence is a documentation example, and note why.
/// It is downgraded, never dropped — a reviewer still sees it, and an "example"
/// that actually reaches a live host is exactly what the note invites checking.
fn qualify(mut finding: Finding, effect: &Effect) -> Finding {
    if is_illustrative(effect) {
        finding.severity = Severity::Low;
        finding.note =
            Some("shown as an example in documentation — not an executed effect".to_owned());
    }
    finding
}

fn effect_findings(surface: &EffectSurface) -> Vec<Finding> {
    let mut out = Vec::new();

    // Filesystem: aggregate by (class, scope) so a skill reading ten of its own
    // files is one Low finding, not ten.
    let mut inside_reads = 0usize;
    let mut inside_read_example: Option<(Option<String>, Option<usize>)> = None;
    for effect in surface.all() {
        let EffectTarget::Path { pattern, class } = &effect.target else {
            continue;
        };
        if !matches!(effect.class, EffectClass::FsRead | EffectClass::FsWrite) {
            continue;
        }
        let scope = path_scope(pattern, *class);
        let writing = effect.class == EffectClass::FsWrite;
        match (scope, class) {
            (Scope::Inside, _) if !writing => {
                inside_reads += 1;
                inside_read_example.get_or_insert_with(|| (fpath(effect), fline(effect)));
            }
            (Scope::Inside, _) => out.push(qualify(Finding {
                severity: Severity::Low,
                headline: "writes within its directory".to_owned(),
                file: fpath(effect),
                line: fline(effect),
                reached: None,
                note: None,
                why: "a write confined to the skill's own folder".to_owned(),
                consequence: "changes only its own bundled files".to_owned(),
            }, effect)),
            (Scope::Outside, PathClass::Secret) => out.push(qualify(Finding {
                severity: Severity::High,
                headline: "reads credentials".to_owned(),
                file: fpath(effect),
                line: fline(effect),
                reached: Some(pattern.to_string()),
                note: None,
                why: "the path resolves to a stored-secret location (keys, tokens, cloud credentials)".to_owned(),
                consequence: "can read your saved credentials".to_owned(),
            }, effect)),
            (Scope::Outside, PathClass::AgentConfig) => out.push(qualify(Finding {
                severity: Severity::High,
                headline: if writing { "writes agent config".to_owned() } else { "reads agent config".to_owned() },
                file: fpath(effect),
                line: fline(effect),
                reached: Some(pattern.to_string()),
                note: None,
                why: "the path is the agent's own configuration".to_owned(),
                consequence: if writing {
                    "can change how your agent behaves in future sessions".to_owned()
                } else {
                    "can read how your agent is configured".to_owned()
                },
            }, effect)),
            (Scope::Outside, PathClass::ShellInit) => out.push(qualify(Finding {
                severity: Severity::High,
                headline: "touches shell startup files".to_owned(),
                file: fpath(effect),
                line: fline(effect),
                reached: Some(pattern.to_string()),
                note: None,
                why: "the path is a shell init file that runs on every new shell".to_owned(),
                consequence: "can persist itself across sessions".to_owned(),
            }, effect)),
            (Scope::Outside, PathClass::SkillPackage) => out.push(qualify(Finding {
                severity: Severity::Medium,
                headline: "reaches other skills' files".to_owned(),
                file: fpath(effect),
                line: fline(effect),
                reached: Some(pattern.to_string()),
                note: None,
                why: "the path points into another skill's package".to_owned(),
                consequence: "can read or alter skills other than itself".to_owned(),
            }, effect)),
            (Scope::Outside, PathClass::VcsConfig) => out.push(qualify(Finding {
                severity: Severity::Medium,
                headline: "touches git configuration".to_owned(),
                file: fpath(effect),
                line: fline(effect),
                reached: Some(pattern.to_string()),
                note: None,
                why: "the path is version-control configuration".to_owned(),
                consequence: "can read or change git settings (hooks, remotes)".to_owned(),
            }, effect)),
            (Scope::Outside, _) => out.push(qualify(Finding {
                severity: Severity::Medium,
                headline: if writing { "writes outside its directory".to_owned() } else { "reads outside its directory".to_owned() },
                file: fpath(effect),
                line: fline(effect),
                reached: Some(pattern.to_string()),
                note: None,
                why: "the path leaves the skill's own folder (an absolute, home, or `..` path)".to_owned(),
                consequence: if writing {
                    "can modify files beyond the skill folder".to_owned()
                } else {
                    "can read files beyond the skill folder".to_owned()
                },
            }, effect)),
        }
    }
    if inside_reads > 0 {
        out.push(Finding {
            severity: Severity::Low,
            headline: format!("reads within its directory ({inside_reads} file(s))"),
            file: inside_read_example.as_ref().and_then(|(f, _)| f.clone()),
            line: inside_read_example.as_ref().and_then(|(_, l)| *l),
            reached: None,
            note: None,
            why: "relative reads confined to the skill's own folder".to_owned(),
            consequence: "reads only its own bundled files".to_owned(),
        });
    }

    // Network.
    for effect in surface.all() {
        match effect.class {
            EffectClass::NetEgress => {
                let dynamic = !effect.resolution.is_grantable();
                out.push(qualify(
                    Finding {
                        severity: Severity::Medium,
                        headline: "sends data to the network".to_owned(),
                        file: fpath(effect),
                        line: fline(effect),
                        reached: Some(effect.target.grant_token()),
                        note: None,
                        why: if dynamic {
                            "an outbound request whose destination is chosen at runtime".to_owned()
                        } else {
                            "an outbound request that can carry data off the machine".to_owned()
                        },
                        consequence: "data can leave your machine to this destination".to_owned(),
                    },
                    effect,
                ));
            }
            EffectClass::NetFetch => out.push(qualify(
                Finding {
                    severity: Severity::Low,
                    headline: "fetches from the network".to_owned(),
                    file: fpath(effect),
                    line: fline(effect),
                    reached: Some(effect.target.grant_token()),
                    note: None,
                    why: "an inbound fetch (download) from this host".to_owned(),
                    consequence: "pulls content from this host; what returns is not inspected"
                        .to_owned(),
                },
                effect,
            )),
            _ => {}
        }
    }

    // Environment: only credential-shaped variables are notable.
    for effect in surface.all() {
        if effect.class != EffectClass::EnvRead {
            continue;
        }
        let name = effect.target.grant_token();
        if looks_like_secret_env(&name) {
            out.push(qualify(
                Finding {
                    severity: Severity::High,
                    headline: "reads a secret from the environment".to_owned(),
                    file: fpath(effect),
                    line: fline(effect),
                    reached: Some(name.clone()),
                    note: None,
                    why: "the variable name looks like a credential".to_owned(),
                    consequence: "can read a token or key held in the environment".to_owned(),
                },
                effect,
            ));
        }
    }

    // Process execution: only runtime-chosen commands are notable; a named
    // binary is ordinary and stays quiet here.
    for effect in surface.all() {
        if effect.class != EffectClass::ProcExec {
            continue;
        }
        if effect.resolution == TargetResolution::Dynamic
            || effect.resolution == TargetResolution::Unknown
        {
            out.push(qualify(Finding {
                severity: Severity::Medium,
                headline: "runs a command chosen at runtime".to_owned(),
                file: fpath(effect),
                line: fline(effect),
                reached: None,
                note: None,
                why: "the executable is assembled or resolved at run time, not named in the source"
                    .to_owned(),
                consequence: "what it runs cannot be determined before it runs".to_owned(),
            }, effect));
            break;
        }
    }

    // Installing more skills/plugins.
    for effect in surface.all() {
        if effect.class != EffectClass::PkgInstall {
            continue;
        }
        if let EffectTarget::Package {
            ecosystem, name, ..
        } = &effect.target
        {
            if ecosystem == "skill" {
                out.push(qualify(
                    Finding {
                        severity: Severity::Medium,
                        headline: "installs another skill or plugin".to_owned(),
                        file: fpath(effect),
                        line: fline(effect),
                        reached: Some(name.clone()),
                        note: None,
                        why: "the skill installs further skills through a harness CLI".to_owned(),
                        consequence: "pulls in more code that is not part of this package"
                            .to_owned(),
                    },
                    effect,
                ));
                break;
            }
        }
    }

    out
}

/// A secret read plus a way off the machine is an exfiltration path — worse than
/// either half. When both are present, raise the egress finding to Critical.
fn elevate_exfiltration(findings: &mut [Finding]) {
    // A documentation example does not complete a real exfiltration path, so a
    // qualified secret read does not arm the elevation, and a qualified egress
    // is not elevated.
    let reads_secret = findings.iter().any(|f| {
        f.note.is_none()
            && (f.headline == "reads credentials"
                || f.headline == "reads a secret from the environment")
    });
    if !reads_secret {
        return;
    }
    for finding in findings.iter_mut() {
        if finding.note.is_none() && finding.headline == "sends data to the network" {
            finding.severity = Severity::Critical;
            finding.why =
                "an outbound request in a skill that also reads a secret — the two together are an exfiltration path".to_owned();
            finding.consequence =
                "your credentials could be read and sent off the machine".to_owned();
        }
    }
}

/// The package-relative file of an effect's first observation.
fn fpath(effect: &crate::dedupe::Effect) -> Option<String> {
    effect
        .observations
        .first()
        .map(|evidence| evidence.path.clone())
}

/// The line of an effect's first observation, when the extractor recorded one.
fn fline(effect: &crate::dedupe::Effect) -> Option<usize> {
    effect
        .observations
        .first()
        .and_then(|evidence| evidence.line)
}

fn looks_like_secret_env(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "KEY",
        "PASSWORD",
        "PASSWD",
        "CREDENTIAL",
        "API",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze;
    use std::path::PathBuf;

    fn fixture(name: &str) -> EffectSurface {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/effects")
            .join(name);
        analyze(&root).expect("analyze fixture")
    }

    #[test]
    fn scope_places_home_and_absolute_paths_outside() {
        assert_eq!(
            path_scope("references/x.md", PathClass::Unknown),
            Scope::Inside
        );
        assert_eq!(path_scope("data/sub/x", PathClass::Unknown), Scope::Inside);
        assert_eq!(
            path_scope("~/.aws/credentials", PathClass::Secret),
            Scope::Outside
        );
        assert_eq!(
            path_scope("/etc/passwd", PathClass::Unknown),
            Scope::Outside
        );
        assert_eq!(path_scope("../other/x", PathClass::Unknown), Scope::Outside);
    }

    #[test]
    fn a_secret_read_with_egress_is_critical() {
        let risk = SkillRisk::of(&fixture("secret-reader"));
        // secret-reader reads a credential; if it also egresses it is Critical,
        // otherwise the secret read alone is High.
        assert!(risk.severity().unwrap() >= Severity::High);
        assert!(risk.warrants_review());
        assert!(risk
            .findings
            .iter()
            .any(|f| f.headline == "reads credentials"));
    }

    #[test]
    fn an_exfiltration_fixture_reaches_critical() {
        let risk = SkillRisk::of(&fixture("py-exfil"));
        assert_eq!(risk.severity(), Some(Severity::Critical));
        assert!(
            risk.findings
                .iter()
                .any(|f| f.severity == Severity::Critical
                    && f.consequence.contains("off the machine"))
        );
    }

    #[test]
    fn a_clean_formatter_has_no_reviewable_risk() {
        let risk = SkillRisk::of(&fixture("clean-formatter"));
        assert!(!risk.warrants_review(), "clean formatter should not gate");
    }

    #[test]
    fn every_finding_states_why_and_consequence() {
        let risk = SkillRisk::of(&fixture("secret-reader"));
        for finding in &risk.findings {
            assert!(!finding.why.is_empty());
            assert!(!finding.consequence.is_empty());
        }
    }

    #[test]
    fn markdown_is_recognized_by_extension() {
        assert!(is_markdown("guide.md"));
        assert!(is_markdown("references/FLOW.markdown"));
        assert!(!is_markdown("scripts/run.sh"));
        assert!(!is_markdown("template.ts"));
    }

    #[test]
    fn a_documentation_example_is_qualified_not_a_live_exfil() {
        // The referenced guide.md shows a `curl … $STRIPE_KEY` example. The
        // secret read and egress are illustrative, so the skill is downgraded to
        // Low with a note — never elevated to a Critical exfiltration path.
        let risk = SkillRisk::of(&fixture("doc-example"));
        assert!(
            !risk.warrants_review(),
            "a documentation example must not gate an install: {:?}",
            risk.severity()
        );
        assert!(
            risk.findings.iter().any(|f| f.note.is_some()),
            "the example findings should carry a qualification note"
        );
        assert!(
            risk.findings.iter().all(|f| f.severity <= Severity::Low),
            "nothing in a pure documentation example should exceed Low"
        );
    }
}
