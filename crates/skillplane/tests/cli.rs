use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

// ---------------------------------------------------------------------------
// test_check_valid_skill
// ---------------------------------------------------------------------------

#[test]
fn test_check_valid_skill() {
    let fixture = fixture_path("valid-skill");

    let mut cmd = Command::cargo_bin("skillplane").unwrap();
    cmd.args(["check", fixture.to_str().unwrap()]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Score:"))
        .stdout(predicate::str::contains("valid-skill"));
}

// ---------------------------------------------------------------------------
// test_check_missing_skill
// ---------------------------------------------------------------------------

#[test]
fn test_check_missing_skill() {
    let tmp = TempDir::new().unwrap();
    let empty_dir = tmp.path().join("empty");
    fs::create_dir_all(&empty_dir).unwrap();

    let mut cmd = Command::cargo_bin("skillplane").unwrap();
    cmd.args(["check", empty_dir.to_str().unwrap()]);

    cmd.assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("E001"));
}

// ---------------------------------------------------------------------------
// test_check_json_format
// ---------------------------------------------------------------------------

#[test]
fn test_check_json_format() {
    let fixture = fixture_path("valid-skill");

    let mut cmd = Command::cargo_bin("skillplane").unwrap();
    cmd.args(["check", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(
        stdout.contains("\"version\""),
        "stdout did not contain '\"version\"': {stdout}"
    );
    assert!(
        stdout.contains("\"skills\""),
        "stdout did not contain '\"skills\"': {stdout}"
    );

    // Verify it parses as valid JSON
    let _parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout is not valid JSON");
}

// ---------------------------------------------------------------------------
// test_check_quiet_mode
// ---------------------------------------------------------------------------

#[test]
fn test_check_quiet_mode() {
    let fixture = fixture_path("valid-skill");

    let mut cmd = Command::cargo_bin("skillplane").unwrap();
    cmd.args(["check", fixture.to_str().unwrap(), "--quiet"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Score:").not());
}

// ---------------------------------------------------------------------------
// test_check_no_score
// ---------------------------------------------------------------------------

#[test]
fn test_check_no_score() {
    let fixture = fixture_path("valid-skill");

    let mut cmd = Command::cargo_bin("skillplane").unwrap();
    cmd.args(["check", fixture.to_str().unwrap(), "--no-score"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Score:").not());
}

// ---------------------------------------------------------------------------
// test_check_min_score_pass
// ---------------------------------------------------------------------------

#[test]
fn test_check_min_score_pass() {
    let fixture = fixture_path("valid-skill");

    let mut cmd = Command::cargo_bin("skillplane").unwrap();
    cmd.args(["check", fixture.to_str().unwrap(), "--min-score", "50"]);

    cmd.assert().success();
}

// ---------------------------------------------------------------------------
// test_check_min_score_fail
// ---------------------------------------------------------------------------

#[test]
fn test_check_min_score_fail() {
    let fixture = fixture_path("valid-skill");

    let mut cmd = Command::cargo_bin("skillplane").unwrap();
    cmd.args(["check", fixture.to_str().unwrap(), "--min-score", "100"]);

    cmd.assert().failure().code(2);
}

// ---------------------------------------------------------------------------
// test_fix_dry_run
// ---------------------------------------------------------------------------

#[test]
fn test_fix_dry_run() {
    let tmp = TempDir::new().unwrap();
    let skill_dir = tmp.path().join("bad-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    // Create a skill.md (wrong case, no frontmatter) — fixer should want to rename it
    let skill_md = skill_dir.join("skill.md");
    let original_content = "# Bad Skill\n\nNo frontmatter here.\n";
    fs::write(&skill_md, original_content).unwrap();

    let mut cmd = Command::cargo_bin("skillplane").unwrap();
    cmd.args(["fix", skill_dir.to_str().unwrap(), "--dry-run"]);

    cmd.assert()
        .stdout(predicate::str::contains("dry-run").or(predicate::str::contains("dry_run")));

    // Original file must be unchanged
    let on_disk = fs::read_to_string(&skill_md).unwrap();
    assert_eq!(
        on_disk, original_content,
        "dry-run must not modify the file on disk"
    );
}
