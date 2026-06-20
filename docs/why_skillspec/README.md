# Prose Says It. SkillSpec Proves It.

This folder is a demo script in document form. It compares the same browser
research skill written two ways:

- [prose_only/SKILL.md](prose_only/SKILL.md): a normal prose-heavy skill.
- [skillspec_backed/SKILL.md](skillspec_backed/SKILL.md): a thin loader that
  points to [skillspec_backed/skill.spec.yml](skillspec_backed/skill.spec.yml).

The point is not that prose is bad. Prose is still the right place for tone,
judgment, examples, and domain context. The problem is that prose is a weak
place to hide behavior that should be checked: which tool to use, what not to
substitute, which dependencies matter, when to ask the user, what counts as
evidence, and how to report why the agent made a decision.

SkillSpec keeps the prose but moves the contract into a file that can be
validated, tested, compiled, and traced.

## The Demo Story

Start with the prose-only skill. It sounds reasonable:

- Use browser automation for browsing tasks.
- Do not replace browser work with native search.
- Search only to discover a URL when the user did not provide one.
- Ask for the URL when the target page is ambiguous.
- Report the evidence at the end.

That is good guidance for a human reader. It is also fragile as an agent
contract. A maintainer cannot run a command to prove that the "never use search
as the answer" rule is still present. A reviewer cannot tell whether a typo in a
route field made the rule inert. CI cannot prove the expected behavior for
representative inputs. A user cannot inspect a compact trace explaining why the
agent chose browser automation instead of search.

Now look at the SkillSpec-backed version. The loader stays short. The behavior
lives in `skill.spec.yml`:

- `routes` name browser automation and URL discovery.
- `rules` bind task language to route choices, forbids, allows, elicitations,
  and post-success obligations.
- `dependencies` declare browser and network requirements.
- `tests` prove route, forbid, route-order, elicitation, and matched-rule
  behavior.
- `trace.required` makes the decision path inspectable.

The demo is powerful because it is not a claim. It is runnable.

```sh
skillspec validate docs/why_skillspec/skillspec_backed/skill.spec.yml
skillspec imports check docs/why_skillspec/skillspec_backed/skill.spec.yml
skillspec test docs/why_skillspec/skillspec_backed/skill.spec.yml
skillspec deps check docs/why_skillspec/skillspec_backed/skill.spec.yml
skillspec decide docs/why_skillspec/skillspec_backed/skill.spec.yml \
  --input='browse https://example.com and take a snapshot' \
  --trace-dir /tmp/skillspec-demo-traces
```

The punchline for a live demo is simple:

```text
The prose-only skill says the rule.
The SkillSpec-backed skill proves the rule.
```

## Side-By-Side Comparison

| Prose-only skill | SkillSpec-backed skill |
| --- | --- |
| Human-readable | Human-readable plus machine-checkable |
| Important behavior lives in paragraphs | Important behavior lives in typed fields |
| "Use browser automation" is advisory | `routes` and `rules` select `browser` |
| "Never use search instead" is easy to miss | `forbid` makes `native_search_as_answer` testable |
| URL discovery is mixed into prose | `route_order` can say discovery first, browser second |
| User questions are open-ended | `elicitations` define bounded choices |
| Dependencies are implied by instructions | `dependencies` make browser and network needs visible |
| Dependency readiness is discovered mid-task | `skillspec deps check` exposes readiness before execution |
| Behavior examples are informal | scenario tests pass or fail in CI |
| A typo can silently weaken the skill | strict validation rejects unknown typed fields |
| No standard decision trail | required `trace` records matched rules and selected route |
| Final report depends on memory | `after_success` names reporting obligations |
| Reviewers read the whole skill manually | reviewers inspect schema, tests, and focused diffs |
| Porting means reinterpreting prose | compiler targets emit thin harness loaders |
| Reference files are either over-loaded or ignored | `imports` load active guidance only when needed |
| Original source can be lost during rewrites | `resources` preserve the original prose as provenance |
| Security claims are broad | safety-sensitive choices are explicit and reviewable |
| Regression risk is mostly vibes | golden snapshots and scenario tests catch behavior drift |
| Sharing relies on trust | validation, tests, deps, and traces create a compliance gate |

## What To Show In The Demo

Use one task:

```text
browse https://example.com and take a snapshot
```

Show the prose-only skill first. Ask the audience what guarantees the agent will
not answer from native search. The honest answer is: nothing but instruction
following.

Then run the SkillSpec-backed version:

```sh
skillspec test docs/why_skillspec/skillspec_backed/skill.spec.yml
skillspec decide docs/why_skillspec/skillspec_backed/skill.spec.yml \
  --input='browse https://example.com and take a snapshot' \
  --trace-dir /tmp/skillspec-demo-traces
```

Show the output:

- selected route: `browser`
- matched rule: `browser_request_requires_browser`
- forbidden substitution: `native_search_as_answer`
- trace directory written

Then make a temporary copy of the spec and typo the rule:

```yaml
preferr: browser
```

Run validation again. It fails. That is the moment the value becomes obvious:
the same instruction that was only prose is now part of a contract.

## The Narrative To Use

Agent skills are becoming the unit of reusable agent behavior. MCP gives agents
tools and data. But between "load this skill" and "call this tool" there is a
missing layer: how should the agent decide?

SkillSpec is that layer.

- Agent Skills define what to load.
- MCP defines what tools and data are available.
- SkillSpec defines how the agent should decide, verify, and report behavior.

This does not replace prose. It gives prose a partner. Prose explains judgment.
SkillSpec carries the decisions that should survive refactors, ports, reviews,
and CI.

The goal is not a bigger prompt. The goal is a smaller loader and a stronger
contract.
