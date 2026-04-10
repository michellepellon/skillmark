use serde::Serialize;
use super::SkillReport;
use crate::model::{Category, Grade, Severity, Span};

const VERSION: &str = "0.1.0";

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn category_str(c: Category) -> &'static str {
    match c {
        Category::SpecCompliance => "spec_compliance",
        Category::DescriptionQuality => "description_quality",
        Category::ContentEfficiency => "content_efficiency",
        Category::ComposabilityClarity => "composability_clarity",
        Category::ScriptQuality => "script_quality",
        Category::Discoverability => "discoverability",
    }
}

fn grade_str(g: Grade) -> &'static str {
    match g {
        Grade::A => "A",
        Grade::B => "B",
        Grade::C => "C",
        Grade::D => "D",
        Grade::F => "F",
    }
}

#[derive(Serialize)]
struct JsonOutput<'a> {
    version: &'static str,
    skills: Vec<JsonSkill<'a>>,
}

#[derive(Serialize)]
struct JsonSkill<'a> {
    path: &'a str,
    score: Option<JsonScore<'a>>,
    diagnostics: Vec<JsonDiagnostic<'a>>,
    summary: JsonSummary,
}

#[derive(Serialize)]
struct JsonScore<'a> {
    composite: i64,
    grade: &'static str,
    categories: Vec<JsonCategory<'a>>,
}

#[derive(Serialize)]
struct JsonCategory<'a> {
    name: &'a str,
    weight: f64,
    score: f64,
    weighted_score: f64,
    rule_results: Vec<JsonRuleResult<'a>>,
}

#[derive(Serialize)]
struct JsonRuleResult<'a> {
    rule_id: &'a str,
    passed: bool,
}

#[derive(Serialize)]
struct JsonDiagnostic<'a> {
    rule_id: &'a str,
    severity: &'static str,
    message: &'a str,
    path: &'a str,
    span: Option<JsonSpan>,
    fix_available: bool,
    category: &'static str,
}

#[derive(Serialize)]
struct JsonSpan {
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

impl From<&Span> for JsonSpan {
    fn from(s: &Span) -> Self {
        JsonSpan {
            start_line: s.start_line,
            start_col: s.start_col,
            end_line: s.end_line,
            end_col: s.end_col,
        }
    }
}

#[derive(Serialize)]
struct JsonSummary {
    errors: usize,
    warnings: usize,
    info: usize,
}

pub fn format_json(skills: &[SkillReport]) -> String {
    let json_skills: Vec<JsonSkill<'_>> = skills
        .iter()
        .map(|report| {
            let path_str = report.path.to_str().unwrap_or("unknown");

            let score = report.score.as_ref().map(|sc| JsonScore {
                composite: sc.composite.round() as i64,
                grade: grade_str(sc.grade),
                categories: sc
                    .categories
                    .iter()
                    .map(|cat| JsonCategory {
                        name: cat.name.as_str(),
                        weight: cat.weight,
                        score: cat.score,
                        weighted_score: cat.weighted_score,
                        rule_results: cat
                            .rule_results
                            .iter()
                            .map(|rr| JsonRuleResult {
                                rule_id: rr.rule_id.as_str(),
                                passed: rr.passed,
                            })
                            .collect(),
                    })
                    .collect(),
            });

            let diagnostics = report
                .diagnostics
                .iter()
                .map(|d| JsonDiagnostic {
                    rule_id: d.rule_id.as_str(),
                    severity: severity_str(d.severity),
                    message: d.message.as_str(),
                    path: d.path.to_str().unwrap_or("unknown"),
                    span: d.span.as_ref().map(JsonSpan::from),
                    fix_available: d.fix_available,
                    category: category_str(d.category),
                })
                .collect();

            let errors = report
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            let warnings = report
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Warning)
                .count();
            let info = report
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Info)
                .count();

            JsonSkill {
                path: path_str,
                score,
                diagnostics,
                summary: JsonSummary { errors, warnings, info },
            }
        })
        .collect();

    let output = JsonOutput {
        version: VERSION,
        skills: json_skills,
    };

    serde_json::to_string_pretty(&output).expect("JSON serialization should not fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CategoryScore, Diagnostic, Grade, RuleResult, ScoreCard, Span};
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
    fn test_json_output() {
        let report = make_report();
        let output = format_json(&[report]);

        // Parses as valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("output should be valid JSON");

        // Top-level version
        assert_eq!(parsed["version"], "0.1.0");

        // Skills array
        let skills = parsed["skills"].as_array().expect("skills should be array");
        assert_eq!(skills.len(), 1);

        let skill = &skills[0];
        assert_eq!(skill["path"], "skills/my-skill");

        // Score block
        let score = &skill["score"];
        assert_eq!(score["composite"], 75);
        assert_eq!(score["grade"], "C");

        let cats = score["categories"].as_array().expect("categories should be array");
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0]["name"], "spec_compliance");

        // Diagnostics
        let diags = skill["diagnostics"].as_array().expect("diagnostics should be array");
        assert_eq!(diags.len(), 2);

        let first = &diags[0];
        assert_eq!(first["rule_id"], "E030");
        assert_eq!(first["severity"], "error");
        assert_eq!(first["category"], "spec_compliance");
        assert_eq!(first["span"]["start_line"], 1);
        assert_eq!(first["span"]["end_col"], 20);

        let second = &diags[1];
        assert_eq!(second["rule_id"], "W003");
        assert_eq!(second["severity"], "warning");
        assert!(second["span"].is_null(), "span should be null when None");

        // Summary
        let summary = &skill["summary"];
        assert_eq!(summary["errors"], 1);
        assert_eq!(summary["warnings"], 1);
        assert_eq!(summary["info"], 0);
    }
}
