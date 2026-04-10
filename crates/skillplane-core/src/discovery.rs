use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::model::{FileTree, Skill};
use crate::parser::{parse_body, parse_frontmatter};

// ---------------------------------------------------------------------------
// find_skill_md
// ---------------------------------------------------------------------------

/// Look for a file matching "skill.md" (case-insensitive) inside `skill_dir`.
///
/// Preference order:
///   1. "SKILL.md" (exact uppercase)
///   2. Any other case variant (e.g. "skill.md", "Skill.md")
///
/// Returns the path to the best match, or `None` if nothing is found.
pub fn find_skill_md(skill_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(skill_dir).ok()?;

    let mut best: Option<PathBuf> = None;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.to_lowercase() != "skill.md" {
            continue;
        }

        let path = entry.path();

        if name_str == "SKILL.md" {
            // Exact uppercase wins immediately.
            return Some(path);
        }

        // Keep the first non-uppercase match as a fallback.
        if best.is_none() {
            best = Some(path);
        }
    }

    best
}

// ---------------------------------------------------------------------------
// build_file_tree
// ---------------------------------------------------------------------------

/// Scan the immediate children of `skill_dir` and build a `FileTree`.
///
/// Recognised subdirectories: `scripts/`, `references/`, `assets/`, `examples/`.
/// `total_content_size` is the sum of file sizes found inside those subdirectories.
pub fn build_file_tree(skill_dir: &Path) -> FileTree {
    let mut tree = FileTree::default();

    // Collect immediate children.
    if let Ok(entries) = fs::read_dir(skill_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_lowercase();
            let path = entry.path();

            if path.is_dir() {
                match name_str.as_str() {
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

    // Sum file sizes from the known subdirectories.
    let subdirs = ["scripts", "references", "assets", "examples"];
    for subdir in &subdirs {
        let dir_path = skill_dir.join(subdir);
        if dir_path.is_dir() {
            for entry in WalkDir::new(&dir_path)
                .into_iter()
                .flatten()
                .filter(|e| e.file_type().is_file())
            {
                if let Ok(meta) = entry.metadata() {
                    tree.total_content_size += meta.len() as usize;
                }
            }
        }
    }

    tree
}

// ---------------------------------------------------------------------------
// load_skill
// ---------------------------------------------------------------------------

/// Load a skill from a directory.
///
/// Returns `Err` if `SKILL.md` cannot be found or the content cannot be parsed.
pub fn load_skill(skill_dir: &Path) -> Result<Skill, String> {
    let skill_md_path =
        find_skill_md(skill_dir).ok_or_else(|| format!("no SKILL.md found in {skill_dir:?}"))?;

    let content = fs::read_to_string(&skill_md_path)
        .map_err(|e| format!("failed to read {skill_md_path:?}: {e}"))?;

    let (frontmatter, body_raw) = parse_frontmatter(&content)
        .map_err(|e| format!("parse error in {skill_md_path:?}: {e}"))?;

    let body = parse_body(&body_raw, skill_dir);
    let file_tree = build_file_tree(skill_dir);

    Ok(Skill {
        path: skill_dir.to_path_buf(),
        frontmatter,
        body,
        file_tree,
    })
}

// ---------------------------------------------------------------------------
// discover_skills
// ---------------------------------------------------------------------------

/// Walk the directory tree rooted at `root` and return a sorted, deduplicated
/// list of directories that contain a file matching "skill.md"
/// (case-insensitive).
pub fn discover_skills(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .flatten()
        .filter(|e| {
            e.file_type().is_file()
                && e.file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .as_str()
                    == "skill.md"
        })
        .filter_map(|e| e.path().parent().map(|p| p.to_path_buf()))
        .collect();

    dirs.sort();
    dirs.dedup();
    dirs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
