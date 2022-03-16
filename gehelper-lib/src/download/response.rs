use crate::download::{APPLICATION_GZIP, APPLICATION_OCTET_STREAM, APPLICATION_XZ};
use serde::Deserialize;

pub struct DownloadedTar {
    pub compressed_content: Vec<u8>,
    pub file_name: String,
}

impl DownloadedTar {
    pub fn new(compressed_content: Vec<u8>, file_name: String) -> Self {
        DownloadedTar {
            compressed_content,
            file_name,
        }
    }
}

pub struct DownloadedChecksum {
    pub checksum: String,
    pub file_name: String,
}

impl DownloadedChecksum {
    pub fn new(checksum: String, file_name: String) -> Self {
        DownloadedChecksum { checksum, file_name }
    }
}

pub struct DownloadedAssets {
    pub tag: String,
    pub compressed_tar: DownloadedTar,
    pub checksum: Option<DownloadedChecksum>,
}

impl DownloadedAssets {
    pub fn new(tag: String, compressed_tar: DownloadedTar, checksum: Option<DownloadedChecksum>) -> Self {
        DownloadedAssets {
            tag,
            compressed_tar,
            checksum,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GeRelease {
    pub tag_name: String,
    pub assets: Vec<GeAsset>,
}

impl GeRelease {
    pub fn new(tag_name: String, assets: Vec<GeAsset>) -> Self {
        GeRelease { tag_name, assets }
    }

    fn is_checksum_asset(asset: &GeAsset) -> bool {
        asset.content_type.eq(APPLICATION_OCTET_STREAM)
    }

    fn is_tar_asset(asset: &GeAsset) -> bool {
        asset.content_type.eq(APPLICATION_GZIP) || asset.content_type.eq(APPLICATION_XZ)
    }

    pub fn checksum_asset(&self) -> &GeAsset {
        self.assets
            .iter()
            .find(|asset| GeRelease::is_checksum_asset(asset))
            .unwrap()
    }

    pub fn tar_asset(&self) -> &GeAsset {
        self.assets.iter().find(|asset| GeRelease::is_tar_asset(asset)).unwrap()
    }
}

#[derive(Debug, Deserialize)]
pub struct GeAsset {
    pub name: String,
    pub content_type: String,
    pub browser_download_url: String,
}

impl GeAsset {
    pub fn new<S: Into<String>>(name: S, content_type: S, browser_download_url: S) -> Self {
        GeAsset {
            name: name.into(),
            content_type: content_type.into(),
            browser_download_url: browser_download_url.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CompatibilityToolTag {
    pub name: String,
}

#[cfg(test)]
mod ge_release_tests {
    use super::*;
    use crate::download::{APPLICATION_GZIP, APPLICATION_OCTET_STREAM};

    #[test]
    fn get_checksum_asset() {
        let tag = String::from("6.20-GE-1");
        let assets = vec![
            GeAsset::new("Proton-6.20-GE-1.tar.gz", APPLICATION_GZIP, "gzip"),
            GeAsset::new("Proton-6.20-GE-1.sha512sum", APPLICATION_OCTET_STREAM, "octet"),
        ];
        let release = GeRelease::new(tag, assets);

        let checksum_asset = release.checksum_asset();
        assert_eq!(checksum_asset.name, "Proton-6.20-GE-1.sha512sum");
        assert_eq!(checksum_asset.content_type, APPLICATION_OCTET_STREAM);
        assert_eq!(checksum_asset.browser_download_url, "octet");
    }

    #[test]
    fn get_archive_asset() {
        let tag = String::from("6.20-GE-1");
        let assets = vec![
            GeAsset::new("Proton-6.20-GE-1.tar.gz", APPLICATION_GZIP, "gzip"),
            GeAsset::new("Proton-6.20-GE-1.sha512sum", APPLICATION_OCTET_STREAM, "octet"),
        ];
        let release = GeRelease::new(tag, assets);

        let gzip_asset = release.tar_asset();
        assert_eq!(gzip_asset.name, "Proton-6.20-GE-1.tar.gz");
        assert_eq!(gzip_asset.content_type, APPLICATION_GZIP);
        assert_eq!(gzip_asset.browser_download_url, "gzip");
    }
}
