//! Contains custom deserializer logic for the items from the GitLab API.

use crate::model::{
    Issue, MergeRequest, TrackableItem, TrackableItemFields, TrackableItemKind, UserNodes,
};
use serde::{Deserialize, Deserializer};

/// To not always check if a trackable item is an issue or a merge request when accessing common
/// fields, they are contained inside [`TrackableItemFields`].
/// The type of the trackable item is encoded in [`TrackableItemKind`].
/// To create the structure, a custom deserializer is needed. It consists of multiple steps
/// 1. Create temporary structs that model the structure of the JSON (the ones ending on `Deserialize`)
/// 2. Deserialize them with the default deserializer (derive macro)
/// 3. Create the real [`TrackableItem`] that sets `TrackableItem.kind` accordingly.
impl<'de> Deserialize<'de> for TrackableItem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Create temporary structs to deserialize the JSON as it exists in the API
        // Both fields can be present in the JSON, but only one will be non-null
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TrackableItemDeserialize {
            issue: Option<IssueDeserialize>,
            merge_request: Option<MergeRequestDeserialize>,
        }

        #[derive(Deserialize)]
        struct IssueDeserialize {
            #[serde(flatten)]
            common: TrackableItemFields,
        }

        #[derive(Deserialize)]
        struct MergeRequestDeserialize {
            #[serde(flatten)]
            common: TrackableItemFields,
            reviewers: UserNodes,
        }

        // Deserialize the trackable item type with the derived deserializer
        let trackable_item_from_api = TrackableItemDeserialize::deserialize(deserializer)?;

        // Create the real trackable item from the temporary structs based on which field is non-null
        let real_trackable_item = match (
            trackable_item_from_api.issue,
            trackable_item_from_api.merge_request,
        ) {
            (Some(issue), None) => TrackableItem {
                common: issue.common,
                kind: TrackableItemKind::Issue(Issue {}),
            },
            (None, Some(mr)) => TrackableItem {
                common: mr.common,
                kind: TrackableItemKind::MergeRequest(MergeRequest {
                    reviewers: mr.reviewers,
                }),
            },
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "Both issue and mergeRequest are present, expected only one",
                ));
            }
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "Neither issue nor mergeRequest is present, expected one",
                ));
            }
        };

        Ok(real_trackable_item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::User;
    use std::assert_matches;

    #[test]
    fn deserialize_issue() {
        let json_str = r#"{"issue":{"iid":"1","title":"Issue Name","timeEstimate":3600,"totalTimeSpent":10800,"assignees":{"nodes":[]},"milestone":null,"labels":{"nodes":[{"title":"Project Management"}]}},"mergeRequest":null}"#;
        let deserialized: TrackableItem = serde_json::from_str(json_str).unwrap();
        assert_eq!(deserialized.common.title, "Issue Name");
        assert_eq!(
            deserialized.common.time_estimate,
            chrono::Duration::seconds(3600)
        );
        assert_eq!(
            deserialized.common.total_time_spent,
            chrono::Duration::seconds(10800)
        );
        assert_matches!(deserialized.kind, TrackableItemKind::Issue(_));
    }

    #[test]
    fn deserialize_mr() {
        let json_str = r#"{"issue":null,"mergeRequest":{"iid":"1","title":"Update README.md","timeEstimate":1800,"totalTimeSpent":1800,"assignees":{"nodes":[]},"reviewers":{"nodes":[{"name":"User1","username":"user.1"}]},"milestone":null,"labels":{"nodes":[]}}}"#;
        let reviewers = UserNodes {
            users: vec![User {
                name: "User1".to_string(),
                username: "user.1".to_string(),
            }],
        };

        let deserialized: TrackableItem = serde_json::from_str(json_str).unwrap();
        assert_eq!(deserialized.common.title, "Update README.md");
        assert_eq!(
            deserialized.common.time_estimate,
            chrono::Duration::seconds(1800)
        );
        assert_eq!(
            deserialized.common.total_time_spent,
            chrono::Duration::seconds(1800)
        );
        assert_matches!(deserialized.kind, TrackableItemKind::MergeRequest(_));

        if let TrackableItemKind::MergeRequest(deserialized_mr) = deserialized.kind {
            assert_eq!(deserialized_mr.reviewers, reviewers);
        }
    }

    #[test]
    fn deserialize_mr_and_issue_present() {
        let json_str = r#"{"issue":{"iid":"1","title":"Issue Name","timeEstimate":3600,"totalTimeSpent":10800,"assignees":{"nodes":[]},"milestone":null,"labels":{"nodes":[]}},"mergeRequest":{"iid":"1","title":"Update README.md","timeEstimate":1800,"totalTimeSpent":1800,"assignees":{"nodes":[]},"reviewers":{"nodes":[]},"milestone":null,"labels":{"nodes":[]}}}"#;
        let deserialized = serde_json::from_str::<TrackableItem>(json_str);

        assert!(deserialized.is_err());
        let error = deserialized.unwrap_err();
        assert!(error.is_data());
        assert_eq!(
            error.to_string(),
            "Both issue and mergeRequest are present, expected only one"
        );
    }

    #[test]
    fn deserialize_mr_and_issue_null() {
        let json_str = r#"{"issue":null,"mergeRequest":null}"#;
        let deserialized = serde_json::from_str::<TrackableItem>(json_str);

        assert!(deserialized.is_err());
        let error = deserialized.unwrap_err();
        assert!(error.is_data());
        assert_eq!(
            error.to_string(),
            "Neither issue nor mergeRequest is present, expected one"
        );
    }
}
