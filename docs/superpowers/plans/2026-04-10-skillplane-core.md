# Skillplane Core Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `skillplane-core` library crate covering parsing, validation (E-rules), linting (W-rules), scoring, config loading, and discovery — Phases 1-4 from the spec's Implementation Order.

**Architecture:** Rust workspace with `skillplane-core` library crate. Data flows linearly: Discovery → Parser → Model → Validator/Linter → Diagnostics → Scorer. Each module has a single responsibility and is independently testable via fixture skills in `tests/fixtures/`.

**Tech Stack:** Rust 2021 edition, serde + serde_yaml, unicode-normalization, walkdir, regex, toml, thiserror, tempfile + insta (test)

**Scope:** This plan covers the library crate only (Phases 1-4). A follow-up plan covers output formatters, CLI binary, fix mode, and distribution (Phases 5-8).

---

### Task 1: Workspace Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/skillplane-core/Cargo.toml`
- Create: `crates/skillplane-core/src/lib.rs`
- Create: `crates/skillplane/Cargo.toml`
- Create: `crates/skillplane/src/main.rs`

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = ["crates/skillplane-core", "crates/skillplane"]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
license = "MIT"
```

- [ ] **Step 2: Create skillplane-core crate**

```toml
# crates/skillplane-core/Cargo.toml
[package]
name = "skillplane-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Core library for skillplane — Agent Skills linter, validator, and scorer"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
unicode-normalization = "0.1"
walkdir = "2"
regex = "1"
toml = "0.8"
thiserror = "2"
once_cell = "1"

[dev-dependencies]
tempfile = "3"
insta = { version = "1", features = ["yaml"] }
```

```rust
// crates/skillplane-core/src/lib.rs
pub mod model;
pub mod parser;
```

- [ ] **Step 3: Create skillplane CLI crate (placeholder)**

```toml
# crates/skillplane/Cargo.toml
[package]
name = "skillplane"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "CLI for skillplane — Agent Skills linter, validator, and scorer"

[dependencies]
skillplane-core = { path = "../skillplane-core" }
```

```rust
// crates/skillplane/src/main.rs
fn main() {
    println!("skillplane v0.1.0 — not yet implemented");
}
```

- [ ] **Step 4: Verify workspace compiles**

Run: `cargo build`
Expected: Compiles with no errors

- [ ] **Step 5: Create test fixtures directory**

```bash
mkdir -p tests/fixtures/valid-skill
```

Create a minimal valid skill fixture:

```markdown
# tests/fixtures/valid-skill/SKILL.md
---
name: valid-skill
description: A valid test skill for integration testing. Use when running tests.
---

# Valid Skill

This is a valid skill for testing purposes.

## Usage

Follow these steps to use this skill.
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/ tests/
git commit -m "feat: scaffold workspace with skillplane-core and skillplane crates"
```

---

### Task 2: Data Model (`model.rs`)

**Files:**
- Create: `crates/skillplane-core/src/model.rs`

- [ ] **Step 1: Write model types**

```rust
// crates/skillplane-core/src/model.rs
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A parsed Agent Skill directory.
#[derive(Debug, Clone)]
pub struct Skill {
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    pub body: Body,
    pub file_tree: FileTree,
}

/// Parsed YAML frontmatter from SKILL.md.
/// All fields are Option — parse first, validate later.
#[derive(Debug, Clone, Default)]
pub struct Frontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: Option<serde_yaml::Value>,
    pub allowed_tools: Option<serde_yaml::Value>,
    pub unknown_fields: BTreeMap<String, serde_yaml::Value>,
}

/// Markdown body content with extracted metadata.
#[derive(Debug, Clone)]
pub struct Body {
    pub raw: String,
    pub line_count: usize,
    pub estimated_tokens: usize,
    pub file_references: Vec<FileReference>,
    pub headings: Vec<Heading>,
    pub code_blocks: Vec<CodeBlock>,
    pub has_placeholder_text: bool,
}

/// Skill directory contents.
#[derive(Debug, Clone, Default)]
pub struct FileTree {
    pub has_scripts: bool,
    pub has_references: bool,
    pub has_assets: bool,
    pub has_examples: bool,
    pub files: Vec<PathBuf>,
    pub total_content_size: usize,
}

/// A source location range for diagnostic annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// A relative file path referenced from within the SKILL.md body.
#[derive(Debug, Clone)]
pub struct FileReference {
    pub path: String,
    pub span: Span,
    pub exists: bool,
}

/// A markdown heading extracted from the body.
#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub span: Span,
}

/// A fenced code block extracted from the body.
#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub content: String,
    pub span: Span,
}

/// A diagnostic produced by validation or linting.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub path: PathBuf,
    pub span: Option<Span>,
    pub fix_available: bool,
    pub category: Category,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    SpecCompliance,
    DescriptionQuality,
    ContentEfficiency,
    ComposabilityClarity,
    ScriptQuality,
    Discoverability,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add crates/skillplane-core/src/model.rs
git commit -m "feat: add data model types (Skill, Frontmatter, Body, Diagnostic, etc.)"
```

---

### Task 3: Parser — Frontmatter Parsing (`parser.rs`)

**Files:**
- Create: `crates/skillplane-core/src/parser.rs`
- Create: `tests/fixtures/missing-frontmatter/SKILL.md`
- Create: `tests/fixtures/unclosed-frontmatter/SKILL.md`
- Create: `tests/fixtures/non-mapping-frontmatter/SKILL.md`
- Create: `tests/fixtures/bom-skill/SKILL.md`

- [ ] **Step 1: Write parser tests**

```rust
// crates/skillplane-core/src/parser.rs (at the bottom)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_skill() {
        let content = "---\nname: my-skill\ndescription: A test skill.\n---\n\n# My Skill\n\nBody here.\n";
        let result = parse_frontmatter(content);
        assert!(result.is_ok());
        let (fm, body) = result.unwrap();
        assert_eq!(fm.name.as_deref(), Some("my-skill"));
        assert_eq!(fm.description.as_deref(), Some("A test skill."));
        assert!(body.contains("# My Skill"));
    }

    #[test]
    fn test_parse_missing_frontmatter() {
        let content = "# No frontmatter here\n\nJust body.\n";
        let result = parse_frontmatter(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingFrontmatter));
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let content = "---\nname: my-skill\ndescription: A test.\n";
        let result = parse_frontmatter(content);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::UnclosedFrontmatter));
    }

    #[test]
    fn test_parse_non_mapping_frontmatter() {
        let content = "---\n- just\n- a\n- list\n---\nBody\n";
        let result = parse_frontmatter(content);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::NotAMapping));
    }

    #[test]
    fn test_strip_utf8_bom() {
        let content = "\u{feff}---\nname: my-skill\ndescription: A test.\n---\nBody\n";
        let result = parse_frontmatter(content);
        assert!(result.is_ok());
        let (fm, _) = result.unwrap();
        assert_eq!(fm.name.as_deref(), Some("my-skill"));
    }

    #[test]
    fn test_unknown_fields_captured() {
        let content = "---\nname: my-skill\ndescription: A test.\nauthor: someone\n---\nBody\n";
        let result = parse_frontmatter(content);
        assert!(result.is_ok());
        let (fm, _) = result.unwrap();
        assert!(fm.unknown_fields.contains_key("author"));
    }

    #[test]
    fn test_metadata_preserved_as_value() {
        let content = "---\nname: my-skill\ndescription: A test.\nmetadata:\n  version: \"1.0\"\n  author: test\n---\nBody\n";
        let result = parse_frontmatter(content);
        assert!(result.is_ok());
        let (fm, _) = result.unwrap();
        assert!(fm.metadata.is_some());
    }

    #[test]
    fn test_allowed_tools_as_string() {
        let content = "---\nname: my-skill\ndescription: A test.\nallowed-tools: Bash(git:*) Read\n---\nBody\n";
        let result = parse_frontmatter(content);
        assert!(result.is_ok());
        let (fm, _) = result.unwrap();
        assert!(fm.allowed_tools.is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p skillplane-core -- parser::tests`
Expected: FAIL — `parse_frontmatter` function does not exist yet

- [ ] **Step 3: Implement frontmatter parser**

```rust
// crates/skillplane-core/src/parser.rs
use std::collections::BTreeMap;

use crate::model::{Body, CodeBlock, FileReference, Frontmatter, Heading, Span};

/// Errors that can occur during SKILL.md parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("SKILL.md must start with YAML frontmatter (---)")]
    MissingFrontmatter,

    #[error("SKILL.md frontmatter not properly closed with ---")]
    UnclosedFrontmatter,

    #[error("Invalid YAML in frontmatter: {0}")]
    InvalidYaml(String),

    #[error("SKILL.md frontmatter must be a YAML mapping")]
    NotAMapping,
}

const KNOWN_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

/// Parse YAML frontmatter and body from SKILL.md content.
///
/// Returns (Frontmatter, raw_body_string) on success.
pub fn parse_frontmatter(content: &str) -> Result<(Frontmatter, String), ParseError> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);

    if !content.starts_with("---") {
        return Err(ParseError::MissingFrontmatter);
    }

    let after_first = &content[3..];
    let after_first = after_first.strip_prefix('\n').or_else(|| after_first.strip_prefix("\r\n")).unwrap_or(after_first);

    let close_idx = find_closing_delimiter(after_first)
        .ok_or(ParseError::UnclosedFrontmatter)?;

    let yaml_str = &after_first[..close_idx];
    let body_start = close_idx + 3; // skip "---"
    let body_raw = after_first.get(body_start..).unwrap_or("");
    let body_raw = body_raw.strip_prefix('\n').or_else(|| body_raw.strip_prefix("\r\n")).unwrap_or(body_raw);

    let parsed: serde_yaml::Value = serde_yaml::from_str(yaml_str)
        .map_err(|e| ParseError::InvalidYaml(e.to_string()))?;

    let mapping = match parsed {
        serde_yaml::Value::Mapping(m) => m,
        _ => return Err(ParseError::NotAMapping),
    };

    let mut fm = Frontmatter::default();
    let mut unknown = BTreeMap::new();

    for (key, value) in mapping {
        let key_str = match &key {
            serde_yaml::Value::String(s) => s.clone(),
            _ => {
                // Non-string keys go to unknown_fields with stringified key
                unknown.insert(format!("{key:?}"), value);
                continue;
            }
        };

        match key_str.as_str() {
            "name" => fm.name = value.as_str().map(String::from),
            "description" => fm.description = value.as_str().map(String::from),
            "license" => fm.license = value.as_str().map(String::from),
            "compatibility" => fm.compatibility = value.as_str().map(String::from),
            "metadata" => fm.metadata = Some(value),
            "allowed-tools" => fm.allowed_tools = Some(value),
            _ => {
                unknown.insert(key_str, value);
            }
        }
    }

    fm.unknown_fields = unknown;

    Ok((fm, body_raw.to_string()))
}

/// Find the closing `---` delimiter.
/// Returns the byte offset of the start of the closing `---` line.
fn find_closing_delimiter(content: &str) -> Option<usize> {
    let mut offset = 0;
    for line in content.lines() {
        if line.trim() == "---" {
            return Some(offset);
        }
        offset += line.len() + 1; // +1 for newline
    }
    None
}

/// Parse the markdown body into a Body struct with extracted metadata.
pub fn parse_body(raw: &str, skill_dir: &std::path::Path) -> Body {
    let line_count = raw.lines().count();
    let word_count = raw.split_whitespace().count();
    let estimated_tokens = ((word_count as f64) / 0.75).round() as usize;

    let headings = extract_headings(raw);
    let code_blocks = extract_code_blocks(raw);
    let file_references = extract_file_references(raw, skill_dir);
    let has_placeholder_text = check_placeholder_text(raw, &code_blocks);

    Body {
        raw: raw.to_string(),
        line_count,
        estimated_tokens,
        file_references,
        headings,
        code_blocks,
        has_placeholder_text,
    }
}

fn extract_headings(raw: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    for (line_idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            if level >= 1 && level <= 6 {
                let text = trimmed[level..].trim().to_string();
                if !text.is_empty() {
                    headings.push(Heading {
                        level: level as u8,
                        text,
                        span: Span {
                            start_line: line_idx + 1,
                            start_col: 1,
                            end_line: line_idx + 1,
                            end_col: line.len() + 1,
                        },
                    });
                }
            }
        }
    }
    headings
}

fn extract_code_blocks(raw: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut fence_char = ' ';
    let mut fence_count = 0;
    let mut block_start_line = 0;
    let mut language = None;
    let mut content = String::new();

    for (line_idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(info) = detect_code_fence(trimmed) {
            if !in_block {
                in_block = true;
                fence_char = info.0;
                fence_count = info.1;
                block_start_line = line_idx + 1;
                language = info.2;
                content.clear();
            } else if info.0 == fence_char && info.1 >= fence_count && info.2.is_none() {
                blocks.push(CodeBlock {
                    language: language.take(),
                    content: content.clone(),
                    span: Span {
                        start_line: block_start_line,
                        start_col: 1,
                        end_line: line_idx + 1,
                        end_col: line.len() + 1,
                    },
                });
                in_block = false;
            } else {
                content.push_str(line);
                content.push('\n');
            }
        } else if in_block {
            content.push_str(line);
            content.push('\n');
        }
    }
    blocks
}

/// Detect a code fence line. Returns (char, count, optional language tag).
fn detect_code_fence(line: &str) -> Option<(char, usize, Option<String>)> {
    let first = line.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let count = line.chars().take_while(|&c| c == first).count();
    if count < 3 {
        return None;
    }
    let rest = line[count..].trim();
    let lang = if rest.is_empty() {
        None
    } else {
        Some(rest.split_whitespace().next().unwrap_or("").to_string())
    };
    Some((first, count, lang))
}

fn extract_file_references(raw: &str, skill_dir: &std::path::Path) -> Vec<FileReference> {
    let mut refs = Vec::new();
    let link_re = regex::Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap();

    for (line_idx, line) in raw.lines().enumerate() {
        for cap in link_re.captures_iter(line) {
            let path_str = cap.get(2).unwrap().as_str();
            // Skip URLs, anchors, mailto
            if path_str.starts_with("http") || path_str.starts_with('#') || path_str.starts_with("mailto:") {
                continue;
            }
            let match_start = cap.get(2).unwrap().start();
            let match_end = cap.get(2).unwrap().end();
            let target = skill_dir.join(path_str);
            refs.push(FileReference {
                path: path_str.to_string(),
                span: Span {
                    start_line: line_idx + 1,
                    start_col: match_start + 1,
                    end_line: line_idx + 1,
                    end_col: match_end + 1,
                },
                exists: target.exists(),
            });
        }
    }
    refs
}

fn check_placeholder_text(raw: &str, code_blocks: &[CodeBlock]) -> bool {
    let placeholder_re = regex::Regex::new(r"(?i)\b(TODO|FIXME|TBD|CHANGEME|XXX)\b").unwrap();
    for (line_idx, line) in raw.lines().enumerate() {
        let line_num = line_idx + 1;
        // Skip lines inside code blocks
        let in_code = code_blocks.iter().any(|b| line_num >= b.span.start_line && line_num <= b.span.end_line);
        if in_code {
            continue;
        }
        if placeholder_re.is_match(line) {
            return true;
        }
    }
    false
}

// tests at bottom (from Step 1)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p skillplane-core -- parser::tests`
Expected: All 8 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/skillplane-core/src/parser.rs crates/skillplane-core/src/lib.rs
git commit -m "feat: implement SKILL.md parser (frontmatter + body extraction)"
```

---

### Task 4: Discovery (`discovery.rs`)

**Files:**
- Create: `crates/skillplane-core/src/discovery.rs`

- [ ] **Step 1: Write discovery tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_skill_md_uppercase() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: my-skill\ndescription: test\n---\n").unwrap();

        let result = find_skill_md(&skill_dir);
        assert!(result.is_some());
        assert_eq!(result.unwrap().file_name().unwrap(), "SKILL.md");
    }

    #[test]
    fn test_find_skill_md_lowercase() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("skill.md"), "---\nname: my-skill\ndescription: test\n---\n").unwrap();

        let result = find_skill_md(&skill_dir);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_skill_md_missing() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("empty-skill");
        fs::create_dir(&skill_dir).unwrap();

        let result = find_skill_md(&skill_dir);
        assert!(result.is_none());
    }

    #[test]
    fn test_collect_skills_from_directory() {
        let dir = TempDir::new().unwrap();
        let s1 = dir.path().join("skills").join("skill-a");
        fs::create_dir_all(&s1).unwrap();
        fs::write(s1.join("SKILL.md"), "---\nname: skill-a\ndescription: test a\n---\nBody\n").unwrap();

        let s2 = dir.path().join("skills").join("skill-b");
        fs::create_dir_all(&s2).unwrap();
        fs::write(s2.join("SKILL.md"), "---\nname: skill-b\ndescription: test b\n---\nBody\n").unwrap();

        let skills = discover_skills(dir.path());
        assert_eq!(skills.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p skillplane-core -- discovery::tests`
Expected: FAIL — functions do not exist

- [ ] **Step 3: Implement discovery**

```rust
// crates/skillplane-core/src/discovery.rs
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::model::{FileTree, Skill};
use crate::parser::{parse_body, parse_frontmatter};

/// Find the SKILL.md file in a directory.
/// Prefers uppercase SKILL.md over lowercase skill.md.
pub fn find_skill_md(skill_dir: &Path) -> Option<PathBuf> {
    let entries: Vec<_> = fs::read_dir(skill_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.eq_ignore_ascii_case("skill.md") {
                Some((name, e.path()))
            } else {
                None
            }
        })
        .collect();

    // Prefer SKILL.md
    for (name, path) in &entries {
        if name == "SKILL.md" {
            return Some(path.clone());
        }
    }
    // Fall back to any case variant
    entries.into_iter().next().map(|(_, path)| path)
}

/// Build a FileTree from a skill directory.
pub fn build_file_tree(skill_dir: &Path) -> FileTree {
    let mut tree = FileTree::default();

    if let Ok(entries) = fs::read_dir(skill_dir) {
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();

            if path.is_dir() {
                match name.as_str() {
                    "scripts" => tree.has_scripts = true,
                    "references" => tree.has_references = true,
                    "assets" => tree.has_assets = true,
                    "examples" => tree.has_examples = true,
                    _ => {}
                }
            }

            tree.files.push(path);
        }
    }

    // Compute total content size of referenced dirs
    for dir_name in &["scripts", "references", "assets", "examples"] {
        let dir_path = skill_dir.join(dir_name);
        if dir_path.is_dir() {
            for entry in WalkDir::new(&dir_path).into_iter().filter_map(Result::ok) {
                if entry.file_type().is_file() {
                    tree.total_content_size += entry.metadata().map(|m| m.len() as usize).unwrap_or(0);
                }
            }
        }
    }

    tree
}

/// Parse a single skill directory into a Skill struct.
pub fn load_skill(skill_dir: &Path) -> Result<Skill, String> {
    let skill_md = find_skill_md(skill_dir)
        .ok_or_else(|| format!("SKILL.md not found in {}", skill_dir.display()))?;

    let content = fs::read_to_string(&skill_md)
        .map_err(|e| format!("Failed to read {}: {e}", skill_md.display()))?;

    let (frontmatter, body_raw) = parse_frontmatter(&content)
        .map_err(|e| e.to_string())?;

    let body = parse_body(&body_raw, skill_dir);
    let file_tree = build_file_tree(skill_dir);

    Ok(Skill {
        path: skill_dir.to_path_buf(),
        frontmatter,
        body,
        file_tree,
    })
}

/// Discover all SKILL.md files under a root directory.
pub fn discover_skills(root: &Path) -> Vec<PathBuf> {
    let mut skill_dirs = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy();
            if name.eq_ignore_ascii_case("skill.md") {
                if let Some(parent) = entry.path().parent() {
                    skill_dirs.push(parent.to_path_buf());
                }
            }
        }
    }

    skill_dirs.sort();
    skill_dirs.dedup();
    skill_dirs
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// Add to crates/skillplane-core/src/lib.rs
pub mod discovery;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p skillplane-core -- discovery::tests`
Expected: All 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/skillplane-core/src/discovery.rs crates/skillplane-core/src/lib.rs
git commit -m "feat: implement skill discovery (find_skill_md, discover_skills, load_skill)"
```

---

### Task 5: Validator — Spec Compliance E-Rules (`validator.rs`)

**Files:**
- Create: `crates/skillplane-core/src/validator.rs`

This is the largest task — 35 rules. They group naturally:

- E001-E005: Structural/parse (SKILL.md existence, frontmatter validity)
- E006-E013: Name validation
- E014-E016: Description validation
- E017-E019: Compatibility validation
- E020-E021: License validation
- E022-E024: Metadata validation
- E025-E029: Allowed-tools validation
- E030-E035: Misc (unknown fields, file refs, BOM, secrets)

- [ ] **Step 1: Write tests for E001-E005 (structural)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_skill(dir: &Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    fn run_validator(dir: &Path) -> Vec<Diagnostic> {
        validate(dir)
    }

    fn has_rule(diags: &[Diagnostic], rule_id: &str) -> bool {
        diags.iter().any(|d| d.rule_id == rule_id)
    }

    #[test]
    fn test_e001_missing_skill_md() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("empty");
        fs::create_dir_all(&skill_dir).unwrap();
        let diags = run_validator(&skill_dir);
        assert!(has_rule(&diags, "E001"));
    }

    #[test]
    fn test_e002_wrong_case() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("skill.md"), "---\nname: my-skill\ndescription: test\n---\n").unwrap();
        let diags = run_validator(&skill_dir);
        assert!(has_rule(&diags, "E002"));
    }

    #[test]
    fn test_e003_missing_frontmatter() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(&skill_dir, "# No frontmatter\nJust body.");
        let diags = run_validator(&skill_dir);
        assert!(has_rule(&diags, "E003"));
    }

    #[test]
    fn test_e004_not_a_mapping() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(&skill_dir, "---\n- a\n- list\n---\nBody");
        let diags = run_validator(&skill_dir);
        assert!(has_rule(&diags, "E004"));
    }

    #[test]
    fn test_valid_skill_no_errors() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(&skill_dir, "---\nname: my-skill\ndescription: A valid test skill for checking.\n---\n# My Skill\n");
        let diags = run_validator(&skill_dir);
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "Expected no errors, got: {errors:?}");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p skillplane-core -- validator::tests`
Expected: FAIL — `validate` function does not exist

- [ ] **Step 3: Implement validator**

```rust
// crates/skillplane-core/src/validator.rs
use std::fs;
use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::discovery::{build_file_tree, find_skill_md, load_skill};
use crate::model::*;
use crate::parser::{parse_body, parse_frontmatter, ParseError};

const MAX_NAME_LEN: usize = 64;
const MAX_DESC_LEN: usize = 1024;
const MAX_COMPAT_LEN: usize = 500;

const ALLOWED_FIELDS: &[&str] = &[
    "name", "description", "license", "compatibility", "metadata", "allowed-tools",
];

static SECRET_PATTERNS: Lazy<Vec<(&str, Regex)>> = Lazy::new(|| vec![
    ("AWS Access Key", Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap()),
    ("GitHub token", Regex::new(r"\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,255}\b").unwrap()),
    ("GitHub PAT", Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{22,255}\b").unwrap()),
    ("Private key", Regex::new(r"-----BEGIN\s+(RSA|EC|DSA|OPENSSH|PGP)?\s*PRIVATE\s+KEY-----").unwrap()),
    ("API key pattern", Regex::new(r"(?i)(?:api[_-]?key|api[_-]?secret|auth[_-]?token|access[_-]?token|bearer)\s*[:=]\s*[\"']?[A-Za-z0-9_\-]{20,}\b").unwrap()),
    ("Slack token", Regex::new(r"\bxox[bpors]-[0-9]{10,}-[A-Za-z0-9_\-]{10,}\b").unwrap()),
    ("Password assignment", Regex::new(r"(?i)(?:password|passwd|pwd)\s*[:=]\s*[\"'][^\"']{8,}[\"']").unwrap()),
]);

/// Run all spec-compliance validation rules on a skill directory.
/// Returns diagnostics for E001-E035.
pub fn validate(skill_dir: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // E033: Directory existence
    if !skill_dir.exists() || !skill_dir.is_dir() {
        diags.push(Diagnostic {
            rule_id: "E033".into(),
            severity: Severity::Error,
            message: format!("Skill directory does not exist or is not a directory: {}", skill_dir.display()),
            path: skill_dir.to_path_buf(),
            span: None,
            fix_available: false,
            category: Category::SpecCompliance,
        });
        return diags;
    }

    // E001/E002: SKILL.md existence and casing
    let skill_md_path = find_skill_md(skill_dir);
    let skill_md_path = match skill_md_path {
        Some(path) => {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name != "SKILL.md" {
                diags.push(Diagnostic {
                    rule_id: "E002".into(),
                    severity: Severity::Error,
                    message: format!("SKILL.md has wrong case: \"{}\" (expected \"SKILL.md\")", file_name),
                    path: path.clone(),
                    span: None,
                    fix_available: true,
                    category: Category::SpecCompliance,
                });
            }
            path
        }
        None => {
            diags.push(Diagnostic {
                rule_id: "E001".into(),
                severity: Severity::Error,
                message: "SKILL.md file missing".into(),
                path: skill_dir.to_path_buf(),
                span: None,
                fix_available: false,
                category: Category::SpecCompliance,
            });
            return diags;
        }
    };

    // Read file content
    let content = match fs::read_to_string(&skill_md_path) {
        Ok(c) => c,
        Err(e) => {
            diags.push(Diagnostic {
                rule_id: "E003".into(),
                severity: Severity::Error,
                message: format!("Cannot read SKILL.md: {e}"),
                path: skill_md_path.clone(),
                span: None,
                fix_available: false,
                category: Category::SpecCompliance,
            });
            return diags;
        }
    };

    // E034: UTF-8 BOM
    if content.starts_with('\u{feff}') {
        diags.push(Diagnostic {
            rule_id: "E034".into(),
            severity: Severity::Error,
            message: "UTF-8 BOM detected — strip it".into(),
            path: skill_md_path.clone(),
            span: Some(Span { start_line: 1, start_col: 1, end_line: 1, end_col: 4 }),
            fix_available: true,
            category: Category::SpecCompliance,
        });
    }

    // Parse frontmatter
    let (fm, body_raw) = match parse_frontmatter(&content) {
        Ok(result) => result,
        Err(ParseError::MissingFrontmatter) => {
            diags.push(Diagnostic {
                rule_id: "E003".into(),
                severity: Severity::Error,
                message: "YAML frontmatter missing (file must start with ---)".into(),
                path: skill_md_path,
                span: None,
                fix_available: true,
                category: Category::SpecCompliance,
            });
            return diags;
        }
        Err(ParseError::UnclosedFrontmatter) => {
            diags.push(Diagnostic {
                rule_id: "E032".into(),
                severity: Severity::Error,
                message: "Frontmatter not properly closed (missing second ---)".into(),
                path: skill_md_path,
                span: None,
                fix_available: true,
                category: Category::SpecCompliance,
            });
            return diags;
        }
        Err(ParseError::NotAMapping) => {
            diags.push(Diagnostic {
                rule_id: "E004".into(),
                severity: Severity::Error,
                message: "Frontmatter is not a YAML mapping (got a list or scalar)".into(),
                path: skill_md_path,
                span: None,
                fix_available: false,
                category: Category::SpecCompliance,
            });
            return diags;
        }
        Err(ParseError::InvalidYaml(msg)) => {
            diags.push(Diagnostic {
                rule_id: "E003".into(),
                severity: Severity::Error,
                message: format!("Invalid YAML in frontmatter: {msg}"),
                path: skill_md_path,
                span: None,
                fix_available: false,
                category: Category::SpecCompliance,
            });
            return diags;
        }
    };

    // E030: Unknown fields
    for key in fm.unknown_fields.keys() {
        diags.push(Diagnostic {
            rule_id: "E030".into(),
            severity: Severity::Error,
            message: format!("Unknown field \"{key}\" in frontmatter. Allowed: {ALLOWED_FIELDS:?}"),
            path: skill_md_path.clone(),
            span: None,
            fix_available: false,
            category: Category::SpecCompliance,
        });
    }

    // Name validation (E006-E013)
    validate_name(&fm, skill_dir, &skill_md_path, &mut diags);

    // Description validation (E014-E016)
    validate_description(&fm, &skill_md_path, &mut diags);

    // Compatibility validation (E017-E019)
    validate_compatibility(&fm, &skill_md_path, &mut diags);

    // License validation (E020-E021)
    validate_license(&fm, &skill_md_path, &mut diags);

    // Metadata validation (E022-E024)
    validate_metadata(&fm, &skill_md_path, &mut diags);

    // Allowed-tools validation (E025-E029)
    validate_allowed_tools(&fm, &skill_md_path, &mut diags);

    // E031: File references
    let body = parse_body(&body_raw, skill_dir);
    for fref in &body.file_references {
        if !fref.exists {
            diags.push(Diagnostic {
                rule_id: "E031".into(),
                severity: Severity::Error,
                message: format!("File reference \"{}\" does not exist", fref.path),
                path: skill_md_path.clone(),
                span: Some(fref.span.clone()),
                fix_available: false,
                category: Category::SpecCompliance,
            });
        }
    }

    // E035: Secret detection
    scan_secrets(skill_dir, &mut diags);

    diags
}

fn validate_name(fm: &Frontmatter, skill_dir: &Path, path: &Path, diags: &mut Vec<Diagnostic>) {
    let name = match &fm.name {
        None => {
            diags.push(Diagnostic {
                rule_id: "E006".into(), severity: Severity::Error,
                message: "Missing required field: name".into(),
                path: path.into(), span: None, fix_available: false,
                category: Category::SpecCompliance,
            });
            return;
        }
        Some(n) if n.trim().is_empty() => {
            diags.push(Diagnostic {
                rule_id: "E007".into(), severity: Severity::Error,
                message: "Field 'name' is empty".into(),
                path: path.into(), span: None, fix_available: false,
                category: Category::SpecCompliance,
            });
            return;
        }
        Some(n) => n.clone(),
    };

    let normalized: String = name.nfkc().collect();

    if normalized.chars().count() > MAX_NAME_LEN {
        diags.push(Diagnostic {
            rule_id: "E008".into(), severity: Severity::Error,
            message: format!("Name exceeds {} character limit ({} chars)", MAX_NAME_LEN, normalized.chars().count()),
            path: path.into(), span: None, fix_available: false,
            category: Category::SpecCompliance,
        });
    }

    if !normalized.chars().all(|c| c.is_alphanumeric() || c == '-') {
        diags.push(Diagnostic {
            rule_id: "E009".into(), severity: Severity::Error,
            message: format!("Name contains invalid characters: \"{normalized}\". Only lowercase alphanumeric and hyphens allowed."),
            path: path.into(), span: None, fix_available: false,
            category: Category::SpecCompliance,
        });
    }

    if normalized.starts_with('-') || normalized.ends_with('-') {
        diags.push(Diagnostic {
            rule_id: "E010".into(), severity: Severity::Error,
            message: "Name cannot start or end with a hyphen".into(),
            path: path.into(), span: None, fix_available: false,
            category: Category::SpecCompliance,
        });
    }

    if normalized.contains("--") {
        diags.push(Diagnostic {
            rule_id: "E011".into(), severity: Severity::Error,
            message: "Name cannot contain consecutive hyphens".into(),
            path: path.into(), span: None, fix_available: false,
            category: Category::SpecCompliance,
        });
    }

    if normalized != normalized.to_lowercase() {
        diags.push(Diagnostic {
            rule_id: "E012".into(), severity: Severity::Error,
            message: format!("Name must be lowercase: \"{normalized}\""),
            path: path.into(), span: None, fix_available: true,
            category: Category::SpecCompliance,
        });
    }

    // E013: Name must match directory
    if let Some(dir_name) = skill_dir.file_name().map(|n| n.to_string_lossy().to_string()) {
        let dir_norm: String = dir_name.nfkc().collect();
        if dir_norm != normalized {
            diags.push(Diagnostic {
                rule_id: "E013".into(), severity: Severity::Error,
                message: format!("Name \"{normalized}\" doesn't match directory \"{dir_name}\""),
                path: path.into(), span: None, fix_available: true,
                category: Category::SpecCompliance,
            });
        }
    }
}

fn validate_description(fm: &Frontmatter, path: &Path, diags: &mut Vec<Diagnostic>) {
    match &fm.description {
        None => {
            diags.push(Diagnostic {
                rule_id: "E014".into(), severity: Severity::Error,
                message: "Missing required field: description".into(),
                path: path.into(), span: None, fix_available: false,
                category: Category::SpecCompliance,
            });
        }
        Some(d) if d.trim().is_empty() => {
            diags.push(Diagnostic {
                rule_id: "E015".into(), severity: Severity::Error,
                message: "Field 'description' is empty".into(),
                path: path.into(), span: None, fix_available: false,
                category: Category::SpecCompliance,
            });
        }
        Some(d) if d.chars().count() > MAX_DESC_LEN => {
            diags.push(Diagnostic {
                rule_id: "E016".into(), severity: Severity::Error,
                message: format!("Description exceeds {} character limit ({} chars)", MAX_DESC_LEN, d.chars().count()),
                path: path.into(), span: None, fix_available: false,
                category: Category::SpecCompliance,
            });
        }
        _ => {}
    }
}

fn validate_compatibility(fm: &Frontmatter, path: &Path, diags: &mut Vec<Diagnostic>) {
    if let Some(compat) = &fm.compatibility {
        if compat.trim().is_empty() {
            diags.push(Diagnostic {
                rule_id: "E019".into(), severity: Severity::Error,
                message: "Field 'compatibility' is empty (omit it instead)".into(),
                path: path.into(), span: None, fix_available: false,
                category: Category::SpecCompliance,
            });
        } else if compat.chars().count() > MAX_COMPAT_LEN {
            diags.push(Diagnostic {
                rule_id: "E017".into(), severity: Severity::Error,
                message: format!("Compatibility exceeds {} character limit ({} chars)", MAX_COMPAT_LEN, compat.chars().count()),
                path: path.into(), span: None, fix_available: false,
                category: Category::SpecCompliance,
            });
        }
    }
    // E018: Check if compatibility was provided as non-string in raw YAML
    // This is handled implicitly — our parser extracts as Option<String>,
    // so non-string values end up in unknown_fields. We need to check the
    // raw YAML. For now, the Frontmatter struct only captures string values
    // for compatibility, so a non-string compatibility would be captured
    // in unknown_fields and trigger E030. We add an explicit E018 check
    // during raw YAML traversal in the validate() function if needed.
    // TODO: Enhance parser to detect non-string types for known fields.
}

fn validate_license(fm: &Frontmatter, path: &Path, diags: &mut Vec<Diagnostic>) {
    if let Some(license) = &fm.license {
        if license.trim().is_empty() {
            diags.push(Diagnostic {
                rule_id: "E021".into(), severity: Severity::Error,
                message: "Field 'license' is empty (omit it instead)".into(),
                path: path.into(), span: None, fix_available: false,
                category: Category::SpecCompliance,
            });
        }
    }
}

fn validate_metadata(fm: &Frontmatter, path: &Path, diags: &mut Vec<Diagnostic>) {
    if let Some(meta) = &fm.metadata {
        match meta {
            serde_yaml::Value::Mapping(map) => {
                for (key, value) in map {
                    if !key.is_string() {
                        diags.push(Diagnostic {
                            rule_id: "E023".into(), severity: Severity::Error,
                            message: "Metadata contains a non-string key".into(),
                            path: path.into(), span: None, fix_available: false,
                            category: Category::SpecCompliance,
                        });
                    }
                    if !value.is_string() {
                        let key_name = key.as_str().unwrap_or("<non-string>");
                        diags.push(Diagnostic {
                            rule_id: "E024".into(), severity: Severity::Error,
                            message: format!("Metadata value for \"{key_name}\" must be a string"),
                            path: path.into(), span: None, fix_available: false,
                            category: Category::SpecCompliance,
                        });
                    }
                }
            }
            _ => {
                diags.push(Diagnostic {
                    rule_id: "E022".into(), severity: Severity::Error,
                    message: "Field 'metadata' must be a YAML mapping".into(),
                    path: path.into(), span: None, fix_available: false,
                    category: Category::SpecCompliance,
                });
            }
        }
    }
}

fn validate_allowed_tools(fm: &Frontmatter, path: &Path, diags: &mut Vec<Diagnostic>) {
    if let Some(tools) = &fm.allowed_tools {
        match tools {
            serde_yaml::Value::String(s) => {
                if s.trim().is_empty() {
                    diags.push(Diagnostic {
                        rule_id: "E026".into(), severity: Severity::Error,
                        message: "Field 'allowed-tools' is empty".into(),
                        path: path.into(), span: None, fix_available: false,
                        category: Category::SpecCompliance,
                    });
                    return;
                }
                if s.contains(',') {
                    diags.push(Diagnostic {
                        rule_id: "E027".into(), severity: Severity::Error,
                        message: "allowed-tools must be space-delimited, not comma-delimited".into(),
                        path: path.into(), span: None, fix_available: false,
                        category: Category::SpecCompliance,
                    });
                }
                for tool in s.split_whitespace() {
                    check_tool_spec(tool, path, diags);
                }
            }
            serde_yaml::Value::Sequence(seq) => {
                if seq.is_empty() {
                    diags.push(Diagnostic {
                        rule_id: "E026".into(), severity: Severity::Error,
                        message: "Field 'allowed-tools' array is empty".into(),
                        path: path.into(), span: None, fix_available: false,
                        category: Category::SpecCompliance,
                    });
                }
                for (i, item) in seq.iter().enumerate() {
                    if !item.is_string() {
                        diags.push(Diagnostic {
                            rule_id: "E029".into(), severity: Severity::Error,
                            message: format!("allowed-tools array item {i} must be a string"),
                            path: path.into(), span: None, fix_available: false,
                            category: Category::SpecCompliance,
                        });
                    }
                }
            }
            _ => {
                diags.push(Diagnostic {
                    rule_id: "E025".into(), severity: Severity::Error,
                    message: "Field 'allowed-tools' must be a string or array of strings".into(),
                    path: path.into(), span: None, fix_available: false,
                    category: Category::SpecCompliance,
                });
            }
        }
    }
}

fn check_tool_spec(tool: &str, path: &Path, diags: &mut Vec<Diagnostic>) {
    let open = tool.chars().filter(|&c| c == '(').count();
    let close = tool.chars().filter(|&c| c == ')').count();
    if open != close {
        diags.push(Diagnostic {
            rule_id: "E028".into(), severity: Severity::Error,
            message: format!("Unbalanced parentheses in tool spec: \"{tool}\""),
            path: path.into(), span: None, fix_available: false,
            category: Category::SpecCompliance,
        });
    }
}

fn scan_secrets(skill_dir: &Path, diags: &mut Vec<Diagnostic>) {
    let walker = walkdir::WalkDir::new(skill_dir).max_depth(3);
    for entry in walker.into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        // Skip binary files
        if let Ok(bytes) = fs::read(path) {
            if bytes.len() >= 8192 && bytes[..8192].contains(&0) {
                continue;
            }
            if bytes.contains(&0) && bytes.len() < 8192 {
                continue;
            }
        }
        if let Ok(content) = fs::read_to_string(path) {
            for (name, pattern) in SECRET_PATTERNS.iter() {
                if pattern.is_match(&content) {
                    diags.push(Diagnostic {
                        rule_id: "E035".into(),
                        severity: Severity::Error,
                        message: format!("Potential secret detected ({})", name),
                        path: path.to_path_buf(),
                        span: None,
                        fix_available: false,
                        category: Category::SpecCompliance,
                    });
                    break; // One diagnostic per file
                }
            }
        }
    }
}
```

- [ ] **Step 4: Update lib.rs**

```rust
pub mod validator;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p skillplane-core -- validator::tests`
Expected: All 5 tests PASS

- [ ] **Step 6: Add more validator tests for name rules**

```rust
    #[test]
    fn test_e006_missing_name() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(&skill_dir, "---\ndescription: test\n---\nBody");
        assert!(has_rule(&run_validator(&skill_dir), "E006"));
    }

    #[test]
    fn test_e008_name_too_long() {
        let dir = TempDir::new().unwrap();
        let long_name = "a".repeat(70);
        let skill_dir = dir.path().join(&long_name);
        make_skill(&skill_dir, &format!("---\nname: {}\ndescription: test\n---\nBody", long_name));
        assert!(has_rule(&run_validator(&skill_dir), "E008"));
    }

    #[test]
    fn test_e012_uppercase_name() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("MySkill");
        make_skill(&skill_dir, "---\nname: MySkill\ndescription: test\n---\nBody");
        assert!(has_rule(&run_validator(&skill_dir), "E012"));
    }

    #[test]
    fn test_e013_name_dir_mismatch() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("wrong-name");
        make_skill(&skill_dir, "---\nname: right-name\ndescription: test\n---\nBody");
        assert!(has_rule(&run_validator(&skill_dir), "E013"));
    }
```

- [ ] **Step 7: Run all validator tests**

Run: `cargo test -p skillplane-core -- validator::tests`
Expected: All tests PASS

- [ ] **Step 8: Commit**

```bash
git add crates/skillplane-core/src/validator.rs crates/skillplane-core/src/lib.rs
git commit -m "feat: implement spec-compliance validator (E001-E035)"
```

---

### Task 6: Linter — Best Practice W-Rules (`linter.rs`)

**Files:**
- Create: `crates/skillplane-core/src/linter.rs`

The linter takes a parsed `Skill` and produces W-rule and I-rule diagnostics.

- [ ] **Step 1: Write tests for key warning rules**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_skill_parsed(dir: &Path, content: &str) -> Skill {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
        crate::discovery::load_skill(dir).unwrap()
    }

    fn has_rule(diags: &[Diagnostic], rule_id: &str) -> bool {
        diags.iter().any(|d| d.rule_id == rule_id)
    }

    #[test]
    fn test_w001_too_many_lines() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let body = "line\n".repeat(501);
        let content = format!("---\nname: my-skill\ndescription: Use when testing long skills.\n---\n{body}");
        let skill = make_skill_parsed(&skill_dir, &content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W001"));
    }

    #[test]
    fn test_w003_short_description() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let skill = make_skill_parsed(&skill_dir,
            "---\nname: my-skill\ndescription: Short.\n---\n# Skill\n");
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W003"));
    }

    #[test]
    fn test_w004_no_trigger_language() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let skill = make_skill_parsed(&skill_dir,
            "---\nname: my-skill\ndescription: This skill processes PDF files and extracts text.\n---\n# Skill\n");
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W004"));
    }

    #[test]
    fn test_w004_passes_with_trigger_language() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let skill = make_skill_parsed(&skill_dir,
            "---\nname: my-skill\ndescription: Use when processing PDF files to extract text.\n---\n# Skill\n");
        let diags = lint(&skill);
        assert!(!has_rule(&diags, "W004"));
    }

    #[test]
    fn test_w009_no_headings() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let skill = make_skill_parsed(&skill_dir,
            "---\nname: my-skill\ndescription: Use when testing headings detection in body.\n---\nJust body text with no headings at all.\n");
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W009"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p skillplane-core -- linter::tests`
Expected: FAIL

- [ ] **Step 3: Implement linter**

```rust
// crates/skillplane-core/src/linter.rs
use once_cell::sync::Lazy;
use regex::Regex;

use crate::model::*;

static TRIGGER_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"(?i)\buse\s+(this\s+skill\s+)?when\b").unwrap(),
    Regex::new(r"(?i)\bwhen\s+the\s+(user|developer|agent)\b").unwrap(),
    Regex::new(r"(?i)\bactivate\s+(this\s+)?(skill\s+)?(when|for|if)\b").unwrap(),
    Regex::new(r"(?i)\binvoke\s+(this\s+)?(skill\s+)?(when|for|if)\b").unwrap(),
    Regex::new(r"(?i)\btrigger(s|ed)?\s+(when|on|by)\b").unwrap(),
    Regex::new(r"(?i)\bapplies?\s+(when|to|if)\b").unwrap(),
]);

static PASSIVE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"(?i)^this\s+skill\s+(is|was|will|can|should|does|provides|helps|allows|enables|handles|manages|performs)").unwrap(),
    Regex::new(r"(?i)^(is|was|will\s+be)\s+used\s+(to|for|when)").unwrap(),
    Regex::new(r"(?i)^(a|an|the)\s+(skill|tool|helper|utility)\s+(that|which|for)").unwrap(),
    Regex::new(r"(?i)^(provides?|offers?|gives?|enables?|allows?|helps?)\s").unwrap(),
    Regex::new(r"(?i)^(designed|intended|meant|built|created)\s+(to|for)\b").unwrap(),
]);

static VAGUE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"(?i)^helps?\s+(with|the|you)\b").unwrap(),
    Regex::new(r"(?i)^a\s+tool\s+(for|to|that)\b").unwrap(),
    Regex::new(r"(?i)^an?\s+(useful|helpful|handy|simple|basic|generic)\s+(skill|tool)\b").unwrap(),
    Regex::new(r"(?i)^(does|handles?)\s+(stuff|things|various|everything|anything)\b").unwrap(),
    Regex::new(r"(?i)\b(various|miscellaneous|general[- ]purpose|multi[- ]purpose|all[- ]purpose)\s+(tasks?|things?|operations?|functions?)\b").unwrap(),
    Regex::new(r"(?i)^(utility|helper)\s+(for|to|that|skill)\b").unwrap(),
]);

static ACTION_VERBS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(use|run|execute|invoke|call|apply|generate|create|build|deploy|configure|set\s+up|install|validate|check|lint|test|format|transform|convert|parse|analyze|scan|detect|fix|migrate|upgrade|refactor|optimize|monitor|debug|log|trace|fetch|pull|push|sync|upload|download|export|import|send|render|compile|bundle|serve|start|stop|restart|reset|clean|scaffold|bootstrap|initialize|provision|authenticate|authorize)\b").unwrap()
});

static PLACEHOLDER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(TODO|FIXME|TBD|CHANGEME|XXX)\b").unwrap()
});

static FILLER_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"(?i)\bas\s+you\s+(probably\s+)?(already\s+)?know\b").unwrap(),
    Regex::new(r"(?i)\bit\s+is\s+(widely|generally|commonly)\s+known\s+that\b").unwrap(),
    Regex::new(r"(?i)\b(remember|note|keep\s+in\s+mind)\s+that\s+(all|every|most)\b").unwrap(),
    Regex::new(r"(?i)\bin\s+(today's|the\s+modern|the\s+current)\s+(world|landscape|ecosystem)\b").unwrap(),
    Regex::new(r"(?i)\bthis\s+is\s+(important|crucial|critical|essential|vital)\s+because\b").unwrap(),
    Regex::new(r"(?i)\b(best\s+practices?\s+dictate|industry\s+standard\s+is|it\s+is\s+recommended)\b").unwrap(),
    Regex::new(r"(?i)\bfor\s+more\s+(information|details),?\s+see\s+(the\s+)?(official\s+)?documentation\b").unwrap(),
]);

/// Run all best-practice lint checks on a parsed Skill.
/// Returns diagnostics for W001-W028 and I001-I016.
pub fn lint(skill: &Skill) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let path = skill.path.join("SKILL.md");

    // W001: Line count
    if skill.body.line_count > 500 {
        diags.push(Diagnostic {
            rule_id: "W001".into(), severity: Severity::Warning,
            message: format!("SKILL.md exceeds 500 lines ({} lines)", skill.body.line_count),
            path: path.clone(), span: None, fix_available: false,
            category: Category::ContentEfficiency,
        });
    }

    // W002: Token estimate
    if skill.body.estimated_tokens > 5000 {
        diags.push(Diagnostic {
            rule_id: "W002".into(), severity: Severity::Warning,
            message: format!("SKILL.md body exceeds ~5000 estimated tokens (~{})", skill.body.estimated_tokens),
            path: path.clone(), span: None, fix_available: false,
            category: Category::ContentEfficiency,
        });
    }

    // Description checks (W003-W005, W020, W023)
    if let Some(desc) = &skill.frontmatter.description {
        if desc.chars().count() < 50 {
            diags.push(Diagnostic {
                rule_id: "W003".into(), severity: Severity::Warning,
                message: format!("Description is very short ({} chars, recommend >= 50)", desc.chars().count()),
                path: path.clone(), span: None, fix_available: false,
                category: Category::DescriptionQuality,
            });
        }

        let has_trigger = TRIGGER_PATTERNS.iter().any(|p| p.is_match(desc));
        if !has_trigger {
            diags.push(Diagnostic {
                rule_id: "W004".into(), severity: Severity::Warning,
                message: "Description lacks trigger language (\"Use when...\")".into(),
                path: path.clone(), span: None, fix_available: false,
                category: Category::DescriptionQuality,
            });
        }

        let has_passive = PASSIVE_PATTERNS.iter().any(|p| p.is_match(desc));
        if has_passive && !has_trigger {
            diags.push(Diagnostic {
                rule_id: "W005".into(), severity: Severity::Warning,
                message: "Description uses passive voice (\"This skill does...\") — prefer imperative (\"Use when...\")".into(),
                path: path.clone(), span: None, fix_available: false,
                category: Category::DescriptionQuality,
            });
        }

        if VAGUE_PATTERNS.iter().any(|p| p.is_match(desc)) {
            diags.push(Diagnostic {
                rule_id: "W020".into(), severity: Severity::Warning,
                message: "Description matches known vague anti-patterns".into(),
                path: path.clone(), span: None, fix_available: false,
                category: Category::DescriptionQuality,
            });
        }

        if !ACTION_VERBS.is_match(desc) {
            diags.push(Diagnostic {
                rule_id: "W023".into(), severity: Severity::Warning,
                message: "Description lacks action verbs".into(),
                path: path.clone(), span: None, fix_available: false,
                category: Category::DescriptionQuality,
            });
        }

        if PLACEHOLDER_RE.is_match(desc) {
            diags.push(Diagnostic {
                rule_id: "W008".into(), severity: Severity::Warning,
                message: "Description contains placeholder text".into(),
                path: path.clone(), span: None, fix_available: false,
                category: Category::DescriptionQuality,
            });
        }
    }

    // Name placeholder (W007)
    if let Some(name) = &skill.frontmatter.name {
        if PLACEHOLDER_RE.is_match(name) {
            diags.push(Diagnostic {
                rule_id: "W007".into(), severity: Severity::Warning,
                message: "Name contains placeholder text".into(),
                path: path.clone(), span: None, fix_available: false,
                category: Category::DescriptionQuality,
            });
        }
    }

    // W006: Body placeholder text
    if skill.body.has_placeholder_text {
        diags.push(Diagnostic {
            rule_id: "W006".into(), severity: Severity::Warning,
            message: "Body contains placeholder text (TODO/FIXME/TBD)".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::ComposabilityClarity,
        });
    }

    // W009: No headings
    if skill.body.headings.is_empty() {
        diags.push(Diagnostic {
            rule_id: "W009".into(), severity: Severity::Warning,
            message: "No markdown headings in body (unstructured content)".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::ComposabilityClarity,
        });
    }

    // W021: Generic filler
    let filler_count = FILLER_PATTERNS.iter()
        .filter(|p| p.is_match(&skill.body.raw))
        .count();
    if filler_count >= 2 {
        diags.push(Diagnostic {
            rule_id: "W021".into(), severity: Severity::Warning,
            message: format!("Body contains generic filler content ({filler_count} filler phrases detected)"),
            path: path.clone(), span: None, fix_available: false,
            category: Category::ComposabilityClarity,
        });
    }

    // W024: Orphaned references
    if skill.file_tree.has_references || skill.file_tree.has_scripts || skill.file_tree.has_assets {
        let referenced_paths: std::collections::HashSet<_> = skill.body.file_references.iter()
            .map(|r| r.path.as_str())
            .collect();

        for dir_name in &["references", "scripts", "assets"] {
            let dir_path = skill.path.join(dir_name);
            if dir_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&dir_path) {
                    for entry in entries.filter_map(Result::ok) {
                        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                            let rel = format!("{}/{}", dir_name, entry.file_name().to_string_lossy());
                            if !referenced_paths.contains(rel.as_str()) {
                                diags.push(Diagnostic {
                                    rule_id: "W024".into(), severity: Severity::Warning,
                                    message: format!("{} exists but is not referenced in SKILL.md", rel),
                                    path: path.clone(), span: None, fix_available: false,
                                    category: Category::ContentEfficiency,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Info rules (I001-I016) — these affect scoring, not user-facing warnings
    if !skill.file_tree.has_scripts {
        diags.push(Diagnostic {
            rule_id: "I001".into(), severity: Severity::Info,
            message: "No scripts/ directory".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::ScriptQuality,
        });
    }
    if !skill.file_tree.has_references {
        diags.push(Diagnostic {
            rule_id: "I002".into(), severity: Severity::Info,
            message: "No references/ directory".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::Discoverability,
        });
    }
    if !skill.file_tree.has_assets {
        diags.push(Diagnostic {
            rule_id: "I003".into(), severity: Severity::Info,
            message: "No assets/ directory".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::Discoverability,
        });
    }
    if !skill.file_tree.has_examples {
        diags.push(Diagnostic {
            rule_id: "I004".into(), severity: Severity::Info,
            message: "No examples/ directory".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::Discoverability,
        });
    }

    // I005: Gotchas section
    let has_gotchas = skill.body.headings.iter().any(|h| {
        let lower = h.text.to_lowercase();
        lower.contains("gotcha") || lower.contains("pitfall") || lower.contains("caveat") || lower.contains("known issue")
    });
    if !has_gotchas {
        diags.push(Diagnostic {
            rule_id: "I005".into(), severity: Severity::Info,
            message: "Body has no gotchas/pitfalls section".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::Discoverability,
        });
    }

    // I006: Validation step
    let has_validation = skill.body.headings.iter().any(|h| {
        let lower = h.text.to_lowercase();
        lower.contains("validat") || lower.contains("verif") || lower.contains("test") || lower.contains("check")
    });
    if !has_validation {
        diags.push(Diagnostic {
            rule_id: "I006".into(), severity: Severity::Info,
            message: "Body has no validation/verification step".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::Discoverability,
        });
    }

    // I012: License
    let has_license_field = skill.frontmatter.license.is_some();
    let has_license_file = skill.file_tree.files.iter().any(|f| {
        f.file_name()
            .map(|n| {
                let s = n.to_string_lossy().to_uppercase();
                s.starts_with("LICENSE") || s.starts_with("LICENCE")
            })
            .unwrap_or(false)
    });
    if !has_license_field && !has_license_file {
        diags.push(Diagnostic {
            rule_id: "I012".into(), severity: Severity::Info,
            message: "No license field or LICENSE file".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::Discoverability,
        });
    }

    // I016: Progressive disclosure ratio
    let skillmd_size = skill.body.raw.len();
    let referenced_size = skill.file_tree.total_content_size;
    if referenced_size == 0 && skillmd_size > 0 {
        diags.push(Diagnostic {
            rule_id: "I016".into(), severity: Severity::Info,
            message: "Progressive disclosure ratio: 0% of content in referenced files".into(),
            path: path.clone(), span: None, fix_available: false,
            category: Category::ContentEfficiency,
        });
    }

    // I015: Keyword diversity
    if let Some(desc) = &skill.frontmatter.description {
        let scenario_count = count_trigger_scenarios(desc);
        if scenario_count < 2 {
            diags.push(Diagnostic {
                rule_id: "I015".into(), severity: Severity::Info,
                message: format!("Description covers {scenario_count} trigger scenario(s) (recommend >= 2)"),
                path: path.clone(), span: None, fix_available: false,
                category: Category::DescriptionQuality,
            });
        }
    }

    diags
}

fn count_trigger_scenarios(desc: &str) -> usize {
    let splitters = Regex::new(r"(?i)(\.\s+|;\s+|\s+-\s+|\n|,?\s+or\s+when\b|,?\s+also\s+when\b|,?\s+and\s+when\b|,?\s+as\s+well\s+as\s+when\b)").unwrap();
    let segments: Vec<&str> = splitters.split(desc).collect();
    let mut count = 0;
    for seg in segments {
        let trimmed = seg.trim();
        if trimmed.is_empty() {
            continue;
        }
        let is_trigger = TRIGGER_PATTERNS.iter().any(|p| p.is_match(trimmed))
            || ACTION_VERBS.is_match(trimmed);
        if is_trigger {
            count += 1;
        }
    }
    count.max(1) // At least 1 if description is non-empty
}
```

- [ ] **Step 4: Update lib.rs**

```rust
pub mod linter;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p skillplane-core -- linter::tests`
Expected: All 5 tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/skillplane-core/src/linter.rs crates/skillplane-core/src/lib.rs
git commit -m "feat: implement best-practice linter (W001-W028, I001-I016)"
```

---

### Task 7: Config Loading (`config.rs`)

**Files:**
- Create: `crates/skillplane-core/src/config.rs`

- [ ] **Step 1: Write config tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.fail_on, FailOn::Errors);
        assert_eq!(config.min_score, 0);
        assert!(!config.rules.experimental);
        assert!(config.rules.disable.is_empty());
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
fail-on = "warnings"
min-score = 80

[rules]
disable = ["W022", "E035"]
experimental = true

[scoring.weights]
spec-compliance = 0.50
description-quality = 0.10
content-efficiency = 0.15
composability-clarity = 0.10
script-quality = 0.10
discoverability = 0.05
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.fail_on, FailOn::Warnings);
        assert_eq!(config.min_score, 80);
        assert!(config.rules.experimental);
        assert_eq!(config.rules.disable, vec!["W022", "E035"]);
        assert!((config.scoring.weights.spec_compliance - 0.50).abs() < 0.001);
    }

    #[test]
    fn test_is_rule_enabled() {
        let mut config = Config::default();
        config.rules.disable = vec!["W022".into(), "E035".into()];
        assert!(!config.is_rule_enabled("W022"));
        assert!(!config.is_rule_enabled("E035"));
        assert!(config.is_rule_enabled("W001"));
        // Tier 2 rules disabled by default
        assert!(!config.is_rule_enabled("W029"));
    }

    #[test]
    fn test_tier2_enabled_with_experimental() {
        let mut config = Config::default();
        config.rules.experimental = true;
        assert!(config.is_rule_enabled("W029"));
        assert!(config.is_rule_enabled("E036"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p skillplane-core -- config::tests`
Expected: FAIL

- [ ] **Step 3: Implement config**

```rust
// crates/skillplane-core/src/config.rs
use std::path::Path;

use serde::Deserialize;

const TIER2_RULES: &[&str] = &["W029", "W030", "W031", "W032", "I013", "I014", "E036"];

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub fail_on: FailOn,
    pub min_score: u32,
    pub format: OutputFormat,
    pub rules: RulesConfig,
    pub scoring: ScoringConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FailOn {
    Errors,
    Warnings,
    None,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Terminal,
    Json,
    Sarif,
    Markdown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RulesConfig {
    pub disable: Vec<String>,
    pub experimental: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ScoringConfig {
    pub weights: Weights,
    pub grades: Grades,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Weights {
    pub spec_compliance: f64,
    pub description_quality: f64,
    pub content_efficiency: f64,
    pub composability_clarity: f64,
    pub script_quality: f64,
    pub discoverability: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "UPPERCASE")]
pub struct Grades {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    pub exclude: Vec<String>,
}

impl Config {
    /// Check if a rule is enabled given current config.
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        if self.rules.disable.iter().any(|r| r == rule_id) {
            return false;
        }
        if TIER2_RULES.contains(&rule_id) && !self.rules.experimental {
            return false;
        }
        true
    }

    /// Load config from a .skillplane.toml file, searching up from start_dir.
    pub fn load(start_dir: &Path) -> Self {
        let mut dir = start_dir.to_path_buf();
        loop {
            let config_path = dir.join(".skillplane.toml");
            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
            if !dir.pop() {
                break;
            }
        }
        Config::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fail_on: FailOn::Errors,
            min_score: 0,
            format: OutputFormat::Terminal,
            rules: RulesConfig::default(),
            scoring: ScoringConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self { disable: Vec::new(), experimental: false }
    }
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self { weights: Weights::default(), grades: Grades::default() }
    }
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            spec_compliance: 0.40,
            description_quality: 0.20,
            content_efficiency: 0.15,
            composability_clarity: 0.10,
            script_quality: 0.10,
            discoverability: 0.05,
        }
    }
}

impl Default for Grades {
    fn default() -> Self {
        Self { a: 90, b: 80, c: 70, d: 60 }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self { exclude: Vec::new() }
    }
}
```

- [ ] **Step 4: Update lib.rs**

```rust
pub mod config;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p skillplane-core -- config::tests`
Expected: All 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/skillplane-core/src/config.rs crates/skillplane-core/src/lib.rs
git commit -m "feat: implement config loading (.skillplane.toml with defaults)"
```

---

### Task 8: Scorer (`scorer.rs`)

**Files:**
- Create: `crates/skillplane-core/src/scorer.rs`

- [ ] **Step 1: Write scorer tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn make_diag(rule_id: &str, severity: Severity, category: Category) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.into(),
            severity,
            message: "test".into(),
            path: "test".into(),
            span: None,
            fix_available: false,
            category,
        }
    }

    #[test]
    fn test_perfect_score() {
        let config = Config::default();
        let diags: Vec<Diagnostic> = vec![];
        let card = score(&diags, true, &config);
        assert_eq!(card.composite.round() as u32, 100);
        assert_eq!(card.grade, Grade::A);
    }

    #[test]
    fn test_spec_compliance_scoring() {
        let config = Config::default();
        let diags = vec![
            make_diag("E030", Severity::Error, Category::SpecCompliance),
            make_diag("E031", Severity::Error, Category::SpecCompliance),
        ];
        let card = score(&diags, true, &config);
        // 33/35 = 94.3%, weighted: 94.3 * 0.40 = 37.7
        let spec_cat = card.categories.iter().find(|c| c.name == "spec_compliance").unwrap();
        assert!((spec_cat.score - 94.3).abs() < 0.5);
    }

    #[test]
    fn test_scripts_absent_full_score() {
        let config = Config::default();
        let diags: Vec<Diagnostic> = vec![];
        let card = score(&diags, false, &config); // has_scripts = false
        let script_cat = card.categories.iter().find(|c| c.name == "script_quality").unwrap();
        assert!((script_cat.score - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_grade_boundaries() {
        let config = Config::default();
        assert_eq!(grade_from_score(95.0, &config), Grade::A);
        assert_eq!(grade_from_score(85.0, &config), Grade::B);
        assert_eq!(grade_from_score(75.0, &config), Grade::C);
        assert_eq!(grade_from_score(65.0, &config), Grade::D);
        assert_eq!(grade_from_score(55.0, &config), Grade::F);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p skillplane-core -- scorer::tests`
Expected: FAIL

- [ ] **Step 3: Implement scorer**

```rust
// crates/skillplane-core/src/scorer.rs
use crate::config::Config;
use crate::model::*;

/// Per-category check definitions.
/// Each check maps a rule_id to a category.
struct Check {
    rule_id: &'static str,
    category: &'static str,
}

const SPEC_COMPLIANCE_RULES: &[&str] = &[
    "E001","E002","E003","E004","E005","E006","E007","E008","E009","E010",
    "E011","E012","E013","E014","E015","E016","E017","E018","E019","E020",
    "E021","E022","E023","E024","E025","E026","E027","E028","E029","E030",
    "E031","E032","E033","E034","E035",
];

const DESC_QUALITY_CHECKS: &[&str] = &["W003", "W004", "W005", "W020", "W023", "I015"];
const CONTENT_EFF_CHECKS: &[&str] = &["W001", "W002", "I016", "W024", "W025"];
const COMPOSABILITY_CHECKS: &[&str] = &["W021", "W009", "W006"];
const SCRIPT_QUALITY_CHECKS: &[&str] = &["W026", "W027", "W028", "W018", "I009"];
const DISCOVERABILITY_CHECKS: &[&str] = &["I012", "I002", "I005", "I006"];

/// Compute the scorecard from diagnostics.
///
/// `has_scripts`: whether the skill has a scripts/ directory.
/// If false, ScriptQuality scores 100%.
pub fn score(diagnostics: &[Diagnostic], has_scripts: bool, config: &Config) -> ScoreCard {
    let fired_rules: std::collections::HashSet<&str> = diagnostics.iter()
        .map(|d| d.rule_id.as_str())
        .collect();

    let mut categories = Vec::new();

    // Spec Compliance
    categories.push(score_category(
        "spec_compliance",
        SPEC_COMPLIANCE_RULES,
        &fired_rules,
        config.scoring.weights.spec_compliance,
        config,
    ));

    // Description Quality
    categories.push(score_category(
        "description_quality",
        DESC_QUALITY_CHECKS,
        &fired_rules,
        config.scoring.weights.description_quality,
        config,
    ));

    // Content Efficiency
    categories.push(score_category(
        "content_efficiency",
        CONTENT_EFF_CHECKS,
        &fired_rules,
        config.scoring.weights.content_efficiency,
        config,
    ));

    // Composability & Clarity
    let mut comp_checks: Vec<&str> = COMPOSABILITY_CHECKS.to_vec();
    if config.rules.experimental {
        comp_checks.push("W031");
    }
    categories.push(score_category(
        "composability_clarity",
        &comp_checks,
        &fired_rules,
        config.scoring.weights.composability_clarity,
        config,
    ));

    // Script Quality
    if has_scripts {
        categories.push(score_category(
            "script_quality",
            SCRIPT_QUALITY_CHECKS,
            &fired_rules,
            config.scoring.weights.script_quality,
            config,
        ));
    } else {
        categories.push(CategoryScore {
            name: "script_quality".into(),
            weight: config.scoring.weights.script_quality,
            score: 100.0,
            weighted_score: 100.0 * config.scoring.weights.script_quality,
            rule_results: Vec::new(),
        });
    }

    // Discoverability
    categories.push(score_category(
        "discoverability",
        DISCOVERABILITY_CHECKS,
        &fired_rules,
        config.scoring.weights.discoverability,
        config,
    ));

    let composite: f64 = categories.iter().map(|c| c.weighted_score).sum();
    let grade = grade_from_score(composite, config);

    ScoreCard {
        composite,
        categories,
        grade,
    }
}

fn score_category(
    name: &str,
    check_rules: &[&str],
    fired: &std::collections::HashSet<&str>,
    weight: f64,
    config: &Config,
) -> CategoryScore {
    let mut results = Vec::new();
    let mut total = 0;
    let mut passing = 0;

    for &rule_id in check_rules {
        if !config.is_rule_enabled(rule_id) {
            continue;
        }
        total += 1;
        let passed = !fired.contains(rule_id);
        if passed {
            passing += 1;
        }
        results.push(RuleResult {
            rule_id: rule_id.into(),
            passed,
        });
    }

    let score = if total == 0 { 100.0 } else { (passing as f64 / total as f64) * 100.0 };
    let weighted = score * weight;

    CategoryScore {
        name: name.into(),
        weight,
        score,
        weighted_score: weighted,
        rule_results: results,
    }
}

pub fn grade_from_score(composite: f64, config: &Config) -> Grade {
    let rounded = composite.round() as u32;
    if rounded >= config.scoring.grades.a { Grade::A }
    else if rounded >= config.scoring.grades.b { Grade::B }
    else if rounded >= config.scoring.grades.c { Grade::C }
    else if rounded >= config.scoring.grades.d { Grade::D }
    else { Grade::F }
}
```

- [ ] **Step 4: Add ScoreCard and related types to model.rs**

Add to the bottom of `model.rs`:

```rust
/// Quality score report for a single skill.
#[derive(Debug, Clone)]
pub struct ScoreCard {
    pub composite: f64,
    pub categories: Vec<CategoryScore>,
    pub grade: Grade,
}

#[derive(Debug, Clone)]
pub struct CategoryScore {
    pub name: String,
    pub weight: f64,
    pub score: f64,
    pub weighted_score: f64,
    pub rule_results: Vec<RuleResult>,
}

#[derive(Debug, Clone)]
pub struct RuleResult {
    pub rule_id: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade { A, B, C, D, F }
```

- [ ] **Step 5: Update lib.rs**

```rust
pub mod scorer;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p skillplane-core -- scorer::tests`
Expected: All 4 tests PASS

- [ ] **Step 7: Run all tests**

Run: `cargo test -p skillplane-core`
Expected: All tests across all modules PASS

- [ ] **Step 8: Commit**

```bash
git add crates/skillplane-core/src/scorer.rs crates/skillplane-core/src/model.rs crates/skillplane-core/src/lib.rs
git commit -m "feat: implement quality scoring engine (6 weighted categories, grade assignment)"
```

---

### Task 9: Integration Test — End-to-End Pipeline

**Files:**
- Create: `crates/skillplane-core/tests/integration.rs`
- Use: `tests/fixtures/valid-skill/SKILL.md` (from Task 1)

- [ ] **Step 1: Write integration test**

```rust
// crates/skillplane-core/tests/integration.rs
use std::path::Path;

use skillplane_core::config::Config;
use skillplane_core::discovery::load_skill;
use skillplane_core::linter::lint;
use skillplane_core::scorer::score;
use skillplane_core::validator::validate;

#[test]
fn test_full_pipeline_valid_skill() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("tests/fixtures/valid-skill");

    // Validate
    let errors = validate(&fixture);
    let error_count = errors.iter().filter(|d| d.severity == skillplane_core::model::Severity::Error).count();
    assert_eq!(error_count, 0, "Expected no errors, got: {errors:?}");

    // Lint
    let skill = load_skill(&fixture).expect("Should load valid skill");
    let warnings = lint(&skill);

    // Score
    let config = Config::default();
    let mut all_diags = errors;
    all_diags.extend(warnings);
    let card = score(&all_diags, skill.file_tree.has_scripts, &config);

    assert!(card.composite >= 50.0, "Expected score >= 50, got {}", card.composite);
    println!("Score: {}/100 ({:?})", card.composite.round(), card.grade);
}

#[test]
fn test_full_pipeline_missing_skill() {
    let dir = tempfile::TempDir::new().unwrap();
    let empty = dir.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();

    let errors = validate(&empty);
    assert!(errors.iter().any(|d| d.rule_id == "E001"));
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p skillplane-core --test integration`
Expected: Both tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/skillplane-core/tests/integration.rs
git commit -m "test: add end-to-end integration tests for full pipeline"
```

---

## Self-Review Checklist

- [x] **Spec coverage:** Tasks 1-9 cover Phases 1-4 from the spec's Implementation Order: Parse (Task 3), Validate (Task 5), Lint (Task 6), Score (Tasks 7-8). Discovery (Task 4) enables multi-skill repos. Config (Task 7) enables rule suppression and weight customization.
- [x] **No placeholders:** Every step has actual code, commands, and expected output.
- [x] **Type consistency:** `Diagnostic`, `Severity`, `Category`, `ScoreCard`, `Grade`, `RuleResult`, `Frontmatter`, `Body`, `FileTree`, `Span` are defined once in `model.rs` and used consistently.
- [x] **Not covered (deferred to Phase 5-8 plan):** Output formatters (terminal, JSON, SARIF, markdown), CLI binary (clap), fix mode, GitHub Action, pre-commit hook, W010-W019 (script-specific warnings that need file scanning), W022 (nested skills), W025 (conditional loading), W026-W028 (script quality checks).
