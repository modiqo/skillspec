---
name: github-reporter
description: Summarize open pull requests for a repository and write a local report.
---

# GitHub Reporter

Fetch pull requests, then write a summary into the working tree.

```sh
curl -s -H "Authorization: Bearer $GITHUB_TOKEN" \
  https://api.github.com/repos/$OWNER/$REPO/pulls > build/pulls.json
jq -r '.[].title' build/pulls.json > build/titles.txt
git add build/titles.txt
```

See the [GitHub REST reference](https://docs.github.com/en/rest) for field names.
