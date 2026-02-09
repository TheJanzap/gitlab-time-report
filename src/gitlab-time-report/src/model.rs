//! Contains the data model for the time log data.

use chrono::{DateTime, Duration, Local, NaiveDate};
use serde::Deserialize;
use serde_with::{DisplayFromStr, DurationSeconds, NoneAsEmptyString, serde_as};

/// The queried GitLab repository.
#[derive(Debug, PartialEq, Default)]
pub struct Project {
    /// The name of the repository.
    pub name: String,
    /// The time logs of the repository.
    pub time_logs: Vec<TimeLog>,
    /// Total Time spent on the project
    pub total_spent_time: Duration,
}

impl Project {
    /// Merges two projects into one. The name of the resulting project is a comma-seperated string
    /// of the input projects.
    pub fn merge(&mut self, other: Project) {
        self.name = format!("{}, {}", self.name, other.name);
        self.total_spent_time += other.total_spent_time;
        self.time_logs.extend(other.time_logs);
    }
}

/// A single entry of time spent on an issue or merge request.
#[serde_as]
#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "Nodes")]
pub struct TimeLog {
    /// The date the time was spent. Is always set to a valid date in the API.
    pub spent_at: DateTime<Local>,
    /// The entered time that was spent.
    #[serde_as(as = "DurationSeconds<i64>")]
    pub time_spent: Duration,
    /// The optional summary of what was done during the time.
    /// Empty summaries are returned as empty strings by the GitLab API and turned into `None`.
    #[serde_as(as = "NoneAsEmptyString")]
    pub summary: Option<String>,
    /// The user who spent the time.
    pub user: User,
    /// The Issue or Merge Request the time that was spent on.
    #[serde(flatten)]
    pub trackable_item: TrackableItem,
}

/// A list of GitLab users.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Default, Deserialize)]
pub struct UserNodes {
    #[serde(rename = "nodes")]
    pub users: Vec<User>,
}

/// The details of a GitLab user.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Deserialize)]
pub struct User {
    pub name: String,
    pub username: String,
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// A trackable item is either an issue or a merge request.
#[derive(Debug, PartialEq, Default, Eq, Hash, PartialOrd, Ord, Clone)]
pub struct TrackableItem {
    /// Fields shared by all trackable items.
    // Implementation note: The fields are in a separate struct to use serde attributes, as they
    // are only supported when Deserialize is derived.
    pub common: TrackableItemFields,
    /// The type of the trackable item. Contains the fields that are only available for the specific type.
    pub kind: TrackableItemKind,
}

/// The type of the trackable item. Each variant contains a struct with the fields that are
/// only available for the specific type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum TrackableItemKind {
    Issue(Issue),
    MergeRequest(MergeRequest),
}

impl Default for TrackableItemKind {
    fn default() -> Self {
        Self::Issue(Issue::default())
    }
}

impl std::fmt::Display for TrackableItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackableItemKind::Issue(_) => write!(f, "Issue"),
            TrackableItemKind::MergeRequest(_) => write!(f, "Merge Request"),
        }
    }
}

/// Contains the fields that are common to all trackable items.
#[serde_as]
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackableItemFields {
    /// The issue or merge request ID.
    #[serde(rename = "iid")]
    #[serde_as(as = "DisplayFromStr")]
    pub id: u32,
    /// The title of the issue or merge request.
    pub title: String,
    /// The estimated time set for this item.
    #[serde_as(as = "DurationSeconds<i64>")]
    pub time_estimate: Duration,
    /// The total time spent on this item.
    #[serde_as(as = "DurationSeconds<i64>")]
    pub total_time_spent: Duration,
    /// The users assigned to this item. On the GitLab Free version, only one user can be assigned.
    pub assignees: UserNodes,
    /// The milestone assigned to this item.
    pub milestone: Option<Milestone>,
    /// The labels assigned to this item
    pub labels: Labels,
}

/// Contains fields that are only available for issues (there are none).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Default, Deserialize)]
pub struct Issue {}

/// Contains fields that are only available for merge requests.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Default, Deserialize)]
pub struct MergeRequest {
    /// The users reviewing this MR. On the GitLab Free version, only one user can be assigned.
    pub reviewers: UserNodes,
}

/// A milestone is a due date assigned to an issue or merge request.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    /// The name of the milestone.
    pub title: String,
    /// When the milestone is due.
    pub due_date: Option<NaiveDate>,
}

impl std::fmt::Display for Milestone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

/// A list of labels assigned to an issue or merge request.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Default, Deserialize)]
pub struct Labels {
    /// The labels assigned to this item.
    #[serde(rename = "nodes")]
    pub labels: Vec<Label>,
}

/// A single label assigned to an issue or merge request.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Default, Deserialize)]
pub struct Label {
    /// The title of the label.
    pub title: String,
}

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_projects() {
        let mut project1 = Project {
            name: "Frontend".to_string(),
            total_spent_time: Duration::hours(2),
            time_logs: vec![],
        };
        project1.time_logs.push(TimeLog {
            spent_at: Local::now() - Duration::days(1),
            time_spent: Duration::hours(2),
            ..Default::default()
        });

        let mut project2 = Project {
            name: "Backend".to_string(),
            total_spent_time: Duration::hours(1),
            time_logs: vec![],
        };
        project2.time_logs.push(TimeLog {
            spent_at: Local::now() - Duration::weeks(1),
            time_spent: Duration::hours(1),
            ..Default::default()
        });

        project1.merge(project2);
        assert_eq!(project1.name, "Frontend, Backend");
        assert_eq!(project1.total_spent_time, Duration::hours(3));
        assert_eq!(project1.time_logs.len(), 2);
    }
}
