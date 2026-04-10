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
    metadata: Option<serde_yaml::Value>,       // Parsed as raw Value; validator checks it is a mapping with string keys and string values (E022/E023/E024)
    allowed_tools: Option<serde_yaml::Value>,  // Parsed as raw Value; validator checks it is a string or array-of-strings (E025/E029)
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
    has_examples: bool,
    files: Vec<PathBuf>,
    total_content_size: usize,  // For progressive disclosure ratio
}

/// A source location range for diagnostic annotations.
struct Span {
    start_line: usize,          // 1-based line number
    start_col: usize,           // 1-based column (byte offset within line)
    end_line: usize,            // 1-based, inclusive
    end_col: usize,             // 1-based, exclusive (points one past the last character)
}

/// A relative file path referenced from within the SKILL.md body
/// (e.g., `scripts/deploy.sh` in a markdown link or bare path).
struct FileReference {
    path: String,               // The referenced relative path as written
    span: Span,                 // Location of the reference in SKILL.md
    exists: bool,               // Whether the target file exists on disk (populated during parsing)
}

/// A markdown heading extracted from the body for structure analysis.
struct Heading {
    level: u8,                  // 1-6 (H1-H6)
    text: String,               // Heading text content (stripped of `#` prefix and inline formatting)
    span: Span,                 // Location in SKILL.md
}

/// A fenced code block extracted from the body.
struct CodeBlock {
    language: Option<String>,   // Language tag after opening ```, if present (e.g., "bash", "python")
    content: String,            // Raw content between the fences (not including the fence lines)
    span: Span,                 // Location of the opening ``` line in SKILL.md
}
```

Design decisions:
- All frontmatter fields are `Option` — parse first, validate later, producing multiple diagnostics per file.
- Unknown fields captured in `unknown_fields` rather than dropped, so the linter can warn.
- `metadata` and `allowed_tools` are parsed as raw `serde_yaml::Value` so the validator can inspect their structure and produce specific diagnostics (E022-E024 for metadata, E025/E029 for allowed-tools) rather than failing silently at deserialization.
- Token estimation uses `split_whitespace().count() / 0.75` — documented as approximate, based on the industry rule of thumb that one token is roughly 0.75 words.
- File references extracted from markdown body by matching relative paths in links and bare paths.
- `Span` uses 1-based line/column numbers to match SARIF and editor conventions. For structural rules that apply to the entire file or directory (E001, E033), the diagnostic's `span` field is `None`.

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
enum Category { SpecCompliance, DescriptionQuality, ContentEfficiency, ComposabilityClarity, ScriptQuality, Discoverability }
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
| `E002` | Rename `skill.md` -> `SKILL.md` |
| `E003` | Inject empty frontmatter skeleton (`---\nname:\ndescription:\n---`) if missing entirely |
| `E012` | Normalize name to lowercase |
| `E013` | Set `name` to match parent directory (NFKC normalized) |
| `E032` | Append closing `---` |
| `E034` | Strip UTF-8 BOM |

### Fix Pipeline

```
Parse -> Diagnose -> Filter fixable -> Apply fixes -> Re-validate -> Report remaining
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
| **Composability & Clarity** | 10% | Structured headings (W009), no placeholder text (W006), no generic filler (W021); Tier 2 adds contradictions (W031) |
| **Script Quality** | 10% | Error handling (W026), no hardcoded paths (W027), help docs (W028), no interactive prompts (W018), structured output (I009). **If no scripts exist, this category scores 100%** (not penalized). |
| **Discoverability** | 5% | License present (I012), references dir (I002), gotchas section (I005), validation loops (I006) |

### Per-Category Scoring Formulas

Each category score is computed as the percentage of its constituent checks that pass. A check "passes" when its associated rule is NOT triggered (no diagnostic emitted for that rule). Disabled and suppressed rules are excluded from both numerator and denominator.

**General formula (all categories):**

```
category_score = (passing_checks / total_checks) * 100
weighted_contribution = category_score * weight
composite = sum(weighted_contribution for each category)
```

Where:
- `passing_checks` = number of rules in that category that did NOT fire
- `total_checks` = number of applicable (non-disabled, non-suppressed) rules in that category

**Rounding:** Per-category scores are stored as `f64` for calculation. The final composite score is rounded to the nearest integer (`composite.round() as u32`) for display and grade assignment. For example, a raw composite of 79.5 displays as 80 and receives grade B.

#### Spec Compliance (weight: 0.40)

**Checks:** E001 through E035 (35 rules in Tier 1). Each is binary: pass (0) or fail (1).

Formula: `spec_score = (rules_not_triggered / applicable_rule_count) * 100`

**Worked example:** A skill triggers E031 (broken file reference) and E030 (unknown field). All 35 rules are enabled.
- Passing: 33 / 35 = 94.3%
- Weighted contribution: 94.3 * 0.40 = **37.7**

#### Description Quality (weight: 0.20)

**Checks (6 total):**
| Check | Rule | Criterion |
|-------|------|-----------|
| DQ1 | W003 | Description >= 50 characters |
| DQ2 | W004 | Contains trigger language |
| DQ3 | W005 | Uses imperative (not passive) voice |
| DQ4 | W020 | No vague anti-patterns |
| DQ5 | W023 | Contains action verbs |
| DQ6 | I015 | Keyword diversity >= 2 distinct trigger scenarios |

Formula: `desc_score = (checks_passing / 6) * 100`

**Worked example:** A skill has a 38-char description with no trigger language but uses imperative voice, has no anti-patterns, has action verbs, and covers 1 trigger scenario.
- Failing: DQ1 (W003), DQ2 (W004), DQ6 (I015). Passing: DQ3, DQ4, DQ5.
- Score: 3 / 6 = 50.0%
- Weighted contribution: 50.0 * 0.20 = **10.0**

#### Content Efficiency (weight: 0.15)

**Checks (5 total):**
| Check | Rule | Criterion |
|-------|------|-----------|
| CE1 | W001 | SKILL.md <= 500 lines |
| CE2 | W002 | Body <= ~5000 estimated tokens |
| CE3 | I016 | Progressive disclosure ratio > 0% (some content in referenced files) |
| CE4 | W024 | No orphaned reference files |
| CE5 | W025 | Referenced files have conditional loading instructions |

Formula: `efficiency_score = (checks_passing / 5) * 100`

**Worked example:** A skill has 200 lines, 2000 tokens, no referenced files at all (I016 fires), no orphaned references (vacuously passes), and no conditional loading issue (vacuously passes).
- Failing: CE3 (I016). Passing: CE1, CE2, CE4, CE5.
- Score: 4 / 5 = 80.0%
- Weighted contribution: 80.0 * 0.15 = **12.0**

Note on vacuous passing: CE4 and CE5 only fire when there are reference files present. If no reference files exist, those checks pass (the bad condition cannot occur). The penalty for having no referenced files is already captured by CE3/I016.

#### Composability & Clarity (weight: 0.10)

**Checks (3 total in Tier 1; 4 with Tier 2):**
| Check | Rule | Criterion | Tier |
|-------|------|-----------|------|
| CC1 | W021 | No generic filler content | 1 |
| CC2 | W009 | Has markdown headings (structured) | 1 |
| CC3 | W006 | No placeholder text in body | 1 |
| CC4 | W031 | No contradictory instructions | 2 (off by default) |

Formula: `composability_score = (checks_passing / applicable_checks) * 100`

When `--experimental` is off, only CC1-CC3 apply (denominator = 3). When on, CC4 is added (denominator = 4).

**Worked example (Tier 1 only):** A skill has headings and no placeholders, but contains generic filler (W021 fires).
- Failing: CC1 (W021). Passing: CC2, CC3.
- Score: 2 / 3 = 66.7%
- Weighted contribution: 66.7 * 0.10 = **6.7**

#### Script Quality (weight: 0.10)

**Checks (5 total):**
| Check | Rule | Criterion |
|-------|------|-----------|
| SQ1 | W026 | Scripts have error handling |
| SQ2 | W027 | No hardcoded absolute paths |
| SQ3 | W028 | Scripts have help/usage docs |
| SQ4 | W018 | No interactive prompt patterns |
| SQ5 | I009 | Scripts use structured output |

**Special case: if `file_tree.has_scripts == false`, this category scores 100%.** The rationale is that many valid skills have no scripts, and they should not be penalized.

Formula (when scripts exist): `script_score = (checks_passing / 5) * 100`

**Worked example:** A skill has scripts. W026 fires (no `set -e`), W028 fires (no `--help`). Others pass.
- Score: 3 / 5 = 60.0%
- Weighted contribution: 60.0 * 0.10 = **6.0**

#### Discoverability (weight: 0.05)

**Checks (4 total):**
| Check | Rule | Criterion |
|-------|------|-----------|
| DS1 | I012 | License field or LICENSE file present |
| DS2 | I002 | `references/` directory exists |
| DS3 | I005 | Body has gotchas/pitfalls section |
| DS4 | I006 | Body has validation/verification step |

Formula: `discovery_score = (checks_passing / 4) * 100`

**Worked example:** A skill has a LICENSE file and a references dir, but no gotchas section and no validation step.
- Passing: DS1, DS2. Failing: DS3 (I005), DS4 (I006).
- Score: 2 / 4 = 50.0%
- Weighted contribution: 50.0 * 0.05 = **2.5**

### End-to-End Worked Example

Using the per-category results from the examples above:

| Category | Score | Weight | Weighted |
|----------|-------|--------|----------|
| Spec Compliance | 94.3% | 0.40 | 37.7 |
| Description Quality | 50.0% | 0.20 | 10.0 |
| Content Efficiency | 80.0% | 0.15 | 12.0 |
| Composability & Clarity | 66.7% | 0.10 | 6.7 |
| Script Quality | 60.0% | 0.10 | 6.0 |
| Discoverability | 50.0% | 0.05 | 2.5 |
| **Composite** | | | **74.9 -> 75** |

Composite rounds to **75**. Grade: **C** (70-79 range).

**Complete rule trace for this example (2 errors, 8 warnings):**

Errors:
1. E030 — Unknown field in frontmatter
2. E031 — File reference points to nonexistent file

Warnings:
1. W003 — Description is very short (< 50 chars)
2. W004 — Description lacks trigger language
3. W021 — Body contains generic filler content
4. W026 — Scripts lack error handling (`set -e`)
5. W028 — Scripts lack `--help`/usage docs

Info (score inputs, not counted as warnings):
1. I015 — Keyword diversity < 2 trigger scenarios
2. I016 — Progressive disclosure ratio is 0%
3. I005 — No gotchas/pitfalls section
4. I006 — No validation/verification step
5. I002 — No `references/` directory (note: the worked example for Discoverability says DS2 passes — see reconciliation below)

**Reconciliation note:** The per-category worked examples describe a *single hypothetical skill* with these combined properties: has an unknown field "autor", references nonexistent `scripts/deploy.sh`, has a 38-char description with no trigger language but imperative voice with action verbs covering 1 scenario, has 200 lines/2000 tokens, no referenced files, has headings but generic filler and no placeholders, has scripts without `set -e` or `--help`, and has a LICENSE file + references dir but no gotchas or validation section. The exact diagnostics that fire:

| # | Rule | Severity | Category |
|---|------|----------|----------|
| 1 | E030 | Error | Spec Compliance |
| 2 | E031 | Error | Spec Compliance |
| 3 | W003 | Warning | Description Quality |
| 4 | W004 | Warning | Description Quality |
| 5 | W021 | Warning | Composability & Clarity |
| 6 | W026 | Warning | Script Quality |
| 7 | W028 | Warning | Script Quality |
| 8 | I015 | Info | Description Quality |
| 9 | I016 | Info | Content Efficiency |
| 10 | I005 | Info | Discoverability |
| 11 | I006 | Info | Discoverability |

**Total: 2 errors, 5 warnings, 4 info. Terminal summary line shows "5 warnings, 2 errors"** (info-severity diagnostics are not counted in the warning/error tallies but do affect scoring).

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

struct RuleResult {
    rule_id: String,
    passed: bool,
}

enum Grade { A, B, C, D, F }
```

Default grade boundaries (configurable in `.skillplane.toml`). Grade is assigned from the **rounded** composite score:
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
composability-clarity = 0.10
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

Matches the end-to-end worked example (2 errors, 5 warnings, 4 info):

```
skillplane v0.1.0 — my-skill

  Score: 75/100 (C)

  Spec Compliance    ██████████████████░░   94%  (37.7/40)
  Description        ██████████░░░░░░░░░░   50%  (10.0/20)
  Content Efficiency ████████████████░░░░   80%  (12.0/15)
  Composability      █████████████░░░░░░░   67%   (6.7/10)
  Script Quality     ████████████░░░░░░░░   60%   (6.0/10)
  Discoverability    ██████████░░░░░░░░░░   50%   (2.5/5)

  5 warnings, 2 errors

  E030  SKILL.md:1   Unknown field "autor" in frontmatter
  E031  SKILL.md:45  File reference "scripts/deploy.sh" does not exist
  W003  SKILL.md:2   Description is very short (38 chars, recommend >= 50)
  W004  SKILL.md:2   Description lacks trigger language ("Use when...")
  W021  SKILL.md:18  Body contains generic filler content (2 filler phrases detected)
  W026  scripts/build.sh:1  Bash script lacks "set -e" or equivalent error handling
  W028  scripts/build.sh:1  Script has no --help or usage documentation
```

Info-severity diagnostics (I015, I016, I005, I006) affect scoring but are not printed in default terminal output. Use `--format json` or `-v` (verbose, if implemented) to see all diagnostics including info.

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

## Implementation Order

Recommended build sequence for Tier 1 (v0.1). Each phase builds on the previous one and produces a testable milestone.

| Phase | Components | Milestone |
|-------|-----------|-----------|
| **1. Parse** | `model.rs`, `parser.rs` | Can parse any SKILL.md into a `Skill` struct; snapshot tests for valid/malformed inputs |
| **2. Validate** | `validator.rs` (E001-E035), `diagnostics model` | Runs all spec-compliance checks; produces `Vec<Diagnostic>` |
| **3. Lint** | `linter.rs` (W001-W028), `discovery.rs` | Adds best-practice warnings; can discover and lint a directory tree |
| **4. Score** | `scorer.rs`, `config.rs` | Computes composite score + grade; loads `.skillplane.toml` for weight/grade overrides |
| **5. Output** | `terminal.rs`, `json.rs`, `sarif.rs`, `markdown.rs` | All four output formats working |
| **6. CLI** | `main.rs` (clap), exit codes, `--format`/`--min-score`/`--fail-on`/`--disable` flags | Fully usable CLI binary |
| **7. Fix** | `fixer.rs`, `--fix`/`--dry-run` | Auto-repair for the 6 fixable rules |
| **8. Distribute** | `action.yml`, `.pre-commit-hooks.yaml`, CI release pipeline | GitHub Action + pre-commit hook + binary releases |

Within each phase, implement rules in order of their rule ID (E001 before E002, etc.). Write tests alongside each rule using fixture skills in `tests/fixtures/`.

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

## Appendix A: Detection Patterns

Exact string/regex patterns for every heuristic rule. All pattern matching is case-insensitive unless noted. Patterns are applied after stripping markdown formatting (links, emphasis markers) from the target text.

### W004 — Trigger Language Detection

**Fires when:** The `description` field does NOT match any of the following trigger phrase patterns.

```
Trigger phrases (match at any position in description):
  /\buse\s+(this\s+skill\s+)?when\b/i
  /\bwhen\s+the\s+(user|developer|agent)\b/i
  /\bactivate\s+(this\s+)?(skill\s+)?(when|for|if)\b/i
  /\binvoke\s+(this\s+)?(skill\s+)?(when|for|if)\b/i
  /\btrigger(s|ed)?\s+(when|on|by)\b/i
  /\bapplies?\s+(when|to|if)\b/i
```

A description passes W004 if it matches at least one of these patterns.

### W005 — Passive Voice Detection

**Fires when:** The `description` field starts with a passive-voice prefix AND does not also contain trigger language (W004 patterns).

```
Passive-voice prefix patterns (must match at start of description):
  /^this\s+skill\s+(is|was|will|can|should|does|provides|helps|allows|enables|handles|manages|performs)/i
  /^(is|was|will\s+be)\s+used\s+(to|for|when)/i
  /^(a|an|the)\s+(skill|tool|helper|utility)\s+(that|which|for)/i
  /^(provides?|offers?|gives?|enables?|allows?|helps?)\s/i
  /^(designed|intended|meant|built|created)\s+(to|for)\b/i
```

### W020 — Vague Description Anti-Patterns

**Fires when:** The `description` field matches any of these vague phrasing patterns.

```
Vague anti-patterns (match anywhere in description):
  /^helps?\s+(with|the|you)\b/i
  /^a\s+tool\s+(for|to|that)\b/i
  /^an?\s+(useful|helpful|handy|simple|basic|generic)\s+(skill|tool)\b/i
  /^(does|handles?)\s+(stuff|things|various|everything|anything)\b/i
  /\b(various|miscellaneous|general[- ]purpose|multi[- ]purpose|all[- ]purpose)\s+(tasks?|things?|operations?|functions?)\b/i
  /^(utility|helper)\s+(for|to|that|skill)\b/i
```

### W021 — Generic Filler Phrase Detection

**Fires when:** The markdown body contains 2 or more distinct matches from the filler phrase list below (a single match is tolerated to reduce false positives).

```
Filler phrases (match anywhere in body, case-insensitive):
  /\bas\s+you\s+(probably\s+)?(already\s+)?know\b/i
  /\bit\s+is\s+(widely|generally|commonly)\s+known\s+that\b/i
  /\b(remember|note|keep\s+in\s+mind)\s+that\s+(all|every|most)\b/i
  /\bin\s+(today's|the\s+modern|the\s+current)\s+(world|landscape|ecosystem)\b/i
  /\b(basically|essentially|fundamentally|obviously|clearly)\b/i  — only when NOT inside a code block
  /\bthis\s+is\s+(important|crucial|critical|essential|vital)\s+because\b/i
  /\b(best\s+practices?\s+dictate|industry\s+standard\s+is|it\s+is\s+recommended)\b/i
  /\bfor\s+more\s+(information|details),?\s+see\s+(the\s+)?(official\s+)?documentation\b/i
```

Threshold: fires when `distinct_matches >= 2`. Each pattern counts at most once regardless of how many times it appears.

### W023 — Action Verb Detection

**Fires when:** The `description` field does NOT contain at least one word from the action verb list.

```
Action verb list (must appear as a whole word in description):
  use, run, execute, invoke, call, apply, generate, create, build,
  deploy, configure, set up, install, validate, check, lint, test,
  format, transform, convert, parse, analyze, scan, detect, fix,
  migrate, upgrade, refactor, optimize, monitor, debug, log, trace,
  fetch, pull, push, sync, upload, download, export, import, send,
  render, compile, bundle, serve, start, stop, restart, reset, clean,
  scaffold, bootstrap, initialize, provision, authenticate, authorize
```

Pattern: `/\b(use|run|execute|invoke|call|apply|generate|create|build|deploy|configure|set\s+up|install|validate|check|lint|test|format|transform|convert|parse|analyze|scan|detect|fix|migrate|upgrade|refactor|optimize|monitor|debug|log|trace|fetch|pull|push|sync|upload|download|export|import|send|render|compile|bundle|serve|start|stop|restart|reset|clean|scaffold|bootstrap|initialize|provision|authenticate|authorize)\b/i`

### E035 — Secret/Credential Detection

**Fires when:** Any text file in the skill directory matches one or more of the following patterns. Binary files (detected by null byte in first 8192 bytes) are skipped.

```
Secret patterns (applied per-line to every text file in the skill directory):

  # AWS Access Key ID (starts with AKIA)
  /\bAKIA[0-9A-Z]{16}\b/

  # AWS Secret Access Key (40 chars base64-ish after common prefixes)
  /(?:aws_secret_access_key|secret_key)\s*[:=]\s*["']?[A-Za-z0-9/+=]{40}\b/i

  # GitHub tokens (classic and fine-grained)
  /\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,255}\b/
  /\bgithub_pat_[A-Za-z0-9_]{22,255}\b/

  # Generic private key header
  /-----BEGIN\s+(RSA|EC|DSA|OPENSSH|PGP)?\s*PRIVATE\s+KEY-----/

  # Generic high-entropy API key patterns
  /(?:api[_-]?key|api[_-]?secret|auth[_-]?token|access[_-]?token|bearer)\s*[:=]\s*["']?[A-Za-z0-9_\-]{20,}\b/i

  # Slack tokens
  /\bxox[bpors]-[0-9]{10,}-[A-Za-z0-9_\-]{10,}\b/

  # Generic password in assignment
  /(?:password|passwd|pwd)\s*[:=]\s*["'][^"']{8,}["']/i
```

Lines inside YAML comments (`# ...`) and markdown code blocks with explicit `example` or `placeholder` language tags are excluded from scanning.

### I015 — Keyword Diversity Score

**Fires when:** The description covers fewer than 2 distinct trigger scenarios.

Detection method: the description is scanned for **scenario indicator clauses** — independent segments that each describe a distinct use case. The count is determined by:

1. Split the description on sentence boundaries (`. `, `; `, ` - `, newlines) and coordinating conjunctions used to join scenarios (`or when`, `also when`, `and when`, `as well as when`).
2. Each segment that matches a trigger-like pattern (W004 pattern list, or starts with an imperative verb from the W023 list) counts as one scenario.
3. Segments that are pure continuation of the same clause (e.g., subordinate clauses starting with `that`, `which`, `by`, `via`) do not count as additional scenarios.

The rule fires (I015 diagnostic emitted) when `scenario_count < 2`. For scoring purposes (DQ6), the check passes when `scenario_count >= 2`.

### I016 — Progressive Disclosure Ratio

**Fires when:** 100% of the skill's text content is in SKILL.md with 0% in referenced files (ratio = 0.0).

Detection method:

1. Compute `skillmd_size` = byte length of the SKILL.md body (after frontmatter).
2. Compute `referenced_size` = sum of byte lengths of all files in `scripts/`, `references/`, `examples/`, and `assets/` directories.
3. `ratio = referenced_size / (skillmd_size + referenced_size)`.
4. The rule fires when `ratio == 0.0` (i.e., `referenced_size == 0`).

For scoring purposes (CE3), the check passes when `ratio > 0.0` (any non-zero amount of content in referenced files). The ratio value itself is reported in the I016 info diagnostic message for user visibility (e.g., "Progressive disclosure ratio: 0% of content in referenced files").

## Appendix B: Output Schemas

### JSON Output

Produced by `--format json`. The top-level object contains one entry per skill path. All field names use snake_case.

```json
{
  "version": "0.1.0",
  "skills": [
    {
      "path": "skills/my-skill",
      "score": {
        "composite": 75,
        "grade": "C",
        "categories": [
          {
            "name": "spec_compliance",
            "weight": 0.40,
            "score": 94.3,
            "weighted_score": 37.7,
            "rule_results": [
              { "rule_id": "E001", "passed": true },
              { "rule_id": "E030", "passed": false },
              { "rule_id": "E031", "passed": false }
            ]
          }
        ]
      },
      "diagnostics": [
        {
          "rule_id": "E030",
          "severity": "error",
          "message": "Unknown field \"autor\" in frontmatter",
          "path": "skills/my-skill/SKILL.md",
          "span": {
            "start_line": 1,
            "start_col": 1,
            "end_line": 1,
            "end_col": 20
          },
          "fix_available": false,
          "category": "spec_compliance"
        },
        {
          "rule_id": "E001",
          "severity": "error",
          "message": "SKILL.md file missing",
          "path": "skills/other-skill",
          "span": null,
          "fix_available": false,
          "category": "spec_compliance"
        }
      ],
      "summary": {
        "errors": 2,
        "warnings": 5,
        "info": 4
      }
    }
  ]
}
```

Field types:
- `version`: string — skillplane version that produced the output
- `skills[].score.composite`: integer — rounded composite score (0-100)
- `skills[].score.grade`: string — one of "A", "B", "C", "D", "F"
- `skills[].score.categories[].score`: float — per-category percentage (0.0-100.0)
- `skills[].score.categories[].weighted_score`: float — score * weight
- `skills[].diagnostics[].span`: object | null — null for structural rules (E001, E033)
- `skills[].diagnostics[].severity`: string — one of "error", "warning", "info"
- `skills[].diagnostics[].category`: string — snake_case category name

### SARIF Output

Produced by `--format sarif`. Conforms to SARIF v2.1.0 for GitHub code scanning integration.

```json
{
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "skillplane",
          "version": "0.1.0",
          "informationUri": "https://github.com/your-org/skillplane",
          "rules": [
            {
              "id": "E030",
              "shortDescription": { "text": "Unknown/unexpected fields in frontmatter" },
              "defaultConfiguration": { "level": "error" }
            }
          ]
        }
      },
      "results": [
        {
          "ruleId": "E030",
          "level": "error",
          "message": { "text": "Unknown field \"autor\" in frontmatter" },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": { "uri": "skills/my-skill/SKILL.md" },
                "region": {
                  "startLine": 1,
                  "startColumn": 1,
                  "endLine": 1,
                  "endColumn": 20
                }
              }
            }
          ]
        },
        {
          "ruleId": "E001",
          "level": "error",
          "message": { "text": "SKILL.md file missing" },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": { "uri": "skills/other-skill" }
              }
            }
          ]
        }
      ]
    }
  ]
}
```

Key mapping details:
- `rule_id` maps to SARIF `ruleId` (e.g., "E030")
- `Severity::Error` maps to SARIF `level: "error"`, `Warning` to `"warning"`, `Info` to `"note"`
- When `span` is `None` (structural rules E001, E033): the SARIF `location` omits the `region` property entirely, using only `artifactLocation` with the directory path
- When `span` is `Some`: the `region` object maps `start_line`/`start_col`/`end_line`/`end_col` to SARIF's `startLine`/`startColumn`/`endLine`/`endColumn`

### Markdown Output

Produced by `--format markdown`. Designed for GitHub job summaries and PR comments.

```markdown
## Skillplane Report — my-skill

**Score: 75/100 (C)**

| Category | Score | Weighted |
|----------|-------|----------|
| Spec Compliance | 94% | 37.7/40 |
| Description Quality | 50% | 10.0/20 |
| Content Efficiency | 80% | 12.0/15 |
| Composability & Clarity | 67% | 6.7/10 |
| Script Quality | 60% | 6.0/10 |
| Discoverability | 50% | 2.5/5 |

### Errors (2)

| Rule | Location | Message |
|------|----------|---------|
| E030 | `SKILL.md:1` | Unknown field "autor" in frontmatter |
| E031 | `SKILL.md:45` | File reference "scripts/deploy.sh" does not exist |

### Warnings (5)

| Rule | Location | Message |
|------|----------|---------|
| W003 | `SKILL.md:2` | Description is very short (38 chars, recommend >= 50) |
| W004 | `SKILL.md:2` | Description lacks trigger language ("Use when...") |
| W021 | `SKILL.md:18` | Body contains generic filler content (2 filler phrases detected) |
| W026 | `scripts/build.sh:1` | Bash script lacks "set -e" or equivalent error handling |
| W028 | `scripts/build.sh:1` | Script has no --help or usage documentation |
```

For multi-skill repos, each skill gets its own H2 section, followed by an aggregate summary section.

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
