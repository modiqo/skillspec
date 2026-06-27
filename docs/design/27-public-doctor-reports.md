# Public Doctor Reports

SkillSpec supports two public doctor-report paths:

1. CI dogfood runs `skillspec doctor skills/skillspec/` on every main and pull
   request quality run.
2. Public users can open a "Doctor report request" issue with a public GitHub
   skill URL. GitHub Actions validates the URL, runs `skillspec doctor`, uploads
   report artifacts, and comments back with the formatted report.

The goal is to make the static skill-risk report visible before someone installs
or ports a skill. The workflow is intentionally read-only: it stages public
source, parses Markdown/YAML, classifies shape, and renders reports. It does not
execute code from the target repository.

## Dogfood CI

The CI quality job builds the local CLI and then runs:

```sh
skillspec doctor skills/skillspec/
skillspec doctor skills/skillspec/ --json
skillspec doctor skills/skillspec/ --html
```

The formatted report is printed into the GitHub job summary. The text, JSON, and
HTML reports are uploaded as the `skillspec-doctor-dogfood` artifact.

Dogfood is observational first. CI should not fail merely because doctor finds a
medium or high issue; otherwise the report becomes a noisy gate instead of a
useful signal. A stricter regression gate can be added later once thresholds are
stable.

## Public Request Workflow

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
and public GitHub folder URLs under them. It rejects URLs with credentials,
query strings, fragments, non-GitHub hosts, unsupported owner/repo characters,
or excessive length.

After syntax validation, the workflow calls the GitHub repository API for
`owner/repo`. If the repository is private or cannot be read publicly, the
workflow comments with local-run instructions:

```sh
curl -fsSL https://raw.githubusercontent.com/modiqo/skillspec/main/install.sh | sh
git clone <your-private-repo-url>
skillspec doctor /path/to/local/skill
skillspec doctor /path/to/local/skill --html > skillspec-doctor.html
skillspec doctor /path/to/local/skill --json > skillspec-doctor.json
```

This keeps private source out of public Actions logs and avoids implying that
SkillSpec can inspect private repositories without the user's local credentials.

## Report Outputs

For accepted public targets, the workflow builds the current repo CLI and runs:

```sh
skillspec doctor "$target" > doctor-report.txt
skillspec doctor "$target" --html > doctor-report.html
skillspec doctor "$target" --json > doctor-report.json
```

The issue comment includes the formatted text report inside a fenced block, so a
public user can read the result directly in GitHub. The artifact contains:

- `doctor-report.txt`
- `doctor-report.html`
- `doctor-report.json`
- `target.txt`
- `doctor-stderr.txt` when a run fails

Artifacts are retained for 14 days. The issue comment is updated in place on
issue edits or reruns using a hidden `skillspec-doctor-report` marker so repeated
runs do not leave a comment trail.

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
