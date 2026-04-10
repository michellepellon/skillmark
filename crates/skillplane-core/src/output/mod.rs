pub mod terminal;
pub mod json;
pub mod sarif;
pub mod markdown;

use std::path::PathBuf;
use crate::model::{Diagnostic, ScoreCard};

/// A complete report for one skill, ready for formatting.
pub struct SkillReport {
    pub path: PathBuf,
    pub diagnostics: Vec<Diagnostic>,
    pub score: Option<ScoreCard>,  // None when --no-score
}
