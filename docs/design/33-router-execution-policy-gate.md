# Router Execution Policy Gate

Router mode has two different jobs that must stay separate:

1. Select the logical skill, if the request clearly matches one.
2. Decide whether the selected work must run through a durable rote substrate.

The router is still not the execution engine. It returns a decision and a policy
that the first-hop loader must obey before reading domain skill instructions or
using tools.

## YAML Surfaces

This feature touches three SkillSpec YAML surfaces:

- `crates/skillspec-harness/src/router_lifecycle/template.rs` renders the
  installed `skill-router/skill.spec.yml`. This is the runtime contract used
  after `skillspec router install`, `enable`, or `update`.
- `examples/skill-router/skill.spec.yml` is the hand-maintained example of that
  runtime contract.
- `skills/skillspec/skill.spec.yml` is the standard SkillSpec self-skill. It
  teaches `/skillspec install router`, `/skillspec router update`, and operator
  explanations. It is not the runtime router contract, but it must describe the
  same behavior so installs and docs do not drift.

Do not edit installed copies under `.agents`, `.codex`, `.claude`, or
`.claude/skills/skillspec` directly. They are generated or local installs.

## Ladder

`skillspec route` emits `execution_policy` when the request needs a managed rote
substrate:

| Kind | Substrate | Required skills | Use case |
| --- | --- | --- | --- |
| `service_api` | `rote_adapter` | `durable-executor` plus a service-specific rote skill or `rote` | `connect to X`, `fetch from X`, `fetch information from X`, service/vendor/SaaS/API work |
| `local_action` | `rote_shell` | `durable-executor`, `rote-shell` | shell, CLI, scripts, builds, tests, package installs, git, local file mutation |
| `browse` | `rote_browse` | `durable-executor`, `rote-browse` | browser, web page, dashboard, login, authenticated session, navigation, click, URL, web search |

When the policy is active, route can choose the substrate skill even if lexical
confidence alone would be too low. For example, a browse request should select
`rote-browse` when `durable-executor` and `rote-browse` are active.

When required substrate is unavailable, route returns:

```json
{
  "decision": "bypass",
  "bypass_reason": "required_execution_substrate_unavailable",
  "execution_policy": {
    "availability": "unavailable",
    "repair": "install or enable required execution skills: durable-executor, rote-browse"
  }
}
```

The harness must not silently fall back to direct Chrome, Playwright, shell,
HTTP, SDK, or web-search tooling for a request that matched one of these
policies. It either follows the active substrate or reports the repair.

## Browse Defaults

Browse policy is intentionally strict:

- forbid direct browser tooling;
- forbid direct Chrome/Playwright;
- forbid REPL-based browser control;
- forbid direct web search;
- prefer the current authenticated browser session;
- fall back to headless only when no active session is available.

The reason is practical: many useful pages require authentication or a
post-authenticated state. Headless should be a fallback, not the default.

## Activation Anchor Gate

The router also applies an activation-anchor gate before accepting normal
lexical matches. A broad skill cannot win only because the query contains
generic words such as `docs`, `document`, `create`, `skill`, `router`, or
`rote`.

The selected candidate must match its full name or a non-generic name anchor.
Examples:

- `fetch information from Linear API` can activate `rote-linear`.
- `extract text from a pdf` can activate `pdf`.
- `create a Stripe adapter` can activate adapter authoring because `adapter` is
  an anchor.
- `ok, do this and document it in docs/design` should bypass rather than
  selecting `rote-adapter-create`.

## Non-Goals

- The router does not execute rote.
- The router does not create adapters.
- The router does not infer credentials or service auth state.
- The router does not make unavailable durable substrates optional for matched
  policy work.

These remain runtime, durable-executor, or rote responsibilities.

## Regression Coverage

Core unit tests cover:

- generic docs prompts bypassing broad rote-adapter false positives;
- browse selecting `rote-browse` when active;
- local action selecting `rote-shell` when active;
- service/API selecting a service-specific rote skill ahead of adapter creation;
- unavailable substrate returning
  `required_execution_substrate_unavailable`.
