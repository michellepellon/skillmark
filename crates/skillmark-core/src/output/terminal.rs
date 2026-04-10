use super::SkillReport;
use crate::model::{Grade, Severity};

const BAR_WIDTH: usize = 20;
const VERSION: &str = "v0.1.0";

fn grade_str(grade: Grade) -> &'static str {
    match grade {
        Grade::A => "A",
        Grade::B => "B",
        Grade::C => "C",
        Grade::D => "D",
        Grade::F => "F",
    }
}

fn progress_bar(pct: f64) -> String {
    let filled = ((pct / 100.0) * BAR_WIDTH as f64).round() as usize;
    let filled = filled.min(BAR_WIDTH);
    let empty = BAR_WIDTH - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

pub fn format_terminal(skills: &[SkillReport], quiet: bool) -> String {
    let mut out = String::new();

    for report in skills {
        let skill_dir_name = report
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if !quiet {
            // Header
            out.push_str(&format!("skillmark {} — {}\n\n", VERSION, skill_dir_name));

            // Score block
            if let Some(score) = &report.score {
                let composite_rounded = score.composite.round() as i64;
                out.push_str(&format!(
                    "  Score: {}/100 ({})\n\n",
                    composite_rounded,
                    grade_str(score.grade)
                ));

                // Category rows
                let category_labels = [
                    "Spec Compliance",
                    "Description",
                    "Content Efficiency",
                    "Composability",
                    "Script Quality",
                    "Discoverability",
                ];

                for (cat, label) in score.categories.iter().zip(category_labels.iter()) {
                    let max_weighted = cat.weight * 100.0;
                    let pct = if max_weighted > 0.0 {
                        (cat.weighted_score / max_weighted * 100.0).min(100.0)
                    } else {
                        0.0
                    };
                    let bar = progress_bar(pct);
                    out.push_str(&format!(
                        "  {:<18} {}  {:.0}%  ({:.1}/{:.1})\n",
                        label,
                        bar,
                        pct,
                        cat.weighted_score,
                        max_weighted,
                    ));
                }
                out.push('\n');
            }
        }

        // Collect diagnostics: errors first (sorted by rule_id), then warnings (sorted by rule_id)
        // Skip Info severity.
        let mut errors: Vec<&crate::model::Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        errors.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));

        let mut warnings: Vec<&crate::model::Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        warnings.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));

        let ordered: Vec<&crate::model::Diagnostic> =
            errors.iter().chain(warnings.iter()).copied().collect();

        if !quiet {
            let warning_count = warnings.len();
            let error_count = errors.len();
            out.push_str(&format!(
                "  {} warnings, {} errors\n\n",
                warning_count, error_count
            ));
        }

        // Diagnostic lines
        for diag in &ordered {
            let location = match &diag.span {
                Some(span) => {
                    let filename = diag
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    format!("{}:{}", filename, span.start_line)
                }
                None => "—".to_string(),
            };
            out.push_str(&format!(
                "  {}  {}  {}\n",
                diag.rule_id, location, diag.message
            ));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CategoryScore, Diagnostic, Grade, RuleResult, ScoreCard, Severity, Category, Span,
    };
    use crate::output::SkillReport;
    use std::path::PathBuf;

    fn make_score() -> ScoreCard {
        let categories = vec![
            CategoryScore {
                name: "Spec Compliance".into(),
                weight: 0.40,
                score: 80.0,
                weighted_score: 32.0,
                rule_results: vec![RuleResult { rule_id: "SP001".into(), passed: true }],
            },
            CategoryScore {
                name: "Description".into(),
                weight: 0.20,
                score: 75.0,
                weighted_score: 15.0,
                rule_results: vec![],
            },
            CategoryScore {
                name: "Content Efficiency".into(),
                weight: 0.10,
                score: 90.0,
                weighted_score: 9.0,
                rule_results: vec![],
            },
            CategoryScore {
                name: "Composability".into(),
                weight: 0.10,
                score: 60.0,
                weighted_score: 6.0,
                rule_results: vec![],
            },
            CategoryScore {
                name: "Script Quality".into(),
                weight: 0.10,
                score: 100.0,
                weighted_score: 10.0,
                rule_results: vec![],
            },
            CategoryScore {
                name: "Discoverability".into(),
                weight: 0.10,
                score: 50.0,
                weighted_score: 5.0,
                rule_results: vec![],
            },
        ];
        ScoreCard {
            composite: 77.0,
            categories,
            grade: Grade::C,
        }
    }

    fn make_diagnostics() -> Vec<Diagnostic> {
        vec![
            Diagnostic {
                rule_id: "SP001".into(),
                severity: Severity::Error,
                message: "Missing required field".into(),
                path: PathBuf::from("SKILL.md"),
                span: Some(Span { start_line: 3, start_col: 1, end_line: 3, end_col: 10 }),
                fix_available: false,
                category: Category::SpecCompliance,
            },
            Diagnostic {
                rule_id: "DQ001".into(),
                severity: Severity::Warning,
                message: "Description too short".into(),
                path: PathBuf::from("SKILL.md"),
                span: None,
                fix_available: false,
                category: Category::DescriptionQuality,
            },
            Diagnostic {
                rule_id: "INFO001".into(),
                severity: Severity::Info,
                message: "This is an info message".into(),
                path: PathBuf::from("SKILL.md"),
                span: None,
                fix_available: false,
                category: Category::SpecCompliance,
            },
        ]
    }

    #[test]
    fn test_terminal_format_with_score() {
        let report = SkillReport {
            path: PathBuf::from("/skills/my-skill"),
            diagnostics: make_diagnostics(),
            score: Some(make_score()),
        };

        let output = format_terminal(&[report], false);

        // Header present
        assert!(output.contains("skillmark v0.1.0 — my-skill"), "missing header");

        // Score line present
        assert!(output.contains("Score: 77/100 (C)"), "missing score line");

        // Error diagnostic present with location
        assert!(output.contains("SP001"), "missing SP001 rule_id");
        assert!(output.contains("SKILL.md:3"), "missing file:line location");
        assert!(output.contains("Missing required field"), "missing error message");

        // Warning diagnostic present with dash location
        assert!(output.contains("DQ001"), "missing DQ001 rule_id");
        assert!(output.contains("—"), "missing dash for no-span");
        assert!(output.contains("Description too short"), "missing warning message");

        // Info diagnostic skipped
        assert!(!output.contains("INFO001"), "info diagnostic should be skipped");

        // Warning/error summary present
        assert!(output.contains("1 warnings, 1 errors"), "missing summary line");

        // Errors come before warnings
        let error_pos = output.find("SP001").unwrap();
        let warning_pos = output.find("DQ001").unwrap();
        assert!(error_pos < warning_pos, "errors should come before warnings");
    }

    #[test]
    fn test_terminal_format_quiet() {
        let report = SkillReport {
            path: PathBuf::from("/skills/my-skill"),
            diagnostics: make_diagnostics(),
            score: Some(make_score()),
        };

        let output = format_terminal(&[report], true);

        // No score line
        assert!(!output.contains("Score:"), "quiet mode should not show score");

        // No header
        assert!(!output.contains("skillmark"), "quiet mode should not show header");

        // No summary line
        assert!(!output.contains("warnings,"), "quiet mode should not show summary");

        // Diagnostics still present
        assert!(output.contains("SP001"), "error diagnostic should still appear");
        assert!(output.contains("DQ001"), "warning diagnostic should still appear");

        // Info still skipped
        assert!(!output.contains("INFO001"), "info diagnostic should be skipped in quiet mode");
    }
}
