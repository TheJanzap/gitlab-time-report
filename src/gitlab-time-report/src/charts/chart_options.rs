//! Data structures for creating charts

use crate::model::TimeLog;
use chrono::{Local, NaiveDate};
use std::path::Path;
use std::process;
use thiserror::Error;

/// Contains all information needed for rendering a chart.
/// Create a new instance with [`RenderOptions::new()`].
pub struct RenderOptions<'a> {
    /// The width of the rendered chart.
    pub(super) width: u16,
    /// The height of the rendered chart.
    pub(super) height: u16,
    /// The path to the chart theme JSON file.
    pub(super) theme_file_path: Option<&'a Path>,
    /// The path the charts will be written to.
    pub(super) output_path: &'a Path,
    /// Counter that will be added to the file name.
    /// Used to determine the order in which the charts will be added to the dashboard.
    pub(super) file_name_prefix: u8,
    /// The name of the repository. Will be added to the file name to disambiguate charts
    /// from different repos.
    pub(super) repository_name: String,
}

impl<'a> RenderOptions<'a> {
    /// Creates a new `RenderOptions` instance.
    /// # Errors
    /// Returns a [`ChartSettingError::FileNotFound`] if the theme file does not exist.
    pub fn new(
        width: u16,
        height: u16,
        theme_file_path: Option<&'a Path>,
        output_path: &'a Path,
        repository_name: &'a str,
    ) -> Result<Self, ChartSettingError> {
        if let Some(path) = &theme_file_path
            && !path.exists()
        {
            return Err(ChartSettingError::FileNotFound);
        }

        Ok(Self {
            width,
            height,
            theme_file_path,
            output_path,
            file_name_prefix: 1,
            repository_name: repository_name
                .replace(", ", "_")
                .replace(' ', "-")
                .to_lowercase(),
        })
    }
}

/// The type of burndown chart to create.
#[derive(Debug, PartialEq)]
pub enum BurndownType {
    /// The burndown chart shows the total amount of work done per week/sprint.
    Total,
    /// The burndown chart shows the amount of work done per week/sprint per person.
    PerPerson,
}

impl std::fmt::Display for BurndownType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BurndownType::Total => write!(f, "total"),
            BurndownType::PerPerson => write!(f, "per-person"),
        }
    }
}

/// Contains all information needed for creating a burndown chart.
/// Create a new instance with [`BurndownOptions::new()`].
#[derive(Debug)]
pub struct BurndownOptions {
    /// The number of weeks a sprint has.
    pub(super) weeks_per_sprint: u16,
    /// The number of sprints the project has.
    pub(super) sprints: u16,
    /// How many hours a single person should work on the project in total.
    pub(super) hours_per_person: f32,
    /// The start date of the project.
    pub(super) start_date: NaiveDate,
}

impl BurndownOptions {
    /// Creates a new `BurndownOptions` instance. For the meaning of the parameters, see [`BurndownOptions`].
    /// `TimeLogs` need to be passed in as a fallback for the start date if it is `None`.
    /// # Parameters
    /// - `time_logs`: Entries from the GitLab API
    /// - `weeks_per_sprint`: Duration of a sprint in weeks.
    /// - `sprints`: How many sprints the project has. If sprints aren't used, set it
    ///   to the number of weeks your project has.
    /// - `hours_per_person`: How many hours *a single user/team* should work on the project in total.
    /// - `start_date`: Starting date of the project (usually first log date)
    /// # Errors
    /// Returns [`ChartSettingError::InvalidInputData`] if the input data is not valid.
    pub fn new(
        time_logs: &[TimeLog],
        weeks_per_sprint: u16,
        sprints: u16,
        hours_per_person: f32,
        start_date: Option<NaiveDate>,
    ) -> Result<Self, ChartSettingError> {
        if time_logs.is_empty() {
            return Err(ChartSettingError::InvalidInputData(
                "No time logs found".to_string(),
            ));
        }

        // Set the start date to the earliest time log date if not set
        let start_date = start_date.unwrap_or_else(|| {
            time_logs
                .iter()
                .map(|t| t.spent_at.date_naive())
                .min()
                .unwrap_or_else(|| {
                    eprintln!("No time logs found.");
                    process::exit(6);
                })
        });

        // Some validation checks
        if weeks_per_sprint == 0 {
            return Err(ChartSettingError::InvalidInputData(
                "Weeks per Sprint cannot be 0".to_string(),
            ));
        }

        if hours_per_person == 0.0 {
            return Err(ChartSettingError::InvalidInputData(
                "Hours per Person cannot be 0".to_string(),
            ));
        }

        if sprints == 0 {
            return Err(ChartSettingError::InvalidInputData(
                "Sprints cannot be 0".to_string(),
            ));
        }

        if start_date > Local::now().date_naive() {
            return Err(ChartSettingError::InvalidInputData(
                "Start date cannot be in the future".to_string(),
            ));
        }

        Ok(Self {
            weeks_per_sprint,
            sprints,
            hours_per_person,
            start_date,
        })
    }
}

/// Possible errors when creating a chart.
#[derive(Debug, Error)]
pub enum ChartSettingError {
    /// The JSON file containing the chart theme settings was not found.
    #[error("The theme JSON file was not found.")]
    FileNotFound,
    /// IO error while reading or writing the file.
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    /// Error during chart creation.
    #[error("Could not create chart: {0}")]
    CharmingError(#[from] charming::EchartsError),
    /// The chart input values failed validation.
    #[error("Invalid input data: {0}")]
    InvalidInputData(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charts::tests::*;

    const WIDTH: u16 = 600;
    const HEIGHT: u16 = 600;
    const REPOSITORY_NAME_INPUT: &str = "Sample Repository";
    const REPOSITORY_NAME_OUTPUT: &str = "sample-repository";

    #[test]
    fn renderoptions_new_returns_ok_with_theme_path_set() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let theme_path = tmp.path();
        let output_path = Path::new("docs/charts/");
        let chart_options = RenderOptions::new(
            WIDTH,
            HEIGHT,
            Some(theme_path),
            output_path,
            REPOSITORY_NAME_INPUT,
        );
        let result = chart_options;
        assert!(result.is_ok());
        let render_options = result.unwrap();

        assert_eq!(render_options.width, WIDTH);
        assert_eq!(render_options.height, HEIGHT);
        assert_eq!(render_options.theme_file_path, Some(theme_path));
        assert_eq!(render_options.output_path, output_path);
        assert_eq!(render_options.repository_name, REPOSITORY_NAME_OUTPUT);
    }

    #[test]
    fn renderoptions_new_returns_ok_with_no_theme_path_set() {
        let theme_path = None;
        let output_path = Path::new("docs/charts/");
        let chart_options = RenderOptions::new(
            WIDTH,
            HEIGHT,
            theme_path,
            output_path,
            REPOSITORY_NAME_INPUT,
        );
        let result = chart_options;
        assert!(result.is_ok());
        let render_options = result.unwrap();

        assert_eq!(render_options.width, WIDTH);
        assert_eq!(render_options.height, HEIGHT);
        assert_eq!(render_options.theme_file_path, None);
        assert_eq!(render_options.output_path, output_path);
        assert_eq!(render_options.repository_name, REPOSITORY_NAME_OUTPUT);
    }

    #[test]
    fn renderoptions_new_returns_err_with_invalid_path() {
        let theme_path = Path::new("invalidfile");
        let output_path = Path::new("charts");
        let chart_options = RenderOptions::new(
            WIDTH,
            HEIGHT,
            Some(theme_path),
            output_path,
            REPOSITORY_NAME_INPUT,
        );
        let result = chart_options;
        assert!(result.is_err());
        assert!(matches!(result, Err(ChartSettingError::FileNotFound)));
    }

    #[test]
    fn burndownoptions_new_returns_ok_with_valid_input_data() {
        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT_DEFAULT,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            PROJECT_START,
        );
        let result = chart_options;
        assert!(result.is_ok());
        let burndown_options = result.unwrap();
        assert_eq!(burndown_options.weeks_per_sprint, WEEKS_PER_SPRINT_DEFAULT);
        assert_eq!(burndown_options.sprints, SPRINTS);
        #[expect(clippy::float_cmp)]
        {
            assert_eq!(burndown_options.hours_per_person, TOTAL_HOURS_PER_PERSON);
        }
        assert_eq!(burndown_options.start_date, PROJECT_START.unwrap());
    }

    #[test]
    fn burndownoptions_new_returns_ok_with_implicit_start_date() {
        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT_DEFAULT,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            None,
        );
        let result = chart_options;
        assert!(result.is_ok());
        let burndown_options = result.unwrap();
        assert_eq!(burndown_options.weeks_per_sprint, WEEKS_PER_SPRINT_DEFAULT);
        assert_eq!(burndown_options.sprints, SPRINTS);
        #[expect(clippy::float_cmp)]
        {
            assert_eq!(burndown_options.hours_per_person, TOTAL_HOURS_PER_PERSON);
        }
        let first_date = time_logs.iter().map(|l| l.spent_at).min().unwrap();
        assert_eq!(burndown_options.start_date, first_date.date_naive());
    }

    #[test]
    fn burndownoptions_new_returns_err_without_timelogs() {
        let time_logs = Vec::<TimeLog>::new();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT_DEFAULT,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            PROJECT_START,
        );
        let result = chart_options;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ChartSettingError::InvalidInputData(_)),
            "Should not allow empty time logs"
        );
    }

    #[test]
    fn burndownoptions_new_returns_err_with_zero_weeks_per_sprint() {
        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            0,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            PROJECT_START,
        );
        let result = chart_options;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ChartSettingError::InvalidInputData(_)),
            "Should not allow zero weeks per sprint"
        );
    }

    #[test]
    fn burndownoptions_new_returns_err_with_invalid_hours_per_person() {
        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT_DEFAULT,
            SPRINTS,
            0.0,
            PROJECT_START,
        );
        let result = chart_options;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ChartSettingError::InvalidInputData(_)),
            "Should not allow zero hours per person"
        );
    }

    #[test]
    fn burndownoptions_new_returns_err_with_zero_sprints() {
        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT_DEFAULT,
            0,
            TOTAL_HOURS_PER_PERSON,
            PROJECT_START,
        );
        let result = chart_options;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ChartSettingError::InvalidInputData(_)),
            "Should not allow zero sprints"
        );
    }

    #[test]
    fn burndownoptions_new_returns_err_with_start_date_in_future() {
        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT_DEFAULT,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            Some(Local::now().date_naive() + chrono::Duration::days(1)),
        );
        let result = chart_options;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ChartSettingError::InvalidInputData(_)),
            "Should not allow start date in the future"
        );
    }
}
