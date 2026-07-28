//! The shared pre-install assessment: show the shape, state the risk in terms a
//! person can decide on, and let them choose.
//!
//! `gate` and `pull` both do the same thing before touching anything — map the
//! target, report what any skill could reach *and how bad that is, why, where,
//! and what happens if it runs*, and confirm. That flow lives here so the two
//! commands cannot drift apart.
//!
//! Every finding's `where:` is made actionable: a clickable blob URL when the
//! target is a remote repository, an openable absolute path when it is on disk —
//! so a reviewer can go straight to the line and decide.

use skillspec::boundary::risk::{Finding, Severity, SkillRisk};
use skillspec::boundary::style::{self, Style};
use skillspec::{boundary, error::Result, report};
use skillspec_source::remote::{self, RemoteSkillSource};
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

/// The risk verdict for a target, and whether it warrants a pause.
pub(super) struct Assessment {
    pub concerning: bool,
    pub summary: String,
}

/// Print the target's shape and its ranked risk under two headings — Structure,
/// then Security Analysis — and return the verdict.
pub(super) fn present(target: &str) -> Result<Assessment> {
    // The slow part is the clone and scan; spin while it runs. The map and the
    // analysis are computed first, then printed, so the spinner is not woven
    // through the output. An early `?` drops the spinner, which clears its line.
    let spinner =
        super::spinner::Spinner::start(format!("Analyzing {target} — cloning and scanning…"));
    let map = boundary::surface_map(target)?;
    let analysis = boundary::analyze_any(target)?;
    spinner.stop();

    let color = style::colors_enabled();
    report::text(&heading("Structure — what the package contains", color))?;
    report::text(&boundary::map::render(&map))?;

    let source = resolve_source(target);
    let assessment = summarize(&analysis, &source);
    report::text(&format!(
        "\n{}\n{}\n",
        heading(
            "Security analysis — what it could reach, ranked by risk",
            color
        ),
        assessment.summary
    ))?;
    Ok(assessment)
}

/// Run only the security analysis (the ranked risk view), as a standalone
/// command. `--json` emits the structured findings instead of the tree.
pub(super) fn security(target: &str, json: bool) -> Result<()> {
    let spinner =
        super::spinner::Spinner::start(format!("Analyzing {target} — cloning and scanning…"));
    let analysis = boundary::analyze_any(target)?;
    spinner.stop();

    if json {
        return report::json(&security_json(target, &analysis));
    }
    let source = resolve_source(target);
    let assessment = summarize(&analysis, &source);
    report::text(&format!("{}\n", assessment.summary))
}

/// A bold section title over a rule the width of the title.
fn heading(title: &str, color: bool) -> String {
    let rule = "─".repeat(title.chars().count());
    format!(
        "{}\n{}",
        style::paint(title, Style::Heading, color),
        style::paint(&rule, Style::Muted, color)
    )
}

/// Each analyzed skill paired with its ranked risk: one entry for a single
/// skill (empty package), one per package for a workspace.
fn scored_risks(analysis: &boundary::Analysis) -> Vec<(String, SkillRisk)> {
    match analysis {
        boundary::Analysis::Single(surface) => vec![(String::new(), SkillRisk::of(surface))],
        boundary::Analysis::Workspace(workspace) => workspace
            .packages
            .iter()
            .map(|entry| (entry.package.clone(), SkillRisk::of(&entry.surface)))
            .collect(),
    }
}

/// The machine-readable security report: per-skill severity and findings, plus
/// the severity tally.
fn security_json(target: &str, analysis: &boundary::Analysis) -> serde_json::Value {
    let scored = scored_risks(analysis);
    let mut counts = [0usize; 5]; // critical, high, medium, low, clean
    for (_, risk) in &scored {
        let bucket = match risk.severity() {
            Some(Severity::Critical) => 0,
            Some(Severity::High) => 1,
            Some(Severity::Medium) => 2,
            Some(Severity::Low) => 3,
            None => 4,
        };
        counts[bucket] += 1;
    }
    let skills: Vec<serde_json::Value> = scored
        .into_iter()
        .map(|(package, risk)| {
            serde_json::json!({
                "package": package,
                "severity": risk.severity(),
                "warrants_review": risk.warrants_review(),
                "findings": risk.findings,
            })
        })
        .collect();
    serde_json::json!({
        "schema": "skillspec.boundary.security.v0",
        "target": target,
        "summary": {
            "critical": counts[0],
            "high": counts[1],
            "medium": counts[2],
            "low": counts[3],
            "clean": counts[4],
        },
        "skills": skills,
    })
}

/// What the user decided, or what a non-interactive run forced.
pub(super) enum Outcome {
    /// Clean, or approved: proceed.
    Approved,
    /// The user declined at the prompt.
    Declined,
    /// Findings, but no terminal to confirm and no `--yes`.
    Refused,
}

/// Decide whether to proceed. A skill confined to its own directory (or clean)
/// is approved without a prompt; one that reaches beyond it needs `--yes` or an
/// interactive `y`. This never exits — the caller maps the outcome.
pub(super) fn confirm(assessment: &Assessment, assume_yes: bool) -> Result<Outcome> {
    if !assessment.concerning || assume_yes {
        return Ok(Outcome::Approved);
    }
    if !std::io::stdin().is_terminal() {
        return Ok(Outcome::Refused);
    }
    if prompt_proceed()? {
        Ok(Outcome::Approved)
    } else {
        Ok(Outcome::Declined)
    }
}

/// Where the target came from, resolved once so every locus links the same way.
enum Source {
    /// On disk: the absolute root, so a locus is an openable path.
    Local(PathBuf),
    /// A remote repository, so a locus is a blob URL for the host.
    Remote(RemoteSkillSource),
}

fn resolve_source(target: &str) -> Source {
    if Path::new(target).exists() {
        let root = std::fs::canonicalize(target).unwrap_or_else(|_| PathBuf::from(target));
        return Source::Local(root);
    }
    match remote::parse_target(target) {
        Ok(Some(remote)) => Source::Remote(remote),
        _ => Source::Local(PathBuf::from(target)),
    }
}

/// An openable evidence link for a finding: an absolute `path:line` on disk, or
/// a blob URL for a remote repo. The reached target is a separate column, so it
/// is not appended here.
fn evidence_link(source: &Source, package_prefix: &str, finding: &Finding) -> String {
    let Some(file) = &finding.file else {
        return String::new();
    };
    match source {
        Source::Local(root) => {
            let mut path = root.clone();
            if !package_prefix.is_empty() {
                path.push(package_prefix);
            }
            path.push(file);
            match finding.line {
                Some(line) => format!("{}:{}", path.display(), line),
                None => path.display().to_string(),
            }
        }
        Source::Remote(remote) => {
            let mut rel = String::new();
            if let Some(subpath) = &remote.path {
                rel.push_str(subpath.trim_matches('/'));
                rel.push('/');
            }
            if !package_prefix.is_empty() {
                rel.push_str(package_prefix);
                rel.push('/');
            }
            rel.push_str(file);
            remote::web_url(remote, &rel, finding.line)
        }
    }
}

fn summarize(analysis: &boundary::Analysis, source: &Source) -> Assessment {
    use std::fmt::Write as _;
    use termtree::Tree;

    match analysis {
        boundary::Analysis::Single(surface) => {
            let color = style::colors_enabled();
            let risk = SkillRisk::of(surface);
            let concerning = risk.warrants_review();
            let label = match risk.severity() {
                Some(severity) => format!("Risk: {}", severity_tag(severity, color)),
                None => style::paint(
                    "Risk: none found — this skill stays within its own directory and reaches nothing outside it.",
                    Style::Ok,
                    color,
                ),
            };
            let mut root = Tree::new(label);
            for finding in &risk.findings {
                root.push(finding_tree(
                    finding,
                    &evidence_link(source, "", finding),
                    color,
                ));
            }
            let mut summary = String::new();
            let _ = write!(summary, "{root}");
            Assessment {
                concerning,
                summary,
            }
        }
        boundary::Analysis::Workspace(workspace) => {
            let mut scored: Vec<(String, SkillRisk)> = workspace
                .packages
                .iter()
                .map(|entry| (entry.package.clone(), SkillRisk::of(&entry.surface)))
                .collect();
            scored.sort_by_key(|(_, risk)| std::cmp::Reverse(risk.severity()));
            summarize_workspace(&scored, source)
        }
    }
}

fn summarize_workspace(scored: &[(String, SkillRisk)], source: &Source) -> Assessment {
    use std::fmt::Write as _;
    use termtree::Tree;

    let color = style::colors_enabled();
    let reviewable: Vec<&(String, SkillRisk)> = scored
        .iter()
        .filter(|(_, risk)| risk.warrants_review())
        .collect();

    if reviewable.is_empty() {
        let summary = format!(
            "Risk across {} skills — {}\n{}",
            scored.len(),
            tally(scored, color),
            style::paint(
                "Nothing reaches outside its own directory. Nothing warrants review before install.",
                Style::Ok,
                color,
            )
        );
        return Assessment {
            concerning: false,
            summary,
        };
    }

    let root_label = format!(
        "Risk across {} skills — {}\n{} skill(s) reach beyond their own directory — review before installing",
        scored.len(),
        tally(scored, color),
        reviewable.len()
    );
    let mut root = Tree::new(root_label);

    const SHOWN: usize = 6;
    for (package, risk) in &reviewable {
        let severity = risk
            .severity()
            .map(|severity| severity_tag(severity, color))
            .unwrap_or_default();
        let mut skill_node = Tree::new(format!("{severity}  {package}"));
        let findings: Vec<&Finding> = risk
            .findings
            .iter()
            .filter(|finding| finding.severity >= Severity::Medium)
            .collect();
        for finding in findings.iter().take(SHOWN) {
            skill_node.push(finding_tree(
                finding,
                &evidence_link(source, package, finding),
                color,
            ));
        }
        if findings.len() > SHOWN {
            skill_node.push(Tree::new(format!(
                "… and {} more — run `skillspec boundary {package}` for the full report",
                findings.len() - SHOWN
            )));
        }
        root.push(skill_node);
    }

    let mut summary = String::new();
    let _ = write!(summary, "{root}");

    // The skills that need no review, shown with structure so the section reads
    // as "checked and fine" rather than a wall of names: clean skills (which
    // touch nothing outside their directory) are only counted, and the few that
    // are low risk (they touch their own files) are named.
    if let Some(cleared) = cleared_tree(scored, color) {
        let _ = write!(summary, "\n\n{cleared}");
    }

    Assessment {
        concerning: true,
        summary,
    }
}

/// A small tree of the skills that did not warrant review, split by why.
fn cleared_tree(scored: &[(String, SkillRisk)], color: bool) -> Option<termtree::Tree<String>> {
    use termtree::Tree;

    let low: Vec<&str> = scored
        .iter()
        .filter(|(_, risk)| risk.severity() == Some(Severity::Low))
        .map(|(package, _)| package.as_str())
        .collect();
    let clean = scored
        .iter()
        .filter(|(_, risk)| risk.severity().is_none())
        .count();
    if low.is_empty() && clean == 0 {
        return None;
    }

    let mut root = Tree::new(style::paint(
        "Cleared — no review needed",
        Style::Heading,
        color,
    ));
    if clean > 0 {
        root.push(Tree::new(format!(
            "{clean} clean · reach nothing outside their own directory"
        )));
    }
    if !low.is_empty() {
        let mut node = Tree::new(format!("{} low · touch only their own files", low.len()));
        // Name them when there are few; past that a count is clearer than a wall.
        const NAMED: usize = 15;
        if low.len() <= NAMED {
            for package in low {
                node.push(Tree::new(package.to_owned()));
            }
        }
        root.push(node);
    }
    Some(root)
}

/// A finding as a tree node: the headline, what it reaches, and the consequence
/// on the node line, with the evidence link as a leaf beneath it — so the link
/// stays attached to its finding and sits alone on its own line (clickable, and
/// never fractured by wrapping).
fn finding_tree(finding: &Finding, link: &str, color: bool) -> termtree::Tree<String> {
    use std::fmt::Write as _;
    use termtree::Tree;
    let mut label = format!(
        "[{}] {}",
        severity_tag(finding.severity, color),
        finding.headline
    );
    if let Some(reached) = &finding.reached {
        let _ = write!(label, " → {reached}");
    }
    if !finding.consequence.is_empty() {
        let _ = write!(
            label,
            "  ·  {}",
            style::paint(&finding.consequence, Style::Muted, color)
        );
    }
    if let Some(note) = &finding.note {
        let _ = write!(
            label,
            "  {}",
            style::paint(&format!("[{note}]"), Style::Muted, color)
        );
    }
    let mut node = Tree::new(label);
    if !link.is_empty() {
        node.push(Tree::new(hyperlink(link)));
    }
    node
}

/// A clickable rendering of an evidence location. A remote URL is wrapped in an
/// OSC 8 terminal hyperlink when stdout is a terminal, so it is clickable even
/// though the visible text is the full URL; a local path is left as-is, since
/// terminals already resolve `path:line`. When output is not a terminal (a pipe
/// or a file) the bare text is emitted, never an escape sequence.
fn hyperlink(target: &str) -> String {
    let is_url = target.starts_with("http://") || target.starts_with("https://");
    if is_url && std::io::stdout().is_terminal() {
        format!("\u{1b}]8;;{target}\u{1b}\\{target}\u{1b}]8;;\u{1b}\\")
    } else {
        target.to_owned()
    }
}

/// The severity word, colored by how much it should weigh on the decision.
fn severity_tag(severity: Severity, color: bool) -> String {
    style::paint(severity.label(), severity_style(severity), color)
}

fn severity_style(severity: Severity) -> Style {
    match severity {
        Severity::Critical => Style::Alarm,
        Severity::High => Style::Danger,
        Severity::Medium => Style::Warn,
        Severity::Low => Style::Info,
    }
}

fn tally(scored: &[(String, SkillRisk)], color: bool) -> String {
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    let mut low = 0;
    let mut clean = 0;
    for (_, risk) in scored {
        match risk.severity() {
            Some(Severity::Critical) => critical += 1,
            Some(Severity::High) => high += 1,
            Some(Severity::Medium) => medium += 1,
            Some(Severity::Low) => low += 1,
            None => clean += 1,
        }
    }
    // Color a count only when it is non-zero, so a clean run stays quiet.
    let paint = |count: usize, word: &str, style: Style| {
        let text = format!("{count} {word}");
        if count > 0 {
            style::paint(&text, style, color)
        } else {
            text
        }
    };
    format!(
        "{} · {} · {} · {} · {}",
        paint(critical, "critical", Style::Alarm),
        paint(high, "high", Style::Danger),
        paint(medium, "medium", Style::Warn),
        paint(low, "low", Style::Info),
        paint(clean, "clean", Style::Ok),
    )
}

fn prompt_proceed() -> Result<bool> {
    print!("\nProceed with install? [y/N] ");
    std::io::stdout()
        .flush()
        .map_err(skillspec::error::Error::Output)?;
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(skillspec::error::Error::Output)?;
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
