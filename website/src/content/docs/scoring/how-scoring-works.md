---
title: How Scoring Works
description: Understanding skillmark's quality scoring system
---

## Composite Score

skillmark calculates a composite score from 0 to 100 by evaluating your skill across 6 weighted categories. Each category contributes a percentage of the total score.

## Default Weights

| Category | Weight | Max Points |
|----------|--------|------------|
| Spec Compliance | 40% | 40 |
| Description Quality | 20% | 20 |
| Content Efficiency | 15% | 15 |
| Composability & Clarity | 10% | 10 |
| Script Quality | 10% | 10 |
| Discoverability | 5% | 5 |

## How Categories are Scored

Each category contains a set of rules (see [Info rules](/skillmark/rules/info/)). Each rule evaluates a specific aspect and produces a pass/fail result. The category score is the percentage of rules that pass, multiplied by the category weight.

**Example:** If Spec Compliance has 35 rules and 33 pass, the category score is `(33/35) × 40 = 37.7`.

## Letter Grades

| Grade | Score Range |
|-------|------------|
| A | 90–100 |
| B | 80–89 |
| C | 70–79 |
| D | 60–69 |
| F | 0–59 |

## Custom Weights

Override the default weights in `.skillmark.toml`:

```toml
[scoring.weights]
spec-compliance = 0.40
description-quality = 0.20
content-efficiency = 0.15
composability-clarity = 0.10
script-quality = 0.10
discoverability = 0.05
```

Weights must sum to 1.0.

## Skipping Scoring

Use `--no-score` to skip scoring entirely (faster for CI when you only need pass/fail):

```bash
skillmark check --no-score
```
