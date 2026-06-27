# Plugin Marketplace Install

SkillSpec has two install surfaces, and they serve different jobs.

The public harness install path is a plugin marketplace path. It lets Claude and
Codex install the `skillspec` trampoline through their native plugin systems.
That is the path users should see first.

The local `skillspec install skill` command remains useful, but it is a
developer and generated-skill path. It copies a prepared skill folder into a
detected harness root. It is still the right command for testing local drafts,
installing generated skills, and exercising workspace install logic, but it
should not be presented as the primary public install path for SkillSpec itself.

## Repository Layout

The repository is also a plugin marketplace:

```text
.claude-plugin/
  marketplace.json
.agents/
  plugins/
    marketplace.json
plugins/
  skillspec/
    .claude-plugin/
      plugin.json
    .codex-plugin/
      plugin.json
    INDEX.md
    skills/
      skillspec/
        SKILL.md
        skill.spec.yml
        source/
          SKILL_md.old
```

Claude reads `.claude-plugin/marketplace.json`. Codex reads
`.agents/plugins/marketplace.json`. Both catalogs point to
`plugins/skillspec`, which contains the product-specific plugin metadata and the
actual `skills/skillspec` payload.

The canonical source skill remains `skills/skillspec/`. The plugin payload is a
release package mirror of that source. Before release, keep these files aligned:

```text
skills/skillspec/SKILL.md
plugins/skillspec/skills/skillspec/SKILL.md

skills/skillspec/skill.spec.yml
plugins/skillspec/skills/skillspec/skill.spec.yml

skills/skillspec/source/SKILL_md.old
plugins/skillspec/skills/skillspec/source/SKILL_md.old
```

## Public Install Flow

Install the CLI:

```sh
cargo install skillspec
skillspec --version
```

For unreleased `main`:

```sh
cargo install --git https://github.com/modiqo/skillspec --package skillspec --force
skillspec --version
```

Install into Claude Code:

```sh
claude plugin marketplace add modiqo/skillspec --sparse .claude-plugin plugins/skillspec
claude plugin install skillspec@skillspec
claude plugin list
```

Claude installs the plugin enabled by default in current Claude Code builds. If
`claude plugin list` shows it disabled, run:

```sh
claude plugin enable skillspec
```

Install into Codex:

```sh
codex plugin marketplace add modiqo/skillspec --ref main --sparse .agents --sparse plugins/skillspec
codex plugin add skillspec@skillspec
```

Codex does not have a separate plugin enable command. The top-level `--enable`
flag in Codex CLI help is a feature flag override, not plugin activation.

## Local Development Fallback

For local development and unreleased package testing:

```sh
cargo install --path crates/skillspec-cli --force
skillspec install skill skills/skillspec --target codex --retire-existing
skillspec install skill skills/skillspec --target agents --retire-existing
skillspec install skill skills/skillspec --target claude-local --retire-existing
```

Use this path when testing a local checkout before marketplace packaging or when
installing generated SkillSpec-backed skills that are not plugins.

## Trampoline Contract

The plugin-installed `SKILL.md` must stay thin. It should not duplicate the
SkillSpec behavior contract. It should:

- resolve the directory that contains the loaded `SKILL.md`;
- treat `<skill_dir>/skill.spec.yml` as the source of truth;
- call `skillspec run-loop <skill_dir>/skill.spec.yml --guide agent`;
- resume with `skillspec run-loop <skill_dir>/skill.spec.yml --resume <run_dir> --guide agent`;
- ask the user to install the CLI if `skillspec` is missing;
- fall back to reading `<skill_dir>/skill.spec.yml` only when the CLI exists but
  guide mode fails.

This keeps the plugin payload small while preserving deterministic route,
progress, resume, and alignment behavior in the CLI.

## Validation Gate

Before publishing or tagging a plugin release:

```sh
claude plugin validate . --strict
claude plugin validate plugins/skillspec --strict
codex plugin marketplace add ./ --json
codex plugin list --json
```

When testing install behavior against the real user environment, use a local
checkout first:

```sh
claude plugin marketplace add ./ --scope user
claude plugin install skillspec@skillspec --scope user
claude plugin list

codex plugin marketplace add ./ --json
codex plugin add skillspec@skillspec --json
codex plugin list --json
```

If the marketplace was already configured, update or remove the old source
before re-adding it so the harness does not install a stale snapshot.

## Release Notes

The plugin version should track the CLI crate version in
`crates/skillspec-cli/Cargo.toml`. If the CLI and plugin are released
independently later, the plugin manifest should declare the minimum compatible
CLI version in its description or a supported metadata field once the harness
schema exposes one.

The release gate must verify and publish the crate package:

```sh
cargo package --locked -p skillspec
cargo publish --locked --dry-run -p skillspec
cargo publish --locked -p skillspec
```

CI runs the dry-run package verification. Tagged releases publish to crates.io
through the `CARGO_REGISTRY_TOKEN` repository secret.
