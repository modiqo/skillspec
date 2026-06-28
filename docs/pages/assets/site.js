const STAR_REPO_OWNER = "modiqo";
const STAR_REPO_NAME = "skillspec";

hydrateGitHubStars();

async function hydrateGitHubStars() {
  const starTargets = [...document.querySelectorAll("[data-github-stars]")];
  if (starTargets.length === 0) {
    return;
  }

  try {
    const response = await fetch(
      `https://api.github.com/repos/${STAR_REPO_OWNER}/${STAR_REPO_NAME}`,
      {
        headers: {
          Accept: "application/vnd.github+json",
        },
      },
    );
    if (!response.ok) {
      return;
    }

    const payload = await response.json();
    const stars = Number(payload.stargazers_count);
    if (!Number.isFinite(stars)) {
      return;
    }

    const label = formatStars(stars);
    for (const target of starTargets) {
      target.textContent = label;
    }
  } catch (_error) {
    // The link remains useful even when the public GitHub API is unavailable.
  }
}

function formatStars(value) {
  return new Intl.NumberFormat("en", {
    maximumFractionDigits: value >= 1000 ? 1 : 0,
    notation: value >= 1000 ? "compact" : "standard",
  })
    .format(value)
    .toLowerCase();
}
