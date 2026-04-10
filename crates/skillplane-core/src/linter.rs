use once_cell::sync::Lazy;
use regex::Regex;

use crate::model::{Category, Diagnostic, Severity, Skill};

// ---------------------------------------------------------------------------
// Helper to create a diagnostic
// ---------------------------------------------------------------------------

fn warn(rule_id: &str, message: String, skill: &Skill, category: Category) -> Diagnostic {
    Diagnostic {
        rule_id: rule_id.to_string(),
        severity: Severity::Warning,
        message,
        path: skill.path.clone(),
        span: None,
        fix_available: false,
        category,
    }
}

fn info(rule_id: &str, message: String, skill: &Skill, category: Category) -> Diagnostic {
    Diagnostic {
        rule_id: rule_id.to_string(),
        severity: Severity::Info,
        message,
        path: skill.path.clone(),
        span: None,
        fix_available: false,
        category,
    }
}

// ---------------------------------------------------------------------------
// Regex patterns (compiled once via Lazy)
// ---------------------------------------------------------------------------

static TRIGGER_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\buse\s+(this\s+skill\s+)?when\b").unwrap(),
        Regex::new(r"(?i)\bwhen\s+the\s+(user|developer|agent)\b").unwrap(),
        Regex::new(r"(?i)\bactivate\s+(this\s+)?(skill\s+)?(when|for|if)\b").unwrap(),
        Regex::new(r"(?i)\binvoke\s+(this\s+)?(skill\s+)?(when|for|if)\b").unwrap(),
        Regex::new(r"(?i)\btrigger(s|ed)?\s+(when|on|by)\b").unwrap(),
        Regex::new(r"(?i)\bapplies?\s+(when|to|if)\b").unwrap(),
    ]
});

static PASSIVE_PREFIXES: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)^this\s+skill\s+(is|was|will|can|should|does|provides|helps|allows|enables|handles|manages|performs)").unwrap(),
        Regex::new(r"(?i)^(is|was|will\s+be)\s+used\s+(to|for|when)").unwrap(),
        Regex::new(r"(?i)^(a|an|the)\s+(skill|tool|helper|utility)\s+(that|which|for)").unwrap(),
        Regex::new(r"(?i)^(provides?|offers?|gives?|enables?|allows?|helps?)\s").unwrap(),
        Regex::new(r"(?i)^(designed|intended|meant|built|created)\s+(to|for)\b").unwrap(),
    ]
});

static VAGUE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)^helps?\s+(with|the|you)\b").unwrap(),
        Regex::new(r"(?i)^a\s+tool\s+(for|to|that)\b").unwrap(),
        Regex::new(r"(?i)^an?\s+(useful|helpful|handy|simple|basic|generic)\s+(skill|tool)\b").unwrap(),
        Regex::new(r"(?i)^(does|handles?)\s+(stuff|things|various|everything|anything)\b").unwrap(),
        Regex::new(r"(?i)\b(various|miscellaneous|general[- ]purpose|multi[- ]purpose|all[- ]purpose)\s+(tasks?|things?|operations?|functions?)\b").unwrap(),
        Regex::new(r"(?i)^(utility|helper)\s+(for|to|that|skill)\b").unwrap(),
    ]
});

static ACTION_VERBS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(use|run|execute|invoke|call|apply|generate|create|build|deploy|configure|set\s+up|install|validate|check|lint|test|format|transform|convert|parse|analyze|scan|detect|fix|migrate|upgrade|refactor|optimize|monitor|debug|log|trace|fetch|pull|push|sync|upload|download|export|import|send|render|compile|bundle|serve|start|stop|restart|reset|clean|scaffold|bootstrap|initialize|provision|authenticate|authorize)\b").unwrap()
});

static FILLER_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\bas\s+you\s+(probably\s+)?(already\s+)?know\b").unwrap(),
        Regex::new(r"(?i)\bit\s+is\s+(widely|generally|commonly)\s+known\s+that\b").unwrap(),
        Regex::new(r"(?i)\b(remember|note|keep\s+in\s+mind)\s+that\s+(all|every|most)\b").unwrap(),
        Regex::new(r"(?i)\bin\s+(today's|the\s+modern|the\s+current)\s+(world|landscape|ecosystem)\b").unwrap(),
        Regex::new(r"(?i)\bthis\s+is\s+(important|crucial|critical|essential|vital)\s+because\b").unwrap(),
        Regex::new(r"(?i)\b(best\s+practices?\s+dictate|industry\s+standard\s+is|it\s+is\s+recommended)\b").unwrap(),
        Regex::new(r"(?i)\bfor\s+more\s+(information|details),?\s+see\s+(the\s+)?(official\s+)?documentation\b").unwrap(),
    ]
});

static TODO_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(TODO|FIXME|TBD|CHANGEME|XXX)\b").unwrap()
});

static GOTCHA_HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(gotcha|pitfall|caveat|known\s+issue)").unwrap()
});

static VALIDATION_HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(validat|verif|test|check)").unwrap()
});

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Lint a parsed Skill for best-practice warnings (W001-W028) and info (I001-I016).
pub fn lint(skill: &Skill) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let desc = skill.frontmatter.description.as_deref().unwrap_or("");
    let name = skill.frontmatter.name.as_deref().unwrap_or("");

    // -- Content size warnings --
    // W001
    if skill.body.line_count > 500 {
        diagnostics.push(warn(
            "W001",
            format!("body has {} lines (>500); consider splitting into referenced files", skill.body.line_count),
            skill,
            Category::ContentEfficiency,
        ));
    }

    // W002
    if skill.body.estimated_tokens > 5000 {
        diagnostics.push(warn(
            "W002",
            format!("body has ~{} tokens (>5000); consider reducing content", skill.body.estimated_tokens),
            skill,
            Category::ContentEfficiency,
        ));
    }

    // -- Description quality --
    let has_trigger = TRIGGER_PATTERNS.iter().any(|p| p.is_match(desc));

    // W003
    if !desc.is_empty() && desc.len() < 50 {
        diagnostics.push(warn(
            "W003",
            format!("description is only {} chars (<50); consider being more descriptive", desc.len()),
            skill,
            Category::DescriptionQuality,
        ));
    }

    // W004
    if !desc.is_empty() && !has_trigger {
        diagnostics.push(warn(
            "W004",
            "description lacks trigger language (e.g. 'Use this skill when...')".into(),
            skill,
            Category::DescriptionQuality,
        ));
    }

    // W005: passive voice AND no trigger language
    if !desc.is_empty() && !has_trigger {
        let is_passive = PASSIVE_PREFIXES.iter().any(|p| p.is_match(desc));
        if is_passive {
            diagnostics.push(warn(
                "W005",
                "description starts with passive voice; lead with when/how to use this skill".into(),
                skill,
                Category::DescriptionQuality,
            ));
        }
    }

    // W007: name contains TODO/FIXME etc
    if !name.is_empty() && TODO_PATTERN.is_match(name) {
        diagnostics.push(warn(
            "W007",
            "name contains placeholder marker (TODO/FIXME/TBD/CHANGEME/XXX)".into(),
            skill,
            Category::DescriptionQuality,
        ));
    }

    // W008: description contains TODO/FIXME etc
    if !desc.is_empty() && TODO_PATTERN.is_match(desc) {
        diagnostics.push(warn(
            "W008",
            "description contains placeholder marker (TODO/FIXME/TBD/CHANGEME/XXX)".into(),
            skill,
            Category::DescriptionQuality,
        ));
    }

    // W020: vague anti-patterns
    if !desc.is_empty() && VAGUE_PATTERNS.iter().any(|p| p.is_match(desc)) {
        diagnostics.push(warn(
            "W020",
            "description matches a vague anti-pattern; be more specific".into(),
            skill,
            Category::DescriptionQuality,
        ));
    }

    // W023: lacks action verbs
    if !desc.is_empty() && !ACTION_VERBS.is_match(desc) {
        diagnostics.push(warn(
            "W023",
            "description lacks action verbs; include what the skill does".into(),
            skill,
            Category::DescriptionQuality,
        ));
    }

    // -- Body quality --
    // W006
    if skill.body.has_placeholder_text {
        diagnostics.push(warn(
            "W006",
            "body contains placeholder text".into(),
            skill,
            Category::ComposabilityClarity,
        ));
    }

    // W009
    if skill.body.headings.is_empty() {
        diagnostics.push(warn(
            "W009",
            "body has no headings; consider adding structure".into(),
            skill,
            Category::ComposabilityClarity,
        ));
    }

    // W021: filler phrases (>= 2 distinct matches)
    {
        let distinct_count = FILLER_PATTERNS
            .iter()
            .filter(|p| p.is_match(&skill.body.raw))
            .count();
        if distinct_count >= 2 {
            diagnostics.push(warn(
                "W021",
                format!("body contains {} distinct filler phrases; tighten the prose", distinct_count),
                skill,
                Category::ComposabilityClarity,
            ));
        }
    }

    // W024: reference files in scripts/references/assets dirs not mentioned in body
    check_unreferenced_files(skill, &mut diagnostics);

    // -- Info rules --
    // I001
    if !skill.file_tree.has_scripts {
        diagnostics.push(info(
            "I001",
            "no scripts/ directory found".into(),
            skill,
            Category::ScriptQuality,
        ));
    }

    // I002
    if !skill.file_tree.has_references {
        diagnostics.push(info(
            "I002",
            "no references/ directory found".into(),
            skill,
            Category::Discoverability,
        ));
    }

    // I003
    if !skill.file_tree.has_assets {
        diagnostics.push(info(
            "I003",
            "no assets/ directory found".into(),
            skill,
            Category::Discoverability,
        ));
    }

    // I004
    if !skill.file_tree.has_examples {
        diagnostics.push(info(
            "I004",
            "no examples/ directory found".into(),
            skill,
            Category::Discoverability,
        ));
    }

    // I005: no heading with gotcha/pitfall/caveat/known issue
    {
        let has_gotcha = skill
            .body
            .headings
            .iter()
            .any(|h| GOTCHA_HEADING.is_match(&h.text));
        if !has_gotcha {
            diagnostics.push(info(
                "I005",
                "no heading for gotchas/pitfalls/caveats/known issues".into(),
                skill,
                Category::Discoverability,
            ));
        }
    }

    // I006: no heading with validation/verification/test/check
    {
        let has_validation = skill
            .body
            .headings
            .iter()
            .any(|h| VALIDATION_HEADING.is_match(&h.text));
        if !has_validation {
            diagnostics.push(info(
                "I006",
                "no heading for validation/verification/testing".into(),
                skill,
                Category::Discoverability,
            ));
        }
    }

    // I012: no license field AND no LICENSE file
    if skill.frontmatter.license.is_none() {
        let has_license_file = skill.file_tree.files.iter().any(|f| {
            f.file_name()
                .map(|n| {
                    let lower = n.to_string_lossy().to_lowercase();
                    lower == "license" || lower.starts_with("license.")
                })
                .unwrap_or(false)
        });
        if !has_license_file {
            diagnostics.push(info(
                "I012",
                "no license field and no LICENSE file found".into(),
                skill,
                Category::Discoverability,
            ));
        }
    }

    // I015: description keyword diversity < 2 trigger scenarios
    if !desc.is_empty() {
        let trigger_count = TRIGGER_PATTERNS
            .iter()
            .filter(|p| p.is_match(desc))
            .count();
        if trigger_count < 2 {
            diagnostics.push(info(
                "I015",
                "description has fewer than 2 trigger scenarios; consider adding more context".into(),
                skill,
                Category::DescriptionQuality,
            ));
        }
    }

    // I016: progressive disclosure ratio = 0 (no referenced content)
    if skill.file_tree.total_content_size == 0 {
        diagnostics.push(info(
            "I016",
            "no referenced content (scripts/references/assets/examples are empty or missing)".into(),
            skill,
            Category::ContentEfficiency,
        ));
    }

    diagnostics
}

// ---------------------------------------------------------------------------
// W024: files in scripts/references/assets not mentioned in body
// ---------------------------------------------------------------------------

fn check_unreferenced_files(skill: &Skill, diagnostics: &mut Vec<Diagnostic>) {
    let subdirs = ["scripts", "references", "assets"];
    let body_lower = skill.body.raw.to_lowercase();

    for subdir in &subdirs {
        let dir_path = skill.path.join(subdir);
        if !dir_path.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    // Check if file name appears anywhere in body
                    if !body_lower.contains(&name_str.to_lowercase()) {
                        diagnostics.push(warn(
                            "W024",
                            format!("file '{}/{}' exists but is not referenced in the body", subdir, name_str),
                            skill,
                            Category::ContentEfficiency,
                        ));
                    }
                }
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
    use crate::discovery::load_skill;
    use std::fs;
    use tempfile::TempDir;

    fn make_skill_parsed(dir: &std::path::Path, content: &str) -> Skill {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
        load_skill(dir).unwrap()
    }

    fn has_rule(diags: &[Diagnostic], rule_id: &str) -> bool {
        diags.iter().any(|d| d.rule_id == rule_id)
    }

    #[test]
    fn test_w001_too_many_lines() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        // 501 lines of body content
        let body_lines = "line\n".repeat(501);
        let content = format!(
            "---\nname: my-skill\ndescription: Use this skill when you need to validate large files with action verbs like check and parse\n---\n{}",
            body_lines
        );
        let skill = make_skill_parsed(&skill_dir, &content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W001"), "expected W001, got: {diags:?}");
    }

    #[test]
    fn test_w003_short_description() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: Short desc\n---\n# Heading\nBody\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W003"), "expected W003, got: {diags:?}");
    }

    #[test]
    fn test_w004_no_trigger_language() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: This skill formats code and checks for errors in source files across the project\n---\n# Heading\nBody\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W004"), "expected W004, got: {diags:?}");
    }

    #[test]
    fn test_w004_passes_with_trigger_language() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: Use this skill when you need to format and lint source files across the project\n---\n# Heading\nBody\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(!has_rule(&diags, "W004"), "W004 should not fire, got: {diags:?}");
    }

    #[test]
    fn test_w009_no_headings() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: Use this skill when you need to validate and check files\n---\nJust plain text body with no headings at all.\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W009"), "expected W009, got: {diags:?}");
    }

    #[test]
    fn test_w005_passive_voice() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: This skill is used for formatting and checking source code across projects\n---\n# Heading\nBody\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W005"), "expected W005, got: {diags:?}");
    }

    #[test]
    fn test_w020_vague_description() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: Helps with various tasks and operations in the codebase\n---\n# Heading\nBody\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W020"), "expected W020, got: {diags:?}");
    }

    #[test]
    fn test_i001_no_scripts() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: Use this skill when you need to validate and check files\n---\n# Heading\nBody\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "I001"), "expected I001, got: {diags:?}");
    }

    #[test]
    fn test_i012_no_license() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: Use this skill when you need to validate and check files\n---\n# Heading\nBody\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "I012"), "expected I012, got: {diags:?}");
    }

    #[test]
    fn test_w021_filler_phrases() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        let content = "---\nname: my-skill\ndescription: Use this skill when you need to validate and check files\n---\n# Heading\nAs you probably know, this is important. It is widely known that best practices dictate doing things right.\n";
        let skill = make_skill_parsed(&skill_dir, content);
        let diags = lint(&skill);
        assert!(has_rule(&diags, "W021"), "expected W021, got: {diags:?}");
    }
}
