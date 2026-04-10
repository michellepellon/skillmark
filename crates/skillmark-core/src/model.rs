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
    pub start_line: usize,  // 1-based
    pub start_col: usize,   // 1-based
    pub end_line: usize,    // 1-based, inclusive
    pub end_col: usize,     // 1-based, exclusive
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
