use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, ACCEPT, USER_AGENT};
use reqwest::StatusCode;

use crate::error::GithubError;

pub trait GithubDownload {
    fn download_from_url(&self, url: &str) -> Result<Response, GithubError>;
}

pub struct GithubDownloader {}

impl GithubDownloader {
    pub fn new() -> Self {
        GithubDownloader {}
    }
}

impl Default for GithubDownloader {
    fn default() -> Self {
        GithubDownloader::new()
    }
}

impl GithubDownload for GithubDownloader {
    fn download_from_url(&self, url: &str) -> Result<Response, GithubError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "gehelper-lib".parse().unwrap());
        headers.insert(ACCEPT, "application/vnd.github.v3+json".parse().unwrap());

        let client = Client::builder().default_headers(headers).build().unwrap();

        let response = client
            .get(url)
            .send()
            .map_err(|err| GithubError::ReqwestError { source: err })?;

        match response.status() {
            StatusCode::OK => Ok(response),
            _ => Err(GithubError::StatusNotOk(response)),
        }
    }
}

#[cfg(test)]
mod tests {
    use httpmock::Method::GET;
    use httpmock::MockServer;

    use super::*;

    #[test]
    fn successful_url_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/releases")
                .header("User-Agent", "gehelper-lib")
                .header("Accept", "application/vnd.github.v3+json");
            then.status(200).header("Content-Type", "application/json").body("{}");
        });

        let downloader = GithubDownloader::new();
        let response = downloader.download_from_url(&server.url("/releases")).unwrap();

        mock.assert();
        assert_eq!(response.status(), 200);
    }

    #[test]
    fn not_found_url_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/releases")
                .header("User-Agent", "gehelper-lib")
                .header("Accept", "application/vnd.github.v3+json");
            then.status(404);
        });

        let downloader = GithubDownloader::new();
        let response = downloader.download_from_url(&server.url("/releases"));

        mock.assert();
        assert!(response.is_err());

        let err = response.unwrap_err();
        assert!(matches!(err, GithubError::StatusNotOk(_)));
    }

    #[test]
    fn other_errors_should_be_returned_as_result() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/releases")
                .header("User-Agent", "gehelper-lib")
                .header("Accept", "application/vnd.github.v3+json");
            then.status(500);
        });

        let downloader = GithubDownloader::new();
        let result = downloader.download_from_url(&server.url("/releases"));

        mock.assert();
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, GithubError::StatusNotOk(_)));
    }
}
