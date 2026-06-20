# SkillSpec Roadmap

Keep the prose. Structure the decisions.

This project started from a very practical annoyance: good skills often become
long Markdown files full of buried decisions. Agents can read them, but they can
also skim past the important part, pick the wrong tool, or forget to leave a
trace of why they did what they did.

SkillSpec is the shape we want instead:

- `SKILL.md` is a small loader.
- `skill.spec.yml` is the durable contract.
- `source/` keeps the original prose and references around for context.

The goal is not to get rid of prose. Prose is still where tone, judgment, and
background live. The goal is to stop making prose carry every routing decision,
dependency, safety rule, command recipe, and completion checklist.

## The Claim

SkillSpec makes skills easier to trust because the important choices are no
longer hidden in paragraphs.

A harness should be able to open a skill, find the spec, ask it what route fits
the task, check the dependencies, execute the right recipe, and leave behind a
trace that explains what happened. That is the bar.

If this works, the win is not abstract. A user should see fewer wrong turns,
less repeated reasoning, clearer setup prompts, and a better answer at the end:
what was done, what evidence was captured, what it cost, and whether the work is
worth saving as reusable muscle memory.

## What Feels Good Already

- The minimal loader pattern feels right. Generated `SKILL.md` files should not
  become a second copy of the skill. They should point to the spec and tell the
  harness how to use it.
- The spec gives names to things that used to be implied: routes, rules,
  dependencies, elicitations, resources, code snippets, recipes, tests, traces,
  and completion behavior.
- Importing a whole skill folder is the right direction. Real skills are rarely
  just one `SKILL.md`; they have references, examples, scripts, snippets, and
  local conventions.
- Dependency preflight is necessary. If a skill needs `qpdf`, `gh`, `python3`,
  a browser session, or an authenticated service, the user should learn that
  before the agent is halfway through a task.
- Traces are a real unlock. They turn "the agent decided something" into a run
  directory we can inspect, compare, and summarize.
- Keeping the original source material matters. A good port should not erase the
  original skill; it should preserve it and add structure around it.

## What Still Feels Rough

The importer is the main place where the idea is ahead of the tool. If we point
SkillSpec at a real folder like Anthropic's PDF skill, it can find the obvious
shape: `SKILL.md`, companion reference files, Python snippets, shell commands,
and dependencies like `qpdf` or `pdftotext`. That is useful, but it is not the
same as understanding the skill. A good port still needs someone, human or
agent, to ask: which parts are routing rules, which are recipes, which snippets
are reusable artifacts, and which warnings are actually safety policy?

Dependencies are another honest rough edge. Finding `gh`, `python3`, or `qpdf`
in a code block is a start. It is not enough. A real skill may need a specific
version, a package manager, a logged-in CLI, an API token, a browser profile, or
a service adapter. SkillSpec should be able to say, plainly: this machine is
ready, this part is missing, this part needs approval, and this part should not
be installed automatically.

Code snippets also need to become more than fenced text we happened to preserve.
If a snippet reads a PDF form and writes JSON, the spec should know the expected
input file, the output artifact, the runtime, the dependencies, the validation
command, and the safety boundary. Otherwise we have only moved the snippet from
one Markdown file into another structured-looking place.

Resources need the same kind of care. Imported skills often point at sidecar
files: `reference.md`, `forms.md`, scripts, screenshots, templates, fixtures.
Today we can preserve them, but the next bar is stricter: catch a resource that
is referenced but missing, a file that was imported but never used, or a code
example that mentions a path that no longer exists.

Traces are promising, but still too developer-shaped. It is good that
`skillspec decide` leaves a run directory. It is not good enough if a user has
to inspect raw events to understand what happened. At the end of a run, the
harness should be able to say: I chose this route, skipped that route, asked no
questions because the dependencies were present, captured this evidence, and the
next run can be shorter because this path is now known.

The schema is intentionally loose while we learn, and that has helped. But the
same looseness will become a liability once people start sharing specs. We will
need stricter validation modes for specs that claim to be installable,
shareable, or release-ready.

Security cannot stay as a paragraph in a README. A SkillSpec can steer agents
toward CLIs, files, package installs, browsers, adapters, credentials, and
networked services. Those surfaces need to be visible in the spec, checked by
the CLI, and explained before a harness acts.

The last rough edge is proof. We have good intuition from dogfooding, but that
does not count as enough. We need side-by-side runs: the old prose skill versus
the SkillSpec-backed version, on the same tasks, with the same harness, measuring
whether the spec actually routes better, asks fewer unnecessary questions,
wastes fewer tokens, and leaves better evidence.

## What We Should Build Next

### 1. Better Imports

- Import local skill folders and public GitHub skill folders into a staged local
  workspace before conversion.
- Preserve every source file under `source/` and link useful pieces back into
  `resources`, `code`, `artifacts`, and `recipes`.
- Separate reference prose, executable snippets, command templates, dependency
  declarations, and route/rule candidates.
- Emit review notes where the importer is unsure. Do not flatten ambiguity into
  fake confidence.
- Validate missing resources, orphaned resources, unused snippets, and stale
  links.

### 2. Better Runtime Discipline

- Keep generated harness skills as minimal loaders.
- Validate the spec and check dependencies before serious use.
- Run `skillspec decide` with a trace directory every time.
- End each interaction with the selected route, evidence, trace directory,
  dependency status, and anything the user should do next.
- Add compact trace summaries for humans and JSON summaries for harnesses.

### 3. Better Dependency And Permission Modeling

- Expand dependency kinds: CLI, file, directory, package, service, browser,
  adapter, environment variable, and credential.
- Model provision choices explicitly: already present, optional, required,
  user-approved, forbidden, externally managed, or not yet supported.
- Add version checks and package-manager hints without silently mutating a
  machine.
- Make dependency questions first-class elicitations.

### 4. Better Code And Artifact Semantics

- Treat code snippets as named artifacts with language, provenance,
  dependencies, inputs, outputs, safety notes, and validation commands.
- Bind snippets to recipes instead of leaving them as passive examples.
- Let recipes consume and produce named artifacts.
- Add tests that prove a recipe chooses the right commands without running
  unsafe work.

### 5. Better Proof

- Build a small benchmark from real skills: shell routing, code review, PDF
  handling, repo research, commit conventions, and repo readiness.
- Compare prose-only skills and SkillSpec-backed skills on the same tasks.
- Measure route accuracy, forbidden-substitution avoidance, dependency
  detection, trace completeness, task success, and token usage.
- Publish the misses too. The misses will tell us where the grammar is vague.

### 6. Better Portability

- Keep Codex, Claude, and Markdown targets aligned.
- Add install helpers so users do not manually copy generated files into harness
  directories.
- Preserve the same spec semantics across targets.
- Document what each harness can do natively, what is optional, and what the
  loader has to emulate.

## Near-Term Definition Of Done

- `skillspec import-skill` can import multi-file public GitHub skill folders and
  local skill folders into a reviewed project shape.
- `skillspec validate` catches orphaned resources and missing local files.
- `skillspec deps check` reports declared dependencies with clear provisioning
  guidance.
- `skillspec compile --target codex-skill|claude-skill` emits a minimal loader,
  not a verbose mirror.
- Repo-local dogfood skills use the minimal loader + full spec + preserved
  source pattern.
- At least three real imported skills have passing tests and traceable route
  decisions.
