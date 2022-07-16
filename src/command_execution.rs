use std::io;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context};
use ge_man_lib::archive;
use ge_man_lib::download::response::DownloadedAssets;
use ge_man_lib::download::{DownloadRequest, GeDownload};
use ge_man_lib::error::GithubError;
use ge_man_lib::tag::TagKind;

use crate::command_input::{
    AddCommandInput, ApplyCommandInput, CheckCommandInput, CleanCommandInput, CleanDryRunInput,
    CopyUserSettingsCommandInput, GivenVersion, ListCommandInput, MigrationCommandInput, RemoveCommandInput,
};
use crate::compat_tool_app::ApplicationConfig;
use crate::data::{ManagedVersion, ManagedVersions};
use crate::filesystem::FilesystemManager;
use crate::message;
use crate::progress::{DownloadProgressTracker, ExtractionProgressTracker};
use crate::version::{Version, Versioned};

#[derive(Debug)]
/// Contains new managed versions and all managed versions including the new ones.
pub struct NewAndManagedVersions {
    pub new_versions: ManagedVersions,
    pub managed_versions: ManagedVersions,
}

impl NewAndManagedVersions {
    pub fn new(new_versions: ManagedVersions, managed_versions: ManagedVersions) -> Self {
        Self {
            new_versions,
            managed_versions,
        }
    }

    pub fn single_add(added_version: ManagedVersion, managed_versions: ManagedVersions) -> Self {
        Self {
            new_versions: ManagedVersions::new(vec![added_version]),
            managed_versions,
        }
    }
}

#[derive(Debug)]
/// Contains removed managed versions and all managed versions excluding the removed ones.
pub struct RemovedAndManagedVersions {
    pub removed_versions: ManagedVersions,
    pub managed_versions: ManagedVersions,
}

impl RemovedAndManagedVersions {
    pub fn new(removed_versions: ManagedVersions, managed_versions: ManagedVersions) -> Self {
        Self {
            removed_versions,
            managed_versions,
        }
    }

    pub fn single_remove(removed_version: ManagedVersion, managed_versions: ManagedVersions) -> Self {
        Self {
            removed_versions: ManagedVersions::new(vec![removed_version]),
            managed_versions,
        }
    }
}

struct ListManagedVersionsInput {
    pub in_use_directory_name: Option<String>,
    pub newest: bool,
    pub tag_kind: TagKind,
    pub application_name: String,
    pub managed_versions: ManagedVersions,
}

impl ListManagedVersionsInput {
    pub fn new(
        in_use_directory_name: Option<String>,
        newest: bool,
        tag_kind: TagKind,
        application_name: String,
        managed_versions: ManagedVersions,
    ) -> Self {
        Self {
            in_use_directory_name,
            newest,
            tag_kind,
            application_name,
            managed_versions,
        }
    }
}

impl From<ListCommandInput> for ListManagedVersionsInput {
    fn from(input: ListCommandInput) -> Self {
        ListManagedVersionsInput::new(
            input.in_use_directory_name,
            input.newest,
            input.tag_kind,
            input.application_name,
            input.managed_versions,
        )
    }
}

struct ListAppCompatToolsDirectory {
    pub kind: TagKind,
    pub app_name: String,
    pub app_compat_tool_dir: PathBuf,
    pub in_use_directory_name: Option<String>,
}

impl ListAppCompatToolsDirectory {
    pub fn new(
        kind: TagKind,
        app_name: String,
        app_compat_tool_dir: PathBuf,
        in_use_directory_name: Option<String>,
    ) -> Self {
        Self {
            kind,
            app_name,
            app_compat_tool_dir,
            in_use_directory_name,
        }
    }
}

impl From<ListCommandInput> for ListAppCompatToolsDirectory {
    fn from(input: ListCommandInput) -> Self {
        ListAppCompatToolsDirectory::new(
            input.tag_kind,
            input.application_name,
            input.app_compat_tool_dir,
            input.in_use_directory_name,
        )
    }
}

/// Handles user interaction and user feedback. This struct basically ties everything together to provide the
/// functionality of each terminal command.
pub struct CommandHandler<'a> {
    ge_downloader: &'a dyn GeDownload,
    fs_mng: &'a dyn FilesystemManager,
}

impl<'a> CommandHandler<'a> {
    pub fn new(ge_downloader: &'a dyn GeDownload, fs_mng: &'a dyn FilesystemManager) -> Self {
        CommandHandler { ge_downloader, fs_mng }
    }

    pub fn list_versions(
        &self,
        stdout: &mut impl Write,
        stderr: &mut impl Write,
        input: ListCommandInput,
    ) -> anyhow::Result<()> {
        let list_fs = input.file_system;

        if list_fs {
            self.list_app_compat_tool_directory(stdout, ListAppCompatToolsDirectory::from(input))?;
        } else {
            self.list_managed_versions(stdout, stderr, ListManagedVersionsInput::from(input));
        }

        Ok(())
    }

    fn list_managed_versions(&self, stdout: &mut impl Write, stderr: &mut impl Write, input: ListManagedVersionsInput) {
        let ListManagedVersionsInput {
            in_use_directory_name,
            newest,
            tag_kind,
            application_name,
            managed_versions,
        } = input;

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

            managed_versions.sort_unstable_by(|a, b| a.tag().cmp(b.tag()).reverse());
            for version in &managed_versions {
                let line = match &in_use_directory_name {
                    Some(dir) if dir.eq(version.directory_name()) => {
                        format!("{} - In use by {}", version.tag(), application_name)
                    }
                    Some(_) => version.tag().str().clone(),
                    None => version.tag().str().clone(),
                };
                writeln!(stdout, "* {}", line).unwrap();
            }
        } else {
            writeln!(stdout, "No versions installed").unwrap();
        }
    }

    fn list_app_compat_tool_directory(
        &self,
        stdout: &mut impl Write,
        input: ListAppCompatToolsDirectory,
    ) -> anyhow::Result<()> {
        let ListAppCompatToolsDirectory {
            app_compat_tool_dir,
            kind,
            in_use_directory_name,
            app_name,
        } = input;

        writeln!(stdout, "{}:", kind.compatibility_tool_name()).unwrap();
        let mut paths = self.fs_mng.paths_for_directory_items(&app_compat_tool_dir)?;
        paths.sort_unstable_by(|a, b| a.file_name().cmp(&b.file_name()).reverse());
        for path in paths {
            let file_name = path.file_name().map(|os_str| os_str.to_string_lossy()).unwrap();
            let line = match &in_use_directory_name {
                Some(in_use_dir) if in_use_dir.eq(&file_name) => {
                    format!("{} - In use by {}", path.display(), app_name)
                }
                Some(_) => path.display().to_string(),
                None => path.display().to_string(),
            };
            writeln!(stdout, "* {}", line).unwrap();
        }

        Ok(())
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
            return Ok(NewAndManagedVersions::single_add(existing_version, managed_versions));
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

        let new_version = managed_versions.add(version);
        Ok(NewAndManagedVersions::single_add(new_version, managed_versions))
    }

    fn remove_version(
        &self,
        app_config: &ApplicationConfig,
        version: &ManagedVersion,
        managed_versions: &mut ManagedVersions,
        forget: bool,
    ) -> anyhow::Result<ManagedVersion> {
        if app_config.check_if_version_is_in_use(version) {
            bail!(
                "{} version is in use. Apply a different version first to make removal possible",
                version.kind().compatibility_tool_name()
            );
        }

        if !forget {
            self.fs_mng.remove_version(version)?;
        }
        match managed_versions.remove(version) {
            Some(removed_version) => Ok(removed_version),
            None => bail!(format!("Can not remove version: Version {} does not exist", version)),
        }
    }

    pub fn remove(&self, input: RemoveCommandInput) -> anyhow::Result<RemovedAndManagedVersions> {
        let RemoveCommandInput {
            mut managed_versions,
            version_to_remove,
            app_config_path,
            forget,
        } = input;

        let app_config = ApplicationConfig::create_copy(*version_to_remove.kind(), &app_config_path);
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

        let removed_version =
            self.remove_version(&app_config.unwrap(), &version_to_remove, &mut managed_versions, forget)?;
        Ok(RemovedAndManagedVersions::single_remove(
            removed_version,
            managed_versions,
        ))
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

    pub fn migrate(&self, input: MigrationCommandInput) -> anyhow::Result<NewAndManagedVersions> {
        let MigrationCommandInput {
            source_path,
            version,
            mut managed_versions,
        } = input;

        if managed_versions.find_version(&version).is_some() {
            bail!("Given version to migrate already exists as a managed version");
        }

        let version = self
            .fs_mng
            .migrate_folder(version, &source_path)
            .context("Could not migrate directory")?;

        let version = managed_versions.add(version);
        Ok(NewAndManagedVersions::single_add(version, managed_versions))
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

    pub fn copy_user_settings(
        &self,
        stdout: &mut impl Write,
        input: CopyUserSettingsCommandInput,
    ) -> anyhow::Result<()> {
        let CopyUserSettingsCommandInput {
            src_version,
            dst_version,
        } = input;

        self.fs_mng.copy_user_settings(&src_version, &dst_version)?;

        writeln!(
            stdout,
            "Copied user_settings.py from {} to {}",
            src_version, dst_version
        )
        .unwrap();
        Ok(())
    }

    fn get_versions_for_clean<S, E>(
        &self,
        before: Option<&S>,
        start: Option<&S>,
        end: Option<&E>,
        managed_versions: &ManagedVersions,
    ) -> anyhow::Result<ManagedVersions>
    where
        S: Versioned,
        E: Versioned,
    {
        let filtered = if let Some(remove_before_version) = before {
            managed_versions.versions_before_given(remove_before_version)
        } else {
            let start_version = start.unwrap();
            let end_version = end.unwrap();
            managed_versions.versions_in_range(start_version, end_version)?
        };
        Ok(filtered)
    }

    pub fn clean<S, E>(
        &self,
        stderr: &mut impl Write,
        input: CleanCommandInput<S, E>,
    ) -> anyhow::Result<RemovedAndManagedVersions>
    where
        S: Versioned,
        E: Versioned,
    {
        let CleanCommandInput {
            remove_before_version,
            start_version,
            end_version,
            mut managed_versions,
            app_config,
            forget,
        } = input;

        let versions_to_remove = self.get_versions_for_clean(
            remove_before_version.as_ref(),
            start_version.as_ref(),
            end_version.as_ref(),
            &managed_versions,
        )?;

        let mut removed_versions = ManagedVersions::default();
        for version in versions_to_remove {
            removed_versions.push(version.clone());
            let remove_result = self.remove_version(&app_config, &version, &mut managed_versions, forget);
            if let Err(err) = remove_result {
                let re_added_version = removed_versions.pop().unwrap();
                writeln!(stderr, "Failed to remove the version {}:", re_added_version.tag(),).unwrap();
                writeln!(stderr, "\t{:#}", err).unwrap();
            }
        }

        Ok(RemovedAndManagedVersions::new(removed_versions, managed_versions))
    }

    pub fn clean_dry_run<S, E>(&self, stdout: &mut impl Write, input: CleanDryRunInput<S, E>) -> anyhow::Result<()>
    where
        S: Versioned,
        E: Versioned,
    {
        let CleanDryRunInput {
            remove_before_version,
            start_version,
            end_version,
            managed_versions,
        } = input;

        let mut versions_to_remove = self.get_versions_for_clean(
            remove_before_version.as_ref(),
            start_version.as_ref(),
            end_version.as_ref(),
            &managed_versions,
        )?;
        versions_to_remove.sort_unstable_by(|a, b| a.cmp(b).reverse());

        writeln!(stdout, "DRY-RUN - The following versions would be removed:").unwrap();
        for version in versions_to_remove {
            writeln!(stdout, "* {}", version.tag()).unwrap();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::{fs, io};

    use anyhow::bail;
    use ge_man_lib::download::response::{DownloadedArchive, DownloadedChecksum, GeRelease};
    use mockall::mock;

    use crate::filesystem::MockFilesystemManager;

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
            assert_eq!(line.trim_end(), expected);
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

    #[test]
    fn list_steam_newest_version() {
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("GE-Proton7-22", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
        ]);
        let input = ListCommandInput::new(
            TagKind::Proton,
            true,
            None,
            managed_versions,
            String::from("Steam"),
            false,
            PathBuf::default(),
        );
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        command_handler.list_versions(&mut stdout, &mut stderr, input).unwrap();

        stdout.assert_count(2);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* GE-Proton7-22");
    }

    #[test]
    fn list_newest_steam_version_with_in_use_info() {
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("GE-Proton7-22", TagKind::Proton, "GE-Proton7-22"),
            ManagedVersion::new("6.21-GE-1", TagKind::Proton, "6.21-GE-1"),
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6.20-GE-1"),
        ]);
        let input = ListCommandInput::new(
            TagKind::Proton,
            true,
            Some(String::from("GE-Proton7-22")),
            managed_versions.clone(),
            String::from("Steam"),
            false,
            PathBuf::default(),
        );
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        command_handler.list_versions(&mut stdout, &mut stderr, input).unwrap();

        stdout.assert_count(2);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* GE-Proton7-22 - In use by Steam");
    }

    #[test]
    fn list_steam_versions() {
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("GE-Proton7-22", TagKind::Proton, "GE-Proton7-22"),
            ManagedVersion::new("6.21-GE-1", TagKind::Proton, "6.21-GE-1"),
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6.20-GE-1"),
        ]);
        let input = ListCommandInput::new(
            TagKind::Proton,
            false,
            None,
            managed_versions,
            String::from("Steam"),
            false,
            PathBuf::default(),
        );
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        command_handler.list_versions(&mut stdout, &mut stderr, input).unwrap();

        stdout.assert_count(4);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* GE-Proton7-22");
        stdout.assert_line(2, "* 6.21-GE-1");
        stdout.assert_line(3, "* 6.20-GE-1");
    }

    #[test]
    fn list_steam_versions_with_in_use_info() {
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
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
            false,
            PathBuf::default(),
        );
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        command_handler.list_versions(&mut stdout, &mut stderr, input).unwrap();

        stdout.assert_count(4);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* GE-Proton7-22 - In use by Steam");
        stdout.assert_line(2, "* 6.21-GE-1");
        stdout.assert_line(3, "* 6.20-GE-1");
    }

    #[test]
    fn list_versions_from_file_system() {
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_paths_for_directory_items().once().returning(|_| {
            Ok(vec![
                PathBuf::from("/tmp/test/Proton-6.20-GE-1"),
                PathBuf::from("/tmp/test/Proton-6.21-GE-1"),
                PathBuf::from("/tmp/test/Proton-6.21-GE-2"),
            ])
        });

        let ge_downloader = MockDownloader::new();

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        let input = ListCommandInput::new(
            TagKind::Proton,
            false,
            Some(String::from("GE-Proton7-22")),
            ManagedVersions::default(),
            String::from("Steam"),
            true,
            PathBuf::from("/tmp/test"),
        );
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        command_handler.list_versions(&mut stdout, &mut stderr, input).unwrap();

        stdout.assert_count(4);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* /tmp/test/Proton-6.21-GE-2");
        stdout.assert_line(2, "* /tmp/test/Proton-6.21-GE-1");
        stdout.assert_line(3, "* /tmp/test/Proton-6.20-GE-1");
    }

    #[test]
    fn list_versions_from_file_system_with_in_use_hint() {
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_paths_for_directory_items().once().returning(|_| {
            Ok(vec![
                PathBuf::from("/tmp/test/Proton-6.20-GE-1"),
                PathBuf::from("/tmp/test/Proton-6.21-GE-1"),
                PathBuf::from("/tmp/test/Proton-6.21-GE-2"),
            ])
        });

        let ge_downloader = MockDownloader::new();

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        let input = ListCommandInput::new(
            TagKind::Proton,
            false,
            Some(String::from("Proton-6.21-GE-1")),
            ManagedVersions::default(),
            String::from("Steam"),
            true,
            PathBuf::from("/tmp/test"),
        );
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        command_handler.list_versions(&mut stdout, &mut stderr, input).unwrap();

        stdout.assert_count(4);
        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* /tmp/test/Proton-6.21-GE-2");
        stdout.assert_line(2, "* /tmp/test/Proton-6.21-GE-1 - In use by Steam");
        stdout.assert_line(3, "* /tmp/test/Proton-6.20-GE-1");
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

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

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

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

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

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

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

        let mut ge_downloader = MockDownloader::new();
        ge_downloader
            .expect_fetch_release()
            .once()
            .returning(move |_, _| Ok(GeRelease::new(String::from(tag), Vec::new())));

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

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
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().once().returning(|_| Ok(()));

        let steam_config_path = PathBuf::from("test_resources/assets/config.vdf");
        let version_to_remove = ManagedVersion::new("6.20-GE-1", TagKind::Proton, "");
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1"),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, "6-21-GE-2"),
        ]);
        let input = RemoveCommandInput::new(managed_versions, version_to_remove.clone(), steam_config_path, false);

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        let removed_and_managed_versions = command_handler.remove(input).unwrap();
        assert_eq!(removed_and_managed_versions.managed_versions.len(), 1);
        assert_eq!(
            removed_and_managed_versions.managed_versions,
            ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-2", TagKind::Proton, "")])
        );
        assert_eq!(removed_and_managed_versions.removed_versions[0], version_to_remove)
    }

    #[test]
    fn remove_version_used_by_app_config() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().never();

        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6.20-GE-1"),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, "Proton-6.21-GE-2"),
        ]);

        let steam_config_path = PathBuf::from("test_resources/assets/config.vdf");
        let version_to_remove = ManagedVersion::new("6.21-GE-2", TagKind::Proton, "Proton-6.21-GE-2");
        let input = RemoveCommandInput::new(managed_versions, version_to_remove, steam_config_path, false);

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        let result = command_handler.remove(input);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Proton GE version is in use. Apply a different version first to make removal possible"
        );
    }

    #[test]
    fn remove_version_with_forget_flag_should_not_delete_from_file_system() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().never();

        let steam_config_path = PathBuf::from("test_resources/assets/config.vdf");
        let version_to_remove = ManagedVersion::new("6.20-GE-1", TagKind::Proton, "");
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1"),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, "6-21-GE-2"),
        ]);
        let input = RemoveCommandInput::new(managed_versions, version_to_remove.clone(), steam_config_path, true);

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);
        let removed_and_managed_versions = command_handler.remove(input).unwrap();
        assert_eq!(removed_and_managed_versions.managed_versions.len(), 1);
        assert_eq!(
            removed_and_managed_versions.managed_versions,
            ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-2", TagKind::Proton, "")])
        );
        assert_eq!(removed_and_managed_versions.removed_versions[0], version_to_remove)
    }

    #[test]
    fn check_with_successful_requests() {
        let fs_mng = MockFilesystemManager::new();
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

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        let input = CheckCommandInput::new(None);
        command_handler.check(&mut stdout, &mut stderr, input);

        stdout.assert_line(0, "These are the latest releases.");
        stdout.assert_line(1, "");
        stdout.assert_line(2, "Proton GE: 6.20-GE-1");
        stdout.assert_line(3, "Wine GE: 6.20-GE-1");
        stdout.assert_line(4, "Wine GE (LoL): 6.16-GE-3-LoL");
    }

    #[test]
    fn check_with_only_errors() {
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

        let fs_mng = MockFilesystemManager::new();

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        let input = CheckCommandInput::new(None);
        command_handler.check(&mut stdout, &mut stderr, input);

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
        let ge_downloader = MockDownloader::new();
        let fs_mng = MockFilesystemManager::new();

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let input = MigrationCommandInput::new(
            PathBuf::from("invalid-path"),
            Version::new("6.20-GE-1", TagKind::Proton),
            ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1")]),
        );
        let result = command_handler.migrate(input);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Given version to migrate already exists as a managed version"
        );
    }

    #[test]
    fn migrate_not_present_version() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_migrate_folder()
            .once()
            .returning(|_, _| Ok(ManagedVersion::new("6.20-GE-1", TagKind::Proton, "Proton-6.20-GE-1")));

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let input = MigrationCommandInput::new(
            PathBuf::from("valid-path"),
            Version::new("6.20-GE-1", TagKind::Proton),
            ManagedVersions::default(),
        );
        let new_and_managed_versions = command_handler.migrate(input).unwrap();

        let expected_version = ManagedVersion::new("6.20-GE-1", TagKind::Proton, "Proton-6.20-GE-1");
        assert_eq!(new_and_managed_versions.managed_versions.vec_ref().len(), 1);
        assert_eq!(new_and_managed_versions.managed_versions.vec_ref()[0], expected_version);
        assert_eq!(new_and_managed_versions.new_versions[0], expected_version);
    }

    #[test]
    fn migrate_fails_due_to_filesystem_error() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_migrate_folder()
            .once()
            .returning(|_, _| bail!("Mocked error"));

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let input = MigrationCommandInput::new(
            PathBuf::from("valid-path"),
            Version::new("6.20-GE-1", TagKind::Proton),
            ManagedVersions::default(),
        );
        let result = command_handler.migrate(input);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Could not migrate directory");
    }

    #[test]
    fn apply_to_app_config_for_non_existent_version() {
        let ge_downloader = MockDownloader::new();
        let fs_mng = MockFilesystemManager::new();

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

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

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

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

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

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

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

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
    fn copy_user_settings_for_present_versions() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_copy_user_settings().once().returning(|_, _| Ok(()));

        let src_version = ManagedVersion::new("6.20-GE-1", TagKind::Proton, "");
        let dst_version = ManagedVersion::new("6.21-GE-1", TagKind::Proton, "");
        let input = CopyUserSettingsCommandInput::new(src_version, dst_version);
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stdout = AssertLines::new();
        command_handler.copy_user_settings(&mut stdout, input).unwrap();

        stdout.assert_line(
            0,
            "Copied user_settings.py from 6.20-GE-1 (Proton) to 6.21-GE-1 (Proton)",
        );
    }

    #[test]
    fn copy_user_settings_fails_on_filesystem_operation() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_copy_user_settings()
            .once()
            .returning(|_, _| bail!("Mocked error"));

        let src_version = ManagedVersion::new("6.20-GE-1", TagKind::Proton, "");
        let dst_version = ManagedVersion::new("6.21-GE-1", TagKind::Proton, "");
        let input = CopyUserSettingsCommandInput::new(src_version, dst_version);
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stdout = AssertLines::new();
        let result = command_handler.copy_user_settings(&mut stdout, input);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Mocked error");
        stdout.assert_empty();
    }

    #[test]
    fn clean_should_remove_versions_below_given_version() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().times(2).returning(|_| Ok(()));

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stderr = AssertLines::new();
        let app_config = ApplicationConfig::new(TagKind::Proton, "GE-Proton7-20".to_string());
        let input: CleanCommandInput<Version, Version> = CleanCommandInput::new(
            Some(Version::new("6.21-GE-2", TagKind::Proton)),
            None,
            None,
            ManagedVersions::new(vec![
                ManagedVersion::new("6.20-GE-1", TagKind::lol(), ""),
                ManagedVersion::new("6.21-GE-1", TagKind::lol(), ""),
                ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.21-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
                ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
            ]),
            app_config,
            false,
        );

        let removed_and_managed_versions = command_handler.clean(&mut stderr, input).unwrap();
        stderr.assert_empty();
        assert_eq!(removed_and_managed_versions.managed_versions.len(), 6);
        assert_eq!(removed_and_managed_versions.removed_versions.len(), 2);
        vec![
            ManagedVersion::new("6.20-GE-1", TagKind::lol(), ""),
            ManagedVersion::new("6.21-GE-1", TagKind::lol(), ""),
            ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
            ManagedVersion::new("6.21-GE-1", TagKind::wine(), ""),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
            ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
        ]
        .iter()
        .for_each(|version| {
            assert!(removed_and_managed_versions.managed_versions.contains(version));
        });
        assert_eq!(
            removed_and_managed_versions.removed_versions,
            ManagedVersions::new(vec![
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
            ])
        );
    }

    #[test]
    fn clean_should_remove_versions_in_given_range() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().times(4).returning(|_| Ok(()));

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stderr = AssertLines::new();
        let app_config = ApplicationConfig::new(TagKind::Proton, "GE-Proton7-20".to_string());
        let input: CleanCommandInput<Version, Version> = CleanCommandInput::new(
            None,
            Some(Version::new("6.19-GE-1", TagKind::Proton)),
            Some(Version::new("6.22-GE-2", TagKind::Proton)),
            ManagedVersions::new(vec![
                ManagedVersion::new("6.20-GE-1", TagKind::lol(), ""),
                ManagedVersion::new("6.21-GE-1", TagKind::lol(), ""),
                ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.21-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
                ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
            ]),
            app_config,
            false,
        );

        let removed_and_managed_versions = command_handler.clean(&mut stderr, input).unwrap();
        stderr.assert_empty();
        assert_eq!(removed_and_managed_versions.managed_versions.len(), 4);
        assert_eq!(removed_and_managed_versions.removed_versions.len(), 4);
        vec![
            ManagedVersion::new("6.20-GE-1", TagKind::lol(), ""),
            ManagedVersion::new("6.21-GE-1", TagKind::lol(), ""),
            ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
            ManagedVersion::new("6.21-GE-1", TagKind::wine(), ""),
        ]
        .iter()
        .for_each(|version| {
            assert!(removed_and_managed_versions.managed_versions.contains(version));
        });
        assert_eq!(
            removed_and_managed_versions.removed_versions,
            ManagedVersions::new(vec![
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
                ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
            ])
        );
    }

    #[test]
    fn clean_should_not_remove_versions_where_an_file_system_error_occurred() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_remove_version()
            .once()
            .withf(|version: &ManagedVersion| version.tag().str().eq("6.20-GE-1"))
            .returning(|_| bail!("Mocked error"));

        fs_mng
            .expect_remove_version()
            .once()
            .withf(|version: &ManagedVersion| version.tag().str().eq("6.21-GE-1"))
            .returning(|_| Ok(()));

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stderr = AssertLines::new();
        let app_config = ApplicationConfig::new(TagKind::Proton, "GE-Proton7-20".to_string());
        let input: CleanCommandInput<Version, Version> = CleanCommandInput::new(
            Some(Version::new("6.21-GE-2", TagKind::Proton)),
            None,
            None,
            ManagedVersions::new(vec![
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
                ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
            ]),
            app_config,
            false,
        );

        let removed_and_managed_versions = command_handler.clean(&mut stderr, input).unwrap();
        assert_eq!(removed_and_managed_versions.managed_versions.len(), 3);
        assert_eq!(removed_and_managed_versions.removed_versions.len(), 1);
        vec![
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
            ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
        ]
        .iter()
        .for_each(|version| {
            assert!(removed_and_managed_versions.managed_versions.contains(version));
        });
        assert_eq!(
            removed_and_managed_versions.removed_versions,
            ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-1", TagKind::Proton, "")])
        );

        stderr.assert_count(2);
        stderr.assert_line(0, "Failed to remove the version 6.20-GE-1:");
        stderr.assert_line(1, "\tMocked error");
    }

    #[test]
    fn clean_with_forget_arg_should_not_delete_from_file_system() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().never();

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stderr = AssertLines::new();
        let app_config = ApplicationConfig::new(TagKind::Proton, "GE-Proton7-20".to_string());
        let input: CleanCommandInput<Version, Version> = CleanCommandInput::new(
            None,
            Some(Version::new("6.19-GE-1", TagKind::Proton)),
            Some(Version::new("6.22-GE-2", TagKind::Proton)),
            ManagedVersions::new(vec![
                ManagedVersion::new("6.20-GE-1", TagKind::lol(), ""),
                ManagedVersion::new("6.21-GE-1", TagKind::lol(), ""),
                ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.21-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
                ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
            ]),
            app_config,
            true,
        );

        let removed_and_managed_versions = command_handler.clean(&mut stderr, input).unwrap();
        stderr.assert_empty();
        assert_eq!(removed_and_managed_versions.managed_versions.len(), 4);
        assert_eq!(removed_and_managed_versions.removed_versions.len(), 4);
        vec![
            ManagedVersion::new("6.20-GE-1", TagKind::lol(), ""),
            ManagedVersion::new("6.21-GE-1", TagKind::lol(), ""),
            ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
            ManagedVersion::new("6.21-GE-1", TagKind::wine(), ""),
        ]
        .iter()
        .for_each(|version| {
            assert!(removed_and_managed_versions.managed_versions.contains(version));
        });
        assert_eq!(
            removed_and_managed_versions.removed_versions,
            ManagedVersions::new(vec![
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
                ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
            ])
        );
    }

    #[test]
    fn clean_should_not_remove_version_in_use() {
        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng
            .expect_remove_version()
            .never()
            .withf(|version: &ManagedVersion| version.tag().str().eq("6.20-GE-1"));

        fs_mng
            .expect_remove_version()
            .once()
            .withf(|version: &ManagedVersion| version.tag().str().eq("6.21-GE-1"))
            .returning(|_| Ok(()));

        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stderr = AssertLines::new();
        let app_config = ApplicationConfig::new(TagKind::Proton, "6-20-GE-1".to_string());
        let input: CleanCommandInput<Version, Version> = CleanCommandInput::new(
            Some(Version::new("6.21-GE-2", TagKind::Proton)),
            None,
            None,
            ManagedVersions::new(vec![
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, "6-20-GE-1"),
                ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
                ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
            ]),
            app_config,
            false,
        );

        let removed_and_managed_versions = command_handler.clean(&mut stderr, input).unwrap();
        assert_eq!(removed_and_managed_versions.managed_versions.len(), 3);
        assert_eq!(removed_and_managed_versions.removed_versions.len(), 1);
        vec![
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
            ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
        ]
        .iter()
        .for_each(|version| {
            assert!(removed_and_managed_versions.managed_versions.contains(version));
        });
        assert_eq!(
            removed_and_managed_versions.removed_versions,
            ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-1", TagKind::Proton, "")])
        );

        stderr.assert_count(2);
        stderr.assert_line(0, "Failed to remove the version 6.20-GE-1:");
        stderr.assert_line(
            1,
            "\tProton GE version is in use. Apply a different version first to make removal possible",
        );
    }

    #[test]
    fn clean_dry_run_should_print_versions() {
        let ge_downloader = MockDownloader::new();
        let fs_mng = MockFilesystemManager::new();
        let command_handler = CommandHandler::new(&ge_downloader, &fs_mng);

        let mut stdout = AssertLines::new();
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", TagKind::lol(), ""),
            ManagedVersion::new("6.21-GE-1", TagKind::lol(), ""),
            ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
            ManagedVersion::new("6.21-GE-1", TagKind::wine(), ""),
            ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-2", TagKind::Proton, ""),
            ManagedVersion::new("6.22-GE-1", TagKind::Proton, ""),
        ]);
        let input: CleanDryRunInput<Version, Version> = CleanDryRunInput::new(
            Some(Version::new("6.21-GE-2", TagKind::Proton)),
            None,
            None,
            &managed_versions,
        );

        command_handler.clean_dry_run(&mut stdout, input).unwrap();
        stdout.assert_count(3);
        stdout.assert_line(0, "DRY-RUN - The following versions would be removed:");
        stdout.assert_line(1, "* 6.21-GE-1");
        stdout.assert_line(2, "* 6.20-GE-1");
    }
}
