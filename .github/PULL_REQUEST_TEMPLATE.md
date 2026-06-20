## Summary

- 

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets`
- [ ] `cargo build --workspace`
- [ ] `skillspec validate` / `skillspec test` / `skillspec deps check` for changed examples

## Notes

- Schema changes update `spec/skill.spec.schema.json`, Rust models, docs, and examples together.
- Compiler or importer output changes update the relevant files in `fixtures/golden/`.
