# Skillplane CLI, Output, Fix & Distribution Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build output formatters, CLI binary, fix mode, and distribution — Phases 5-8 from the spec.

**Architecture:** Adds output formatters to `skillplane-core`, implements the `skillplane` CLI binary with clap, adds fix mode, and creates GitHub Action + pre-commit hook configs.

**Tech Stack:** clap (CLI), colored (terminal), serde_json (JSON/SARIF), existing skillplane-core library

---

### Task 1: Terminal Output Formatter

**Files:**
- Create: `crates/skillplane-core/src/output/mod.rs`
- Create: `crates/skillplane-core/src/output/terminal.rs`

Implement `format_terminal(skills: &[SkillReport], quiet: bool) -> String` that produces the colored bar-chart output from the spec. A `SkillReport` bundles a skill path, its diagnostics, and its scorecard.

Define in `output/mod.rs`:
```rust
pub struct SkillReport {
    pub path: PathBuf,
    pub diagnostics: Vec<Diagnostic>,
    pub score: Option<ScoreCard>,  // None when --no-score
}
```

Terminal format per skill:
- Header: `skillplane v0.1.0 — {skill_name}`
- Score bar chart (6 categories with progress bars, percentages, weighted scores)
- Summary: `{warnings} warnings, {errors} errors`
- Diagnostic list (errors first, then warnings — skip Info severity)
- Quiet mode: only diagnostic list, no score

---

### Task 2: JSON Output Formatter

**Files:**
- Create: `crates/skillplane-core/src/output/json.rs`

Implement `format_json(skills: &[SkillReport]) -> String` matching the JSON schema from Appendix B of the spec. Use `serde_json` with `Serialize` derives on output structs.

---

### Task 3: SARIF Output Formatter

**Files:**
- Create: `crates/skillplane-core/src/output/sarif.rs`

Implement `format_sarif(skills: &[SkillReport]) -> String` matching SARIF v2.1.0 from Appendix B. Key mappings: rule_id → SARIF ruleId, Severity → SARIF level (Error→error, Warning→warning, Info→note), span:None → omit region.

---

### Task 4: Markdown Output Formatter

**Files:**
- Create: `crates/skillplane-core/src/output/markdown.rs`

Implement `format_markdown(skills: &[SkillReport]) -> String` matching the markdown template from Appendix B. H2 per skill, score table, errors table, warnings table.

---

### Task 5: CLI Binary (clap)

**Files:**
- Modify: `crates/skillplane/Cargo.toml` (add clap, colored deps)
- Rewrite: `crates/skillplane/src/main.rs`

Implement the full CLI with clap:
- `check` subcommand (default): discover → validate → lint → score → format → output
- `fix` subcommand: discover → validate → fix → re-validate → output
- All flags from the spec: --format, --min-score, --fail-on, --fix, --dry-run, --no-score, --quiet, --color, --disable, --experimental, --exclude, --config
- Exit codes: 0 (pass), 1 (errors/warnings), 2 (score below threshold), 3 (no skills found)
- Precedence: 1 > 2 > 3

---

### Task 6: Fix Mode

**Files:**
- Create: `crates/skillplane-core/src/fixer.rs`

Implement `fix_skill(skill_dir: &Path, dry_run: bool) -> FixResult` covering the 6 fixable rules:
- E002: rename skill.md → SKILL.md
- E003: inject frontmatter skeleton if missing
- E012: lowercase the name field
- E013: set name to match directory
- E032: append closing ---
- E034: strip UTF-8 BOM

FixResult reports what changed, what was written, and any remaining issues.

---

### Task 7: GitHub Action & Pre-commit Hook

**Files:**
- Create: `action.yml`
- Create: `.pre-commit-hooks.yaml`

Create the distribution configs from the spec. The action.yml uses `composite` runs type. The pre-commit hooks define `skillplane` (check) and `skillplane-fix` (manual fix).

---

### Task 8: Final Integration Test

**Files:**
- Modify: `crates/skillplane-core/tests/integration.rs`

Add tests for output formatters and fix mode. Test the CLI binary with `assert_cmd`.
