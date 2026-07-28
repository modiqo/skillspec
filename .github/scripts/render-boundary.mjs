// Render `skillspec boundary assess --json` into a clean, actionable Markdown
// report for the public CI: a ranked finding list with clickable evidence, then
// a short "what to do next" tutorial (check & decide, install safely, guard your
// harness). Reads the JSON, writes Markdown to stdout. Prints nothing when the
// analysis is missing or unparseable, so the caller can skip it.

import fs from "node:fs";

const jsonPath = process.argv[2] || "doctor-report/boundary-assess.json";
if (!fs.existsSync(jsonPath)) {
  process.exit(0);
}

let data;
try {
  data = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
} catch {
  process.exit(0);
}

const target = typeof data.target === "string" ? data.target : "";
const loc = parseGitHubTarget(target);
const summary = data.summary || {};
const skills = Array.isArray(data.skills) ? data.skills : [];
const reviewable = skills.filter((skill) => skill.warrants_review);
const dot = { critical: "\u{1F7E5}", high: "\u{1F534}", medium: "\u{1F7E0}", low: "\u{1F535}" };
const rank = { critical: 4, high: 3, medium: 2, low: 1 };

const out = [];
out.push("## Security analysis — what each skill can reach");
out.push("");
out.push(
  `**${summary.critical || 0} critical · ${summary.high || 0} high · ` +
    `${summary.medium || 0} medium · ${summary.low || 0} low · ` +
    `${summary.clean || 0} clean** across ${skills.length} skill(s).`,
);

if (reviewable.length) {
  out.push(
    `${reviewable.length} skill(s) reach beyond their own directory and warrant review before install.`,
  );
  out.push("");
  out.push("### Findings to review");
  for (const skill of reviewable) {
    out.push("");
    out.push(`**${(skill.severity || "").toUpperCase()} — \`${skill.package || "."}\`**`);
    const findings = (skill.findings || []).filter(
      (finding) => (rank[finding.severity] || 0) >= rank.medium,
    );
    for (const finding of findings) {
      const link = blobUrl(loc, skill.package, finding.file, finding.line);
      const label = finding.file
        ? `${finding.file}${finding.line ? `:${finding.line}` : ""}`
        : "";
      const where = link ? ` — [${label}](${link})` : label ? ` — ${label}` : "";
      const reached = finding.reached ? ` → \`${finding.reached}\`` : "";
      const note = finding.note ? ` _(${finding.note})_` : "";
      out.push(
        `- ${dot[finding.severity] || "•"} **${finding.headline}**${reached}: ${finding.consequence}.${where}${note}`,
      );
    }
  }
} else {
  out.push("");
  out.push(
    "✅ Nothing reaches outside its own directory — no skill warrants review before install.",
  );
}

const clean = skills.filter((skill) => !skill.severity).length;
const low = skills.filter((skill) => skill.severity === "low").length;
if (clean || low) {
  const bits = [];
  if (clean) bits.push(`${clean} clean (reach nothing outside their directory)`);
  if (low) bits.push(`${low} low (touch only their own files)`);
  out.push("");
  out.push(`_Cleared: ${bits.join("; ")}._`);
}

out.push("");
out.push("### What to do next");
out.push("");
out.push("**1. Check and decide.** Review the findings above, or re-run the analysis yourself:");
out.push("");
out.push("```bash");
out.push(`skillspec boundary assess ${target || "<skill-url>"}`);
out.push("```");
out.push("");
out.push(
  "**2. Install safely through the gate.** Assess, then install only on your approval — across harnesses (Claude, Codex, or any `AGENTS.md`/`SKILL.md` skills dir):",
);
out.push("");
out.push("```bash");
out.push(`skillspec pull ${target || "<skill-url>"}`);
out.push("```");
out.push("");
out.push(
  "**3. Guard your harness.** Compile a deny-by-default policy from the surface and enforce it on every tool call, so an unreviewed effect is blocked:",
);
out.push("");
out.push("```bash");
out.push("skillspec boundary emit <skill> --format skillspec   # least-privilege policy");
out.push("skillspec boundary guard install                     # managed PreToolUse hook (observe mode)");
out.push("skillspec boundary guard mode enforce                # deny effects no policy covers");
out.push("```");
out.push("");
out.push(
  "Static analysis only — nothing in the package was executed. See the [boundary guide](https://github.com/modiqo/skillspec/blob/main/docs/boundary-guide.md).",
);

process.stdout.write(out.join("\n") + "\n");

function parseGitHubTarget(value) {
  try {
    const url = new URL(value);
    const parts = url.pathname.split("/").filter(Boolean);
    const owner = parts[0];
    const repo = (parts[1] || "").replace(/\.git$/, "");
    let ref = "HEAD";
    let subpath = "";
    if (/^(tree|blob)$/i.test(parts[2] || "")) {
      ref = parts[3] || "HEAD";
      subpath = parts.slice(4).join("/");
    }
    return { owner, repo, ref, subpath };
  } catch {
    return {};
  }
}

function blobUrl(loc, pkg, file, line) {
  if (!loc || !loc.owner || !loc.repo || !file) {
    return "";
  }
  const rel = [loc.subpath, pkg, file].filter(Boolean).join("/").replace(/\/+/g, "/");
  return `https://github.com/${loc.owner}/${loc.repo}/blob/${loc.ref || "HEAD"}/${rel}${line ? `#L${line}` : ""}`;
}
