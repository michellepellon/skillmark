---
title: Fix Mode
description: Auto-repair common issues with skillmark fix
---

## Overview

skillmark can auto-repair 6 common issues. Run fix mode with:

```bash
skillmark fix path/to/my-skill
```

## Preview first

Always preview fixes before writing:

```bash
skillmark fix path/to/my-skill --dry-run
```

This shows what would change without modifying any files.

## Fixable Rules

| Rule | What it fixes |
|------|--------------|
| E003 | Adds missing frontmatter delimiters (`---`) |
| E010 | Normalizes name to lowercase-hyphenated format |
| E011 | Strips leading/trailing hyphens from name |
| E013 | Applies NFKC normalization to name |
| E032 | Closes unclosed frontmatter delimiters |
| E033 | Removes UTF-8 BOM |
| E034 | Trims trailing whitespace from frontmatter values |

## Fix + Check

Combine fix and check in one pass:

```bash
skillmark check --fix path/to/my-skill
```

This runs fixes first, then validates the result.

## What fix mode won't do

Fix mode only handles mechanical, unambiguous repairs. It will not:

- Add missing required fields (you need to write the content)
- Rewrite descriptions for quality
- Restructure body content
- Remove secrets (requires human judgment)
