import fs from "node:fs";
import path from "node:path";

const sourceDir = process.env.DOCTOR_REPORT_DIR || "doctor-report";
const outputDir = process.env.PUBLIC_REPORT_DIR || "doctor-report-public";
const target = process.env.TARGET || "";
const displayTarget = process.env.DISPLAY_TARGET || displayForTarget(target);
const replacementTargets = new Set(
  [target, process.env.DOCTOR_TARGET || "", process.env.STAGED_SOURCE_PATH || ""].filter(Boolean),
);

fs.rmSync(outputDir, { recursive: true, force: true });
fs.mkdirSync(outputDir, { recursive: true });

const reportFiles = [
  "source-stage.json",
  "shape-gate.json",
  "shape-gate.md",
  "doctor-report.txt",
  "doctor-report.md",
  "doctor-report.html",
  "doctor-report.json",
  "doctor-stderr.txt",
];

for (const file of reportFiles) {
  const source = path.join(sourceDir, file);
  if (!fs.existsSync(source)) {
    continue;
  }

  const content = fs.readFileSync(source, "utf8");
  fs.writeFileSync(path.join(outputDir, file), sanitize(content));
}

fs.writeFileSync(path.join(outputDir, "target.txt"), `${displayTarget}\n`);

function sanitize(content) {
  let output = content;
  if (displayTarget) {
    for (const value of replacementTargets) {
      output = output.split(value).join(displayTarget);
      output = output.split(encodeURI(value)).join(encodeURI(displayTarget));
    }
  }

  return output
    .replace(/\/home\/runner\/work\/[^\s"'`<>)\]]+/g, "[runner-path]")
    .replace(/\/private\/tmp\/[^\s"'`<>)\]]+/g, "[temp-path]")
    .replace(/\/tmp\/[^\s"'`<>)\]]+/g, "[temp-path]")
    .replace(/\/private\/var\/folders\/[^\s"'`<>)\]]+/g, "[temp-path]")
    .replace(/\/Users\/[^\s"'`<>)\]]+/g, "[local-path]");
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

    if (rest.length === 0) {
      return `github.com/${ownerRepo}`;
    }

    return `github.com/${ownerRepo}/.../${rest.at(-1)}`;
  } catch (_error) {
    return "[submitted target]";
  }
}
