//! Contains the methods to create a HTML file with statistics from a list of time logs.

use crate::model::{Label, TimeLog};
use crate::tables::{
    populate_table_timelogs_by_label, populate_table_timelogs_by_milestone,
    populate_table_timelogs_in_timeframes_by_user,
};
use build_html::{Html as HtmlBuilder, HtmlContainer, Table, TableCell, TableCellType, TableRow};
#[cfg(test)]
use mockall::automock;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Contains the method to abstract the writing of HTML files
#[cfg_attr(test, automock)]
trait HtmlWriter {
    fn write_html(&self, data: &str, path: &Path) -> Result<(), HtmlError>;
}

/// Default implementation that writes to actual files
struct FileHtmlWriter;

impl HtmlWriter for FileHtmlWriter {
    fn write_html(&self, data: &str, path: &Path) -> Result<(), HtmlError> {
        Ok(fs::write(path, data)?)
    }
}

/// The JavaScript extracted from the Charming-generated HTML files.
struct ExtractedChartJs {
    /// The function that defines the chart.
    chart: String,
    /// `<script>` tags that load the `ECharts` JavaScript in the dashboard.
    external_script_tags: String,
}

/// Creates a HTML file that contains statistics and charts about the given time logs.
/// Returns the path of the created HTML file.
/// # Errors
/// Possible errors can be seen in [`HtmlError`].
#[cfg(not(tarpaulin_include))]
pub fn create_html(
    time_logs: &[TimeLog],
    charts_dir: &Path,
    label_filter: Option<&HashSet<String>>,
    label_others: Option<&Label>,
    repository_name: &str,
) -> Result<PathBuf, HtmlError> {
    create_html_with_writer(
        time_logs,
        charts_dir,
        label_filter,
        label_others,
        repository_name,
        &FileHtmlWriter,
    )
}

/// Implementation of [`create_html()`] that takes a [`HtmlWriter`] to write the file.
fn create_html_with_writer(
    time_logs: &[TimeLog],
    charts_dir: &Path,
    label_filter: Option<&HashSet<String>>,
    label_others: Option<&Label>,
    repository_name: &str,
    writer: &impl HtmlWriter,
) -> Result<PathBuf, HtmlError> {
    let parent_directory = charts_dir
        .parent()
        .ok_or(HtmlError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Path does not contain parent directory",
        )))?;

    let html_filename = format!(
        "{}_dashboard.html",
        repository_name
            .replace(", ", "_")
            .replace(' ', "-")
            .to_lowercase()
    );
    let html_path = parent_directory.join(html_filename);

    let html_string = create_html_string(
        time_logs,
        label_filter,
        label_others,
        charts_dir,
        repository_name,
    )?;
    writer.write_html(&html_string, &html_path)?;
    Ok(html_path)
}

/// Creates a string with the current timestamp in the ISO 8601 format.
fn create_timestamp() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Creates an HTML string from the given time logs and the HTML template
/// which then is used to create the HTML page.
fn create_html_string(
    time_logs: &[TimeLog],
    label_filter: Option<&HashSet<String>>,
    label_others: Option<&Label>,
    charts_dir: &Path,
    repository_name: &str,
) -> Result<String, HtmlError> {
    const TEMPLATE: &str = include_str!("templates/base.html");

    let timeframe_by_user_table = create_table_timelogs_in_timeframes_by_user(time_logs);
    let timelogs_by_label_table =
        create_table_total_time_by_label(time_logs, label_filter, label_others);
    let timelogs_by_milestone_table = create_table_timelogs_by_milestone(time_logs);

    // Extract the JS from the generated chart HTML files.
    let mut chart_files = fs::read_dir(charts_dir)?
        // Filter out unreadable files
        .filter_map(Result::ok)
        // Get the path to each file
        .map(|entry| entry.path())
        // Filter out non-HTML files
        .filter(|file| file.extension().is_some_and(|ext| ext == "html"))
        .collect::<Vec<_>>();

    // Sort the files by name so that the order is deterministic.
    chart_files.sort();

    let charts_js = chart_files
        .into_iter()
        // Get an index for each HTML file
        .enumerate()
        .map(|(index, html_file)| extract_js_from_html_files(index, &html_file))
        .collect::<Result<Vec<_>, _>>()?;

    let charts_divs = create_chart_divs(charts_js.len());

    // Take the external script tags from the first chart and use them for all charts.
    let chart_external_script_tags = charts_js
        .first()
        .map(|js| js.external_script_tags.clone())
        .unwrap_or_default();

    // Get the chart JS code from all charts and join them together.
    let chart_js_code = charts_js
        .into_iter()
        .map(|js| js.chart)
        .collect::<Vec<_>>()
        .join("\n");

    let main_title = &format!("{repository_name} Time Tracking Dashboard");
    let html = TEMPLATE
        .replace("$main_title", main_title)
        .replace("$timestamp", &create_timestamp())
        .replace("$sub_title_1", "Time Spent per User:")
        .replace("$content_1", &timeframe_by_user_table.to_html_string())
        .replace("$sub_title_2", "Time Spent per Label:")
        .replace("$content_2", &timelogs_by_label_table.to_html_string())
        .replace("$sub_title_3", "Time Spent per Milestone:")
        .replace("$content_3", &timelogs_by_milestone_table.to_html_string())
        .replace("$charts_divs", &charts_divs)
        .replace("$external_script_tags", &chart_external_script_tags)
        .replace("$charts_js", &chart_js_code);

    Ok(html)
}

/// Creates a string with `index` number of `<div>` tags for each chart.
fn create_chart_divs(index: usize) -> String {
    use std::fmt::Write;
    (0..index).fold(String::new(), |mut str, i| {
        let _ = write!(str, r#"<div id="chart-{i}"></div>"#);
        str
    })
}

/// Creates the Table that shows the time spent per user in the last N days.
fn create_table_timelogs_in_timeframes_by_user(time_logs: &[TimeLog]) -> Table {
    let (mut table_data, table_header) = populate_table_timelogs_in_timeframes_by_user(time_logs);

    let totals_row = table_data
        .pop()
        .expect("Table should always have at least one row");

    let mut table = Table::from(table_data).with_header_row(table_header);
    let mut footer_row = TableRow::new().with_attributes([("class", "total-row")]);

    for cell_text in totals_row {
        footer_row = footer_row.with_cell(TableCell::new(TableCellType::Data).with_raw(cell_text));
    }
    table.add_custom_footer_row(footer_row);
    table
}

/// Creates the Table that shows the time spent per label.
fn create_table_total_time_by_label(
    time_logs: &[TimeLog],
    label_filter: Option<&HashSet<String>>,
    label_others: Option<&Label>,
) -> Table {
    let (table_data, table_header) =
        populate_table_timelogs_by_label(time_logs, label_filter, label_others);
    Table::from(table_data).with_header_row(table_header)
}

/// Creates a Table that shows the time spent per milestone.
fn create_table_timelogs_by_milestone(time_logs: &[TimeLog]) -> Table {
    let (table_data, table_header) = populate_table_timelogs_by_milestone(time_logs);
    Table::from(table_data).with_header_row(table_header)
}

/// Extracts the content of the last `<script>` tag from an HTML file.
fn extract_js_from_html_files(
    index: usize,
    entry: &PathBuf,
) -> Result<ExtractedChartJs, HtmlError> {
    extract_charming_chart_js(&fs::read_to_string(entry)?, &format!("chart-{index}"))
}

/// Extracts the content of the last `<script>` tag from an HTML string.
fn extract_charming_chart_js(
    html: &str,
    target_div_id: &str,
) -> Result<ExtractedChartJs, HtmlError> {
    let document_root = Html::parse_document(html);
    let script_tags_selector = Selector::parse("script").expect("Selector should always be valid");

    // Extract the <script src=""> tags that load the ECharts JavaScript libraries.
    let external_script_tags = document_root
        .select(&script_tags_selector)
        .filter(|node| node.attr("src").is_some())
        .map(|node| node.html())
        .collect::<Vec<_>>()
        .join("\n");

    // Extract the <script> tag that defines the chart.
    let chart_script_tag = document_root
        .select(&script_tags_selector)
        .next_back()
        .ok_or(HtmlError::ChartExtraction(
            "No <script> tag found in chart HTML".to_string(),
        ))?;

    let script_body = chart_script_tag.text().collect::<String>()
        .replace(
            "document.getElementById('chart')",
            &format!("document.getElementById('{target_div_id}')"),
        )
        .replace(
            "chart.setOption(option);",
            "if (typeof getChartBackgroundColor === 'function') { option.backgroundColor = getChartBackgroundColor(); }\n  chart.setOption(option);",
        );

    // Wrap in a function to avoid leaking vars into global scope
    let wrapped = format!("(function() {{\n{script_body}\n}})();");

    Ok(ExtractedChartJs {
        chart: wrapped,
        external_script_tags,
    })
}

/// Errors that can occur during HTML file creation.
#[derive(Debug, thiserror::Error)]
pub enum HtmlError {
    /// An error has occurred when reading or writing files to disk.
    #[error("I/O error while reading/writing HTML file: {0}")]
    Io(#[from] std::io::Error),

    /// An error occurred when extracting the chart JS from the Charming-generated HTML.
    #[error("Error extracting chart JS from Charming HTML: {0}")]
    ChartExtraction(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Issue, MergeRequest, TrackableItem, TrackableItemFields, TrackableItemKind, User, UserNodes,
    };
    use chrono::{Duration, Local};
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    const REPOSITORY_NAME: &str = "Test Repository";
    const HTML_FILE_NAME: &str = "test-repository_dashboard.html";
    const EXTERNAL_SCRIPTS: &str = r#"<script src="https://cdn.jsdelivr.net/npm/echarts@5.5.1/dist/echarts.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/echarts-gl@2.0.9/dist/echarts-gl.min.js"></script>"#;

    fn get_timelogs() -> Vec<TimeLog> {
        vec![
            TimeLog {
                spent_at: Local::now(),
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
                spent_at: Local::now() - Duration::days(1),
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

    fn setup_charts_dir(path: &Path) {
        let charts_dir = path.join("charts");
        if !charts_dir.exists() {
            fs::create_dir(path.join("charts")).unwrap();
        }

        fs::write(
            path.join("charts/burndown-per-person.html"),
            format!(
                "<html><body>
                {EXTERNAL_SCRIPTS}
                <script>
                var chart = echarts.init(document.getElementById('chart');
                var option = {{
                    title: {{ text: 'Burndown Chart per Person' }},
                }}
                </script></div></body></html>"
            ),
        )
        .unwrap();

        fs::write(
            path.join("charts/barchart-Users.html"),
            format!(
                "<html><body>
                {EXTERNAL_SCRIPTS}
                <script>
                var chart = echarts.init(document.getElementById('chart');
                var option = {{
                    title: {{ text: 'Hours spent by Users' }},
                }}
                </script></div></body></html>",
            ),
        )
        .unwrap();
    }

    #[test]
    fn test_create_html_mocked() {
        let root_dir = tempdir().unwrap();
        setup_charts_dir(root_dir.path());
        let root_dir_path = root_dir.path().to_path_buf();

        let mut mock_writer = MockHtmlWriter::new();
        let captured_html = Arc::new(Mutex::new(String::new()));
        let clone_for_closure = Arc::clone(&captured_html);
        let root_dir_path_clone = root_dir_path.clone();

        mock_writer
            .expect_write_html()
            .times(1)
            .withf(move |_, path| path == root_dir_path_clone.join(HTML_FILE_NAME))
            .returning(move |data, _| {
                // Extract the HTML from the closure
                *clone_for_closure.lock().unwrap() = data.to_string();
                Ok(())
            });
        let time_logs = get_timelogs();

        let charts_dir = root_dir.path().join("charts");
        let result = create_html_with_writer(
            &time_logs,
            &charts_dir,
            None,
            None,
            REPOSITORY_NAME,
            &mock_writer,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), root_dir_path.join(HTML_FILE_NAME));

        let html = captured_html.lock().unwrap();

        assert!(html.contains(REPOSITORY_NAME));

        assert!(html.contains("<table>"));
        assert!(html.contains("<th>User</th>"));
        assert!(html.contains("<th>Today</th>"));
        assert!(html.contains("<td>User 1</td>"));
        assert!(html.contains("<td>01h 00m</td>"));

        assert!(html.contains("chart-0"));
        assert!(html.contains("script src"));
        assert!(html.contains("title: { text: 'Burndown Chart per Person' }"));
    }

    #[test]
    fn test_create_html_string() {
        let time_logs = get_timelogs();
        let root_dir = tempdir().unwrap();
        setup_charts_dir(root_dir.path());
        let html = create_html_string(
            &time_logs,
            None,
            None,
            &root_dir.path().join("charts"),
            REPOSITORY_NAME,
        );
        assert!(html.is_ok());
        let html = html.unwrap();
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>User</th>"));
        assert!(html.contains("<th>Today</th>"));
        assert!(html.contains("var chart = echarts.init(document.getElementById('chart-0')"));
        assert!(html.contains("var chart = echarts.init(document.getElementById('chart-1')"));
    }

    #[test]
    fn test_extract_charming_chart_js_from_string() {
        let html = EXTERNAL_SCRIPTS.to_string()
            + r#"
            <div id="chart"></div>
            <script>
                var chart = echarts.init(document.getElementById('chart'));
            </script>"#;
        let result = extract_charming_chart_js(&html, "chart-0").unwrap();

        let js_code = result.chart;
        assert!(js_code.contains("var chart"));
        assert!(js_code.starts_with("(function() {"));
        assert!(js_code.ends_with("})();"));
        assert!(js_code.contains("document.getElementById('chart-0')"));
        assert!(!js_code.contains("document.getElementById('chart')"));

        let external_script_tags = result.external_script_tags;
        assert_eq!(external_script_tags, EXTERNAL_SCRIPTS);
    }

    #[test]
    fn test_extract_charming_chart_js_from_string_nonexisting_tag() {
        let html = r#"
            <div id="chart"></div>
        "#;
        let result = extract_charming_chart_js(html, "chart-0");
        let error_msg = "No <script> tag found in chart HTML";
        assert!(
            matches!(result,Err(HtmlError::ChartExtraction(err_msg)) if err_msg.eq(&error_msg))
        );
    }

    #[test]
    fn test_create_chart_divs() {
        let divs = create_chart_divs(3);
        assert_eq!(
            divs,
            r#"<div id="chart-0"></div><div id="chart-1"></div><div id="chart-2"></div>"#
        );
    }
}
