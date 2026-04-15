---
title: Pre-commit Hook
description: Run skillmark as a pre-commit hook
---

## Setup

Add to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/michellepellon/skillmark
    rev: v0.1.0
    hooks:
      - id: skillmark
      - id: skillmark-fix
        stages: [manual]
```

## Hooks

### `skillmark`

Runs `skillmark check` on changed skill directories. Fails the commit if errors are found.

### `skillmark-fix`

Runs `skillmark fix` on changed skill directories. Registered as a manual stage — run it explicitly:

```bash
pre-commit run skillmark-fix --all-files
```

## Configuration

The hooks respect `.skillmark.toml` in your repo root. You can also pass additional arguments:

```yaml
hooks:
  - id: skillmark
    args: ['--fail-on', 'warnings', '--min-score', '80']
```
