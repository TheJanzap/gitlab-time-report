//! Data structure with settings for accessing the GitLab API.

use thiserror::Error;

/// Contains all information needed for a call to the GitLab API.
/// Create a new instance with [`FetchOptions::new()`].
#[derive(Default, Debug, PartialEq, Clone)]
pub struct FetchOptions {
    /// The protocol the GitLab server should be contacted with.
    pub(super) protocol: String,
    /// The host part of the project URL, i.e. `gitlab.ost.ch`
    pub(super) host: String,
    /// The path part of the project URL, i.e. `gitlab-time-report/gitlab-time-report`
    pub(super) path: String,
    /// A GitLab access token. Required if the project visibility is set to "internal" or "private".
    pub(super) token: Option<String>,
}

impl FetchOptions {
    /// Creates a new `FetchOption` instance. Takes the full URL to a GitLab repository and a
    /// access token, if needed.
    /// ```
    /// # use gitlab_time_report::FetchOptions;
    /// let access_token = "MyAccessToken".to_string();
    /// let options_result = FetchOptions::new("https://gitlab.com/gitlab-org/gitlab", Some(access_token));
    /// // Check for errors
    /// let Ok(option) = options_result else {
    ///     panic!("Error happened when creating FetchOption: {:?}", options_result.unwrap_err());
    /// };
    ///  println!("{:?}", option);
    /// ```
    ///
    /// # Errors
    /// The function returns an `Err` if the URL path component does not contain two `/`, i.e. `https://gitlab.com/gitlab-org`
    pub fn new(url: &str, token: Option<String>) -> Result<Self, FetchOptionsError> {
        let (protocol, host, path) =
            Self::split_url(url).ok_or(FetchOptionsError::InvalidUrl(url.into()))?;
        Ok(Self {
            protocol,
            host,
            path,
            token,
        })
    }

    /// Takes a URL and splits it into protocol, host and path component.
    fn split_url(full_url: &str) -> Option<(String, String, String)> {
        // Get the protocol of the URL if specified. If not, use https.
        let (protocol, rest) = full_url.split_once("://").unwrap_or(("https", full_url));

        let url_parts = rest.splitn(2, '/').collect::<Vec<_>>();

        // Check if the URL contains a host and a path and
        // if the path contains both the user/group name and the project name.
        if url_parts.len() != 2 || !url_parts[1].contains('/') {
            return None;
        }
        Some((protocol.into(), url_parts[0].into(), url_parts[1].into()))
    }
}

/// Possible errors when creating a [`FetchOptions`].
#[derive(Debug, Error)]
pub enum FetchOptionsError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_new_fetch_options() {
        let url = "https://gitlab.ost.ch/gitlab-time-report/gitlab-time-report";
        let token = "MyAccessToken".to_string();
        let result = FetchOptions {
            protocol: "https".into(),
            host: "gitlab.ost.ch".into(),
            path: "gitlab-time-report/gitlab-time-report".into(),
            token: Some(token.clone()),
        };

        let output = FetchOptions::new(url, Some(token)).unwrap();
        assert_eq!(output, result);
    }

    #[test]
    fn create_new_fetch_options_without_protocol() {
        let url = "gitlab.ost.ch/gitlab-time-report/gitlab-time-report";
        let token = "MyAccessToken".to_string();
        let result = FetchOptions {
            protocol: "https".into(),
            host: "gitlab.ost.ch".into(),
            path: "gitlab-time-report/gitlab-time-report".into(),
            token: Some(token.clone()),
        };

        let output = FetchOptions::new(url, Some(token)).unwrap();
        assert_eq!(output, result);
    }

    #[test]
    fn split_url_correctly() {
        let input = "https://gitlab.ost.ch/gitlab-time-report/gitlab-time-report";
        let result = (
            "https".into(),
            "gitlab.ost.ch".into(),
            "gitlab-time-report/gitlab-time-report".into(),
        );
        let output = FetchOptions::split_url(input).unwrap();
        assert_eq!(output, result);
    }

    #[test]
    fn split_url_one_slash() {
        let input = "https://gitlab.com/gitlab-org";
        let output = FetchOptions::split_url(input);
        assert!(output.is_none());
    }
}
