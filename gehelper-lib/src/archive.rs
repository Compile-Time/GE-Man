//! Operations for GloriousEgroll's Proton and Wine release tar archives.
//!
//! This module defines operations that work with compressed tar archives from GloriousEgroll's Proton and Wine
//! releases.
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

use data_encoding::HEXLOWER;
use flate2::read::GzDecoder;
use tar::Archive;
use xz2::read::XzDecoder;

use crate::tag::TagKind;

/// Compares a given checksum to the checksum of the given compressed tar file.
///
/// Generates a checksum from the provided `compressed_tar` file and compares it to the `expected_sum`. This method
/// attempts to split the expected sum by whitespaces before comparing it with the generated checksum from
/// `compressed_tar`. This is done because GE releases provide checksums with the sha512sum tool which also outputs
/// the file name additionally to the generated sum.
///
/// # Examples
///
/// Comparing checksums where `expected_sum` contains no file name.
///
/// ```ignore
/// let file = std::fs::read("filename.txt").unwrap();
/// let expected = "<checksum>";
/// let is_matching = archive::checksums_match(file, expected);
/// ```
///
/// Comparing checksums where `expected_sum` contains a file name.
/// ```ignore
/// let file = std::fs::read("filename.txt").unwrap();
/// let expected = "<checksum> filename.txt";
/// let is_matching = archive::checksums_match(file, expected);
/// ```
pub fn checksums_match(compressed_tar: &[u8], expected_sum: &[u8]) -> bool {
    let expected_sum = String::from_utf8_lossy(expected_sum)
        .split_whitespace()
        .next()
        .map(String::from)
        .unwrap();

    let digest = ring::digest::digest(&ring::digest::SHA512, compressed_tar);
    let sum = HEXLOWER.encode(digest.as_ref());

    expected_sum.eq(&sum)
}

/// Extracts a compressed archive for a tag kind into the given `extract_destination` and returns a `PathBuf` to the
/// extracted location.
///
/// This method first decompresses `compressed_tar` with `flate2` or `xz2` and then extracts it with `tar`. The
/// decompression algorithm to use is decided by the provided `kind`. If `kind` is a `TagKind::Proton`, gzip
/// decompression is used. If `kind` is a `TagKind::Wine {..}` xz decompression is used.
///
/// The difference in decompression is due to the fact that Proton GE releases use gzip compression and Wine GE
/// releases use xz compressions.
///
/// # Examples
///
/// Extracting a Proton GE release (Proton GE releases use GZIP for compression).
///
/// ```ignore
/// # use gcm_lib::tag::TagKind;
///
/// let archive = std::fs::read("archive.tar.gz").unwrap();
/// let kind = TagKind::Proton;
/// let destination = PathBuf::from("/path/to/destination");
/// let extracted_location = archive::extract_tar(&kind, archive, destination);
/// ```
///
/// Extracting a Wine GE release (Wine GE releases use XZ for compression).
///
/// ```ignore
/// # use gcm_lib::tag::TagKind;
///
/// let archive = std::fs::read("archive.tar.xz").unwrap();
/// let kind = TagKind::wine();
/// let destination = PathBuf::from("/path/to/destination");
/// let extracted_location = archive::extract_tar(&kind, archive, destination);
/// ```
///
/// # Errors
///
/// This method returns a `std::io::Error` when:
///
/// * any standard library IO error is encountered
/// * the `flat2` crate returns an error during decompression
/// * the `xz2` crate returns an error during decompression
/// * the `tar` crate returns an error during extraction
pub fn extract_compressed_tar(
    kind: &TagKind,
    compressed_tar: impl Read,
    extract_destination: &Path,
) -> Result<PathBuf, io::Error> {
    let extracted_dst = match kind {
        TagKind::Proton => {
            let decoder = GzDecoder::new(compressed_tar);
            extract_tar(decoder, extract_destination)?
        }
        TagKind::Wine { .. } => {
            let decoder = XzDecoder::new(compressed_tar);
            extract_tar(decoder, extract_destination)?
        }
    };

    Ok(extracted_dst)
}

fn extract_tar(decoder: impl Read, extract_destination: &Path) -> Result<PathBuf, std::io::Error> {
    let mut archive = Archive::new(decoder);

    let mut iter = archive.entries()?;
    let first_entry = &mut iter.next().unwrap()?;

    let dir_name = first_entry.path().unwrap().into_owned();

    first_entry.unpack_in(extract_destination)?;
    for entry in iter {
        let mut entry = entry?;
        entry.unpack_in(extract_destination)?;
    }

    Ok(extract_destination.join(dir_name))
}

#[cfg(test)]
mod checksum_tests {
    use std::fs;

    use super::*;

    #[test]
    fn check_if_equal_checksums_where_expected_sum_contains_file_name_match() {
        let tar = fs::read("tests/resources/assets/test.tar.gz").unwrap();
        let checksum =
            "f2ad7b96bb24ae5fa71398127927b22c8c11eba2d3578df5a47e6ad5b5a06b0c4c66d25cf53bed0d9ed0864b76aea73794cc4be7f01249f43b796f70d068f972  test.tar.gz";

        let is_equal = checksums_match(&tar, checksum.as_bytes());
        assert!(is_equal);
    }

    #[test]
    fn check_if_equal_checksums_match() {
        let tar = fs::read("tests/resources/assets/test.tar.gz").unwrap();
        let checksum =
            "f2ad7b96bb24ae5fa71398127927b22c8c11eba2d3578df5a47e6ad5b5a06b0c4c66d25cf53bed0d9ed0864b76aea73794cc4be7f01249f43b796f70d068f972";

        let is_equal = checksums_match(&tar, checksum.as_bytes());
        assert!(is_equal);
    }

    #[test]
    fn check_if_not_equal_checksums_do_not_match() {
        let tar = fs::read("tests/resources/assets/test.tar.gz").unwrap();
        let checksum = "unreal-checksum";

        let is_equal = checksums_match(&tar, checksum.as_bytes());
        assert!(!is_equal);
    }
}

#[cfg(test)]
mod extraction_tests {
    use std::fs::File;
    use std::io;

    use assert_fs::assert::PathAssert;
    use assert_fs::fixture::PathChild;
    use assert_fs::TempDir;
    use test_case::test_case;

    use super::*;

    #[test]
    fn extract_proton_ge_release_with_correct_tag_kind() {
        let tmp_dir = TempDir::new().unwrap();

        let archive = File::open("tests/resources/assets/test.tar.gz").unwrap();
        let kind = TagKind::Proton;

        let dst = super::extract_compressed_tar(&kind, archive, tmp_dir.path()).unwrap();

        assert_eq!(dst, tmp_dir.join("test"));
        tmp_dir
            .child(&dst)
            .assert(predicates::path::exists())
            .child(dst.join("hello-world.txt"))
            .assert(predicates::path::exists());
        tmp_dir
            .child(&dst)
            .child(dst.join("nested"))
            .assert(predicates::path::exists())
            .child(dst.join("nested/nested.txt"));
        tmp_dir
            .child(&dst)
            .child(dst.join("other-file.txt"))
            .assert(predicates::path::exists());

        tmp_dir.close().unwrap();
    }

    #[test]
    fn extract_proton_ge_release_with_wrong_tag_kind() {
        let tmp_dir = TempDir::new().unwrap();

        let archive = File::open("tests/resources/assets/test.tar.gz").unwrap();
        let kind = TagKind::wine();

        let result = super::extract_compressed_tar(&kind, archive, tmp_dir.path());
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);

        assert!(tmp_dir.iter().size_hint().eq(&(0, None)));
        tmp_dir.close().unwrap();
    }

    #[test_case(TagKind::wine(); "Running with Wine kind")]
    #[test_case(TagKind::lol(); "Running with LoL kind")]
    fn extract_wine_ge_release_with_correct_tag_kind(kind: TagKind) {
        let tmp_dir = TempDir::new().unwrap();

        let archive = File::open("tests/resources/assets/test.tar.xz").unwrap();

        let dst = super::extract_compressed_tar(&kind, archive, tmp_dir.path()).unwrap();

        assert_eq!(dst, tmp_dir.join("test"));
        tmp_dir
            .child(&dst)
            .assert(predicates::path::exists())
            .child(dst.join("hello-world.txt"))
            .assert(predicates::path::exists());
        tmp_dir
            .child(&dst)
            .child(dst.join("nested"))
            .assert(predicates::path::exists())
            .child(dst.join("nested/nested.txt"));
        tmp_dir
            .child(&dst)
            .child(dst.join("other-file.txt"))
            .assert(predicates::path::exists());

        tmp_dir.close().unwrap();
    }

    #[test]
    fn extract_wine_ge_release_with_wrong_tag_kind() {
        let tmp_dir = TempDir::new().unwrap();
        let kind = TagKind::Proton;

        let archive = File::open("tests/resources/assets/test.tar.xz").unwrap();
        let result = super::extract_compressed_tar(&kind, archive, tmp_dir.path());
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

        assert!(tmp_dir.iter().size_hint().eq(&(0, None)));
        tmp_dir.close().unwrap();
    }
}
