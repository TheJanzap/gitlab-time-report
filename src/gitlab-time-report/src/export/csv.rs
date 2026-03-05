//! Contains the methods to create a CSV file from a list of time logs.

use crate::model::{TimeLog, TrackableItemKind};
use chrono::{DateTime, Local};
use csv::{Writer, WriterBuilder};
use serde::Serialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(test)]
use mockall::automock;

/// The columns of the CSV to be exported.
#[derive(Serialize)]
struct TimeLogCsvRow {
    spent_at: DateTime<Local>,
    time_spent_seconds: i64,
    summary: Option<String>,
    user_name: String,
    trackable_item_title: String,
    trackable_item_type: String,
    trackable_item_id: u32,
    trackable_item_estimate_seconds: i64,
    trackable_item_total_time_spent: i64,
    trackable_item_milestone: Option<String>,
    trackable_item_labels: String,
    trackable_item_assignees: String,
    merge_request_reviewers: String,
}

/// Contains the method to abstract the writing of CSV files
#[cfg_attr(test, automock)]
pub trait CsvWriter {
    fn write_csv(&self, data: &str, path: &Path) -> Result<(), CsvError>;
}

/// Default implementation that writes to actual files
pub struct FileCsvWriter;

impl CsvWriter for FileCsvWriter {
    fn write_csv(&self, data: &str, path: &Path) -> Result<(), CsvError> {
        Ok(std::fs::write(path, data)?)
    }
}

/// Creates a CSV file at `path` which contains the given time logs.
/// # Errors
/// Possible errors can be seen in [`CsvError`].
pub fn create_csv(time_logs: &Vec<TimeLog>, path: PathBuf) -> Result<(), CsvError> {
    create_csv_with_writer(time_logs, path, &FileCsvWriter)
}

/// Implementation of [`create_csv()`] that takes a [`CsvWriter`] to write the file.
fn create_csv_with_writer(
    time_logs: &Vec<TimeLog>,
    mut path: PathBuf,
    writer: &impl CsvWriter,
) -> Result<(), CsvError> {
    if path.extension().is_none() {
        path.set_extension("csv");
    }

    // Write to an in-memory buffer first
    let mut buffer = WriterBuilder::new().from_writer(vec![]);

    for log in time_logs {
        let trackable_item = &log.trackable_item.common;

        let milestone = trackable_item.milestone.as_ref().map(|m| m.title.clone());

        let labels = trackable_item
            .labels
            .labels
            .iter()
            .map(|l| l.title.clone())
            .collect::<Vec<_>>()
            .join(",");

        let assignees = trackable_item
            .assignees
            .users
            .iter()
            .map(|u| u.name.clone())
            .collect::<Vec<_>>()
            .join(",");

        let mr_reviewers = match &log.trackable_item.kind {
            TrackableItemKind::MergeRequest(mr) => mr
                .reviewers
                .users
                .iter()
                .map(|u| u.name.clone())
                .collect::<Vec<_>>()
                .join(","),
            TrackableItemKind::Issue(_) => String::new(),
        };

        let csv_row = TimeLogCsvRow {
            spent_at: log.spent_at,
            time_spent_seconds: log.time_spent.num_seconds(),
            summary: log.summary.clone(),
            user_name: log.user.name.clone(),
            trackable_item_title: trackable_item.title.clone(),
            trackable_item_type: log.trackable_item.kind.to_string(),
            trackable_item_id: trackable_item.id,
            trackable_item_estimate_seconds: trackable_item.time_estimate.num_seconds(),
            trackable_item_total_time_spent: trackable_item.total_time_spent.num_seconds(),
            trackable_item_milestone: milestone,
            trackable_item_labels: labels,
            trackable_item_assignees: assignees,
            merge_request_reviewers: mr_reviewers,
        };
        buffer.serialize(csv_row)?;
    }

    let bytes = buffer
        .into_inner()
        .map_err(|e| CsvError::CsvFinalize(Box::new(e)))?;

    let csv_string = String::from_utf8(bytes)?;

    // Use the writer to write the data to the file
    writer.write_csv(&csv_string, &path)?;

    Ok(())
}

/// Errors that can occur during CSV file creation.
#[derive(Debug, Error)]
pub enum CsvError {
    /// An error has occurred when writing files to disk.
    #[error("I/O error while writing CSV file: {0}")]
    Io(#[from] std::io::Error),

    /// An Error happened during serialization/flush from [`csv`].
    #[error("CSV serialization error: {0}")]
    Csv(#[from] csv::Error),

    /// Error when finalizing the writer.
    #[error("Failed to finalize CSV writer: {0}")]
    CsvFinalize(#[from] Box<csv::IntoInnerError<Writer<Vec<u8>>>>),

    /// The in-memory CSV data could not be converted to UTF-8 text.
    #[error("CSV data is not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Issue, MergeRequest, TimeLog, TrackableItem, TrackableItemFields, User, UserNodes,
    };
    use chrono::{DateTime, Duration, Local};

    fn get_timelogs() -> Vec<TimeLog> {
        vec![
            TimeLog {
                spent_at: "2025-10-14T10:00:00+01:00"
                    .parse::<DateTime<Local>>()
                    .unwrap(),
                time_spent: Duration::seconds(3600),
                summary: Some("Timelog 1 Summary".to_string()),
                user: User {
                    name: "User 1".to_string(),
                    username: String::default(),
                },
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        id: 1,
                        title: "Issue Title".to_string(),
                        time_estimate: Duration::seconds(4200),
                        total_time_spent: Duration::seconds(3600),
                        ..Default::default()
                    },
                    kind: TrackableItemKind::Issue(Issue::default()),
                },
            },
            TimeLog {
                spent_at: "2025-10-13T14:30:00+01:00"
                    .parse::<DateTime<Local>>()
                    .unwrap(),
                time_spent: Duration::seconds(3600),
                summary: Some("Timelog 2 Summary".to_string()),
                user: User {
                    name: "User 2".to_string(),
                    username: String::default(),
                },
                trackable_item: TrackableItem {
                    common: TrackableItemFields {
                        id: 2,
                        title: "MR Title".to_string(),
                        time_estimate: Duration::seconds(2700),
                        total_time_spent: Duration::seconds(3600),
                        ..Default::default()
                    },
                    kind: TrackableItemKind::MergeRequest(MergeRequest {
                        reviewers: UserNodes { users: vec![] },
                    }),
                },
            },
        ]
    }

    #[test]
    fn test_create_csv_mocked() {
        let path = PathBuf::from("test");
        let mut mock_writer = MockCsvWriter::new();

        mock_writer
            .expect_write_csv()
            .times(1)
            .returning(|data, path| {
                assert_eq!(path, Path::new("test.csv"));
                assert!(data.contains("spent_at,time_spent_seconds"));
                assert!(data.contains("Timelog 1 Summary"));
                assert!(data.contains("User 1"));
                Ok(())
            });

        let time_logs = get_timelogs();

        let result = create_csv_with_writer(&time_logs, path, &mock_writer);
        assert!(result.is_ok());
    }
}
