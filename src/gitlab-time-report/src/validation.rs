//! This module contains methods to validate [`TimeLog`]s and report possible problems.
//! They are implemented via the validator chain pattern.

use crate::model::{TimeLog, TrackableItem};
use chrono::{Local, NaiveDate};
use std::collections::HashSet;

/// Possible problems in [`TimeLog`] that can be found during validation.
#[derive(Debug)]
pub enum ValidationProblem {
    /// The time spent exceeds the maximum allowed.
    ExcessiveHours { max_hours: u16 },
    /// No summary was entered.
    MissingSummary,
    /// Entered date is in the future.
    FutureDate,
    /// Duplicate entry has been found (same user, date, trackable item, time spent and summary)
    DuplicateEntry,
    /// `TimeLog` is before the configured project start date.
    BeforeStartDate { start_date: NaiveDate },
}

/// Stores the result of a validation run.
pub struct ValidationResult<'a> {
    /// The time log that was validated.
    pub time_log: &'a TimeLog,
    /// The problems found in this time log.
    pub problems: Vec<ValidationProblem>,
}

impl ValidationResult<'_> {
    /// Returns true if there are no problems found in this time log.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.problems.is_empty()
    }

    /// Returns true if there is at least one problem of the given type.
    #[must_use]
    pub fn has_problems(&self, problem_type: &ValidationProblem) -> bool {
        self.problems
            .iter()
            .any(|i| std::mem::discriminant(i) == std::mem::discriminant(problem_type))
    }
}

/// Trait functions that a validator must implement.
pub trait Validator {
    /// Validate each [`TimeLog`] on its own. Returns a Vec containing the problems found
    /// or an empty Vec if there are none.
    fn validate_single(&mut self, time_log: &TimeLog) -> Vec<ValidationProblem>;
}

/// The main validator that runs all validators.  Add the other validators to this one with
/// [`TimeLogValidator::with_validator`], then call [`TimeLogValidator::validate`] to run the validation.
/// # Example
/// ```
/// # use gitlab_time_report::validation::{TimeLogValidator, ValidationProblem, ExcessiveHoursValidator, HasSummaryValidator};
/// # use gitlab_time_report::model::TimeLog;
/// # use chrono::Local;
/// let time_logs = [
///     TimeLog{ summary: None, ..Default::default() },
///     TimeLog{ summary: Some("Code Review".to_string()), ..Default::default() }
/// ];
///
/// let mut validator = TimeLogValidator::new()
///     .with_validator(ExcessiveHoursValidator::new(10))
///     .with_validator(HasSummaryValidator);
/// let results = validator.validate(&time_logs);
///
/// // Assertions to check the results, you don't need to do this in your code
/// assert!(results[0].has_problems(&ValidationProblem::MissingSummary));
/// assert!(!results[0].has_problems(&ValidationProblem::ExcessiveHours{ max_hours: 10 }));
/// assert!(results[1].is_valid());
///
/// for result in results {
///     if result.is_valid() { continue; }
///     for problem in &result.problems {
///         match problem {
///             ValidationProblem::ExcessiveHours { max_hours } => println!("Time spent exceeds maximum of {max_hours} hours"),
///             ValidationProblem::MissingSummary => println!("No summary was entered"),
///             _ => {}
///         }
///     }
/// }
/// ```
pub struct TimeLogValidator {
    validators: Vec<Box<dyn Validator>>,
}

/// Added for <https://rust-lang.github.io/rust-clippy/master/index.html#new_without_default>
impl Default for TimeLogValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeLogValidator {
    /// Create a new [`TimeLogValidator`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    #[must_use]
    /// Add a new validator to the chain. See [`Validator`] for more information.
    /// # Example
    /// ```
    /// # use gitlab_time_report::validation::{TimeLogValidator, ValidationProblem, ExcessiveHoursValidator, HasSummaryValidator};
    /// # use gitlab_time_report::model::TimeLog;
    /// # use chrono::Local;
    /// let mut validator = TimeLogValidator::new()
    ///     .with_validator(ExcessiveHoursValidator::new(10))
    ///     .with_validator(HasSummaryValidator);
    /// ```
    pub fn with_validator(mut self, validator: impl Validator + 'static) -> Self {
        self.validators.push(Box::new(validator));
        self
    }

    /// Run all validators on the given time logs.
    pub fn validate<'a>(&mut self, time_logs: &'a [TimeLog]) -> Vec<ValidationResult<'a>> {
        let validation_results: Vec<ValidationResult<'a>> = time_logs
            .iter()
            .map(|time_log| {
                let mut problems = Vec::new();

                // Run all validators on this time log
                for validator in &mut self.validators {
                    // Append the problems Vec with the problems found by this validator
                    problems.extend(validator.validate_single(time_log));
                }

                ValidationResult { time_log, problems }
            })
            .collect();

        validation_results
    }
}

/// Validates that a single time log does not exceed the given maximum hours.
pub struct ExcessiveHoursValidator {
    max_hours: u16,
}

impl ExcessiveHoursValidator {
    /// Set the maximum hours allowed.
    #[must_use]
    pub fn new(max_hours: u16) -> Self {
        Self { max_hours }
    }
}

impl Validator for ExcessiveHoursValidator {
    fn validate_single(&mut self, time_log: &TimeLog) -> Vec<ValidationProblem> {
        let hours = time_log.time_spent.num_hours();

        match hours > i64::from(self.max_hours) {
            true => vec![ValidationProblem::ExcessiveHours {
                max_hours: self.max_hours,
            }],
            false => Vec::new(),
        }
    }
}

/// Validates that a time log has a summary.
pub struct HasSummaryValidator;

impl Validator for HasSummaryValidator {
    fn validate_single(&mut self, time_log: &TimeLog) -> Vec<ValidationProblem> {
        match time_log.summary.is_none() {
            true => vec![ValidationProblem::MissingSummary],
            false => Vec::new(),
        }
    }
}

/// Validates that a time log has no date in the future.
pub struct NoFutureDateValidator;

impl Validator for NoFutureDateValidator {
    fn validate_single(&mut self, time_log: &TimeLog) -> Vec<ValidationProblem> {
        if time_log.spent_at > Local::now() {
            return vec![ValidationProblem::FutureDate];
        }
        Vec::new()
    }
}

/// Validates that a time log does not contain duplicates (same user, date, trackable item, time spent and summary)
pub struct DuplicatesValidator {
    seen: HashSet<(String, NaiveDate, i64, Option<String>, TrackableItem)>,
}

/// Added for <https://rust-lang.github.io/rust-clippy/master/index.html#new_without_default>
impl Default for DuplicatesValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl DuplicatesValidator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            seen: HashSet::new(),
        }
    }
}

impl Validator for DuplicatesValidator {
    fn validate_single(&mut self, time_log: &TimeLog) -> Vec<ValidationProblem> {
        let key = (
            time_log.user.name.clone(),
            time_log.spent_at.date_naive(),
            time_log.time_spent.num_seconds(),
            time_log.summary.clone(),
            time_log.trackable_item.clone(),
        );

        if self.seen.insert(key) {
            Vec::new()
        } else {
            vec![ValidationProblem::DuplicateEntry]
        }
    }
}

/// Validates that a time log date is not before the project start date.
pub struct BeforeStartDateValidator {
    start_date: NaiveDate,
}

impl BeforeStartDateValidator {
    #[must_use]
    pub fn new(start_date: NaiveDate) -> Self {
        Self { start_date }
    }
}

impl Validator for BeforeStartDateValidator {
    fn validate_single(&mut self, time_log: &TimeLog) -> Vec<ValidationProblem> {
        let log_date = time_log.spent_at.date_naive();
        if log_date < self.start_date {
            return vec![ValidationProblem::BeforeStartDate {
                start_date: self.start_date,
            }];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MergeRequest, TimeLog, TrackableItemFields, TrackableItemKind};
    use chrono::{Duration, Local, TimeDelta};

    const NUMBER_OF_LOGS: usize = 7;
    fn get_time_logs() -> [TimeLog; NUMBER_OF_LOGS] {
        [
            TimeLog {
                summary: Some("Valid Time log".to_string()),
                spent_at: Local::now() - TimeDelta::days(1),
                time_spent: Duration::hours(4),
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        title: "test".to_string(),
                        ..Default::default()
                    },
                    kind: TrackableItemKind::MergeRequest(MergeRequest::default()),
                },
                ..Default::default()
            },
            TimeLog {
                summary: Some("Excessive Hours".to_string()),
                spent_at: Local::now() - TimeDelta::days(1),
                time_spent: Duration::hours(12),
                ..Default::default()
            },
            TimeLog {
                summary: None,
                spent_at: Local::now() - TimeDelta::days(1),
                time_spent: Duration::hours(5) + Duration::minutes(30),
                ..Default::default()
            },
            TimeLog {
                summary: Some("Future Date".to_string()),
                spent_at: Local::now() + TimeDelta::hours(1),
                time_spent: Duration::hours(3),
                ..Default::default()
            },
            TimeLog {
                // Duplicate entry
                summary: Some("Valid Time log".to_string()),
                spent_at: Local::now() - TimeDelta::days(1),
                time_spent: Duration::hours(4),
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        title: "test".to_string(),
                        ..Default::default()
                    },
                    kind: TrackableItemKind::MergeRequest(MergeRequest::default()),
                },
                ..Default::default()
            },
            TimeLog {
                // No Summary, Future Date and Excessive Hours
                summary: None,
                spent_at: Local::now() + TimeDelta::days(1),
                time_spent: Duration::hours(15),
                ..Default::default()
            },
            TimeLog {
                // Should not trigger a duplicate entry problem
                summary: Some("Same time spent & spent_at as 'Summary missing' timelog, but different summary".to_string()),
                spent_at: Local::now() - TimeDelta::days(1),
                time_spent: Duration::hours(5) + Duration::minutes(30),
                ..Default::default()
            }
        ]
    }

    #[test]
    fn test_excessive_hours_validator() {
        const EXCESSIVE_HOURS_LIMIT: u16 = 10;

        let time_logs = get_time_logs();
        let expected_problem = ValidationProblem::ExcessiveHours {
            max_hours: EXCESSIVE_HOURS_LIMIT,
        };

        let mut validator = TimeLogValidator::new()
            .with_validator(ExcessiveHoursValidator::new(EXCESSIVE_HOURS_LIMIT));

        let results = validator.validate(&time_logs);
        assert_eq!(results.len(), NUMBER_OF_LOGS);
        for (i, result) in results.iter().enumerate() {
            match i {
                1 | 5 => assert!(result.has_problems(&expected_problem)),
                _ => assert!(result.is_valid()),
            }
        }
    }

    #[test]
    fn test_has_summary_validator() {
        let time_logs = get_time_logs();
        let expected_problem = ValidationProblem::MissingSummary;

        let mut validator = TimeLogValidator::new().with_validator(HasSummaryValidator);

        let results = validator.validate(&time_logs);
        assert_eq!(results.len(), NUMBER_OF_LOGS);
        for (i, result) in results.iter().enumerate() {
            match i {
                2 | 5 => assert!(result.has_problems(&expected_problem)),
                _ => assert!(result.is_valid()),
            }
        }
    }

    #[test]
    fn test_future_date_validator() {
        let time_logs = get_time_logs();
        let expected_problem = ValidationProblem::FutureDate;

        let mut validator = TimeLogValidator::new().with_validator(NoFutureDateValidator);

        let results = validator.validate(&time_logs);
        assert_eq!(results.len(), NUMBER_OF_LOGS);
        for (i, result) in results.iter().enumerate() {
            match i {
                3 | 5 => assert!(result.has_problems(&expected_problem)),
                _ => assert!(result.is_valid()),
            }
        }
    }

    #[test]
    fn test_duplicates_validator() {
        let time_logs = get_time_logs();
        let expected_problem = ValidationProblem::DuplicateEntry;

        let mut validator = TimeLogValidator::new().with_validator(DuplicatesValidator::new());

        let results = validator.validate(&time_logs);
        assert_eq!(results.len(), NUMBER_OF_LOGS);
        for (i, result) in results.iter().enumerate() {
            match i {
                4 => assert!(result.has_problems(&expected_problem)),
                _ => assert!(result.is_valid()),
            }
        }
    }

    #[test]
    fn test_before_start_date_validator() {
        let time_logs = get_time_logs();
        let start_date = Local::now().date_naive();

        let expected_problem = ValidationProblem::BeforeStartDate { start_date };

        let mut validator =
            TimeLogValidator::new().with_validator(BeforeStartDateValidator::new(start_date));

        let results = validator.validate(&time_logs);
        assert_eq!(results.len(), NUMBER_OF_LOGS);
        for (i, result) in results.iter().enumerate() {
            match i {
                0 | 1 | 2 | 4 | 6 => assert!(result.has_problems(&expected_problem)),
                _ => assert!(result.is_valid()),
            }
        }
    }

    #[test]
    fn test_all_validators() {
        const EXCESSIVE_HOURS_LIMIT: u16 = 10;

        let time_logs = get_time_logs();
        let mut validator = TimeLogValidator::new()
            .with_validator(ExcessiveHoursValidator::new(EXCESSIVE_HOURS_LIMIT))
            .with_validator(HasSummaryValidator)
            .with_validator(NoFutureDateValidator)
            .with_validator(DuplicatesValidator::new());

        let excessive_hours_validator = ValidationProblem::ExcessiveHours {
            max_hours: EXCESSIVE_HOURS_LIMIT,
        };

        let results = validator.validate(&time_logs);

        assert_eq!(results.len(), NUMBER_OF_LOGS);
        assert!(results[0].is_valid());

        assert_eq!(results[1].problems.len(), 1);
        assert!(results[1].has_problems(&excessive_hours_validator));

        assert_eq!(results[2].problems.len(), 1);
        assert!(results[2].has_problems(&ValidationProblem::MissingSummary));

        assert_eq!(results[3].problems.len(), 1);
        assert!(results[3].has_problems(&ValidationProblem::FutureDate));

        assert_eq!(results[4].problems.len(), 1);
        assert!(results[4].has_problems(&ValidationProblem::DuplicateEntry));

        assert_eq!(results[5].problems.len(), 3);
        assert!(results[5].has_problems(&excessive_hours_validator));
        assert!(results[5].has_problems(&ValidationProblem::MissingSummary));
        assert!(results[5].has_problems(&ValidationProblem::FutureDate));

        assert!(results[6].is_valid());
    }
}
