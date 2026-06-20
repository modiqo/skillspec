---
name: browser-research-skillspec
description: Use the colocated SkillSpec contract for browser research tasks.
---

# Browser Research

This skill is a thin loader for `./skill.spec.yml`. The spec is the source of
truth for routes, rules, dependencies, elicitations, tests, and trace
requirements.

## Runtime Contract

1. Load `./skill.spec.yml` before taking task actions.
2. When the `skillspec` CLI is available, run:

   ```sh
   skillspec decide ./skill.spec.yml --input='<user task>' --trace-dir "${PWD}/.skillspec/traces"
   ```

3. Preserve the emitted trace directory.
4. Follow the selected route, matched rules, forbids, elicitations, route order,
   dependencies, and post-success obligations from the spec.
5. If the CLI is unavailable, read `skill.spec.yml` directly and apply the same
   contract manually.

## Quick Checks

```sh
skillspec validate ./skill.spec.yml
skillspec imports check ./skill.spec.yml
skillspec test ./skill.spec.yml
skillspec deps check ./skill.spec.yml
```
