//! Contains functions to generate the tables with time statistics.

use crate::model::{Label, TimeLog, User};
use crate::{TimeDeltaExt, filters};
use chrono::{Duration, Local};
use std::collections::{HashMap, HashSet};

/// Sums up all time logs by user for the last N days and returns a `HashMap`.
#[must_use]
fn get_total_time_by_user_in_last_n_days(
    time_logs: &[TimeLog],
    days: Duration,
) -> HashMap<&User, Duration> {
    let filtered = filters::filter_by_last_n_days(time_logs, days);
    filters::total_time_spent_by_user(filtered).collect()
}

/// Sums up all time logs by user for yesterday and returns a `HashMap`.
#[must_use]
fn get_total_time_by_user_yesterday(time_logs: &[TimeLog]) -> HashMap<&User, Duration> {
    let yesterday = Local::now().date_naive() - Duration::days(1);
    let filtered = filters::filter_by_date(time_logs, yesterday, yesterday);
    filters::total_time_spent_by_user(filtered).collect()
}

/// Helper function to get duration from a map, returning zero if not found
#[must_use]
fn get_duration_or_zero<'a>(map: &HashMap<&'a User, Duration>, user: &'a User) -> Duration {
    map.get(user).copied().unwrap_or(Duration::zero())
}

/// Assemble data for the table with time statistics for the last N days.
/// Returns a tuple of the table data and the table header.
#[must_use]
pub fn populate_table_timelogs_in_timeframes_by_user(
    time_logs: &[TimeLog],
) -> (Vec<Vec<String>>, &[&str]) {
    let today_timelogs = get_total_time_by_user_in_last_n_days(time_logs, Duration::days(1));
    let yesterday_timelogs = get_total_time_by_user_yesterday(time_logs);
    let this_week_timelogs = get_total_time_by_user_in_last_n_days(time_logs, Duration::weeks(1));
    let this_month_timelogs = get_total_time_by_user_in_last_n_days(time_logs, Duration::days(30));

    // Calculate totals for each timeframe
    let total_today = today_timelogs.values().copied().sum::<Duration>();
    let total_yesterday = yesterday_timelogs.values().copied().sum::<Duration>();
    let total_this_week = this_week_timelogs.values().copied().sum::<Duration>();
    let total_this_month = this_month_timelogs.values().copied().sum::<Duration>();
    let total_time = filters::total_time_spent(time_logs);

    let mut table_data: Vec<_> = filters::total_time_spent_by_user(time_logs)
        .map(|(user, total_time_of_user)| {
            vec![
                user.name.clone(),
                get_duration_or_zero(&today_timelogs, user).to_hm_string(),
                get_duration_or_zero(&yesterday_timelogs, user).to_hm_string(),
                get_duration_or_zero(&this_week_timelogs, user).to_hm_string(),
                get_duration_or_zero(&this_month_timelogs, user).to_hm_string(),
                total_time_of_user.to_hm_string(),
            ]
        })
        .collect();

    table_data.push(vec![
        "Total".to_string(),
        total_today.to_hm_string(),
        total_yesterday.to_hm_string(),
        total_this_week.to_hm_string(),
        total_this_month.to_hm_string(),
        total_time.to_hm_string(),
    ]);

    let table_header = &[
        "User",
        "Today",
        "Yesterday",
        "Last seven days",
        "Last 30 days",
        "Total time spent",
    ];

    (table_data, table_header)
}

/// Assemble data for a table with time statistics for the given labels.
/// Returns a tuple of the table data and the table header.
#[must_use]
pub fn populate_table_timelogs_by_label<'a>(
    time_logs: &[TimeLog],
    label_filter: Option<&HashSet<String>>,
    label_others: Option<&Label>,
) -> (Vec<Vec<String>>, &'a [&'a str]) {
    let time_by_label = filters::group_by_label(time_logs, label_filter, label_others);

    let table_data: Vec<_> = time_by_label
        .map(|(label, timelogs)| {
            let total_time = filters::total_time_spent(timelogs);
            let label_name = match label {
                Some(label) => label.title.clone(),
                None => "No label".to_string(),
            };
            vec![label_name, total_time.to_hm_string()]
        })
        .collect();

    let table_header = &["Label", "Time Spent"];

    (table_data, table_header)
}

/// Assemble data for a table with time statistics for all milestones.
/// Returns a tuple of the table data and the table header.
#[must_use]
pub fn populate_table_timelogs_by_milestone<'a>(
    time_logs: &[TimeLog],
) -> (Vec<Vec<String>>, &'a [&'a str]) {
    let time_by_milestone = filters::group_by_milestone(time_logs);

    let table_data: Vec<_> = time_by_milestone
        .map(|(milestone, timelogs)| {
            let total_time = filters::total_time_spent(timelogs);
            let title = milestone.map_or_else(|| "No milestone".to_string(), |m| m.title.clone());
            vec![title, total_time.to_hm_string()]
        })
        .collect();
    let table_header = &["Milestone", "Time Spent"];
    (table_data, table_header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Issue, Label, Labels, MergeRequest, Milestone, TimeLog, TrackableItem, TrackableItemFields,
        TrackableItemKind, User,
    };
    use chrono::{Duration, Local};

    const NUMBER_OF_LOGS: usize = 4;

    fn create_test_user(name: &str) -> User {
        User {
            username: name.to_string(),
            name: name.to_string(),
        }
    }

    fn get_timelogs() -> [TimeLog; NUMBER_OF_LOGS] {
        let now = Local::now();
        [
            TimeLog {
                spent_at: now - Duration::days(2),
                time_spent: Duration::seconds(3600),
                summary: None,
                user: User {
                    name: "user1".to_string(),
                    username: "user1".to_string(),
                },
                trackable_item: TrackableItem::default(),
            },
            TimeLog {
                spent_at: now - Duration::days(1),
                time_spent: Duration::seconds(3600),
                summary: Some("test".to_string()),
                user: User {
                    name: "user2".to_string(),
                    username: "user2".to_string(),
                },
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        milestone: Some(Milestone {
                            title: "End of Elaboration".to_string(),
                            ..Default::default()
                        }),
                        labels: Labels {
                            labels: vec![Label {
                                title: "Documentation".into(),
                            }],
                        },
                        ..Default::default()
                    },
                    kind: TrackableItemKind::Issue(Issue::default()),
                },
            },
            TimeLog {
                spent_at: now,
                time_spent: Duration::seconds(1800),
                summary: Some("test".to_string()),
                user: User {
                    name: "user1".to_string(),
                    username: "user1".to_string(),
                },
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        milestone: None,
                        labels: Labels {
                            labels: vec![
                                Label {
                                    title: "Documentation".into(),
                                },
                                Label {
                                    title: "Development".into(),
                                },
                            ],
                        },
                        ..Default::default()
                    },
                    kind: TrackableItemKind::MergeRequest(MergeRequest::default()),
                },
            },
            TimeLog {
                spent_at: now,
                time_spent: Duration::seconds(5400),
                summary: Some("Fix a big bug".to_string()),
                user: User {
                    name: "user3".to_string(),
                    username: "user3".to_string(),
                },
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        labels: Labels {
                            labels: vec![
                                Label {
                                    title: "Bug".into(),
                                },
                                Label {
                                    title: "Development".into(),
                                },
                            ],
                        },
                        ..Default::default()
                    },
                    kind: TrackableItemKind::MergeRequest(MergeRequest::default()),
                },
            },
        ]
    }

    /// Helper function: turn `&User` key map into name & Duration for easy assertions.
    fn to_name_map(m: &HashMap<&User, Duration>) -> HashMap<String, Duration> {
        m.iter()
            .map(|(u, d)| (u.name.clone(), *d))
            .collect::<HashMap<_, _>>()
    }

    #[test]
    fn test_get_duration_or_zero_existing_user() {
        const DURATION: Duration = Duration::seconds(1200);
        let user = create_test_user("user1");
        let mut map = HashMap::new();
        map.insert(&user, DURATION);

        let result = get_duration_or_zero(&map, &user);
        assert_eq!(result, DURATION);
    }

    #[test]
    fn test_get_duration_or_zero_missing_user() {
        const DURATION: Duration = Duration::seconds(1200);
        let user_in_map = create_test_user("user1");
        let user_not_in_map = create_test_user("user4");
        let mut map = HashMap::new();
        map.insert(&user_in_map, DURATION);

        let result = get_duration_or_zero(&map, &user_not_in_map);
        assert_eq!(result, Duration::zero());
    }

    #[test]
    fn test_total_time_by_user_yesterday() {
        const NUMBER_OF_USERS: usize = 1;
        const TIME_SPENT: Duration = Duration::seconds(3600);

        let time_logs = get_timelogs();
        let result = get_total_time_by_user_yesterday(&time_logs);
        let name_map = to_name_map(&result);

        assert_eq!(name_map.len(), NUMBER_OF_USERS);
        assert_eq!(name_map.get("user2"), Some(&TIME_SPENT));
    }

    #[test]
    fn test_total_time_by_user_in_last_n_days_empty() {
        const N_DAYS: Duration = Duration::days(7);
        let time_logs = vec![];
        let result = get_total_time_by_user_in_last_n_days(&time_logs, N_DAYS);
        assert!(result.is_empty());
    }

    #[test]
    fn test_total_time_by_user_in_last_n_days_one_day() {
        const N_DAYS: Duration = Duration::days(1);
        const TIME_SPENT_USER_1: Duration = Duration::seconds(1800);
        const TIME_SPENT_USER_3: Duration = Duration::seconds(5400);

        let time_logs = get_timelogs();
        let result = get_total_time_by_user_in_last_n_days(&time_logs, N_DAYS);
        let name_map = to_name_map(&result);

        assert_eq!(name_map.len(), 2);
        assert_eq!(name_map.get("user1"), Some(&TIME_SPENT_USER_1));
        assert_eq!(name_map.get("user3"), Some(&TIME_SPENT_USER_3));
        assert!(!name_map.contains_key("user2"));
    }

    #[test]
    fn test_total_time_by_user_in_last_n_days_seven_days() {
        const N_DAYS: Duration = Duration::days(7);
        const TIME_SPENT_USER_1: Duration = Duration::seconds(5400);
        const TIME_SPENT_USER_2: Duration = Duration::seconds(3600);
        const TIME_SPENT_USER_3: Duration = Duration::seconds(5400);

        let time_logs = get_timelogs();
        let result = get_total_time_by_user_in_last_n_days(&time_logs, N_DAYS);
        let name_map = to_name_map(&result);

        assert_eq!(name_map.get("user1"), Some(&TIME_SPENT_USER_1));
        assert_eq!(name_map.get("user2"), Some(&TIME_SPENT_USER_2));
        assert_eq!(name_map.get("user3"), Some(&TIME_SPENT_USER_3));
    }

    #[test]
    fn test_create_table_timelogs_in_timeframes_by_user_header_and_rows() {
        const NUMBER_OF_STATS: usize = 5;
        const NUMBER_OF_COLUMNS: usize = NUMBER_OF_STATS + 1;

        let time_logs = get_timelogs();
        let (table, _header) = populate_table_timelogs_in_timeframes_by_user(&time_logs);

        // We should have one row per user
        let name_column: HashSet<String> = table.iter().map(|row| row[0].clone()).collect();
        assert!(name_column.contains("user1"));
        assert!(name_column.contains("user2"));
        assert!(name_column.contains("user3"));

        // Each row should have 6 columns: user + 5 stats
        for row in &table {
            assert_eq!(row.len(), NUMBER_OF_COLUMNS);
        }
    }

    #[test]
    fn test_create_table_timelogs_by_label() {
        let time_logs = get_timelogs();
        let (table, _header) = populate_table_timelogs_by_label(&time_logs, None, None);

        let label_time_spent_map: HashMap<String, String> = table
            .into_iter()
            .map(|row| (row[0].clone(), row[1].clone()))
            .collect();

        let expected_map = std::collections::HashMap::from([
            ("Documentation".to_string(), "01h 30m".to_string()),
            ("Development".to_string(), "02h 00m".to_string()),
            ("Bug".to_string(), "01h 30m".to_string()),
            ("No label".to_string(), "01h 00m".to_string()),
        ]);

        assert_eq!(label_time_spent_map, expected_map);
    }

    #[test]
    fn test_create_table_timelogs_by_label_with_others() {
        let time_logs = get_timelogs();
        let label_documentation = Label {
            title: "Documentation".into(),
        };
        let label_others = Label {
            title: "Others".into(),
        };

        let selected = HashSet::from([label_documentation.title.clone()]);
        let (table, _) =
            populate_table_timelogs_by_label(&time_logs, Some(&selected), Some(&label_others));

        let label_time_spent_map: HashMap<String, String> = table
            .into_iter()
            .map(|row| (row[0].clone(), row[1].clone()))
            .collect();

        let expected_map = HashMap::from([
            ("Documentation".to_string(), "01h 30m".to_string()),
            ("Others".to_string(), "02h 30m".to_string()),
        ]);
        assert_eq!(label_time_spent_map, expected_map);
    }
}
