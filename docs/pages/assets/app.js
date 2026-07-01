const REPO_OWNER = "modiqo";
const REPO_NAME = "skillspec";
const REPORT_LABEL = "doctor-report";
const REPORT_MARKER = "<!-- skillspec-doctor-report -->";
const REPORT_WORKFLOW = "doctor-report.yml";
const CACHE_TTL_MS = 60 * 60 * 1000;
const REPORTS_CACHE_KEY = "skillspec.publicReports.v2";
const RUNS_CACHE_KEY = "skillspec.workflowRuns.v2";

const form = document.querySelector("#doctor-form");
const targetInput = document.querySelector("#target-url");
const formMessage = document.querySelector("#form-message");
const reportsGrid = document.querySelector("#reports-grid");
const reportsStatus = document.querySelector("#reports-status");
const runsList = document.querySelector("#runs-list");
const runsStatus = document.querySelector("#runs-status");
const refreshButton = document.querySelector("#refresh-reports");
const reportViewer = document.querySelector("#report-viewer");
const reportContent = document.querySelector("#report-content");
const viewerTitle = document.querySelector("#viewer-title");
const closeViewer = document.querySelector("#close-viewer");
const cardTemplate = document.querySelector("#report-card-template");
const runRowTemplate = document.querySelector("#run-row-template");

let reports = [];
let activeFilter = "all";

if (form && targetInput && formMessage) {
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const validation = validateGitHubUrl(targetInput.value);
    if (!validation.ok) {
      showFormMessage(validation.message, true);
      return;
    }

    const url = validation.url;
    const issueUrl = new URL(`https://github.com/${REPO_OWNER}/${REPO_NAME}/issues/new`);
    issueUrl.searchParams.set("title", "Doctor report: request");
    issueUrl.searchParams.set("labels", REPORT_LABEL);
    issueUrl.searchParams.set(
      "body",
      [
        "### Public GitHub skill URL",
        "",
        url,
        "",
        "### Notes",
        "",
        "Submitted from the public SkillSpec Doctor page.",
      ].join("\n"),
    );

    showFormMessage("Opening a prefilled GitHub issue request...");
    window.location.href = issueUrl.toString();
  });
}

if (refreshButton) {
  refreshButton.addEventListener("click", () => {
    loadReports({ force: true });
    loadWorkflowRuns({ force: true });
  });
}

document.querySelectorAll(".filter").forEach((button) => {
  button.addEventListener("click", () => {
    activeFilter = button.dataset.filter || "all";
    document.querySelectorAll(".filter").forEach((item) => {
      item.classList.toggle("active", item === button);
    });
    renderCards();
  });
});

if (closeViewer && reportViewer && reportContent) {
  closeViewer.addEventListener("click", () => {
    reportViewer.hidden = true;
    reportContent.replaceChildren();
  });
}

function validateGitHubUrl(rawValue) {
  const raw = rawValue.trim();
  if (!raw) {
    return { ok: false, message: "Enter a public GitHub repository or folder URL." };
  }
  if (raw.length > 400) {
    return { ok: false, message: "The URL is too long for the public workflow." };
  }

  try {
    const url = new URL(raw);
    if (url.protocol !== "https:") {
      return { ok: false, message: "Use an https://github.com/... URL." };
    }
    if (url.hostname.toLowerCase() !== "github.com") {
      return { ok: false, message: "Only github.com URLs are supported." };
    }
    if (url.username || url.password) {
      return { ok: false, message: "Remove credentials from the URL." };
    }
    if (url.search || url.hash) {
      return { ok: false, message: "Remove query strings and fragments from the URL." };
    }

    const parts = url.pathname.split("/").filter(Boolean);
    if (parts.length < 2) {
      return {
        ok: false,
        message: "Use a repository URL such as https://github.com/org/repo.",
      };
    }

    const safeSegment = /^[A-Za-z0-9._-]+$/;
    if (!safeSegment.test(parts[0]) || !safeSegment.test(parts[1])) {
      return {
        ok: false,
        message: "The owner or repository name contains unsupported characters.",
      };
    }

    return { ok: true, url: url.toString().replace(/\/$/, "") };
  } catch (_error) {
    return { ok: false, message: "The submitted value is not a valid URL." };
  }
}

function showFormMessage(message, isError = false) {
  formMessage.textContent = message;
  formMessage.classList.toggle("error", isError);
}

async function loadReports({ force = false } = {}) {
  if (!reportsGrid || !reportsStatus) {
    return;
  }

  const cached = readCache(REPORTS_CACHE_KEY);
  if (!force && isFreshCache(cached)) {
    reports = cached.data;
    renderCards();
    appendCacheNotice(reportsStatus, cached);
    return;
  }

  reportsStatus.textContent = force
    ? "Refreshing reports from GitHub..."
    : cached
      ? "Refreshing reports from GitHub..."
      : "Loading reports...";
  reportsGrid.replaceChildren();

  try {
    const issues = await fetchDoctorIssues();
    const issueReports = await Promise.all(
      issues
        .filter((issue) => !issue.pull_request)
        .filter(isDoctorIssue)
        .map((issue) => loadIssueReport(issue)),
    );

    reports = issueReports
      .sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))
      .map(compactReport);
    const cache = writeCache(REPORTS_CACHE_KEY, reports);
    renderCards();
    appendCacheNotice(reportsStatus, cache);
  } catch (error) {
    if (cached) {
      const checkedCache = touchCache(REPORTS_CACHE_KEY, cached);
      reports = checkedCache.data;
      renderCards();
      reportsStatus.textContent = `Showing cached reports from ${formatCacheTimestamp(checkedCache.cachedAt)} because GitHub refresh failed: ${error.message}. Next GitHub refresh after ${formatCacheExpiry(checkedCache)}.`;
      return;
    }

    reports = [];
    reportsStatus.textContent = `Could not load public reports: ${error.message}`;
  }
}

async function loadWorkflowRuns({ force = false } = {}) {
  if (!runsList || !runsStatus) {
    return;
  }

  const cached = readCache(RUNS_CACHE_KEY);
  if (!force && isFreshCache(cached)) {
    renderWorkflowRuns(cached.data);
    appendCacheNotice(runsStatus, cached);
    return;
  }

  runsStatus.textContent = force
    ? "Refreshing workflow runs from GitHub..."
    : cached
      ? "Refreshing workflow runs from GitHub..."
      : "Loading workflow runs...";
  runsList.replaceChildren();

  try {
    const runsUrl = new URL(
      `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/actions/workflows/${REPORT_WORKFLOW}/runs`,
    );
    runsUrl.searchParams.set("per_page", "50");

    const payload = await fetchJson(runsUrl);
    const workflowRuns = (payload.workflow_runs || []).map(compactWorkflowRun);
    const cache = writeCache(RUNS_CACHE_KEY, workflowRuns);
    renderWorkflowRuns(workflowRuns);
    appendCacheNotice(runsStatus, cache);
  } catch (error) {
    if (cached) {
      const checkedCache = touchCache(RUNS_CACHE_KEY, cached);
      renderWorkflowRuns(checkedCache.data);
      runsStatus.textContent = `Showing cached workflow runs from ${formatCacheTimestamp(checkedCache.cachedAt)} because GitHub refresh failed: ${error.message}. Next GitHub refresh after ${formatCacheExpiry(checkedCache)}.`;
      return;
    }

    runsStatus.textContent = `Could not load workflow runs: ${error.message}`;
  }
}

function readCache(key) {
  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) {
      return null;
    }

    const parsed = JSON.parse(raw);
    if (!parsed || !Number.isFinite(parsed.cachedAt) || !Array.isArray(parsed.data)) {
      return null;
    }

    return parsed;
  } catch (_error) {
    return null;
  }
}

function writeCache(key, data) {
  const cache = {
    cachedAt: Date.now(),
    checkedAt: Date.now(),
    data,
  };

  try {
    window.localStorage.setItem(key, JSON.stringify(cache));
    return cache;
  } catch (_error) {
    return null;
  }
}

function touchCache(key, cache) {
  const checkedCache = {
    ...cache,
    checkedAt: Date.now(),
  };

  try {
    window.localStorage.setItem(key, JSON.stringify(checkedCache));
    return checkedCache;
  } catch (_error) {
    return checkedCache;
  }
}

function isFreshCache(cache) {
  return Boolean(cache && Date.now() - (cache.checkedAt || cache.cachedAt) < CACHE_TTL_MS);
}

function appendCacheNotice(statusElement, cache) {
  if (!cache) {
    return;
  }

  const current = statusElement.textContent.trim();
  const notice = `Cached ${formatCacheTimestamp(cache.cachedAt)}. Next GitHub refresh after ${formatCacheExpiry(cache)}.`;
  statusElement.textContent = current ? `${current} ${notice}` : notice;
}

function formatCacheTimestamp(cachedAt) {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(cachedAt));
}

function formatCacheExpiry(cache) {
  const checkedAt = cache.checkedAt || cache.cachedAt;
  return new Intl.DateTimeFormat(undefined, {
    timeStyle: "short",
  }).format(new Date(checkedAt + CACHE_TTL_MS));
}

function compactReport(report) {
  return {
    issue: {
      number: report.issue.number,
      html_url: report.issue.html_url,
      title: report.issue.title,
    },
    status: report.status,
    title: report.title,
    shape: report.shape,
    verdict: report.verdict,
    risk: report.risk,
    markdown: report.markdown,
    updatedAt: report.updatedAt,
  };
}

function compactWorkflowRun(run) {
  return {
    id: run.id,
    conclusion: run.conclusion,
    status: run.status,
    display_title: run.display_title,
    name: run.name,
    run_number: run.run_number,
    run_attempt: run.run_attempt,
    created_at: run.created_at,
    html_url: run.html_url,
  };
}

function renderWorkflowRuns(workflowRuns) {
  if (!runsList || !runsStatus || !runRowTemplate) {
    return;
  }

  runsList.replaceChildren();
  if (workflowRuns.length === 0) {
    runsStatus.textContent = "No workflow runs found yet.";
    return;
  }

  runsStatus.textContent = `${workflowRuns.length} run${workflowRuns.length === 1 ? "" : "s"} shown.`;
  for (const run of workflowRuns) {
    const node = runRowTemplate.content.firstElementChild.cloneNode(true);
    const status = run.conclusion || run.status || "pending";
    const statusPill = node.querySelector(".status-pill");
    const title = node.querySelector("h3");
    const meta = node.querySelector("p");
    const link = node.querySelector("a");

    statusPill.textContent = status;
    statusPill.classList.add(status === "success" ? "success" : status === "failure" ? "error" : "pending");
    const fullRunTitle = run.display_title || run.name || `Run ${run.id}`;
    title.textContent = compactRunTitle(fullRunTitle);
    title.title = fullRunTitle;
    meta.textContent = [
      `Run #${run.run_number}`,
      run.run_attempt > 1 ? `attempt ${run.run_attempt}` : null,
      formatDate(run.created_at),
    ]
      .filter(Boolean)
      .join(" · ");
    link.href = run.html_url;
    runsList.appendChild(node);
  }
}

async function fetchDoctorIssues() {
  const allIssues = [];
  for (let page = 1; page <= 5; page += 1) {
    const issuesUrl = new URL(
      `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/issues`,
    );
    issuesUrl.searchParams.set("state", "all");
    issuesUrl.searchParams.set("per_page", "100");
    issuesUrl.searchParams.set("page", String(page));

    const issues = await fetchJson(issuesUrl);
    allIssues.push(...issues);
    if (issues.length < 100) {
      break;
    }
  }
  return allIssues;
}

function isDoctorIssue(issue) {
  const labels = issue.labels || [];
  return (
    /^Doctor report:/i.test(issue.title || "") ||
    labels.some((label) => label.name === REPORT_LABEL)
  );
}

async function loadIssueReport(issue) {
  const comments = await fetchJson(issue.comments_url);
  const reportComment = comments
    .filter((comment) => comment.body && comment.body.includes(REPORT_MARKER))
    .at(-1);

  if (!reportComment) {
    return {
      issue,
      status: "pending",
      title: titleFromIssue(issue),
      shape: "pending",
      verdict: "waiting for workflow",
      risk: "pending",
      markdown: "## Report pending\n\nThe workflow has not posted a report yet.",
      updatedAt: issue.updated_at,
    };
  }

  const markdown = cleanReportMarkdown(reportComment.body);
  const isError =
    /could not run a public report|could not complete the public report|report failed/i.test(
      markdown,
    );
  const summary = parseSummary(markdown);

  return {
    issue,
    status: isError ? "error" : "success",
    title: summary.target || titleFromIssue(issue),
    shape: summary.shape || (isError ? "not evaluated" : "unknown"),
    verdict: summary.verdict || (isError ? "error" : "reported"),
    risk: summary.risk || (isError ? "error" : "see report"),
    markdown,
    updatedAt: reportComment.updated_at || issue.updated_at,
  };
}

function renderCards() {
  if (!reportsGrid || !reportsStatus || !cardTemplate) {
    return;
  }

  reportsGrid.replaceChildren();
  const visibleReports = reports.filter((report) => {
    return activeFilter === "all" || report.status === activeFilter;
  });

  if (visibleReports.length === 0) {
    reportsStatus.textContent =
      reports.length === 0 ? "No doctor reports found yet." : "No reports match this filter.";
    return;
  }

  reportsStatus.textContent = `${visibleReports.length} report${visibleReports.length === 1 ? "" : "s"} shown.`;

  for (const report of visibleReports) {
    const node = cardTemplate.content.firstElementChild.cloneNode(true);
    const status = node.querySelector(".status-pill");
    const issueNumber = node.querySelector(".issue-number");
    const title = node.querySelector("h3");
    const viewButton = node.querySelector('[data-action="view"]');
    const issueLink = node.querySelector('[data-action="issue"]');

    status.textContent = report.status;
    status.classList.add(report.status);
    issueNumber.textContent = `#${report.issue.number}`;
    title.textContent = compactTitle(report.title);
    title.title = report.title;
    node.querySelector('[data-field="verdict"]').textContent = report.verdict;
    node.querySelector('[data-field="shape"]').textContent = report.shape;
    node.querySelector('[data-field="risk"]').textContent = report.risk;
    issueLink.href = report.issue.html_url;

    if (viewButton && viewerTitle && reportContent && reportViewer) {
      viewButton.addEventListener("click", () => {
        const model = buildReportModel(report);
        viewerTitle.textContent = model.viewerTitle;
        reportContent.innerHTML = renderReportDashboard(model);
        reportViewer.hidden = false;
        reportViewer.scrollIntoView({ behavior: "smooth", block: "start" });
      });
    }

    reportsGrid.appendChild(node);
  }
}

function parseSummary(markdown) {
  const target = matchLine(markdown, /\*\*Target:\*\*\s+(.*)/);
  const shape = matchLine(markdown, /\*\*Shape:\*\*\s+`?([^`\n-]+)`?/);
  const verdict = matchLine(markdown, /- \*\*Verdict:\*\*\s+(.+)/);
  const decision = matchLine(markdown, /- \*\*Decision:\*\*\s+(.+)/);
  const risk =
    matchLine(markdown, /- \*\*Agent follow-through risk:\*\*\s+(.+)/) ||
    matchLine(markdown, /- \*\*Follow-through risk:\*\*\s+(.+)/);
  return {
    target: stripMarkdown(target),
    shape: stripMarkdown(shape),
    verdict: stripMarkdown(verdict || decision),
    risk: stripMarkdown(risk || (decision ? "shape gate" : "")),
  };
}

function matchLine(text, regex) {
  const match = text.match(regex);
  return match ? match[1].trim() : "";
}

function cleanReportMarkdown(body) {
  let markdown = body.replace(REPORT_MARKER, "").trim();
  markdown = markdown.replace(/\n+Full artifacts:[\s\S]*$/i, "").trim();
  return markdown;
}

function titleFromIssue(issue) {
  return issue.title.replace(/^Doctor report:\s*/i, "").trim() || issue.title;
}

function compactTitle(title) {
  const cleanTitle = title.replace(/^Doctor report:\s*/i, "").trim();
  return compactGitHubPath(cleanTitle) || compactMiddle(cleanTitle, 74, 34, 28);
}

function compactRunTitle(title) {
  const cleanTitle = title.replace(/^Doctor report:\s*/i, "").trim();
  const compact = compactGitHubPath(cleanTitle) || compactMiddle(cleanTitle, 92, 42, 32);
  return title.startsWith(cleanTitle) ? compact : `Doctor report: ${compact}`;
}

function compactGitHubPath(value) {
  let url;
  try {
    url = new URL(value);
  } catch (_error) {
    return "";
  }

  if (url.hostname.toLowerCase() !== "github.com") {
    return "";
  }

  const parts = url.pathname.split("/").filter(Boolean);
  if (parts.length < 2) {
    return value;
  }

  const ownerRepo = `${parts[0]}/${parts[1]}`;
  let pathParts = parts.slice(2);
  if (/^(tree|blob)$/i.test(pathParts[0] || "") && pathParts.length >= 3) {
    pathParts = pathParts.slice(2);
  }

  if (pathParts.length === 0) {
    return ownerRepo;
  }

  const tail = pathParts.at(-1);
  return `${ownerRepo}/.../${tail}`;
}

function compactMiddle(value, maxLength, startLength, endLength) {
  if (value.length <= maxLength) {
    return value;
  }
  return `${value.slice(0, startLength)}...${value.slice(-endLength)}`;
}

function stripMarkdown(value) {
  if (!value) {
    return "";
  }
  return value
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/`/g, "")
    .replace(/\*\*/g, "")
    .trim();
}

function formatDate(value) {
  if (!value) {
    return "unknown date";
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

async function fetchJson(url) {
  const response = await fetch(url, {
    headers: {
      Accept: "application/vnd.github+json",
    },
  });
  if (!response.ok) {
    if (response.status === 403) {
      throw new Error("GitHub API rate limit reached. Try again later.");
    }
    throw new Error(`GitHub returned ${response.status}`);
  }
  return response.json();
}

function buildReportModel(report) {
  const markdown = report.markdown || "";
  const summary = parseSummary(markdown);
  const shapeKind = normalizeShapeKind(summary.shape || report.shape);
  const shape = shapeProfile(shapeKind);
  const baseline = extractSection(markdown, "Current Skill Baseline");
  const shapeContract = extractSection(markdown, "Shape Contract");
  const surface = extractSection(markdown, "Surface");
  const workspace = extractSection(markdown, "Workspace Identity");
  const nextActions = extractSection(markdown, "Next Actions");
  const risk = parseRiskValue(
    fieldValue(baseline, "Agent follow-through risk") ||
      fieldValue(baseline, "Follow-through risk") ||
      report.risk,
  );
  const discovery = parseRiskValue(fieldValue(baseline, "Discovery risk"));
  const packageRollup = fieldValue(baseline, "Package risk rollup");
  const nextCommand = extractNextCommand(markdown);
  const nextSteps = extractMarkdownList(nextActions)
    .filter((item) => !/^Recommended next action$/i.test(item))
    .slice(0, 4);
  const metrics = {
    skillFiles: numberFromField(shapeContract, "Skill files"),
    packages: numberFromField(shapeContract, "Packages"),
    namespaces: numberFromField(shapeContract, "Namespaces"),
    pluginRoots: numberFromField(shapeContract, "Plugin roots"),
    sourceFiles: numberFromField(workspace, "Source files"),
    uniqueContent: numberFromPattern(workspace, /`([0-9,]+)` unique byte content item/),
    repeatedOccurrences: numberFromPattern(workspace, /`([0-9,]+)` repeated occurrence/),
    repeatedGroups: numberFromPattern(workspace, /`([0-9,]+)` referentiable group/),
    totalTokens: numberFromPattern(workspace, /approximately `([0-9,]+)` total/),
    uniqueTokens: numberFromPattern(workspace, /`([0-9,]+)` unique if referenced/),
    repeatedTokens: numberFromPattern(workspace, /unique if referenced, `([0-9,]+)` repeated/),
    activationTokens: numberFromPattern(surface, /approximately `([0-9,]+)` token/),
    activationLines: numberFromPattern(surface, /Activation body:\*\* `([0-9,]+)` line/),
    unmappedFiles: numberFromField(surface, "Unmapped package files"),
  };
  const findings = extractFindings(markdown).slice(0, 4);
  const packages = extractPackages(markdown).slice(0, 6);
  const isNonSkill = shapeKind === "non_skill_repository";
  const outcome = outcomeForReport(shapeKind, risk, packageRollup);
  const nextAction = nextActionForReport(shapeKind, risk, nextSteps[0], nextCommand);
  const target = summary.target || report.title || report.issue.title;

  return {
    issue: report.issue,
    issueNumber: report.issue.number,
    viewerTitle: `Report #${report.issue.number}`,
    target,
    compactTarget: compactGitHubPath(target) || compactMiddle(stripMarkdown(target), 70, 34, 24),
    shapeKind,
    shape,
    status: report.status,
    verdict: stripMarkdown(summary.verdict || report.verdict || ""),
    risk,
    discovery,
    packageRollup: stripMarkdown(packageRollup),
    metrics,
    findings,
    packages,
    nextAction,
    nextCommand,
    nextSteps,
    outcome,
    isNonSkill,
    markdown,
  };
}

function renderReportDashboard(model) {
  if (model.isNonSkill) {
    return renderCompactShapeOnlyReport(model);
  }

  const scoreLabel = model.risk.score === null ? "Not scored" : `${model.risk.score}/100`;
  const packageSummary = model.packageRollup || packageSummaryForMetrics(model.metrics);
  return [
    `<div class="report-dashboard risk-${riskClass(model.risk.level)}">`,
    `<section class="dashboard-hero">`,
    `<div class="dashboard-hero-copy">`,
    `<p class="dashboard-kicker">${escapeHtml(model.shape.label)} assessment</p>`,
    `<h1>${escapeHtml(model.outcome.title)}</h1>`,
    `<p>${escapeHtml(model.outcome.copy)}</p>`,
    `<div class="dashboard-chip-row">`,
    dashboardChip("Target", model.compactTarget),
    dashboardChip("Shape", model.shape.shortLabel),
    dashboardChip("Issue", `#${model.issueNumber}`),
    `</div>`,
    `</div>`,
    `<div class="dashboard-score-panel">`,
    `<span>Follow-through risk</span>`,
    `<strong>${escapeHtml(scoreLabel)}</strong>`,
    `<p>${escapeHtml(model.risk.levelLabel)}</p>`,
    `</div>`,
    `</section>`,
    renderNextAction(model),
    `<section class="dashboard-metrics" aria-label="Report metrics">`,
    metricCard("Shape", model.shape.shortLabel, model.shape.explain),
    metricCard("Packages", displayNumber(model.metrics.packages || model.metrics.skillFiles), packageSummary),
    metricCard("Namespaces", displayNumber(model.metrics.namespaces || model.metrics.pluginRoots), namespaceCopy(model)),
    metricCard("Reusable content", reusableContentValue(model.metrics), reusableContentCopy(model.metrics)),
    metricCard("Activation load", activationValue(model.metrics), activationCopy(model.metrics)),
    metricCard("Discovery", model.discovery.score === null ? "Not scored" : `${model.discovery.score}/100`, model.discovery.levelLabel),
    `</section>`,
    `<section class="dashboard-learn-grid" aria-label="How to read this report">`,
    teachCard("What Doctor checked", model.shape.teach),
    teachCard("Why shape matters", model.shape.identity),
    teachCard("How to use the score", scoreTeaching(model.risk)),
    `</section>`,
    `<section class="dashboard-detail-grid">`,
    renderFindingsPanel(model.findings),
    renderPackagePanel(model),
    `</section>`,
    renderTechnicalAppendix(model.markdown),
    `</div>`,
  ].join("");
}

function renderCompactShapeOnlyReport(model) {
  return [
    `<div class="report-dashboard report-dashboard-compact">`,
    `<section class="dashboard-hero compact">`,
    `<div class="dashboard-hero-copy">`,
    `<p class="dashboard-kicker">Shape gate</p>`,
    `<h1>No skill package found.</h1>`,
    `<p>Doctor stopped after the cheap shape check. Detailed report space is reserved for folders that contain one or more <code>SKILL.md</code> files.</p>`,
    `<div class="dashboard-chip-row">`,
    dashboardChip("Target", model.compactTarget),
    dashboardChip("Shape", model.shape.shortLabel),
    dashboardChip("Skill files", "0"),
    `</div>`,
    `</div>`,
    `<div class="dashboard-score-panel">`,
    `<span>Next action</span>`,
    `<strong>Choose a skill folder</strong>`,
    `<p>Submit a GitHub path that contains SKILL.md.</p>`,
    `</div>`,
    `</section>`,
    renderTechnicalAppendix(model.markdown),
    `</div>`,
  ].join("");
}

function renderNextAction(model) {
  return [
    `<section class="dashboard-next-action">`,
    `<div>`,
    `<span>Do next</span>`,
    `<strong>${escapeHtml(model.nextAction.title)}</strong>`,
    `<p>${escapeHtml(model.nextAction.copy)}</p>`,
    `</div>`,
    model.nextCommand
      ? `<pre><code>${escapeHtml(model.nextCommand)}</code></pre>`
      : `<a href="${escapeAttribute(model.issue.html_url || "#")}">Open GitHub issue</a>`,
    `</section>`,
  ].join("");
}

function renderFindingsPanel(findings) {
  const items = findings.length
    ? findings
        .map(
          (finding) => `
            <article>
              <span class="${riskClass(finding.severity)}">${escapeHtml(finding.severityLabel)}</span>
              <h3>${escapeHtml(finding.title)}</h3>
              <p>${escapeHtml(finding.fix || finding.evidence || "Review this finding before install.")}</p>
            </article>
          `,
        )
        .join("")
    : `<article><span class="low">clear</span><h3>No top findings</h3><p>No static structure issues were listed in the public report.</p></article>`;

  return `<section class="dashboard-panel"><div class="panel-title"><span>Priority findings</span><strong>Fix before trust</strong></div><div class="dashboard-finding-list">${items}</div></section>`;
}

function renderPackagePanel(model) {
  const packageRows = model.packages.length
    ? model.packages
        .map(
          (item) => `
            <tr>
              <td>${escapeHtml(compactMiddle(item.name, 38, 20, 14))}</td>
              <td>${escapeHtml(compactMiddle(item.path || item.role, 46, 22, 18))}</td>
              <td><span class="${riskClass(item.drift)}">${escapeHtml(item.drift || "unknown")}</span></td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td>${escapeHtml(model.shape.shortLabel)}</td><td>${escapeHtml(model.shape.explain)}</td><td><span class="${riskClass(model.risk.level)}">${escapeHtml(model.risk.levelLabel)}</span></td></tr>`;

  return `
    <section class="dashboard-panel">
      <div class="panel-title"><span>Folder intelligence</span><strong>${escapeHtml(model.shape.identityHeadline)}</strong></div>
      <dl class="folder-metadata">
        <div><dt>Source files</dt><dd>${displayNumber(model.metrics.sourceFiles)}</dd></div>
        <div><dt>Skill files</dt><dd>${displayNumber(model.metrics.skillFiles)}</dd></div>
        <div><dt>Unique content</dt><dd>${displayNumber(model.metrics.uniqueContent)}</dd></div>
        <div><dt>Repeated refs</dt><dd>${displayNumber(model.metrics.repeatedOccurrences)}</dd></div>
      </dl>
      <div class="package-table-wrap">
        <table>
          <thead><tr><th>Package</th><th>Path / role</th><th>Drift</th></tr></thead>
          <tbody>${packageRows}</tbody>
        </table>
      </div>
    </section>
  `;
}

function renderTechnicalAppendix(markdown) {
  return `
    <details class="technical-appendix">
      <summary>Technical report Markdown</summary>
      <div class="markdown-render">${renderMarkdown(markdown)}</div>
    </details>
  `;
}

function dashboardChip(label, value) {
  return `<span><em>${escapeHtml(label)}</em>${escapeHtml(value || "Not reported")}</span>`;
}

function metricCard(label, value, copy) {
  return `<article><span>${escapeHtml(label)}</span><strong>${escapeHtml(value || "Not reported")}</strong><p>${escapeHtml(copy || "")}</p></article>`;
}

function teachCard(title, copy) {
  return `<article><h3>${escapeHtml(title)}</h3><p>${escapeHtml(copy)}</p></article>`;
}

function extractSection(markdown, heading) {
  const pattern = new RegExp(`(?:^|\\n)## ${escapeRegExp(heading)}\\n\\n([\\s\\S]*?)(?=\\n## |$)`);
  const match = markdown.match(pattern);
  return match ? match[1].trim() : "";
}

function fieldValue(section, label) {
  if (!section) {
    return "";
  }
  const pattern = new RegExp(`(?:^|\\n)- \\*\\*${escapeRegExp(label)}:\\*\\*\\s+([^\\n]+)`);
  const match = section.match(pattern);
  return match ? stripMarkdown(match[1]) : "";
}

function numberFromField(section, label) {
  return numberFromText(fieldValue(section, label));
}

function numberFromPattern(text, pattern) {
  const match = text.match(pattern);
  return match ? numberFromText(match[1]) : null;
}

function numberFromText(value) {
  const match = String(value || "").match(/([0-9][0-9,]*)/);
  return match ? Number(match[1].replace(/,/g, "")) : null;
}

function parseRiskValue(value) {
  const clean = stripMarkdown(value || "");
  const level = (clean.match(/\b(critical|high|medium|low|error|pending)\b/i) || [])[1]?.toLowerCase() || "";
  const scoreMatch = clean.match(/([0-9]{1,3})\s*\/\s*100/);
  const score = scoreMatch ? Number(scoreMatch[1]) : null;
  return {
    score,
    level,
    levelLabel: riskLabel(level, score),
  };
}

function riskLabel(level, score) {
  if (level) {
    return `${level} risk`;
  }
  if (score === null) {
    return "not scored";
  }
  if (score >= 75) {
    return "critical risk";
  }
  if (score >= 50) {
    return "high risk";
  }
  if (score >= 25) {
    return "medium risk";
  }
  return "low risk";
}

function riskClass(level) {
  const clean = String(level || "").toLowerCase();
  return ["critical", "high", "medium", "low", "error", "pending"].includes(clean)
    ? clean
    : "neutral";
}

function normalizeShapeKind(value) {
  const clean = stripMarkdown(value || "").split(/\s+-\s+/)[0].trim();
  return clean || "unknown";
}

function shapeProfile(kind) {
  const profiles = {
    simple_skill: {
      label: "Single skill folder",
      shortLabel: "Single skill",
      explain: "One atomic SKILL.md package.",
      teach: "Doctor checks whether one package can be trusted as raw prose: discovery metadata, activation load, dependencies, references, tests, and proof.",
      identity: "There is one package identity. The key question is whether its load-bearing prose has been promoted into a contract.",
      identityHeadline: "One package identity",
    },
    multi_skill_workspace: {
      label: "Multi-skill folder",
      shortLabel: "Multi-skill",
      explain: "Many SKILL.md packages under one folder.",
      teach: "Doctor rolls up package risks while preserving each folder path as its own package identity.",
      identity: "Path is identity. Repeated names or repeated content can be referenceable, but should not be flattened.",
      identityHeadline: "Path identities matter",
    },
    plugin_workspace: {
      label: "Plugin workspace",
      shortLabel: "Plugin",
      explain: "Plugin namespace plus skill folders and shared files.",
      teach: "Doctor checks the plugin root, package count, namespaces, repeated names, and repeated content before import or install.",
      identity: "Plugin parent, namespace, shared files, and skills folders are all part of runtime identity.",
      identityHeadline: "Preserve plugin shape",
    },
    entry_skill_with_subskills: {
      label: "Entry skill with subskills",
      shortLabel: "Entry + subskills",
      explain: "A root SKILL.md points at nested skill packages.",
      teach: "Doctor treats the entry skill and nested skills as separate identities so import can preserve cross-skill references.",
      identity: "The root entry is not the whole package set. Nested folders need their own review loop.",
      identityHeadline: "Entry plus nested identities",
    },
    non_skill_repository: {
      label: "Source repository",
      shortLabel: "No skill",
      explain: "No SKILL.md entrypoint was found.",
      teach: "Doctor stops early for generic source repositories.",
      identity: "There is no skill identity to score yet.",
      identityHeadline: "No skill identity",
    },
  };
  return profiles[kind] || {
    label: "Unknown shape",
    shortLabel: "Unknown",
    explain: "Shape was not recognized.",
    teach: "Inspect the technical report before importing or installing.",
    identity: "Do not infer install readiness from an unknown shape.",
    identityHeadline: "Inspect shape first",
  };
}

function outcomeForReport(shapeKind, risk, packageRollup) {
  if (shapeKind === "plugin_workspace") {
    return {
      title: "Preserve the plugin shape before import.",
      copy: "This report is about namespace, package, and shared-folder identity as much as prose quality. Do not flatten it into independent skills.",
    };
  }
  if (shapeKind === "multi_skill_workspace" || shapeKind === "entry_skill_with_subskills") {
    return {
      title: "Treat this as a workspace, not one skill.",
      copy: "Map the folder, keep every package path addressable, then port packages in risk order.",
    };
  }
  if (risk.score !== null && risk.score >= 75) {
    return {
      title: "Do not install this raw skill yet.",
      copy: "The current prose shape has critical follow-through risk. Port the behavior into SkillSpec before relying on it.",
    };
  }
  if (risk.score !== null && risk.score >= 50) {
    return {
      title: "Port before trusting the run.",
      copy: "The skill can be useful, but its current shape leaves too much behavior in prose.",
    };
  }
  return {
    title: packageRollup ? "Review the highest-risk package first." : "This skill is readable, now prove it.",
    copy: "Use the next command, preserve the source shape, and make proof explicit before install.",
  };
}

function nextActionForReport(shapeKind, risk, firstStep, command) {
  if (shapeKind === "plugin_workspace") {
    return {
      title: "Map as plugin workspace",
      copy: "Keep plugin parent folders, namespace, shared files, and each skills/ folder intact. Then import package-by-package.",
    };
  }
  if (shapeKind === "multi_skill_workspace" || shapeKind === "entry_skill_with_subskills") {
    return {
      title: "Map the workspace",
      copy: "Preserve package paths, run workspace map/validate/import, then port each generated draft in manifest order.",
    };
  }
  if (command) {
    return {
      title: risk.score !== null && risk.score >= 50 ? "Port before install" : "Convert and prove",
      copy: firstStep || "Run the recommended command, then review routes, rules, dependencies, tests, and proof.",
    };
  }
  return {
    title: "Open the report artifacts",
    copy: firstStep || "Inspect the full report and choose the next SkillSpec command.",
  };
}

function packageSummaryForMetrics(metrics) {
  if (metrics.packages && metrics.skillFiles && metrics.packages !== metrics.skillFiles) {
    return `${metrics.skillFiles} skill file(s), ${metrics.packages} package identity item(s).`;
  }
  if (metrics.skillFiles) {
    return `${metrics.skillFiles} SKILL.md file(s) found.`;
  }
  return "Package count was not reported.";
}

function namespaceCopy(model) {
  if (model.metrics.pluginRoots) {
    return `${model.metrics.pluginRoots} plugin root(s) detected.`;
  }
  if (model.metrics.namespaces) {
    return `${model.metrics.namespaces} namespace bucket(s) in this workspace.`;
  }
  return "No namespace split was reported.";
}

function reusableContentValue(metrics) {
  if (metrics.repeatedOccurrences !== null) {
    return `${displayNumber(metrics.repeatedOccurrences)} repeated`;
  }
  if (metrics.uniqueContent !== null) {
    return `${displayNumber(metrics.uniqueContent)} unique`;
  }
  return "Not reported";
}

function reusableContentCopy(metrics) {
  if (metrics.repeatedTokens) {
    return `About ${displayNumber(metrics.repeatedTokens)} repeated token(s) can be referenced instead of copied.`;
  }
  if (metrics.uniqueContent) {
    return "Repeated content was not prominent in the public report.";
  }
  return "Use JSON artifacts for full content identity details.";
}

function activationValue(metrics) {
  if (metrics.activationTokens) {
    return `~${displayNumber(metrics.activationTokens)} tokens`;
  }
  if (metrics.activationLines) {
    return `${displayNumber(metrics.activationLines)} lines`;
  }
  return "Not reported";
}

function activationCopy(metrics) {
  if (metrics.unmappedFiles) {
    return `${displayNumber(metrics.unmappedFiles)} package-local file(s) were not clearly reachable.`;
  }
  return "Lower activation load leaves more room for task state, tools, and proof.";
}

function scoreTeaching(risk) {
  if (risk.score === null) {
    return "No numeric risk was computed for this surface. Use shape and next action first.";
  }
  return "Higher risk means more behavior is trapped in prose where a model can skip, reorder, improvise, or fail to prove it.";
}

function extractNextCommand(markdown) {
  const match = markdown.match(/\*\*Recommended next action\*\*[\s\S]*?```(?:text)?\n([\s\S]*?)\n```/);
  if (match) {
    return match[1].trim();
  }
  const shapeMatch = markdown.match(/\*\*Next command\*\*[\s\S]*?```(?:text)?\n([\s\S]*?)\n```/);
  return shapeMatch ? shapeMatch[1].trim() : "";
}

function extractMarkdownList(section) {
  return (section.match(/^- .+$/gm) || []).map((line) => stripMarkdown(line.replace(/^- /, "")));
}

function extractFindings(markdown) {
  const findingsSection = extractSection(markdown, "Findings");
  const matches = [...findingsSection.matchAll(/###\s+\d+\.\s+([A-Z]+):\s+(.+?)\n\n([\s\S]*?)(?=\n###\s+\d+\.|$)/g)];
  return matches.map((match) => {
    const body = match[3] || "";
    return {
      severity: match[1].toLowerCase(),
      severityLabel: match[1].toLowerCase(),
      title: stripMarkdown(match[2]),
      evidence: stripMarkdown((body.match(/\*\*Evidence:\*\*\s+(.+)/) || [])[1] || ""),
      fix: stripMarkdown((body.match(/\*\*Fix:\*\*\s+(.+)/) || [])[1] || ""),
    };
  });
}

function extractPackages(markdown) {
  const packagesSection = extractSection(markdown, "Packages");
  const matches = [...packagesSection.matchAll(/###\s+(.+?)\n\n([\s\S]*?)(?=\n###\s+|$)/g)];
  return matches.map((match) => {
    const body = match[2] || "";
    return {
      name: stripMarkdown(match[1]),
      path: fieldValue(body, "Path"),
      role: fieldValue(body, "Role"),
      drift: (fieldValue(body, "Drift risk") || "unknown").toLowerCase(),
      discovery: (fieldValue(body, "Discovery risk") || "unknown").toLowerCase(),
    };
  });
}

function displayNumber(value) {
  return value === null || value === undefined ? "Not reported" : new Intl.NumberFormat().format(value);
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function renderMarkdown(markdown) {
  const blocks = [];
  const lines = markdown.split(/\r?\n/);
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];

    if (!line.trim()) {
      index += 1;
      continue;
    }

    if (line.startsWith("```")) {
      const language = line.slice(3).trim();
      const codeLines = [];
      index += 1;
      while (index < lines.length && !lines[index].startsWith("```")) {
        codeLines.push(lines[index]);
        index += 1;
      }
      index += 1;
      blocks.push(
        `<pre><code${language ? ` data-language="${escapeAttribute(language)}"` : ""}>${escapeHtml(codeLines.join("\n"))}</code></pre>`,
      );
      continue;
    }

    const heading = line.match(/^(#{1,3})\s+(.+)$/);
    if (heading) {
      const level = heading[1].length;
      blocks.push(`<h${level}>${renderInline(heading[2])}</h${level}>`);
      index += 1;
      continue;
    }

    if (line.startsWith(">")) {
      const quoteLines = [];
      while (index < lines.length && lines[index].startsWith(">")) {
        quoteLines.push(lines[index].replace(/^>\s?/, ""));
        index += 1;
      }
      blocks.push(`<blockquote>${renderInline(quoteLines.join(" "))}</blockquote>`);
      continue;
    }

    if (/^- /.test(line)) {
      const items = [];
      while (index < lines.length && /^- /.test(lines[index])) {
        items.push(`<li>${renderInline(lines[index].replace(/^- /, ""))}</li>`);
        index += 1;
      }
      blocks.push(`<ul>${items.join("")}</ul>`);
      continue;
    }

    const paragraphLines = [];
    while (
      index < lines.length &&
      lines[index].trim() &&
      !lines[index].startsWith("```") &&
      !/^(#{1,3})\s+/.test(lines[index]) &&
      !/^- /.test(lines[index]) &&
      !lines[index].startsWith(">")
    ) {
      paragraphLines.push(lines[index]);
      index += 1;
    }
    blocks.push(`<p>${renderInline(paragraphLines.join(" "))}</p>`);
  }

  return blocks.join("\n");
}

function renderInline(value) {
  let escaped = escapeHtml(value);
  escaped = escaped.replace(/\[([^\]]+)\]\(((?:https?:\/\/|github\.com\/)[^)\s]+)\)/g, (_match, label, url) => {
    const href = url.startsWith("http") ? url : `https://${url}`;
    return `<a href="${escapeAttribute(href)}" target="_blank" rel="noreferrer">${label}</a>`;
  });
  escaped = escaped.replace(/`([^`]+)`/g, "<code>$1</code>");
  escaped = escaped.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  return escaped;
}

function escapeHtml(value) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function escapeAttribute(value) {
  return escapeHtml(value).replace(/`/g, "&#96;");
}

if (reportsGrid) {
  loadReports();
}

if (runsList) {
  loadWorkflowRuns();
}
