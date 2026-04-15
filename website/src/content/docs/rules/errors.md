---
title: Errors (E001–E035)
description: Spec compliance rules that fail CI by default
---

These rules enforce the [AgentSkills.io specification](https://agentskills.io/specification). They produce errors and will cause `skillmark check` to exit non-zero by default.

## SKILL.md Existence & Structure

| Rule | What it checks | Fixable |
|------|---------------|---------|
| E001 | SKILL.md file exists in the directory | No |
| E002 | SKILL.md is valid UTF-8 | No |
| E003 | SKILL.md contains YAML frontmatter delimiters (`---`) | Yes |
| E004 | Frontmatter is valid YAML | No |
| E005 | Frontmatter parses as a YAML mapping | No |

## Name Validation

| Rule | What it checks | Fixable |
|------|---------------|---------|
| E006 | `name` field is present | No |
| E007 | `name` is a string | No |
| E008 | `name` is non-empty | No |
| E009 | `name` is ≤ 50 characters | No |
| E010 | `name` contains only lowercase letters, digits, and hyphens | Yes |
| E011 | `name` does not start or end with a hyphen | Yes |
| E012 | `name` matches the directory name | No |
| E013 | `name` is NFKC-normalized | Yes |

## Description Validation

| Rule | What it checks | Fixable |
|------|---------------|---------|
| E014 | `description` field is present | No |
| E015 | `description` is non-empty | No |
| E016 | `description` is ≤ 200 characters | No |

## Optional Field Types

| Rule | What it checks | Fixable |
|------|---------------|---------|
| E017 | `license` is a string (if present) | No |
| E018 | `license` is a valid SPDX expression (if present) | No |
| E019 | `compatibility` is a string (if present) | No |
| E020 | `compatibility` matches known format (if present) | No |
| E021 | `metadata` is a mapping (if present) | No |
| E022 | `metadata` values are strings (if present) | No |
| E023 | `metadata` has ≤ 10 keys (if present) | No |
| E024 | `metadata` keys are ≤ 30 characters (if present) | No |
| E025 | `metadata` values are ≤ 100 characters (if present) | No |
| E026 | `allowed-tools` is a sequence (if present) | No |
| E027 | `allowed-tools` entries are strings (if present) | No |
| E028 | `allowed-tools` entries match tool name format (if present) | No |
| E029 | `allowed-tools` has no duplicates (if present) | No |

## Structural Issues

| Rule | What it checks | Fixable |
|------|---------------|---------|
| E030 | No unknown frontmatter fields | No |
| E031 | All file references in the body resolve to existing files | No |
| E032 | Frontmatter delimiters are properly closed | Yes |
| E033 | No UTF-8 BOM at the start of the file | Yes |
| E034 | No trailing whitespace in frontmatter values | Yes |
| E035 | No secrets or credentials detected in content | No |
