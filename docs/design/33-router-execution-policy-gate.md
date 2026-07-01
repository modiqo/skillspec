# Router Provider Neutrality

Router mode has one job: select the logical skill, if the request clearly
matches one. It must stay universal and open-source friendly. It must not
hardcode a vendor, adapter system, browser system, shell runner, recorder, or
durable execution substrate.

## Boundary

`skillspec route` returns:

- `use_skill` with a selected skill path when the match is clear;
- `bypass` when no skill should be loaded;
- `ambiguous` when the router cannot choose safely.

That is the router boundary. Execution policy belongs after selection:

- the selected skill's own `skill.spec.yml` owns route phases, tool boundaries,
  forbids, checks, and proof;
- durable-executor owns the durable execution envelope when it is explicitly or
  implicitly active;
- future provider/capability routing must be declarative skill metadata, not
  hardcoded router knowledge.

## YAML Surfaces

Router behavior is described in three SkillSpec YAML surfaces:

- `crates/skillspec-harness/src/router_lifecycle/template.rs` renders the
  installed `skill-router/skill.spec.yml`.
- `examples/skill-router/skill.spec.yml` is the maintained example of that
  runtime contract.
- `skills/skillspec/skill.spec.yml` teaches `/skillspec install router`,
  `/skillspec router update`, and operator explanations.

These surfaces must describe the same provider-neutral behavior. Do not edit
installed copies under `.agents`, `.codex`, `.claude`, or
`.claude/skills/skillspec` directly; they are generated or local installs.

## Activation Anchor Gate

The router applies an activation-anchor gate before accepting normal lexical
matches. A broad skill cannot win only because the query contains generic words
such as `docs`, `document`, `create`, `skill`, or `router`.

The selected candidate must match its full name or a non-generic name anchor.
Examples:

- `extract text from a pdf` can activate `pdf`.
- `create a Stripe adapter` can activate adapter authoring because `adapter` is
  an anchor.
- `ok, do this and document it in docs/design` should bypass rather than
  selecting a broad creation skill.

## Non-Goals

- The router does not execute tools.
- The router does not create adapters.
- The router does not infer credentials or service auth state.
- The router does not translate browser, shell, API, or SaaS intent into a
  provider-specific runtime.
- The router does not make durable execution decisions except to surface the
  existing direct-versus-durable elicitation when a skill is selected and the
  caller has not already chosen.

## Future Provider Routing

If SkillSpec later supports a portable provider/capability layer, the router can
consume declarative metadata from skills or specs. That design must keep the
router generic:

- providers advertise capabilities in metadata;
- skills declare requirements or preferences;
- the router ranks compatible installed skills without naming a specific
  provider in harness code;
- unavailable providers produce a provider-neutral bypass or repair state.

## Regression Coverage

Core unit tests and pseudo-harness report cards cover:

- generic docs prompts bypassing broad false positives;
- duplicate physical installs collapsing to one logical selection;
- route decisions loading a domain skill only for `use_skill`;
- `bypass` and `ambiguous` never silently loading a candidate.
