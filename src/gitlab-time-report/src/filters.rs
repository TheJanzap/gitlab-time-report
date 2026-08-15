//! Contains functions to filter and group [`TimeLog`].

use crate::model::{Label, Milestone, TimeLog, TrackableItem, User};
use chrono::{Duration, Local, NaiveDate};
use std::collections::{BTreeMap, HashSet};

/// Groups a list of nodes by a given filter. Returns an Iterator over a `BTreeMap`.
/// # Parameter
/// - `nodes`: A collection of time log nodes. Can be any collection that can be turned into an iterator.
/// - `filter`: A function to group the nodes by, usually a closure. It takes a reference to a time
///   log and returns a reference to the item to group by.
///
/// The lifetime annotation `'a` means that the references to `T` and `TimeLog` must both live at
/// least as long as each other.
pub fn group_by_filter<'a, T, I, F>(
    nodes: I,
    filter: F,
) -> impl Iterator<Item = (&'a T, Vec<&'a TimeLog>)>
where
    T: Ord + 'a,
    I: IntoIterator<Item = &'a TimeLog>,
    F: Fn(&'a TimeLog) -> &'a T,
{
    let mut grouped = BTreeMap::<&'a T, Vec<&'a TimeLog>>::new();
    for node in nodes {
        let key = filter(node);
        grouped.entry(key).or_default().push(node);
    }
    grouped.into_iter()
}

/// Groups a slice of nodes by their user and returns a iterator over a `BTreeMap`.
/// # Example
/// ```
/// # use gitlab_time_report::model::{TimeLog, User};
/// # use gitlab_time_report::filters::group_by_user;
/// # use std::collections::BTreeMap;
/// let nodes = vec![
///     TimeLog { user: User { name: "user1".to_string(), username: "user1".to_string() }, ..Default::default() },
///     TimeLog { user: User { name: "user1".to_string(), username: "user1".to_string() }, ..Default::default() },
///     TimeLog { user: User { name: "user2".to_string(), username: "user2".to_string() }, ..Default::default() }
/// ];
/// let grouped_by_user = group_by_user(&nodes).collect::<BTreeMap<_, _>>();
/// assert_eq!(grouped_by_user.len(), 2);
/// assert_eq!(grouped_by_user.get(&User { name: "user1".to_string(), username: "user1".to_string() }).unwrap().len(), 2);
/// assert_eq!(grouped_by_user.get(&User { name: "user2".to_string(), username: "user2".to_string() }).unwrap().len(), 1);
/// ```
pub fn group_by_user<'a>(
    nodes: impl IntoIterator<Item = &'a TimeLog>,
) -> impl Iterator<Item = (&'a User, Vec<&'a TimeLog>)> {
    group_by_filter(nodes, |node| &node.user)
}

/// Filter the `TimeLog` by milestone and returns an iterator over `(Option<&Milestone>, Vec<&TimeLog>)`.
pub fn group_by_milestone<'a>(
    nodes: impl IntoIterator<Item = &'a TimeLog>,
) -> impl Iterator<Item = (Option<&'a Milestone>, Vec<&'a TimeLog>)> {
    group_by_filter(nodes, |node| &node.trackable_item.common.milestone)
        .map(|(milestone, nodes)| (milestone.as_ref(), nodes))
}

/// Group the `TimeLog` by type (Issue/MR) and return an iterator. Will return a `String` containing the type name.
/// Because `TrackableItemKind` is an enum where each variant carries data, it is not sensible to group directly on
/// this type, because if the data inside is different, it counts as a different element.
pub fn group_by_type<'a, 'b>(
    nodes: impl IntoIterator<Item = &'a TimeLog>,
) -> impl Iterator<Item = (String, Vec<&'a TimeLog>)> {
    let mut grouped = BTreeMap::<String, Vec<&'a TimeLog>>::new();
    for node in nodes {
        let key = node.trackable_item.kind.to_string();
        grouped.entry(key).or_default().push(node);
    }
    grouped.into_iter()
}

/// Group the `TimeLog` by trackable item (Issue/MR) and return an iterator.
pub fn group_by_trackable_item<'a>(
    time_logs: impl IntoIterator<Item = &'a TimeLog>,
) -> impl Iterator<Item = (&'a TrackableItem, Vec<&'a TimeLog>)> {
    group_by_filter(time_logs, |node| &node.trackable_item)
}

/// Group the `TimeLog`s by labels and return an iterator.
/// Note that items with multiple labels are included multiple times.
/// Items with no labels are included in the label `other_label`, if set.
/// # Parameters
/// - `selected_labels`: A list of labels that should be included in the result.
///   If this `Some`, only `TimeLog`s with at least one of the selected labels
///   are grouped under the corresponding label. If this is `None`, all labels are selected.
/// - `other_label`: The label used for items without any of the selected labels.
///   If this is `Some`, all items not in `selected_labels` are grouped under this label.
///   If this is `None`, all items not in `selected_labels` are ignored.
pub fn group_by_label<'a>(
    nodes: impl IntoIterator<Item = &'a TimeLog>,
    selected_labels: Option<&HashSet<String>>,
    other_label: Option<&'a Label>,
) -> impl Iterator<Item = (Option<&'a Label>, Vec<&'a TimeLog>)> {
    let mut label_map = BTreeMap::<Option<&Label>, Vec<&TimeLog>>::new();

    for time_log in nodes {
        let labels = &time_log.trackable_item.common.labels.labels;

        if labels.is_empty() {
            // TrackableItem has no labels
            // Add None entry if selected_labels is not set
            if selected_labels.is_none() {
                label_map.entry(None).or_default().push(time_log);
                continue;
            }

            // Add to the "Other" label if selected_labels is set
            if other_label.is_some() {
                label_map.entry(other_label).or_default().push(time_log);
            }
            continue;
        }

        let mut matched_any_selected_label = false;

        for label in labels {
            // True if selected_labels is None or if the label is in the selected_labels
            let should_include_label =
                selected_labels.is_none_or(|sel_labels| sel_labels.contains(&label.title));

            if should_include_label {
                label_map.entry(Some(label)).or_default().push(time_log);
                matched_any_selected_label = true;
            }
        }

        // When label is not under the selected one, add to the "Other" label
        if other_label.is_some() && !matched_any_selected_label {
            label_map.entry(other_label).or_default().push(time_log);
        }
    }
    label_map.into_iter()
}

/// Filter the `TimeLog` by date and return an iterator. The dates are both inclusive.
pub fn filter_by_date<'a>(
    nodes: impl IntoIterator<Item = &'a TimeLog>,
    start: NaiveDate,
    end: NaiveDate,
) -> impl Iterator<Item = &'a TimeLog> {
    nodes.into_iter().filter(move |node| {
        let spent_day = node.spent_at.with_timezone(&Local).date_naive();
        spent_day >= start && spent_day <= end
    })
}

/// Returns the time logs in the last X days. The number of days is specified as `Duration`.
/// 1 day is the current day, 2 days is today and yesterday.
pub fn filter_by_last_n_days<'a>(
    time_logs: impl IntoIterator<Item = &'a TimeLog>,
    days: Duration,
) -> impl Iterator<Item = &'a TimeLog> {
    let end: NaiveDate = Local::now().date_naive();
    let start: NaiveDate = end - days + Duration::days(1);
    filter_by_date(time_logs, start, end)
}

/// Returns the total time spent on the project.
#[must_use]
pub fn total_time_spent<'a>(time_logs: impl IntoIterator<Item = &'a TimeLog>) -> Duration {
    time_logs
        .into_iter()
        .map(|node| node.time_spent)
        .sum::<Duration>()
}

/// Returns the total time spent on the project by every user.
pub fn total_time_spent_by_user<'a>(
    time_logs: impl IntoIterator<Item = &'a TimeLog>,
) -> impl Iterator<Item = (&'a User, Duration)> {
    group_by_user(time_logs).map(|(user, timelogs)| (user, total_time_spent(timelogs)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Issue, Labels, MergeRequest, TrackableItem, TrackableItemFields, TrackableItemKind,
    };
    use std::assert_matches;

    const NUMBER_OF_LOGS: usize = 5;

    #[expect(clippy::too_many_lines)]
    fn get_timelogs() -> [TimeLog; NUMBER_OF_LOGS] {
        let user1 = User {
            name: "user1".to_string(),
            username: "user1".to_string(),
        };
        let user2 = User {
            name: "user2".to_string(),
            username: "user2".to_string(),
        };

        [
            TimeLog {
                spent_at: Local::now() - Duration::days(2),
                time_spent: Duration::seconds(3600),
                summary: None,
                user: user1.clone(),
                trackable_item: TrackableItem::default(),
            },
            TimeLog {
                spent_at: Local::now() - Duration::days(5),
                time_spent: Duration::seconds(7200),
                summary: Some("First entry".to_string()),
                user: user2.clone(),
                trackable_item: TrackableItem::default(),
            },
            TimeLog {
                spent_at: Local::now() - Duration::days(1),
                time_spent: Duration::seconds(3600),
                summary: Some("test".to_string()),
                user: user2.clone(),
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        id: 1,
                        title: "Second Issue".to_string(),
                        milestone: Some(Milestone {
                            title: "End of Elaboration".to_string(),
                            ..Default::default()
                        }),
                        labels: Labels {
                            labels: vec![Label {
                                title: "Documentation".into(),
                            }],
                        },
                        time_estimate: Duration::seconds(7200),
                        ..Default::default()
                    },
                    kind: TrackableItemKind::Issue(Issue::default()),
                },
            },
            TimeLog {
                spent_at: Local::now(),
                time_spent: Duration::seconds(1800),
                summary: Some("test".to_string()),
                user: user1.clone(),
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        title: "First MR".into(),
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
                        time_estimate: Duration::seconds(1800),
                        ..Default::default()
                    },
                    kind: TrackableItemKind::MergeRequest(MergeRequest::default()),
                },
            },
            TimeLog {
                spent_at: Local::now(),
                time_spent: Duration::seconds(5400),
                summary: Some("Fix a big bug".to_string()),
                user: User {
                    name: "user3".to_string(),
                    username: "user3".to_string(),
                },
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        id: 1,
                        title: "Second MR".into(),
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
                        time_estimate: Duration::seconds(3600),
                        ..Default::default()
                    },
                    kind: TrackableItemKind::MergeRequest(MergeRequest::default()),
                },
            },
        ]
    }

    #[test]
    fn test_group_by_user() {
        const NUMBER_OF_USERS: usize = 3;
        const NUMBER_OF_USER1_LOGS: usize = 2;
        const NUMBER_OF_USER2_LOGS: usize = 2;
        const NUMBER_OF_USER3_LOGS: usize = 1;

        let input = get_timelogs();
        assert_eq!(input.len(), NUMBER_OF_LOGS);

        let output = group_by_user(&input).collect::<BTreeMap<_, _>>();
        let user1 = User {
            name: "user1".to_string(),
            username: "user1".to_string(),
        };
        let user2 = User {
            name: "user2".to_string(),
            username: "user2".to_string(),
        };
        let user3 = User {
            name: "user3".to_string(),
            username: "user3".to_string(),
        };

        assert_eq!(output.len(), NUMBER_OF_USERS);
        assert_eq!(output.get(&user1).unwrap().len(), NUMBER_OF_USER1_LOGS);
        assert_eq!(output.get(&user2).unwrap().len(), NUMBER_OF_USER2_LOGS);
        assert_eq!(output.get(&user3).unwrap().len(), NUMBER_OF_USER3_LOGS);
    }

    #[test]
    fn test_group_by_milestone() {
        const NUMBER_OF_MILESTONES: usize = 2;
        const NUMBER_OF_NONE: usize = 4;
        const NUMBER_OF_SOME: usize = 1;

        let input = get_timelogs();
        assert_eq!(input.len(), NUMBER_OF_LOGS);

        let milestone_none = None;
        let milestone_some = Some(&Milestone {
            title: "End of Elaboration".to_string(),
            ..Default::default()
        });

        let output = group_by_milestone(&input).collect::<BTreeMap<_, _>>();

        assert_eq!(output.len(), NUMBER_OF_MILESTONES);

        let output_none = output.get(&milestone_none).unwrap();
        let output_some = output.get(&milestone_some).unwrap();
        assert_eq!(output_none.len(), NUMBER_OF_NONE);
        assert_eq!(output_some.len(), NUMBER_OF_SOME);

        // Verify the correct timelogs are grouped
        assert!(output_none.contains(&&input[0]));
        assert!(output_none.contains(&&input[1]));
        assert!(output_some.contains(&&input[2]));
        assert!(output_none.contains(&&input[3]));
        assert!(output_none.contains(&&input[4]));
    }

    #[test]
    fn test_group_by_type() {
        const NUMBER_OF_TYPES: usize = 2;
        const NUMBER_OF_ISSUES: usize = 3;
        const NUMBER_OF_MERGE_REQUESTS: usize = 2;

        let input = get_timelogs();
        assert_eq!(input.len(), NUMBER_OF_LOGS);

        let output = group_by_type(&input).collect::<BTreeMap<_, _>>();

        assert_eq!(output.len(), NUMBER_OF_TYPES);
        assert_eq!(output.get("Issue").unwrap().len(), NUMBER_OF_ISSUES);
        assert_eq!(
            output.get("Merge Request").unwrap().len(),
            NUMBER_OF_MERGE_REQUESTS
        );
    }

    #[test]
    fn test_group_by_trackable_item() {
        const NUMBER_OF_ITEMS: usize = 4;
        const NUMBER_OF_ISSUE_0: usize = 2;
        const NUMBER_OF_ISSUE_1: usize = 1;
        const NUMBER_OF_MR_0: usize = 1;
        const NUMBER_OF_MR_1: usize = 1;

        let input = get_timelogs();
        let mut result = group_by_trackable_item(&input).collect::<BTreeMap<_, _>>();
        assert_eq!(result.len(), NUMBER_OF_ITEMS);

        let item_1 = result.pop_first().unwrap();
        assert_matches!(item_1.0.kind, TrackableItemKind::Issue(_));

        assert_eq!(item_1.0.common.id, 0);
        assert_eq!(item_1.1.len(), NUMBER_OF_ISSUE_0);

        let item_2 = result.pop_first().unwrap();
        assert_matches!(&item_2.0.kind, TrackableItemKind::MergeRequest(_));

        assert_eq!(item_2.0.common.id, 0);
        assert_eq!(item_2.1.len(), NUMBER_OF_MR_0);

        let item_3 = result.pop_first().unwrap();
        assert_matches!(item_3.0.kind, TrackableItemKind::Issue(_));

        assert_eq!(item_3.0.common.id, 1);
        assert_eq!(item_3.1.len(), NUMBER_OF_ISSUE_1);

        let item_4 = result.pop_first().unwrap();
        assert_matches!(item_4.0.kind, TrackableItemKind::MergeRequest(_));

        assert_eq!(item_4.0.common.id, 1);
        assert_eq!(item_4.1.len(), NUMBER_OF_MR_1);
    }

    #[test]
    fn test_group_by_label_contains_selected_labels() {
        const NUMBER_OF_SELECTED_LABELS: usize = 2;

        let input = get_timelogs();

        let label_documentation = Some(&Label {
            title: "Documentation".to_string(),
        });
        let label_development = Some(&Label {
            title: "Development".to_string(),
        });
        let label_bug = Some(&Label {
            title: "Bug".to_string(),
        });
        let label_others = Some(&Label {
            title: "Others".to_string(),
        });

        #[expect(clippy::unnecessary_literal_unwrap)]
        let label_filter = HashSet::from([
            label_development.unwrap().title.clone(),
            label_documentation.unwrap().title.clone(),
        ]);
        assert_eq!(label_filter.len(), NUMBER_OF_SELECTED_LABELS);

        let result = group_by_label(&input, Some(&label_filter), None).collect::<BTreeMap<_, _>>();

        assert_eq!(result.len(), NUMBER_OF_SELECTED_LABELS);
        assert!(result.contains_key(&label_documentation));
        assert!(result.contains_key(&label_development));
        assert!(!result.contains_key(&label_bug));
        assert!(!result.contains_key(&label_others));
        assert!(!result.contains_key(&None));
    }

    #[test]
    fn test_group_by_label_contains_items_without_labels() {
        const NUMBER_OF_LABELS_INCLUDING_NO_LABEL: usize = 4;
        const NUMBER_OF_NO_LABEL: usize = 2;
        const TIME_SPENT_BY_NO_LABEL: Duration = Duration::seconds(10800);

        let time_logs = get_timelogs();
        let result = group_by_label(&time_logs, None, None).collect::<BTreeMap<_, _>>();

        assert_eq!(result.len(), NUMBER_OF_LABELS_INCLUDING_NO_LABEL);
        assert!(result.contains_key(&None));
        let no_label = result.get(&None).unwrap();
        assert_eq!(no_label.len(), NUMBER_OF_NO_LABEL);
        assert_eq!(
            no_label.iter().map(|t| t.time_spent).sum::<Duration>(),
            TIME_SPENT_BY_NO_LABEL
        );
    }

    #[test]
    fn test_group_by_label_none_selected_labels_contains_all_labels() {
        const NUMBER_OF_ALL_LABELS: usize = 3;

        let input = get_timelogs();

        let label_documentation = Some(&Label {
            title: "Documentation".to_string(),
        });
        let label_development = Some(&Label {
            title: "Development".to_string(),
        });
        let label_bug = Some(&Label {
            title: "Bug".to_string(),
        });
        let label_others = Some(&Label {
            title: "Others".to_string(),
        });

        let result = group_by_label(&input, None, None).collect::<BTreeMap<_, _>>();
        // All labels + 1 for No label
        assert_eq!(result.len(), NUMBER_OF_ALL_LABELS + 1);
        assert!(result.contains_key(&label_documentation));
        assert!(result.contains_key(&label_development));
        assert!(result.contains_key(&label_bug));
        assert!(result.contains_key(&None));
        assert!(!result.contains_key(&label_others));
    }

    #[test]
    fn test_group_by_label_with_other_label() {
        const NUMBER_OF_SELECTED_LABELS: usize = 2;
        const NUMBER_OF_DOCUMENTATION: usize = 2;
        const NUMBER_OF_DEVELOPMENT: usize = 2;
        const NUMBER_OF_OTHERS: usize = 2;

        const TIME_SPENT_BY_DOCUMENTATION: Duration = Duration::seconds(5400);
        const TIME_SPENT_BY_DEVELOPMENT: Duration = Duration::seconds(7200);
        const TIME_SPENT_BY_OTHERS: Duration = Duration::seconds(10800);

        let input = get_timelogs();
        assert_eq!(input.len(), NUMBER_OF_LOGS);

        let label_documentation = Some(&Label {
            title: "Documentation".to_string(),
        });
        let label_development = Some(&Label {
            title: "Development".to_string(),
        });
        let label_bug = Some(&Label {
            title: "Bug".to_string(),
        });
        let label_others = Label {
            title: "Others".to_string(),
        };

        #[expect(clippy::unnecessary_literal_unwrap)]
        let label_filter = HashSet::from([
            label_development.unwrap().title.clone(),
            label_documentation.unwrap().title.clone(),
        ]);
        assert_eq!(label_filter.len(), NUMBER_OF_SELECTED_LABELS);

        let result = group_by_label(&input, Some(&label_filter), Some(&label_others))
            .collect::<BTreeMap<_, _>>();

        // Selected labels + 1 for the "Other" label
        assert_eq!(result.len(), NUMBER_OF_SELECTED_LABELS + 1);

        // Check the contents of the labels
        let result_documentation = result.get(&label_documentation).unwrap();
        assert_eq!(result_documentation.len(), NUMBER_OF_DOCUMENTATION);
        assert_eq!(
            result_documentation
                .iter()
                .map(|t| t.time_spent)
                .sum::<Duration>(),
            TIME_SPENT_BY_DOCUMENTATION
        );

        let result_development = result.get(&label_development).unwrap();
        assert_eq!(result_development.len(), NUMBER_OF_DEVELOPMENT);
        assert_eq!(
            result_development
                .iter()
                .map(|t| t.time_spent)
                .sum::<Duration>(),
            TIME_SPENT_BY_DEVELOPMENT
        );

        let result_others = result.get(&Some(&label_others)).unwrap();
        assert_eq!(result_others.len(), NUMBER_OF_OTHERS);
        assert_eq!(
            result_others.iter().map(|t| t.time_spent).sum::<Duration>(),
            TIME_SPENT_BY_OTHERS
        );
        assert!(!result.contains_key(&label_bug));
    }

    #[test]
    fn test_group_by_label_without_other_label() {
        const NUMBER_OF_LABELS: usize = 3;
        const NUMBER_OF_DOCUMENTATION: usize = 2;
        const NUMBER_OF_DEVELOPMENT: usize = 2;
        const NUMBER_OF_BUGS: usize = 1;

        const TIME_SPENT_BY_DOCUMENTATION: Duration = Duration::seconds(5400);
        const TIME_SPENT_BY_DEVELOPMENT: Duration = Duration::seconds(7200);
        const TIME_SPENT_BY_BUGS: Duration = Duration::seconds(5400);

        let input = get_timelogs();
        assert_eq!(input.len(), NUMBER_OF_LOGS);

        let label_documentation = Some(&Label {
            title: "Documentation".to_string(),
        });
        let label_development = Some(&Label {
            title: "Development".to_string(),
        });
        let label_bug = Some(&Label {
            title: "Bug".to_string(),
        });

        #[expect(clippy::unnecessary_literal_unwrap)]
        let label_filter = HashSet::from([
            label_development.unwrap().title.clone(),
            label_documentation.unwrap().title.clone(),
            label_bug.unwrap().title.clone(),
        ]);

        let result = group_by_label(&input, Some(&label_filter), None).collect::<BTreeMap<_, _>>();
        assert_eq!(result.len(), NUMBER_OF_LABELS);

        // Check the contents of the labels
        let result_documentation = result.get(&label_documentation).unwrap();
        assert_eq!(result_documentation.len(), NUMBER_OF_DOCUMENTATION);
        assert_eq!(
            result_documentation
                .iter()
                .map(|t| t.time_spent)
                .sum::<Duration>(),
            TIME_SPENT_BY_DOCUMENTATION
        );

        let result_development = result.get(&label_development).unwrap();
        assert_eq!(result_development.len(), NUMBER_OF_DEVELOPMENT);
        assert_eq!(
            result_development
                .iter()
                .map(|t| t.time_spent)
                .sum::<Duration>(),
            TIME_SPENT_BY_DEVELOPMENT
        );

        let result_bug = result.get(&label_bug).unwrap();
        assert_eq!(result_bug.len(), NUMBER_OF_BUGS);
        assert_eq!(
            result_bug.iter().map(|t| t.time_spent).sum::<Duration>(),
            TIME_SPENT_BY_BUGS
        );

        assert!(!result.contains_key(&None));
    }

    #[test]
    fn test_total_time_spent_by_user() {
        const NUMBER_OF_USERS: usize = 3;
        const TIME_SPENT_BY_USER_1: Duration = Duration::seconds(5400);
        const TIME_SPENT_BY_USER_2: Duration = Duration::seconds(10800);
        const TIME_SPENT_BY_USER_3: Duration = Duration::seconds(5400);

        let input = get_timelogs();
        assert_eq!(input.len(), NUMBER_OF_LOGS);

        let totals = total_time_spent_by_user(&input).collect::<BTreeMap<_, _>>();

        let user1 = User {
            name: "user1".to_string(),
            username: "user1".to_string(),
        };
        let user2 = User {
            name: "user2".to_string(),
            username: "user2".to_string(),
        };
        let user3 = User {
            name: "user3".to_string(),
            username: "user3".to_string(),
        };

        assert_eq!(totals.len(), NUMBER_OF_USERS);
        assert_eq!(totals.get(&user1), Some(&TIME_SPENT_BY_USER_1));
        assert_eq!(totals.get(&user2), Some(&TIME_SPENT_BY_USER_2));
        assert_eq!(totals.get(&user3), Some(&TIME_SPENT_BY_USER_3));
    }

    #[test]
    fn test_filter_by_dates() {
        const NUMBER_OF_FILTERED_LOGS: usize = 3;

        let input = get_timelogs();
        assert_eq!(input.len(), NUMBER_OF_LOGS);

        // should take today and yesterday
        let end = Local::now().date_naive();
        let start = end - Duration::days(1);
        let output = filter_by_date(&input, start, end).collect::<Vec<_>>();

        assert_eq!(output.len(), NUMBER_OF_FILTERED_LOGS);
        for node in output {
            let spent_day = node.spent_at.with_timezone(&Local).date_naive();
            assert!(spent_day >= start && spent_day <= end);
        }
    }

    #[test]
    fn test_filter_by_last_n_days() {
        const NUMBER_OF_FILTERED_LOGS: usize = 3;
        const NUMBER_OF_DAYS: i64 = 2;

        let input = get_timelogs();
        let output =
            filter_by_last_n_days(&input, Duration::days(NUMBER_OF_DAYS)).collect::<Vec<_>>();

        let end = Local::now();
        let start = end - Duration::days(NUMBER_OF_DAYS);

        assert_eq!(output.len(), NUMBER_OF_FILTERED_LOGS);
        for log in output {
            assert!(log.spent_at >= start && log.spent_at <= end);
        }
    }
}
