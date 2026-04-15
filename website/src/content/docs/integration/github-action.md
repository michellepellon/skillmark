---
title: GitHub Action
description: Run skillmark in GitHub Actions CI
---

## Basic Usage

```yaml
- uses: michellepellon/skillmark@v1
```

This checks all skills in the repository with default settings.

## With Score Threshold

```yaml
- uses: michellepellon/skillmark@v1
  with:
    min-score: '80'
```

## SARIF Output (Code Annotations)

Upload SARIF output to get inline code annotations on pull requests:

```yaml
- uses: michellepellon/skillmark@v1
  with:
    min-score: '80'
    format: sarif

- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: skillmark.sarif
```

## PR Comment Summary

Generate a Markdown summary for pull request comments:

```yaml
- uses: michellepellon/skillmark@v1
  with:
    min-score: '80'
    format: markdown
  id: lint

- run: echo "${{ steps.lint.outputs.summary }}" >> $GITHUB_STEP_SUMMARY
```

## Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `paths` | Space-separated paths to check | `.` |
| `min-score` | Minimum composite score (0–100) | none |
| `fail-on` | `errors` \| `warnings` \| `none` | `errors` |
| `format` | `terminal` \| `json` \| `sarif` \| `markdown` | `terminal` |
| `version` | skillmark version to install | `latest` |
| `experimental` | Enable Tier 2 heuristic rules | `false` |

## Outputs

| Output | Description |
|--------|-------------|
| `score` | Composite score (0–100) |
| `grade` | Letter grade (A–F) |
| `errors` | Error count |
| `warnings` | Warning count |
| `valid` | `true` if no errors |
| `summary` | Markdown summary (when `format=markdown`) |
