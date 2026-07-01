use crate::support::*;

#[test]
fn doctor_reports_prose_skill_context_and_reliability_debt(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-prose");
    let skill_dir = dir.path().join("source-skill");
    let mut skill = String::from(
        r#"---
name: dense-prose
description: Use when a dense prose skill mixes instructions, snippets, and dependency assumptions.
---

# Dense Prose Skill

Use the shell and Python to inspect the project, fetch external data, create a report, and install missing packages when needed.
See [missing local reference](missing.md).

```
pip install pypdf
```

```python
import pypdf
from reportlab.pdfgen import canvas
```

"#,
    );
    for index in 1..=520 {
        skill.push_str(&format!(
            "{index}. Always run verification step {index} before continuing.\n"
        ));
    }
    skill.push_str("\nNever skip the final proof summary.\n");
    write_file(&skill_dir.join("SKILL.md"), &skill);

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&skill_dir)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert!(report["structural_score"]
        .as_u64()
        .is_some_and(|score| score < 40));
    assert!(report["large_surface_percentage"]
        .as_u64()
        .is_some_and(|percentage| percentage >= 90));
    assert_eq!(
        report["frontmatter_discovery_risk"]["fields"]["name"].as_str(),
        Some("dense-prose")
    );
    assert!(report["agent_drift_risk"]["score"]
        .as_u64()
        .is_some_and(|score| score > 60));
    assert!(report["agent_drift_risk"]["conditions"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing agent drift conditions"))?
        .iter()
        .any(|condition| condition["kind"] == "context_pressure"));
    assert_eq!(report["counts"]["unlabeled_code_blocks_in_skill"], 1);
    assert!(report["counts"]["numbered_steps"]
        .as_u64()
        .is_some_and(|steps| steps >= 520));

    let issues_text = serde_json::to_string(&report["issues"])?;
    assert!(issues_text.contains("large_activation_body"));
    assert!(issues_text.contains("primacy_bias_late_obligations"));
    assert!(issues_text.contains("code_mixed_with_activation_instructions"));
    assert!(issues_text.contains("unlabeled_code_fences"));
    assert!(issues_text.contains("implicit_dependency_contract"));
    assert!(issues_text.contains("ambiguous_execution_substrate"));
    assert!(issues_text.contains("missing_behavior_contract"));
    assert!(issues_text.contains("missing_trace_proof_surface"));
    assert!(issues_text.contains("missing_referenced_files"));
    assert_eq!(
        report["score_model"]["primary_score_label"].as_str(),
        Some("agent_follow_through_risk")
    );
    assert_eq!(
        report["score_model"]["risk_direction"].as_str(),
        Some("higher_score_means_higher_risk")
    );
    assert!(report["score_model"]["not_measuring"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));

    let text = Command::new(bin()).arg("doctor").arg(&skill_dir).output()?;
    assert_success(&text);
    let text = stdout(&text);
    assert!(text.contains("Activation-loaded surface:"));
    assert!(text.contains("What This Measures"));
    assert!(text.contains("Agent follow-through risk:"));
    assert!(text.contains("Discovery risk:"));
    assert!(text.contains("docs/00-skills-reliability-gap.md"));
    assert!(text.contains("docs/08-contract-trace-methodology.md"));
    Ok(())
}

#[test]
fn doctor_reports_contract_mitigation_for_skillspec_backed_skill(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-contract-mitigation");
    let skill_dir = dir.path().join("source-skill");
    write_file(
        &skill_dir.join("SKILL.md"),
        r#"---
name: backed-skill
description: Use when a SkillSpec-backed fixture is needed.
---

# Backed Skill

Use Python, shell, and the local CLI to inspect files and produce proof.

```python
import pypdf
```
"#,
    );
    write_file(&skill_dir.join("skill.spec.yml"), rich_spec());

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&skill_dir)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["contract_mitigation"]["present"], true);
    assert_eq!(report["contract_mitigation"]["level"], "strong");
    assert_eq!(report["contract_mitigation"]["dependencies"], 1);
    assert_eq!(
        report["agent_drift_risk"]["recommended_mode"],
        "thin_trampoline_and_use_guided_cli"
    );
    let issues_text = serde_json::to_string(&report["issues"])?;
    assert!(
        !issues_text.contains("implicit_dependency_contract"),
        "doctor should not require deps.toml when valid skill.spec.yml dependencies exist: {issues_text}"
    );

    let text = Command::new(bin()).arg("doctor").arg(&skill_dir).output()?;
    assert_success(&text);
    let text = stdout(&text);
    assert!(text.contains("Contract mitigation: strong"));
    assert!(text.contains("Residual risk:"));
    assert!(text.contains("Recommended next action: skillspec install skill"));
    assert!(!text.contains("Recommended next action: skillspec doctor"));
    assert!(text.contains("skillspec run-loop --guide agent"));
    Ok(())
}

#[test]
fn doctor_reports_frontmatter_discovery_risk() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let dir = TempDir::new("doctor-frontmatter-risk");
    let skill_dir = dir.path().join("source-skill");
    write_file(
        &skill_dir.join("SKILL.md"),
        "---\nname: vague\ndescription: Helper.\n---\n# Vague\n\nDo useful work.\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&skill_dir)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["analysis_status"], "full");
    assert!(report["frontmatter_discovery_risk"]["score"]
        .as_u64()
        .is_some_and(|score| score > 0));
    let conditions = serde_json::to_string(&report["frontmatter_discovery_risk"]["conditions"])?;
    assert!(conditions.contains("ambiguous_short_description"));
    assert!(conditions.contains("claude_skill_frontmatter_discovery"));
    Ok(())
}

#[test]
fn doctor_reports_malformed_frontmatter_discovery_risk(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-bad-frontmatter");
    let skill_dir = dir.path().join("source-skill");
    write_file(
        &skill_dir.join("SKILL.md"),
        "---\nname: bad\ndescription: Bad: unquoted colon\n---\n# Bad\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&skill_dir)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(
        report["frontmatter_discovery_risk"]["fields"]["parse_status"].as_str(),
        Some("invalid_yaml")
    );
    let issues = serde_json::to_string(&report["issues"])?;
    assert!(issues.contains("missing_or_malformed_frontmatter"));
    Ok(())
}

#[test]
fn doctor_reports_parent_folder_with_multiple_skills_as_workspace(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-multi");
    let root = dir.path().join("skills");
    write_file(
        &root.join("pdf").join("SKILL.md"),
        "---\nname: pdf\ndescription: PDF skill.\n---\n# PDF\n",
    );
    write_file(
        &root.join("csv").join("SKILL.md"),
        "---\nname: csv\ndescription: CSV skill.\n---\n# CSV\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["analysis_status"], "workspace");
    assert_eq!(report["shape"]["kind"], "multi_skill_workspace");
    assert_eq!(
        report["shape"]["skill_files"]
            .as_array()
            .ok_or_else(|| invalid_json_shape("missing skill files"))?
            .len(),
        2
    );
    assert_eq!(
        report["packages"]
            .as_array()
            .ok_or_else(|| invalid_json_shape("missing packages"))?
            .len(),
        2
    );
    assert!(report["workspace_agent_drift_risk"]["score"]
        .as_u64()
        .is_some());
    assert!(report["packages"][0]["frontmatter_discovery_risk"]["score"]
        .as_u64()
        .is_some());
    assert!(report["shape"]["recommended_command"]
        .as_str()
        .is_some_and(|command| command.contains("workspace map")));
    Ok(())
}

#[test]
fn doctor_ignores_hidden_harness_skill_roots_when_classifying_shape(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-hidden-harness");
    let root = dir.path().join("repo");
    write_file(
        &root.join("SKILL.md"),
        "---\nname: public-skill\ndescription: Public skill.\n---\n# Public\n",
    );
    write_file(
        &root
            .join(".claude")
            .join("skills")
            .join("private")
            .join("SKILL.md"),
        "---\nname: private\ndescription: Hidden harness state.\n---\n# Private\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["analysis_status"], "full");
    assert_eq!(report["shape"]["kind"], "simple_skill");
    assert_eq!(
        report["shape"]["skill_files"]
            .as_array()
            .ok_or_else(|| invalid_json_shape("missing skill files"))?
            .len(),
        1
    );
    Ok(())
}

#[test]
fn doctor_detects_entry_skill_with_subskills() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let dir = TempDir::new("doctor-entry-subskills");
    let root = dir.path().join("skills-repo");
    write_file(
        &root.join("SKILL.md"),
        r#"---
name: parent
description: Parent skill.
---
# Parent

Use `./legal-review/SKILL.md` and `/contract-review` when those workflows apply.
"#,
    );
    write_file(
        &root.join("legal-review").join("SKILL.md"),
        "---\nname: legal-review\ndescription: Legal review.\n---\n# Legal\n",
    );
    write_file(
        &root.join("contract-review").join("SKILL.md"),
        "---\nname: contract-review\ndescription: Contract review.\n---\n# Contract\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["analysis_status"], "workspace");
    assert_eq!(report["shape"]["kind"], "entry_skill_with_subskills");
    assert_eq!(report["shape"]["primary_skill"], "SKILL.md");
    assert_eq!(
        report["packages"]
            .as_array()
            .ok_or_else(|| invalid_json_shape("missing packages"))?
            .len(),
        3
    );
    let workspace_issues = serde_json::to_string(&report["issues"])?;
    assert!(workspace_issues.contains("workspace_cross_skill_reference_risk"));
    let referenced = report["shape"]["referenced_skill_paths"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing referenced skill paths"))?;
    assert!(referenced
        .iter()
        .any(|path| path.as_str() == Some("legal-review")));
    assert!(referenced
        .iter()
        .any(|path| path.as_str() == Some("contract-review")));
    Ok(())
}

#[test]
fn doctor_detects_plugin_workspace_shape() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-plugin-shape");
    let root = dir.path().join("claude-for-legal");
    write_file(
        &root
            .join("commercial")
            .join(".claude-plugin")
            .join("plugin.json"),
        r#"{"name":"commercial-legal","version":"1.0.0"}"#,
    );
    write_file(
        &root
            .join("commercial")
            .join("skills")
            .join("review")
            .join("SKILL.md"),
        "---\nname: review\ndescription: Review.\n---\n# Review\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["analysis_status"], "workspace");
    assert_eq!(report["shape"]["kind"], "plugin_workspace");
    let plugins = report["shape"]["plugin_roots"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing plugin roots"))?;
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0]["namespace"], "commercial-legal");
    assert_eq!(plugins[0]["path"], "commercial");
    assert_eq!(
        report["packages"]
            .as_array()
            .ok_or_else(|| invalid_json_shape("missing packages"))?
            .len(),
        1
    );
    assert_eq!(
        report["packages"][0]["plugin_name"].as_str(),
        Some("commercial-legal")
    );
    assert_eq!(
        report["packages"][0]["shape_role"].as_str(),
        Some("plugin_skill")
    );
    Ok(())
}

#[test]
fn doctor_detects_generic_plugin_metadata_shape(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-generic-plugin-shape");
    let root = dir.path().join("skills-marketplace");
    write_file(
        &root.join(".agent-plugin").join("marketplace.json"),
        r#"{"name":"neutral-marketplace","version":"1.0.0"}"#,
    );
    write_file(
        &root.join("skills").join("triage").join("SKILL.md"),
        "---\nname: triage\ndescription: Triage.\n---\n# Triage\n",
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["shape"]["kind"], "plugin_workspace");
    let plugins = report["shape"]["plugin_roots"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing plugin roots"))?;
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0]["namespace"], "neutral-marketplace");
    assert_eq!(plugins[0]["path"], "");
    assert_eq!(
        report["packages"][0]["plugin_name"].as_str(),
        Some("neutral-marketplace")
    );
    assert_eq!(
        report["packages"][0]["shape_role"].as_str(),
        Some("plugin_skill")
    );
    Ok(())
}

#[test]
fn doctor_reports_repeated_workspace_skill_content_as_referentiable_identity(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-repeated-content");
    let root = dir.path().join("posthog-skills");
    write_file(
        &root.join(".agent-plugin").join("marketplace.json"),
        r#"{"name":"posthog-skills","version":"1.0.0"}"#,
    );
    let repeated = r#"---
name: integration-python
description: PostHog integration for any Python application using the Python SDK
---
# Integration Python

Use Python SDK.
"#;
    write_file(
        &root
            .join("skills")
            .join("posthog")
            .join("all")
            .join("skills")
            .join("integration-python")
            .join("SKILL.md"),
        repeated,
    );
    write_file(
        &root
            .join("skills")
            .join("posthog")
            .join("integration")
            .join("skills")
            .join("python")
            .join("SKILL.md"),
        repeated,
    );
    write_file(
        &root
            .join("skills")
            .join("posthog")
            .join("integration")
            .join("skills")
            .join("node")
            .join("SKILL.md"),
        r#"---
name: integration-node
description: PostHog integration for Node.
---
# Integration Node
"#,
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["analysis_status"], "workspace");
    assert_eq!(report["shape"]["kind"], "plugin_workspace");
    assert_eq!(report["workspace_identity"]["skill_file_count"], 3);
    assert_eq!(report["workspace_identity"]["namespaced_package_count"], 3);
    assert_eq!(
        report["workspace_identity"]["unique_skill_content_count"],
        2
    );
    assert_eq!(
        report["workspace_identity"]["repeated_skill_content_groups"],
        1
    );
    assert_eq!(
        report["workspace_identity"]["repeated_skill_content_occurrences"],
        1
    );
    assert_eq!(
        report["workspace_identity"]["same_frontmatter_name_groups"],
        1
    );
    assert!(report["workspace_identity"]["source_file_count"]
        .as_u64()
        .is_some_and(|count| count >= 4));
    let refs = report["workspace_identity"]["source_content_refs"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing source_content_refs"))?;
    let repeated_ref = refs
        .iter()
        .find(|item| item["occurrence_count"] == 2)
        .ok_or_else(|| invalid_json_shape("missing repeated source content ref"))?;
    assert_eq!(
        repeated_ref["aliases"]
            .as_array()
            .ok_or_else(|| invalid_json_shape("missing repeated aliases"))?
            .len(),
        1
    );
    assert!(repeated_ref["canonical_path"]
        .as_str()
        .is_some_and(|path| path.contains("integration-python")));
    assert!(report["workspace_identity"]["recommendation"]
        .as_str()
        .is_some_and(|text| text.contains("source_content_ref")));
    let issues_text = serde_json::to_string(&report["issues"])?;
    assert!(issues_text.contains("workspace_repeated_skill_content"));
    assert!(issues_text.contains("workspace_reused_frontmatter_names"));

    let text = Command::new(bin()).arg("doctor").arg(&root).output()?;
    assert_success(&text);
    let text = stdout(&text);
    assert!(text.contains("Shape Contract"));
    assert!(text.contains("Kind: plugin_workspace"));
    assert!(text.contains("Packages: 3"));
    assert!(text.contains("Namespaces: 1"));
    assert!(text.contains("Plugin roots: 1"));
    assert!(text.contains("Next command: skillspec workspace map"));
    assert!(text.contains("Workspace Identity"));
    assert!(text.contains("unique byte content"));
    assert!(text.contains("referentiable"));
    assert!(text.contains("source_content_ref"));
    assert!(text
        .contains("Assessment scope: plugin workspace plus full per-package raw skill profiles"));
    assert!(text.contains("Risk interpretation: Plugin workspace risk is the maximum"));
    assert!(text.contains("Package risk rollup:"));
    Ok(())
}

#[test]
fn doctor_workspace_rolls_up_full_package_risk_profiles(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-workspace-package-risk");
    let root = dir.path().join("posthog-skills");
    let package = root
        .join("skills")
        .join("posthog")
        .join("tools-and-features")
        .join("skills")
        .join("hogql");
    write_file(
        &root.join(".agent-plugin").join("marketplace.json"),
        r#"{"name":"posthog-skills","version":"1.0.0"}"#,
    );
    write_file(
        &package.join("SKILL.md"),
        r#"---
name: hogql
description: HogQL queries for PostHog analytics.
---
# HogQL

Use Python, shell, and API calls to inspect analytics, run queries, fetch results, and write a final report.
See [missing reference](missing.md) before using query examples.

```python
import posthog
```

```
curl https://app.posthog.com/api/projects/
```
"#,
    );
    write_file(
        &root
            .join("skills")
            .join("posthog")
            .join("survey")
            .join("skills")
            .join("creator")
            .join("SKILL.md"),
        r#"---
name: survey-creator
description: Create PostHog surveys.
---
# Survey
"#,
    );

    let standalone_output = Command::new(bin())
        .arg("doctor")
        .arg(&package)
        .arg("--json")
        .output()?;
    assert_success(&standalone_output);
    let standalone = json_stdout(&standalone_output);
    let standalone_score = standalone["agent_drift_risk"]["score"]
        .as_u64()
        .ok_or_else(|| invalid_json_shape("missing standalone score"))?;
    assert!(
        standalone_score >= 75,
        "fixture should exercise critical standalone risk, got {standalone_score}"
    );

    let workspace_output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--json")
        .output()?;
    assert_success(&workspace_output);
    let workspace = json_stdout(&workspace_output);
    assert_eq!(workspace["shape"]["kind"], "plugin_workspace");

    let packages = workspace["packages"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing workspace packages"))?;
    let hogql = packages
        .iter()
        .find(|package| {
            package["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("hogql/SKILL.md"))
        })
        .ok_or_else(|| invalid_json_shape("missing hogql package profile"))?;
    assert_eq!(
        hogql["agent_drift_risk"]["score"].as_u64(),
        Some(standalone_score)
    );
    assert_eq!(
        hogql["agent_drift_risk"]["level"],
        standalone["agent_drift_risk"]["level"]
    );
    assert_eq!(
        hogql["risk_profile_source"].as_str(),
        Some("full_package_analysis")
    );
    assert_eq!(
        hogql["source_content_sha256"]
            .as_str()
            .ok_or_else(|| invalid_json_shape("missing source content hash"))?
            .len(),
        64
    );
    assert!(workspace["workspace_agent_drift_risk"]["score"]
        .as_u64()
        .is_some_and(|score| score >= standalone_score));
    let conditions = workspace["workspace_agent_drift_risk"]["conditions"]
        .as_array()
        .ok_or_else(|| invalid_json_shape("missing workspace risk conditions"))?;
    let rollup = conditions
        .iter()
        .find(|condition| condition["id"] == "workspace_package_risk_rollup")
        .ok_or_else(|| invalid_json_shape("missing package risk rollup"))?;
    assert_eq!(rollup["claim_scope"], "full_package_risk_rollup");
    assert!(rollup["measurement"]["critical_package_count"]
        .as_u64()
        .is_some_and(|count| count >= 1));
    assert!(rollup["measurement"]["top_packages"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    Ok(())
}

#[test]
fn doctor_default_output_is_formatted_user_report(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-human-report");
    let root = dir.path().join("review-skill");
    write_file(
        &root.join("SKILL.md"),
        r#"---
name: review-skill
description: Review one file and report risks.
---
# Review Skill

1. Read the requested file.
2. Report risks.
"#,
    );

    let output = Command::new(bin()).arg("doctor").arg(&root).output()?;
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SkillSpec Doctor"));
    assert!(stdout.contains("What This Measures"));
    assert!(stdout.contains("Assessment scope: one atomic SKILL.md package"));
    assert!(stdout.contains("Risk interpretation: A high score means this raw package"));
    assert!(stdout.contains("Current Skill Baseline"));
    assert!(stdout.contains("Agent follow-through risk:"));
    assert!(stdout.contains("Shape Contract"));
    assert!(stdout.contains("Kind: simple_skill"));
    assert!(stdout.contains("Skill files: 1"));
    assert!(stdout.contains("Packages: 1"));
    assert!(stdout.contains("Next command: /skillspec import"));
    assert!(stdout.contains("Surface"));
    assert!(stdout.contains("Findings"));
    assert!(stdout.contains("Next Actions"));
    assert!(stdout.contains("Recommended next action: /skillspec import"));
    assert!(!stdout.contains("Structural score:"));
    assert!(!stdout.contains("shape_kind:"));
    assert!(!stdout.contains("analysis_status:"));
    assert!(!stdout.contains("Recommended next action: skillspec doctor"));
    Ok(())
}

#[test]
fn doctor_html_output_is_self_contained_report(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-html-report");
    let root = dir.path().join("review-skill");
    write_file(
        &root.join("SKILL.md"),
        r#"---
name: review-skill
description: Review one file and report risks.
---
# Review Skill

1. Read the requested file.
2. Report risks.
"#,
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--html")
        .output()?;
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!doctype html>"));
    assert!(stdout.contains("<title>SkillSpec Doctor Report</title>"));
    assert!(stdout.contains("class=\"hero\""));
    assert!(stdout.contains("What This Measures"));
    assert!(stdout.contains("Assessment scope"));
    assert!(stdout.contains("Risk interpretation"));
    assert!(stdout.contains("Agent follow-through risk"));
    assert!(stdout.contains("Shape Contract"));
    assert!(stdout.contains("Next Actions"));
    assert!(stdout.contains("Research Basis"));
    assert!(stdout.contains("https://github.com/modiqo/skillspec/blob/main/docs/"));
    assert!(!stdout.contains("href=\"docs/"));
    Ok(())
}

#[test]
fn doctor_markdown_output_is_renderable_report(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-markdown-report");
    let root = dir.path().join("review-skill");
    write_file(
        &root.join("SKILL.md"),
        r#"---
name: review-skill
description: Review one file and report risks.
---
# Review Skill

1. Read the requested file.
2. Report risks.
"#,
    );

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--markdown")
        .output()?;
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# SkillSpec Doctor report"));
    assert!(stdout.contains("## What This Measures"));
    assert!(stdout.contains("**Assessment scope:**"));
    assert!(stdout.contains("**Risk interpretation:**"));
    assert!(stdout.contains("## Current Skill Baseline"));
    assert!(stdout.contains("**Agent follow-through risk:**"));
    assert!(stdout.contains("## Shape Contract"));
    assert!(stdout.contains("- **Kind:** `simple_skill`"));
    assert!(stdout.contains("**Next command**"));
    assert!(stdout.contains("## Surface"));
    assert!(stdout.contains("## Findings"));
    assert!(stdout.contains("## Next Actions"));
    assert!(stdout.contains("## Research Basis"));
    assert!(stdout.contains("**Recommended next action**"));
    assert!(!stdout.contains("**Structural score:**"));
    assert!(!stdout.contains("```text\nSkillSpec Doctor"));
    Ok(())
}

#[test]
fn doctor_reports_non_skill_repository_shape_without_source_mapping(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("doctor-code-repo");
    let root = dir.path().join("code-repo");
    write_file(
        &root.join("Cargo.toml"),
        "[package]\nname = \"not-a-skill\"\n",
    );
    write_file(&root.join("src").join("main.rs"), "fn main() {}\n");

    let output = Command::new(bin())
        .arg("doctor")
        .arg(&root)
        .arg("--json")
        .output()?;
    assert_success(&output);
    let report = json_stdout(&output);
    assert_eq!(report["analysis_status"], "shape_only");
    assert_eq!(report["shape"]["kind"], "non_skill_repository");
    assert_eq!(report["counts"]["code_files"], 1);
    assert_eq!(report["counts"]["manifest_files"], 1);
    let issues = serde_json::to_string(&report["issues"])?;
    assert!(issues.contains("no_skill_entrypoint"));
    Ok(())
}
