//! Handles fetching of repositories

#![cfg(not(tarpaulin_include))]

use gitlab_time_report::model::Project;
use gitlab_time_report::{FetchOptions, QueryError};

/// Fetches the time logs and related data for a single GitLab repository.
pub(super) fn fetch_project(
    url: &str,
    token: Option<&String>,
) -> Result<Project, Box<dyn std::error::Error>> {
    let fetch_options = FetchOptions::new(url, token.cloned())?;

    let project_result = gitlab_time_report::fetch_project_time_logs(&fetch_options);
    match project_result {
        Ok(project) => Ok(project),
        Err(QueryError::ProjectNotFound(url)) => {
            print_project_not_found(&url);
            Err(QueryError::ProjectNotFound(url).into())
        }
        Err(error) => Err(error.into()),
    }
}

/// Fetch time logs for multiple GitLab repositories and merges them into a single project.
pub(super) fn fetch_projects(
    urls: Vec<String>,
    token: Option<&String>,
) -> Result<Project, Box<dyn std::error::Error>> {
    // Fetch the data for the first URL
    let mut url_iter = urls.into_iter();
    let first_url = url_iter.next().expect("Should be at least one URL");
    println!("Fetching time logs from '{first_url}'...");
    let mut project = fetch_project(&first_url, token)?;

    // Fetch the data for all other URLs and merge them into the first project
    for url in url_iter {
        println!("Fetching time logs from '{url}'...");
        let current_project = fetch_project(&url, token)?;
        project.merge(current_project);
    }
    Ok(project)
}

/// Prints a message regarding access token. URL passed to this function should be without a
/// protocol prefix (`https://`)
fn print_project_not_found(url: &str) {
    let (host, path) = url.split_once('/').expect("Should be a URL with one `/`");
    eprintln!("\
Project \"{url}\" could not be found. If the project visibility is set to 'internal' or 'private', \
you need to provide a GitLab access token with the 'read_api' permission and \
specify it with the --token option.

You can use a personal, group or project access token. Create a new one with one of the following URLs:
Personal: https://{host}/-/user_settings/personal_access_tokens?name=GitLab+Time-Report&scopes=read_api
Project:  https://{host}/{path}/-/settings/access_tokens
Group:    Navigate to your group settings => Access Tokens => Generate new token\n");
}
