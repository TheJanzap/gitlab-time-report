//! Contains the CLI arguments for gitlab-time-report.

use chrono::NaiveDate;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

const BEFORE_HELP: &str = "\n
gitlab-time-report exports time logs from Issues and Merge Requests of a GitLab repository \
to create statistics and charts of your working hours. Use --help on commands to get more information. \
If no subcommand is used, statistics will be printed directly to your console.";

const AFTER_HELP: &str = "By default, validation of time logs is enabled. \
It checks for excessive hours, future dates, duplicate time logs and missing summaries.
If a time log violates any of these rules, it is printed to the console. \
No automatic correction is performed. You can display the full validation results with --validation-details.";

#[derive(Parser)]
#[command(version, about, before_help = BEFORE_HELP, after_help = AFTER_HELP,
    arg_required_else_help = true, subcommand_precedence_over_arg = true
)]
pub(super) struct Arguments {
    /// The URLs of the GitLab repositories you want to export time logs from.
    /// When using multiple URLs, separate them with a space.
    #[arg(value_name = "URLs", value_delimiter = ' ', env = "GITLAB_URL")]
    pub(super) url: Vec<String>,

    /// A GitLab access token. Needed if the repository is not public.
    /// Can be a personal, group, or repository access token.
    /// All repositories must be accessible with the same token, using different tokens per
    /// repository is not supported.
    #[arg(short, long, env = "GITLAB_TOKEN")]
    pub(super) token: Option<String>,

    /// Comma-separated list of GitLab labels to filter by. All other labels will be grouped under
    /// "Others". If your labels contain spaces, delimit them with `"`:
    /// `--labels "Epic 1",Documentation`. The labels are case-sensitive.
    /// If not set, all labels are shown.
    #[arg(short, long, value_delimiter = ',', env = "GITLAB_LABELS")]
    pub(super) labels: Vec<String>,

    /// Displays the problems with the time logs found during validation.
    #[arg(long)]
    pub(super) validation_details: bool,

    /// When validating time logs, the maximum number of hours a time log should have.
    #[arg(long, default_value = "10")]
    pub(super) validation_max_hours: u16,

    /// The subcommand to run.
    #[command(subcommand)]
    pub(super) command: Option<Command>,
}

#[derive(Subcommand)]
pub(super) enum Command {
    /// Export the time logs to a CSV file.
    Export {
        /// The path to the CSV file. If not set, the file will be saved to the current working
        /// directory with the name `timelogs.csv`.
        #[arg(short, long, default_value = "timelogs.csv")]
        output: PathBuf,
    },
    /// Generate charts from your time logs as HTML and SVG. Creates the following charts:
    /// Time spent per user, per milestone, per label, per user/label, per type (issue or MR),
    /// per type/user, estimates/actual time per label and burndown charts (total and per user).
    /// Burndown charts are only generated if `--sprints`, `--weeks-per-sprint` and
    /// `--hours-per-person` are provided.
    Charts {
        #[command(flatten)]
        chart_options: ChartOptionsArgs,
    },
    /// Generate an HTML file with charts and statistics about the time logs.
    /// Burndown charts are only generated if `--sprints`, `--weeks-per-sprint` and
    /// `--hours-per-person` are provided.
    Dashboard {
        #[command(flatten)]
        chart_options: ChartOptionsArgs,
    },
}

/// Shared options for generating charts.
#[derive(Args)]
#[clap(disable_help_flag = true)]
pub(super) struct ChartOptionsArgs {
    /// The width of the charts.
    #[arg(short, long, default_value = "800")]
    pub(super) width: u16,

    /// The height of the charts.
    #[arg(short, long, default_value = "480")]
    pub(super) height: u16,

    /// Create your own theme for the charts at <https://echarts.apache.org/en/theme-builder.html>
    /// and specify the path of the resulting JSON file. If not set, the charts are created using
    /// default settings.
    #[arg(long, value_name = "PATH_TO_THEME_JSON", env)]
    pub(super) theme_json: Option<PathBuf>,

    /// The path the charts will be written to. If not set, a "charts" directory will be created in
    /// the current working directory.
    #[arg(short, long, default_value = "charts")]
    pub(super) output: PathBuf,

    #[command(flatten)]
    pub(super) burndown: Option<BurndownOptionsArgs>,

    /// The start date of the burndown chart.
    /// If not set, the date of the earliest time log is used.
    #[arg(long, value_name = "YYYY-MM-DD")]
    pub(super) start_date: Option<NaiveDate>,

    // Implements help only for --help, not for -h
    /// Print help
    #[clap(long, action = clap::ArgAction::HelpLong)]
    help: Option<bool>,
}

/// Options for generating burndown charts.
#[derive(Args)]
#[group(requires_all = ["sprints", "weeks_per_sprint", "hours_per_person"])]
pub(super) struct BurndownOptionsArgs {
    /// The number of sprints the project is planned to take for the burndown chart.
    /// If sprints are not used in your project, enter the number of weeks that the project has.
    #[arg(long, value_name = "NUMBER_OF_SPRINTS", env, required = false)]
    pub(super) sprints: u16,

    /// The number of weeks each sprint lasts. Required for the burndown charts.
    #[arg(long, env, required = false)]
    pub(super) weeks_per_sprint: u16,

    /// The number of hours a person is expected to work in total. Required for the burndown charts.
    #[arg(long, env, required = false)]
    pub(super) hours_per_person: f32,
}
