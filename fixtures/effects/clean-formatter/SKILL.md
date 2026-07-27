---
name: clean-formatter
description: Format Markdown tables in the current project so columns align.
---

# Clean Formatter

Use this skill when a Markdown table in the working tree has ragged columns.

## Steps

1. Find the table the user named.
2. Rewrite the column widths.
3. Show the diff.

```sh
git status --short
git diff -- docs/
```

The formatting rules follow the CommonMark table extension.
