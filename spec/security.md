# Security

SkillSpec can steer agents toward tools, commands, browsers, and credentials.
That makes trust explicit.

## V0 Rules

- A SkillSpec must not silently request credentials.
- A SkillSpec must not silently install global dependencies.
- A SkillSpec must not silently attach to a personal browser.
- A SkillSpec must not weaken a parent safety rule in v0 because v0 has no
  inheritance.
- A SkillSpec must mark uncertain imported behavior with `review_required`.

## Trust Metadata

Recommended shape:

```yaml
security:
  can_request_browser_attach: true
  can_request_credentials: false
  can_install_dependencies: false
  can_run_destructive_commands: false
  requires_human_review: true
```

## Threat Model

A malicious or careless spec could steer an agent to:

- choose an unsafe route
- skip user consent
- use native search instead of a trusted browser route
- run shell commands outside the intended scope
- request or expose credentials

The v0 answer is transparency, validation, scenario tests, and conservative
defaults.

