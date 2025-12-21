//! This library exports time logs from a GitLab project and creates statistics and charts
//! showing the working hours spent on it.

pub mod charts;
mod chrono_extensions;
pub mod dashboard;
pub mod export;
mod fetch_api;
pub mod filters;
pub mod model;
pub mod tables;
pub mod validation;

pub use chrono_extensions::TimeDeltaExt;
pub use export::create_csv;
pub use fetch_api::{FetchOptions, QueryError, fetch_project_time_logs};
