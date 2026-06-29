use skillspec_harness_lab::{assert_success, json_stdout, HarnessLab};
use std::path::Path;
use std::process::Output;

pub fn import_skill_output(lab: &HarnessLab, source: &Path, out: &Path) -> Output {
    lab.command()
        .arg("import-skill")
        .arg(source)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

pub fn import_skill_with_source_map_output(
    lab: &HarnessLab,
    source: &Path,
    out: &Path,
    source_map: &Path,
) -> Output {
    lab.command()
        .arg("import-skill")
        .arg(source)
        .arg("--out")
        .arg(out)
        .arg("--source-map")
        .arg(source_map)
        .output()
        .unwrap()
}

pub fn validate_output(lab: &HarnessLab, spec: &Path) -> Output {
    lab.command().arg("validate").arg(spec).output().unwrap()
}

pub fn deps_check_json(lab: &HarnessLab, spec: &Path) -> serde_json::Value {
    let output = lab
        .command()
        .arg("deps")
        .arg("check")
        .arg(spec)
        .output()
        .unwrap();
    assert_success(&output);
    json_stdout(&output)
}

pub fn source_map_output(lab: &HarnessLab, source: &Path, out: &Path) -> Output {
    lab.command()
        .arg("source")
        .arg("map")
        .arg(source)
        .arg("--out")
        .arg(out)
        .arg("--json")
        .output()
        .unwrap()
}

pub fn workspace_map_output(lab: &HarnessLab, source: &Path, manifest: &Path) -> Output {
    lab.command()
        .arg("workspace")
        .arg("map")
        .arg(source)
        .arg("--out")
        .arg(manifest)
        .arg("--json")
        .output()
        .unwrap()
}

pub fn workspace_import_output(lab: &HarnessLab, manifest: &Path, build: &Path) -> Output {
    lab.command()
        .arg("workspace")
        .arg("import")
        .arg(manifest)
        .arg("--out")
        .arg(build)
        .arg("--json")
        .output()
        .unwrap()
}

pub fn importable_skill_md() -> String {
    r#"---
name: review-skill
description: Review one file and report risks.
---
# Review Skill

Always validate inputs before writing files.

See [reference](reference.md).

```bash
echo "hello"
```
"#
    .to_owned()
}

pub fn source_mapped_skill_md(line: &str) -> String {
    format!(
        r#"---
name: mapped-skill
description: Use when source map freshness is required.
---
# Mapped Skill

{line}

```python
import pypdf
```
"#
    )
}

pub fn read_file(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}
