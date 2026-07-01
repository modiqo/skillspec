use skillspec_harness_lab::HarnessLab;
use std::path::{Path, PathBuf};

pub const INPUT: &str =
    "run a local command and remember the result: printf skillspec-durable-proof";
pub const PROOF_MARKER: &str = "skillspec-durable-proof";

pub struct RoteExecFixture {
    pub lab: HarnessLab,
    pub spec: PathBuf,
    pub input: &'static str,
}

pub fn rote_exec_fixture(name: &str) -> RoteExecFixture {
    let lab = HarnessLab::new(name);
    let spec = repo_root().join("examples/durable-executor/skill.spec.yml");
    assert!(
        spec.is_file(),
        "expected durable-executor example spec at {}",
        spec.display()
    );
    write_rote_shell_skill(&lab);
    RoteExecFixture {
        lab,
        spec,
        input: INPUT,
    }
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("harness lab crate lives under crates/")
        .to_path_buf()
}

pub fn latest_run_dir(trace_dir: &Path) -> PathBuf {
    std::fs::read_dir(trace_dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            path.is_dir().then_some(path)
        })
        .max()
        .expect("expected trace run directory")
}

fn write_rote_shell_skill(lab: &HarnessLab) {
    let skill = r#"---
name: rote-shell
description: Use for CLI and shell work through rote exec with captured stdout, stderr, files, workspace memory, and replayable process evidence.
disable-model-invocation: true
---
# rote-shell

Use rote for shell and CLI work when the result should become workspace memory.

For one-shot commands, use:

```bash
rote exec -- <program> [args...]
```

CLI work must happen inside a named rote workspace.
"#;
    lab.write_skill(&lab.agents_root(), "rote-shell", skill, None);
}
