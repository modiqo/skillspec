# SkillSpec Conformance Fixtures

This folder contains small fixtures that define expected validation behavior for SkillSpec v0.

- `valid/` contains specs that must pass `skillspec validate`.
- `invalid/` contains specs that must fail `skillspec validate`.

The integration test `conformance_fixtures_have_expected_validation_outcomes` runs these through the public CLI. Add fixtures here when changing grammar, strictness, or validation behavior.

Run locally:

```sh
cargo test --workspace --all-targets conformance_fixtures_have_expected_validation_outcomes
```
