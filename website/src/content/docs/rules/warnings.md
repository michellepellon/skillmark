---
title: Warnings (W001–W028)
description: Best practice rules that are advisory by default
---

These rules are based on [AgentSkills.io best practices](https://agentskills.io/skill-creation/best-practices) and [SkillsBench](https://github.com/benchflow-ai/skillsbench) findings. They produce warnings and do not fail CI unless you set `--fail-on warnings`.

## Description Quality

| Rule | What it checks |
|------|---------------|
| W001 | Description starts with an action verb |
| W002 | Description contains trigger language ("Use when...", "Activate if...") |
| W003 | Description avoids passive voice |
| W004 | Description uses diverse keywords (not repeating the name) |
| W005 | Description is specific (no vague terms like "various", "things", "stuff") |

## Content Efficiency

| Rule | What it checks |
|------|---------------|
| W006 | Body is ≤ 500 lines |
| W007 | Estimated token count is ≤ 8,000 |
| W008 | Uses progressive disclosure (headings to organize content) |
| W009 | No excessive blank lines (≤ 2 consecutive) |
| W010 | No overly long lines (≤ 200 characters) |

## Composability & Clarity

| Rule | What it checks |
|------|---------------|
| W011 | No filler phrases ("In order to", "It should be noted") |
| W012 | No placeholder text ("TODO", "TBD", "FIXME", "lorem ipsum") |
| W013 | No first-person pronouns ("I", "my", "we") |
| W014 | Headings follow a logical hierarchy (no skipped levels) |
| W015 | No duplicate headings |

## Script Quality

| Rule | What it checks |
|------|---------------|
| W016 | Scripts include error handling (`set -e` or equivalent) |
| W017 | No hardcoded absolute paths in scripts |
| W018 | Scripts include usage/help documentation |
| W019 | No shell scripts without a shebang line |
| W020 | No scripts with overly broad permissions |

## Discoverability

| Rule | What it checks |
|------|---------------|
| W021 | License field is present |
| W022 | References directory exists with supporting docs |
| W023 | Body documents known limitations or gotchas |
| W024 | Body includes validation or testing steps |
| W025 | Example usage is provided |

## Experimental (Tier 2)

These require `--experimental` to enable:

| Rule | What it checks |
|------|---------------|
| W026 | Description scores well on readability metrics |
| W027 | Body avoids jargon without explanation |
| W028 | Code blocks specify a language |
