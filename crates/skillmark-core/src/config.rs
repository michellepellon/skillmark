use serde::Deserialize;
use std::path::Path;

const TIER2_RULES: &[&str] = &["W029", "W030", "W031", "W032", "I013", "I014", "E036"];

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub fail_on: FailOn,
    pub min_score: u32,
    pub format: OutputFormat,
    pub rules: RulesConfig,
    pub scoring: ScoringConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FailOn {
    Errors,
    Warnings,
    None,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Terminal,
    Json,
    Sarif,
    Markdown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RulesConfig {
    pub disable: Vec<String>,
    pub experimental: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ScoringConfig {
    pub weights: Weights,
    pub grades: Grades,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Weights {
    pub spec_compliance: f64,
    pub description_quality: f64,
    pub content_efficiency: f64,
    pub composability_clarity: f64,
    pub script_quality: f64,
    pub discoverability: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Grades {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    pub exclude: Vec<String>,
}

// --- Default implementations ---

impl Default for Config {
    fn default() -> Self {
        Self {
            fail_on: FailOn::default(),
            min_score: 0,
            format: OutputFormat::default(),
            rules: RulesConfig::default(),
            scoring: ScoringConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

impl Default for FailOn {
    fn default() -> Self {
        FailOn::Errors
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Terminal
    }
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            disable: Vec::new(),
            experimental: false,
        }
    }
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            weights: Weights::default(),
            grades: Grades::default(),
        }
    }
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            spec_compliance: 0.40,
            description_quality: 0.20,
            content_efficiency: 0.15,
            composability_clarity: 0.10,
            script_quality: 0.10,
            discoverability: 0.05,
        }
    }
}

impl Default for Grades {
    fn default() -> Self {
        Self {
            a: 90,
            b: 80,
            c: 70,
            d: 60,
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            exclude: Vec::new(),
        }
    }
}

// --- Methods ---

impl Config {
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        if self.rules.disable.iter().any(|r| r == rule_id) {
            return false;
        }
        if TIER2_RULES.contains(&rule_id) && !self.rules.experimental {
            return false;
        }
        true
    }

    pub fn load(start_dir: &Path) -> Self {
        let mut current = start_dir.to_path_buf();
        loop {
            let candidate = current.join(".skillmark.toml");
            if candidate.is_file() {
                if let Ok(contents) = std::fs::read_to_string(&candidate) {
                    if let Ok(cfg) = toml::from_str::<Config>(&contents) {
                        return cfg;
                    }
                }
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }
        Config::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.fail_on, FailOn::Errors);
        assert_eq!(cfg.min_score, 0);
        assert_eq!(cfg.format, OutputFormat::Terminal);
        assert!(cfg.rules.disable.is_empty());
        assert!(!cfg.rules.experimental);
        assert!((cfg.scoring.weights.spec_compliance - 0.40).abs() < f64::EPSILON);
        assert!((cfg.scoring.weights.description_quality - 0.20).abs() < f64::EPSILON);
        assert!((cfg.scoring.weights.content_efficiency - 0.15).abs() < f64::EPSILON);
        assert!((cfg.scoring.weights.composability_clarity - 0.10).abs() < f64::EPSILON);
        assert!((cfg.scoring.weights.script_quality - 0.10).abs() < f64::EPSILON);
        assert!((cfg.scoring.weights.discoverability - 0.05).abs() < f64::EPSILON);
        assert_eq!(cfg.scoring.grades.a, 90);
        assert_eq!(cfg.scoring.grades.b, 80);
        assert_eq!(cfg.scoring.grades.c, 70);
        assert_eq!(cfg.scoring.grades.d, 60);
        assert!(cfg.paths.exclude.is_empty());
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
fail-on = "warnings"
min-score = 75
format = "json"

[rules]
disable = ["E001", "W010"]
experimental = true

[scoring.weights]
spec-compliance = 0.35
description-quality = 0.25
content-efficiency = 0.15
composability-clarity = 0.10
script-quality = 0.10
discoverability = 0.05

[scoring.grades]
a = 95
b = 85
c = 75
d = 65

[paths]
exclude = ["vendor/**", "node_modules/**"]
"#;
        let cfg: Config = toml::from_str(toml_str).expect("should parse");
        assert_eq!(cfg.fail_on, FailOn::Warnings);
        assert_eq!(cfg.min_score, 75);
        assert_eq!(cfg.format, OutputFormat::Json);
        assert_eq!(cfg.rules.disable, vec!["E001", "W010"]);
        assert!(cfg.rules.experimental);
        assert!((cfg.scoring.weights.spec_compliance - 0.35).abs() < f64::EPSILON);
        assert_eq!(cfg.scoring.grades.a, 95);
        assert_eq!(cfg.paths.exclude, vec!["vendor/**", "node_modules/**"]);
    }

    #[test]
    fn test_is_rule_enabled() {
        let mut cfg = Config::default();
        cfg.rules.disable = vec!["E001".to_string(), "W010".to_string()];

        // Disabled rules return false
        assert!(!cfg.is_rule_enabled("E001"));
        assert!(!cfg.is_rule_enabled("W010"));

        // Tier 2 rules are disabled when experimental=false
        assert!(!cfg.is_rule_enabled("W029"));
        assert!(!cfg.is_rule_enabled("E036"));

        // Normal enabled rules return true
        assert!(cfg.is_rule_enabled("E002"));
        assert!(cfg.is_rule_enabled("W005"));
    }

    #[test]
    fn test_tier2_enabled_with_experimental() {
        let mut cfg = Config::default();
        cfg.rules.experimental = true;

        // Tier 2 rules are now enabled
        assert!(cfg.is_rule_enabled("W029"));
        assert!(cfg.is_rule_enabled("W030"));
        assert!(cfg.is_rule_enabled("W031"));
        assert!(cfg.is_rule_enabled("W032"));
        assert!(cfg.is_rule_enabled("I013"));
        assert!(cfg.is_rule_enabled("I014"));
        assert!(cfg.is_rule_enabled("E036"));
    }
}
