# SkillSpec CLI

`skillspec` is the command-line interface for SkillSpec: a structured contract
format for agent skills.

It validates, tests, imports, compiles, routes, runs guided loops, records
progress, and prints alignment reports for SkillSpec-backed skills.

This crate is published so users can install the CLI with Cargo. Its Rust
modules are implementation details for now, not a stable library API.

Install the prebuilt binary from the latest GitHub release:

```sh
curl -fsSL https://raw.githubusercontent.com/modiqo/skillspec/main/install.sh | sh
skillspec --version
```

Binary releases are available for macOS, Linux x86_64, and Windows x86_64.

Install from crates.io:

```sh
cargo install skillspec
skillspec --version
```

Then install the SkillSpec plugin into your agent harness from the repository
marketplace. See the project README for Claude and Codex plugin install
commands:

<https://github.com/modiqo/skillspec>

Local development install:

```sh
cargo install --path crates/skillspec-cli --force
```

License: MIT OR Apache-2.0.
