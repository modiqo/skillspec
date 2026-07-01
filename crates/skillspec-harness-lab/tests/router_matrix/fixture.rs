use skillspec_harness_lab::HarnessLab;
use std::path::PathBuf;

pub struct RouterFixture {
    pub lab: HarnessLab,
    pub index: PathBuf,
    pub manifest: PathBuf,
    pub config: PathBuf,
    pub codex_hooks: PathBuf,
    pub claude_settings: PathBuf,
}

pub fn router_fixture(name: &str) -> RouterFixture {
    let lab = HarnessLab::new(name);
    let index = lab.skillspec_home().join("router/skill-index.sqlite");
    let manifest = lab.skillspec_home().join("router/visibility-manifest.json");
    let config = lab.skillspec_home().join("router/config.json");
    let codex_hooks = lab.home().join(".codex/hooks.json");
    let claude_settings = lab.project().join(".claude/settings.json");

    lab.write_file(
        &codex_hooks,
        r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"echo keep-codex"}]}]}}"#,
    );
    lab.write_file(
        &claude_settings,
        r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"echo keep-claude"}]}]}}"#,
    );

    write_initial_skills(&lab);

    RouterFixture {
        lab,
        index,
        manifest,
        config,
        codex_hooks,
        claude_settings,
    }
}

pub fn write_initial_skills(lab: &HarnessLab) {
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

pub fn write_out_of_band_skill(lab: &HarnessLab) {
    lab.write_skill(&lab.codex_root(), "markdown", &markdown_skill(), None);
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
description: Use as the durable execution first-hop for tool-backed requests that need trace, evidence, and alignment.
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
