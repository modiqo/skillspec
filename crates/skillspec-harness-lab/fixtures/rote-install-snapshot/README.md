# Rote Install Snapshot

These fixture skills are copied from the real rote skill installer output. They
are intentionally not hand-written stubs.

Refresh command:

```bash
rote install skill --path /private/tmp/skillspec-rote-install-inspect --target codex --copy --force
```

Then copy only:

- `rote-shell/SKILL.md`
- `rote-browse/SKILL.md`

The pseudo-harness tests use this snapshot so CI can validate SkillSpec router
selection without requiring a separate `rote` binary to be installed.
