set shell := ["bash", "-uc"]

packages := "skillspec-core skillspec-runtime skillspec-doctor skillspec-authoring skillspec-harness skillspec-workspace skillspec"

# Show the local crate hierarchy and dependency direction.
packages:
    @printf "%s\n" \
      "1. skillspec-core" \
      "2. skillspec-runtime -> skillspec-core" \
      "3. skillspec-doctor -> skillspec-core" \
      "4. skillspec-authoring -> skillspec-core, skillspec-runtime, skillspec-doctor" \
      "5. skillspec-harness -> skillspec-core, skillspec-runtime" \
      "6. skillspec-workspace -> skillspec-core, skillspec-doctor, skillspec-authoring, skillspec-harness" \
      "7. skillspec CLI -> all internal crates"

# List detected harness skill roots.
install-targets:
    skillspec install targets

# Build every workspace package in debug mode.
build-debug:
    cargo build --workspace

# Build every workspace package in release mode.
build-release:
    cargo build --workspace --release

# Build one workspace package in debug mode.
build-package package:
    cargo build -p {{package}}

# Build one workspace package in release mode.
build-package-release package:
    cargo build -p {{package}} --release

# Install the local CLI with Cargo's debug profile.
install-debug:
    cargo install --path crates/skillspec-cli --debug --force

# Install the local CLI with Cargo's release profile.
install-release:
    cargo install --path crates/skillspec-cli --force

# Install the repo's SkillSpec skill into one harness target.
install-skill target:
    skillspec install skill skills/skillspec --target {{target}} --retire-existing

# Install the repo's SkillSpec skill into every detected harness root.
install-skill-all:
    skillspec install skill skills/skillspec --all-detected --retire-existing

# Build and install a debug CLI, then install the SkillSpec skill everywhere detected.
dev-install-all: build-debug install-debug install-skill-all

# Formatting must be stable before a branch leaves the machine.
fmt-check:
    cargo fmt --all --check

# Type-check the full workspace, including tests and examples.
check:
    cargo check --workspace --all-targets

# Treat lint warnings as failures.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run the workspace test suite.
test:
    cargo test --workspace --all-targets

# Verify crate packaging boundaries without publishing or creating release artifacts.
package-list:
    for package in {{packages}}; do \
      cargo package --list -p "$package" --allow-dirty > /dev/null; \
    done

# Local CI equivalent before pushing a branch.
preflight: fmt-check check clippy test package-list
