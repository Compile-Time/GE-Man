use std::io;
use std::io::Write;

use anyhow::{anyhow, bail, Context};
use ge_man_lib::archive;
use ge_man_lib::download::response::DownloadedAssets;
use ge_man_lib::download::{DownloadRequest, GeDownload};
use ge_man_lib::error::GithubError;
use ge_man_lib::tag::TagKind;
use log::debug;

use crate::args::{
    AddCommandInput, ApplyCommandInput, CheckCommandInput, CopyUserSettingsArgs, ForgetArgs, GivenVersion,
    ListCommandInput, MigrationArgs, RemoveCommandInput,
};
use crate::compat_tool_app::ApplicationConfig;
use crate::data::{ManagedVersion, ManagedVersions};
use crate::filesystem::FilesystemManager;
use crate::message;
use crate::path::{overrule, PathConfiguration};
use crate::progress::{DownloadProgressTracker, ExtractionProgressTracker};
use crate::version::{Version, Versioned};

#[derive(Debug)]
pub struct NewAndManagedVersions {
    pub version: ManagedVersion,
    pub managed_versions: ManagedVersions,
}

impl NewAndManagedVersions {
    pub fn new(version: ManagedVersion, managed_versions: ManagedVersions) -> Self {
        Self {
            version,
            managed_versions,
        }
    }
}

#[derive(Debug)]
pub struct RemovedAndManagedVersions {
    pub version: ManagedVersion,
    pub managed_versions: ManagedVersions,
}

impl RemovedAndManagedVersions {
    pub fn new(version: ManagedVersion, managed_versions: ManagedVersions) -> Self {
        Self {
            version,
            managed_versions,
        }
    }
}

/// Handles user interaction and user feedback. This struct basically ties everything together to provide the
/// functionality of each terminal command.
pub struct CommandHandler<'a> {
    ge_downloader: &'a dyn GeDownload,
    fs_mng: &'a dyn FilesystemManager,
    // TODO: Remove path_cfg after ui module refactoring is complete
    path_cfg: &'a dyn PathConfiguration,
}

impl<'a> CommandHandler<'a> {
    pub fn new(
        ge_downloader: &'a dyn GeDownload,
        fs_mng: &'a dyn FilesystemManager,
        path_cfg: &'a dyn PathConfiguration,
    ) -> Self {
        CommandHandler {
            ge_downloader,
            fs_mng,
            path_cfg,
        }
    }

    fn list_line(&self, version: &ManagedVersion, compat_tool_dir_name: Option<&String>, app_name: &str) -> String {
        if let Some(dir) = compat_tool_dir_name {
            if dir.eq(version.directory_name()) {
                return format!("{} - In use by {}", version.tag(), app_name);
            }
        }

        version.tag().value().clone()
    }

    fn read_managed_versions(&self) -> anyhow::Result<ManagedVersions> {
        let path = self.path_cfg.managed_versions_config(overrule::xdg_data_home());
        debug!("Reading managed versions from the following file: {}", path.display());
        ManagedVersions::from_file(&path)
            .context(format!("Could not read managed_versions.json from {}", path.display()))
    }

    fn write_managed_versions(&self, managed_versions: ManagedVersions) -> anyhow::Result<()> {
        let path = self.path_cfg.managed_versions_config(overrule::xdg_data_home());
        debug!("Writing managed versions from the following file: {}", path.display());
        managed_versions
            .write_to_file(&path)
            .context(format!("Could not write managed_versions.json to {}", path.display()))
    }

    pub fn list_versions(
        &self,
        stdout: &mut impl Write,
        stderr: &mut impl Write,
        list_command_input: ListCommandInput,
    ) {
        let ListCommandInput {
            tag_kind,
            newest,
            in_use_directory_name,
            managed_versions,
            application_name,
        } = list_command_input;

        if in_use_directory_name.is_none() {
            writeln!(
                stderr,
                r###"{} config not found - "In use" information can not be provided"###,
                application_name
            )
            .unwrap();
        }

        let mut managed_versions = if newest {
            managed_versions.latest_versions()
        } else {
            managed_versions.vec_ref().clone()
        };

        if !managed_versions.is_empty() {
            writeln!(stdout, "{}:", tag_kind.compatibility_tool_name()).unwrap();

            managed_versions.sort_unstable_by(|a, b| a.tag().cmp_semver(b.tag()).reverse());
            for version in &managed_versions {
                let line = self.list_line(version, in_use_directory_name.as_ref(), &application_name);
                writeln!(stdout, "* {}", line).unwrap();
            }
            writeln!(stdout).unwrap();
        } else {
            writeln!(stdout, "No versions installed").unwrap();
        }
    }

    pub fn add(&self, stdout: &mut impl Write, input: AddCommandInput) -> anyhow::Result<NewAndManagedVersions> {
        let AddCommandInput {
            version,
            skip_checksum,
            mut managed_versions,
        } = input;

        let version = match version {
            GivenVersion::Explicit { version: versioned } => Version::from(versioned),
            GivenVersion::Latest { kind } => match self.ge_downloader.fetch_release(None, kind) {
                Ok(release) => Version::new(release.tag_name, kind),
                Err(err) => {
                    return Err(anyhow!(err).context(r#"Could not get latest tag for tagless "add" operation."#))
                }
            },
        };

        if let Some(existing_version) = managed_versions.find_version(&version) {
            writeln!(stdout, "Version {} is already managed", existing_version)?;
            return Ok(NewAndManagedVersions::new(existing_version, managed_versions));
        }

        let download_tracker = Box::new(DownloadProgressTracker::default());
        let request = DownloadRequest::new(
            Some(version.tag().to_string()),
            *version.kind(),
            download_tracker,
            skip_checksum,
        );

        let assets = match self.ge_downloader.download_release_assets(request) {
            Ok(assets) => assets,
            Err(err) => {
                let res = if let GithubError::ReleaseHasNoAssets { tag, kind } = err {
                    let err = GithubError::ReleaseHasNoAssets { tag: tag.clone(), kind };

                    anyhow!(err).context(format!(
                        "The given release has no assets for {} {}. It might be possible that the \
                        release assets have been removed due to fixes in a newer version.",
                        tag, kind
                    ))
                } else {
                    anyhow!(err).context("Could not fetch release assets from Github")
                };

                bail!(res);
            }
        };

        let DownloadedAssets {
            compressed_archive: compressed_tar,
            checksum,
            ..
        } = assets;

        if skip_checksum {
            writeln!(stdout, "Skipping checksum comparison").unwrap();
        } else {
            write!(stdout, "Performing checksum comparison").unwrap();
            let checksum = checksum.unwrap();

            let result = archive::checksums_match(&compressed_tar.compressed_content, checksum.checksum.as_bytes());

            if !result {
                bail!("Checksum comparison failed: Checksum generated from downloaded archive does not match downloaded expected checksum");
            } else {
                writeln!(stdout, ": Checksums match").unwrap();
            }
        }

        let extraction_tracker = ExtractionProgressTracker::new(compressed_tar.compressed_content.len() as u64);
        let compressed_tar_reader = extraction_tracker
            .inner()
            .wrap_read(std::io::Cursor::new(compressed_tar.compressed_content));

        let version = self
            .fs_mng
            .setup_version(version, Box::new(compressed_tar_reader))
            .context("Could not add version")?;
        extraction_tracker.finish();

        let new_version = managed_versions.add(version)?;
        Ok(NewAndManagedVersions::new(new_version, managed_versions))
    }

    pub fn remove(&self, input: RemoveCommandInput) -> anyhow::Result<RemovedAndManagedVersions> {
        let RemoveCommandInput {
            mut managed_versions,
            version_to_remove,
            app_config_path,
        } = input;

        let app_config = ApplicationConfig::create_copy(version_to_remove.kind(), &app_config_path);
        if let Err(err) = &app_config {
            if let Some(err) = err.downcast_ref::<io::Error>() {
                // Ignore file not found errors.
                if err.raw_os_error().unwrap() != 2 {
                    bail!("{}", err)
                }
            } else {
                bail!("{}", err);
            }
        }

        let app_config = app_config.unwrap();
        if app_config.check_if_version_is_in_use(&version_to_remove) {
            bail!(
                "{} version is in use. Select a different version to make removal possible",
                version_to_remove.kind()
            );
        }

        self.fs_mng.remove_version(&version_to_remove).unwrap();
        managed_versions.remove(&version_to_remove).unwrap();
        Ok(RemovedAndManagedVersions::new(version_to_remove, managed_versions))
    }

    pub fn check(&self, stdout: &mut impl Write, stderr: &mut impl Write, input: CheckCommandInput) {
        match input.kind {
            Some(kind) => match self.ge_downloader.fetch_release(None, kind) {
                Ok(release) => {
                    writeln!(
                        stdout,
                        "The latest version of {} is \"{}\"",
                        kind.compatibility_tool_name(),
                        release.tag_name
                    )
                    .unwrap();
                }
                Err(err) => {
                    writeln!(stderr, "Could not fetch latest release from Github: {}", err).unwrap();
                }
            },
            None => {
                let tag_kinds = vec![TagKind::Proton, TagKind::wine(), TagKind::lol()];

                writeln!(stdout, "These are the latest releases.").unwrap();
                writeln!(stdout).unwrap();
                for kind in tag_kinds {
                    let release = self.ge_downloader.fetch_release(None, kind);
                    match release {
                        Ok(release) => {
                            writeln!(stdout, "{}: {}", kind.compatibility_tool_name(), release.tag_name).unwrap()
                        }
                        Err(err) => writeln!(
                            stderr,
                            "{} - Could not fetch release information from GitHub: {}",
                            kind.compatibility_tool_name(),
                            err
                        )
                        .unwrap(),
                    }
                }
            }
        }
    }

    pub fn migrate(&self, stdout: &mut impl Write, args: MigrationArgs) -> anyhow::Result<()> {
        let version = args.tag_arg.version();
        let mut managed_versions = self.read_managed_versions()?;

        if managed_versions.find_version(&version).is_some() {
            bail!("Given version to migrate already exists as a managed version");
        }

        let source_path = &args.source_path;
        let version = self
            .fs_mng
            .migrate_folder(version, source_path)
            .context("Could not migrate directory")?;
        let version = managed_versions.add(version)?;

        self.write_managed_versions(managed_versions)?;
        writeln!(stdout, "Successfully migrated directory as {}", version).unwrap();
        Ok(())
    }

    pub fn apply(&self, stdout: &mut impl Write, input: ApplyCommandInput) -> anyhow::Result<()> {
        let ApplyCommandInput {
            version,
            managed_versions,
        } = input;

        let version = match version {
            GivenVersion::Explicit { version } => match managed_versions.find_version(version.as_ref()) {
                Some(v) => v,
                None => bail!("Given version is not managed"),
            },
            GivenVersion::Latest { kind } => match managed_versions.find_latest_by_kind(&kind) {
                Some(v) => v,
                None => bail!("No managed versions exist"),
            },
        };

        writeln!(stdout, "{}", message::apply::modifying_config(&version)).unwrap();

        self.fs_mng
            .apply_to_app_config(&version)
            .context("Could not modify app config")?;

        writeln!(stdout, "{}", message::apply::modify_config_success(version.kind())).unwrap();

        Ok(())
    }

    pub fn copy_user_settings(&self, stdout: &mut impl Write, args: CopyUserSettingsArgs) -> anyhow::Result<()> {
        let managed_versions = self.read_managed_versions()?;
        let src_version = Version::new(args.src_tag, TagKind::Proton);
        let dst_version = Version::new(args.dst_tag, TagKind::Proton);

        let src_version = match managed_versions.find_version(&src_version) {
            Some(v) => v,
            None => bail!("Given source Proton version does not exist"),
        };
        let dst_version = match managed_versions.find_version(&dst_version) {
            Some(v) => v,
            None => bail!("Given destination Proton version does not exist"),
        };

        self.fs_mng.copy_user_settings(&src_version, &dst_version)?;

        writeln!(
            stdout,
            "Copied user_settings.py from {} to {}",
            src_version, dst_version
        )
        .unwrap();
        Ok(())
    }

    pub fn forget(&self, stdout: &mut impl Write, args: ForgetArgs) -> anyhow::Result<()> {
        let version = args.tag_arg.version();
        let mut managed_versions = self.read_managed_versions()?;
        if managed_versions.remove(&version).is_none() {
            bail!("Failed to forget version: Version is not managed");
        }

        self.write_managed_versions(managed_versions)?;
        writeln!(stdout, "{} is now not managed by GE Helper", version).unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::{fs, io};

    use anyhow::bail;
    use assert_fs::TempDir;
    use ge_man_lib::download::response::{DownloadedArchive, DownloadedChecksum, GeRelease};
    use ge_man_lib::tag::Tag;
    use mockall::mock;

    use crate::args::TagArg;
    use crate::filesystem::MockFilesystemManager;
    use crate::path::MockPathConfiguration;

    use super::*;

    mock! {
        Downloader {}
        impl GeDownload for Downloader {
            fn fetch_release(&self, tag: Option<String>, kind: TagKind) -> Result<GeRelease, GithubError>;
            fn download_release_assets(&self, request: DownloadRequest) -> Result<DownloadedAssets, GithubError>;
        }
    }

    struct AssertLines {
        latest_line: String,
        pub lines: Vec<String>,
    }

    impl AssertLines {
        pub fn new() -> Self {
            AssertLines {
                latest_line: String::new(),
                lines: Vec::new(),
            }
        }

        pub fn assert_line(&self, line: usize, expected: &str) {
            let line = self.lines.get(line).unwrap();
            assert!(line.contains("\n"));
            assert_eq!(line.trim(), expected);
        }

        pub fn assert_count(&self, size: usize) {
            assert_eq!(self.lines.len(), size)
        }

        pub fn assert_empty(&self) {
            assert!(self.lines.is_empty())
        }
    }

    impl io::Write for AssertLines {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let len = buf.len();
            let str = String::from_utf8_lossy(buf);
            self.latest_line.push_str(&str);

            if self.latest_line.contains("\n") {
                self.lines.push(self.latest_line.clone());
                self.latest_line = String::new();
            }

            Ok(len)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn proton_6_20_1() -> ManagedVersion {
        ManagedVersion::from(Version::proton("6.20-GE-1"))
    }

    fn setup_managed_versions(json_path: &Path, versions: Vec<ManagedVersion>) {
        fs::create_dir_all(json_path.parent().unwrap()).unwrap();
        let managed_versions = ManagedVersions::new(versions);
        managed_versions.write_to_file(json_path).unwrap();
    }

    #[test]
    fn forget_should_print_success_message() {
        let args = ForgetArgs::new(TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton));
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![proton_6_20_1()]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .times(2)
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        command_handler.forget(&mut stdout, args).unwrap();

        stdout.assert_line(0, "6.20-GE-1 (Proton) is now not managed by GE Helper");
    }

    #[test]
    fn forget_should_print_error_message() {
        let args = ForgetArgs::new(TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton));
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = command_handler.forget(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Failed to forget version: Version is not managed");
        stdout.assert_empty();
    }

    fn list_test_template(stdout: &mut AssertLines, stderr: &mut AssertLines, input: ListCommandInput) {
        // A lot of these variables will disappear once more refactorings have been completed.
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let path_cfg = MockPathConfiguration::new();

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);
        command_handler.list_versions(stdout, stderr, input);
    }

    #[test]
    fn list_steam_newest_version() {
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("GE-Proton7-22", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
        ]);
        let input = ListCommandInput::new(TagKind::Proton, true, None, managed_versions, String::from("Steam"));

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        list_test_template(&mut stdout, &mut stderr, input);

        stdout.assert_count(3);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* GE-Proton7-22");
        stdout.assert_line(2, "");
    }

    #[test]
    fn list_newest_steam_version_with_in_use_info() {
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("GE-Proton7-22", TagKind::Proton, "GE-Proton7-22"),
            ManagedVersion::new("6.21-GE-1", TagKind::Proton, "6.21-GE-1"),
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6.20-GE-1"),
        ]);
        let input = ListCommandInput::new(
            TagKind::Proton,
            true,
            Some(String::from("GE-Proton7-22")),
            managed_versions,
            String::from("Steam"),
        );

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        list_test_template(&mut stdout, &mut stderr, input);

        stdout.assert_count(3);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* GE-Proton7-22 - In use by Steam");
        stdout.assert_line(2, "");
    }

    #[test]
    fn list_steam_versions() {
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("GE-Proton7-22", TagKind::Proton, "GE-Proton7-22"),
            ManagedVersion::new("6.21-GE-1", TagKind::Proton, "6.21-GE-1"),
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6.20-GE-1"),
        ]);
        let input = ListCommandInput::new(TagKind::Proton, false, None, managed_versions, String::from("Steam"));

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        list_test_template(&mut stdout, &mut stderr, input);

        stdout.assert_count(5);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* GE-Proton7-22");
        stdout.assert_line(2, "* 6.21-GE-1");
        stdout.assert_line(3, "* 6.20-GE-1");
        stdout.assert_line(4, "");
    }

    #[test]
    fn list_steam_versions_with_in_use_info() {
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("GE-Proton7-22", TagKind::Proton, "GE-Proton7-22"),
            ManagedVersion::new("6.21-GE-1", TagKind::Proton, "6.21-GE-1"),
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6.20-GE-1"),
        ]);
        let input = ListCommandInput::new(
            TagKind::Proton,
            false,
            Some(String::from("GE-Proton7-22")),
            managed_versions,
            String::from("Steam"),
        );

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        list_test_template(&mut stdout, &mut stderr, input);

        stdout.assert_count(5);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* GE-Proton7-22 - In use by Steam");
        stdout.assert_line(2, "* 6.21-GE-1");
        stdout.assert_line(3, "* 6.20-GE-1");
        stdout.assert_line(4, "");
    }

    #[test]
    fn add_successful_output() {
        let mut ge_downloader = MockDownloader::new();
        ge_downloader.expect_download_release_assets().once().returning(|_| {
            Ok(DownloadedAssets {
                tag: "".to_string(),
                compressed_archive: DownloadedArchive {
                    compressed_content: vec![],
                    file_name: "".to_string(),
                },
                checksum: None,
            })
        });

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_setup_version()
            .once()
            .returning(|_, _| Ok(ManagedVersion::new("6.20-GE-1", TagKind::Proton, "")));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let path_cfg = MockPathConfiguration::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let version = Version::new("6.20-GE-1", TagKind::Proton);
        let managed_versions = ManagedVersions::new(Vec::new());
        let input = AddCommandInput::new(
            GivenVersion::Explicit {
                version: Box::new(version),
            },
            true,
            managed_versions,
        );

        let mut stdout = AssertLines::new();
        command_handler.add(&mut stdout, input).unwrap();

        stdout.assert_line(0, "Skipping checksum comparison");
    }

    #[test]
    fn add_with_checksum_comparison_successful_output() {
        let mut ge_downloader = MockDownloader::new();
        ge_downloader.expect_download_release_assets().once().returning(|_| {
            let tar = fs::read("test_resources/assets/Proton-6.20-GE-1.tar.gz").unwrap();
            let checksum = fs::read_to_string("test_resources/assets/Proton-6.20-GE-1.sha512sum").unwrap();

            Ok(DownloadedAssets {
                tag: "6.20-GE-1".to_string(),
                compressed_archive: DownloadedArchive {
                    compressed_content: tar,
                    file_name: "Proton-6.20-GE-1.tar.gz".to_string(),
                },
                checksum: Some(DownloadedChecksum {
                    checksum,
                    file_name: "Proton-6.20-GE-1.sha512sum".to_string(),
                }),
            })
        });

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_setup_version()
            .once()
            .returning(|_, _| Ok(ManagedVersion::new("6.20-GE-1", TagKind::Proton, "")));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let path_cfg = MockPathConfiguration::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let version = Version::new("6.20-GE-1", TagKind::Proton);
        let managed_versions = ManagedVersions::new(Vec::new());
        let input = AddCommandInput::new(
            GivenVersion::Explicit {
                version: Box::new(version),
            },
            false,
            managed_versions,
        );

        let mut stdout = AssertLines::new();
        command_handler.add(&mut stdout, input).unwrap();

        stdout.assert_line(0, "Performing checksum comparison: Checksums match");
    }

    #[test]
    fn add_specific_version_which_is_already_managed_again_expect_message_about_already_being_managed() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_setup_version().never();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "")]);

        let path_cfg = MockPathConfiguration::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let version = Version::new("6.20-GE-1", TagKind::Proton);
        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1")]);
        let input = AddCommandInput::new(
            GivenVersion::Explicit {
                version: Box::new(version),
            },
            false,
            managed_versions,
        );

        let mut stdout = AssertLines::new();
        let result = command_handler.add(&mut stdout, input);

        assert!(result.is_ok());
        stdout.assert_line(0, "Version 6.20-GE-1 (Proton) is already managed");
    }

    #[test]
    fn add_latest_version_which_is_already_managed_again_expect_message_about_already_being_managed() {
        let tag = "6.20-GE-1";
        let managed_versions = ManagedVersions::new(vec![ManagedVersion::new(tag, TagKind::Proton, "")]);

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_setup_version().never();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, managed_versions.vec_ref().clone());

        let mut ge_downloader = MockDownloader::new();
        ge_downloader
            .expect_fetch_release()
            .once()
            .returning(move |_, _| Ok(GeRelease::new(String::from(tag), Vec::new())));

        let path_cfg = MockPathConfiguration::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let input = AddCommandInput::new(
            GivenVersion::Latest { kind: TagKind::Proton },
            false,
            managed_versions.clone(),
        );

        let mut stdout = AssertLines::new();
        let result = command_handler.add(&mut stdout, input);

        assert!(result.is_ok());
        stdout.assert_line(0, "Version 6.20-GE-1 (Proton) is already managed");
    }

    #[test]
    fn remove_existing_version() {
        let ge_downloader = MockDownloader::new();
        let path_cfg = MockPathConfiguration::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().once().returning(|_| Ok(()));

        let steam_config_path = PathBuf::from("test_resources/assets/config.vdf");
        let version_to_remove = ManagedVersion::new("6.20-GE-1", TagKind::Proton, "");
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1"),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, "6-21-GE-2"),
        ]);
        let input = RemoveCommandInput::new(managed_versions, version_to_remove.clone(), steam_config_path);

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);
        let removed_and_managed_versions = command_handler.remove(input).unwrap();
        assert_eq!(removed_and_managed_versions.managed_versions.vec_ref().len(), 1);
        assert_eq!(
            removed_and_managed_versions.managed_versions,
            ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-2", TagKind::Proton, "")])
        );
        assert_eq!(removed_and_managed_versions.version, version_to_remove)
    }

    #[test]
    fn remove_version_used_by_app_config() {
        let ge_downloader = MockDownloader::new();
        let path_cfg = MockPathConfiguration::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().never();

        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6.20-GE-1"),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, "Proton-6.21-GE-2"),
        ]);

        let steam_config_path = PathBuf::from("test_resources/assets/config.vdf");
        let version_to_remove = ManagedVersion::new("6.21-GE-2", TagKind::Proton, "Proton-6.21-GE-2");
        let input = RemoveCommandInput::new(managed_versions, version_to_remove, steam_config_path);

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);
        let result = command_handler.remove(input);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "PROTON version is in use. Select a different version to make removal possible"
        );
    }

    #[test]
    fn check_with_successful_requests() {
        let args = CheckCommandInput::new(None);

        let mut ge_downloader = MockDownloader::new();
        ge_downloader
            .expect_fetch_release()
            .once()
            .withf(|tag, kind| tag.is_none() && kind.eq(&TagKind::Proton))
            .returning(|_, _| Ok(GeRelease::new(String::from("6.20-GE-1"), vec![])));
        ge_downloader
            .expect_fetch_release()
            .once()
            .withf(|tag, kind| tag.is_none() && kind.eq(&TagKind::wine()))
            .returning(|_, _| Ok(GeRelease::new(String::from("6.20-GE-1"), vec![])));
        ge_downloader
            .expect_fetch_release()
            .once()
            .withf(|tag, kind| tag.is_none() && kind.eq(&TagKind::lol()))
            .returning(|_, _| Ok(GeRelease::new(String::from("6.16-GE-3-LoL"), vec![])));

        let path_cfg = MockPathConfiguration::new();
        let fs_mng = MockFilesystemManager::new();

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        command_handler.check(&mut stdout, &mut stderr, args);

        stdout.assert_line(0, "These are the latest releases.");
        stdout.assert_line(1, "");
        stdout.assert_line(2, "Proton GE: 6.20-GE-1");
        stdout.assert_line(3, "Wine GE: 6.20-GE-1");
        stdout.assert_line(4, "Wine GE (LoL): 6.16-GE-3-LoL");
    }

    #[test]
    fn check_with_only_errors() {
        let args = CheckCommandInput::new(None);

        let mut ge_downloader = MockDownloader::new();
        ge_downloader
            .expect_fetch_release()
            .once()
            .withf(|tag, kind| tag.is_none() && kind.eq(&TagKind::Proton))
            .returning(|_, _| Err(GithubError::NoTags));
        ge_downloader
            .expect_fetch_release()
            .once()
            .withf(|tag, kind| tag.is_none() && kind.eq(&TagKind::wine()))
            .returning(|_, _| Err(GithubError::NoTags));
        ge_downloader
            .expect_fetch_release()
            .once()
            .withf(|tag, kind| tag.is_none() && kind.eq(&TagKind::lol()))
            .returning(|_, _| Err(GithubError::NoTags));

        let path_cfg = MockPathConfiguration::new();
        let fs_mng = MockFilesystemManager::new();

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        command_handler.check(&mut stdout, &mut stderr, args);

        stdout.assert_line(0, "These are the latest releases.");
        stderr.assert_line(
            0,
            "Proton GE - Could not fetch release information from GitHub: No tags could be found",
        );
        stderr.assert_line(
            1,
            "Wine GE - Could not fetch release information from GitHub: No tags could be found",
        );
        stderr.assert_line(
            2,
            "Wine GE (LoL) - Could not fetch release information from GitHub: No tags could be found",
        );
    }

    #[test]
    fn migrate_already_managed_version() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = MigrationArgs::new(tag_arg, "invalid-path");

        let ge_downloader = MockDownloader::new();
        let fs_mng = MockFilesystemManager::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![ManagedVersion::new(
                Tag::from("6.20-GE-1"),
                TagKind::Proton,
                "Proton-6.20-GE-1",
            )],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = command_handler.migrate(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Given version to migrate already exists as a managed version"
        );
        stdout.assert_empty();
    }

    #[test]
    fn migrate_not_present_version() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = MigrationArgs::new(tag_arg, "migration-source");

        let ge_downloader = MockDownloader::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_migrate_folder()
            .once()
            .returning(|_, _| Ok(ManagedVersion::new("6.20-GE-1", TagKind::Proton, "Proton-6.20-GE-1")));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .times(2)
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        command_handler.migrate(&mut stdout, args).unwrap();
        stdout.assert_line(0, "Successfully migrated directory as 6.20-GE-1 (Proton)");
    }

    #[test]
    fn migrate_fails_due_to_filesystem_error() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = MigrationArgs::new(tag_arg, "migration-source");

        let ge_downloader = MockDownloader::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_migrate_folder()
            .once()
            .returning(|_, _| bail!("Mocked error"));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = command_handler.migrate(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Could not migrate directory");
        stdout.assert_empty();
    }

    #[test]
    fn apply_to_app_config_for_non_existent_version() {
        let ge_downloader = MockDownloader::new();
        let fs_mng = MockFilesystemManager::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let path_cfg = MockPathConfiguration::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let version = Version::new("6.20-GE-1", TagKind::Proton);
        let managed_versions = ManagedVersions::new(Vec::new());
        let input = ApplyCommandInput::new(
            GivenVersion::Explicit {
                version: Box::new(version),
            },
            managed_versions,
        );

        let mut stdout = AssertLines::new();
        let result = command_handler.apply(&mut stdout, input);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Given version is not managed");
        stdout.assert_empty();
    }

    #[test]
    fn apply_to_app_config_for_latest_version() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_apply_to_app_config().once().returning(|_| Ok(()));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "Proton-6.20-GE-1")],
        );

        let path_cfg = MockPathConfiguration::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1")]);
        let input = ApplyCommandInput::new(GivenVersion::Latest { kind: TagKind::Proton }, managed_versions);

        let mut stdout = AssertLines::new();
        command_handler.apply(&mut stdout, input).unwrap();

        stdout.assert_line(
            0,
            &message::apply::modifying_config(&Version::new("6.20-GE-1", TagKind::Proton)),
        );
        stdout.assert_line(1, message::apply::modify_config_success(&TagKind::Proton));
    }

    #[test]
    fn apply_to_app_config_for_existent_version() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_apply_to_app_config().once().returning(|_| Ok(()));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "Proton-6.20-GE-1")],
        );

        let path_cfg = MockPathConfiguration::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let version = Version::new("6.20-GE-1", TagKind::Proton);
        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1")]);
        let input = ApplyCommandInput::new(
            GivenVersion::Explicit {
                version: Box::new(version),
            },
            managed_versions,
        );

        let mut stdout = AssertLines::new();
        command_handler.apply(&mut stdout, input).unwrap();

        stdout.assert_line(
            0,
            &message::apply::modifying_config(&Version::new("6.20-GE-1", TagKind::Proton)),
        );
        stdout.assert_line(1, message::apply::modify_config_success(&TagKind::Proton));
    }

    #[test]
    fn apply_to_app_config_fails_with_an_error() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_apply_to_app_config()
            .once()
            .returning(|_| bail!("Mocked error"));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "Proton-6.20-GE-1")],
        );

        let path_cfg = MockPathConfiguration::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let version = Version::new("6.20-GE-1", TagKind::Proton);
        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1")]);
        let input = ApplyCommandInput::new(
            GivenVersion::Explicit {
                version: Box::new(version),
            },
            managed_versions,
        );

        let mut stdout = AssertLines::new();
        let result = command_handler.apply(&mut stdout, input);
        assert!(result.is_err());

        stdout.assert_line(0, "Modifying Steam configuration to use 6.20-GE-1");
    }

    #[test]
    fn copy_user_settings_where_source_tag_does_not_exist() {
        let args = CopyUserSettingsArgs::new("6.20-GE-1", "6.21-GE-1");
        let ge_downloader = MockDownloader::new();
        let fs_mng = MockFilesystemManager::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![ManagedVersion::new("6.21-GE-1", TagKind::Proton, "")]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = command_handler.copy_user_settings(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Given source Proton version does not exist");
        stdout.assert_empty();
    }

    #[test]
    fn copy_user_settings_where_destination_tag_does_not_exist() {
        let args = CopyUserSettingsArgs::new("6.20-GE-1", "6.21-GE-1");
        let ge_downloader = MockDownloader::new();
        let fs_mng = MockFilesystemManager::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "")]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = command_handler.copy_user_settings(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Given destination Proton version does not exist");
        stdout.assert_empty();
    }

    #[test]
    fn copy_user_settings_for_present_versions() {
        let args = CopyUserSettingsArgs::new("6.20-GE-1", "6.21-GE-1");
        let ge_downloader = MockDownloader::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_copy_user_settings().once().returning(|_, _| Ok(()));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
            ],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        command_handler.copy_user_settings(&mut stdout, args).unwrap();

        stdout.assert_line(
            0,
            "Copied user_settings.py from 6.20-GE-1 (Proton) to 6.21-GE-1 (Proton)",
        );
    }

    #[test]
    fn copy_user_settings_fails_on_filesystem_operation() {
        let args = CopyUserSettingsArgs::new("6.20-GE-1", "6.21-GE-1");
        let ge_downloader = MockDownloader::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_copy_user_settings()
            .once()
            .returning(|_, _| bail!("Mocked error"));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
            ],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = command_handler.copy_user_settings(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Mocked error");
        stdout.assert_empty();
    }
}
