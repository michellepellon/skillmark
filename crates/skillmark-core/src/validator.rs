use std::collections::HashSet;
use std::fs;
use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;
use walkdir::WalkDir;

use crate::discovery::find_skill_md;
use crate::model::{Category, Diagnostic, Severity, Span};
use crate::parser::{parse_body, parse_frontmatter, ParseError};

// ---------------------------------------------------------------------------
// Allowed frontmatter fields
// ---------------------------------------------------------------------------

static ALLOWED_FIELDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("name");
    s.insert("description");
    s.insert("license");
    s.insert("compatibility");
    s.insert("metadata");
    s.insert("allowed-tools");
    s
});

// ---------------------------------------------------------------------------
// Secret patterns for E035
// ---------------------------------------------------------------------------

static SECRET_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(),
        Regex::new(r"\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,255}\b").unwrap(),
        Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{22,255}\b").unwrap(),
        Regex::new(r"-----BEGIN\s+(RSA|EC|DSA|OPENSSH|PGP)?\s*PRIVATE\s+KEY-----").unwrap(),
        Regex::new(r#"(?i)(?:api[_\-]?key|api[_\-]?secret|auth[_\-]?token|access[_\-]?token|bearer)\s*[:=]\s*["']?[A-Za-z0-9_\-]{20,}\b"#).unwrap(),
        Regex::new(r"\bxox[bpors]-[0-9]{10,}-[A-Za-z0-9_\-]{10,}\b").unwrap(),
        Regex::new(r#"(?i)(?:password|passwd|pwd)\s*[:=]\s*["'][^"']{8,}["']"#).unwrap(),
    ]
});

// ---------------------------------------------------------------------------
// Helper to create a diagnostic
// ---------------------------------------------------------------------------

fn diag(
    rule_id: &str,
    message: String,
    path: &Path,
    span: Option<Span>,
    fix_available: bool,
) -> Diagnostic {
    Diagnostic {
        rule_id: rule_id.to_string(),
        severity: Severity::Error,
        message,
        path: path.to_path_buf(),
        span,
        fix_available,
        category: Category::SpecCompliance,
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Validate a skill directory for spec-compliance (E001-E035).
pub fn validate(skill_dir: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // E033: Directory doesn't exist or not a directory
    if !skill_dir.exists() || !skill_dir.is_dir() {
        diagnostics.push(diag(
            "E033",
            "skill directory does not exist or is not a directory".into(),
            skill_dir,
            None,
            false,
        ));
        return diagnostics;
    }

    // Find SKILL.md (case-insensitive)
    let skill_md_path = match find_skill_md(skill_dir) {
        Some(p) => p,
        None => {
            // E001: SKILL.md missing
            diagnostics.push(diag(
                "E001",
                "SKILL.md is missing".into(),
                skill_dir,
                None,
                false,
            ));
            // Still scan for E035
            check_secrets(skill_dir, &mut diagnostics);
            return diagnostics;
        }
    };

    // E002: Wrong case
    let filename = skill_md_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if filename != "SKILL.md" {
        diagnostics.push(diag(
            "E002",
            format!("SKILL.md has wrong case: found '{filename}'"),
            &skill_md_path,
            None,
            true,
        ));
    }

    // Read content
    let content = match fs::read_to_string(&skill_md_path) {
        Ok(c) => c,
        Err(e) => {
            diagnostics.push(diag(
                "E003",
                format!("could not read SKILL.md: {e}"),
                &skill_md_path,
                None,
                false,
            ));
            return diagnostics;
        }
    };

    // E034: UTF-8 BOM present
    if content.starts_with('\u{FEFF}') {
        diagnostics.push(diag(
            "E034",
            "UTF-8 BOM detected at start of SKILL.md".into(),
            &skill_md_path,
            Some(Span {
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 2,
            }),
            true,
        ));
    }

    // Parse frontmatter
    let (fm, body_raw) = match parse_frontmatter(&content) {
        Ok(result) => result,
        Err(e) => {
            let (rule_id, fix) = match &e {
                ParseError::MissingFrontmatter => ("E003", true),
                ParseError::InvalidYaml(_) => ("E003", true),
                ParseError::UnclosedFrontmatter => ("E032", true),
                ParseError::NotAMapping => ("E004", false),
            };
            diagnostics.push(diag(
                rule_id,
                format!("frontmatter error: {e}"),
                &skill_md_path,
                Some(Span {
                    start_line: 1,
                    start_col: 1,
                    end_line: 1,
                    end_col: 1,
                }),
                fix,
            ));
            // Can't continue without frontmatter
            check_secrets(skill_dir, &mut diagnostics);
            return diagnostics;
        }
    };

    // E005: Non-string keys in frontmatter — we need raw YAML for this
    check_non_string_keys(&content, &skill_md_path, &mut diagnostics);

    // --- Name rules (E006-E013) ---
    validate_name(&fm.name, skill_dir, &skill_md_path, &mut diagnostics);

    // --- Description rules (E014-E016) ---
    validate_description(&fm.description, &skill_md_path, &mut diagnostics);

    // --- Compatibility rules (E017-E019) ---
    validate_compatibility(&fm.compatibility, &skill_md_path, &mut diagnostics);

    // --- License rules (E020-E021) ---
    validate_license(&fm.license, &skill_md_path, &mut diagnostics);

    // --- Metadata rules (E022-E024) ---
    validate_metadata(&fm.metadata, &skill_md_path, &mut diagnostics);

    // --- Allowed-tools rules (E025-E029) ---
    validate_allowed_tools(&fm.allowed_tools, &skill_md_path, &mut diagnostics);

    // --- E030: Unknown fields ---
    for key in fm.unknown_fields.keys() {
        if !ALLOWED_FIELDS.contains(key.as_str()) {
            diagnostics.push(diag(
                "E030",
                format!("unknown frontmatter field: '{key}'"),
                &skill_md_path,
                None,
                false,
            ));
        }
    }

    // --- E031: File references to nonexistent files ---
    let body = parse_body(&body_raw, skill_dir);
    for fref in &body.file_references {
        if !fref.exists {
            diagnostics.push(diag(
                "E031",
                format!("file reference '{}' points to nonexistent file", fref.path),
                &skill_md_path,
                Some(fref.span.clone()),
                false,
            ));
        }
    }

    // --- E035: Secret/credential patterns ---
    check_secrets(skill_dir, &mut diagnostics);

    diagnostics
}

// ---------------------------------------------------------------------------
// E005: Non-string keys
// ---------------------------------------------------------------------------

fn check_non_string_keys(content: &str, path: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let clean = content.strip_prefix('\u{FEFF}').unwrap_or(content);
    // Extract YAML portion between --- delimiters
    let mut lines = clean.lines();
    if let Some(first) = lines.next() {
        if first.trim() != "---" {
            return;
        }
    } else {
        return;
    }

    let yaml_lines: Vec<&str> = lines.take_while(|l| l.trim() != "---").collect();
    let yaml_str = yaml_lines.join("\n");

    let value: serde_yaml::Value = match serde_yaml::from_str(&yaml_str) {
        Ok(v) => v,
        Err(_) => return,
    };

    if let serde_yaml::Value::Mapping(ref m) = value {
        check_mapping_keys(m, path, diagnostics);
    }
}

fn check_mapping_keys(
    mapping: &serde_yaml::Mapping,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (k, _v) in mapping {
        if !k.is_string() {
            diagnostics.push(diag(
                "E005",
                "frontmatter contains non-string key".into(),
                path,
                None,
                false,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Name validation (E006-E013)
// ---------------------------------------------------------------------------

fn validate_name(
    name: &Option<String>,
    skill_dir: &Path,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name_val = match name {
        None => {
            diagnostics.push(diag("E006", "name field is missing".into(), path, None, false));
            return;
        }
        Some(n) => n,
    };

    // E007: empty
    if name_val.is_empty() {
        diagnostics.push(diag("E007", "name field is empty".into(), path, None, false));
        return;
    }

    // E008: > 64 chars
    if name_val.chars().count() > 64 {
        diagnostics.push(diag(
            "E008",
            format!("name exceeds 64 characters ({})", name_val.chars().count()),
            path,
            None,
            false,
        ));
    }

    let nfkc_name: String = name_val.nfkc().collect();

    // E012: not lowercase (NFKC normalized)
    let lowered: String = nfkc_name.chars().map(|c| {
        let mut lower = String::new();
        for lc in c.to_lowercase() {
            lower.push(lc);
        }
        lower
    }).collect();
    if nfkc_name != lowered {
        diagnostics.push(diag(
            "E012",
            "name contains uppercase characters".into(),
            path,
            None,
            true,
        ));
    }

    // E009: invalid chars (must be unicode lowercase alphanumeric + hyphens)
    for ch in nfkc_name.chars() {
        if ch == '-' {
            continue;
        }
        if !ch.is_alphanumeric() {
            diagnostics.push(diag(
                "E009",
                format!("name contains invalid character: '{ch}'"),
                path,
                None,
                false,
            ));
            break;
        }
    }

    // E010: starts/ends with hyphen
    if nfkc_name.starts_with('-') || nfkc_name.ends_with('-') {
        diagnostics.push(diag(
            "E010",
            "name must not start or end with a hyphen".into(),
            path,
            None,
            false,
        ));
    }

    // E011: consecutive hyphens
    if nfkc_name.contains("--") {
        diagnostics.push(diag(
            "E011",
            "name contains consecutive hyphens".into(),
            path,
            None,
            false,
        ));
    }

    // E013: doesn't match parent directory name (NFKC)
    if let Some(dir_name) = skill_dir.file_name().and_then(|n| n.to_str()) {
        let dir_nfkc: String = dir_name.nfkc().collect();
        if nfkc_name != dir_nfkc {
            diagnostics.push(diag(
                "E013",
                format!(
                    "name '{nfkc_name}' does not match parent directory '{dir_nfkc}'"
                ),
                path,
                None,
                true,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Description validation (E014-E016)
// ---------------------------------------------------------------------------

fn validate_description(
    description: &Option<String>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let desc = match description {
        None => {
            diagnostics.push(diag(
                "E014",
                "description field is missing".into(),
                path,
                None,
                false,
            ));
            return;
        }
        Some(d) => d,
    };

    if desc.is_empty() {
        diagnostics.push(diag(
            "E015",
            "description field is empty".into(),
            path,
            None,
            false,
        ));
    } else if desc.chars().count() > 1024 {
        diagnostics.push(diag(
            "E016",
            format!("description exceeds 1024 characters ({})", desc.chars().count()),
            path,
            None,
            false,
        ));
    }
}

// ---------------------------------------------------------------------------
// Compatibility validation (E017-E019)
// ---------------------------------------------------------------------------

fn validate_compatibility(
    compatibility: &Option<String>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let compat = match compatibility {
        None => return, // compatibility is optional
        Some(c) => c,
    };

    // E019: empty string
    if compat.is_empty() {
        diagnostics.push(diag(
            "E019",
            "compatibility field is empty".into(),
            path,
            None,
            false,
        ));
        return;
    }

    // E017: > 500 chars
    if compat.chars().count() > 500 {
        diagnostics.push(diag(
            "E017",
            format!("compatibility exceeds 500 characters ({})", compat.chars().count()),
            path,
            None,
            false,
        ));
    }

    // NOTE: E018/E020 are currently unreachable — the parser extracts these
    // fields as Option<String>, so non-string values silently become None.
    // This is an accepted limitation per the design spec.
}

// ---------------------------------------------------------------------------
// License validation (E020-E021)
// ---------------------------------------------------------------------------

fn validate_license(
    license: &Option<String>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let lic = match license {
        None => return, // license is optional
        Some(l) => l,
    };

    // E021: empty string
    if lic.is_empty() {
        diagnostics.push(diag(
            "E021",
            "license field is empty".into(),
            path,
            None,
            false,
        ));
    }

    // NOTE: E018/E020 are currently unreachable — the parser extracts these
    // fields as Option<String>, so non-string values silently become None.
    // This is an accepted limitation per the design spec.
}

// ---------------------------------------------------------------------------
// Metadata validation (E022-E024)
// ---------------------------------------------------------------------------

fn validate_metadata(
    metadata: &Option<serde_yaml::Value>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let meta = match metadata {
        None => return,
        Some(v) => v,
    };

    // E022: not a mapping
    let mapping = match meta {
        serde_yaml::Value::Mapping(m) => m,
        _ => {
            diagnostics.push(diag(
                "E022",
                "metadata must be a YAML mapping".into(),
                path,
                None,
                false,
            ));
            return;
        }
    };

    for (k, v) in mapping {
        // E023: non-string keys
        if !k.is_string() {
            diagnostics.push(diag(
                "E023",
                "metadata contains a non-string key".into(),
                path,
                None,
                false,
            ));
        }

        // E024: non-string values
        if !v.is_string() {
            diagnostics.push(diag(
                "E024",
                format!(
                    "metadata value for key '{}' is not a string",
                    k.as_str().unwrap_or("<non-string>")
                ),
                path,
                None,
                false,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Allowed-tools validation (E025-E029)
// ---------------------------------------------------------------------------

fn validate_allowed_tools(
    allowed_tools: &Option<serde_yaml::Value>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let tools = match allowed_tools {
        None => return,
        Some(v) => v,
    };

    match tools {
        serde_yaml::Value::String(s) => {
            // E026: empty string
            if s.is_empty() {
                diagnostics.push(diag(
                    "E026",
                    "allowed-tools is empty".into(),
                    path,
                    None,
                    false,
                ));
            }
            // E027: comma-delimited instead of space-delimited
            if s.contains(',') {
                diagnostics.push(diag(
                    "E027",
                    "allowed-tools appears to use comma delimiters instead of spaces".into(),
                    path,
                    None,
                    false,
                ));
            }
            // E028: unbalanced parentheses
            check_unbalanced_parens(s, path, diagnostics);
        }
        serde_yaml::Value::Sequence(seq) => {
            // E026: empty array
            if seq.is_empty() {
                diagnostics.push(diag(
                    "E026",
                    "allowed-tools array is empty".into(),
                    path,
                    None,
                    false,
                ));
            }
            for item in seq {
                match item {
                    serde_yaml::Value::String(s) => {
                        // E027
                        if s.contains(',') {
                            diagnostics.push(diag(
                                "E027",
                                "allowed-tools entry appears to use comma delimiters".into(),
                                path,
                                None,
                                false,
                            ));
                        }
                        // E028
                        check_unbalanced_parens(s, path, diagnostics);
                    }
                    _ => {
                        // E029: array contains non-string items
                        diagnostics.push(diag(
                            "E029",
                            "allowed-tools array contains a non-string item".into(),
                            path,
                            None,
                            false,
                        ));
                    }
                }
            }
        }
        _ => {
            // E025: not a string or array
            diagnostics.push(diag(
                "E025",
                "allowed-tools must be a string or array".into(),
                path,
                None,
                false,
            ));
        }
    }
}

fn check_unbalanced_parens(s: &str, path: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let open = s.chars().filter(|&c| c == '(').count();
    let close = s.chars().filter(|&c| c == ')').count();
    if open != close {
        diagnostics.push(diag(
            "E028",
            "allowed-tools contains unbalanced parentheses".into(),
            path,
            None,
            false,
        ));
    }
}

// ---------------------------------------------------------------------------
// E035: Secret/credential pattern scanning
// ---------------------------------------------------------------------------

fn check_secrets(skill_dir: &Path, diagnostics: &mut Vec<Diagnostic>) {
    for entry in WalkDir::new(skill_dir)
        .max_depth(3)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();

        // Read file bytes; skip if unreadable
        let bytes = match fs::read(file_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        // Skip binary files (null byte in first 8192 bytes)
        let check_len = bytes.len().min(8192);
        if bytes[..check_len].contains(&0u8) {
            continue;
        }

        let text = match std::str::from_utf8(&bytes) {
            Ok(t) => t,
            Err(_) => continue,
        };

        for pattern in SECRET_PATTERNS.iter() {
            if let Some(m) = pattern.find(text) {
                // Find line number
                let line_num = text[..m.start()].lines().count() + 1;
                diagnostics.push(diag(
                    "E035",
                    format!(
                        "potential secret/credential detected in {}",
                        file_path.strip_prefix(skill_dir).unwrap_or(file_path).display()
                    ),
                    file_path,
                    Some(Span {
                        start_line: line_num,
                        start_col: 1,
                        end_line: line_num,
                        end_col: 1,
                    }),
                    false,
                ));
                // One diagnostic per file per pattern match is enough; break to avoid noise
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_skill(dir: &Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    fn has_rule(diags: &[Diagnostic], rule_id: &str) -> bool {
        diags.iter().any(|d| d.rule_id == rule_id)
    }

    #[test]
    fn test_e001_missing_skill_md() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("empty-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E001"), "expected E001, got: {diags:?}");
    }

    #[test]
    fn test_e002_wrong_case() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("skill.md"),
            "---\nname: my-skill\ndescription: test\n---\nbody\n",
        )
        .unwrap();
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E002"), "expected E002, got: {diags:?}");
    }

    #[test]
    fn test_e003_missing_frontmatter() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(&skill_dir, "no frontmatter here");
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E003"), "expected E003, got: {diags:?}");
    }

    #[test]
    fn test_e004_not_a_mapping() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(&skill_dir, "---\n- item1\n- item2\n---\nbody\n");
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E004"), "expected E004, got: {diags:?}");
    }

    #[test]
    fn test_e006_missing_name() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(&skill_dir, "---\ndescription: test\n---\nbody\n");
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E006"), "expected E006, got: {diags:?}");
    }

    #[test]
    fn test_e008_name_too_long() {
        let dir = TempDir::new().unwrap();
        let long_name = "a".repeat(65);
        let skill_dir = dir.path().join(&long_name);
        make_skill(
            &skill_dir,
            &format!("---\nname: {long_name}\ndescription: test\n---\nbody\n"),
        );
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E008"), "expected E008, got: {diags:?}");
    }

    #[test]
    fn test_e012_uppercase_name() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(
            &skill_dir,
            "---\nname: My-Skill\ndescription: test\n---\nbody\n",
        );
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E012"), "expected E012, got: {diags:?}");
    }

    #[test]
    fn test_e013_name_dir_mismatch() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("wrong-dir-name");
        make_skill(
            &skill_dir,
            "---\nname: correct-name\ndescription: test\n---\nbody\n",
        );
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E013"), "expected E013, got: {diags:?}");
    }

    #[test]
    fn test_valid_skill_no_errors() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(
            &skill_dir,
            "---\nname: my-skill\ndescription: A useful test skill\n---\n# Usage\n\nSome content.\n",
        );
        let diags = validate(&skill_dir);
        assert!(
            diags.is_empty(),
            "expected no diagnostics, got: {diags:?}"
        );
    }

    #[test]
    fn test_e033_nonexistent_dir() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("does-not-exist");
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E033"), "expected E033, got: {diags:?}");
    }

    #[test]
    fn test_e032_unclosed_frontmatter() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(&skill_dir, "---\nname: my-skill\nno closing delimiter");
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E032"), "expected E032, got: {diags:?}");
    }

    #[test]
    fn test_e034_utf8_bom() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(
            &skill_dir,
            "\u{FEFF}---\nname: my-skill\ndescription: test\n---\nbody\n",
        );
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E034"), "expected E034, got: {diags:?}");
    }

    #[test]
    fn test_e030_unknown_field() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(
            &skill_dir,
            "---\nname: my-skill\ndescription: test\nauthor: someone\n---\nbody\n",
        );
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E030"), "expected E030, got: {diags:?}");
    }

    #[test]
    fn test_e035_secret_detection() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        make_skill(
            &skill_dir,
            "---\nname: my-skill\ndescription: test\n---\nbody\n",
        );
        // Create a file with a fake AWS key
        fs::write(
            skill_dir.join("config.txt"),
            "aws_key = AKIAIOSFODNN7EXAMPLE",
        )
        .unwrap();
        let diags = validate(&skill_dir);
        assert!(has_rule(&diags, "E035"), "expected E035, got: {diags:?}");
    }
}
