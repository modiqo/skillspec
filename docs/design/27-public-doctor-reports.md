# Public Doctor Reports

SkillSpec supports two public doctor-report paths:

1. CI dogfood runs `skillspec doctor skills/skillspec/` on every main and pull
   request quality run.
2. Public users can open a "Doctor report request" issue with a public GitHub
   skill URL. GitHub Actions validates the URL, runs `skillspec doctor`, uploads
   report artifacts, and comments back with the formatted report.
3. The GitHub Pages site at `https://skillspec.sh/` provides a
   public form and report gallery. The form opens a prefilled issue request; the
   gallery reads public doctor-report issues and renders their workflow comments.

The goal is to make the static skill-risk report visible before someone installs
or ports a skill. Doctor reports the current skill baseline: how likely the
current shape is to make an agent skip, reorder, improvise, use the wrong
surface, or finish without proof. The score is not a grade of domain expertise,
human usefulness, author effort, or legal/medical/factual correctness.

The workflow is intentionally read-only: it stages public source, parses
Markdown/YAML, classifies shape, and renders reports. It does not execute code
from the target repository.

## Dogfood CI

The CI quality job builds the local CLI and then runs:

```sh
skillspec doctor skills/skillspec/
skillspec doctor skills/skillspec/ --markdown
skillspec doctor skills/skillspec/ --json
skillspec doctor skills/skillspec/ --html
```

The Markdown report is printed into the GitHub job summary. The text, Markdown,
JSON, and HTML reports are uploaded as the `skillspec-doctor-dogfood` artifact.

Dogfood is observational first. CI should not fail merely because doctor finds a
medium or high issue; otherwise the report becomes a noisy gate instead of a
useful signal. A stricter regression gate can be added later once thresholds are
stable.

## Public Request Workflow

The browser page is static and intentionally has no secret token. It cannot and
must not create issues directly on behalf of anonymous users. Instead, the form
validates the URL client-side and opens a prefilled GitHub issue. The user
reviews and submits that issue in GitHub, and the existing issue workflow remains
the trusted queue.

The public issue form asks for one field:

```text
Public GitHub skill URL
```

Requests are recognized by either the `doctor-report` label or a title that
starts with `Doctor report:`. The label is still attached by the issue template
when the repository label exists, but the workflow also accepts the title prefix
so first-time setup mistakes do not silently skip public requests. The parser
prefers the issue-form body and falls back to a GitHub URL in the title.

The workflow accepts only normalized `https://github.com/<owner>/<repo>` URLs
and public GitHub folder URLs under them. Canonical GitHub folder URLs use
`/tree/<branch>/<path>`, but public requests also accept `/blob/<branch>/<path>`
when users copy a folder-like URL from the GitHub UI and the path resolves to a
folder rather than `SKILL.md`. It rejects URLs with credentials, query strings,
fragments, non-GitHub hosts, unsupported owner/repo characters, or excessive
length.

After syntax validation, the workflow calls the GitHub repository API for
`owner/repo`. If the repository is private or cannot be read publicly, the
workflow comments with local-run instructions:

```sh
curl -fsSL https://skillspec.sh/install.sh | sh
git clone <your-private-repo-url>
skillspec doctor /path/to/local/skill
skillspec doctor /path/to/local/skill --markdown > skillspec-doctor.md
skillspec doctor /path/to/local/skill --html > skillspec-doctor.html
skillspec doctor /path/to/local/skill --json > skillspec-doctor.json
```

This keeps private source out of public Actions logs and avoids implying that
SkillSpec can inspect private repositories without the user's local credentials.

## Report Outputs

For accepted public targets, the workflow builds the current repo CLI and runs:

```sh
skillspec doctor "$target" > doctor-report.txt
skillspec doctor "$target" --markdown > doctor-report.md
skillspec doctor "$target" --html > doctor-report.html
skillspec doctor "$target" --json > doctor-report.json
```

The Actions run summary and issue comment both render the Markdown report
directly, so a public user can read the result directly in GitHub without
downloading an artifact. Long reports are truncated in GitHub surfaces and
preserved in full in the artifact. The artifact contains:

- `doctor-report.txt`
- `doctor-report.md`
- `doctor-report.html`
- `doctor-report.json`
- `target.txt`
- `doctor-stderr.txt` when a run fails

The report's next actions should teach the post-doctor path:

1. Capture and publish the baseline Doctor report in the skill repository or PR.
2. In the harness, ask `/skillspec import <skill-repo-or-folder>, compile it,
   verify it, test it, and prove it. Print the alignment summary.`
3. Review the alignment summary for selected route, requirements proven, missing
   proof, forbidden-action status, and token/wall-clock metrics when available.
4. Optionally publish generated `skill.spec.yml`, compiled loader, and alignment
   report next to the baseline report.
5. Restart the harness and try the SkillSpec-backed skill normally.

Artifacts are retained for 14 days. The issue comment is updated in place on
issue edits or reruns using a hidden `skillspec-doctor-report` marker so repeated
runs do not leave a comment trail.

## Public Pages Gallery

The Pages source lives under `docs/pages/` and is deployed by
`.github/workflows/pages.yml`. It is a static client-side app:

- The request form accepts only public `https://github.com/...` URLs and opens a
  prefilled `Doctor report request` issue.
- The report gallery reads public issues through GitHub's public Issues API and
  keeps entries whose title starts with `Doctor report:` or whose labels include
  `doctor-report`.
- It looks for the hidden `skillspec-doctor-report` marker in bot comments,
  extracts the Markdown report, renders it in the page, and links back to the
  original issue.
- Pending requests are shown when no report comment exists yet.
- Error and private-repository cases remain visible because the issue workflow
  writes a marker-bearing comment with local-run instructions.

This design avoids public write tokens in the browser while still giving users a
friendly form, a public status list, and readable prior output.

## Security Model

The workflow treats issue input as untrusted.

- It does not run shell commands assembled from issue text.
- The target URL is passed as an environment variable and quoted.
- Only public GitHub repositories are allowed.
- Repository visibility is checked before checkout/staging.
- The job has `contents: read` and `issues: write`; it does not request broad
  repository write permissions.
- The job has a 15 minute timeout.
- The doctor command itself is static analysis; it should not execute target
  repository code, scripts, snippets, package managers, or tests.

This is enough for a public "run doctor on my skill" path without turning the
project's Actions minutes into a general remote execution service.
