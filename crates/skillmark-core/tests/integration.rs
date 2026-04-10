use std::path::Path;
use skillmark_core::config::Config;
use skillmark_core::discovery::load_skill;
use skillmark_core::linter::lint;
use skillmark_core::scorer::score;
use skillmark_core::validator::validate;

#[test]
fn test_full_pipeline_valid_skill() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("tests/fixtures/valid-skill");

    // Validate
    let errors = validate(&fixture);
    let error_count = errors.iter()
        .filter(|d| d.severity == skillmark_core::model::Severity::Error)
        .count();
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
