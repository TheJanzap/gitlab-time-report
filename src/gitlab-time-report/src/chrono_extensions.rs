//! Convenience functions for the types provided by the [`chrono`] crate.

use chrono::TimeDelta;

/// Functions to simplify the handling of [`TimeDelta`]
pub trait TimeDeltaExt {
    fn to_hm_string(&self) -> String;
    fn total_hours(&self) -> f32;
}

impl TimeDeltaExt for TimeDelta {
    /// Turn a `TimeDelta` into a string of the format `"00h 00m"`.
    /// # Example
    /// ```
    /// # use chrono::TimeDelta;
    /// # use gitlab_time_report::TimeDeltaExt;
    /// let delta = TimeDelta::hours(1) + TimeDelta::minutes(2) + TimeDelta::seconds(3);
    /// assert_eq!(delta.to_hm_string(), "01h 02m");
    /// ```
    fn to_hm_string(&self) -> String {
        // TimeDelta doesn't have a format() function like DateTime, so we have to do it manually
        let hours = self.num_hours();
        let minutes = self.num_minutes() % 60;
        format!("{hours:02}h {minutes:02}m")
    }

    /// Get the total hours, minutes and seconds of a `TimeDelta` as a `String`.
    fn total_hours(&self) -> f32 {
        self.as_seconds_f32() / 3600.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_readable_string() {
        let delta = TimeDelta::hours(1) + TimeDelta::minutes(45) + TimeDelta::seconds(3);
        assert_eq!(delta.to_hm_string(), "01h 45m");
    }

    #[test]
    #[expect(clippy::float_cmp)]
    fn test_total_hours() {
        let delta = TimeDelta::hours(5) + TimeDelta::minutes(45);
        assert_eq!(delta.total_hours(), 5.75);
    }
}
