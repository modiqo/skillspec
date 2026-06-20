---
name: browser-research-prose
description: Use when the user asks to browse, inspect, click, snapshot, or extract evidence from a web page.
---

# Browser Research

Use this skill when the user asks you to browse a website, open a page, click
through an interface, inspect page content, take a snapshot, extract evidence
from a page, or verify what a page currently shows.

For browser tasks, use browser automation. Do not answer from native search when
the user asked you to observe a page. Native search can be used only to discover
the likely URL when the user did not provide one, and it must not be presented
as page evidence.

If the user provides a URL, open that URL in the browser and inspect the page
directly. If the user names a page but does not provide a URL, ask for the URL
unless there is a clear, low-risk discovery path. If discovery is needed, search
only long enough to identify the target URL, then switch to browser automation
and verify the page itself.

Do not use curl, static HTML fetches, or search snippets as substitutes for
browser evidence when the task depends on rendered page state, navigation,
logged-in state, screenshots, dynamic content, or visible UI.

Before taking action, make sure a browser session is available and that network
access is allowed. If a login, credential, or private workspace is needed, stop
and ask before proceeding.

When the work is done, report what page was opened, what evidence was observed,
which route was used, and any limitations. If there was URL discovery before
browser inspection, say so clearly.

Good behavior examples:

- "browse https://example.com and take a snapshot" means open that URL in the
  browser and report browser evidence.
- "inspect the latest docs page but I do not have the URL" means discover the
  likely URL first, then verify the page in the browser.
- "summarize search results about this company" is not a browser-inspection
  task unless the user asks to open or inspect a page.

This skill relies on the agent following the prose carefully. There is no
separate validation command, scenario test, dependency preflight, or trace
contract in this prose-only version.

