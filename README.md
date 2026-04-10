# skillplane

CI-native linter, validator, and quality scorer for [Agent Skills](https://agentskills.io) (`SKILL.md`).

- **84 rules** — spec compliance (errors), best practices (warnings), quality scoring (info)
- **Quality scoring** — 0-100 composite score across 6 weighted categories, letter grades A-F
- **4 output formats** — terminal (colored), JSON, SARIF (GitHub code annotations), Markdown (PR comments)
- **Fix mode** — auto-repair 6 common issues with `--dry-run` preview
- **CI-native** — GitHub Action, pre-commit hook, configurable via `.skillplane.toml`

## Quick Start

```bash
# Install
cargo install skillplane

# Check a skill
skillplane check path/to/my-skill

# Check all skills in a repo
skillplane check

# Fix issues
skillplane fix path/to/my-skill --dry-run
skillplane fix path/to/my-skill
```

## Example Output

```
skillplane v0.1.0 — my-skill

  Score: 92/100 (A)

  Spec Compliance    ████████████████████  100%  (40.0/40.0)
  Description        ████████████████████  100%  (20.0/20.0)
  Content Efficiency ████████████████░░░░   80%  (12.0/15.0)
  Composability      ████████████████████  100%  (10.0/10.0)
  Script Quality     ████████████████████  100%  (10.0/10.0)
  Discoverability    ░░░░░░░░░░░░░░░░░░░░    0%   (0.0/5.0)

  0 warnings, 0 errors
```

## GitHub Action

```yaml
- uses: michellepellon/skillplane@v1
  with:
    min-score: '80'
    format: sarif

- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: skillplane.sarif
```

Or for PR comments:

```yaml
- uses: michellepellon/skillplane@v1
  with:
    min-score: '80'
    format: markdown
  id: lint
- run: echo "${{ steps.lint.outputs.summary }}" >> $GITHUB_STEP_SUMMARY
```

## Pre-commit Hook

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/michellepellon/skillplane
    rev: v0.1.0
    hooks:
      - id: skillplane
      - id: skillplane-fix
        stages: [manual]
```

## Configuration

Create `.skillplane.toml` in your repo root:

```toml
fail-on = "errors"
min-score = 80

[rules]
disable = ["W022", "E035"]
experimental = false

[scoring.weights]
spec-compliance = 0.40
description-quality = 0.20
content-efficiency = 0.15
composability-clarity = 0.10
script-quality = 0.10
discoverability = 0.05

[paths]
exclude = ["drafts/", "vendor/"]
```

## CLI Reference

```
skillplane check [PATHS...]    Validate, lint, and score skills
skillplane fix [PATHS...]      Auto-repair fixable issues

Options:
  --format <FORMAT>     terminal | json | sarif | markdown  [default: terminal]
  --min-score <N>       Exit non-zero if score < N
  --fail-on <LEVEL>     errors | warnings | none            [default: errors]
  --no-score            Skip scoring (faster)
  --quiet               Only output diagnostics
  --disable <RULES>     Comma-separated rule IDs to disable
  --experimental        Enable Tier 2 heuristic rules
  --exclude <GLOBS>     Comma-separated paths to exclude
  --config <PATH>       Path to .skillplane.toml
  --fix                 Run fix mode (with check)
  --dry-run             Preview fixes without writing (with fix)
  --color <WHEN>        auto | always | never               [default: auto]
```

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All checks pass |
| `1` | Errors or warnings found (per `--fail-on`) |
| `2` | Score below `--min-score` threshold |
| `3` | No SKILL.md files found |

## Scoring Categories

| Category | Weight | What it measures |
|----------|--------|-----------------|
| Spec Compliance | 40% | AgentSkills.io spec conformance (35 rules) |
| Description Quality | 20% | Trigger language, action verbs, keyword diversity |
| Content Efficiency | 15% | Line count, token budget, progressive disclosure |
| Composability & Clarity | 10% | Structure, no filler, no placeholders |
| Script Quality | 10% | Error handling, no hardcoded paths, help docs |
| Discoverability | 5% | License, references, gotchas, validation steps |

## Rule Reference

### Errors (E001-E035) — Spec Compliance

These fail CI by default. See the [AgentSkills.io specification](https://agentskills.io/specification) for the authoritative reference.

| Rule | What it checks |
|------|---------------|
| E001-E005 | SKILL.md existence, frontmatter validity |
| E006-E013 | Name format (length, chars, case, directory match, NFKC) |
| E014-E016 | Description (required, non-empty, length limit) |
| E017-E029 | Optional field types and constraints |
| E030 | Unknown frontmatter fields |
| E031 | Broken file references |
| E032-E034 | Structural issues (unclosed frontmatter, BOM) |
| E035 | Secret/credential detection |

### Warnings (W001-W028) — Best Practices

Advisory by default. Based on [AgentSkills.io best practices](https://agentskills.io/skill-creation/best-practices) and [SkillsBench](https://github.com/benchflow-ai/skillsbench) findings.

### Info (I001-I016) — Quality Scoring Inputs

Not shown in terminal output. Affect the quality score only.

## Acknowledgments

Built on the [AgentSkills.io](https://agentskills.io) open specification. Informed by:
- [agent-skills-lint](https://github.com/greggdonovan/agent-skills-lint) (Rust)
- [agent-skill-validator](https://github.com/ollieb89/agent-skill-validator) (TypeScript)
- [SkillsBench](https://github.com/benchflow-ai/skillsbench) (benchmark findings on skill quality)
- [Official skills-ref library](https://github.com/agentskills/agentskills/tree/main/skills-ref)

## License

MIT
