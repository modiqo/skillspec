pub fn basic_skill_md(name: &str) -> String {
    format!(
        r#"---
name: {name}
description: Harness lab fixture skill.
---
# {name}

Use this fixture only for controlled harness lab tests.
"#
    )
}

pub fn basic_skill_spec(id: &str, title: &str) -> String {
    format!(
        r#"schema: skillspec/v0
id: {id}
title: {title}
description: Harness lab fixture contract.
routes:
  - id: default
    label: Default
"#
    )
}
