---
title: Understanding Output
description: How to read skillmark's terminal output
---

## The Score Card

After checking a skill, skillmark displays a score card:

```
skillmark v0.1.0 — my-skill

  Score: 92/100 (A)

  Spec Compliance    ████████████████████  100%  (40.0/40.0)
  Description        ████████████████████  100%  (20.0/20.0)
  Content Efficiency ████████████████░░░░   80%  (12.0/15.0)
  Composability      ████████████████████  100%  (10.0/10.0)
  Script Quality     ████████████████████  100%  (10.0/10.0)
  Discoverability    ░░░░░░░░░░░░░░░░░░░░    0%   (0.0/5.0)

  0 warnings, 0 errors
```

## Reading the categories

Each bar shows a category's score as a percentage of its maximum weighted contribution:

- **Percentage** — how well the skill scores in that category (0–100%)
- **Points** — the weighted score out of the category maximum (e.g., `12.0/15.0`)

The composite score is the sum of all weighted scores.

## Diagnostics

Errors and warnings appear above the score card:

```
  E014  SKILL.md:1  description is required
  W003  SKILL.md:5  description uses passive voice — prefer action verbs
```

Each diagnostic shows:

| Field | Meaning |
|-------|---------|
| Rule ID | e.g., `E014` — see [Errors](/skillmark/rules/errors/) or [Warnings](/skillmark/rules/warnings/) |
| Location | File and line number |
| Message | What's wrong and how to fix it |

## Letter grades

| Grade | Score Range |
|-------|------------|
| A | 90–100 |
| B | 80–89 |
| C | 70–79 |
| D | 60–69 |
| F | 0–59 |
