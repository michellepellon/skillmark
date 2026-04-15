---
title: CLI Reference
description: Complete command-line reference for skillmark
---

## Commands

### `skillmark check [PATHS...]`

Validate, lint, and score skills. Discovers `SKILL.md` files in the given paths (or current directory if none specified).

```bash
# Check all skills
skillmark check

# Check specific paths
skillmark check path/to/skill-a path/to/skill-b

# Check with options
skillmark check --format json --min-score 80 --fail-on warnings
```

### `skillmark fix [PATHS...]`

Auto-repair fixable issues. See [Fix Mode](/skillmark/guides/fix-mode/) for details.

```bash
# Preview fixes
skillmark fix path/to/skill --dry-run

# Apply fixes
skillmark fix path/to/skill

# Fix + check in one pass
skillmark check --fix path/to/skill
```

## Options

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--format` | `terminal` \| `json` \| `sarif` \| `markdown` | `terminal` | Output format |
| `--min-score` | `0`–`100` | none | Exit non-zero if score below threshold |
| `--fail-on` | `errors` \| `warnings` \| `none` | `errors` | Severity level that causes non-zero exit |
| `--no-score` | flag | off | Skip scoring (faster) |
| `--quiet` | flag | off | Only output diagnostics |
| `--disable` | comma-separated rule IDs | none | Rules to skip |
| `--experimental` | flag | off | Enable Tier 2 heuristic rules |
| `--exclude` | comma-separated globs | none | Paths to skip |
| `--config` | file path | `.skillmark.toml` | Config file location |
| `--fix` | flag | off | Run fix mode (with check) |
| `--dry-run` | flag | off | Preview fixes without writing (with fix) |
| `--color` | `auto` \| `always` \| `never` | `auto` | Color output control |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All checks pass |
| `1` | Errors or warnings found (per `--fail-on`) |
| `2` | Score below `--min-score` threshold |
| `3` | No SKILL.md files found |
