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
      "7. skillspec CLI -> all internal crates" \
      "test-only. skillspec-harness-lab -> sandbox harness test helpers"

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
    cargo check --locked --workspace --all-targets

# Treat lint warnings as failures.
clippy:
    cargo clippy --locked --workspace --all-targets -- -D warnings

# Run the workspace test suite.
test:
    cargo test --locked --workspace --all-targets

# Run only the controlled harness lab tests.
harness-lab-test:
    cargo test --locked -p skillspec-harness-lab

# Verify crate packaging boundaries without requiring already-published sibling crates.
package-list:
    for package in {{packages}}; do \
      cargo package --locked --list -p "$package" --allow-dirty > /dev/null; \
    done

# Validate every example spec.
examples-validate:
    find examples -name '*.yml' -print0 | sort -z | xargs -0 -n1 cargo run --locked -p skillspec -- validate

# Run every example scenario test.
examples-test:
    find examples -name '*.yml' -print0 | sort -z | xargs -0 -n1 cargo run --locked -p skillspec -- test

# Check every example dependency ledger.
examples-deps:
    find examples -name '*.yml' -print0 | sort -z | xargs -0 -n1 cargo run --locked -p skillspec -- deps check

# Validate valid conformance fixtures and reject invalid ones.
conformance:
    for spec in $(find conformance/valid -name '*.yml' | sort); do \
      cargo run --locked -p skillspec -- validate "$spec"; \
    done
    for spec in $(find conformance/invalid -name '*.yml' | sort); do \
      if cargo run --locked -p skillspec -- validate "$spec"; then \
        echo "invalid conformance fixture unexpectedly passed: $spec" >&2; \
        exit 1; \
      fi; \
    done

# Local CI equivalent before pushing a branch.
preflight: fmt-check check clippy test package-list examples-validate examples-test examples-deps conformance
