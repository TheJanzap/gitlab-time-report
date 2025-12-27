//! Methods to turn [`TimeLog`] into SVG and HTML charts.

pub mod burndown;
mod charming_extensions;
mod chart_options;
mod estimates;

use crate::charts::charming_extensions::Series;
use crate::model::TimeLog;
use charming::component::Toolbox;
use charming::{
    Chart,
    series::{Bar, Line},
};
use charming_extensions::{ChartExt, MultiSeries, SingleSeries};
pub use chart_options::{BurndownOptions, BurndownType, ChartSettingError, RenderOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// Data to be converted into types from [`charming::series`].
pub type SeriesData = Vec<(String, Vec<f32>)>;

/// Number of digits after the decimal point when displaying values in charts.
const ROUNDING_PRECISION: u8 = 2;

/// Create a bar chart.
/// # Parameters
/// - `grouped_time_log`: A map of keys (e.g., User or Milestone) to a list of time logs
/// - `title`: The title for the chart
/// - `x_axis_label`: The text set for the X-axis
/// - `render`: Options for the rendering of the chart.
/// # Errors
/// - Returns [`ChartSettingError::CharmingError`] if the creation of the chart failed.
/// - Returns [`ChartSettingError::FileNotFound`] if the theme file in [`RenderOptions`] does not exist.
/// - Returns [`ChartSettingError::IoError`] if there was an error reading or writing from/to the filesystem.
pub fn create_bar_chart<'a, T>(
    grouped_time_log: BTreeMap<impl Into<Option<&'a T>>, Vec<&'a TimeLog>>,
    title: &str,
    x_axis_label: &str,
    render: &mut RenderOptions,
) -> Result<(), ChartSettingError>
where
    T: std::fmt::Display + 'a,
{
    let hours_per_t = create_multi_series(grouped_time_log);
    let chart = Chart::create_bar_chart(hours_per_t, &[x_axis_label.into()], 0.0, title);

    let chart_name = format!("barchart-{title}");
    render_chart_with_settings(chart, render, &chart_name)
}

/// Create a grouped bar chart, i.e., Hours per Label per User.
/// # Parameters
/// - `grouped_time_log`: A map of `Outer` keys (e.g., Label) to a map of `Inner` keys (e.g., User) to value (Duration)
/// - `title`: The title of the chart
/// - `x_axis_label_rotate`: The rotation of the X-axis labels.
/// - `render`: Options for the rendering of the chart.
/// # Errors
/// - Returns [`ChartSettingError::CharmingError`] if the creation of the chart failed.
/// - Returns [`ChartSettingError::FileNotFound`] if the theme file in [`RenderOptions`] does not exist.
/// - Returns [`ChartSettingError::IoError`] if there was an error reading or writing from/to the filesystem.
pub fn create_grouped_bar_chart<'a, Outer, Inner>(
    grouped_time_log: BTreeMap<
        impl Into<Option<&'a Outer>>,
        BTreeMap<impl Into<Option<&'a Inner>> + Clone, Vec<&'a TimeLog>>,
    >,
    title: &str,
    x_axis_label_rotate: f64,
    render: &mut RenderOptions,
) -> Result<(), ChartSettingError>
where
    Outer: std::fmt::Display + 'a,
    Inner: std::fmt::Display + 'a,
{
    let (series, axis_labels) = create_grouped_series(grouped_time_log);
    let chart = Chart::create_bar_chart(series, &axis_labels, x_axis_label_rotate, title);

    let chart_name = format!("barchart-grouped-{title}");
    render_chart_with_settings(chart, render, &chart_name)
}

/// Create a pie chart.
/// # Parameters
/// - `grouped_time_log`: A map of keys (e.g., User or Milestone) to a list of time logs
/// - `title`: The title of the chart
/// - `render`: Options for the rendering of the chart.
/// # Errors
/// - Returns [`ChartSettingError::CharmingError`] if the creation of the chart failed.
/// - Returns [`ChartSettingError::FileNotFound`] if the theme file in [`RenderOptions`] does not exist.
/// - Returns [`ChartSettingError::IoError`] if there was an error reading or writing from/to the filesystem.
pub fn create_pie_chart<'a, T>(
    grouped_time_log: BTreeMap<impl Into<Option<&'a T>>, Vec<&'a TimeLog>>,
    title: &str,
    render: &mut RenderOptions,
) -> Result<(), ChartSettingError>
where
    T: std::fmt::Display + 'a,
{
    let hours_per_t = create_single_series(grouped_time_log);
    let chart = Chart::create_pie_chart(hours_per_t, title);

    let chart_name = format!("piechart-{title}");
    render_chart_with_settings(chart, render, &chart_name)
}

/// Calculate the total hours per key (i.e., User or Milestone) and create a `Series`
/// for each data point to display in a chart. The key of the `BTreeMap` is `Into<Option<T>` to
/// allow for `Option<T>` and non-`Option<T>` keys. If it is `None`, the key is "None"
fn create_multi_series<'a, T, Series>(
    grouped_time_log: BTreeMap<impl Into<Option<&'a T>>, Vec<&'a TimeLog>>,
) -> Vec<Series>
where
    T: std::fmt::Display + 'a,
    Series: MultiSeries,
{
    let map = Series::create_data_point_mapping(grouped_time_log);
    map.into_iter()
        .map(|(hours, key)| Series::with_defaults(key.as_str(), vec![hours]))
        .collect()
}

/// Calculate the total hours per key (i.e., User or Milestone) and create a singular `Series` for
/// all data points display in a chart. The key of the `BTreeMap` is `Into<Option<T>` to allow for
/// `Option<T>` and non-`Option<T>` keys. If it is `None`, the key is "None"
fn create_single_series<'a, T, Series>(
    grouped_time_log: BTreeMap<impl Into<Option<&'a T>>, Vec<&'a TimeLog>>,
) -> Series
where
    T: std::fmt::Display + 'a,
    Series: SingleSeries,
{
    Series::with_defaults(Series::create_data_point_mapping(grouped_time_log))
}

/// Create a `Series` for all data points of a key to display in a chart.
/// The key of the `BTreeMap` is `Into<Option<T>` to allow for `Option<T>` and non-`Option<T>` keys.
/// If it is `None`, the key is "None".
/// Returns the `Series` and the axis labels for the grouped bar chart.
fn create_grouped_series<'a, Outer, Inner, Series>(
    grouped_time_log: BTreeMap<
        impl Into<Option<&'a Outer>>,
        BTreeMap<impl Into<Option<&'a Inner>> + Clone, Vec<&'a TimeLog>>,
    >,
) -> (Vec<Series>, Vec<String>)
where
    Outer: std::fmt::Display + 'a,
    Inner: std::fmt::Display + 'a,
    Series: MultiSeries,
{
    let mut duration_per_inner = BTreeMap::new();
    let mut axis_labels = Vec::new();

    // First, get all inner keys to validate later that they are present in all outer keys
    let all_inner_keys = grouped_time_log
        .values()
        .flat_map(|inner_map| inner_map.keys().cloned().map(|k| Bar::option_to_string(k)))
        .collect::<BTreeSet<_>>();

    for (outer_key, inner_map) in grouped_time_log {
        // Add the outer key to Vec of axis labels
        let outer_key_string = Bar::option_to_string(outer_key);
        axis_labels.push(outer_key_string);

        // Create a map with the inner key as String and the total hours as f32
        let mut data_points = Bar::create_data_point_mapping(inner_map)
            .into_iter()
            .map(|(v, k)| (k, v))
            .collect::<BTreeMap<_, _>>();

        // Ensure that all inner keys are present
        for key in &all_inner_keys {
            data_points.entry(key.clone()).or_insert("0".into());
        }

        // Insert all data points into the outer map
        for (key, value) in data_points {
            duration_per_inner
                .entry(key)
                .or_insert_with(Vec::new)
                .push(value);
        }
    }

    let series = duration_per_inner
        .into_iter()
        .map(|(key, hours)| Series::with_defaults(key.as_str(), hours))
        .collect();

    (series, axis_labels)
}

/// Create a burndown line chart.
/// # Errors
/// - Returns [`ChartSettingError::FileNotFound`] if the file in [`RenderOptions`] does not exist.
/// - Returns [`ChartSettingError::CharmingError`] if the graph could not be created.
pub fn create_burndown_chart(
    time_logs: &[TimeLog],
    burndown_type: &BurndownType,
    burndown_options: &BurndownOptions,
    render_options: &mut RenderOptions,
) -> Result<(), ChartSettingError> {
    let (burndown_data, x_axis) =
        burndown::calculate_burndown_data(time_logs, burndown_type, burndown_options);

    let burndown_series = burndown_data
        .into_iter()
        .map(|(name, data)| {
            let data = data
                .into_iter()
                .map(|d| round_to_string(d, ROUNDING_PRECISION))
                .collect();
            Line::with_defaults(&name, data)
        })
        .collect::<Vec<_>>();

    let title = match burndown_type {
        BurndownType::Total => "Burndown Chart Total",
        BurndownType::PerPerson => "Burndown Chart per Person",
    };

    let chart = Chart::create_line_chart(burndown_series, &x_axis, 0.0, title);
    let chart_name = format!("burndown-{burndown_type}");
    render_chart_with_settings(chart, render_options, &chart_name)
}

/// Creates a chart with estimates and actual time from grouped time logs (i.e., Estimates vs.
/// actual time on all labels)
/// # Errors
/// - Returns [`ChartSettingError::FileNotFound`] if the file in [`RenderOptions`] does not exist.
/// - Returns [`ChartSettingError::CharmingError`] if the graph could not be created.
pub fn create_estimate_chart<'a, T>(
    grouped_time_log: BTreeMap<impl Into<Option<&'a T>> + Clone, Vec<&'a TimeLog>>,
    title: &str,
    render_options: &mut RenderOptions,
) -> Result<(), ChartSettingError>
where
    T: std::fmt::Display + 'a,
{
    let (estimate_data, x_axis) = estimates::calculate_estimate_data::<T, Bar>(grouped_time_log);
    let estimate_series = estimate_data
        .into_iter()
        .map(|(name, data)| {
            let data = data
                .into_iter()
                .map(|d| round_to_string(d, ROUNDING_PRECISION))
                .collect();
            Bar::with_defaults(&name, data)
        })
        .collect();

    let chart = Chart::create_bar_chart(estimate_series, &x_axis, 50.0, title);
    let chart_name = format!("barchart-{title}");
    render_chart_with_settings(chart, render_options, &chart_name)
}

/// Renders a chart as an SVG and an HTML file.
fn render_chart_with_settings(
    mut chart: Chart,
    render_options: &mut RenderOptions,
    chart_name: &str,
) -> Result<(), ChartSettingError> {
    let chart_theme = render_options
        .theme_file_path
        .map(fs::read_to_string)
        .transpose()?;

    if !render_options.output_path.exists() {
        fs::create_dir_all(render_options.output_path)?;
    }

    let chart_filename = format!(
        "{prefix:02}_{repository}_{name}",
        prefix = render_options.file_name_prefix,
        repository = render_options.repository_name,
        name = chart_name.replace(' ', "-").to_lowercase()
    );

    let html = chart.render_html(
        u64::from(render_options.width),
        u64::from(render_options.height),
        chart_theme.as_deref(),
    )?;
    let html_path = render_options
        .output_path
        .join(format!("{chart_filename}.html"));
    fs::write(html_path, html)?;

    // Disable toolbox in SVG
    chart = chart.toolbox(Toolbox::new().show(false));
    let svg = chart.render_svg(
        u32::from(render_options.width),
        u32::from(render_options.height),
        chart_theme.as_deref(),
    )?;
    let svg_path = render_options
        .output_path
        .join(format!("{chart_filename}.svg"));

    fs::write(svg_path, svg)?;

    render_options.file_name_prefix += 1;
    Ok(())
}

/// Rounds the value to `precision` number of decimal places and returns a string
/// to avoid floating point inaccuracies
fn round_to_string(value: f32, max_precision: u8) -> String {
    let p_i32 = i32::from(max_precision);
    let rounded = (value * 10.0_f32.powi(p_i32)).round() / 10.0_f32.powi(p_i32);
    format!("{rounded:.precision$}", precision = max_precision as usize)
        // Remove zeros behind the point
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters;
    use crate::model::{
        Issue, MergeRequest, Milestone, TrackableItem, TrackableItemFields, TrackableItemKind, User,
    };
    use charming::series::{Bar, Pie};
    use chrono::{DateTime, Duration, Local, NaiveDate};

    const NUMBER_OF_LOGS: usize = 6;
    pub(super) const PROJECT_WEEKS: u16 = 4;
    pub(super) const WEEKS_PER_SPRINT_DEFAULT: u16 = 1;
    pub(super) const SPRINTS: u16 = PROJECT_WEEKS;
    pub(super) const TOTAL_HOURS_PER_PERSON: f32 = 10.0;
    pub(super) const PROJECT_START: Option<NaiveDate> = NaiveDate::from_ymd_opt(2025, 1, 1);

    #[expect(clippy::too_many_lines)]
    pub(super) fn get_time_logs() -> [TimeLog; NUMBER_OF_LOGS] {
        let user1 = User {
            name: "User 1".into(),
            username: "user1".to_string(),
        };
        let user2 = User {
            name: "User 2".into(),
            username: "user2".to_string(),
        };

        let m1 = Milestone {
            title: "M1".into(),
            ..Milestone::default()
        };
        let m2 = Milestone {
            title: "M2".into(),
            ..Milestone::default()
        };

        let issue_0 = TrackableItem {
            kind: TrackableItemKind::Issue(Issue::default()),
            common: TrackableItemFields {
                id: 0,
                title: "Issue 0".into(),
                time_estimate: Duration::hours(2),
                total_time_spent: Duration::hours(3) + Duration::minutes(30),
                milestone: Some(m1.clone()),
                ..Default::default()
            },
        };

        [
            TimeLog {
                time_spent: Duration::hours(1),
                spent_at: "2025-01-01T12:00:00+01:00"
                    .parse::<DateTime<Local>>()
                    .unwrap(),
                user: user1.clone(),
                trackable_item: issue_0.clone(),
                ..Default::default()
            },
            TimeLog {
                time_spent: Duration::hours(2) + Duration::minutes(30),
                spent_at: "2025-01-02T09:10:23+01:00"
                    .parse::<DateTime<Local>>()
                    .unwrap(),
                user: user1.clone(),
                trackable_item: issue_0.clone(),
                ..Default::default()
            },
            TimeLog {
                time_spent: Duration::hours(1) + Duration::minutes(30),
                spent_at: "2025-01-10T12:00:00+01:00"
                    .parse::<DateTime<Local>>()
                    .unwrap(),
                user: user1.clone(),
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        id: 1,
                        title: "Issue 1".into(),
                        time_estimate: Duration::hours(2),
                        total_time_spent: Duration::hours(1) + Duration::minutes(30),
                        milestone: Some(m2.clone()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            TimeLog {
                time_spent: Duration::hours(4) + Duration::minutes(15),
                spent_at: "2025-01-01T12:00:00+01:00"
                    .parse::<DateTime<Local>>()
                    .unwrap(),
                user: user2.clone(),
                trackable_item: TrackableItem {
                    kind: TrackableItemKind::MergeRequest(MergeRequest::default()),
                    common: TrackableItemFields {
                        id: 0,
                        title: "MR 0".into(),
                        time_estimate: Duration::hours(5),
                        total_time_spent: Duration::hours(4) + Duration::minutes(15),
                        milestone: Some(m1.clone()),
                        ..Default::default()
                    },
                },
                ..Default::default()
            },
            TimeLog {
                time_spent: Duration::hours(1),
                spent_at: "2025-01-08T12:00:00+01:00"
                    .parse::<DateTime<Local>>()
                    .unwrap(),
                user: user2.clone(),
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        id: 2,
                        title: "Issue 2".into(),
                        time_estimate: Duration::hours(2),
                        total_time_spent: Duration::hours(1),
                        milestone: None,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            TimeLog {
                time_spent: Duration::hours(4),
                spent_at: "2025-01-28T12:00:00+01:00"
                    .parse::<DateTime<Local>>()
                    .unwrap(),
                user: user2.clone(),
                trackable_item: TrackableItem {
                    kind: TrackableItemKind::MergeRequest(MergeRequest::default()),
                    common: TrackableItemFields {
                        id: 1,
                        title: "MR 1".to_string(),
                        total_time_spent: Duration::hours(4),
                        ..Default::default()
                    },
                },
                ..Default::default()
            },
        ]
    }

    #[test]
    fn validate_test_data() {
        let time_logs = get_time_logs();
        let by_item = filters::group_by_trackable_item(&time_logs);
        by_item.into_iter().for_each(|(item, time_logs)| {
            let total_time = filters::total_time_spent(time_logs.clone());
            assert_eq!(
                total_time, item.common.total_time_spent,
                "{} {} has an incorrect total time spent",
                item.kind, item.common.id
            );
        });
    }

    #[test]
    fn test_create_multi_series() {
        const USER_1_TIME: f32 = 5.0;
        const USER_2_TIME: f32 = 9.25;

        let time_logs = get_time_logs();
        let time_logs_per_user = filters::group_by_user(&time_logs).collect();
        let expected_result = [
            Bar::with_defaults(
                "User 1",
                vec![round_to_string(USER_1_TIME, ROUNDING_PRECISION)],
            ),
            Bar::with_defaults(
                "User 2",
                vec![round_to_string(USER_2_TIME, ROUNDING_PRECISION)],
            ),
        ];

        let result: Vec<Bar> = create_multi_series(time_logs_per_user);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_create_multi_series_with_optional_key() {
        const NONE_TIME: f32 = 5.0;
        const M1_TIME: f32 = 7.75;
        const M2_TIME: f32 = 1.5;

        let time_logs = get_time_logs();
        let time_logs_per_milestone = filters::group_by_milestone(&time_logs).collect();
        let expected_result = [
            Bar::with_defaults("None", vec![round_to_string(NONE_TIME, ROUNDING_PRECISION)]),
            Bar::with_defaults("M1", vec![round_to_string(M1_TIME, ROUNDING_PRECISION)]),
            Bar::with_defaults("M2", vec![round_to_string(M2_TIME, ROUNDING_PRECISION)]),
        ];

        let result: Vec<Bar> = create_multi_series(time_logs_per_milestone);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_create_single_series() {
        const USER_1_TIME: f32 = 5.0;
        const USER_2_TIME: f32 = 9.25;

        let time_logs = get_time_logs();
        let time_logs_per_user = filters::group_by_user(&time_logs).collect();
        let expected_result = Pie::with_defaults(vec![
            (round_to_string(USER_1_TIME, ROUNDING_PRECISION), "User 1"),
            (round_to_string(USER_2_TIME, ROUNDING_PRECISION), "User 2"),
        ]);

        let result: Pie = create_single_series(time_logs_per_user);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_create_single_series_with_optional_key() {
        const NONE_TIME: f32 = 5.0;
        const M1_TIME: f32 = 7.75;
        const M2_TIME: f32 = 1.5;

        let time_logs = get_time_logs();
        let time_logs_per_label = filters::group_by_milestone(&time_logs).collect();
        let expected_result = Pie::with_defaults(vec![
            (round_to_string(NONE_TIME, ROUNDING_PRECISION), "None"),
            (round_to_string(M1_TIME, ROUNDING_PRECISION), "M1"),
            (round_to_string(M2_TIME, ROUNDING_PRECISION), "M2"),
        ]);

        let result: Pie = create_single_series(time_logs_per_label);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_create_grouped_series() {
        const USER_1_NONE: f32 = 0.0;
        const USER_1_M1: f32 = 3.5;
        const USER_1_M2: f32 = 1.5;
        const USER_2_NONE: f32 = 5.0;
        const USER_2_M1: f32 = 4.25;
        const USER_2_M2: f32 = 0.0;

        let time_logs = get_time_logs();
        let time_logs_per_milestone_per_user: BTreeMap<_, _> =
            filters::group_by_milestone(&time_logs)
                .map(|(m, t)| (m, filters::group_by_user(t).collect::<BTreeMap<_, _>>()))
                .collect();

        let user_1_expected_data = vec![
            round_to_string(USER_1_NONE, 2),
            round_to_string(USER_1_M1, 2),
            round_to_string(USER_1_M2, 2),
        ];
        let user_2_expected_data = vec![
            round_to_string(USER_2_NONE, 2),
            round_to_string(USER_2_M1, 2),
            round_to_string(USER_2_M2, 2),
        ];

        let expected_result = [
            Bar::with_defaults("User 1", user_1_expected_data),
            Bar::with_defaults("User 2", user_2_expected_data),
        ];

        let expected_labels = ["None", "M1", "M2"];

        let (series, labels): (Vec<Bar>, _) =
            create_grouped_series(time_logs_per_milestone_per_user);
        assert_eq!(series, expected_result);
        assert_eq!(labels, expected_labels);
    }

    #[test]
    fn test_round_to_string() {
        assert_eq!(round_to_string(1.23456, 2), "1.23");
        assert_eq!(round_to_string(1.75, 2), "1.75");
        assert_eq!(round_to_string(1.75, 3), "1.75");
        assert_eq!(round_to_string(1.66666, 2), "1.67");
        assert_eq!(round_to_string(1.66666, 1), "1.7");
        assert_eq!(round_to_string(1.66666, 0), "2");
        assert_eq!(round_to_string(1.99999, 2), "2");
        assert_eq!(round_to_string(1.0, 2), "1");
        assert_eq!(round_to_string(-1.286, 2), "-1.29");
    }
}
