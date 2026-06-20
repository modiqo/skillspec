# Before And After: Browser Skill

This example shows the intended migration shape:

- `SKILL.before.md` is the original prose-only skill.
- `skill.spec.yml` is the structured behavior contract.
- `SKILL.after.md` is the thin harness loader generated or maintained around the spec.

The important change is not that prose disappears. The route decision and forbidden substitution become testable:

```sh
skillspec validate examples/before-after/skill.spec.yml
skillspec test examples/before-after/skill.spec.yml
skillspec compile examples/before-after/skill.spec.yml --target markdown
```
