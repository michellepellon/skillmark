use std::fs;
use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

use crate::discovery::find_skill_md;

// ---------------------------------------------------------------------------
// FixResult
// ---------------------------------------------------------------------------

pub struct FixResult {
    pub changed: bool,
    pub fixes_applied: Vec<String>,
    pub new_content: Option<String>, // For dry-run preview
    pub target_path: Option<PathBuf>, // Path that was written to
}

impl FixResult {
    fn no_change() -> Self {
        FixResult {
            changed: false,
            fixes_applied: Vec::new(),
            new_content: None,
            target_path: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Apply all fixable rules to the skill directory.
///
/// Fix order: E034 → E002 → E032 → E003 → E012 → E013
///
/// If `dry_run` is true, no files are written/renamed; the result contains the
/// would-be content in `new_content`.
pub fn fix_skill(skill_dir: &Path, dry_run: bool) -> FixResult {
    // -----------------------------------------------------------------------
    // Locate the SKILL.md file (or case variant)
    // -----------------------------------------------------------------------
    let skill_md_path = match find_skill_md(skill_dir) {
        Some(p) => p,
        None => {
            // No skill file to fix
            return FixResult::no_change();
        }
    };

    // -----------------------------------------------------------------------
    // Read content
    // -----------------------------------------------------------------------
    let original_content = match fs::read_to_string(&skill_md_path) {
        Ok(c) => c,
        Err(_) => return FixResult::no_change(),
    };

    let mut content = original_content.clone();
    let mut fixes_applied: Vec<String> = Vec::new();
    let mut needs_rename = false;

    // -----------------------------------------------------------------------
    // E034 — Strip BOM
    // -----------------------------------------------------------------------
    if content.starts_with('\u{FEFF}') {
        content = content.strip_prefix('\u{FEFF}').unwrap().to_string();
        fixes_applied.push("E034: Stripped UTF-8 BOM".to_string());
    }

    // -----------------------------------------------------------------------
    // E002 — Rename to SKILL.md (detect but execute after content is final)
    // -----------------------------------------------------------------------
    let filename = skill_md_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    if filename != "SKILL.md" {
        needs_rename = true;
        fixes_applied.push(format!("E002: Renamed '{}' to 'SKILL.md'", filename));
    }

    // -----------------------------------------------------------------------
    // E032 — Close unclosed frontmatter
    // -----------------------------------------------------------------------
    if content.starts_with("---") && !has_closing_delimiter(&content) {
        // Find where YAML content ends and append ---
        content = append_closing_delimiter(content);
        fixes_applied.push("E032: Appended closing '---' to frontmatter".to_string());
    }

    // -----------------------------------------------------------------------
    // E003 — Inject frontmatter if missing
    // -----------------------------------------------------------------------
    if !content.starts_with("---") {
        let dir_name = skill_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let dir_name_nfkc: String = dir_name.nfkc().collect();
        let injected = format!(
            "---\nname: {}\ndescription: \n---\n\n{}",
            dir_name_nfkc, content
        );
        content = injected;
        fixes_applied.push("E003: Injected missing frontmatter".to_string());
    }

    // -----------------------------------------------------------------------
    // E012 & E013 — Fix name field in frontmatter
    // -----------------------------------------------------------------------
    content = fix_name_in_frontmatter(content, skill_dir, &mut fixes_applied);

    // -----------------------------------------------------------------------
    // Determine if anything changed
    // -----------------------------------------------------------------------
    if fixes_applied.is_empty() {
        return FixResult::no_change();
    }

    let target_path = if needs_rename {
        skill_dir.join("SKILL.md")
    } else {
        skill_md_path.clone()
    };

    // -----------------------------------------------------------------------
    // Write or return preview
    // -----------------------------------------------------------------------
    if dry_run {
        return FixResult {
            changed: true,
            fixes_applied,
            new_content: Some(content),
            target_path: Some(target_path),
        };
    }

    if needs_rename && skill_md_path != target_path {
        // On case-insensitive filesystems (e.g. macOS HFS+), writing directly
        // to `SKILL.md` when `skill.md` exists hits the same inode.  We must
        // write to a temporary file and then rename in two steps:
        //   1. Write updated content to a temp file.
        //   2. Rename the original to the canonical uppercase name via a
        //      round-trip through a temporary name to force the case change.
        let tmp_path = skill_dir.join("__skillplane_tmp__.md");
        if let Err(_e) = fs::write(&tmp_path, &content) {
            return FixResult::no_change();
        }
        // Remove old file first so the rename can land on the correct case.
        let _ = fs::remove_file(&skill_md_path);
        if let Err(_e) = fs::rename(&tmp_path, &target_path) {
            // Clean up temp file on failure
            let _ = fs::remove_file(&tmp_path);
            return FixResult::no_change();
        }
    } else {
        // No rename needed — just overwrite in place.
        if let Err(_e) = fs::write(&target_path, &content) {
            return FixResult::no_change();
        }
    }

    FixResult {
        changed: true,
        fixes_applied,
        new_content: None,
        target_path: Some(target_path),
    }
}

// ---------------------------------------------------------------------------
// Helper: detect closing delimiter
// ---------------------------------------------------------------------------

fn has_closing_delimiter(content: &str) -> bool {
    // Skip the first line (opening ---), look for another --- line
    let mut lines = content.lines();
    lines.next(); // consume opening ---
    lines.any(|l| l.trim() == "---")
}

// ---------------------------------------------------------------------------
// Helper: append closing --- after YAML content
// ---------------------------------------------------------------------------

fn append_closing_delimiter(content: String) -> String {
    // The content starts with "---\n" but has no closing ---
    // We append \n---\n at the end of the YAML block.
    // Since the whole file is YAML (no body), just append.
    if content.ends_with('\n') {
        format!("{content}---\n")
    } else {
        format!("{content}\n---\n")
    }
}

// ---------------------------------------------------------------------------
// Helper: fix the name field in frontmatter (E012 + E013)
// ---------------------------------------------------------------------------

fn fix_name_in_frontmatter(
    content: String,
    skill_dir: &Path,
    fixes_applied: &mut Vec<String>,
) -> String {
    // Parse frontmatter region
    let without_bom = content.strip_prefix('\u{FEFF}').unwrap_or(&content);

    let mut lines = without_bom.lines();
    // Must start with ---
    match lines.next() {
        Some(l) if l.trim() == "---" => {}
        _ => return content,
    }

    // Collect YAML lines until closing ---
    let rest_start = without_bom
        .find('\n')
        .map(|i| i + 1)
        .unwrap_or(without_bom.len());
    let rest = &without_bom[rest_start..];

    let closing_idx = rest
        .lines()
        .enumerate()
        .find(|(_, l)| l.trim() == "---")
        .map(|(idx, _)| idx);

    let closing_idx = match closing_idx {
        Some(i) => i,
        None => return content, // unclosed — handled by E032
    };

    let yaml_lines: Vec<&str> = rest.lines().take(closing_idx).collect();
    let yaml_str = yaml_lines.join("\n");

    // Body starts after closing ---
    let body_start: usize = rest
        .lines()
        .take(closing_idx + 1)
        .map(|l| l.len() + 1)
        .sum();
    let body_raw = &rest[body_start.min(rest.len())..];

    // Parse YAML into a Value
    let value: serde_yaml::Value = match serde_yaml::from_str(&yaml_str) {
        Ok(v) => v,
        Err(_) => return content,
    };

    let mapping = match value {
        serde_yaml::Value::Mapping(m) => m,
        serde_yaml::Value::Null => return content,
        _ => return content,
    };

    // Extract current name
    let current_name = mapping
        .get(serde_yaml::Value::String("name".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let current_name = match current_name {
        Some(n) => n,
        None => return content,
    };

    let nfkc_name: String = current_name.nfkc().collect();
    let lowered_name: String = nfkc_name.to_lowercase();

    // Compute expected name from directory
    let dir_name = skill_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let dir_nfkc: String = dir_name.nfkc().collect();

    let mut new_name = nfkc_name.clone();
    let mut name_changed = false;

    // E012: lowercase
    if nfkc_name != lowered_name {
        new_name = lowered_name.clone();
        name_changed = true;
        fixes_applied.push("E012: Lowercased name field".to_string());
    }

    // E013: must match directory name
    if new_name != dir_nfkc {
        new_name = dir_nfkc.clone();
        name_changed = true;
        fixes_applied.push(format!(
            "E013: Set name to match directory '{}'",
            dir_nfkc
        ));
    }

    if !name_changed {
        return content;
    }

    // Rebuild frontmatter with updated name
    // Use deterministic field ordering: name, description, license,
    // compatibility, allowed-tools, metadata.
    let new_fm_str = build_frontmatter_yaml(&mapping, &new_name);
    let new_content = format!("---\n{}---\n{}", new_fm_str, body_raw);
    new_content
}

// ---------------------------------------------------------------------------
// Helper: serialize frontmatter with deterministic field ordering
// ---------------------------------------------------------------------------

fn build_frontmatter_yaml(mapping: &serde_yaml::Mapping, new_name: &str) -> String {
    let field_order = [
        "name",
        "description",
        "license",
        "compatibility",
        "allowed-tools",
        "metadata",
    ];

    let mut output = String::new();

    // Output known fields in order
    for field in &field_order {
        let key = serde_yaml::Value::String(field.to_string());
        let value = if *field == "name" {
            Some(serde_yaml::Value::String(new_name.to_string()))
        } else {
            mapping.get(&key).cloned()
        };

        if let Some(v) = value {
            match &v {
                serde_yaml::Value::String(s) => {
                    output.push_str(&format!("{}: \"{}\"\n", field, escape_yaml_string(s)));
                }
                serde_yaml::Value::Null => {
                    output.push_str(&format!("{}: \"\"\n", field));
                }
                _ => {
                    // For complex types (mapping, sequence), serialize via serde_yaml
                    let serialized = serde_yaml::to_string(&v).unwrap_or_default();
                    // serde_yaml adds "---\n" prefix; strip it
                    let serialized = serialized.trim_start_matches("---\n");
                    let serialized = serialized.trim_end_matches('\n');
                    output.push_str(&format!("{}: {}\n", field, serialized));
                }
            }
        }
    }

    // Output any unknown fields that aren't in our known list
    for (k, v) in mapping {
        if let Some(key_str) = k.as_str() {
            if field_order.contains(&key_str) {
                continue;
            }
            match v {
                serde_yaml::Value::String(s) => {
                    output.push_str(&format!("{}: \"{}\"\n", key_str, escape_yaml_string(s)));
                }
                _ => {
                    let serialized = serde_yaml::to_string(v).unwrap_or_default();
                    let serialized = serialized.trim_start_matches("---\n");
                    let serialized = serialized.trim_end_matches('\n');
                    output.push_str(&format!("{}: {}\n", key_str, serialized));
                }
            }
        }
    }

    output
}

fn escape_yaml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Test 1: fix_rename_to_uppercase
    // -----------------------------------------------------------------------
    #[test]
    fn test_fix_rename_to_uppercase() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();

        // Write skill.md (lowercase)
        fs::write(
            skill_dir.join("skill.md"),
            "---\nname: my-skill\ndescription: test\n---\nbody\n",
        )
        .unwrap();

        let result = fix_skill(&skill_dir, false);

        assert!(result.changed, "expected a fix to be applied");
        assert!(
            result.fixes_applied.iter().any(|f| f.contains("E002")),
            "expected E002 fix, got: {:?}",
            result.fixes_applied
        );

        // SKILL.md must exist now
        assert!(
            skill_dir.join("SKILL.md").exists(),
            "SKILL.md should exist after fix"
        );

        // Verify the canonical filename is SKILL.md by listing the directory
        let entries: Vec<String> = fs::read_dir(&skill_dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.to_lowercase() == "skill.md")
            .collect();
        assert_eq!(entries.len(), 1, "should be exactly one skill.md variant");
        assert_eq!(
            entries[0], "SKILL.md",
            "the file should be named SKILL.md (uppercase)"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: fix_inject_frontmatter
    // -----------------------------------------------------------------------
    #[test]
    fn test_fix_inject_frontmatter() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();

        // Write SKILL.md without any frontmatter
        fs::write(skill_dir.join("SKILL.md"), "# Usage\n\nSome body content.\n").unwrap();

        let result = fix_skill(&skill_dir, false);

        assert!(result.changed, "expected a fix to be applied");
        assert!(
            result.fixes_applied.iter().any(|f| f.contains("E003")),
            "expected E003 fix, got: {:?}",
            result.fixes_applied
        );

        // Read back the file and verify frontmatter was injected
        let written = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert!(
            written.starts_with("---"),
            "file should start with frontmatter"
        );
        assert!(
            written.contains("name:"),
            "injected frontmatter should contain name field"
        );
        assert!(
            written.contains("description:"),
            "injected frontmatter should contain description field"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: fix_dry_run
    // -----------------------------------------------------------------------
    #[test]
    fn test_fix_dry_run() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();

        let original = "# No frontmatter here\n";
        fs::write(skill_dir.join("SKILL.md"), original).unwrap();

        let result = fix_skill(&skill_dir, true /* dry_run */);

        assert!(result.changed, "dry_run should still report changed=true");
        assert!(
            result.new_content.is_some(),
            "dry_run should return new_content"
        );

        // File must NOT be modified
        let on_disk = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert_eq!(
            on_disk, original,
            "dry_run must not modify the file on disk"
        );
    }
}
