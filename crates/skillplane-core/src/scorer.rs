use std::collections::HashSet;

use crate::config::Config;
use crate::model::{CategoryScore, Diagnostic, Grade, RuleResult, ScoreCard};

// ---------------------------------------------------------------------------
// Category rule sets
// ---------------------------------------------------------------------------

const SPEC_COMPLIANCE_RULES: &[&str] = &[
    "E001", "E002", "E003", "E004", "E005", "E006", "E007", "E008", "E009", "E010", "E011",
    "E012", "E013", "E014", "E015", "E016", "E017", "E018", "E019", "E020", "E021", "E022",
    "E023", "E024", "E025", "E026", "E027", "E028", "E029", "E030", "E031", "E032", "E033",
    "E034", "E035",
];

const DESCRIPTION_QUALITY_RULES: &[&str] = &["W003", "W004", "W005", "W020", "W023", "I015"];

const CONTENT_EFFICIENCY_RULES: &[&str] = &["W001", "W002", "I016", "W024", "W025"];

const COMPOSABILITY_CLARITY_RULES_BASE: &[&str] = &["W021", "W009", "W006"];
const COMPOSABILITY_CLARITY_EXPERIMENTAL: &str = "W031";

const SCRIPT_QUALITY_RULES: &[&str] = &["W026", "W027", "W028", "W018", "I009"];

const DISCOVERABILITY_RULES: &[&str] = &["I012", "I002", "I005", "I006"];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute a quality scorecard from diagnostics using weighted category scoring.
pub fn score(diagnostics: &[Diagnostic], has_scripts: bool, config: &Config) -> ScoreCard {
    let fired: HashSet<&str> = diagnostics.iter().map(|d| d.rule_id.as_str()).collect();

    let mut categories = Vec::with_capacity(6);

    // 1. Spec Compliance (weight 0.40)
    categories.push(score_category(
        "Spec Compliance",
        config.scoring.weights.spec_compliance,
        SPEC_COMPLIANCE_RULES,
        &fired,
        config,
    ));

    // 2. Description Quality (weight 0.20)
    categories.push(score_category(
        "Description Quality",
        config.scoring.weights.description_quality,
        DESCRIPTION_QUALITY_RULES,
        &fired,
        config,
    ));

    // 3. Content Efficiency (weight 0.15)
    categories.push(score_category(
        "Content Efficiency",
        config.scoring.weights.content_efficiency,
        CONTENT_EFFICIENCY_RULES,
        &fired,
        config,
    ));

    // 4. Composability & Clarity (weight 0.10) — add W031 if experimental enabled
    let mut composability_rules: Vec<&str> = COMPOSABILITY_CLARITY_RULES_BASE.to_vec();
    if config.rules.experimental {
        composability_rules.push(COMPOSABILITY_CLARITY_EXPERIMENTAL);
    }
    categories.push(score_category(
        "Composability & Clarity",
        config.scoring.weights.composability_clarity,
        &composability_rules,
        &fired,
        config,
    ));

    // 5. Script Quality (weight 0.10) — 100% automatically if has_scripts == false
    if !has_scripts {
        let weight = config.scoring.weights.script_quality;
        let rule_results = SCRIPT_QUALITY_RULES
            .iter()
            .filter(|&&id| config.is_rule_enabled(id))
            .map(|&id| RuleResult {
                rule_id: id.to_string(),
                passed: true,
            })
            .collect();
        categories.push(CategoryScore {
            name: "Script Quality".to_string(),
            weight,
            score: 100.0,
            weighted_score: 100.0 * weight,
            rule_results,
        });
    } else {
        categories.push(score_category(
            "Script Quality",
            config.scoring.weights.script_quality,
            SCRIPT_QUALITY_RULES,
            &fired,
            config,
        ));
    }

    // 6. Discoverability (weight 0.05)
    categories.push(score_category(
        "Discoverability",
        config.scoring.weights.discoverability,
        DISCOVERABILITY_RULES,
        &fired,
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

/// Derive a letter grade from a composite score using config boundaries.
pub fn grade_from_score(composite: f64, config: &Config) -> Grade {
    let rounded = composite.round() as u32;
    let g = &config.scoring.grades;
    if rounded >= g.a {
        Grade::A
    } else if rounded >= g.b {
        Grade::B
    } else if rounded >= g.c {
        Grade::C
    } else if rounded >= g.d {
        Grade::D
    } else {
        Grade::F
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn score_category(
    name: &str,
    weight: f64,
    rules: &[&str],
    fired: &HashSet<&str>,
    config: &Config,
) -> CategoryScore {
    let mut passing = 0usize;
    let mut applicable = 0usize;
    let mut rule_results = Vec::with_capacity(rules.len());

    for &rule_id in rules {
        if !config.is_rule_enabled(rule_id) {
            continue;
        }
        applicable += 1;
        let passed = !fired.contains(rule_id);
        if passed {
            passing += 1;
        }
        rule_results.push(RuleResult {
            rule_id: rule_id.to_string(),
            passed,
        });
    }

    let cat_score = if applicable == 0 {
        100.0
    } else {
        (passing as f64 / applicable as f64) * 100.0
    };

    CategoryScore {
        name: name.to_string(),
        weight,
        score: cat_score,
        weighted_score: cat_score * weight,
        rule_results,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, Diagnostic, Grade, Severity};
    use std::path::PathBuf;

    fn dummy_diagnostic(rule_id: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.to_string(),
            severity: Severity::Error,
            message: "test".to_string(),
            path: PathBuf::from("SKILL.md"),
            span: None,
            fix_available: false,
            category: Category::SpecCompliance,
        }
    }

    #[test]
    fn test_perfect_score() {
        let config = Config::default();
        let scorecard = score(&[], true, &config);
        assert!(
            (scorecard.composite - 100.0).abs() < 0.01,
            "composite should be 100, got {}",
            scorecard.composite
        );
        assert_eq!(scorecard.grade, Grade::A);
    }

    #[test]
    fn test_spec_compliance_scoring() {
        let config = Config::default();
        // Fire 2 E-rules → 33 passing out of 35 = 94.285...%
        // weighted: 94.285... * 0.40 ≈ 37.714...
        let diagnostics = vec![dummy_diagnostic("E001"), dummy_diagnostic("E002")];
        let scorecard = score(&diagnostics, true, &config);

        let spec_cat = scorecard
            .categories
            .iter()
            .find(|c| c.name == "Spec Compliance")
            .expect("Spec Compliance category must exist");

        let expected_score = (33.0 / 35.0) * 100.0;
        assert!(
            (spec_cat.score - expected_score).abs() < 0.1,
            "spec score should be ~{:.2}, got {:.2}",
            expected_score,
            spec_cat.score
        );

        let expected_weighted = expected_score * 0.40;
        assert!(
            (spec_cat.weighted_score - expected_weighted).abs() < 0.1,
            "weighted spec score should be ~{:.2}, got {:.2}",
            expected_weighted,
            spec_cat.weighted_score
        );
    }

    #[test]
    fn test_scripts_absent_full_score() {
        let config = Config::default();
        // Fire a script rule — but has_scripts=false so category is auto 100%
        let diagnostics = vec![dummy_diagnostic("W026")];
        let scorecard = score(&diagnostics, false, &config);

        let script_cat = scorecard
            .categories
            .iter()
            .find(|c| c.name == "Script Quality")
            .expect("Script Quality category must exist");

        assert!(
            (script_cat.score - 100.0).abs() < 0.01,
            "script quality score should be 100, got {}",
            script_cat.score
        );
        assert!(
            script_cat.rule_results.iter().all(|r| r.passed),
            "all rule results should be passed when no scripts"
        );
    }

    #[test]
    fn test_grade_boundaries() {
        let config = Config::default();

        // A >= 90
        assert_eq!(grade_from_score(90.0, &config), Grade::A);
        assert_eq!(grade_from_score(100.0, &config), Grade::A);
        assert_eq!(grade_from_score(89.4, &config), Grade::B); // rounds to 89

        // B >= 80
        assert_eq!(grade_from_score(80.0, &config), Grade::B);
        assert_eq!(grade_from_score(89.0, &config), Grade::B);
        assert_eq!(grade_from_score(79.4, &config), Grade::C); // rounds to 79

        // C >= 70
        assert_eq!(grade_from_score(70.0, &config), Grade::C);
        assert_eq!(grade_from_score(79.0, &config), Grade::C);
        assert_eq!(grade_from_score(69.4, &config), Grade::D); // rounds to 69

        // D >= 60
        assert_eq!(grade_from_score(60.0, &config), Grade::D);
        assert_eq!(grade_from_score(69.0, &config), Grade::D);
        assert_eq!(grade_from_score(59.4, &config), Grade::F); // rounds to 59

        // F < 60
        assert_eq!(grade_from_score(0.0, &config), Grade::F);
        assert_eq!(grade_from_score(59.0, &config), Grade::F);
    }
}
