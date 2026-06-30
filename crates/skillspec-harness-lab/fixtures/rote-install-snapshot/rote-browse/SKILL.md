---
name: rote-browse
description: "Use for browser automation through rote: browsing websites, attaching to active Chrome or Edge sessions, Gmail or other logged-in sites, headed/headless exploration, snapshots, readiness waits, page slices, ref rebasing, and crystallizing browser flows. Prefer rote browser primitives over direct Playwright calls."
---

# rote-browse

Use rote for browser work before calling Playwright or another browser tool
directly. Rote keeps browser observations in the workspace, gives
them stable `@` addresses, stores page/slice/ref state, and turns exploration
into replayable flows.

## Activation Priority

When this skill is invoked explicitly (`/rote-browse`, "use rote-browse", or a
direct browser/navigation request), the user's browser intent is already clear.
Do not start with generic `rote flow search`, do not execute a saved non-browser
flow first, and do not silently choose headless. Activate the browser launch
path first.

Do not replace browser work with native web search, WebFetch, raw Playwright,
`curl`, or shell scraping. If the user asks to browse, open, inspect, snapshot,
click, type, or extract from pages such as public social profiles, use rote
browser primitives to observe the page. Native search may only discover
candidate URLs when the user asked for search/discovery or no URL can be found
from existing adapter, CLI, or browser evidence.

Load the browser guidance when you need more detail:

```bash
rote guidance browser essential
```

For live workspace state, check:

```bash
rote inventory
rote config browser show
rote vars --json
```

## Harness Permissions And Shell Friction

On a fresh Claude/Codex-style harness, clear rote browser prompt friction before
the first navigation. `rote install skill --provider all --personal --force`
installs the supported rules for the local harnesses.

For Claude, the intended permission shape is:

```json
{
  "permissions": {
    "allow": [
      "Bash(rote:*)",
      "Bash(cd:*)",
      "Bash(rg:*)",
      "Bash(grep:*)",
      "Bash(head:*)",
      "Bash(tail:*)",
      "Bash(wc:*)"
    ],
    "additionalDirectories": ["~/.rote"]
  }
}
```

`Bash(rote:*)` covers normal rote calls. `Bash(cd:*)` covers entering a
workspace before running rote. `~/.rote` covers browser workspaces, flow files,
adapter state, and ledger reads outside the project directory. The read-only
helpers are a backstop for harnesses that still emit compact shell inspection
commands; do not use them as the default browser query language.

When navigating, run one logical command per tool call. Avoid `;`, long `&&`
chains, and `grep | head` pipelines when a rote-native command can answer the
question. Prefer:

```bash
rote browser page current --format json
rote browser slice @page.current headings --format json
rote browser slice @page.current clickable --format json
rote browser deps --format compact
```

If you need pattern matching inside a numeric snapshot response, use
`rote browser-find @N --text ...`, `rote browser-find @N --text-regex ...`, or
load the compact slice and inspect it directly. Do not use external `jq`; rote
has native `@N` query support.

After the user has chosen a browser launch shape, you may check for reusable
browser flows:

```bash
rote flow search "<browser task>"
```

If a flow is found, present it as one option that will use the chosen browser
mode/session shape. Do not run it until the user confirms.

## Interactive Browser Choice

When the user asks to browse interactively and has not already specified launch
flags, elicit launch choices before opening the page or running a browser flow.
Use the agent's question UI when available; otherwise ask concise numbered
questions in chat.

Ask first:

1. Headed or headless:
   - Headed: visible browser; best for active login, Gmail, SSO, MFA, debugging.
   - Headless: background browser; best for CI, scripts, and crystallized replay.

If the user chooses headed, ask next:

2. Session source:
   - Attach existing via Playwright extension: best for a normal already
     logged-in Chrome/Edge session, Gmail, SSO/MFA, stateful forms, and
     multi-tab comparisons.
   - New rote-managed headed browser: best for isolated visual exploration.

If the user chooses headless, ask next:

2. Session source:
   - New isolated rote-managed browser.
   - Saved browser auth state, when the task needs login but should replay
     headlessly.

Then ask:

3. Open in a new tab or a new window.

Never start an interactive `/rote-browse` task in headless mode unless the user
explicitly chose headless or the request is clearly script/CI/flow replay.

## Timeboxed Human Elicitation

If the agent asks the user a browser launch, setup, login, challenge, or
transaction question, wait less than one minute for the answer. If no answer
arrives, choose the safest read-only path and state that choice.

No-answer defaults:

- Do not attach to a personal browser unless the user already chose attach.
- Do not wait on extension tab share, login, MFA, CAPTCHA, consent, payment, or
  destructive confirmation prompts.
- Do not click, type, submit, send, purchase, accept, upload, delete, archive,
  mark-read, or otherwise mutate state.
- Use existing ledger state and last good snapshots for read-only reasoning.
- For public pages only, use `--headless --new-session --new-tab --no-prompt`.
- For authenticated or personal sites, stop at visible shell/list inspection or
  report that explicit user confirmation is required.

Use headed Playwright extension attach when the user needs their existing
login, SSO, MFA, browser extensions, or profile state in a new tab:

```bash
rote browser attach setup --method extension --browser chrome
rote browse --headed --attach-existing --new-tab --no-prompt --no-snapshot <url>
```

In a brand-new workspace, `--attach-existing` must be paired with `--headed`.
Do not omit `--headed`; the command will fail before navigation instead of
opening the tab. If the user supplied a target workspace, include `-w <name>`
on the initial navigation command.

If the Playwright extension opens a connection page, copy the
`PLAYWRIGHT_MCP_EXTENSION_TOKEN` it shows and persist it once:

```bash
rote browser attach setup --method extension --extension-token <token> --no-open --no-prompt
```

Use a new session when the task should avoid personal browser state:

```bash
rote browse --headed --new-session --new-tab <url>
```

## Script And Flow Defaults

For non-interactive scripts, CI, and crystallized flows, choose deterministic
defaults unless the flow frontmatter explicitly says otherwise:

```bash
rote browse --headless --new-session --new-tab --no-prompt <url>
```

Exploration may be headed or attached. Replay should normally be headless,
isolated, and backed by saved browser auth state rather than a live personal
tab.

## Dynamic Page Readiness

For SPAs and logged-in apps, avoid snapshotting the loading shell. Navigate
without an immediate snapshot, wait for a positive readiness signal, then
snapshot:

```bash
rote browse --headed --attach-existing --new-tab --no-prompt --no-snapshot <url>
rote browse wait --selector '<ready-selector>' --timeout 30 --quiet-ms 750
rote browse snapshot
```

Use `--text` when visible text is a better readiness signal than a selector.
For Gmail inbox testing, `[role=row]` is usually the positive signal for the
message list; `[role=main]` is a broader shell signal.

Never use `rote browse wait` to wait for a human browser permission prompt,
extension token enrollment, login form, CAPTCHA, or payment/consent checkpoint.
Those are human-gate events: stop, state the gate, and ask the user to complete
the handoff or switch to a new rote-managed session.

## Fresh Live-Attach Recipe

For an existing-profile browser task in a fresh workspace, use this sequence:

```bash
rote browse <url> -w <workspace> --headed --attach-existing --new-tab --no-prompt --no-snapshot
rote browse wait --selector '<ready-selector>' --timeout 30 --quiet-ms 750
rote browse snapshot
```

Then validate before acting:

```bash
rote @N '.content[0].text' -r | wc -l
rote browser deps --format compact
rote browser slice @page.current clickable --format json
```

If `browser_snapshot` returns text that starts with `### Open tabs`, do not
classify it as empty until you check whether the same response also contains
`### Page` and `### Snapshot`. A response with those sections is a valid page
snapshot and should be used through the ledger/slice commands.

## Page Leases

Treat a page lease as the live browser tab/page contract. It is not a snapshot:
snapshots are observations made while a lease is active.

If `rote browse snapshot` reports that capture returned no usable snapshot
text, do not keep looping. Inspect the lease and last good page state:

```bash
rote browser page current --format compact
rote browser deps --format compact
rote browser tabs list --format json
```

If the current lease is `capture_failed`, use the last good snapshot for
read-only reasoning and recover the live target before more actions. For an
extension endpoint, retry once after readiness; rote may recover a usable page
snapshot from the extension tab inventory response:

```bash
rote browse wait
rote browse snapshot
```

If the current lease is `target_missing`, the leased tab is gone or no longer
visible to the extension. Do not act on another tab by guess. List tabs, pick
the intended page by index/title/url, grant a new lease, then snapshot:

```bash
rote browser tabs list --format json
rote browser tabs activate --title-regex '<site-or-title>' --grant-lease --format json
rote browse snapshot
```

For Playwright extension attach, make sure the extension token is configured
before continuing. For new-tab work, stateful forms, and multi-tab comparisons,
use the extension-backed active browser lease or a new managed browser session
instead of waiting on a human share prompt.

For multi-page browsing, keep one lease/page per tab. Query and act through the
active page's slices, and rebase stale refs only within that page's history.
`rote browse` records live tab index/title/url when Playwright exposes tab
inventory, then reconciles that lease before snapshot, wait, click, type,
screenshot, code, or back. If the leased tab is present but inactive, rote
selects it automatically. If it disappeared, rote marks the lease
`target_missing`, preserves the last good snapshot, and stops live actions until
you grant another lease.

Recover or compare tabs with:

```bash
rote browser tabs list --format json
rote browser tabs activate --index <N> --grant-lease --format json
rote browser tabs activate --title-regex '<site-or-title>' --grant-lease --format json
```

Cleanup is ownership-based. Rote-created tabs in an attached browser use
`cleanup_policy=close-tab` and close on `rote browse close`. Human/imported
tabs use `ownership=external`, `cleanup_policy=keep`, and must stay open unless
the user explicitly asks otherwise.

## Stateful Multi-Step Forms And Pickers

Treat form fill, autocomplete, date/time/location pickers, steppers, filters,
configurators, schedulers, checkout-like flows, SaaS forms, CRM entry, and
flight search as multi-step state machines. Prefer Playwright extension attach
for existing-profile testing; use a new managed browser session when personal
browser state is not required.

For each field group, picker, stepper, modal, validation state, or result panel,
use this loop:

```bash
rote browse wait --text '<positive-ready-text>' --timeout 30 --quiet-ms 750
rote browse snapshot
rote browser slice @page.current forms --format json
rote browser slice @page.current clickable --format json
rote browse click <one-ref>
rote browse wait --text '<post-action-ready-text>' --timeout 30 --quiet-ms 750
rote browse snapshot
```

Do not chain a full form-fill/search/configuration path until each transition
has been observed. Rebase stale refs only inside the same page lease:

```bash
rote browser ref rebase @page.previous @page.current <old-ref> --format json
```

Read-only search/filter/result comparison is allowed. Submit, send, save,
publish, delete, archive, mark-read, book, payment, seat/baggage/passenger
detail, upload/download, account edits, and "continue" controls past result
selection are transactional. Require explicit user confirmation. If no answer
arrives in under one minute, stop at read-only inspection.

## Anti-Agent Challenges

When the expected UI is missing, do not loop through repeated navigation and
snapshots. Classify the page first:

```bash
rote browser modal detect --format json
rote browse auth detect
rote browse auth detect --json
```

Use `human_gate` as the primary durable handoff signal. If
`human_gate.status` is `credentials_required`, fill credentials only with
explicit user-approved credentials or ask the human to log in. If
`human_gate.status` is `human_required` or `blocked`, stop the automated loop
and ask the human to resolve or redirect.

`anti_agent` explains CAPTCHA, robot checks, Cloudflare challenges,
bot-detection text, rate limits, and access-denied cues; `human_gate` explains
the handoff decision. Rote detects and delegates these defenses; it does not
bypass them.

After the human resolves the challenge, refresh the browser state:

```bash
rote browse auth captcha-wait
rote browse wait --timeout 30
rote browse snapshot
rote browse auth save
```

## Token-Saving Inspection

Prefer slices and targeted queries over full snapshots:

```bash
rote browser slice @page.current clickable
rote browser slice @page.current forms --format json
rote browser deps
```

Use the Page Ledger instead of reloading whole responses:

```bash
rote browser page current
rote browser page history
```

Full snapshots stay in the workspace. Pull only the relevant slice into model
context when possible.

## Stale Ref Recovery

Playwright refs can expire after reloads, redirects, or SPA updates. Capture a
fresh snapshot and rebase old refs before acting when the page changed:

```bash
rote browse snapshot
rote browser ref rebase @page.previous @page.current e42
```

Rote click/type commands also attempt one automatic stale-ref rebase when the
ledger has enough element evidence.

## Authenticated Replay

If exploration used a logged-in browser, save the browser session before
crystallizing a flow:

```bash
rote browse auth save --domain <domain> --profile <profile>
rote flow frontmatter --browser-session <domain>/<profile> --format typescript
```

Do not store raw passwords or paste cookies into a flow. The flow should declare
the browser session dependency and let runtime preflight validate it before side
effects.

## Private Hub Sharing

Browser flows may be pushed to a private hub for personal recall, but do not
promise that they will work across other machines or accounts. Before sharing a
browser flow, state this caveat:

```text
This browser flow was captured from a specific browser/runtime/session context.
It is intended for private recall or controlled replay. Cross-machine replay is
not guaranteed unless the flow has passed headless validation and declares
portable browser dependencies.
```

Use private recall by default for browser flows that depend on live attach,
saved local auth, device-bound SSO, browser extensions, account-specific UI, or
dynamic site structure. Public or broad team sharing needs headless validation,
declared runtime dependencies, declared saved auth, readiness checks, and no
personal data embedded in the flow.

## Gmail Safety

For Gmail smoke tests, prove attach and UI control without opening messages or
sending mail:

```bash
rote browse --headed --attach-existing --new-tab --no-snapshot https://mail.google.com
rote browse wait --selector '[role=row]' --timeout 30 --quiet-ms 750
rote browse snapshot
rote browser slice @page.current clickable
```

For `/rote-browse check email`, do not replace the browser request with a Gmail
API or email-flow execution unless the user explicitly switches from browsing
to adapter/flow execution. First ask headed/headless; for a logged-in Gmail
browser session, headed plus existing-browser attach is usually the right path.

Click Compose only when the user explicitly asked to test it, then discard the
empty draft.
