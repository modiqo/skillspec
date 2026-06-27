# SkillSpec Plugin

This plugin installs the `skillspec` skill for agent harnesses that support
plugin marketplaces.

The skill is a thin trampoline. It points the agent at the colocated
`skill.spec.yml` and asks the SkillSpec CLI for route, phase, progress, resume,
and alignment guidance.

The CLI must be installed separately:

```sh
cargo install skillspec
skillspec --version
```
