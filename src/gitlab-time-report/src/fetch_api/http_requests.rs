//! This module is an abstraction of the `reqwest` API so dependency injection can be used for
//! unit and integration tests.

// Exclude file from testing, as we would be testing the library implementation.
#![cfg(not(tarpaulin_include))]

use super::fetch_options::FetchOptions;
#[cfg(test)]
use mockall::automock;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use thiserror::Error;

/// Contains methods to abstract HTTP calls.
#[cfg_attr(test, automock)]
pub(super) trait HttpFetcher {
    /// Send a HTTP POST request with JSON as payload.
    fn http_post_request(
        &self,
        url: &str,
        payload: serde_json::Value,
        options: &FetchOptions,
    ) -> Result<String, NetworkError>;
}

impl HttpFetcher for Client {
    fn http_post_request(
        &self,
        url: &str,
        payload: serde_json::Value,
        options: &FetchOptions,
    ) -> Result<String, NetworkError> {
        let mut request = self.post(url).json(&payload);

        // Add the access token to the query if specified
        if let Some(token) = &options.token {
            request = request.bearer_auth(token);
        }

        Ok(request.send()?.error_for_status()?.text()?)
    }
}

/// A generic error that failures in the library can be transformed into.
/// Does still use [`StatusCode`] as return type for error handling.
#[derive(Debug, Clone, Error)]
pub enum NetworkError {
    #[error("HTTP error: {0}")]
    StatusCode(StatusCode),
    #[error("Connection to {0} failed")]
    Connection(String),
}

/// Implements the conversion from [`reqwest::Error`] into [`NetworkError`].
impl From<reqwest::Error> for NetworkError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() {
            return Self::Connection(
                e.url()
                    .expect("Should have a connection URL here")
                    .to_string(),
            );
        }
        let status = e.status().expect("Should have a status code here");
        Self::StatusCode(status)
    }
}
