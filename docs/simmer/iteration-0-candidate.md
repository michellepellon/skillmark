# Skillplane Design Spec

CI-native linter, validator, and quality scorer for Agent Skills (SKILL.md) — adhering to the AgentSkills.io specification and best practices.

## Decisions

- **Distribution:** CLI binary + GitHub Action + pre-commit hook
- **Language:** Rust
- **Architecture:** Workspace with library crate (`skillplane-core`) + thin CLI binary (`skillplane`)
- **Scope:** AgentSkills.io spec only (no multi-ecosystem)
- **Validation:** Spec compliance (errors) + best practices (warnings) + quality scoring (0-100 composite)
- **Output formats:** Terminal (colored), JSON, SARIF, Markdown
- **Fix mode:** Auto-repair fixable issues, with `--dry-run` preview
- **Rule tiers:** Tier 1 (v0.1) ships deterministic rules; Tier 2 (v0.2) adds heuristic/experimental rules

## Architecture

### Workspace Layout

```
skillplane/
├── Cargo.toml                  # Workspace root
├── .skillplane.toml            # Default config (dogfooding)
├── crates/
│   ├── skillplane-core/        # Library crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs       # YAML frontmatter + markdown body parsing
│   │       ├── model.rs        # Skill data model
│   │       ├── validator.rs    # Spec-compliance checks (errors)
│   │       ├── linter.rs       # Best-practice checks (warnings)
│   │       ├── scorer.rs       # Quality scoring engine
│   │       ├── fixer.rs        # Auto-repair logic
│   │       ├── discovery.rs    # Find SKILL.md files in a directory tree
│   │       ├── config.rs       # .skillplane.toml loading + rule configuration
│   │       └── output/
│   │           ├── mod.rs
│   │           ├── terminal.rs # Colored human-readable output
│   │           ├── json.rs     # JSON output
│   │           ├── sarif.rs    # SARIF for GitHub code annotations
│   │           └── markdown.rs # Markdown for job summaries / PR comments
│   └── skillplane/             # CLI binary
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
├── action.yml                  # GitHub Action definition
├── .pre-commit-hooks.yaml      # Pre-commit hook config
└── tests/                      # Integration tests with fixture skills
    └── fixtures/
        ├── valid-skill/
        ├── missing-name/
        ├── bad-description/
        ├── unicode-name/
        └── ...
```

### Data Flow

```
Input (paths) → Discovery → Parser → Model → Validator/Linter/Scorer → Diagnostics → Formatter → Output
                                                      ↓
                                                Fixer (mutates files) → Re-validate → Report
```

Discovery finds SKILL.md files (via `git ls-files` with `walkdir` fallback), the parser produces typed models, checks produce diagnostics with severity and category, then formatters render output. Fix mode branches after diagnostics to apply repairs and re-validates.

## Parsing & Data Model

### Parser (`parser.rs`)

- Splits SKILL.md on YAML frontmatter delimiters (`---`)
- Parses frontmatter with `serde_yaml` into a typed struct
- Preserves raw markdown body as string, plus line/column spans for diagnostics
- Handles malformed files gracefully — missing delimiters, empty frontmatter, no body — producing parse-level diagnostics rather than panicking
- Strips UTF-8 BOM before parsing
- Handles case-insensitive filesystem detection (macOS/Windows) when checking SKILL.md casing

### Model (`model.rs`)

```rust
struct Skill {
    path: PathBuf,              // Path to the skill directory
    frontmatter: Frontmatter,   // Parsed YAML
    body: Body,                 // Markdown content + metadata
    file_tree: FileTree,        // Directory contents
}

struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    license: Option<String>,
    compatibility: Option<String>,
    metadata: Option<BTreeMap<String, String>>,
    allowed_tools: Option<String>,
    unknown_fields: BTreeMap<String, serde_yaml::Value>,
}

struct Body {
    raw: String,
    line_count: usize,
    estimated_tokens: usize,    // word_count / 0.75 heuristic
    file_references: Vec<FileReference>,
    headings: Vec<Heading>,     // Extracted H1-H6 for structure analysis
    code_blocks: Vec<CodeBlock>,// Fenced code blocks with language tags
    has_placeholder_text: bool, // Contains TODO/FIXME/TBD
}

struct FileTree {
    has_scripts: bool,
    has_references: bool,
    has_assets: bool,
    files: Vec<PathBuf>,
    total_content_size: usize,  // For progressive disclosure ratio
}
```

Design decisions:
- All frontmatter fields are `Option` — parse first, validate later, producing multiple diagnostics per file.
- Unknown fields captured in `unknown_fields` rather than dropped, so the linter can warn.
- Token estimation uses `split_whitespace().count() / 0.75` — documented as approximate, based on the industry rule of thumb that one token is roughly 0.75 words.
- File references extracted from markdown body by matching relative paths in links and bare paths.

## Diagnostics Model

```rust
struct Diagnostic {
    rule_id: String,           // "E001", "W003", etc.
    severity: Severity,        // Error, Warning, Info
    message: String,           // Human-readable explanation
    path: PathBuf,             // File that triggered it
    span: Option<Span>,        // Line/column for SARIF annotations (None for structural rules)
    fix_available: bool,       // Whether fix mode can repair this
    category: Category,        // For scoring
}

enum Severity { Error, Warning, Info }
enum Category { SpecCompliance, DescriptionQuality, ContentEfficiency, Composability, ScriptQuality, Discoverability }
```

For structural rules (E001 — SKILL.md missing, E033 — directory doesn't exist), `span` is `None`. SARIF output uses the directory path with no region in these cases.

## Validation Rules

### Tier 1 — v0.1 (Deterministic)

#### Spec Compliance Errors

| Rule | Description | Fixable |
|------|-------------|---------|
| `E001` | `SKILL.md` file missing | No |
| `E002` | `SKILL.md` has wrong case (e.g., `skill.md`) | Yes — rename |
| `E003` | YAML frontmatter missing or unparseable | Partial — inject skeleton if missing entirely |
| `E004` | Frontmatter is not a YAML mapping (e.g., a list) | No |
| `E005` | Frontmatter contains non-string keys | No |
| `E006` | `name` field missing | No |
| `E007` | `name` is empty | No |
| `E008` | `name` exceeds 64 characters | No |
| `E009` | `name` contains invalid characters (must be unicode lowercase alphanumeric + hyphens) | No |
| `E010` | `name` starts or ends with hyphen | No |
| `E011` | `name` contains consecutive hyphens (`--`) | No |
| `E012` | `name` is not lowercase (NFKC normalized) | Yes — lowercase |
| `E013` | `name` doesn't match parent directory (NFKC normalized) | Yes — set to dir name |
| `E014` | `description` field missing | No |
| `E015` | `description` is empty | No |
| `E016` | `description` exceeds 1024 characters | No |
| `E017` | `compatibility` exceeds 500 characters | No |
| `E018` | `compatibility` is not a string type | No |
| `E019` | `compatibility` is empty string (if provided) | No |
| `E020` | `license` is not a string type | No |
| `E021` | `license` is empty string (if provided) | No |
| `E022` | `metadata` is not a YAML mapping | No |
| `E023` | `metadata` has non-string keys | No |
| `E024` | `metadata` has non-string values | No |
| `E025` | `allowed-tools` is not a string or array | No |
| `E026` | `allowed-tools` is empty (if provided) | No |
| `E027` | `allowed-tools` uses comma delimiters instead of spaces | No |
| `E028` | `allowed-tools` has unbalanced parentheses in tool spec | No |
| `E029` | `allowed-tools` array contains non-string items | No |
| `E030` | Unknown/unexpected fields in frontmatter | No |
| `E031` | File reference points to nonexistent file | No |
| `E032` | Frontmatter not properly closed (missing second `---`) | Yes — append `---` |
| `E033` | Skill directory does not exist or is not a directory | No |
| `E034` | UTF-8 BOM present | Yes — strip |
| `E035` | Secret/credential pattern detected in skill files | No |

Note on E009: name validation uses unicode-aware `char.is_alphanumeric()` (not ASCII-only `[a-z0-9]`), matching the official skills-ref library which allows Chinese, Russian, and other unicode lowercase names.

Note on E035: secret detection uses regex patterns for AWS keys, GitHub tokens, private key headers, and generic API key patterns. Scans all text files in the skill directory.

#### Best Practice Warnings

| Rule | Description |
|------|-------------|
| `W001` | `SKILL.md` exceeds 500 lines |
| `W002` | `SKILL.md` body exceeds ~5000 estimated tokens |
| `W003` | Description is very short (< 50 chars) |
| `W004` | Description doesn't include trigger language ("Use when...", "Use this skill when...") |
| `W005` | Description uses passive voice instead of imperative ("This skill does..." vs "Use when...") |
| `W006` | Body contains placeholder text (TODO/FIXME/TBD/CHANGEME) |
| `W007` | Name contains placeholder text |
| `W008` | Description contains placeholder text |
| `W009` | No markdown headings in body (unstructured content) |
| `W010` | Scripts in `scripts/` lack shebang lines |
| `W011` | `scripts/` referenced in body but directory doesn't exist |
| `W012` | `references/` referenced in body but directory doesn't exist |
| `W013` | File references are deeply nested (> 1 level from SKILL.md) |
| `W014` | No code examples in body (for skills that reference scripts) |
| `W015` | `.env` file present in skill directory |
| `W016` | No LICENSE file present |
| `W017` | Large binary files (> 1MB) in skill directory |
| `W018` | Scripts contain interactive prompt patterns (`input()`, `readline()`, `read -p`) |
| `W019` | Deprecated `{baseDir}` placeholder in content |
| `W020` | Description matches known vague anti-patterns ("Helps with X", "A tool for X") |
| `W021` | Body contains generic explanations the agent already knows (detected via common filler phrases) |
| `W022` | Multiple SKILL.md files in nested subdirectories (nested skills) |
| `W023` | Description lacks action verbs |
| `W024` | Reference files exist but are never mentioned in SKILL.md body (orphaned) |
| `W025` | SKILL.md references files but doesn't describe when to load them (missing conditional loading) |
| `W026` | Bash scripts lack `set -e`, Python scripts lack error handling patterns |
| `W027` | Scripts contain hardcoded absolute paths |
| `W028` | Scripts have no `--help` or usage documentation |

#### Quality/Info (Score Inputs)

| Rule | Description |
|------|-------------|
| `I001` | No `scripts/` directory |
| `I002` | No `references/` directory |
| `I003` | No `assets/` directory |
| `I004` | No `examples/` directory |
| `I005` | Body has no gotchas/pitfalls section |
| `I006` | Body has no validation/verification step |
| `I007` | No progressive disclosure — all content in SKILL.md, no referenced files |
| `I008` | `compatibility` field not specified |
| `I009` | Scripts don't use structured output patterns (JSON/CSV) |
| `I010` | Scripts lack meaningful exit code documentation |
| `I011` | Scripts lack idempotency patterns |
| `I012` | No `license` field or LICENSE file |
| `I015` | Description keyword diversity score — number of distinct trigger scenarios covered |
| `I016` | Progressive disclosure ratio: % of total skill content in SKILL.md vs. referenced files |

### Tier 2 — v0.2 (Heuristic/Experimental, off by default)

These rules require heuristic analysis and may produce false positives. Enable with `--experimental` or per-rule in config.

| Rule | Description |
|------|-------------|
| `W029` | Skill body covers multiple unrelated domains (H2 topic cluster analysis) |
| `W030` | Inconsistent library/API naming within body |
| `W031` | Contradictory instructions detected |
| `W032` | Code examples in body have syntax errors (bracket/quote matching) |
| `I013` | Description contains "and" joining unrelated capabilities (composability signal) |
| `I014` | Description doesn't cover implicit trigger scenarios |
| `E036` | Script files fail syntax validation (opt-in, requires runtime: `python`, `bash -n`, `node --check`). Skipped gracefully if runtime unavailable. |

**Tier 1 total: 35 errors + 28 warnings + 14 info = 77 rules**
**Tier 2 total: 1 error + 4 warnings + 2 info = 7 rules**
**Grand total: 84 rules**

## Fix Mode

Fix mode repairs what it can, reports what it can't, and supports `--dry-run`.

### Fixable Rules

| Rule | Fix Action |
|------|------------|
| `E002` | Rename `skill.md` → `SKILL.md` |
| `E003` | Inject empty frontmatter skeleton (`---\nname:\ndescription:\n---`) if missing entirely |
| `E012` | Normalize name to lowercase |
| `E013` | Set `name` to match parent directory (NFKC normalized) |
| `E032` | Append closing `---` |
| `E034` | Strip UTF-8 BOM |

### Fix Pipeline

```
Parse → Diagnose → Filter fixable → Apply fixes → Re-validate → Report remaining
```

Behaviors:
- Always re-validates after applying fixes
- `--dry-run` prints the diff without writing files
- Frontmatter output uses deterministic field ordering: `name`, `description`, `license`, `compatibility`, `allowed-tools`, `metadata`
- String values always quoted in output to avoid YAML type coercion (`"true"`, `"1.0"`)
- Unknown fields are preserved in output (not moved to metadata — that would be destructive if the user has a typo in a known field name)

## Quality Scoring Engine

### Composite Score: 0-100

Built from weighted category scores. Weights are configurable in `.skillplane.toml` with these defaults:

| Category | Default Weight | Inputs |
|----------|---------------|--------|
| **Spec Compliance** | 40% | All E-rules. Binary per rule — any error = 0 for that rule. Score = % passing. |
| **Description Quality** | 20% | Length (W003), trigger language (W004/W005), imperative voice, anti-patterns (W020), action verbs (W023), keyword diversity (I015) |
| **Content Efficiency** | 15% | Line count (W001), token estimate (W002), progressive disclosure ratio (I016), orphaned references (W024), conditional loading (W025) |
| **Composability & Clarity** | 10% | No contradictions (Tier 2 W031 when enabled), consistent naming, no generic filler (W021) |
| **Script Quality** | 10% | Error handling (W026), no hardcoded paths (W027), help docs (W028), no interactive prompts (W018), structured output (I009). **If no scripts exist, this category scores 100%** (not penalized). |
| **Discoverability** | 5% | License present (I012), references dir (I002), gotchas section (I005), validation loops (I006) |

### Score Model

```rust
struct ScoreCard {
    composite: f64,                 // 0.0-100.0
    categories: Vec<CategoryScore>,
    grade: Grade,                   // A/B/C/D/F
}

struct CategoryScore {
    name: String,
    weight: f64,
    score: f64,                     // 0.0-100.0
    weighted_score: f64,            // score * weight
    rule_results: Vec<RuleResult>,
}

enum Grade { A, B, C, D, F }
```

Default grade boundaries (configurable in `.skillplane.toml`):
- A: 90-100
- B: 80-89
- C: 70-79
- D: 60-69
- F: < 60

### Multi-Skill Repos

When scanning a repo with multiple skills:
- Each skill gets its own scorecard
- A summary line shows aggregate stats (total errors, warnings, average score)
- `--min-score` applies **per-skill** — one bad skill fails the entire run

## Configuration

### `.skillplane.toml`

Project-level config file, discovered by walking up from the skill path to the repo root.

```toml
# Defaults
fail-on = "errors"          # errors | warnings | none
min-score = 0               # 0-100, 0 = disabled
format = "terminal"         # terminal | json | sarif | markdown

# Rule overrides
[rules]
disable = ["W022", "E035"]  # Disable specific rules
experimental = false         # Enable Tier 2 rules

# Scoring weights (must sum to 1.0)
[scoring.weights]
spec-compliance = 0.40
description-quality = 0.20
content-efficiency = 0.15
composability = 0.10
script-quality = 0.10
discoverability = 0.05

# Grade boundaries
[scoring.grades]
A = 90
B = 80
C = 70
D = 60

# Path exclusions
[paths]
exclude = ["drafts/", "vendor/", "node_modules/"]
```

### Inline Suppression

Within SKILL.md, suppress specific rules:

```markdown
<!-- skillplane-disable W003 -->
```

Suppresses W003 for the entire file. Placed anywhere in the markdown body or as an HTML comment in the YAML frontmatter region.

### CLI Flags Override Config

CLI flags take precedence over `.skillplane.toml`. Config file values take precedence over built-in defaults.

## CLI Interface

### Commands

```
skillplane check [paths...]     # Validate + lint + score
skillplane fix [paths...]       # Auto-repair fixable issues
```

If no paths given, discovers all SKILL.md files from repo root (via `git ls-files`, falling back to `walkdir`).

### Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--format <terminal\|json\|sarif\|markdown>` | Output format | `terminal` |
| `--min-score <N>` | Exit non-zero if composite score < N | (none) |
| `--min-score <category>:<N>` | Exit non-zero if category score < N | (none) |
| `--fail-on <errors\|warnings\|none>` | When to exit non-zero | `errors` |
| `--fix` | Alias for `skillplane fix` (usable with `check`) | |
| `--dry-run` | Preview fixes without writing (only with `fix`) | |
| `--no-score` | Skip scoring (faster, validate only) | |
| `--quiet` | Only output errors/warnings, no score or decoration | |
| `--color <auto\|always\|never>` | Color output control | `auto` |
| `--disable <rule,...>` | Disable specific rules for this run | |
| `--experimental` | Enable Tier 2 heuristic rules | `false` |
| `--exclude <glob,...>` | Exclude paths from scanning | |
| `--config <path>` | Path to config file | auto-discover |

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All checks pass, score above threshold |
| `1` | Validation errors or warnings (per `--fail-on`) found |
| `2` | Score below `--min-score` threshold (but no errors) |
| `3` | No SKILL.md files found |

Precedence: `1` > `2` > `3`. If both errors exist and score is below threshold, exit code is `1`.

### Terminal Output

```
skillplane v0.1.0 — my-skill

  Score: 87/100 (B)

  Spec Compliance    ████████████████████  100%  (40.0/40)
  Description        ████████████████░░░░   80%  (16.0/20)
  Content Efficiency ██████████████░░░░░░   73%  (11.0/15)
  Composability      ████████████████████  100%  (10.0/10)
  Script Quality     ████████████████████  100%  (10.0/10)
  Discoverability    ██████████████████░░   90%   (4.5/5)

  3 warnings, 0 errors

  W003  SKILL.md:2  Description is very short (32 chars, recommend >= 50)
  W004  SKILL.md:2  Description lacks trigger language ("Use when...")
  W029  —           references/api.md exists but is not referenced in SKILL.md
```

### Usage Examples

```bash
# Local dev — check everything, see score
skillplane check

# CI — fail on errors, require score >= 80
skillplane check --min-score 80

# Pre-commit — just errors, fast
skillplane check --no-score --quiet

# PR comment — markdown output
skillplane check --format markdown

# SARIF for GitHub code annotations
skillplane check --format sarif > skillplane.sarif

# Fix what you can, preview first
skillplane fix --dry-run
skillplane fix

# Disable noisy rules
skillplane check --disable W022,E035

# Enable experimental heuristic rules
skillplane check --experimental
```

## GitHub Action

### Distribution

Pre-compiled binaries for `linux-x86_64`, `linux-aarch64`, `darwin-x86_64`, `darwin-aarch64`, `windows-x86_64` attached to GitHub Releases. The action downloads the correct binary for the runner OS/arch, caches with `actions/cache` keyed on version + platform. Falls back to `cargo install` if no binary available.

### `action.yml`

```yaml
name: 'Skillplane'
description: 'Lint, validate, and score Agent Skills (SKILL.md)'
author: 'skillplane'

branding:
  icon: 'check-square'
  color: 'blue'

inputs:
  paths:
    description: 'Space-separated paths to check'
    required: false
    default: '.'
  min-score:
    description: 'Minimum composite score (0-100)'
    required: false
  fail-on:
    description: 'errors | warnings | none'
    required: false
    default: 'errors'
  format:
    description: 'terminal | json | sarif | markdown'
    required: false
    default: 'terminal'
  version:
    description: 'Skillplane version'
    required: false
    default: 'latest'
  experimental:
    description: 'Enable Tier 2 heuristic rules'
    required: false
    default: 'false'

outputs:
  score:
    description: 'Composite score (0-100)'
  grade:
    description: 'Letter grade (A-F)'
  errors:
    description: 'Error count'
  warnings:
    description: 'Warning count'
  valid:
    description: 'true if no errors'
  summary:
    description: 'Markdown summary (when format=markdown)'
  sarif:
    description: 'Path to SARIF file (when format=sarif)'

runs:
  using: 'composite'
  steps:
    - name: Download skillplane
      # Downloads pre-compiled binary for runner OS/arch
    - name: Run skillplane
      # Executes with provided inputs
```

### Workflow Examples

```yaml
# Inline PR annotations via SARIF
- uses: your-org/skillplane@v1
  with:
    format: sarif
    min-score: '80'
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: skillplane.sarif

# Job summary via Markdown
- uses: your-org/skillplane@v1
  with:
    format: markdown
    min-score: '80'
  id: lint
- run: echo "${{ steps.lint.outputs.summary }}" >> $GITHUB_STEP_SUMMARY
```

## Pre-commit Hook

### `.pre-commit-hooks.yaml`

```yaml
- id: skillplane
  name: skillplane check
  entry: skillplane check --no-score --quiet --fail-on errors
  language: system
  files: '(?i)skill\.md$'
  pass_filenames: false

- id: skillplane-fix
  name: skillplane fix
  entry: skillplane fix
  language: system
  files: '(?i)skill\.md$'
  pass_filenames: false
  stages: [manual]
```

The check hook runs on every commit touching SKILL.md files — fast, no scoring, errors only. The fix hook is manual (`pre-commit run --hook-stage manual skillplane-fix`).

Usage in a project:

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/your-org/skillplane
    rev: v0.1.0
    hooks:
      - id: skillplane
      - id: skillplane-fix
        stages: [manual]
```

Requires `skillplane` binary on `PATH` (via `cargo install skillplane`).

## Rust Dependencies

### Runtime

| Crate | Purpose |
|-------|---------|
| `serde`, `serde_yaml` | YAML frontmatter parsing |
| `serde_json` | JSON output + SARIF generation |
| `clap` | CLI argument parsing |
| `unicode-normalization` | NFKC normalization for name matching |
| `walkdir` | Directory traversal for discovery |
| `colored` | Terminal output with colors |
| `thiserror` | Ergonomic error types |
| `regex` | Pattern matching for description analysis, secret detection |
| `toml` | Config file parsing |

### Test

| Crate | Purpose |
|-------|---------|
| `tempfile` | Temporary directories for integration tests |
| `insta` | Snapshot testing for output formats |
| `assert_cmd` | CLI integration tests |
| `predicates` | Assertion helpers for CLI output |

No heavy dependencies. No tokenizer library (whitespace heuristic). No markdown AST parser (raw line processing for headings, code fences, references). Keeps the binary small and compile times fast.

## References

- [AgentSkills.io Specification](https://agentskills.io/specification)
- [AgentSkills.io Best Practices](https://agentskills.io/skill-creation/best-practices)
- [AgentSkills.io Optimizing Descriptions](https://agentskills.io/skill-creation/optimizing-descriptions)
- [AgentSkills.io Using Scripts](https://agentskills.io/skill-creation/using-scripts)
- [Official skills-ref library](https://github.com/agentskills/agentskills/tree/main/skills-ref)
- [agent-skills-lint (Rust)](https://github.com/greggdonovan/agent-skills-lint)
- [agent-skill-validator (TypeScript)](https://github.com/ollieb89/agent-skill-validator)
- [SkillsBench](https://github.com/benchflow-ai/skillsbench)
- [Anthropic example skills](https://github.com/anthropics/skills)
