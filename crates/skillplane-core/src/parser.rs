use std::collections::BTreeMap;
use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::model::{Body, CodeBlock, FileReference, Frontmatter, Heading, Span};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    MissingFrontmatter,
    UnclosedFrontmatter,
    InvalidYaml(String),
    NotAMapping,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MissingFrontmatter => write!(f, "content does not start with `---`"),
            ParseError::UnclosedFrontmatter => write!(f, "no closing `---` found"),
            ParseError::InvalidYaml(msg) => write!(f, "invalid YAML: {msg}"),
            ParseError::NotAMapping => write!(f, "frontmatter is not a YAML mapping"),
        }
    }
}

impl std::error::Error for ParseError {}

// ---------------------------------------------------------------------------
// Frontmatter parsing
// ---------------------------------------------------------------------------

/// Parse the YAML frontmatter and return the structured `Frontmatter` plus the
/// remaining body text (everything after the closing `---` line).
pub fn parse_frontmatter(content: &str) -> Result<(Frontmatter, String), ParseError> {
    // Strip UTF-8 BOM if present.
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(content);

    // The first line must be exactly `---`.
    let mut lines = content.lines();
    match lines.next() {
        Some(l) if l.trim() == "---" => {}
        _ => return Err(ParseError::MissingFrontmatter),
    }

    // Find the closing `---`.
    let rest: &str = &content[content.find('\n').map(|i| i + 1).unwrap_or(content.len())..];
    let closing_pos = rest
        .lines()
        .enumerate()
        .find(|(_, l)| l.trim() == "---")
        .map(|(idx, _)| idx);

    let closing_idx = closing_pos.ok_or(ParseError::UnclosedFrontmatter)?;

    let yaml_lines: Vec<&str> = rest.lines().take(closing_idx).collect();
    let yaml_str = yaml_lines.join("\n");

    // Body starts after the closing `---` line.
    let body_start: usize = rest
        .lines()
        .take(closing_idx + 1)
        .map(|l| l.len() + 1) // +1 for the newline
        .sum();
    let body_raw = &rest[body_start.min(rest.len())..];

    // Parse YAML.
    let value: serde_yaml::Value =
        serde_yaml::from_str(&yaml_str).map_err(|e| ParseError::InvalidYaml(e.to_string()))?;

    let mapping = match value {
        serde_yaml::Value::Mapping(m) => m,
        serde_yaml::Value::Null => serde_yaml::Mapping::new(), // empty frontmatter is OK
        _ => return Err(ParseError::NotAMapping),
    };

    let mut fm = Frontmatter::default();

    fn as_string(v: &serde_yaml::Value) -> Option<String> {
        match v {
            serde_yaml::Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    let mut unknown = BTreeMap::new();

    for (k, v) in &mapping {
        let key = match k {
            serde_yaml::Value::String(s) => s.as_str(),
            _ => continue,
        };
        match key {
            "name" => fm.name = as_string(v),
            "description" => fm.description = as_string(v),
            "license" => fm.license = as_string(v),
            "compatibility" => fm.compatibility = as_string(v),
            "metadata" => fm.metadata = Some(v.clone()),
            "allowed-tools" => fm.allowed_tools = Some(v.clone()),
            other => {
                unknown.insert(other.to_string(), v.clone());
            }
        }
    }
    fm.unknown_fields = unknown;

    Ok((fm, body_raw.to_string()))
}

// ---------------------------------------------------------------------------
// Body parsing
// ---------------------------------------------------------------------------

static FILE_REF_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap());

static PLACEHOLDER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(TODO|FIXME|TBD|CHANGEME|XXX)\b").unwrap());

/// Parse the markdown body and extract structural metadata.
pub fn parse_body(raw: &str, skill_dir: &Path) -> Body {
    let line_count = raw.lines().count();
    let word_count = raw.split_whitespace().count();
    let estimated_tokens = (word_count as f64 / 0.75).round() as usize;

    let mut headings = Vec::new();
    let mut code_blocks: Vec<CodeBlock> = Vec::new();
    let mut file_references = Vec::new();
    let mut has_placeholder_text = false;

    // State for fenced code block tracking.
    let mut in_code_block = false;
    let mut fence_char: char = '`';
    let mut fence_count: usize = 0;
    let mut code_block_start_line: usize = 0;
    let mut code_block_lang: Option<String> = None;
    let mut code_block_content = String::new();

    for (line_idx, line) in raw.lines().enumerate() {
        let line_num = line_idx + 1; // 1-based

        if in_code_block {
            // Check for closing fence: same char, at least same count, nothing else meaningful.
            let trimmed = line.trim_start();
            let (c, cnt) = fence_info(trimmed);
            if c == Some(fence_char) && cnt >= fence_count && trimmed.trim_matches(fence_char).trim().is_empty() {
                // End of code block.
                in_code_block = false;
                code_blocks.push(CodeBlock {
                    language: code_block_lang.take(),
                    content: code_block_content.clone(),
                    span: Span {
                        start_line: code_block_start_line,
                        start_col: 1,
                        end_line: line_num,
                        end_col: line.len() + 1,
                    },
                });
                code_block_content.clear();
            } else {
                if !code_block_content.is_empty() {
                    code_block_content.push('\n');
                }
                code_block_content.push_str(line);
            }
            continue;
        }

        // Check for opening fence.
        let trimmed = line.trim_start();
        let (c, cnt) = fence_info(trimmed);
        if let Some(ch) = c {
            if cnt >= 3 {
                in_code_block = true;
                fence_char = ch;
                fence_count = cnt;
                code_block_start_line = line_num;
                let after_fence = &trimmed[cnt..];
                let lang = after_fence.trim();
                code_block_lang = if lang.is_empty() {
                    None
                } else {
                    Some(lang.to_string())
                };
                code_block_content.clear();
                continue;
            }
        }

        // Heading detection.
        if let Some(heading) = parse_heading(line, line_num) {
            headings.push(heading);
        }

        // File reference detection.
        for cap in FILE_REF_RE.captures_iter(line) {
            let text = cap.get(1).unwrap().as_str();
            let href = cap.get(2).unwrap().as_str();
            let _ = text; // we don't need the link text

            // Skip URLs, anchors, mailto.
            if href.starts_with("http://")
                || href.starts_with("https://")
                || href.starts_with('#')
                || href.starts_with("mailto:")
            {
                continue;
            }

            let match_start = cap.get(0).unwrap().start();
            let match_end = cap.get(0).unwrap().end();
            let exists = skill_dir.join(href).exists();

            file_references.push(FileReference {
                path: href.to_string(),
                span: Span {
                    start_line: line_num,
                    start_col: match_start + 1,
                    end_line: line_num,
                    end_col: match_end + 1,
                },
                exists,
            });
        }

        // Placeholder detection (only outside code blocks).
        if !has_placeholder_text && PLACEHOLDER_RE.is_match(line) {
            has_placeholder_text = true;
        }
    }

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

/// Return the fence character and count if the line starts with ` or ~.
fn fence_info(trimmed: &str) -> (Option<char>, usize) {
    let first = trimmed.chars().next();
    match first {
        Some('`') | Some('~') => {
            let ch = first.unwrap();
            let count = trimmed.chars().take_while(|&c| c == ch).count();
            (Some(ch), count)
        }
        _ => (None, 0),
    }
}

/// Parse a markdown heading from a single line.
fn parse_heading(line: &str, line_num: usize) -> Option<Heading> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim().to_string();
    Some(Heading {
        level: level as u8,
        text,
        span: Span {
            start_line: line_num,
            start_col: 1,
            end_line: line_num,
            end_col: line.len() + 1,
        },
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_valid_skill() {
        let content = r#"---
name: my-skill
description: A useful skill
---
# Usage

Some body content here.
"#;
        let (fm, body) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.name, Some("my-skill".to_string()));
        assert_eq!(fm.description, Some("A useful skill".to_string()));
        assert!(body.contains("# Usage"));
        assert!(body.contains("Some body content here."));
    }

    #[test]
    fn test_parse_missing_frontmatter() {
        let content = "No frontmatter here.";
        let err = parse_frontmatter(content).unwrap_err();
        assert_eq!(err, ParseError::MissingFrontmatter);
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let content = "---\nname: test\nno closing delimiter";
        let err = parse_frontmatter(content).unwrap_err();
        assert_eq!(err, ParseError::UnclosedFrontmatter);
    }

    #[test]
    fn test_parse_non_mapping_frontmatter() {
        let content = "---\n- item1\n- item2\n---\nbody";
        let err = parse_frontmatter(content).unwrap_err();
        assert_eq!(err, ParseError::NotAMapping);
    }

    #[test]
    fn test_strip_utf8_bom() {
        let content = "\u{FEFF}---\nname: bom-skill\n---\nbody";
        let (fm, body) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.name, Some("bom-skill".to_string()));
        assert_eq!(body, "body");
    }

    #[test]
    fn test_unknown_fields_captured() {
        let content = "---\nname: test\nauthor: someone\n---\nbody";
        let (fm, _) = parse_frontmatter(content).unwrap();
        assert!(fm.unknown_fields.contains_key("author"));
        assert_eq!(
            fm.unknown_fields["author"],
            serde_yaml::Value::String("someone".to_string())
        );
    }

    #[test]
    fn test_metadata_preserved_as_value() {
        let content = "---\nmetadata:\n  version: 2\n  tags:\n    - lint\n---\nbody";
        let (fm, _) = parse_frontmatter(content).unwrap();
        assert!(fm.metadata.is_some());
        let meta = fm.metadata.unwrap();
        // It should be a mapping, not a plain string.
        assert!(meta.is_mapping());
    }

    #[test]
    fn test_allowed_tools_as_string() {
        let content = "---\nallowed-tools: all\n---\nbody";
        let (fm, _) = parse_frontmatter(content).unwrap();
        assert!(fm.allowed_tools.is_some());
        assert_eq!(
            fm.allowed_tools.unwrap(),
            serde_yaml::Value::String("all".to_string())
        );
    }
}
