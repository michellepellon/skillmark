use super::SkillReport;
use crate::model::Severity;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VERSION: &str = "0.1.0";

fn severity_level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

pub fn format_sarif(skills: &[SkillReport]) -> String {
    // Collect all unique rules from diagnostics across all skills
    let mut rules_map: BTreeMap<String, Value> = BTreeMap::new();
    for report in skills {
        for diag in &report.diagnostics {
            rules_map.entry(diag.rule_id.clone()).or_insert_with(|| {
                json!({
                    "id": diag.rule_id,
                    "shortDescription": {
                        "text": diag.rule_id.clone()
                    }
                })
            });
        }
    }
    let rules: Vec<Value> = rules_map.into_values().collect();

    // Build all results
    let mut results: Vec<Value> = Vec::new();
    for report in skills {
        for diag in &report.diagnostics {
            let file_path = diag.path.to_str().unwrap_or("unknown");
            let artifact_location = json!({
                "uri": file_path,
                "uriBaseId": "%SRCROOT%"
            });

            let location = if let Some(span) = &diag.span {
                json!({
                    "artifactLocation": artifact_location,
                    "region": {
                        "startLine": span.start_line,
                        "startColumn": span.start_col,
                        "endLine": span.end_line,
                        "endColumn": span.end_col
                    }
                })
            } else {
                json!({
                    "artifactLocation": artifact_location
                })
            };

            results.push(json!({
                "ruleId": diag.rule_id,
                "level": severity_level(diag.severity),
                "message": {
                    "text": diag.message
                },
                "locations": [
                    {
                        "physicalLocation": location
                    }
                ]
            }));
        }
    }

    let sarif = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "skillmark",
                        "version": VERSION,
                        "rules": rules
                    }
                },
                "results": results
            }
        ]
    });

    serde_json::to_string_pretty(&sarif).expect("SARIF serialization should not fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, Diagnostic, Span};
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
                Diagnostic {
                    rule_id: "I001".into(),
                    severity: Severity::Info,
                    message: "Consider adding examples".into(),
                    path: PathBuf::from("skills/my-skill/SKILL.md"),
                    span: None,
                    fix_available: false,
                    category: Category::Discoverability,
                },
            ],
            score: None,
        }
    }

    #[test]
    fn test_sarif_output() {
        let report = make_report();
        let output = format_sarif(&[report]);

        // Parses as valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("output should be valid SARIF JSON");

        // Top-level SARIF version
        assert_eq!(parsed["version"], "2.1.0");

        // Runs array
        let runs = parsed["runs"].as_array().expect("runs should be array");
        assert_eq!(runs.len(), 1);

        let run = &runs[0];

        // Tool driver
        assert_eq!(run["tool"]["driver"]["name"], "skillmark");
        assert_eq!(run["tool"]["driver"]["version"], VERSION);

        // Rules deduped
        let rules = run["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules should be array");
        assert_eq!(rules.len(), 3, "should have 3 unique rules");
        let rule_ids: Vec<&str> = rules
            .iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect();
        assert!(rule_ids.contains(&"E030"));
        assert!(rule_ids.contains(&"W003"));
        assert!(rule_ids.contains(&"I001"));

        // Results
        let results = run["results"].as_array().expect("results should be array");
        assert_eq!(results.len(), 3);

        // First result (error with span)
        let first = &results[0];
        assert_eq!(first["ruleId"], "E030");
        assert_eq!(first["level"], "error");
        let loc = &first["locations"][0]["physicalLocation"];
        assert_eq!(loc["region"]["startLine"], 1);
        assert_eq!(loc["region"]["endColumn"], 20);

        // Second result (warning without span — no region)
        let second = &results[1];
        assert_eq!(second["ruleId"], "W003");
        assert_eq!(second["level"], "warning");
        let loc2 = &second["locations"][0]["physicalLocation"];
        assert!(loc2["region"].is_null(), "no region when span is None");

        // Third result (info → note)
        let third = &results[2];
        assert_eq!(third["level"], "note");
    }
}
