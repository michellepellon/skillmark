---
title: Configuration
description: Configure skillmark with .skillmark.toml
---

## Overview

Create `.skillmark.toml` in your repo root to configure skillmark. All fields are optional — defaults are used when omitted.

## Full Reference

```toml
# Exit behavior
fail-on = "errors"      # "errors" | "warnings" | "none"
min-score = 80           # Exit non-zero if composite score < this value

# Rule configuration
[rules]
disable = ["W022", "E035"]   # Rule IDs to skip
experimental = false          # Enable Tier 2 heuristic rules (W026-W028)

# Scoring weights (must sum to 1.0)
[scoring.weights]
spec-compliance = 0.40
description-quality = 0.20
content-efficiency = 0.15
composability-clarity = 0.10
script-quality = 0.10
discoverability = 0.05

# Path filters
[paths]
exclude = ["drafts/", "vendor/"]  # Glob patterns to skip
```

## Field Reference

### Top-level

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fail-on` | string | `"errors"` | Severity level that causes non-zero exit |
| `min-score` | integer | none | Minimum composite score threshold |

### `[rules]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `disable` | array of strings | `[]` | Rule IDs to skip |
| `experimental` | boolean | `false` | Enable Tier 2 heuristic rules |

### `[scoring.weights]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `spec-compliance` | float | `0.40` | Weight for spec compliance category |
| `description-quality` | float | `0.20` | Weight for description quality |
| `content-efficiency` | float | `0.15` | Weight for content efficiency |
| `composability-clarity` | float | `0.10` | Weight for composability & clarity |
| `script-quality` | float | `0.10` | Weight for script quality |
| `discoverability` | float | `0.05` | Weight for discoverability |

### `[paths]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `exclude` | array of strings | `[]` | Glob patterns for paths to skip |

## CLI Overrides

CLI flags override `.skillmark.toml` values:

```bash
# Override fail-on from config
skillmark check --fail-on warnings

# Disable specific rules on top of config
skillmark check --disable W022,W023

# Override config file location
skillmark check --config path/to/.skillmark.toml
```
