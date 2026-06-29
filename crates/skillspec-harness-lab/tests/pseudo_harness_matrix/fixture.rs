use skillspec_harness_lab::HarnessLab;
use std::path::PathBuf;

pub struct PseudoHarnessFixture {
    pub lab: HarnessLab,
    pub index: PathBuf,
    pub manifest: PathBuf,
    pub config: PathBuf,
}

pub fn pseudo_fixture(name: &str) -> PseudoHarnessFixture {
    let lab = HarnessLab::new(name);
    let index = lab.skillspec_home().join("router/skill-index.sqlite");
    let manifest = lab.skillspec_home().join("router/visibility-manifest.json");
    let config = lab.skillspec_home().join("router/config.json");
    write_base_skills(&lab);

    PseudoHarnessFixture {
        lab,
        index,
        manifest,
        config,
    }
}

pub fn write_out_of_band_markdown_skill(lab: &HarnessLab) {
    lab.write_skill(&lab.codex_root(), "markdown", &markdown_skill(), None);
}

pub fn write_duplicate_durable_roots(lab: &HarnessLab) {
    lab.write_skill(
        &lab.codex_root(),
        "durable-executor",
        &durable_executor_skill(),
        None,
    );
    lab.write_skill(
        &lab.claude_root(),
        "durable-executor",
        &durable_executor_skill(),
        None,
    );
}

pub fn write_imported_widget_skill(lab: &HarnessLab) {
    lab.write_skill(
        &lab.codex_root(),
        "widget-flow",
        &widget_loader_skill(),
        Some(&widget_spec()),
    );
}

fn write_base_skills(lab: &HarnessLab) {
    lab.write_skill(&lab.agents_root(), "pdf", &pdf_skill(), None);
    lab.write_skill(
        &lab.agents_root(),
        "durable-executor",
        &durable_executor_skill(),
        None,
    );
    lab.write_skill(&lab.codex_root(), "csv", &csv_skill(), None);
    lab.write_skill(&lab.claude_root(), "notes", &notes_skill(), None);
}

fn pdf_skill() -> String {
    r#"---
name: pdf
description: Use when extracting PDF text, tables, and images. Do not use for notes.
---
# PDF
"#
    .to_owned()
}

fn durable_executor_skill() -> String {
    r#"---
name: durable-executor
description: Use as the durable execution first-hop for shell, CLI, and tool-backed requests that need trace, evidence, and alignment.
---
# Durable Executor
"#
    .to_owned()
}

fn csv_skill() -> String {
    r#"---
name: csv
description: Use when working with CSV files and spreadsheet exports. Do not use for notes.
---
# CSV
"#
    .to_owned()
}

fn notes_skill() -> String {
    r#"---
name: notes
description: Use when taking structured notes and summarizing meeting action items. Do not use for PDF extraction.
---
# Notes
"#
    .to_owned()
}

fn markdown_skill() -> String {
    r#"---
name: markdown
description: Use when editing markdown documents and README files. Do not use for PDF extraction.
---
# Markdown
"#
    .to_owned()
}

fn widget_loader_skill() -> String {
    r#"---
name: widget-flow
description: Use for widget-flow fixture tasks that need the SkillSpec-backed trampoline.
---
# Widget Flow

This is a SkillSpec-backed trampoline. Ask the SkillSpec CLI for the current
route, phase, and tool boundary from `skill.spec.yml` before executing.
"#
    .to_owned()
}

fn widget_spec() -> String {
    r#"schema: skillspec/v0
id: widget.flow
name: widget-flow
version: 0.1.0
description: Fixture contract for widget-flow pseudo-harness activation.
routes:
  - id: run_widget
    match:
      any:
        - widget-flow
        - widget fixture
phases:
  - id: execute
    instructions: Run the widget fixture and report proof.
"#
    .to_owned()
}
