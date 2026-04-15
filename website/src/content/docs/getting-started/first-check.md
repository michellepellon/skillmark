---
title: Your First Check
description: Run skillmark on a skill for the first time
---

## Check a single skill

Point skillmark at a directory containing a `SKILL.md` file:

```bash
skillmark check path/to/my-skill
```

## Check all skills in a repository

Run from the repo root to discover and check every skill:

```bash
skillmark check
```

skillmark recursively searches for directories containing `SKILL.md` files.

## Check specific paths

Pass multiple paths to check a subset:

```bash
skillmark check skills/auth skills/deploy
```
