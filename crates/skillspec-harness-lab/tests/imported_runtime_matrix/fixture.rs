use skillspec_harness_lab::{assert_success, HarnessLab};
use std::path::{Path, PathBuf};

pub struct ReviewedImportFixture {
    pub lab: HarnessLab,
    pub package_dir: PathBuf,
    pub spec_path: PathBuf,
}

pub fn reviewed_import_fixture(name: &str) -> ReviewedImportFixture {
    let lab = HarnessLab::new(name);
    let source_root = lab.root().join("source");
    let source = lab.write_skill(&source_root, "runtime-skill", &runtime_skill_md(), None);
    lab.write_file(
        &source.join("reference.md"),
        "# Runtime Reference\n\nKeep verification evidence explicit.\n",
    );

    let package_dir = lab.root().join("reviewed-package");
    let spec_path = package_dir.join("skill.spec.yml");
    let import = lab
        .command()
        .arg("import-skill")
        .arg(&source)
        .arg("--out")
        .arg(&spec_path)
        .output()
        .unwrap();
    assert_success(&import);
    apply_reviewed_runtime_contract(&lab, &spec_path);

    let validate = lab
        .command()
        .arg("validate")
        .arg(&spec_path)
        .output()
        .unwrap();
    assert_success(&validate);
    let test = lab.command().arg("test").arg(&spec_path).output().unwrap();
    assert_success(&test);

    ReviewedImportFixture {
        lab,
        package_dir,
        spec_path,
    }
}

fn runtime_skill_md() -> String {
    r#"---
name: runtime-skill
description: Verify and install a SkillSpec-backed runtime fixture.
---
# Runtime Skill

Always run doctor before trusting the import.
Always validate the generated contract before installing it.
Always report proof and token savings in the final answer.

See [reference](reference.md).

```bash
skillspec validate skill.spec.yml
```
"#
    .to_owned()
}

fn apply_reviewed_runtime_contract(lab: &HarnessLab, spec_path: &Path) {
    let mut spec = std::fs::read_to_string(spec_path).unwrap();
    spec = spec.replace(
        "routes: []",
        r#"routes:
- id: verify_skill
  label: Verify imported skill
  checks: [run_validation]
  execution_plan:
    mode: ordered
    phases:
    - id: assess
      owner_skill: skillspec
      requires: [doctor_report]
    - id: install
      owner_skill: skillspec
      requires: [installed_loader]"#,
    );
    spec = spec.replace(
        "rules: []",
        r#"rules:
- id: verify_request
  when:
    user_says_any: ["verify", "install"]
  prefer: verify_skill"#,
    );
    spec = spec.replace(
        "trace: null",
        r#"trace:
  mode: event_log
  required: true
  record:
  - input_received
  - rule_matched
  - route_selected
  - outcome_recorded"#,
    );
    spec = spec.replace(
        "commands:\n",
        r#"commands:
  summarize_evidence:
    description: Summarize the runtime proof for the user.
    template: echo summary
    safety: read_only
"#,
    );
    spec = spec.replace(
        "tests: []",
        r#"tests:
- name: verify request selects imported runtime route
  input: verify and install this skill
  expect:
    route: verify_skill
    plan_phases: [assess, install]"#,
    );
    lab.write_file(spec_path, &spec);
}
