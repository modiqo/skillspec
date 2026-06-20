# Discussion Guide

Use GitHub Discussions for early design feedback and adoption work that is not ready to be a bug report or PR.

Recommended categories:

- Spec feedback: grammar, field names, strictness, and extension surfaces.
- Skill ports: real skills people tried to represent with SkillSpec.
- Harness/compiler targets: Codex, Claude Code, Markdown, and future targets.
- Security model: prompt injection, dependency safety, permissions, and traces.
- Conformance tests: valid and invalid fixtures, expected CLI behavior, and compatibility.
- Examples wanted: domains where prose skills are common and hard to verify.

## Specific Ask

Port one real skill and tell us what the spec cannot express.

Useful discussion posts include:

- A link to the original skill.
- The generated or hand-written `skill.spec.yml`.
- The harness you want to run it in.
- Decisions that were hard to encode.
- Dependencies that were ambiguous.
- Scenario tests that would prove the port works.
- Compiler output that felt too thin, too verbose, or misleading.
