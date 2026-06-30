use crate::support::*;

#[test]
fn compile_targets_render_loader_and_full_markdown() {
    let dir = TempDir::new("compile");
    let spec = dir.path().join("skill.spec.yml");
    write_file(&spec, rich_spec());

    let loader = Command::new(bin())
        .arg("compile")
        .arg(&spec)
        .arg("--target")
        .arg("codex-skill")
        .output()
        .unwrap();
    assert_success(&loader);
    let loader_out = stdout(&loader);
    assert!(loader_out.contains(
        "description: \"Universal CLI/API/shell router with trace and alignment benefits."
    ));
    assert!(loader_out.contains("Use the directory that contains this loaded `SKILL.md`"));
    assert!(loader_out.contains("skillspec run-loop <skill_dir>/skill.spec.yml"));
    assert!(loader_out.contains("--guide agent"));
    assert!(loader_out.contains("--resume <run_dir>"));
    assert!(loader_out.contains("Keep SkillSpec mechanics in the background"));
    assert!(loader_out.contains("For read-only diagnostic routes"));
    assert!(loader_out.contains("batch routine successful evidence"));
    assert!(loader_out.contains("cargo install skillspec"));
    assert!(loader_out.contains("skill.spec.yml"));
    assert!(loader_out.lines().count() < 70);
    assert!(!loader_out.contains("## Runtime Contract"));
    assert!(!loader_out.contains("## Completion Report"));
    assert!(!loader_out.contains("## Authoring And Revision Contract"));
    assert!(!loader_out.contains("## Durable Handoff Contract"));
    assert!(!loader_out.contains("skillspec act ./skill.spec.yml"));
    assert!(!loader_out.contains("## Rules"));

    let markdown = Command::new(bin())
        .arg("compile")
        .arg(&spec)
        .arg("--target")
        .arg("markdown")
        .output()
        .unwrap();
    assert_success(&markdown);
    let markdown_out = stdout(&markdown);
    assert!(markdown_out.contains("## Authoring And Revision Contract"));
    assert!(markdown_out.contains("skillspec act <skill-folder>/skill.spec.yml"));
    assert!(markdown_out.contains("OODA loop for the selected route"));
    assert!(markdown_out.contains("## Rules"));
    assert!(markdown_out.contains("## Scenario Tests"));
    assert!(markdown_out.contains("browse_rule"));
}

#[test]
fn import_skill_creates_valid_structured_draft() {
    let dir = TempDir::new("import");
    let skill_dir = dir.path().join("source-skill");
    let out = dir.path().join("skill.spec.yml");
    write_file(
        &skill_dir.join("SKILL.md"),
        r#"# Imported Skill

Always validate inputs before writing files.

```bash
echo "hello"
```
"#,
    );
    write_file(
        &skill_dir.join("reference.md"),
        r#"# Reference

```python
print("hello")
```
"#,
    );

    let import = Command::new(bin())
        .arg("import-skill")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&import);
    assert!(out.is_file());
    assert!(dir.path().join("source/SKILL_md.old").is_file());
    assert!(!dir.path().join("source/SKILL.md").is_file());
    let import_out = stdout(&import);
    assert!(import_out.contains("review note"));
    assert!(import_out.contains("skillspec grammar sensemake --view porting"));
    assert!(import_out.contains("skillspec sensemake"));
    assert!(import_out.contains("skillspec grammar checklist --for import-skill"));
    assert!(import_out.contains("deps ledger: wrote deps.toml"));
    assert!(import_out.contains("byte-empty ledger is not"));

    let validate = Command::new(bin())
        .arg("validate")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&validate);

    let content = fs::read_to_string(&out).unwrap();
    assert!(content.contains("review_required"));
    assert!(content.contains("imports:"));
    assert!(content.contains("reference:"));
    assert!(content.contains("import: reference"));
    assert!(content.contains("command_block_1"));
    assert!(content.contains("python3"));
    assert!(content.contains("dependency_ledger"));
    assert!(content.contains("path: source/SKILL_md.old"));

    let draft_sensemake = Command::new(bin())
        .arg("sensemake")
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&draft_sensemake);
    let draft_sensemake_out = stdout(&draft_sensemake);
    assert!(draft_sensemake_out.contains("inspect dependency ledger"));
    assert!(draft_sensemake_out.contains("dependency_count = 0 is valid"));

    let ledger = dir.path().join("deps.toml");
    assert!(ledger.is_file());
    let ledger_content = fs::read_to_string(&ledger).unwrap();
    assert!(ledger_content.contains("schema_version = 1"));
    assert!(ledger_content.contains("dependency_count = "));
    assert!(ledger_content.contains("id = \"python3\""));

    let deps_check = Command::new(bin())
        .arg("deps")
        .arg("check")
        .arg(&out)
        .output()
        .unwrap();
    let deps_report = json_stdout(&deps_check);
    assert!(deps_report["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| dependency["id"] == "dependency_ledger"
            && dependency["status"] == "present"
            && dependency["message"] == "deps.toml exists"));
}

#[test]
fn source_map_guides_progressive_import_and_stale_gate() {
    let dir = TempDir::new("source-map");
    let skill_dir = dir.path().join("source-skill");
    let map_dir = dir.path().join("source-map");
    let out = dir.path().join("draft").join("skill.spec.yml");
    write_file(
        &skill_dir.join("SKILL.md"),
        r#"---
name: progressive-skill
description: Use when a large Markdown skill must be mapped before import.
---

# Progressive Skill

Always inspect dependencies before proof.

See [reference](reference.md).

```python
import json
import pypdf
from reportlab.pdfgen import canvas
```

```ts
import { chromium } from "playwright";
```
"#,
    );
    write_file(
        &skill_dir.join("reference.md"),
        "# Reference\n\nNever skip referenced local files.\n",
    );

    let map = Command::new(bin())
        .arg("source")
        .arg("map")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&map_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&map);
    let map_report = json_stdout(&map);
    assert_eq!(map_report["files"], 2);
    assert!(map_dir.join("source-map.json").is_file());
    assert!(map_dir.join("source-map.md").is_file());

    let nodes = Command::new(bin())
        .arg("source")
        .arg("query")
        .arg(map_dir.join("source-map.json"))
        .arg("nodes")
        .arg("--view")
        .arg("summary")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&nodes);
    let nodes = json_stdout(&nodes);
    let nodes = nodes.as_array().unwrap();
    assert!(nodes
        .iter()
        .any(|node| node["id"] == "frontmatter:skill-md"));
    assert!(nodes
        .iter()
        .any(|node| node["id"] == "heading:skill-md.progressive-skill"));
    assert!(nodes.iter().any(|node| node["kind"] == "code"));

    let deps = Command::new(bin())
        .arg("source")
        .arg("query")
        .arg(map_dir.join("source-map.json"))
        .arg("dependencies")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&deps);
    let deps = json_stdout(&deps);
    let deps_text = serde_json::to_string(&deps).unwrap();
    assert!(deps_text.contains("pypdf"));
    assert!(deps_text.contains("reportlab"));
    assert!(deps_text.contains("playwright"));
    assert!(
        !deps.as_array().unwrap().iter().any(|entry| entry["signals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|signal| signal == "json"))
    );

    let coverage = Command::new(bin())
        .arg("source")
        .arg("coverage")
        .arg(map_dir.join("source-map.json"))
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&coverage);
    let coverage = json_stdout(&coverage);
    assert!(coverage["total_nodes"].as_u64().unwrap() > 0);
    assert!(coverage["review_required"].as_u64().unwrap() > 0);

    let import = Command::new(bin())
        .arg("import-skill")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&out)
        .arg("--source-map")
        .arg(map_dir.join("source-map.json"))
        .output()
        .unwrap();
    assert_success(&import);
    assert!(out.is_file());

    let stale_fresh = Command::new(bin())
        .arg("source")
        .arg("stale")
        .arg(map_dir.join("source-map.json"))
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&stale_fresh);
    assert_eq!(json_stdout(&stale_fresh)["ok"], true);

    write_file(
        &skill_dir.join("SKILL.md"),
        "# Progressive Skill\n\nChanged after source map.\n",
    );

    let stale = Command::new(bin())
        .arg("source")
        .arg("stale")
        .arg(map_dir.join("source-map.json"))
        .arg("--json")
        .output()
        .unwrap();
    assert_failure(&stale);
    assert_eq!(json_stdout(&stale)["ok"], false);

    let stale_import = Command::new(bin())
        .arg("import-skill")
        .arg(&skill_dir)
        .arg("--out")
        .arg(dir.path().join("stale").join("skill.spec.yml"))
        .arg("--source-map")
        .arg(map_dir.join("source-map.json"))
        .output()
        .unwrap();
    assert_failure(&stale_import);
    assert!(stderr(&stale_import).contains("source map"));
    assert!(stderr(&stale_import).contains("stale"));
}

#[test]
fn source_map_prefers_existing_local_path_over_github_shorthand() {
    let dir = TempDir::new("source-map-local-shorthand");
    let skill_dir = dir.path().join("owner").join("repo");
    let map_dir = dir.path().join("source-map");
    write_file(
        &skill_dir.join("SKILL.md"),
        "# Local Skill\n\nAlways treat existing owner/repo paths as local.\n",
    );

    let map = Command::new(bin())
        .current_dir(dir.path())
        .arg("source")
        .arg("map")
        .arg("owner/repo")
        .arg("--out")
        .arg(&map_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&map);
    let report = json_stdout(&map);
    assert_eq!(report["files"], 1);
    assert!(report.get("staged_from").is_none());
    assert!(map_dir.join("source-map.json").is_file());
}

#[test]
fn source_map_chunks_oversized_markdown_without_full_ast() {
    let dir = TempDir::new("source-map-chunked");
    let skill_dir = dir.path().join("source-skill");
    let map_dir = dir.path().join("source-map");
    let mut body = String::from(
        r#"---
name: large-skill
description: Use when a large skill needs chunked source mapping.
---

# Large Skill

Always inspect the generated source map.

See [reference](reference.md).

```python
import pypdf
```

"#,
    );
    for index in 0..7000 {
        body.push_str(&format!(
            "Paragraph {index} must preserve review handles and dependency context.\n\n"
        ));
    }
    write_file(&skill_dir.join("SKILL.md"), &body);
    write_file(
        &skill_dir.join("reference.md"),
        "# Reference\n\nNever skip this.\n",
    );

    let map = Command::new(bin())
        .arg("source")
        .arg("map")
        .arg(&skill_dir)
        .arg("--out")
        .arg(&map_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&map);

    let nodes = Command::new(bin())
        .arg("source")
        .arg("query")
        .arg(map_dir.join("source-map.json"))
        .arg("nodes")
        .arg("--view")
        .arg("summary")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&nodes);
    let nodes = json_stdout(&nodes);
    let nodes = nodes.as_array().unwrap();
    assert!(nodes.iter().any(|node| node["kind"] == "paragraph_chunk"));
    assert!(nodes.iter().any(|node| node["kind"] == "code"));

    let deps = Command::new(bin())
        .arg("source")
        .arg("query")
        .arg(map_dir.join("source-map.json"))
        .arg("dependencies")
        .arg("--json")
        .output()
        .unwrap();
    assert_success(&deps);
    let deps_text = serde_json::to_string(&json_stdout(&deps)).unwrap();
    assert!(deps_text.contains("pypdf"));
}
