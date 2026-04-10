use super::SkillReport;
use crate::model::Severity;

pub fn format_markdown(skills: &[SkillReport]) -> String {
    let mut out = String::new();

    for report in skills {
        let skill_name = report
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        out.push_str(&format!("## Skillplane Report — {}\n\n", skill_name));

        // Score section
        if let Some(score) = &report.score {
            let composite_rounded = score.composite.round() as i64;
            out.push_str(&format!(
                "**Score: {}/100 ({})**\n\n",
                composite_rounded,
                grade_str(score.grade)
            ));

            // Category table
            out.push_str("| Category | Score | Weighted |\n");
            out.push_str("|----------|-------|----------|\n");
            for cat in &score.categories {
                let max_weighted = cat.weight * 100.0;
                out.push_str(&format!(
                    "| {} | {:.0}% | {:.1}/{:.1} |\n",
                    display_category_name(&cat.name),
                    cat.score,
                    cat.weighted_score,
                    max_weighted,
                ));
            }
            out.push('\n');
        }

        // Collect diagnostics by severity
        let errors: Vec<&crate::model::Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();

        let warnings: Vec<&crate::model::Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();

        let info: Vec<&crate::model::Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Info)
            .collect();

        // Errors section
        if !errors.is_empty() {
            out.push_str(&format!("### Errors ({})\n\n", errors.len()));
            out.push_str("| Rule | Location | Message |\n");
            out.push_str("|------|----------|---------|\n");
            for diag in &errors {
                let location = diag_location(diag);
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    diag.rule_id, location, diag.message
                ));
            }
            out.push('\n');
        }

        // Warnings section
        if !warnings.is_empty() {
            out.push_str(&format!("### Warnings ({})\n\n", warnings.len()));
            out.push_str("| Rule | Location | Message |\n");
            out.push_str("|------|----------|---------|\n");
            for diag in &warnings {
                let location = diag_location(diag);
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    diag.rule_id, location, diag.message
                ));
            }
            out.push('\n');
        }

        // Info section
        if !info.is_empty() {
            out.push_str(&format!("### Info ({})\n\n", info.len()));
            out.push_str("| Rule | Location | Message |\n");
            out.push_str("|------|----------|---------|\n");
            for diag in &info {
                let location = diag_location(diag);
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    diag.rule_id, location, diag.message
                ));
            }
            out.push('\n');
        }
    }

    out
}

fn diag_location(diag: &crate::model::Diagnostic) -> String {
    let filename = diag
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    match &diag.span {
        Some(span) => format!("`{}:{}`", filename, span.start_line),
        None => format!("`{}`", filename),
    }
}

fn grade_str(g: crate::model::Grade) -> &'static str {
    match g {
        crate::model::Grade::A => "A",
        crate::model::Grade::B => "B",
        crate::model::Grade::C => "C",
        crate::model::Grade::D => "D",
        crate::model::Grade::F => "F",
    }
}

/// Convert snake_case or internal category names to a display-friendly form.
fn display_category_name(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, CategoryScore, Diagnostic, Grade, RuleResult, ScoreCard, Span};
    use crate::output::SkillReport;
    use std::path::PathBuf;

    fn make_report() -> SkillReport {
        SkillReport {
            path: PathBuf::from("skills/my-skill"),
            diagnostics: vec![
                Diagnostic {
                    rule_id: "E030".into(),
                    severity: Severity::Error,
                    message: "Missing required section".into(),
                    path: PathBuf::from("skills/my-skill/SKILL.md"),
                    span: Some(Span {
                        start_line: 1,
                        start_col: 1,
                        end_line: 1,
                        end_col: 20,
                    }),
                    fix_available: false,
                    category: Category::SpecCompliance,
                },
                Diagnostic {
                    rule_id: "W003".into(),
                    severity: Severity::Warning,
                    message: "Description could be longer".into(),
                    path: PathBuf::from("skills/my-skill/SKILL.md"),
                    span: None,
                    fix_available: true,
                    category: Category::DescriptionQuality,
                },
            ],
            score: Some(ScoreCard {
                composite: 75.4,
                grade: Grade::C,
                categories: vec![CategoryScore {
                    name: "spec_compliance".into(),
                    weight: 0.40,
                    score: 94.3,
                    weighted_score: 37.7,
                    rule_results: vec![RuleResult {
                        rule_id: "E001".into(),
                        passed: true,
                    }],
                }],
            }),
        }
    }

    #[test]
    fn test_markdown_output() {
        let report = make_report();
        let output = format_markdown(&[report]);

        // Report header
        assert!(
            output.contains("## Skillplane Report — my-skill"),
            "missing report header"
        );

        // Score line
        assert!(
            output.contains("**Score: 75/100 (C)**"),
            "missing score line"
        );

        // Category table header
        assert!(output.contains("| Category | Score | Weighted |"), "missing table header");
        assert!(output.contains("|----------|-------|----------|"), "missing table separator");

        // Category row — "spec_compliance" becomes "Spec Compliance"
        assert!(
            output.contains("| Spec Compliance |"),
            "missing category row"
        );

        // Errors section header
        assert!(output.contains("### Errors (1)"), "missing errors header");

        // Error row with location
        assert!(output.contains("E030"), "missing E030 rule id");
        assert!(output.contains("`SKILL.md:1`"), "missing error location");
        assert!(output.contains("Missing required section"), "missing error message");

        // Warnings section header
        assert!(output.contains("### Warnings (1)"), "missing warnings header");

        // Warning row
        assert!(output.contains("W003"), "missing W003 rule id");
        assert!(output.contains("Description could be longer"), "missing warning message");

        // No info section (none present)
        assert!(!output.contains("### Info"), "info section should be absent");
    }
}
