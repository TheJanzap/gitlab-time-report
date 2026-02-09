// Integration tests should not affect test coverage
#![cfg(not(tarpaulin))]
#![allow(clippy::unwrap_used)]

use assert_cmd::cargo_bin_cmd;
use mockito::{Mock, Server};
use predicates::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// Creates a mock for a successful response from the GitLab API
fn create_successful_response_mock(server: &mut Server) -> Mock {
    server
        .mock("POST", "/api/graphql")
        .with_header("Content-Type", "application/json")
        .with_body_from_file("tests/fixtures/serverResponseSuccess.json")
        .create()
}

/// Creates a mock for a response that the project was not found (may be private or internal).
fn create_project_not_found_mock(server: &mut Server) -> Mock {
    // Repository is private
    server
        .mock("POST", "/api/graphql")
        .with_header("Content-Type", "application/json")
        .with_body_from_file("tests/fixtures/serverResponseProjectNotFound.json")
        .create()
}

/// Helper function to test that the output path contains the expected number and types of charts.
fn test_charts(output_path: &Path) {
    const NUMBER_OF_CHARTS: usize = 9;
    const NUMBER_OF_FORMATS: usize = 2;
    const TOTAL_CHARTS: usize = NUMBER_OF_CHARTS * NUMBER_OF_FORMATS;

    let file_paths = output_path
        .read_dir()
        .unwrap()
        .map(|dir_entry| dir_entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(file_paths.len(), TOTAL_CHARTS);

    for chart_prefix in 1..NUMBER_OF_CHARTS {
        // Check for a numbered prefix
        let prefix = format!("{chart_prefix:02}");
        let charts_with_prefix = file_paths
            .iter()
            .filter(|path| {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                file_name.starts_with(&prefix)
            })
            .collect::<Vec<_>>();

        assert_ne!(
            charts_with_prefix.len(),
            0,
            "No charts with the prefix {prefix} found"
        );
        assert_eq!(
            charts_with_prefix.len(),
            NUMBER_OF_FORMATS,
            "Chart should have {NUMBER_OF_FORMATS} files, found {}",
            charts_with_prefix.len()
        );

        for path in charts_with_prefix {
            let file_ext = path.extension().unwrap().to_str().unwrap();
            let expected_first_line = match file_ext {
                "html" => "<!DOCTYPE html>",
                "svg" => "<svg",
                _ => panic!("Unexpected file extension: {file_ext}"),
            };

            let file_contents = std::fs::read_to_string(path).unwrap();
            let first_line = file_contents.lines().next().unwrap();
            assert!(first_line.contains(expected_first_line));
        }
    }
}

#[test]
fn test_help_flag() {
    cargo_bin_cmd!("gitlab-time-report-cli")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn test_tables_function_prints_to_stout() {
    let mut server = Server::new();
    let mock = create_successful_response_mock(&mut server);
    let url = format!("{}/test-user/test-repo", server.url());

    cargo_bin_cmd!("gitlab-time-report-cli")
        .args([&url])
        .assert()
        .success()
        .stdout(predicate::str::contains("Yesterday"))
        .stdout(predicate::str::contains("Integration Test User 1"))
        .stdout(predicate::str::contains("Bug"));

    mock.assert();
}

#[test]
fn test_project_not_found_message_to_stout() {
    let mut server = Server::new();
    let mock = create_project_not_found_mock(&mut server);
    let url = format!("{}/test-user/private-repo", server.url());

    cargo_bin_cmd!("gitlab-time-report-cli")
        .args([&url])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "private-repo\" could not be found",
        ));

    mock.assert();
}

#[test]
fn test_export_command_succeeds_and_writes_csv_to_disk() {
    let mut server = Server::new();
    let mock = create_successful_response_mock(&mut server);
    let url = format!("{}/test-user/test-repo", server.url());

    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("timelogs.csv");

    cargo_bin_cmd!("gitlab-time-report-cli")
        .args([&url, "export", "--output", output_path.to_str().unwrap()])
        .assert()
        .success();

    mock.assert();

    assert!(output_path.exists());
    let result = std::fs::read_to_string(output_path).unwrap();
    let first_line = result.lines().next().unwrap();

    assert!(first_line.contains("spent_at,time_spent_seconds"));
}

#[test]
fn test_charts_command_writes_charts_to_disk() {
    let mut server = Server::new();
    let mock = create_successful_response_mock(&mut server);
    let url = format!("{}/test-user/test-repo", server.url());

    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("test-charts");

    cargo_bin_cmd!("gitlab-time-report-cli")
        .args([
            &url,
            "charts",
            "--output",
            output_path.to_str().unwrap(),
            "--sprints",
            "4",
            "--hours-per-person",
            "50",
            "--weeks-per-sprint",
            "1",
        ])
        .assert()
        .success();

    mock.assert();
    assert!(output_path.exists());
    test_charts(&output_path);
}

#[test]
fn test_charts_command_with_env_vars() {
    let mut server = Server::new();
    let mock = create_successful_response_mock(&mut server);
    let url = format!("{}/test-user/test-repo", server.url());

    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("charts");

    let envs = HashMap::from([
        ("GITLAB_URL", url.as_str()),
        ("SPRINTS", "7"),
        ("WEEKS_PER_SPRINT", "2"),
        ("HOURS_PER_PERSON", "240"),
    ]);

    cargo_bin_cmd!("gitlab-time-report-cli")
        .envs(envs)
        .args(["charts", "--output", output_path.to_str().unwrap()])
        .assert()
        .success();

    mock.assert();
    println!("{}", output_path.display());
    assert!(output_path.exists());
    test_charts(&output_path);
}

#[test]
fn test_dashboard_command_creates_dashboard_and_charts() {
    const DASHBOARD_FILE_NAME: &str = "test_dashboard.html";

    let mut server = Server::new();
    let mock = create_successful_response_mock(&mut server);
    let url = format!("{}/test-user/test-repo", server.url());

    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("test-charts");

    cargo_bin_cmd!("gitlab-time-report-cli")
        .args([
            &url,
            "dashboard",
            "--output",
            output_path.to_str().unwrap(),
            "--sprints",
            "4",
            "--hours-per-person",
            "64",
            "--weeks-per-sprint",
            "1",
        ])
        .assert()
        .success();

    mock.assert();

    assert!(output_path.exists());
    test_charts(&output_path);

    let dashboard_path = output_path.parent().unwrap().join(DASHBOARD_FILE_NAME);
    assert!(dashboard_path.exists());
    let dashboard_content = std::fs::read_to_string(dashboard_path).unwrap();
    assert!(dashboard_content.contains("<!DOCTYPE html>"));
    assert!(dashboard_content.contains("Integration Test User 1"));
    assert!(dashboard_content.contains("Bug"));
}
