use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use colored::control::set_override;

use skillplane_core::config::{Config, FailOn, OutputFormat};
use skillplane_core::discovery::{discover_skills, load_skill};
use skillplane_core::linter::lint;
use skillplane_core::model::{Diagnostic, Severity};
use skillplane_core::output::json::format_json;
use skillplane_core::output::markdown::format_markdown;
use skillplane_core::output::sarif::format_sarif;
use skillplane_core::output::terminal::format_terminal;
use skillplane_core::output::SkillReport;
use skillplane_core::scorer::score;
use skillplane_core::validator::validate;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "skillplane",
    about = "Agent Skills linter, validator, and scorer",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format: terminal|json|sarif|markdown
    #[arg(long, default_value = "terminal", global = true)]
    format: Format,

    /// Exit non-zero on: errors|warnings|none
    #[arg(long, default_value = "errors", global = true)]
    fail_on: FailOnArg,

    /// Minimum composite score (0-100)
    #[arg(long, global = true)]
    min_score: Option<u32>,

    /// Skip scoring
    #[arg(long, global = true)]
    no_score: bool,

    /// Only output diagnostics
    #[arg(long, global = true)]
    quiet: bool,

    /// Color: auto|always|never
    #[arg(long, default_value = "auto", global = true)]
    color: ColorWhen,

    /// Comma-separated rule IDs to disable
    #[arg(long, global = true, value_delimiter = ',')]
    disable: Vec<String>,

    /// Enable Tier 2 rules
    #[arg(long, global = true)]
    experimental: bool,

    /// Comma-separated paths to exclude
    #[arg(long, global = true, value_delimiter = ',')]
    exclude: Vec<String>,

    /// Path to .skillplane.toml
    #[arg(long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate, lint, and score skills (default)
    Check {
        /// Paths to check (default: auto-discover)
        paths: Vec<PathBuf>,

        /// Also run fix mode
        #[arg(long)]
        fix: bool,
    },
    /// Auto-repair fixable issues
    Fix {
        /// Paths to fix
        paths: Vec<PathBuf>,

        /// Preview fixes without writing
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Clone, ValueEnum)]
enum Format {
    Terminal,
    Json,
    Sarif,
    Markdown,
}

#[derive(Clone, ValueEnum)]
enum FailOnArg {
    Errors,
    Warnings,
    None,
}

#[derive(Clone, ValueEnum)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

// ---------------------------------------------------------------------------
// Config merging
// ---------------------------------------------------------------------------

fn merge_cli_into_config(cli: &Cli, mut config: Config) -> Config {
    // --disable appends to config.rules.disable
    for rule in &cli.disable {
        if !config.rules.disable.contains(rule) {
            config.rules.disable.push(rule.clone());
        }
    }

    // --experimental sets config.rules.experimental = true
    if cli.experimental {
        config.rules.experimental = true;
    }

    // --fail-on overrides config.fail_on
    config.fail_on = match cli.fail_on {
        FailOnArg::Errors => FailOn::Errors,
        FailOnArg::Warnings => FailOn::Warnings,
        FailOnArg::None => FailOn::None,
    };

    // --min-score overrides config.min_score
    if let Some(min) = cli.min_score {
        config.min_score = min;
    }

    // --format overrides config.format
    config.format = match cli.format {
        Format::Terminal => OutputFormat::Terminal,
        Format::Json => OutputFormat::Json,
        Format::Sarif => OutputFormat::Sarif,
        Format::Markdown => OutputFormat::Markdown,
    };

    // --exclude appends to config.paths.exclude
    for glob in &cli.exclude {
        if !config.paths.exclude.contains(glob) {
            config.paths.exclude.push(glob.clone());
        }
    }

    config
}

// ---------------------------------------------------------------------------
// Skill discovery
// ---------------------------------------------------------------------------

fn resolve_skill_dirs(paths: &[PathBuf]) -> Vec<PathBuf> {
    if paths.is_empty() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        discover_skills(&cwd)
    } else {
        let mut dirs = Vec::new();
        for p in paths {
            let path = if p.is_relative() {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(p)
            } else {
                p.clone()
            };

            if path.is_dir() {
                // Could be a skill dir itself or a parent containing skills
                let discovered = discover_skills(&path);
                if discovered.is_empty() {
                    // Treat as a skill dir directly
                    dirs.push(path);
                } else {
                    dirs.extend(discovered);
                }
            } else {
                dirs.push(path);
            }
        }
        dirs.sort();
        dirs.dedup();
        dirs
    }
}

// ---------------------------------------------------------------------------
// Check pipeline
// ---------------------------------------------------------------------------

fn run_check(paths: &[PathBuf], config: &Config, no_score: bool, _quiet: bool) -> Vec<SkillReport> {
    let skill_dirs = resolve_skill_dirs(paths);
    let mut reports = Vec::new();

    for skill_dir in &skill_dirs {
        let mut diagnostics: Vec<Diagnostic> = Vec::new();

        // Step a: Run validator -> collect E-rule diagnostics
        let validation_diags = validate(skill_dir);
        let has_parse_error = validation_diags.iter().any(|d| {
            matches!(
                d.rule_id.as_str(),
                "E001" | "E003" | "E004" | "E032" | "E033"
            )
        });
        diagnostics.extend(
            validation_diags
                .into_iter()
                .filter(|d| config.is_rule_enabled(&d.rule_id)),
        );

        // Step b: If skill parsed successfully, run linter
        let mut has_scripts = false;
        if !has_parse_error {
            if let Ok(skill) = load_skill(skill_dir) {
                has_scripts = skill.file_tree.has_scripts;
                let lint_diags = lint(&skill);
                diagnostics.extend(
                    lint_diags
                        .into_iter()
                        .filter(|d| config.is_rule_enabled(&d.rule_id)),
                );
            }
        }

        // Step c: If --no-score is not set, run scorer
        let score_card = if !no_score {
            Some(score(&diagnostics, has_scripts, config))
        } else {
            None
        };

        // Step d: Build SkillReport
        reports.push(SkillReport {
            path: skill_dir.clone(),
            diagnostics,
            score: score_card,
        });
    }

    reports
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

fn format_output(reports: &[SkillReport], config: &Config, quiet: bool) -> String {
    match config.format {
        OutputFormat::Terminal => format_terminal(reports, quiet),
        OutputFormat::Json => format_json(reports),
        OutputFormat::Sarif => format_sarif(reports),
        OutputFormat::Markdown => format_markdown(reports),
    }
}

// ---------------------------------------------------------------------------
// Exit code logic
// ---------------------------------------------------------------------------

fn determine_exit_code(reports: &[SkillReport], config: &Config) -> i32 {
    if reports.is_empty() {
        return 3;
    }

    let has_errors = reports
        .iter()
        .any(|r| r.diagnostics.iter().any(|d| d.severity == Severity::Error));
    let has_warnings = reports
        .iter()
        .any(|r| r.diagnostics.iter().any(|d| d.severity == Severity::Warning));

    let fail_diags = match config.fail_on {
        FailOn::Errors => has_errors,
        FailOn::Warnings => has_errors || has_warnings,
        FailOn::None => false,
    };

    if fail_diags {
        return 1;
    }

    if config.min_score > 0 {
        let below_threshold = reports.iter().any(|r| {
            r.score
                .as_ref()
                .map_or(false, |s| (s.composite.round() as u32) < config.min_score)
        });
        if below_threshold {
            return 2;
        }
    }

    0
}

// ---------------------------------------------------------------------------
// Color setup
// ---------------------------------------------------------------------------

fn setup_color(when: &ColorWhen) {
    match when {
        ColorWhen::Always => set_override(true),
        ColorWhen::Never => set_override(false),
        ColorWhen::Auto => {
            if !atty_stdout() {
                set_override(false);
            }
        }
    }
}

fn atty_stdout() -> bool {
    unsafe { libc_isatty(1) != 0 }
}

extern "C" {
    #[link_name = "isatty"]
    fn libc_isatty(fd: i32) -> i32;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    // Color setup
    setup_color(&cli.color);

    // Load config
    let base_config = match &cli.config {
        Some(path) => {
            if path.is_file() {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    toml::from_str::<Config>(&contents).unwrap_or_else(|e| {
                        eprintln!("warning: failed to parse config {}: {}", path.display(), e);
                        Config::default()
                    })
                } else {
                    eprintln!("warning: could not read config {}", path.display());
                    Config::default()
                }
            } else {
                eprintln!("warning: config file not found: {}", path.display());
                Config::default()
            }
        }
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            Config::load(&cwd)
        }
    };

    // Merge CLI flags into config
    let config = merge_cli_into_config(&cli, base_config);

    match &cli.command {
        Commands::Check { paths, fix } => {
            if *fix {
                println!("Fix mode not yet implemented. Use `skillplane check` to validate.");
                process::exit(0);
            }

            let reports = run_check(paths, &config, cli.no_score, cli.quiet);
            let output = format_output(&reports, &config, cli.quiet);
            print!("{output}");
            let code = determine_exit_code(&reports, &config);
            process::exit(code);
        }
        Commands::Fix { .. } => {
            println!("Fix mode not yet implemented. Use `skillplane check` to validate.");
            process::exit(0);
        }
    }
}
