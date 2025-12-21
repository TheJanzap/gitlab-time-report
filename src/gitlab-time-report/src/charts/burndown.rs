//! Calculates the necessary data for burndown chart creation.

pub use super::{BurndownOptions, BurndownType, SeriesData};
use crate::TimeDeltaExt;
use crate::model::TimeLog;
use chrono::{Duration, NaiveDate};
use std::collections::{BTreeMap, HashSet};

/// Aggregated work hours per sprint and the total required hours for the project.
struct AggregatedHours {
    /// The total hours worked per sprint/week.
    actual_hours: BTreeMap<String, Vec<f32>>,
    /// The total required hours for the project.
    total_required_hours: f32,
}

/// Prepares the data for the burndown chart.
/// # Returns
/// - `LineData`: To be converted into `Line`
/// - `Vec<String>`: To be converted into the x-axis labels.
pub(super) fn calculate_burndown_data(
    time_logs: &[TimeLog],
    burndown_type: &BurndownType,
    burndown_options: &BurndownOptions,
) -> (SeriesData, Vec<String>) {
    let aggregated_hours = aggregate_hours_per_sprint(time_logs, burndown_type, burndown_options);

    // Create the actual times line
    let mut burndown_series = create_actual_hours_line(&aggregated_hours, burndown_options.sprints);

    // Add the ideal burndown line
    let ideal = create_ideal_burndown_line(
        burndown_options.sprints,
        aggregated_hours.total_required_hours,
    );
    burndown_series.push(ideal);

    let x_axis = create_burndown_x_axis_labels(burndown_options);
    (burndown_series, x_axis)
}

/// Aggregates the logged hours per sprint and calculates the required hours.
/// For [`BurndownType::PerPerson`] this returns one series per user.
/// For [`BurndownType::Total`] this returns a single `"Total"` series.
///
/// The returned [`AggregatedHours::total_required_hours`] contains the total required hours,
/// either per person or for all users combined, depending on `burndown_type`.
fn aggregate_hours_per_sprint(
    time_logs: &[TimeLog],
    burndown_type: &BurndownType,
    burndown_options: &BurndownOptions,
) -> AggregatedHours {
    let project_end = project_end_date(burndown_options);

    let mut actual_hours: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    let mut unique_users: HashSet<String> = HashSet::new();

    for log in time_logs {
        let log_date = log.spent_at.naive_local().date();
        if log_date > project_end {
            continue;
        }

        let sprint_number = sprint_index(
            burndown_options.start_date,
            burndown_options.weeks_per_sprint,
            log_date,
        );

        let key = match burndown_type {
            BurndownType::PerPerson => log.user.name.clone(),
            BurndownType::Total => {
                // Track seen users by username (guaranteed to be unique)
                unique_users.insert(log.user.username.clone());
                "Total".to_string()
            }
        };

        let entry = actual_hours
            .entry(key)
            .or_insert_with(|| vec![0.0_f32; burndown_options.sprints as usize]);

        if let Some(sprint) = entry.get_mut(sprint_number) {
            *sprint += log.time_spent.total_hours();
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "unique_users.len() should always be < 23 bits"
    )]
    let total_required_hours = match burndown_type {
        BurndownType::PerPerson => burndown_options.hours_per_person,
        BurndownType::Total => burndown_options.hours_per_person * unique_users.len() as f32,
    };

    AggregatedHours {
        actual_hours,
        total_required_hours,
    }
}

/// Calculates the project end date from the start date, number of sprints and weeks per sprint.
fn project_end_date(burndown_options: &BurndownOptions) -> NaiveDate {
    let project_weeks = i64::from(burndown_options.weeks_per_sprint * burndown_options.sprints);
    burndown_options.start_date + Duration::weeks(project_weeks)
}

/// Creates the line from the actual hours worked per user for the burndown chart.
fn create_actual_hours_line(
    aggregated_hours: &AggregatedHours,
    sprint_amount: u16,
) -> Vec<(String, Vec<f32>)> {
    aggregated_hours
        .actual_hours
        .iter()
        .map(|(user, sprints)| {
            let mut remaining_hours_per_sprint = Vec::with_capacity(sprint_amount as usize + 1);

            // Start at full time remaining
            let mut remaining_hours: f32 = aggregated_hours.total_required_hours;
            remaining_hours_per_sprint.push(remaining_hours);

            for hours_this_sprint in sprints {
                remaining_hours -= *hours_this_sprint;
                remaining_hours_per_sprint.push(remaining_hours);
            }
            (user.clone(), remaining_hours_per_sprint)
        })
        .collect::<Vec<_>>()
}

/// Calculates the ideal burndown line.
fn create_ideal_burndown_line(sprints: u16, total_required_hours: f32) -> (String, Vec<f32>) {
    // Calculate target hours per sprint per person
    let target_hours_per_sprint = total_required_hours / f32::from(sprints);
    (
        "Ideal".to_string(),
        (0..=sprints)
            .map(|i| (total_required_hours - f32::from(i) * target_hours_per_sprint).max(0.0))
            .collect(),
    )
}

/// Creates the x-axis labels for the burndown chart.
/// Start, W1..Wn if `weeks_per_sprint` == 1, S1..Sn otherwise.
fn create_burndown_x_axis_labels(burndown_options: &BurndownOptions) -> Vec<String> {
    // x-axis labels: Start, W1..Wn
    let x_axis_text = match burndown_options.weeks_per_sprint {
        1 => "W",
        _ => "S",
    };
    (0..=burndown_options.sprints)
        .map(|sprint_num| match sprint_num {
            0 => "Start".to_string(),
            _ => format!("{x_axis_text}{sprint_num}"),
        })
        .collect::<Vec<_>>()
}

/// Calculates the number of the week the date is in, based on the start date.
fn sprint_index(start: NaiveDate, sprint_length: u16, date: NaiveDate) -> usize {
    let days = (date - start).num_days();
    let sprint_length_in_days = i64::from(sprint_length * 7);
    usize::try_from(days / sprint_length_in_days).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charts::tests::*;
    use chrono::NaiveDate;

    #[test]
    fn test_create_burndown_chart_per_user_no_sprints() {
        const TOTAL_DATA_LENGTH: usize = (PROJECT_WEEKS + 1) as usize;
        const USER_1_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS_PER_PERSON, 6.5, 5.0, 5.0, 5.0];
        const USER_2_DATA: [f32; TOTAL_DATA_LENGTH] =
            [TOTAL_HOURS_PER_PERSON, 5.75, 4.75, 4.75, 0.75];
        const IDEAL_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS_PER_PERSON, 7.5, 5.0, 2.5, 0.0];

        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT_DEFAULT,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            PROJECT_START,
        );

        let (burndown_series, x_axis) = calculate_burndown_data(
            &time_logs,
            &BurndownType::PerPerson,
            &chart_options.unwrap(),
        );

        assert_eq!(x_axis, vec!["Start", "W1", "W2", "W3", "W4"]);

        let user_1_data = burndown_series.first().unwrap();
        assert_eq!(user_1_data.0, "User 1");
        assert_eq!(user_1_data.1, USER_1_DATA);

        let user_2_data = burndown_series.get(1).unwrap();
        assert_eq!(user_2_data.0, "User 2");
        assert_eq!(user_2_data.1, USER_2_DATA);

        let ideal_data = burndown_series.last().unwrap();
        assert_eq!(ideal_data.0, "Ideal");
        assert_eq!(ideal_data.1, IDEAL_DATA);
    }

    #[test]
    fn test_create_burndown_chart_per_user_with_sprints() {
        const SPRINTS: u16 = 2;
        const WEEKS_PER_SPRINT: u16 = 2;
        const TOTAL_DATA_LENGTH: usize = (SPRINTS + 1) as usize;
        const USER_1_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS_PER_PERSON, 5.0, 5.0];
        const USER_2_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS_PER_PERSON, 4.75, 0.75];
        const IDEAL_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS_PER_PERSON, 5.0, 0.0];

        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            PROJECT_START,
        );
        let (burndown_series, x_axis) = calculate_burndown_data(
            &time_logs,
            &BurndownType::PerPerson,
            &chart_options.unwrap(),
        );

        assert_eq!(x_axis, vec!["Start", "S1", "S2"]);

        let data = &burndown_series;
        let user_1_data = data.first().unwrap();
        assert_eq!(user_1_data.0, "User 1");
        assert_eq!(user_1_data.1, USER_1_DATA);

        let user_2_data = data.get(1).unwrap();
        assert_eq!(user_2_data.0, "User 2");
        assert_eq!(user_2_data.1, USER_2_DATA);

        let ideal_data = data.last().unwrap();
        assert_eq!(ideal_data.0, "Ideal");
        assert_eq!(ideal_data.1, IDEAL_DATA);
    }

    #[test]
    fn test_create_burndown_chart_total_no_sprints() {
        const TOTAL_DATA_LENGTH: usize = (PROJECT_WEEKS + 1) as usize;
        const NUMBER_OF_USERS: f32 = 2.0;
        const TOTAL_HOURS: f32 = NUMBER_OF_USERS * TOTAL_HOURS_PER_PERSON;
        const TOTAL_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS, 12.25, 9.75, 9.75, 5.75];
        const IDEAL_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS, 15.0, 10.0, 5.0, 0.0];

        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT_DEFAULT,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            PROJECT_START,
        );

        let (burndown_series, x_axis) =
            calculate_burndown_data(&time_logs, &BurndownType::Total, &chart_options.unwrap());

        assert_eq!(x_axis, vec!["Start", "W1", "W2", "W3", "W4"]);

        let total_data = burndown_series.first().unwrap();
        assert_eq!(total_data.0, "Total");
        assert_eq!(total_data.1, TOTAL_DATA);

        let ideal_data = burndown_series.last().unwrap();
        assert_eq!(ideal_data.0, "Ideal");
        assert_eq!(ideal_data.1, IDEAL_DATA);
    }

    #[test]
    fn test_create_burndown_chart_total_with_sprints() {
        const SPRINTS: u16 = 2;
        const WEEKS_PER_SPRINT: u16 = 2;
        const NUMBER_OF_USERS: f32 = 2.0;
        const TOTAL_HOURS: f32 = NUMBER_OF_USERS * TOTAL_HOURS_PER_PERSON;
        const TOTAL_DATA_LENGTH: usize = (SPRINTS + 1) as usize;
        const TOTAL_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS, 9.75, 5.75];
        const IDEAL_DATA: [f32; TOTAL_DATA_LENGTH] = [TOTAL_HOURS, 10.0, 0.0];

        let time_logs = get_time_logs();
        let chart_options = BurndownOptions::new(
            &time_logs,
            WEEKS_PER_SPRINT,
            SPRINTS,
            TOTAL_HOURS_PER_PERSON,
            PROJECT_START,
        );
        let (burndown_series, x_axis) =
            calculate_burndown_data(&time_logs, &BurndownType::Total, &chart_options.unwrap());

        assert_eq!(x_axis, vec!["Start", "S1", "S2"]);

        let total_data = burndown_series.first().unwrap();
        assert_eq!(total_data.0, "Total");
        assert_eq!(total_data.1, TOTAL_DATA);

        let ideal_data = burndown_series.last().unwrap();
        assert_eq!(ideal_data.0, "Ideal");
        assert_eq!(ideal_data.1, IDEAL_DATA);
    }

    #[test]
    fn test_create_ideal_burndown_line() {
        const SPRINTS: u16 = 4;
        const IDEAL_DATA_LENGTH: usize = (SPRINTS + 1) as usize;
        const REQUIRED_HOURS: f32 = 100.0;
        const IDEAL_DATA: [f32; IDEAL_DATA_LENGTH] = [REQUIRED_HOURS, 75.0, 50.0, 25.0, 0.0];

        let result = create_ideal_burndown_line(SPRINTS, REQUIRED_HOURS);
        assert_eq!(result.0, "Ideal");
        #[expect(clippy::float_cmp)]
        {
            assert_eq!(
                *result.1.first().unwrap(),
                REQUIRED_HOURS,
                "Ideal burndown line should start with the full amount of hours"
            );
            assert_eq!(
                *result.1.last().unwrap(),
                0.0,
                "Ideal burndown line should end with 0 hours"
            );
        }
        assert_eq!(result.1, IDEAL_DATA);
    }

    #[test]
    fn test_sprint_index() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let week_0 = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let week_4 = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let week_pre_start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        assert_eq!(sprint_index(start, WEEKS_PER_SPRINT_DEFAULT, week_0), 0);
        assert_eq!(sprint_index(start, WEEKS_PER_SPRINT_DEFAULT, week_4), 4);
        assert_eq!(
            sprint_index(start, WEEKS_PER_SPRINT_DEFAULT, week_pre_start),
            0
        );
    }
}
