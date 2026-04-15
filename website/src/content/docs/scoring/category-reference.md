---
title: Category Reference
description: What each scoring category measures and how to improve
---

## Spec Compliance (40%)

**What it measures:** Conformance to the [AgentSkills.io specification](https://agentskills.io/specification).

**Rules:** [E001–E035](/skillmark/rules/errors/)

**How to improve:**
- Ensure SKILL.md exists with valid YAML frontmatter
- Include required `name` and `description` fields
- Match the `name` field to the directory name
- Remove unknown frontmatter fields
- Fix all broken file references

## Description Quality (20%)

**What it measures:** How well the description communicates when and why to use the skill.

**Rules:** W001–W005, I001–I004

**How to improve:**
- Start descriptions with an action verb ("Lint", "Generate", "Deploy")
- Include trigger language ("Use when...", "Activate if...")
- Avoid passive voice
- Use specific, diverse keywords

## Content Efficiency (15%)

**What it measures:** Whether the body is concise and well-structured without wasting token budget.

**Rules:** W006–W010, I005–I008

**How to improve:**
- Keep body under 500 lines and 8,000 tokens
- Use headings to organize content (progressive disclosure)
- Remove excessive blank lines
- Include code examples

## Composability & Clarity (10%)

**What it measures:** Whether the skill reads clearly and can be composed with other skills.

**Rules:** W011–W015, I009–I012

**How to improve:**
- Remove filler phrases ("In order to", "It should be noted")
- Remove placeholder text (TODO, TBD, FIXME)
- Use consistent heading hierarchy
- Write specific, actionable instructions

## Script Quality (10%)

**What it measures:** Whether included scripts follow shell scripting best practices.

**Rules:** W016–W020, I013, I016

**How to improve:**
- Add `set -e` to shell scripts
- Avoid hardcoded absolute paths
- Include usage/help documentation in scripts
- Add shebang lines

## Discoverability (5%)

**What it measures:** Whether the skill is easy to find and evaluate.

**Rules:** W021–W025, I014–I015

**How to improve:**
- Add a `license` field
- Include a `references/` directory with supporting docs
- Document known limitations or gotchas
- Include validation/testing steps
- Provide example usage
