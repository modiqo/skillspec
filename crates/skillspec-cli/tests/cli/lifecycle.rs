use crate::support::*;

#[test]
fn skill_router_indexes_routes_and_audits_local_skills() {
    let dir = TempDir::new("skill-router");
    let root = dir.path().join("skills");
    let index = dir.path().join("skill-index.sqlite");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when the user needs to read, extract, OCR, merge, split, or transform PDF documents. Do not use for ordinary markdown files.
metadata:
  short-description: PDF extraction and transformation.
  routing:
    tags: [documents, extraction]
    triggers:
      - extract PDF text
      - OCR scanned PDF
    negative_triggers:
      - markdown
---
# PDF
"#,
    );
    write_file(
        &root.join("pdf/agents/openai.yaml"),
        r#"policy:
  allow_implicit_invocation: false
"#,
    );
    write_file(
        &root.join("pdf/skill.spec.yml"),
        r#"
schema: skillspec/v0
id: router.pdf
title: PDF Router Fixture
description: SkillSpec metadata for PDF routing.
activation:
  summary: Extract tables and text from PDFs.
  keywords: [pdf tables, pdf text]
routes:
  - id: extract
    label: Extract
rules:
  - id: avoid_markdown
    forbid: [markdown]
    reason: Markdown is not a PDF workflow.
tests:
  - name: route assertion
    input: extract pdf text
    expect:
      route: extract
"#,
    );
    write_file(
        &root.join("deploy/SKILL.md"),
        r#"---
name: deploy
description: Use when publishing an application to production environments, release targets, or hosting platforms. Do not use for document extraction.
disable-model-invocation: true
metadata:
  routing:
    tags: [release, hosting]
    triggers: [deploy application]
---
# Deploy
"#,
    );
    write_file(
        &root.join("alternate-pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when annotating simple PDF files and adding comments. Do not use for OCR or table extraction.
metadata:
  routing:
    tags: [annotation]
    triggers: [annotate PDF]
---
# PDF Annotation
"#,
    );
    write_file(
        &root.join("notes/SKILL.md"),
        r#"---
name: notes
description: Helps with notes.
---
# Notes
"#,
    );

    let index_output = Command::new(bin())
        .arg("index")
        .arg("--roots")
        .arg(&root)
        .arg("--out")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&index_output);
    let index_report = json_stdout(&index_output);
    assert_eq!(index_report["skills_indexed"], 4);
    assert!(index.is_file());

    let directory_status = Command::new(bin())
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(dir.path())
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&directory_status);
    let directory_status_report = json_stdout(&directory_status);
    assert_eq!(
        directory_status_report["index"],
        index.to_string_lossy().as_ref()
    );
    assert_eq!(directory_status_report["exists"], true);
    assert_eq!(directory_status_report["stale"], false);

    let route = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(dir.path())
        .arg("--query")
        .arg("extract pdf text from a scanned document")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&route);
    let route_report = json_stdout(&route);
    assert_eq!(route_report["decision"], "use_skill");
    assert_eq!(route_report["bypass_reason"], Value::Null);
    assert_eq!(route_report["selected"]["name"], "pdf");
    assert_eq!(
        Path::new(route_report["selected"]["path"].as_str().unwrap())
            .strip_prefix(&root)
            .unwrap(),
        Path::new("pdf").join("SKILL.md")
    );
    assert_eq!(route_report["selected"]["visibility"], "manual-only");
    assert_eq!(route_report["selected"]["has_skill_spec"], true);
    assert_eq!(
        route_report["elicitation"],
        "execution_mode_direct_or_durable"
    );

    let direct_route = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("deploy application")
        .arg("--execution-mode")
        .arg("direct")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&direct_route);
    let direct_report = json_stdout(&direct_route);
    assert_eq!(direct_report["decision"], "use_skill");
    assert_eq!(direct_report["bypass_reason"], Value::Null);
    assert_eq!(direct_report["selected"]["name"], "deploy");
    assert_eq!(direct_report["execution_mode"], "direct");
    assert_eq!(direct_report["elicitation"], Value::Null);

    let audit = Command::new(bin())
        .arg("skills")
        .arg("audit")
        .arg("--roots")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&audit);
    let audit_report = json_stdout(&audit);
    assert_eq!(audit_report["skills"], 4);
    assert_eq!(audit_report["vague_descriptions"], 1);
    assert_eq!(audit_report["missing_negative_boundaries"], 1);
    assert!(audit_report["duplicate_names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "pdf"));
}

#[test]
fn router_policy_profile_can_select_skill_and_passthrough() {
    let dir = TempDir::new("router-policy");
    let root = dir.path().join("skills");
    let index = dir.path().join("router").join("skill-index.sqlite");

    write_file(
        &root.join("notes/SKILL.md"),
        r#"---
name: notes
description: Organize personal notes and summaries. Do not use for PDF extraction.
metadata:
  routing:
    tags: [knowledge]
---
# Notes
"#,
    );
    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Extract text and tables from PDF documents. Do not use for personal notes.
metadata:
  routing:
    tags: [documents]
---
# PDF
"#,
    );

    let index_output = Command::new(bin())
        .arg("index")
        .arg("--roots")
        .arg(&root)
        .arg("--out")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&index_output);

    let set_default = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("set-profile")
        .arg("default")
        .arg("--index")
        .arg(&index)
        .arg("--active")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&set_default);
    let set_default_report = json_stdout(&set_default);
    assert_eq!(set_default_report["active_profile"], "default");

    let set_rule = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("set-rule")
        .arg("hidden_knowledge")
        .arg("--index")
        .arg(&index)
        .arg("--profile")
        .arg("default")
        .arg("--priority")
        .arg("100")
        .arg("--mode")
        .arg("hard")
        .arg("--anchor")
        .arg("policy")
        .arg("--when-any")
        .arg("hidden workflow")
        .arg("--prefer")
        .arg("skill:notes")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&set_rule);

    let policy_route = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("please handle the hidden workflow")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&policy_route);
    let policy_route_report = json_stdout(&policy_route);
    assert_eq!(policy_route_report["decision"], "use_skill");
    assert_eq!(policy_route_report["selected"]["name"], "notes");
    assert_eq!(policy_route_report["selected"]["base_score"], 0.0);
    assert!(
        policy_route_report["selected"]["policy_score"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert_eq!(
        policy_route_report["policy"]["matched_rules"][0]["id"],
        "hidden_knowledge"
    );

    let get_rule = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("get")
        .arg("hidden_knowledge")
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&get_rule);
    let get_rule_report = json_stdout(&get_rule);
    assert_eq!(get_rule_report["id"], "hidden_knowledge");
    assert_eq!(get_rule_report["rules"][0]["id"], "hidden_knowledge");

    let set_code = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("set-profile")
        .arg("code")
        .arg("--index")
        .arg(&index)
        .arg("--mode")
        .arg("soft-passthrough")
        .arg("--active")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&set_code);

    let passthrough = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("extract pdf text")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&passthrough);
    let passthrough_report = json_stdout(&passthrough);
    assert_eq!(passthrough_report["decision"], "bypass");
    assert_eq!(passthrough_report["bypass_reason"], "policy_passthrough");
    assert_eq!(passthrough_report["policy"]["profile"], "code");

    let explain_default = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("explain")
        .arg("--index")
        .arg(&index)
        .arg("--profile")
        .arg("default")
        .arg("--query")
        .arg("please handle the hidden workflow")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&explain_default);
    let explain_default_report = json_stdout(&explain_default);
    assert_eq!(explain_default_report["decision"], "use_skill");
    assert_eq!(explain_default_report["policy"]["profile"], "default");
}

#[test]
fn router_profile_apply_status_and_clear_active_policy() {
    let dir = TempDir::new("router-profile");
    let root = dir.path().join("skills");
    let index = dir.path().join("skill-index.sqlite");
    write_file(
        &root.join("notes/SKILL.md"),
        r#"---
name: notes
description: Organize personal notes. Do not use for PDF extraction.
---
# Notes
"#,
    );

    let index_output = Command::new(bin())
        .arg("index")
        .arg("--roots")
        .arg(&root)
        .arg("--out")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&index_output);

    let init = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("init")
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&init);

    let set_profile = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("set-profile")
        .arg("focus")
        .arg("--index")
        .arg(&index)
        .arg("--mode")
        .arg("soft-passthrough")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&set_profile);

    let apply = Command::new(bin())
        .arg("router")
        .arg("profile")
        .arg("apply")
        .arg("focus")
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&apply);
    let apply_report = json_stdout(&apply);
    assert_eq!(apply_report["applied"], true);

    let status = Command::new(bin())
        .arg("router")
        .arg("profile")
        .arg("status")
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&status);
    let status_report = json_stdout(&status);
    assert_eq!(status_report["active_profile"]["name"], "focus");

    let clear = Command::new(bin())
        .arg("router")
        .arg("profile")
        .arg("clear")
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&clear);

    let status_after_clear = Command::new(bin())
        .arg("router")
        .arg("profile")
        .arg("status")
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&status_after_clear);
    let status_after_clear_report = json_stdout(&status_after_clear);
    assert_eq!(status_after_clear_report["active_profile"], Value::Null);
}

#[test]
fn router_policy_strict_profile_rejects_unknown_skill_target() {
    let dir = TempDir::new("router-policy-strict");
    let root = dir.path().join("skills");
    let index = dir.path().join("skill-index.sqlite");
    write_file(
        &root.join("notes/SKILL.md"),
        r#"---
name: notes
description: Organize personal notes. Do not use for PDF extraction.
---
# Notes
"#,
    );

    let index_output = Command::new(bin())
        .arg("index")
        .arg("--roots")
        .arg(&root)
        .arg("--out")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&index_output);

    let set_profile = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("set-profile")
        .arg("strict")
        .arg("--index")
        .arg(&index)
        .arg("--strict")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&set_profile);

    let bad_rule = Command::new(bin())
        .arg("router")
        .arg("policy")
        .arg("set-rule")
        .arg("missing")
        .arg("--index")
        .arg(&index)
        .arg("--profile")
        .arg("strict")
        .arg("--when-any")
        .arg("missing")
        .arg("--prefer")
        .arg("skill:missing")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&bad_rule);
    assert!(stderr(&bad_rule).contains("unknown skill target missing"));
}

#[test]
fn direct_index_warns_about_router_scope_and_disabled_router_mode() {
    let dir = TempDir::new("direct-index-warning");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = dir.path().join("skills");
    let index = dir.path().join("skill-index.sqlite");

    write_file(
        &root.join("notes/SKILL.md"),
        r#"---
name: notes
description: Use when organizing personal notes. Do not use for PDF extraction.
---
# Notes
"#,
    );

    let standalone = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("index")
        .arg("--roots")
        .arg(&root)
        .arg("--out")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&standalone);
    let standalone_report = json_stdout(&standalone);
    let standalone_warnings = standalone_report["warnings"].as_array().unwrap();
    assert!(standalone_warnings.iter().any(|warning| warning
        .as_str()
        .is_some_and(|text| text.contains("router-specific"))));
    assert!(standalone_warnings.iter().any(|warning| warning
        .as_str()
        .is_some_and(|text| text.contains("No installed router config"))));

    write_file(
        &skillspec_home.join("router/config.json"),
        &serde_json::to_string_pretty(&json!({
            "schema": "skillspec/router-config/v1",
            "created_at_unix": 0,
            "enabled": false,
            "roots": [root.to_string_lossy().to_string()],
            "router_skill_dirs": [root.join("skill-router").to_string_lossy().to_string()],
            "index": index.to_string_lossy().to_string(),
            "manifest": skillspec_home
                .join("router/visibility-manifest.json")
                .to_string_lossy()
                .to_string(),
            "router_name": "skill-router"
        }))
        .unwrap(),
    );

    let disabled = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("index")
        .arg("--roots")
        .arg(&root)
        .arg("--out")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disabled);
    let disabled_report = json_stdout(&disabled);
    let disabled_warnings = disabled_report["warnings"].as_array().unwrap();
    assert!(disabled_warnings.iter().any(|warning| warning
        .as_str()
        .is_some_and(|text| text.contains("installed but disabled"))));
    assert!(disabled_warnings.iter().any(|warning| warning
        .as_str()
        .is_some_and(|text| text.contains("will not affect implicit skill selection"))));
}

#[test]
fn install_targets_detect_existing_harness_dirs_and_create_missing_skill_roots() {
    let dir = TempDir::new("install-missing-skill-roots");
    let home = dir.path().join("home");
    let repo = dir.path().join("repo");
    let source = dir.path().join("source-skill");
    fs::create_dir_all(home.join(".agents")).unwrap();
    fs::create_dir_all(home.join(".codex")).unwrap();
    fs::create_dir_all(repo.join(".claude")).unwrap();
    write_file(
        &source.join("SKILL.md"),
        r#"---
name: clean-harness
description: Use when testing clean harness installs.
---
# Clean Harness
"#,
    );
    write_file(
        &source.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: clean.harness
title: Clean Harness
description: Clean harness fixture.
routes:
  - id: default
    label: Default
"#,
    );

    let targets = Command::new(bin())
        .current_dir(&repo)
        .env("HOME", &home)
        .arg("install")
        .arg("targets")
        .output()
        .unwrap();
    assert_success(&targets);
    let targets = json_stdout(&targets);
    let targets = targets.as_array().unwrap();
    for id in ["agents", "codex", "claude-local"] {
        let target = targets
            .iter()
            .find(|target| target["id"] == id)
            .unwrap_or_else(|| panic!("missing target {id}"));
        assert_eq!(target["detected"], true, "{id} should be detected");
        assert!(
            target["path"].as_str().unwrap().ends_with(match id {
                "agents" => ".agents/skills",
                "codex" => ".codex/skills",
                "claude-local" => ".claude/skills",
                _ => unreachable!(),
            }),
            "{id} should still install into the skills subfolder"
        );
    }

    let install = Command::new(bin())
        .current_dir(&repo)
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", dir.path().join(".skillspec"))
        .arg("install")
        .arg("skill")
        .arg(&source)
        .arg("--all-detected")
        .arg("--name")
        .arg("clean-harness")
        .output()
        .unwrap();
    assert_success(&install);
    let report = json_stdout(&install);
    assert_eq!(report["installs"].as_array().unwrap().len(), 3);
    assert!(home
        .join(".agents/skills/clean-harness/skill.spec.yml")
        .is_file());
    assert!(home
        .join(".codex/skills/clean-harness/skill.spec.yml")
        .is_file());
    assert!(repo
        .join(".claude/skills/clean-harness/skill.spec.yml")
        .is_file());
}

#[test]
fn visibility_apply_restore_and_manifest_override_router_index() {
    let dir = TempDir::new("visibility");
    let codex_root = dir.path().join(".codex/skills");
    let claude_root = dir.path().join("repo/.claude/skills");
    let manifest = dir.path().join("visibility-manifest.json");
    let disable_manifest = dir.path().join("disable-manifest.json");
    let index = dir.path().join("skill-index.sqlite");

    write_file(
        &codex_root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting text, tables, and images from PDF documents. Do not use for deployment work.
---
# PDF
"#,
    );
    write_file(
        &claude_root.join("deploy/SKILL.md"),
        r#"---
name: deploy
description: Use when deploying applications to production hosting targets. Do not use for PDF extraction.
---
# Deploy
"#,
    );
    write_file(
        &codex_root.join("durable-executor/SKILL.md"),
        r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#,
    );

    let plan = Command::new(bin())
        .arg("visibility")
        .arg("plan")
        .arg("--roots")
        .arg(&codex_root)
        .arg(&claude_root)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&plan);
    let plan_report = json_stdout(&plan);
    assert_eq!(plan_report["changes"].as_array().unwrap().len(), 2);
    assert!(plan_report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|change| change["after_visibility"] == "manual-only"));
    assert!(!plan_report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|change| change["skill"] == "durable-executor"));

    let apply = Command::new(bin())
        .arg("visibility")
        .arg("apply")
        .arg("--roots")
        .arg(&codex_root)
        .arg(&claude_root)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&apply);
    let apply_report = json_stdout(&apply);
    assert_eq!(apply_report["changes"].as_array().unwrap().len(), 2);
    assert!(manifest.is_file());
    assert!(
        fs::read_to_string(codex_root.join("pdf/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );
    let claude_settings: Value = serde_json::from_str(
        &fs::read_to_string(dir.path().join("repo/.claude/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        claude_settings["skillOverrides"]["deploy"],
        "user-invocable-only"
    );
    assert!(!codex_root
        .join("durable-executor/agents/openai.yaml")
        .exists());

    let restore = Command::new(bin())
        .arg("visibility")
        .arg("restore")
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&restore);
    assert!(!codex_root.join("pdf/agents/openai.yaml").exists());
    assert!(!dir.path().join("repo/.claude/settings.json").exists());

    let disable = Command::new(bin())
        .arg("skills")
        .arg("disable")
        .arg("pdf")
        .arg("--roots")
        .arg(&codex_root)
        .arg("--manifest")
        .arg(&disable_manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable);
    let disable_report = json_stdout(&disable);
    assert_eq!(disable_report["changes"][0]["after_visibility"], "off");

    let index_output = Command::new(bin())
        .arg("index")
        .arg("--roots")
        .arg(&codex_root)
        .arg("--out")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&disable_manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&index_output);

    let route = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("extract pdf text")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&route);
    let route_report = json_stdout(&route);
    assert_eq!(route_report["decision"], "bypass");
    assert_eq!(route_report["bypass_reason"], "no_candidates");
    assert_eq!(route_report["selected"], Value::Null);
    assert!(route_report["candidates"].as_array().unwrap().is_empty());
}

#[test]
fn router_install_hooks_install_skill_and_uninstall_restores_visibility() {
    let dir = TempDir::new("router-lifecycle");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");
    let source = dir.path().join("note-source");
    let codex_hooks = home.join(".codex/hooks.json");
    let claude_settings = home.join(".claude/settings.json");

    write_file(
        &codex_hooks,
        r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"echo keep-codex"}]}]}}"#,
    );
    write_file(
        &claude_settings,
        r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"echo keep-claude"}]}]}}"#,
    );

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#,
    );
    write_file(
        &root.join("durable-executor/SKILL.md"),
        r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    let install_report = json_stdout(&install_router);
    assert_eq!(install_report["router_skill_status"], "installed");
    assert_eq!(install_report["durable_executor"]["present"], true);
    assert_eq!(install_report["harness_hooks"].as_array().unwrap().len(), 2);
    assert!(has_hook_command(&codex_hooks, "skillspec router guard"));
    assert!(has_hook_command(&codex_hooks, "echo keep-codex"));
    assert!(has_hook_command(&claude_settings, "skillspec router guard"));
    assert!(has_hook_command(&claude_settings, "echo keep-claude"));

    let lifecycle_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("status")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&lifecycle_status);
    let lifecycle_report = json_stdout(&lifecycle_status);
    let status_hooks = lifecycle_report["router"]["harness_hooks"]
        .as_array()
        .unwrap();
    assert_eq!(status_hooks.len(), 2);
    assert!(status_hooks
        .iter()
        .all(|hook| hook["status"] == "installed"));
    assert_eq!(
        install_report["visibility"]["changes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(install_report["preparedness"]["ready"], true);
    assert_eq!(install_report["preparedness"]["status_checked"], true);
    assert_eq!(install_report["preparedness"]["index_stale"], false);
    assert_eq!(install_report["preparedness"]["indexed_skills"], 3);
    assert!(root.join("skill-router/SKILL.md").is_file());
    assert!(root.join("skill-router/skill.spec.yml").is_file());
    assert!(root
        .join("skill-router/.skillspec-router-managed")
        .is_file());
    let router_skill = fs::read_to_string(root.join("skill-router/SKILL.md")).unwrap();
    assert!(router_skill.contains("skill.spec.yml"));
    assert!(router_skill.contains("Use for every user request"));
    assert!(router_skill.contains("the first hop for every user request"));
    assert!(router_skill.contains("Fast Path"));
    assert!(router_skill.contains("do not run"));
    assert!(router_skill.contains("router index status"));
    assert!(router_skill.contains("continue with the normal agent path"));
    assert!(router_skill.contains("explicit-only"));
    assert!(router_skill.contains("decision: \"use_skill\""));
    assert!(router_skill.contains("candidate skill"));
    assert!(router_skill.contains("durable-executor"));
    assert!(!router_skill.contains("visible discovery surface"));
    let router_spec = fs::read_to_string(root.join("skill-router/skill.spec.yml")).unwrap();
    assert!(router_spec.contains("schema: skillspec/v0"));
    assert!(router_spec.contains("Use for every user request"));
    assert!(router_spec.contains("first hop for every request"));
    assert!(router_spec.contains("apply_route_decision"));
    assert!(router_spec.contains("decision is use_skill"));
    assert!(!router_spec.contains("id: check_index_status"));
    assert!(!router_spec.contains("--router-root"));
    let validate_router_spec = Command::new(bin())
        .arg("validate")
        .arg(root.join("skill-router/skill.spec.yml"))
        .output()
        .unwrap();
    assert_success(&validate_router_spec);
    assert!(root.join("pdf/agents/openai.yaml").is_file());
    assert!(!root.join("skill-router/agents/openai.yaml").exists());
    assert!(!root.join("durable-executor/agents/openai.yaml").exists());
    assert!(fs::read_to_string(root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!fs::read_to_string(root.join("skill-router/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!fs::read_to_string(root.join("durable-executor/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(index.is_file());
    assert!(skillspec_home.join("router/config.json").is_file());

    let clean_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&clean_status);
    let clean_report = json_stdout(&clean_status);
    assert_eq!(clean_report["stale"], false);
    assert_eq!(clean_report["indexed_skills"], 3);

    write_file(
        &source.join("SKILL.md"),
        r#"---
name: notes
description: Use when taking structured notes and summarizing meeting action items. Do not use for PDF extraction.
---
# Notes
"#,
    );
    write_file(
        &source.join("skill.spec.yml"),
        r#"
schema: skillspec/v0
id: notes.skill
title: Notes
description: Notes fixture.
routes:
  - id: local
    label: Local
"#,
    );
    let install_skill = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("install")
        .arg("skill")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--name")
        .arg("notes")
        .output()
        .unwrap();
    assert_success(&install_skill);
    assert!(root.join("notes/agents/openai.yaml").is_file());

    let refreshed_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&refreshed_status);
    let refreshed_report = json_stdout(&refreshed_status);
    assert_eq!(refreshed_report["stale"], false);
    assert_eq!(refreshed_report["indexed_skills"], 4);
    assert!(fs::read_to_string(root.join("notes/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));

    let route_notes = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("summarize meeting action items as notes")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&route_notes);
    let route_report = json_stdout(&route_notes);
    assert_eq!(route_report["decision"], "use_skill");
    assert_eq!(route_report["selected"]["name"], "notes");

    let route_time = Command::new(bin())
        .arg("route")
        .arg("--index")
        .arg(&index)
        .arg("--query")
        .arg("what is the time today")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&route_time);
    let route_time_report = json_stdout(&route_time);
    assert_ne!(route_time_report["decision"], "use_skill");
    assert_eq!(route_time_report["selected"], Value::Null);
    assert!(route_time_report["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .all(|candidate| candidate["name"] != "skill-router"));

    let uninstall_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("uninstall")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&uninstall_router);
    let uninstall_report = json_stdout(&uninstall_router);
    assert_eq!(uninstall_report["router_skill_status"], "removed");
    assert_eq!(uninstall_report["index_removed"], true);
    assert!(!root.join("skill-router").exists());
    assert!(!index.exists());
    assert!(!skillspec_home.join("router/config.json").exists());
    assert!(!root.join("pdf/agents/openai.yaml").exists());
    assert!(!root.join("durable-executor/agents/openai.yaml").exists());
    assert!(!root.join("notes/agents/openai.yaml").exists());
    assert!(!has_hook_command(&codex_hooks, "skillspec router guard"));
    assert!(has_hook_command(&codex_hooks, "echo keep-codex"));
    assert!(!has_hook_command(
        &claude_settings,
        "skillspec router guard"
    ));
    assert!(has_hook_command(&claude_settings, "echo keep-claude"));
    assert!(!fs::read_to_string(root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!fs::read_to_string(root.join("notes/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
}

#[test]
#[cfg(unix)]
fn router_install_tracks_symlinked_harness_roots_and_uninstalls_all() {
    let dir = TempDir::new("router-symlink-roots");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = home.join(".codex/skills");
    let vendor_root = home.join(".vendor/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");

    fs::create_dir_all(&agents_root).unwrap();
    fs::create_dir_all(codex_root.parent().unwrap()).unwrap();
    fs::create_dir_all(vendor_root.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&agents_root, &codex_root).unwrap();
    std::os::unix::fs::symlink(&agents_root, &vendor_root).unwrap();

    write_file(
        &agents_root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#,
    );
    write_file(
        &agents_root.join("durable-executor/SKILL.md"),
        r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg(&vendor_root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    let install_report = json_stdout(&install_router);
    assert_eq!(install_report["router_skill_status"], "installed");
    assert_eq!(
        install_report["router_skill_dirs"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        install_report["router_skill_reports"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    for root in [&agents_root, &codex_root, &vendor_root] {
        assert!(root.join("skill-router/SKILL.md").is_file());
        assert!(root.join("skill-router/skill.spec.yml").is_file());
        assert!(root
            .join("skill-router/.skillspec-router-managed")
            .is_file());
    }

    let config = fs::read_to_string(skillspec_home.join("router/config.json")).unwrap();
    let config_json: Value = serde_json::from_str(&config).unwrap();
    assert_eq!(config_json["roots"].as_array().unwrap().len(), 3);
    assert_eq!(
        config_json["router_skill_dirs"].as_array().unwrap().len(),
        3
    );

    let uninstall_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("uninstall")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&uninstall_router);
    let uninstall_report = json_stdout(&uninstall_router);
    assert_eq!(uninstall_report["router_skill_status"], "removed");
    assert_eq!(
        uninstall_report["router_skill_reports"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    for root in [&agents_root, &codex_root, &vendor_root] {
        assert!(!root.join("skill-router").exists());
    }
    assert!(!index.exists());
    assert!(!skillspec_home.join("router/config.json").exists());
}

#[test]
fn router_install_handles_duplicate_skill_names_across_roots() {
    let dir = TempDir::new("router-duplicate-names");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = home.join(".codex/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");

    write_file(
        &agents_root.join("rote/SKILL.md"),
        r#"---
name: rote
description: Use rote before tool calls from the shared agents root.
---
# Rote
"#,
    );
    write_file(
        &codex_root.join("rote/SKILL.md"),
        r#"---
name: rote
description: Use rote before tool calls from the Codex root.
---
# Rote
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    let install_report = json_stdout(&install_router);
    assert_eq!(install_report["preparedness"]["ready"], true);
    assert_eq!(install_report["preparedness"]["index_stale"], false);
    assert_eq!(install_report["preparedness"]["indexed_skills"], 4);
    assert_eq!(install_report["preparedness"]["discovered_skills"], 4);
    assert!(skillspec_home.join("router/config.json").is_file());

    let status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&status);
    let status_report = json_stdout(&status);
    assert_eq!(status_report["stale"], false);
    assert_eq!(status_report["indexed_skills"], 4);
    assert_eq!(status_report["discovered_skills"], 4);
    assert!(status_report["new_skills"].as_array().unwrap().is_empty());
    assert!(status_report["changed_skills"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(status_report["missing_skills"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
#[cfg(unix)]
fn router_update_backs_up_and_repairs_all_recorded_router_roots() {
    let dir = TempDir::new("router-update");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = home.join(".codex/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");
    let backup_dir = skillspec_home.join("router/update-backup");

    fs::create_dir_all(&agents_root).unwrap();
    fs::create_dir_all(codex_root.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&agents_root, &codex_root).unwrap();

    write_file(
        &agents_root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    write_file(
        &agents_root.join("skill-router/SKILL.md"),
        r#"---
name: skill-router
description: stale router text
---
# Skill Router

Use this skill as the visible discovery surface for large local skill libraries.
"#,
    );
    fs::remove_file(codex_root.join("skill-router/skill.spec.yml")).unwrap();

    let update_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("update")
        .arg("--backup-dir")
        .arg(&backup_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&update_router);
    let update_report = json_stdout(&update_router);
    assert_eq!(
        update_report["router_skill_reports"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        update_report["backup"]["path"].as_str().unwrap(),
        backup_dir.to_string_lossy()
    );
    assert!(update_report["restart_warning"]
        .as_str()
        .unwrap()
        .contains("Restart active"));
    assert!(backup_dir.join("backup.json").is_file());
    assert!(backup_dir.join("router-skill-0/SKILL.md").is_file());
    assert!(
        fs::read_to_string(backup_dir.join("router-skill-0/SKILL.md"))
            .unwrap()
            .contains("visible discovery surface")
    );

    for root in [&agents_root, &codex_root] {
        let router_skill = fs::read_to_string(root.join("skill-router/SKILL.md")).unwrap();
        assert!(router_skill.contains("router mode is enabled"));
        assert!(router_skill.contains("explicit-only"));
        assert!(!router_skill.contains("visible discovery surface"));
        assert!(root.join("skill-router/skill.spec.yml").is_file());
        assert!(root
            .join("skill-router/.skillspec-router-managed")
            .is_file());
    }
    let config = fs::read_to_string(skillspec_home.join("router/config.json")).unwrap();
    let config_json: Value = serde_json::from_str(&config).unwrap();
    assert_eq!(
        config_json["router_skill_dirs"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn router_index_refresh_repairs_out_of_band_skills_and_advises_conversion() {
    let dir = TempDir::new("router-out-of-band");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let manifest = skillspec_home.join("router/visibility-manifest.json");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#,
    );
    write_file(
        &root.join("durable-executor/SKILL.md"),
        r#"---
name: durable-executor
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    write_file(
        &root.join("legacy-prose/SKILL.md"),
        r#"---
name: legacy-prose
description: Use when a legacy prose-only workflow should be routed. Do not use for PDF extraction.
---
# Legacy Prose
"#,
    );
    write_file(
        &root.join("spec-backed/SKILL.md"),
        r#"---
name: spec-backed
description: Use when a SkillSpec-backed out-of-band workflow should be routed. Do not use for PDF extraction.
---
# Spec Backed
"#,
    );
    write_file(
        &root.join("spec-backed/skill.spec.yml"),
        r#"
schema: skillspec/v0
id: spec.backed
title: Spec Backed
description: Fixture for out-of-band SkillSpec-backed routing.
routes:
  - id: local
    label: Local
"#,
    );

    let stale_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&stale_status);
    let stale_report = json_stdout(&stale_status);
    assert_eq!(stale_report["stale"], true);
    let new_skills = stale_report["new_skills"].as_array().unwrap();
    let prose = new_skills
        .iter()
        .find(|entry| entry["name"] == "legacy-prose")
        .unwrap();
    assert_eq!(prose["has_skill_spec"], false);
    assert!(prose["advice"].as_str().unwrap().contains("import-skill"));
    let spec_backed = new_skills
        .iter()
        .find(|entry| entry["name"] == "spec-backed")
        .unwrap();
    assert_eq!(spec_backed["has_skill_spec"], true);
    assert!(spec_backed["advice"]
        .as_str()
        .unwrap()
        .contains("SkillSpec-backed"));
    assert!(stale_report["advice"]
        .as_array()
        .unwrap()
        .iter()
        .any(|advice| advice
            .as_str()
            .is_some_and(|text| text.contains("router index refresh"))));
    assert!(!root.join("legacy-prose/agents/openai.yaml").exists());
    assert!(!fs::read_to_string(root.join("legacy-prose/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));

    let refresh = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("refresh")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&refresh);
    let refresh_report = json_stdout(&refresh);
    assert_eq!(refresh_report["router_config_present"], true);
    assert_eq!(refresh_report["status_before"]["stale"], true);
    assert_eq!(refresh_report["preparedness"]["ready"], true);
    assert_eq!(refresh_report["index_report"]["skills_indexed"], 5);
    assert!(refresh_report["advice"]
        .as_array()
        .unwrap()
        .iter()
        .any(|advice| advice
            .as_str()
            .is_some_and(|text| text.contains("import-skill"))));
    assert!(root.join("legacy-prose/agents/openai.yaml").is_file());
    assert!(root.join("spec-backed/agents/openai.yaml").is_file());
    assert!(fs::read_to_string(root.join("legacy-prose/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(fs::read_to_string(root.join("spec-backed/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!root.join("durable-executor/agents/openai.yaml").exists());
    assert!(!fs::read_to_string(root.join("durable-executor/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));

    let clean_status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--visibility-manifest")
        .arg(&manifest)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&clean_status);
    let clean_report = json_stdout(&clean_status);
    assert_eq!(clean_report["stale"], false);
    assert_eq!(clean_report["indexed_skills"], 5);
    assert!(clean_report["new_skills"].as_array().unwrap().is_empty());
}

#[test]
fn router_guard_repairs_out_of_band_skills_and_emits_hook_output() {
    let dir = TempDir::new("router-guard");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".codex/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let config = skillspec_home.join("router/config.json");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text.
---
# PDF
"#,
    );

    let missing_hook = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("guard")
        .arg("--config")
        .arg(&config)
        .arg("--hook")
        .output()
        .unwrap();
    assert_success(&missing_hook);
    let missing_hook_report = json_stdout(&missing_hook);
    assert_eq!(missing_hook_report["decision"], "block");
    assert!(missing_hook_report["reason"]
        .as_str()
        .unwrap()
        .contains("router config is missing"));
    assert!(missing_hook_report["reason"]
        .as_str()
        .unwrap()
        .contains("router install"));

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    write_file(
        &root.join("notes/SKILL.md"),
        r#"---
name: notes
description: Use when taking structured notes.
---
# Notes
"#,
    );
    assert!(!root.join("notes/agents/openai.yaml").exists());

    let guard = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("guard")
        .arg("--config")
        .arg(&config)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&guard);
    let guard_report = json_stdout(&guard);
    assert_eq!(guard_report["installed"], true);
    assert_eq!(guard_report["enabled"], true);
    assert_eq!(guard_report["repaired"], true);
    assert_eq!(guard_report["first_hop_ready"], true);
    assert_eq!(guard_report["status_before"]["stale"], true);
    assert_eq!(guard_report["status_after"]["stale"], false);
    assert_eq!(guard_report["index_report"]["skills_indexed"], 3);
    assert!(fs::read_to_string(root.join("notes/agents/openai.yaml"))
        .unwrap()
        .contains("allow_implicit_invocation: false"));

    let hook = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("guard")
        .arg("--config")
        .arg(&config)
        .arg("--hook")
        .output()
        .unwrap();
    assert_success(&hook);
    let hook_report = json_stdout(&hook);
    assert!(hook_report["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap()
        .contains("first_hop_ready=true"));

    let disable_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable_router);

    let disabled_hook = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("guard")
        .arg("--config")
        .arg(&config)
        .arg("--hook")
        .output()
        .unwrap();
    assert_success(&disabled_hook);
    let disabled_hook_report = json_stdout(&disabled_hook);
    assert_eq!(disabled_hook_report["decision"], "block");
    assert!(disabled_hook_report["reason"]
        .as_str()
        .unwrap()
        .contains("router config is installed but disabled"));
    assert!(disabled_hook_report["reason"]
        .as_str()
        .unwrap()
        .contains("router enable"));
}

#[test]
fn router_install_reports_missing_optional_durable_executor() {
    let dir = TempDir::new("router-missing-durable");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images.
---
# PDF
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    let install_report = json_stdout(&install_router);
    assert_eq!(install_report["durable_executor"]["present"], false);
    assert_eq!(install_report["preparedness"]["ready"], true);
    assert!(install_report["durable_executor"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning
            .as_str()
            .is_some_and(|text| text.contains("durable first-hop is unavailable"))));
    assert!(root.join("skill-router/SKILL.md").is_file());
    assert!(root.join("skill-router/skill.spec.yml").is_file());
    assert!(!root.join("skill-router/agents/openai.yaml").exists());
    assert!(fs::read_to_string(root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!fs::read_to_string(root.join("skill-router/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(!root.join("durable-executor").exists());
}

#[test]
fn router_disable_and_enable_toggle_visibility_and_reindex_all_roots() {
    let dir = TempDir::new("router-enable-disable");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = home.join(".codex/skills");
    let codex_hooks = home.join(".codex/hooks.json");
    let index = skillspec_home.join("router/skill-index.sqlite");

    write_file(
        &agents_root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text.
---
# PDF
"#,
    );
    write_file(
        &codex_root.join("csv/SKILL.md"),
        r#"---
name: csv
description: Use when working with CSV files.
---
# CSV
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&agents_root)
        .arg(&codex_root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);
    assert!(has_hook_command(&codex_hooks, "skillspec router guard"));
    assert!(fs::read_to_string(agents_root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(
        fs::read_to_string(codex_root.join("csv/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );
    assert!(
        !fs::read_to_string(agents_root.join("skill-router/SKILL.md"))
            .unwrap()
            .contains("disable-model-invocation: true")
    );

    let disable_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable_router);
    let disable_report = json_stdout(&disable_router);
    assert_eq!(disable_report["enabled"], false);
    assert!(disable_report["index_report"].is_null());
    assert!(!has_hook_command(&codex_hooks, "skillspec router guard"));
    assert!(fs::read_to_string(agents_root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: false"));
    assert!(
        fs::read_to_string(agents_root.join("pdf/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: true")
    );
    assert!(
        fs::read_to_string(codex_root.join("csv/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: true")
    );
    assert!(
        fs::read_to_string(agents_root.join("skill-router/SKILL.md"))
            .unwrap()
            .contains("disable-model-invocation: true")
    );
    assert!(
        fs::read_to_string(codex_root.join("skill-router/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );

    write_file(
        &codex_root.join("markdown/SKILL.md"),
        r#"---
name: markdown
description: Use when editing markdown.
---
# Markdown
"#,
    );

    let enable_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&enable_router);
    let enable_report = json_stdout(&enable_router);
    assert_eq!(enable_report["enabled"], true);
    assert_eq!(enable_report["preparedness"]["ready"], true);
    assert_eq!(enable_report["index_report"]["skills_indexed"], 5);
    assert!(has_hook_command(&codex_hooks, "skillspec router guard"));
    assert!(fs::read_to_string(agents_root.join("pdf/SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(
        fs::read_to_string(codex_root.join("markdown/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );
    assert!(
        !fs::read_to_string(agents_root.join("skill-router/SKILL.md"))
            .unwrap()
            .contains("disable-model-invocation: true")
    );
    assert!(
        fs::read_to_string(codex_root.join("skill-router/agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: true")
    );
}

#[test]
fn durable_executor_lifecycle_installs_updates_and_deletes_managed_dirs() {
    let dir = TempDir::new("durable-lifecycle");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let root = home.join(".agents/skills");
    let source = dir.path().join("source");
    write_durable_source(&source, "initial");

    let install = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install);
    let install_report = json_stdout(&install);
    assert_eq!(install_report["skill_name"], "durable-executor");
    assert_eq!(install_report["rote_preflight"]["present"], true);
    assert_eq!(install_report["managed_installs"][0]["status"], "installed");
    assert!(root.join("durable-executor/SKILL.md").is_file());
    assert!(root
        .join("durable-executor/.skillspec-durable-executor-managed")
        .is_file());
    assert!(skillspec_home
        .join("durable-executor/config.json")
        .is_file());

    fs::remove_file(root.join("durable-executor/.skillspec-durable-executor-managed")).unwrap();
    let unsafe_update = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("update")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&unsafe_update);
    assert!(stderr(&unsafe_update).contains("managed marker"));
    write_file(
        &root.join("durable-executor/.skillspec-durable-executor-managed"),
        "schema: skillspec/durable-executor-managed/v1\n",
    );

    write_durable_source(&source, "updated");
    let update = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("update")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&update);
    let update_report = json_stdout(&update);
    assert_eq!(update_report["rote_preflight"]["present"], true);
    assert_eq!(update_report["managed_installs"][0]["status"], "updated");
    assert!(update_report["backup"]["path"].as_str().is_some());
    assert!(fs::read_to_string(root.join("durable-executor/SKILL.md"))
        .unwrap()
        .contains("updated"));
    assert!(root
        .join("durable-executor/.skillspec-durable-executor-managed")
        .is_file());

    fs::remove_file(root.join("durable-executor/.skillspec-durable-executor-managed")).unwrap();
    let unsafe_delete = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("durable-executor")
        .arg("delete")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&unsafe_delete);
    assert!(stderr(&unsafe_delete).contains("managed marker"));
    assert!(root.join("durable-executor").exists());

    write_file(
        &root.join("durable-executor/.skillspec-durable-executor-managed"),
        "schema: skillspec/durable-executor-managed/v1\n",
    );
    let delete = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("durable-executor")
        .arg("delete")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&delete);
    let delete_report = json_stdout(&delete);
    assert_eq!(delete_report["managed_installs"][0]["status"], "removed");
    assert_eq!(delete_report["config_removed"], true);
    assert!(!root.join("durable-executor").exists());
    assert!(!skillspec_home.join("durable-executor/config.json").exists());
}

#[test]
fn durable_executor_disable_and_enable_toggle_implicit_invocation() {
    let dir = TempDir::new("durable-enable-disable");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let source = dir.path().join("source");
    let agents_install = home.join(".agents/skills/durable-executor");
    let codex_install = home.join(".codex/skills/durable-executor");
    write_durable_source(&source, "toggle visibility");

    let install = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--target")
        .arg("codex")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install);

    let disable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("durable-executor")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable);
    let disable_report = json_stdout(&disable);
    assert_eq!(disable_report["enabled"], false);
    assert!(fs::read_to_string(agents_install.join("SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: true"));
    assert!(
        fs::read_to_string(agents_install.join("agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: false")
    );
    assert!(fs::read_to_string(codex_install.join("agents/openai.yaml"))
        .unwrap()
        .contains("allow_implicit_invocation: false"));

    let config_path = skillspec_home.join("durable-executor/config.json");
    let config: Value = serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(config["enabled"], false);

    let enable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&enable);
    let enable_report = json_stdout(&enable);
    assert_eq!(enable_report["enabled"], true);
    assert!(fs::read_to_string(agents_install.join("SKILL.md"))
        .unwrap()
        .contains("disable-model-invocation: false"));
    assert!(
        fs::read_to_string(agents_install.join("agents/openai.yaml"))
            .unwrap()
            .contains("allow_implicit_invocation: true")
    );
    assert!(fs::read_to_string(codex_install.join("agents/openai.yaml"))
        .unwrap()
        .contains("allow_implicit_invocation: true"));
}

#[test]
fn durable_executor_install_requires_rote_on_path() {
    let dir = TempDir::new("durable-requires-rote");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let empty_path = dir.path().join("empty-path");
    fs::create_dir_all(&empty_path).unwrap();
    let root = home.join(".agents/skills");
    let source = dir.path().join("source");
    write_durable_source(&source, "missing rote");

    let install = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &empty_path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&install);
    assert!(stderr(&install).contains("requires `rote` on PATH"));
    assert!(!root.join("durable-executor").exists());
    assert!(!skillspec_home.join("durable-executor/config.json").exists());

    let dry_run = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &empty_path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert_success(&dry_run);
    let dry_run_report = json_stdout(&dry_run);
    assert_eq!(dry_run_report["rote_preflight"]["present"], false);
    assert!(!root.join("durable-executor").exists());
    assert!(!skillspec_home.join("durable-executor/config.json").exists());
}

#[test]
fn durable_executor_enable_requires_rote_on_path() {
    let dir = TempDir::new("durable-enable-requires-rote");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let empty_path = dir.path().join("empty-path");
    fs::create_dir_all(&empty_path).unwrap();
    let source = dir.path().join("source");
    let install_dir = home.join(".agents/skills/durable-executor");
    write_durable_source(&source, "enable missing rote");

    let install = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install);

    let disable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &empty_path)
        .arg("durable-executor")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable);

    let enable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &empty_path)
        .arg("durable-executor")
        .arg("enable")
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&enable);
    assert!(stderr(&enable).contains("requires `rote` on PATH"));
    assert!(fs::read_to_string(install_dir.join("agents/openai.yaml"))
        .unwrap()
        .contains("allow_implicit_invocation: false"));

    let config_path = skillspec_home.join("durable-executor/config.json");
    let config: Value = serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(config["enabled"], false);
}

#[test]
fn status_reports_lifecycle_roots_index_and_skill_inventory() {
    let dir = TempDir::new("status-lifecycle-inventory");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let durable_source = dir.path().join("durable-source");
    write_durable_source(&durable_source, "status inventory");
    write_file(
        &root.join("alpha/SKILL.md"),
        r#"---
name: alpha
description: Alpha SkillSpec-backed skill.
---
# Alpha
"#,
    );
    write_file(
        &root.join("alpha/skill.spec.yml"),
        r#"
schema: skillspec/v0
id: alpha
title: Alpha
description: Alpha SkillSpec-backed skill.
routes:
  - id: alpha
    label: Alpha
"#,
    );
    write_file(
        &root.join("legacy/SKILL.md"),
        r#"---
name: legacy
description: Legacy prose-only skill.
---
# Legacy
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    let install_durable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&durable_source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_durable);

    let disable_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable_router);

    let disable_durable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("durable-executor")
        .arg("disable")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&disable_durable);

    let status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("status")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&status);
    let report = json_stdout(&status);
    assert_eq!(report["router"]["installed"], true);
    assert_eq!(report["router"]["enabled"], false);
    assert_eq!(report["router"]["disabled"], true);
    assert_eq!(report["durable_executor"]["installed"], true);
    assert_eq!(report["durable_executor"]["enabled"], false);
    assert_eq!(report["durable_executor"]["disabled"], true);
    assert_eq!(report["roots"]["scan_source"], "router_config");
    assert_eq!(report["roots"]["scanned_count"], 1);
    assert!(report["roots"]["supported_count"].as_u64().unwrap() >= 2);
    assert_eq!(report["skills"]["legacy_count"], 1);
    assert!(report["skills"]["skillspec_backed_count"].as_u64().unwrap() >= 3);
    assert!(report["skills"]["legacy"]
        .as_array()
        .unwrap()
        .iter()
        .any(|skill| skill["name"] == "legacy"));
    assert!(report["skills"]["skillspec_backed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|skill| skill["name"] == "alpha"));
    assert_eq!(report["router"]["index_status"]["exists"], true);
    assert_eq!(
        report["router"]["index_status"]["discovered_skills"],
        report["skills"]["total"]
    );
}

#[test]
fn durable_executor_install_refreshes_router_and_remains_implicit() {
    let dir = TempDir::new("durable-router-hook");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let path = write_fake_rote(dir.path());
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");
    let source = dir.path().join("durable-source");
    write_durable_source(&source, "router hook");
    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images.
---
# PDF
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_router);

    let install_durable = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .env("PATH", &path)
        .arg("durable-executor")
        .arg("install")
        .arg(&source)
        .arg("--target")
        .arg("agents")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&install_durable);
    let durable_report = json_stdout(&install_durable);
    assert!(durable_report["router_hook"].is_object());
    assert!(root.join("durable-executor/SKILL.md").is_file());
    assert!(!root.join("durable-executor/agents/openai.yaml").exists());

    let status = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("index")
        .arg("status")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&status);
    let status_report = json_stdout(&status);
    assert_eq!(status_report["stale"], false);
    assert_eq!(status_report["indexed_skills"], 3);
}

#[test]
fn router_install_rejects_invalid_router_name() {
    let dir = TempDir::new("router-invalid-name");
    let home = dir.path().join("home");
    let skillspec_home = dir.path().join("skillspec-home");
    let root = home.join(".agents/skills");
    let index = skillspec_home.join("router/skill-index.sqlite");

    write_file(
        &root.join("pdf/SKILL.md"),
        r#"---
name: pdf
description: Use when extracting PDF text, tables, and images.
---
# PDF
"#,
    );

    let install_router = Command::new(bin())
        .env("HOME", &home)
        .env("SKILLSPEC_HOME", &skillspec_home)
        .arg("router")
        .arg("install")
        .arg("--roots")
        .arg(&root)
        .arg("--index")
        .arg(&index)
        .arg("--router-name")
        .arg("../skill-router")
        .output()
        .unwrap();
    assert_failure(&install_router);
    assert!(stderr(&install_router).contains("router name must start"));
    assert!(!home.join(".agents/skill-router").exists());
}
