//! Calculates the estimates from [`crate::model::TrackableItem`] and prepares the data for chart creation.

use super::SeriesData;
use super::charming_extensions::MultiSeries;
use crate::TimeDeltaExt;
use crate::filters::group_by_trackable_item;
use crate::model::TimeLog;
use chrono::Duration;
use std::collections::BTreeMap;

/// Contains the tracked time per each `TrackableItem`
struct TrackedTime {
    /// The total time spent per `TrackableItems`
    time_spent: Duration,
    /// The estimate on the `TrackableItem`
    estimate: Duration,
}

/// Calculates the estimate and the actual time spent of all items of type `T`.
/// Data can be converted into a [`charming`] chart.
pub(super) fn calculate_estimate_data<'a, T, Series>(
    grouped_time_log: BTreeMap<impl Into<Option<&'a T>> + Clone, Vec<&'a TimeLog>>,
) -> (SeriesData, Vec<String>)
where
    T: std::fmt::Display + 'a,
    Series: MultiSeries,
{
    let capacity = grouped_time_log.len();
    let mut axis_labels = Vec::with_capacity(capacity);
    let mut estimate_sums = Vec::with_capacity(capacity);
    let mut actual_sums = Vec::with_capacity(capacity);

    for (outer_key, time_logs) in grouped_time_log {
        axis_labels.push(Series::option_to_string(outer_key));

        // Get the estimate and actual time per Issue/MR and store it into TrackedTime
        let tracked_time_by_item = group_by_trackable_item(time_logs)
            .map(|(trackable_item, _)| TrackedTime {
                estimate: trackable_item.common.time_estimate,
                time_spent: trackable_item.common.total_time_spent,
            })
            .collect::<Vec<_>>();

        // Sum up all the times from every TrackableItem of this grouping (i.e., Label)
        let (estimate_sum, actual_sum) = tracked_time_by_item.iter().fold(
            (Duration::zero(), Duration::zero()),
            |(est, act), tracked| (est + tracked.estimate, act + tracked.time_spent),
        );

        estimate_sums.push(estimate_sum.total_hours());
        actual_sums.push(actual_sum.total_hours());
    }

    let series = vec![
        ("Estimates".into(), estimate_sums),
        ("Actual Time".into(), actual_sums),
    ];

    (series, axis_labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charts::tests::*;
    use crate::filters::group_by_milestone;
    use crate::model::Milestone;
    use charming::series::Bar;

    #[test]
    fn test_calculate_estimate_data() {
        const AXIS_LABELS: &[&str] = &["None", "M1", "M2"];
        const NUMBER_OF_SERIES: usize = 2;
        const NUMBER_OF_DATA_POINTS: usize = 3;
        const ESTIMATES_DATA: [f32; NUMBER_OF_DATA_POINTS] = [2.0, 7.0, 2.0];
        const ACTUAL_DATA: [f32; NUMBER_OF_DATA_POINTS] = [5.0, 7.75, 1.5];

        let time_logs = get_time_logs();
        let by_milestone = group_by_milestone(&time_logs).collect();
        let (series, axis_labels) = calculate_estimate_data::<Milestone, Bar>(by_milestone);

        assert_eq!(axis_labels, AXIS_LABELS);
        assert_eq!(series.len(), NUMBER_OF_SERIES);

        let estimate_data = series.first().unwrap();
        assert_eq!(estimate_data.0, "Estimates");
        assert_eq!(estimate_data.1, ESTIMATES_DATA);

        let actual_data = series.last().unwrap();
        assert_eq!(actual_data.0, "Actual Time");
        assert_eq!(actual_data.1, ACTUAL_DATA);
    }
}
