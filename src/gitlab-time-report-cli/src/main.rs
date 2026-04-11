//! This crate provides a Command Line Interface to fetch time logs from a GitLab project and
//! generate statistics and charts from them.

#![cfg(not(tarpaulin_include))]
mod arguments;
mod fetch_projects;
mod print_table;

use arguments::Command;
use chrono::NaiveDate;
use clap::Parser;
use gitlab_time_report::charts::{BurndownType, ChartSettingError};
use gitlab_time_report::dashboard::create_html;
use gitlab_time_report::model::{Label, Milestone, TimeLog, TrackableItemKind};
use gitlab_time_report::validation::{TimeLogValidator, ValidationProblem};
use gitlab_time_report::{TimeDeltaExt, charts, create_csv, filters};
use std::collections::{BTreeMap, HashSet};

fn main() -> Result<(), String> {
    let cli = arguments::Arguments::parse();

    let project =
        fetch_projects::fetch_projects(cli.url, cli.token.as_ref()).map_err(|e| e.to_string())?;

    let start_date_for_validation: Option<NaiveDate> = match &cli.command {
        Some(Command::Charts { chart_options } | Command::Dashboard { chart_options, .. }) => {
            chart_options.start_date
        }
        _ => None,
    };

    validate_time_logs(
        &project.time_logs,
        cli.validation_details,
        start_date_for_validation,
        cli.validation_max_hours,
    );

    match &cli.command {
        Some(Command::Export { output }) => {
            create_csv(&project.time_logs, output.clone()).map_err(|e| e.to_string())?;
            println!("CSV created at {}", output.display());
        }
        Some(Command::Charts { chart_options }) => {
            let (selected_labels, other_label) = create_label_options(cli.labels);
            create_charts(
                &project.time_logs,
                chart_options,
                selected_labels.as_ref(),
                other_label.as_ref(),
                &project.name,
            )
            .map_err(|e| e.to_string())?;
        }
        Some(Command::Dashboard { chart_options }) => {
            let (selected_labels, other_label) = create_label_options(cli.labels);
            create_charts(
                &project.time_logs,
                chart_options,
                selected_labels.as_ref(),
                other_label.as_ref(),
                &project.name,
            )
            .map_err(|e| e.to_string())?;

            let path = create_html(
                &project.time_logs,
                &chart_options.output,
                selected_labels.as_ref(),
                other_label.as_ref(),
                &project.name,
            )
            .map_err(|e| e.to_string())?;
            println!("Dashboard '{}' successfully created.", path.display());
        }
        None => {
            print_table::print_timelogs_in_timeframes_by_user(&project.time_logs);

            let (selected_labels, other_label) = create_label_options(cli.labels);
            print_table::print_total_time_by_label(
                &project.time_logs,
                selected_labels.as_ref(),
                other_label.as_ref(),
            );

            print_table::print_total_time_by_milestone(&project.time_logs);
            print_table::print_todays_timelogs(&project.time_logs);
        }
    }
    Ok(())
}

/// Runs validation on the entered time logs with all validators and outputs the result to the console.
pub(crate) fn validate_time_logs(
    time_logs: &[TimeLog],
    show_validation_details: bool,
    start_date: Option<chrono::NaiveDate>,
    max_hours: u16,
) {
    let mut validator = TimeLogValidator::new()
        .with_validator(gitlab_time_report::validation::ExcessiveHoursValidator::new(max_hours))
        .with_validator(gitlab_time_report::validation::HasSummaryValidator)
        .with_validator(gitlab_time_report::validation::NoFutureDateValidator)
        .with_validator(gitlab_time_report::validation::DuplicatesValidator::new());

    if let Some(start_date) = start_date {
        validator = validator.with_validator(
            gitlab_time_report::validation::BeforeStartDateValidator::new(start_date),
        );
    }

    let results = validator.validate(time_logs);
    let number_of_problems = results.iter().filter(|r| !r.is_valid()).count();

    // Print summary and return when detailed listing is not desired
    if !show_validation_details {
        match number_of_problems {
            0 => println!("\nNo problems found in the time logs of the project."),
            _ => println!(
                "\n{number_of_problems} problems found in the time logs of the project. To see the problems, run with --validation-details",
            ),
        }
        return;
    }

    println!(
        "\
=============================================
      Time Logs with validation problems
============================================="
    );

    for result in results {
        if result.is_valid() {
            continue;
        }

        let time_log = result.time_log;
        let trackable_item_text = match &time_log.trackable_item.kind {
            TrackableItemKind::Issue(_) => "Issue #",
            TrackableItemKind::MergeRequest(_) => "Merge Request !",
        };

        println!(
            "{trackable_item_text}{}: {}",
            time_log.trackable_item.common.id, time_log.trackable_item.common.title,
        );
        println!(
            "{}, ({}), {}: {}",
            time_log.spent_at.date_naive(),
            time_log.time_spent.to_hm_string(),
            time_log.user.name,
            time_log.summary.as_deref().unwrap_or_default()
        );

        for problem in &result.problems {
            match problem {
                ValidationProblem::ExcessiveHours { max_hours } => {
                    println!("Time spent exceeds maximum of {max_hours} hours");
                }
                ValidationProblem::MissingSummary => println!("No summary was entered"),
                ValidationProblem::FutureDate => println!("Date is in the future"),
                ValidationProblem::DuplicateEntry => println!("Duplicate entry"),
                ValidationProblem::BeforeStartDate { start_date } => println!(
                    "Date is before project start date: {}",
                    start_date.format("%Y-%m-%d")
                ),
            }
            println!();
        }
    }
    println!(
        "\
=============================================
Total Problems found: {number_of_problems}
=============================================\n",
    );
}

/// Creates the graphs based on the time logs and the chart options.
pub(crate) fn create_charts(
    time_logs: &[TimeLog],
    options: &arguments::ChartOptionsArgs,
    selected_labels: Option<&HashSet<String>>,
    other_label: Option<&Label>,
    repository_name: &str,
) -> Result<(), ChartSettingError> {
    let burndown_options = charts::BurndownOptions::new(
        time_logs,
        options.weeks_per_sprint,
        options.sprints,
        options.hours_per_person,
        options.start_date,
    )?;
    let mut render_options = charts::RenderOptions::new(
        options.width,
        options.height,
        options.theme_json.as_deref(),
        &options.output,
        repository_name,
    )?;

    println!("Creating Bar Chart for Hours spent by Users...");
    let by_user: BTreeMap<_, _> = filters::group_by_user(time_logs).collect();
    charts::create_bar_chart(
        by_user.clone(),
        "Hours spent by Users",
        "Users",
        &mut render_options,
    )?;

    println!("Creating Burndown Charts...");
    charts::create_burndown_chart(
        time_logs,
        &BurndownType::PerPerson,
        &burndown_options,
        &mut render_options,
    )?;
    charts::create_burndown_chart(
        time_logs,
        &BurndownType::Total,
        &burndown_options,
        &mut render_options,
    )?;

    println!("Creating Bar Chart for Hours spent by Milestones...");
    let by_milestone = filters::group_by_milestone(time_logs).collect();
    charts::create_bar_chart::<Milestone>(
        by_milestone,
        "Hours spent by Milestones",
        "Milestones",
        &mut render_options,
    )?;

    println!("Creating Pie Chart for Hours spent by Labels...");
    let by_label: BTreeMap<_, _> =
        filters::group_by_label(time_logs, selected_labels, other_label).collect();
    charts::create_pie_chart(
        by_label.clone(),
        "Hours spent by Labels",
        &mut render_options,
    )?;

    println!("Creating Bar Chart for Hours spent by Label and User...");
    let by_label_and_user = by_label
        .clone()
        .into_iter()
        .map(|(label, logs)| {
            let time_by_label = filters::group_by_user(logs).collect();
            (label, time_by_label)
        })
        .collect::<BTreeMap<Option<_>, BTreeMap<_, Vec<_>>>>();
    charts::create_grouped_bar_chart(
        by_label_and_user,
        "Hours spent by Label and User",
        50.0,
        &mut render_options,
    )?;

    println!("Creating Bar Chart of Estimates and actual time by Labels...");
    charts::create_estimate_chart::<Label>(
        by_label,
        "Estimates and actual time per Label",
        &mut render_options,
    )?;

    println!("Creating Pie Chart for Hours spent by Issue or Merge Request...");
    let by_type: BTreeMap<_, _> = filters::group_by_type(time_logs).collect();
    let by_type_refs: BTreeMap<_, _> = by_type.iter().map(|(k, v)| (k, v.clone())).collect();
    charts::create_pie_chart(
        by_type_refs.clone(),
        "Hours spent by Issue or Merge Request",
        &mut render_options,
    )?;

    println!("Creating Bar Chart for Hours spent by Issue or Merge Request and User...");
    let by_type_and_user = by_type_refs
        .iter()
        .map(|(kind, logs)| {
            let time_by_label = filters::group_by_user(logs.clone()).collect();
            (kind, time_by_label)
        })
        .collect::<BTreeMap<_, BTreeMap<_, Vec<_>>>>();
    charts::create_grouped_bar_chart(
        by_type_and_user,
        "Hours spent by Issue or Merge Request and User",
        0.0,
        &mut render_options,
    )?;

    println!(
        "Charts successfully created in directory '{}'.",
        options.output.display()
    );

    Ok(())
}

/// Checks if a list of labels has been given.
/// If yes, create a [`HashSet`] from the labels and return it, including an "Others" label for the
/// rest of the labels which are not included in the set.
pub(crate) fn create_label_options(
    labels: Vec<String>,
) -> (Option<HashSet<String>>, Option<Label>) {
    let selected_labels = match labels.is_empty() {
        true => None,
        false => Some(HashSet::from_iter(labels)),
    };

    // Only use the "Others" label if there are labels selected.
    let other_label = selected_labels.as_ref().map(|_| Label {
        title: "Others".to_string(),
    });

    (selected_labels, other_label)
}
