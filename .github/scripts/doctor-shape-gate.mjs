import fs from "node:fs";
import path from "node:path";

const reportDir = process.env.DOCTOR_REPORT_DIR || "doctor-report";
const stagePath = process.env.SOURCE_STAGE_JSON || path.join(reportDir, "source-stage.json");
const target = process.env.TARGET || "";
const displayTarget = process.env.DISPLAY_TARGET || displayForTarget(target);

const stage = JSON.parse(fs.readFileSync(stagePath, "utf8"));
const shape = stage.source_shape || {};
const kind = shape.kind || "unknown";
const skillFileCount = Number(shape.skill_file_count || 0);
const pluginRoots = Array.isArray(shape.plugin_roots) ? shape.plugin_roots : [];
const runFullDoctor = kind !== "non_skill_repository";
const gate = {
  schema: "skillspec/public-doctor-shape-gate/v0",
  target: displayTarget,
  shape_kind: kind,
  skill_file_count: skillFileCount,
  plugin_root_count: pluginRoots.length,
  staged_source_path: stage.staged_source_path || "",
  selected_source_path: stage.selected_source_path || null,
  run_full_doctor: runFullDoctor,
  decision: decisionForShape(kind, skillFileCount, pluginRoots.length),
  next: Array.isArray(stage.next) ? stage.next : [],
};

fs.mkdirSync(reportDir, { recursive: true });
fs.writeFileSync(path.join(reportDir, "shape-gate.json"), `${JSON.stringify(gate, null, 2)}\n`);
fs.writeFileSync(path.join(reportDir, "shape-gate.md"), renderMarkdown(gate));

if (!runFullDoctor) {
  fs.writeFileSync(path.join(reportDir, "doctor-report.md"), renderMarkdown(gate));
  fs.writeFileSync(path.join(reportDir, "doctor-report.txt"), renderText(gate));
  fs.writeFileSync(path.join(reportDir, "doctor-report.json"), `${JSON.stringify(gate, null, 2)}\n`);
  fs.writeFileSync(path.join(reportDir, "doctor-report.html"), renderHtml(gate));
}

appendOutput("shape_kind", kind);
appendOutput("skill_file_count", String(skillFileCount));
appendOutput("plugin_root_count", String(pluginRoots.length));
appendOutput("staged_source_path", gate.staged_source_path);
appendOutput("run_full_doctor", String(runFullDoctor));

function decisionForShape(shapeKind, skills, plugins) {
  if (shapeKind === "simple_skill") {
    return "Run full single-skill Doctor assessment.";
  }
  if (shapeKind === "plugin_workspace") {
    return `Run full plugin workspace assessment for ${skills} skill package(s) across ${plugins} plugin namespace(s). Preserve namespace and path identity.`;
  }
  if (shapeKind === "multi_skill_workspace") {
    return `Run full multi-skill workspace assessment for ${skills} skill package(s). Preserve path identity.`;
  }
  if (shapeKind === "entry_skill_with_subskills") {
    return `Run full entry-plus-subskills assessment for ${skills} skill package(s). Treat the root entry and nested skills separately.`;
  }
  if (shapeKind === "non_skill_repository") {
    return "Stop after shape assessment. No SKILL.md package was found, so public CI will not spend compute on a generic source repository.";
  }
  return "Run only after reviewing the shape output; the submitted source shape is not recognized.";
}

function renderMarkdown(report) {
  return [
    "# SkillSpec Doctor shape gate",
    "",
    `**Target:** ${report.target}`,
    "",
    `**Shape:** \`${report.shape_kind}\``,
    "",
    "- **Skill files:** `" + report.skill_file_count + "`",
    "- **Plugin roots:** `" + report.plugin_root_count + "`",
    "- **Decision:** " + report.decision,
    "",
    "## Shape-specific handling",
    "",
    handlingForShape(report.shape_kind),
    "",
    "## Next",
    "",
    ...nextLines(report),
  ].join("\n");
}

function handlingForShape(shapeKind) {
  if (shapeKind === "simple_skill") {
    return "This is one atomic skill package. The public report can score this package directly.";
  }
  if (shapeKind === "plugin_workspace") {
    return "This is a plugin-shaped workspace. The report must keep plugin namespace, folder path, repeated names, and repeated content as separate evidence instead of flattening them.";
  }
  if (shapeKind === "multi_skill_workspace") {
    return "This is a multi-skill workspace. The report must roll up per-package risk while keeping each package path addressable.";
  }
  if (shapeKind === "entry_skill_with_subskills") {
    return "This source has a root entry skill plus nested skills. The report must distinguish entry behavior from subskill packages.";
  }
  if (shapeKind === "non_skill_repository") {
    return "No importable skill package was found. Choose a GitHub folder that contains `SKILL.md`, or run Doctor locally against the intended skill path.";
  }
  return "The source shape is unknown. Inspect `shape-gate.json` before importing, installing, or relying on this report.";
}

function nextLines(report) {
  if (report.run_full_doctor) {
    return ["A full Doctor report follows this shape gate in the workflow artifacts and issue comment."];
  }
  if (report.next.length === 0) {
    return ["- Submit a GitHub folder URL that contains `SKILL.md`."];
  }
  return report.next.map((item) => `- ${item}`);
}

function renderText(report) {
  return [
    "SkillSpec Doctor shape gate",
    "===========================",
    "",
    `Target: ${report.target}`,
    `Shape: ${report.shape_kind}`,
    `Skill files: ${report.skill_file_count}`,
    `Plugin roots: ${report.plugin_root_count}`,
    `Decision: ${report.decision}`,
    "",
    handlingForShape(report.shape_kind),
    "",
  ].join("\n");
}

function renderHtml(report) {
  return [
    "<!doctype html>",
    "<html lang=\"en\">",
    "<head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
    "<title>SkillSpec Doctor Shape Gate</title>",
    "<style>body{font-family:system-ui,sans-serif;margin:2rem;line-height:1.55;color:#111827}main{max-width:880px}code{background:#f3f4f6;padding:.1rem .3rem}.panel{border:1px solid #d1d5db;padding:1rem}</style>",
    "</head><body><main>",
    "<p>SkillSpec Doctor</p>",
    "<h1>Shape gate</h1>",
    "<div class=\"panel\">",
    `<p><strong>Target:</strong> ${escapeHtml(report.target)}</p>`,
    `<p><strong>Shape:</strong> <code>${escapeHtml(report.shape_kind)}</code></p>`,
    `<p><strong>Skill files:</strong> ${report.skill_file_count}</p>`,
    `<p><strong>Plugin roots:</strong> ${report.plugin_root_count}</p>`,
    `<p><strong>Decision:</strong> ${escapeHtml(report.decision)}</p>`,
    "</div>",
    `<p>${escapeHtml(handlingForShape(report.shape_kind))}</p>`,
    "</main></body></html>",
  ].join("\n");
}

function appendOutput(name, value) {
  if (!process.env.GITHUB_OUTPUT) {
    return;
  }
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `${name}=${value}\n`);
}

function displayForTarget(raw) {
  try {
    const url = new URL(raw);
    if (url.protocol !== "https:" || url.hostname.toLowerCase() !== "github.com") {
      return "[submitted target]";
    }
    const parts = url.pathname.split("/").filter(Boolean);
    if (parts.length < 2) {
      return "github.com/[repository]";
    }
    const ownerRepo = `${parts[0]}/${parts[1].replace(/\.git$/, "")}`;
    let rest = parts.slice(2);
    if (/^(tree|blob)$/i.test(rest[0] || "") && rest.length >= 3) {
      rest = rest.slice(2);
    }
    return rest.length === 0 ? `github.com/${ownerRepo}` : `github.com/${ownerRepo}/.../${rest.at(-1)}`;
  } catch (_error) {
    return "[submitted target]";
  }
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
