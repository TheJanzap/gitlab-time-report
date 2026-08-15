//! Fetches time logs and related data from the GitLab API.

mod api_model;
mod deserializer;
mod fetch_options;
mod http_requests;

use crate::fetch_api::api_model::ApiResponse;
use crate::fetch_api::http_requests::NetworkError;
use crate::model::Project;
use chrono::Duration;
pub use fetch_options::FetchOptions;
use reqwest::blocking::Client;
use serde_json::{Error, json};
use thiserror::Error;

/// Runs a query against the GitLab API and returns the response as a string. Parsing the response
/// is up to the caller.
/// If there is an error, the function returns a [`QueryError`].
/// # Parameter
/// `payload`: A valid GraphQL JSON payload
/// `client`: The HTTP client, usually `reqwest::Client`
/// `fetch_options`: The options specified by the user
fn run_query(
    payload: serde_json::Value,
    client: &impl http_requests::HttpFetcher,
    fetch_options: &FetchOptions,
) -> Result<String, QueryError> {
    let url = format!(
        "{}://{}/api/graphql",
        fetch_options.protocol, fetch_options.host
    );
    let response = client.http_post_request(&url, payload, fetch_options);

    // Turn the NetworkError into a QueryError if one occurred
    response.map_err(QueryError::NetworkError)
}

/// Fetches the project time logs from the GitLab API.
/// A valid access token is required for internal and private projects.
/// To call the function, create a `FetchOptions` instance with [`FetchOptions::new()`].
/// # Errors
/// For the possible errors, see [`QueryError`].
/// # Example
/// ```
/// # use gitlab_time_report::{fetch_project_time_logs, FetchOptions};
/// # fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let options = FetchOptions::new("https://gitlab.com/gitlab-org/gitlab", None)?;
/// let project = fetch_project_time_logs(&options);
/// // Check for errors
/// match project {
///     Ok(project) => println!("{:?}", project),
///     Err(err) => println!("{:?}", err),
/// };
/// # Ok(()) }
/// ```
#[cfg(not(tarpaulin_include))]
pub fn fetch_project_time_logs(options: &FetchOptions) -> Result<Project, QueryError> {
    let http_client = Client::new();
    fetch_project_time_logs_impl(options, &http_client)
}

/// Implementation of [`fetch_project_time_logs()`] that takes `FetchOptions` and an HTTP client as parameter.
fn fetch_project_time_logs_impl(
    options: &FetchOptions,
    http_client: &impl http_requests::HttpFetcher,
) -> Result<Project, QueryError> {
    let query_template = include_str!("query_project_time_logs.graphql");

    let mut time_logs = Vec::new();
    let mut name = String::new();
    let mut total_spent_time = Duration::default();
    let mut cursor: Option<String> = None;
    let mut has_next_page = true;

    // Fetch all pages from the GitLab API
    while has_next_page {
        let payload = build_query_payload(query_template, &options.path, cursor.as_deref());
        let response = run_query(payload, http_client, options)?;

        // Create a new deserializer that reads the response
        let deserializer = &mut serde_json::Deserializer::from_str(&response);
        // Run the deserializer. If everything is okay, the result is saved into the model variable
        // Use serde_path_to_error to get the field where the deserialization failed.
        let model: ApiResponse = serde_path_to_error::deserialize(deserializer)?;

        let project = validate_model(model, options)?;

        // Update the pagination information
        has_next_page = project.timelogs.page_info.has_next_page;
        cursor = project.timelogs.page_info.end_cursor;

        // Store the project name and total_spent_time from the last page in the response.
        if !has_next_page {
            name = project.name;
            total_spent_time = project.timelogs.total_spent_time;
        }

        // Accumulate timelogs
        time_logs.extend(project.timelogs.nodes);
    }

    // Remove the `Option` from all time logs. There should be no `None` anyway.
    let time_logs = time_logs.into_iter().flatten().collect();

    Ok(Project {
        name,
        time_logs,
        total_spent_time,
    })
}

/// Builds a GraphQL query with the given project path and optional cursor for pagination.
fn build_query_payload(
    template: &str,
    project_path: &str,
    cursor: Option<&str>,
) -> serde_json::Value {
    let variables = match cursor {
        Some(c) => json!({
            "projectPath": project_path,
            "after": c,
        }),
        None => json!({
            "projectPath": project_path,
            "after": null,
        }),
    };

    json!({
        "query": template,
        "variables": variables,
    })
}

/// Validates the response from the GitLab API by checking for GraphQL errors, and if the project can be accessed.
fn validate_model(
    model: ApiResponse,
    options: &FetchOptions,
) -> Result<api_model::Project, QueryError> {
    // Check for GraphQL errors. If there are any, return the first one.
    if let Some(errors) = &model.errors {
        return Err(QueryError::GraphQlError(errors[0].message.clone()));
    }

    let project = model.data.project.ok_or_else(|| {
        // The API returned `"project":null`, the project doesn't exist or has been accessed without a valid access token.
        QueryError::ProjectNotFound(format!("{}/{}", options.host, options.path))
    })?;
    Ok(project)
}

/// Errors that can occur during an API query.
#[derive(Debug, Error)]
pub enum QueryError {
    /// A network error has occurred during the API call.
    #[error("A network error has occurred: {0}")]
    NetworkError(NetworkError),
    /// API returns `"project":null`, project does not exist or has been accessed without a valid access token.
    #[error("Project '{0}' not found")]
    ProjectNotFound(String),
    /// The GraphQL query returned an error, i.e. incorrect syntax, invalid query.
    #[error("Error with the GraphQL query: {0}")]
    GraphQlError(String),
    /// The response could not be deserialized into the model structs.
    #[error("Could not deserialize response: {0}")]
    JsonParseError(#[from] serde_path_to_error::Error<Error>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch_api::http_requests::MockHttpFetcher;
    use std::assert_matches;

    const URL: &str = "https://gitlab.com/test-user/test-project";
    const PROJECT_NAME: &str = "Test Repo";

    #[test]
    fn fetch_project_correctly() {
        let options = FetchOptions::new(URL, None).unwrap();
        let mut mock = MockHttpFetcher::new();
        mock.expect_http_post_request().return_const({
            Ok(r#"{"data": { "project": { "name": "Test Repo", "timelogs": {"pageInfo": {"hasNextPage": false, "endCursor": null}, "totalSpentTime": "20", "nodes": []}}}}"#.into())
        });

        let result = fetch_project_time_logs_impl(&options, &mock);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, PROJECT_NAME);
    }

    #[test]
    fn fetch_project_with_pagination() {
        const JSON_TEMPLATE: &str = r#"{"data":{"project":{"name":"Test Repo","timelogs":{"pageInfo":{"hasNextPage":$NEXT,"endCursor":"$CURSOR"}, "totalSpentTime": "20", "nodes":[]}}}}"#;

        let options = FetchOptions::new(URL, None).unwrap();
        let mut mock = MockHttpFetcher::new();

        // Mock call when returning the first page
        mock.expect_http_post_request()
            .times(1) // Should be called once
            .withf(|_, payload, _| {
                let after_value = payload.get("variables").unwrap().get("after").unwrap();
                after_value.is_null()
            })
            .return_const(Ok(JSON_TEMPLATE
                .replace("$NEXT", "true")
                .replace("$CURSOR", "firstCursor")));

        // Second page
        mock.expect_http_post_request()
            .times(1)
            .withf(|_, payload, _| {
                let after_value = payload.get("variables").unwrap().get("after").unwrap();
                after_value.as_str() == Some("firstCursor")
            })
            .return_const(Ok(JSON_TEMPLATE
                .replace("$NEXT", "true")
                .replace("$CURSOR", "secondCursor")));

        // Third and final page
        mock.expect_http_post_request()
            .times(1)
            .withf(|_, payload, _| {
                let after_value = payload.get("variables").unwrap().get("after").unwrap();
                after_value.as_str() == Some("secondCursor")
            })
            .return_const(Ok(JSON_TEMPLATE
                .replace("$NEXT", "false")
                .replace("$CURSOR", "thirdCursor")));

        let result = fetch_project_time_logs_impl(&options, &mock);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, PROJECT_NAME);
    }

    #[test]
    fn fetch_project_not_found() {
        let input = "https://gitlab.com/invalid/project";

        let options = FetchOptions::new(input, None).unwrap();
        let mut mock = MockHttpFetcher::new();
        mock.expect_http_post_request()
            .return_const(Ok(r#"{"data": {"project": null}}"#.into()));

        let result = fetch_project_time_logs_impl(&options, &mock);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), QueryError::ProjectNotFound(_));
    }

    #[test]
    fn fetch_with_fine_grained_access_token() {
        const TOKEN: &str = "glpat-fine-grained-access-token";
        let options = FetchOptions::new(URL, Some(TOKEN.to_string())).unwrap();
        let mut mock = MockHttpFetcher::new();
        mock.expect_http_post_request().return_const({
            Ok(r#"{"errors":[{"message": "Access denied: This operation doesn't support fine-grained personal access tokens.","locations":[{"line": 11, "column": 9}],"path": ["project", "timelogs", "nodes", 0, "spentAt"]}],"data":{"project":null}}"#.into())
        });

        let result = fetch_project_time_logs_impl(&options, &mock);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), QueryError::GraphQlError(_));
    }
}
