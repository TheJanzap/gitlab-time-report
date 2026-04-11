//! Contains functions to print the generated tables from the library to the console.

#![cfg(not(tarpaulin_include))]

use cli_table::format::Justify;
use cli_table::{Cell, CellStruct, Style, Table, TableStruct, print_stdout};
use gitlab_time_report::model::{Label, TimeLog};
use gitlab_time_report::tables;
use regex::Regex;
use std::collections::HashSet;

/// Create a table from cells and column titles.
fn create_table(table_data: Vec<Vec<CellStruct>>, column_titles: &[&str]) -> TableStruct {
    let titles: Vec<_> = column_titles
        .iter()
        .map(|title| title.cell().bold(true))
        .collect();
    table_data.table().title(titles)
}

/// Print a table and handle errors
fn print_table(table: TableStruct) {
    print_stdout(table).unwrap_or_else(|err| {
        eprintln!("Error printing table: {err}");
        std::process::exit(4);
    });
}

/// Transform a `Vec<Vec<String>>` from the Library to a `Vec<Vec<CellStruct>>`
/// used by the `cli_table` crate. The first column is left-aligned, all others right-aligned.
fn to_cell_vec(table: Vec<Vec<String>>) -> Vec<Vec<CellStruct>> {
    let is_duration = Regex::new(r"^[0-9]{2,}h [0-9]{2}m$").expect("Regex pattern should compile");
    table
        .into_iter()
        .map(|row| {
            row.into_iter()
                // Check if the cell is a duration cell
                .map(|cell| match is_duration.is_match(&cell) {
                    true => cell.cell().justify(Justify::Right),
                    false => cell.cell(),
                })
                .collect()
        })
        .collect()
}

/// Print a table with the total time spent per selected label.
pub fn print_total_time_by_label(
    time_logs: &[TimeLog],
    label_filter: Option<&HashSet<String>>,
    label_others: Option<&Label>,
) {
    let (table_data, table_header) =
        tables::populate_table_timelogs_by_label(time_logs, label_filter, label_others);
    let table_data_cells = to_cell_vec(table_data);

    let table = create_table(table_data_cells, table_header);
    println!("\nTime Spent per Label:");
    print_table(table);
}

/// Print a table with the total time spent per milestone.
pub fn print_total_time_by_milestone(time_logs: &[TimeLog]) {
    let (table_data, table_header) = tables::populate_table_timelogs_by_milestone(time_logs);
    let table_data_cells = to_cell_vec(table_data);

    let table = create_table(table_data_cells, table_header);
    println!("\nTime Spent per Milestone:");
    print_table(table);
}

/// Print a table of today's, yesterday's, this week's and this month's time spent by user.
pub fn print_timelogs_in_timeframes_by_user(time_logs: &[TimeLog]) {
    let (table_data, table_header) =
        tables::populate_table_timelogs_in_timeframes_by_user(time_logs);

    // Convert the returned data to the correct format used with cli_table.
    let table_data_cells = to_cell_vec(table_data);

    let table = create_table(table_data_cells, table_header);
    println!("\nTime Spent per User:");
    print_table(table);
}

/// Print a table of today's time logs.
pub fn print_todays_timelogs(time_logs: &[TimeLog]) {
    let (table_data, table_header) = tables::populate_table_todays_timelogs(time_logs);
    println!("\nToday's Time Logs:");
    if table_data.is_empty() {
        println!("No time logs found for today.");
        return;
    }
    let table_data_cells = to_cell_vec(table_data);
    let table = create_table(table_data_cells, table_header);
    print_table(table);
}
