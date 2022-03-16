use std::fmt::Display;
use std::io::Read;

use lazy_static::lazy_static;
use reqwest::blocking::Response;

use crate::download::github::{GithubDownload, GithubDownloader};
use crate::download::response::{
    CompatibilityToolTag, DownloadedAssets, DownloadedChecksum, DownloadedTar, GeAsset, GeRelease,
};
use crate::error::GithubError;
use crate::tag::{Tag, TagKind, WineTagKind};

mod github;

pub mod response;

const APPLICATION_GZIP: &str = "application/gzip";
const APPLICATION_OCTET_STREAM: &str = "application/octet-stream";
const APPLICATION_XZ: &str = "application/x-xz";

const GITHUB_API_URL: &str = "https://api.github.com";
const PROTON_GE_RELEASE_LATEST_URL: &str = "repos/GloriousEggroll/proton-ge-custom/releases/latest";
const PROTON_GE_RELEASE_TAGS_URL: &str = "repos/GloriousEggroll/proton-ge-custom/releases/tags";
const WINE_GE_RELEASE_TAGS_URL: &str = "repos/GloriousEggroll/wine-ge-custom/releases/tags";
const WINE_GE_TAGS_URL: &str = "repos/GloriousEggroll/wine-ge-custom/tags";

lazy_static! {
    pub static ref GITHUB_PROTON_GE_LATEST_URL: String = format!("{}/{}", GITHUB_API_URL, PROTON_GE_RELEASE_LATEST_URL);
    pub static ref GITHUB_PROTON_GE_TAG_URL: String = format!("{}/{}", GITHUB_API_URL, PROTON_GE_RELEASE_TAGS_URL);
    pub static ref GITHUB_WINE_GE_RELEASE_TAG_URL: String = format!("{}/{}", GITHUB_API_URL, WINE_GE_RELEASE_TAGS_URL);
    pub static ref GITHUB_WINE_GE_TAGS_URL: String = format!("{}/{}", GITHUB_API_URL, WINE_GE_TAGS_URL);
}

pub trait ReadProgressWrapper {
    fn init(self: Box<Self>, len: u64) -> Box<dyn ReadProgressWrapper>;
    fn wrap(&self, reader: Box<dyn Read>) -> Box<dyn Read>;
    fn finish(&self, release: &GeAsset);
}

pub struct DownloadRequest {
    pub tag: Option<String>,
    pub kind: TagKind,
    pub progress_wrapper: Box<dyn ReadProgressWrapper>,
    pub skip_checksum: bool,
}

impl DownloadRequest {
    pub fn new(
        tag: Option<String>,
        kind: TagKind,
        progress_wrapper: Box<dyn ReadProgressWrapper>,
        skip_checksum: bool,
    ) -> Self {
        DownloadRequest {
            tag,
            kind,
            progress_wrapper,
            skip_checksum,
        }
    }
}

pub trait GeDownload {
    fn fetch_release(&self, tag: Option<String>, kind: TagKind) -> Result<GeRelease, GithubError>;
    fn download_release(&self, request: DownloadRequest) -> Result<DownloadedAssets, GithubError>;
}

pub struct GeDownloader {
    github_downloader: Box<dyn GithubDownload>,
}

impl GeDownloader {
    pub fn new(github_downloader: Box<dyn GithubDownload>) -> Self {
        GeDownloader { github_downloader }
    }

    fn create_url<S: AsRef<str>>(&self, tag: Option<S>, kind: &TagKind) -> Result<String, GithubError>
    where
        S: AsRef<str> + Display,
    {
        let tag = tag.as_ref();
        match kind {
            TagKind::Proton => {
                if let Some(t) = tag {
                    Ok(format!("{}/{}", *GITHUB_PROTON_GE_TAG_URL, t))
                } else {
                    Ok(String::from(&*GITHUB_PROTON_GE_LATEST_URL))
                }
            }
            TagKind::Wine { kind: wine_kind } => {
                if let Some(t) = tag {
                    Ok(format!("{}/{}", *GITHUB_WINE_GE_RELEASE_TAG_URL, t))
                } else {
                    self.find_latest_wine_ge_release_tag(wine_kind)
                        .map(|t| format!("{}/{}", *GITHUB_WINE_GE_RELEASE_TAG_URL, t))
                }
            }
        }
    }

    fn create_wine_ge_tags_url(&self, page: u8) -> String {
        format!("{}?page={}", *GITHUB_WINE_GE_TAGS_URL, page)
    }

    fn find_latest_wine_ge_release_tag(&self, kind: &WineTagKind) -> Result<Tag, GithubError> {
        let mut page = 1;
        loop {
            let mut tags: Vec<CompatibilityToolTag> = self.fetch_wine_ge_tags(page)?.json()?;

            if tags.is_empty() {
                return Err(GithubError::NoTags);
            }

            if let WineTagKind::LolWineGe = kind {
                tags.retain(|t| t.name.contains("LoL"));
            } else {
                tags.retain(|t| !t.name.contains("LoL"));
            }

            let latest_tag = tags
                .into_iter()
                .map(|t| Tag::from(t.name))
                .filter(|t| t.semver().is_some())
                .max_by(Tag::compare_semver);
            if let Some(t) = latest_tag {
                return Ok(t);
            }
            page += 1
        }
    }

    fn fetch_wine_ge_tags(&self, page: u8) -> Result<Response, GithubError> {
        let url = self.create_wine_ge_tags_url(page);
        self.github_downloader.download_from_url(&url)
    }

    fn download_tar(
        &self,
        progress_wrapper: Box<dyn ReadProgressWrapper>,
        asset: &GeAsset,
    ) -> Result<DownloadedTar, GithubError> {
        let response = self.github_downloader.download_from_url(&asset.browser_download_url)?;

        let tar_size: u64 = response.content_length().unwrap();
        let mut compressed_tar: Vec<u8> = Vec::with_capacity(tar_size as usize);

        let progress_wrapper = progress_wrapper.init(tar_size);
        progress_wrapper
            .wrap(Box::new(response))
            .read_to_end(&mut compressed_tar)
            .unwrap();
        progress_wrapper.finish(asset);

        Ok(DownloadedTar::new(compressed_tar, String::from(&asset.name)))
    }

    fn download_checksum(&self, asset: &GeAsset) -> Result<DownloadedChecksum, GithubError> {
        let mut response = self.github_downloader.download_from_url(&asset.browser_download_url)?;

        let file_size = response.content_length().unwrap();
        let mut checksum_str = String::with_capacity(file_size as usize);
        response.read_to_string(&mut checksum_str).unwrap();

        Ok(DownloadedChecksum::new(checksum_str, String::from(&asset.name)))
    }
}

impl GeDownload for GeDownloader {
    fn fetch_release(&self, tag: Option<String>, kind: TagKind) -> Result<GeRelease, GithubError> {
        let tag = tag.as_ref();
        let url = self.create_url(tag, &kind)?;
        self.github_downloader.download_from_url(&url).and_then(|response| {
            response
                .json::<GeRelease>()
                .map_err(|err| GithubError::ReqwestError { source: err })
        })
    }

    fn download_release(&self, request: DownloadRequest) -> Result<DownloadedAssets, GithubError> {
        let DownloadRequest {
            tag,
            kind,
            progress_wrapper,
            skip_checksum,
        } = request;

        let release = self.fetch_release(tag, kind)?;
        if release.assets.is_empty() {
            return Err(GithubError::ReleaseHasNoAssets {
                tag: release.tag_name,
                kind,
            });
        }

        let downloaded_checksum = match skip_checksum {
            false => Some(self.download_checksum(release.checksum_asset())?),
            true => None,
        };

        let downloaded_tar = self.download_tar(progress_wrapper, release.tar_asset())?;

        Ok(DownloadedAssets::new(
            release.tag_name,
            downloaded_tar,
            downloaded_checksum,
        ))
    }
}

impl Default for GeDownloader {
    fn default() -> Self {
        let github_downloader = Box::new(GithubDownloader::new());
        GeDownloader::new(github_downloader)
    }
}

#[cfg(test)]
mod tests {
    use httpmock::Method::GET;
    use httpmock::MockServer;
    use mockall::mock;
    use reqwest::blocking::Response;

    use super::*;

    lazy_static! {
        static ref RELEASES: &'static str = "tests/resources/responses/releases";
        static ref ASSETS: &'static str = "tests/resources/assets";
        static ref TAGS: &'static str = "tests/resources/responses/tags";
        pub static ref PROTON_GE: String = format!("{}/proton-ge-release.json", *RELEASES);
        pub static ref WINE_GE: String = format!("{}/wine-ge-release.json", *RELEASES);
        pub static ref WINE_GE_LOL: String = format!("{}/wine-ge-lol-release.json", *RELEASES);
        pub static ref NO_TAGS: String = format!("{}/empty.json", *TAGS);
        pub static ref WINE_GE_TAGS: String = format!("{}/wine_ge.json", *TAGS);
        pub static ref WINE_GE_LOL_TAGS: String = format!("{}/wine_ge_lol.json", *TAGS);
        pub static ref TEST_TAR_GZ: String = format!("{}/{}", *ASSETS, "test.tar.gz");
        pub static ref TEST_SHA512SUM: String = format!("{}/{}", *ASSETS, "test-gz.sha512sum");
    }

    mock! {
        ProgressWrapper {}
        impl ReadProgressWrapper for ProgressWrapper {
            fn init(self: Box<Self>, len: u64) -> Box<dyn ReadProgressWrapper>;
            fn wrap(&self, reader: Box<dyn Read>) -> Box<dyn Read>;
            fn finish(&self, release: &GeAsset);
        }
    }

    pub fn download_url(server: Option<&str>, tag: &str, kind: &TagKind, file_name: &str) -> String {
        let url = download_url_without_server(tag, kind, file_name);

        if let Some(host) = server {
            format!("{}{}", host, url)
        } else {
            format!("SERVER{}", url)
        }
    }

    pub fn download_url_without_server(tag: &str, kind: &TagKind, file_name: &str) -> String {
        let repo = match kind {
            TagKind::Proton => "proton",
            TagKind::Wine { .. } => "wine",
        };
        format!(
            "/GloriousEggroll/{}-ge-custom/releases/download/{}/{}",
            repo, tag, file_name
        )
    }

    pub fn mock_url(kind: &TagKind, server: &str) -> String {
        let file_path = match kind {
            TagKind::Proton => &*PROTON_GE,
            TagKind::Wine { kind: wine_kind } => match wine_kind {
                WineTagKind::WineGe => &*WINE_GE,
                WineTagKind::LolWineGe => &*WINE_GE_LOL,
            },
        };
        std::fs::read_to_string(file_path).unwrap().replace("SERVER", server)
    }

    struct FetchSpecifiedReleaseTestData {
        given_tag: Option<String>,
        expected_tag: String,
        kind: TagKind,
        github_resource: String,
        body_content_file: String,
        gzip_name: String,
        gzip_download_url: String,
        checksum_name: String,
        checksum_download_url: String,
    }

    impl FetchSpecifiedReleaseTestData {
        pub fn new(
            tag: Option<&str>,
            expected_tag: &str,
            kind: TagKind,
            gzip_name: &str,
            checksum_name: &str,
            github_resource: &str,
            body_content_file: &str,
        ) -> Self {
            let github_resource = if tag.is_some() {
                format!("{}/{}", github_resource, tag.unwrap())
            } else {
                String::from(github_resource)
            };
            let tag = tag.map(String::from);

            let gzip_download_url = download_url(None, expected_tag, &kind, gzip_name);
            let checksum_download_url = download_url(None, expected_tag, &kind, checksum_name);

            let expected_tag = String::from(expected_tag);
            let body_content_file = String::from(body_content_file);
            let gzip_name = String::from(gzip_name);
            let checksum_name = String::from(checksum_name);
            FetchSpecifiedReleaseTestData {
                given_tag: tag,
                expected_tag,
                kind,
                github_resource,
                body_content_file,
                gzip_name,
                gzip_download_url,
                checksum_name,
                checksum_download_url,
            }
        }
    }

    struct FetchLatestReleaseTestData {
        tags_body_from_file: String,
        github_tags_resource: String,
        expected_specific_release: FetchSpecifiedReleaseTestData,
    }

    impl FetchLatestReleaseTestData {
        pub fn new<S: Into<String>>(
            tags_body_from_file: S,
            github_tags_resource: S,
            mut expected_specific_release: FetchSpecifiedReleaseTestData,
        ) -> Self {
            let tags_body_from_file = tags_body_from_file.into();
            let github_tags_resource = github_tags_resource.into();
            expected_specific_release.github_resource = format!(
                "{}/{}",
                expected_specific_release.github_resource, expected_specific_release.expected_tag
            );

            FetchLatestReleaseTestData {
                tags_body_from_file,
                github_tags_resource,
                expected_specific_release,
            }
        }
    }

    struct FetchReleaseContentTestData {
        expected_tag: String,
        kind: TagKind,
        compressed_tar_file_name: String,
        checksum_file_name: String,
        release_url: String,
    }

    impl FetchReleaseContentTestData {
        pub fn new<S: Into<String>>(
            tag: S,
            kind: &TagKind,
            compressed_tar_file_name: S,
            checksum_file_name: S,
            release_url: S,
        ) -> Self {
            let expected_tag = tag.into();
            let kind = kind.clone();
            let compressed_tar_file_name = compressed_tar_file_name.into();
            let checksum_file_name = checksum_file_name.into();
            let release_url = release_url.into();

            FetchReleaseContentTestData {
                expected_tag,
                kind,
                compressed_tar_file_name,
                checksum_file_name,
                release_url,
            }
        }
    }

    struct MockGithubDownloader {
        host: String,
    }

    impl MockGithubDownloader {
        pub fn new<S: Into<String>>(host: S) -> Self {
            MockGithubDownloader { host: host.into() }
        }
    }

    impl GithubDownload for MockGithubDownloader {
        fn download_from_url(&self, url: &str) -> Result<Response, GithubError> {
            let find_index = match url.find("repos") {
                Some(i) => i,
                None => url.find("G").unwrap(),
            };

            let target = url.split_at(find_index).1;
            let mocked_url = format!("{}/{}", self.host, target);

            match reqwest::blocking::get(&mocked_url) {
                Ok(resp) => Ok(resp),
                Err(err) => panic!("Get request failed during integration test: {:?}", err),
            }
        }
    }

    fn fetch_release_test(test_data: FetchSpecifiedReleaseTestData) {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path(format!("/{}", &test_data.github_resource));
            then.status(200)
                .header("Content-Type", "application/json")
                .body_from_file(&test_data.body_content_file);
        });

        let github_downloader = Box::new(MockGithubDownloader::new(String::from(server.base_url())));
        let tool_downloader = GeDownloader::new(github_downloader);

        let release = tool_downloader
            .fetch_release(test_data.given_tag, test_data.kind)
            .unwrap();
        let gzip = release.tar_asset();
        let checksum = release.checksum_asset();
        assert_eq!(release.tag_name, test_data.expected_tag);
        assert_eq!(gzip.name, test_data.gzip_name);
        assert_eq!(gzip.browser_download_url, test_data.gzip_download_url);
        assert_eq!(gzip.content_type, APPLICATION_GZIP);
        assert_eq!(checksum.name, test_data.checksum_name);
        assert_eq!(checksum.browser_download_url, test_data.checksum_download_url);
        assert_eq!(checksum.content_type, APPLICATION_OCTET_STREAM);

        mock.assert();
    }

    fn fetch_latest_wine_or_lol_release_test(test_data: FetchLatestReleaseTestData) {
        let server = MockServer::start();
        let FetchLatestReleaseTestData {
            expected_specific_release: test_data,
            tags_body_from_file,
            github_tags_resource,
        } = test_data;

        let tags_mock = server.mock(|when, then| {
            when.method(GET)
                .path(format!("/{}", github_tags_resource))
                .query_param("page", "1");
            then.status(200)
                .header("Content-Type", "application/json")
                .body_from_file(tags_body_from_file);
        });

        let release_mock = server.mock(|when, then| {
            when.method(GET).path(format!("/{}", &test_data.github_resource));
            then.status(200)
                .header("Content-Type", "application/json")
                .body_from_file(&test_data.body_content_file);
        });

        let github_downloader = Box::new(MockGithubDownloader::new(String::from(server.base_url())));
        let tool_downloader = GeDownloader::new(github_downloader);

        let release = tool_downloader
            .fetch_release(test_data.given_tag, test_data.kind)
            .unwrap();
        let gzip = release.tar_asset();
        let checksum = release.checksum_asset();

        release_mock.assert();
        tags_mock.assert();

        assert_eq!(release.tag_name, test_data.expected_tag);
        assert_eq!(gzip.name, test_data.gzip_name);
        assert_eq!(gzip.browser_download_url, test_data.gzip_download_url);
        assert_eq!(gzip.content_type, APPLICATION_GZIP);
        assert_eq!(checksum.name, test_data.checksum_name);
        assert_eq!(checksum.browser_download_url, test_data.checksum_download_url);
        assert_eq!(checksum.content_type, APPLICATION_OCTET_STREAM);
    }

    #[test]
    fn fetch_proton_ge_release() {
        let expected_tag = "6.20-GE-1";
        let given_tag = Some(expected_tag);
        let gzip = "Proton-6.20-GE-1.tar.gz";
        let checksum = "Proton-6.20-GE-1.sha512sum";
        fetch_release_test(FetchSpecifiedReleaseTestData::new(
            given_tag,
            expected_tag,
            TagKind::Proton,
            gzip,
            checksum,
            PROTON_GE_RELEASE_TAGS_URL,
            &*PROTON_GE,
        ));
    }

    #[test]
    fn fetch_wine_ge_release() {
        let expected_tag = "6.20-GE-1";
        let given_tag = Some(expected_tag);
        let gzip = "wine-lutris-ge-6.20-1-x86_64.tar.gz";
        let checksum = "wine-lutris-ge-6.20-1-x86_64.sha512sum";
        fetch_release_test(FetchSpecifiedReleaseTestData::new(
            given_tag,
            expected_tag,
            TagKind::wine(),
            gzip,
            checksum,
            WINE_GE_RELEASE_TAGS_URL,
            &*WINE_GE,
        ));
    }

    #[test]
    fn fetch_wine_lol_ge_release() {
        let expected_tag = "6.16-GE-3-LoL";
        let given_tag = Some(expected_tag);
        let gzip = "wine-lutris-ge-6.16-3-lol-x86_64.tar.gz";
        let checksum = "wine-lutris-ge-6.16-3-lol-x86_64.sha512sum";
        fetch_release_test(FetchSpecifiedReleaseTestData::new(
            given_tag,
            expected_tag,
            TagKind::lol(),
            gzip,
            checksum,
            WINE_GE_RELEASE_TAGS_URL,
            &*WINE_GE_LOL,
        ));
    }

    #[test]
    fn fetch_latest_proton_release() {
        let expected_tag = "6.20-GE-1";
        let given_tag = None;
        let gzip = "Proton-6.20-GE-1.tar.gz";
        let checksum = "Proton-6.20-GE-1.sha512sum";
        fetch_release_test(FetchSpecifiedReleaseTestData::new(
            given_tag,
            expected_tag,
            TagKind::Proton,
            gzip,
            checksum,
            PROTON_GE_RELEASE_LATEST_URL,
            &*PROTON_GE,
        ));
    }

    #[test]
    fn fetch_latest_wine_ge_release() {
        let expected_tag = "6.20-GE-1";
        let given_tag = None;
        let gzip = "wine-lutris-ge-6.20-1-x86_64.tar.gz";
        let checksum = "wine-lutris-ge-6.20-1-x86_64.sha512sum";
        let expected_data = FetchSpecifiedReleaseTestData::new(
            given_tag,
            expected_tag,
            TagKind::wine(),
            gzip,
            checksum,
            WINE_GE_RELEASE_TAGS_URL,
            &*WINE_GE,
        );

        fetch_latest_wine_or_lol_release_test(FetchLatestReleaseTestData::new(
            &*WINE_GE_TAGS,
            &WINE_GE_TAGS_URL.to_owned(),
            expected_data,
        ));
    }

    #[test]
    fn fetch_latest_wine_ge_lol_release() {
        let expected_tag = "6.16-GE-3-LoL";
        let given_tag = None;
        let gzip = "wine-lutris-ge-6.16-3-lol-x86_64.tar.gz";
        let checksum = "wine-lutris-ge-6.16-3-lol-x86_64.sha512sum";
        let expected_data = FetchSpecifiedReleaseTestData::new(
            given_tag,
            expected_tag,
            TagKind::lol(),
            gzip,
            checksum,
            WINE_GE_RELEASE_TAGS_URL,
            &*WINE_GE_LOL,
        );
        fetch_latest_wine_or_lol_release_test(FetchLatestReleaseTestData::new(
            &*WINE_GE_LOL_TAGS,
            &WINE_GE_TAGS_URL.to_owned(),
            expected_data,
        ));
    }

    #[test]
    fn fetch_latest_wine_ge_lol_version_from_second_tags_page() {
        let tag = "6.16-GE-3-LoL";
        let kind = TagKind::lol();
        let gzip_file_name = "wine-lutris-ge-6.16-3-lol-x86_64.tar.gz";
        let checksum_file_name = "wine-lutris-ge-6.16-3-lol-x86_64.sha512sum";

        let server = MockServer::start();

        let first_page_tags = server.mock(|when, then| {
            when.method(GET)
                .path(format!("/{}", WINE_GE_TAGS_URL))
                .query_param("page", "1");
            then.status(200)
                .header("Content-Type", "application/json")
                .body_from_file(&*WINE_GE_TAGS);
        });

        let second_page_tags = server.mock(|when, then| {
            when.method(GET)
                .path(format!("/{}", WINE_GE_TAGS_URL))
                .query_param("page", "2");
            then.status(200)
                .header("Content-Type", "application/json")
                .body_from_file(&*WINE_GE_LOL_TAGS);
        });

        let release_mock = server.mock(|when, then| {
            when.method(GET).path(format!("/{}/{}", WINE_GE_RELEASE_TAGS_URL, tag));
            then.status(200)
                .header("Content-Type", "application/json")
                .body(mock_url(&kind, &server.base_url()));
        });

        let github_downloader = Box::new(MockGithubDownloader::new(String::from(server.base_url())));
        let tool_downloader = GeDownloader::new(github_downloader);

        let release = tool_downloader.fetch_release(None, TagKind::lol()).unwrap();
        let gzip = release.tar_asset();
        let checksum = release.checksum_asset();

        first_page_tags.assert();
        second_page_tags.assert();
        release_mock.assert();

        assert_eq!(release.tag_name, tag);
        assert_eq!(gzip.name, gzip_file_name);
        assert_eq!(
            gzip.browser_download_url,
            download_url(Some(&server.base_url()), tag, &kind, gzip_file_name)
        );
        assert_eq!(gzip.content_type, APPLICATION_GZIP);
        assert_eq!(checksum.name, checksum_file_name);
        assert_eq!(
            checksum.browser_download_url,
            download_url(Some(&server.base_url()), tag, &kind, checksum_file_name)
        );
        assert_eq!(checksum.content_type, APPLICATION_OCTET_STREAM);
    }

    #[test]
    fn fetch_latest_version_but_could_not_find_latest_tag() {
        let server = MockServer::start();

        let first_page_tags = server.mock(|when, then| {
            when.method(GET)
                .path(format!("/{}", WINE_GE_TAGS_URL))
                .query_param("page", "1");
            then.status(200)
                .header("Content-Type", "application/json")
                .body_from_file(&*WINE_GE_TAGS);
        });

        let second_page_tags = server.mock(|when, then| {
            when.method(GET)
                .path(format!("/{}", WINE_GE_TAGS_URL))
                .query_param("page", "2");
            then.status(200)
                .header("Content-Type", "application/json")
                .body_from_file(&*NO_TAGS);
        });

        let github_downloader = Box::new(MockGithubDownloader::new(String::from(server.base_url())));
        let tool_downloader = GeDownloader::new(github_downloader);

        let release = tool_downloader.fetch_release(None, TagKind::lol());
        assert!(release.is_err());
        let err = release.err().unwrap();
        assert!(
            matches!(err, GithubError::NoTags),
            "Result contains unexpected error: {:?}",
            err
        );

        first_page_tags.assert();
        second_page_tags.assert();
    }

    fn fetch_release_content_test(test_data: FetchReleaseContentTestData) {
        let FetchReleaseContentTestData {
            expected_tag,
            kind,
            compressed_tar_file_name: gzip_file_name,
            checksum_file_name,
            release_url,
        } = test_data;

        let server = MockServer::start();

        let release_mock = server.mock(|when, then| {
            when.method(GET).path(format!("/{}/{}", release_url, expected_tag));
            then.status(200)
                .header("Content-Type", "application/json")
                .body(mock_url(&kind, &server.base_url()));
        });

        let gzip_asset = server.mock(|when, then| {
            when.method(GET)
                .path(download_url_without_server(&expected_tag, &kind, &gzip_file_name));
            then.status(200)
                .header("Content-Type", "application/gzip")
                .body_from_file(&*TEST_TAR_GZ);
        });

        let checksum_asset = server.mock(|when, then| {
            when.method(GET)
                .path(download_url_without_server(&expected_tag, &kind, &checksum_file_name));
            then.status(200)
                .header("Content-Type", "application/octet-stream")
                .body_from_file(&*TEST_SHA512SUM);
        });

        // TODO: Assertions could be improved here, however, that will require changes to the test structure. In
        //  general some things (APPLICATION_XZ) have changed since these tests were written.
        let mut progress_wrapper = MockProgressWrapper::new();
        progress_wrapper.expect_finish().never();
        progress_wrapper.expect_wrap().never();
        progress_wrapper.expect_init().once().returning(|_| {
            let mut initialized_prog_wrapper = MockProgressWrapper::new();
            initialized_prog_wrapper.expect_init().never();
            initialized_prog_wrapper.expect_wrap().once().returning(|reader| reader);
            initialized_prog_wrapper.expect_finish().once().returning(|_| ());

            Box::new(initialized_prog_wrapper)
        });

        let github_downloader = Box::new(MockGithubDownloader::new(String::from(server.base_url())));
        let tool_downloader = GeDownloader::new(github_downloader);

        let request = DownloadRequest::new(Some(expected_tag), kind, Box::new(progress_wrapper), false);
        let fetched_assets = tool_downloader.download_release(request).unwrap();

        release_mock.assert();
        gzip_asset.assert();
        checksum_asset.assert();

        let expected_gzip_content = std::fs::read(&*TEST_TAR_GZ).unwrap();
        let expected_checksum_content = std::fs::read_to_string(&*TEST_SHA512SUM).unwrap();

        let downloaded_tar = fetched_assets.compressed_tar;
        let downloaded_checksum = fetched_assets.checksum.unwrap();

        assert_eq!(downloaded_tar.compressed_content, expected_gzip_content);
        assert_eq!(downloaded_tar.file_name, gzip_file_name);
        assert_eq!(downloaded_checksum.checksum, expected_checksum_content);
        assert_eq!(downloaded_checksum.file_name, checksum_file_name);
    }

    #[test]
    fn fetch_release_content_should_download_data_for_wine_ge() {
        let expected_tag = "6.16-GE-3-LoL";
        let kind = TagKind::lol();
        let gzip_file_name = "wine-lutris-ge-6.16-3-lol-x86_64.tar.gz";
        let checksum_file_name = "wine-lutris-ge-6.16-3-lol-x86_64.sha512sum";

        let data = FetchReleaseContentTestData::new(
            expected_tag,
            &kind,
            gzip_file_name,
            checksum_file_name,
            WINE_GE_RELEASE_TAGS_URL,
        );
        fetch_release_content_test(data);
    }

    #[test]
    fn fetch_release_content_should_download_data_for_proton_ge() {
        let expected_tag = "6.20-GE-1";
        let kind = TagKind::Proton;
        let gzip_file_name = "Proton-6.20-GE-1.tar.gz";
        let checksum_file_name = "Proton-6.20-GE-1.sha512sum";

        let data = FetchReleaseContentTestData::new(
            expected_tag,
            &kind,
            gzip_file_name,
            checksum_file_name,
            PROTON_GE_RELEASE_TAGS_URL,
        );
        fetch_release_content_test(data);
    }
}
