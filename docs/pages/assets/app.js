const REPO_OWNER = "modiqo";
const REPO_NAME = "skillspec";
const REPORT_LABEL = "doctor-report";
const REPORT_MARKER = "<!-- skillspec-doctor-report -->";
const REPORT_WORKFLOW = "doctor-report.yml";

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

form.addEventListener("submit", (event) => {
  event.preventDefault();
  const validation = validateGitHubUrl(targetInput.value);
  if (!validation.ok) {
    showFormMessage(validation.message, true);
    return;
  }

  const url = validation.url;
  const issueUrl = new URL(`https://github.com/${REPO_OWNER}/${REPO_NAME}/issues/new`);
  issueUrl.searchParams.set("title", `Doctor report: ${url}`);
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

refreshButton.addEventListener("click", () => {
  loadReports();
  loadWorkflowRuns();
});

document.querySelectorAll(".filter").forEach((button) => {
  button.addEventListener("click", () => {
    activeFilter = button.dataset.filter || "all";
    document.querySelectorAll(".filter").forEach((item) => {
      item.classList.toggle("active", item === button);
    });
    renderCards();
  });
});

closeViewer.addEventListener("click", () => {
  reportViewer.hidden = true;
  reportContent.replaceChildren();
});

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

async function loadReports() {
  reportsStatus.textContent = "Loading reports...";
  reportsGrid.replaceChildren();

  try {
    const issues = await fetchDoctorIssues();
    const issueReports = await Promise.all(
      issues
        .filter((issue) => !issue.pull_request)
        .filter(isDoctorIssue)
        .map((issue) => loadIssueReport(issue)),
    );

    reports = issueReports.sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt));
    renderCards();
  } catch (error) {
    reports = [];
    reportsStatus.textContent = `Could not load public reports: ${error.message}`;
  }
}

async function loadWorkflowRuns() {
  runsStatus.textContent = "Loading workflow runs...";
  runsList.replaceChildren();

  try {
    const runsUrl = new URL(
      `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/actions/workflows/${REPORT_WORKFLOW}/runs`,
    );
    runsUrl.searchParams.set("per_page", "50");

    const payload = await fetchJson(runsUrl);
    const workflowRuns = payload.workflow_runs || [];
    renderWorkflowRuns(workflowRuns);
  } catch (error) {
    runsStatus.textContent = `Could not load workflow runs: ${error.message}`;
  }
}

function renderWorkflowRuns(workflowRuns) {
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

    viewButton.addEventListener("click", () => {
      viewerTitle.textContent = `Report #${report.issue.number}`;
      reportContent.innerHTML = renderMarkdown(report.markdown);
      reportViewer.hidden = false;
      reportViewer.scrollIntoView({ behavior: "smooth", block: "start" });
    });

    reportsGrid.appendChild(node);
  }
}

function parseSummary(markdown) {
  const target = matchLine(markdown, /\*\*Target:\*\*\s+(.*)/);
  const shape = matchLine(markdown, /\*\*Shape:\*\*\s+`?([^`\n-]+)`?/);
  const verdict = matchLine(markdown, /- \*\*Verdict:\*\*\s+(.+)/);
  const risk =
    matchLine(markdown, /- \*\*Agent follow-through risk:\*\*\s+(.+)/) ||
    matchLine(markdown, /- \*\*Follow-through risk:\*\*\s+(.+)/);
  return {
    target: stripMarkdown(target),
    shape: stripMarkdown(shape),
    verdict: stripMarkdown(verdict),
    risk: stripMarkdown(risk),
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
  escaped = escaped.replace(
    /\[([^\]]+)\]\((https:\/\/[^)\s]+)\)/g,
    '<a href="$2" target="_blank" rel="noreferrer">$1</a>',
  );
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

loadReports();
loadWorkflowRuns();
