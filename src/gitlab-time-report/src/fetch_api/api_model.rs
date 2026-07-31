//! Structs for deserializing the JSON response from the GitLab API

use crate::model::TimeLog;
use chrono::Duration;
use serde::Deserialize;
use serde_with::{DurationSeconds, serde_as};

/// The queried GitLab repository as it appears in the GitLab API.
#[derive(Debug, Deserialize)]
pub(super) struct Project {
    /// The name of the repository.
    pub(super) name: String,
    /// The time logs of the repository.
    pub(super) timelogs: TimeLogs,
}

/// Time logs as they appear in the GitLab API with pagination information.
#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeLogs {
    /// The actual time logs. On some GraphQL errors, `nodes` exists but is empty, so `TimeLog`
    /// needs to be wrapped in `Option`.
    pub(super) nodes: Vec<Option<TimeLog>>,
    /// Pagination for the GitLab API
    pub(super) page_info: PageInfo,
    /// Total Time spent on the project
    #[serde_as(as = "DurationSeconds<String>")]
    pub(super) total_spent_time: Duration,
}

/// Information to aid in the pagination of the GitLab API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PageInfo {
    /// When paginating forwards, are there more items?
    pub(super) has_next_page: bool,
    /// When paginating forwards, the cursor to continue.
    pub(super) end_cursor: Option<String>,
}

/// The top-level node of a GitLab API response.
#[derive(Debug, Deserialize)]
pub(super) struct ApiResponse {
    /// The response data.
    pub(super) data: Data,
    /// Possible GraphQL errors that occurred in the query.
    pub(super) errors: Option<Vec<GraphQlError>>,
}

/// Response data of the GitLab API.
#[derive(Debug, Deserialize)]
pub(super) struct Data {
    /// The data of the project if it exists and is accessed with the right permissions.
    pub(super) project: Option<Project>,
}

/// The actual GraphQL error.
#[derive(Debug, Deserialize)]
pub(super) struct GraphQlError {
    pub(super) message: String,
}
