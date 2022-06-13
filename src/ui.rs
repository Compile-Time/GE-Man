use std::io::Write;

use anyhow::{anyhow, bail, Context};
use ge_man_lib::archive;
use ge_man_lib::config::{LutrisConfig, SteamConfig};
use ge_man_lib::download::response::DownloadedAssets;
use ge_man_lib::download::{DownloadRequest, GeDownload};
use ge_man_lib::error::{GithubError, LutrisConfigError, SteamConfigError};
use ge_man_lib::tag::TagKind;
use itertools::Itertools;
use log::debug;

use crate::args::{
    AddArgs, ApplyArgs, CheckArgs, CopyUserSettingsArgs, ForgetArgs, ListArgs, MigrationArgs, RemoveArgs,
};
use crate::data::{ManagedVersion, ManagedVersions};
use crate::filesystem::FilesystemManager;
use crate::path::{overrule, PathConfiguration};
use crate::progress::{DownloadProgressTracker, ExtractionProgressTracker};
use crate::version::{Version, Versioned};

const PROTON_APPLY_HINT: &str = "Successfully modified Steam config: If Steam is currently running, \
any external change by GE-Man will not take effect and the new version can not be selected in the Steam settings!
 
 To select the latest version you have two options:
 \t1. Restart Steam to select the new version in Steam (which then requires a second restart for Steam to register the change).
 \t2. Close Steam and run the apply command for your desired version. On the next start Steam will use the applied version.";

trait AppConfig {
    fn version_dir_name(&self) -> String;
    fn kind(&self) -> String;
}

impl AppConfig for SteamConfig {
    fn version_dir_name(&self) -> String {
        self.proton_version()
    }

    fn kind(&self) -> String {
        String::from("Steam")
    }
}

impl AppConfig for LutrisConfig {
    fn version_dir_name(&self) -> String {
        self.wine_version()
    }

    fn kind(&self) -> String {
        String::from("Lutris")
    }
}

trait AppConfigError {
    fn kind(&self) -> String;
}

impl AppConfigError for SteamConfigError {
    fn kind(&self) -> String {
        String::from("Steam")
    }
}

impl AppConfigError for LutrisConfigError {
    fn kind(&self) -> String {
        String::from("Lutris")
    }
}

/// Handles user interaction and user feedback. This struct basically ties everything together to provide the
/// functionality of each terminal command.
pub struct TerminalWriter<'a> {
    ge_downloader: &'a dyn GeDownload,
    fs_mng: &'a dyn FilesystemManager,
    path_cfg: &'a dyn PathConfiguration,
}

impl<'a> TerminalWriter<'a> {
    pub fn new(
        ge_downloader: &'a dyn GeDownload,
        fs_mng: &'a dyn FilesystemManager,
        path_cfg: &'a dyn PathConfiguration,
    ) -> Self {
        TerminalWriter {
            ge_downloader,
            fs_mng,
            path_cfg,
        }
    }

    fn create_list_line(
        &self,
        version: ManagedVersion,
        wine_version_dir_name: Option<&String>,
        proton_version_dir_name: Option<&String>,
    ) -> String {
        if let Some(dir) = wine_version_dir_name {
            if dir.eq(version.directory_name()) {
                return format!("{} - In use by Lutris", version.tag());
            }
        }

        if let Some(dir) = proton_version_dir_name {
            if dir.eq(version.directory_name()) {
                return format!("{} - In use by Steam", version.tag());
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

    pub fn list(&self, stdout: &mut impl Write, args: ListArgs) -> anyhow::Result<()> {
        let lutris_path = self.path_cfg.lutris_wine_runner_config(overrule::xdg_config_home());
        debug!(
            "Reading currently used Lutris version from the following config file: {}",
            lutris_path.display()
        );
        let wine_dir_name = match LutrisConfig::create_copy(&lutris_path) {
            Ok(config) => Some(config.wine_version()),
            Err(_) => None,
        };

        let steam_path = self.path_cfg.steam_config(overrule::steam_root());
        debug!(
            "Reading currently used Steam version from the following config file: {}",
            steam_path.display()
        );
        let proton_dir_name = match SteamConfig::create_copy(&steam_path) {
            Ok(config) => Some(config.proton_version()),
            Err(_) => None,
        };

        let mut managed_versions: Vec<ManagedVersion> = if args.newest {
            self.read_managed_versions()?.latest_versions()
        } else {
            self.read_managed_versions()?.versions()
        };

        if let Some(kind) = args.kind {
            managed_versions.retain(|v| v.kind().eq(&kind));
        }

        if !managed_versions.is_empty() {
            // Allow clone of version.kind() due to lifetime not living long enough.
            #[allow(clippy::clone_on_copy)]
            let grouped_versions = managed_versions
                .into_iter()
                .sorted_unstable_by(|a, b| a.kind().cmp(b.kind()))
                .group_by(|version| version.kind().clone());

            for (kind, group) in &grouped_versions {
                writeln!(stdout, "{}:", kind.compatibility_tool_name()).unwrap();

                group
                    .sorted_unstable_by(|a, b| a.tag().cmp_semver(b.tag()).reverse())
                    .for_each(|version| {
                        let line = self.create_list_line(version, wine_dir_name.as_ref(), proton_dir_name.as_ref());
                        writeln!(stdout, "* {}", line).unwrap();
                    });

                writeln!(stdout).unwrap();
            }
        } else {
            writeln!(stdout, "No versions installed").unwrap();
        }
        Ok(())
    }

    pub fn add(&self, stdout: &mut impl Write, args: AddArgs) -> anyhow::Result<()> {
        let tag = args.tag_arg.value();
        let kind = args.tag_arg.kind;
        let mut managed_versions = self.read_managed_versions()?;

        let version = if tag.is_some() {
            Version::new(tag.cloned(), kind)
        } else {
            match self.ge_downloader.fetch_release(tag.cloned(), kind) {
                Ok(release) => Version::new(release.tag_name, kind),
                Err(err) => {
                    return Err(anyhow!(err).context(r#"Could not get latest tag for tagless "add" operation."#))
                }
            }
        };

        if managed_versions.find_version(&version).is_some() {
            writeln!(stdout, "Version {} is already managed", version)?;
            return Ok(());
        }

        let download_tracker = Box::new(DownloadProgressTracker::default());
        let request = DownloadRequest::new(
            Some(version.tag().to_string()),
            *version.kind(),
            download_tracker,
            args.skip_checksum,
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

        if args.skip_checksum {
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

        let version = managed_versions.add(version)?;
        self.write_managed_versions(managed_versions)?;

        writeln!(stdout, "Successfully added version").unwrap();
        if args.apply {
            self.do_apply_to_app_config(stdout, &version)?;
        }

        Ok(())
    }

    pub fn remove(&self, stdout: &mut impl Write, args: RemoveArgs) -> anyhow::Result<()> {
        let version = args.tag_arg.version();
        let mut managed_versions = self.read_managed_versions()?;

        let version = match managed_versions.find_version(&version) {
            Some(v) => v,
            None => bail!("Given version is not managed"),
        };

        match &version.kind() {
            TagKind::Proton => {
                let path = self.path_cfg.steam_config(overrule::steam_root());
                debug!("Removing {} from Steam config file: {}", version.tag(), path.display());
                let config = SteamConfig::create_copy(&path)
                    .map_err(|err| anyhow!(err))
                    .context(format!("Failed to read Steam config: {}", path.display()))?;

                if self.check_if_version_in_use_by_config(&version, &config) {
                    bail!("Proton version is in use by Steam. Select a different version to make removal possible.");
                }
            }
            TagKind::Wine { .. } => {
                let path = self.path_cfg.lutris_wine_runner_config(overrule::xdg_config_home());
                debug!("Removing {} from Lutris config file: {}", version.tag(), path.display());
                let config = LutrisConfig::create_copy(&path);
                match config {
                    Ok(config) => {
                        if self.check_if_version_in_use_by_config(&version, &config) {
                            bail!(
                                "Wine version is in use by Lutris. Select a different version to make removal \
                            possible."
                            );
                        }
                    }
                    Err(err) => {
                        if let LutrisConfigError::IoError { source } = &err {
                            if source.raw_os_error().unwrap() != 2 {
                                bail!(err);
                            }
                        }
                    }
                }
            }
        }

        self.fs_mng.remove_version(&version).unwrap();
        managed_versions.remove(&version).unwrap();

        self.write_managed_versions(managed_versions)?;
        writeln!(stdout, "Successfully removed version {}.", version).unwrap();
        Ok(())
    }

    fn check_if_version_in_use_by_config<T>(&self, version: &ManagedVersion, app_config: &T) -> bool
    where
        T: AppConfig,
    {
        app_config.version_dir_name().eq(version.directory_name())
    }

    pub fn check(&self, stdout: &mut impl Write, stderr: &mut impl Write, args: CheckArgs) {
        match args.kind {
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
                let proton = self.ge_downloader.fetch_release(None, TagKind::Proton);
                let wine = self.ge_downloader.fetch_release(None, TagKind::wine());
                let lol = self.ge_downloader.fetch_release(None, TagKind::lol());

                writeln!(stdout, "These are the latest releases.").unwrap();
                writeln!(stdout).unwrap();
                match proton {
                    Ok(release) => writeln!(stdout, "Proton GE: {}", release.tag_name).unwrap(),
                    Err(err) => writeln!(
                        stderr,
                        "Proton GE: Could not fetch release information from GitHub: {}",
                        err
                    )
                    .unwrap(),
                }

                match wine {
                    Ok(release) => writeln!(stdout, "Wine GE: {}", release.tag_name).unwrap(),
                    Err(err) => writeln!(
                        stderr,
                        "Wine GE: Could not fetch release information from GitHub: {}",
                        err
                    )
                    .unwrap(),
                }

                match lol {
                    Ok(release) => writeln!(stdout, "Wine GE - LoL: {}", release.tag_name).unwrap(),
                    Err(err) => writeln!(
                        stderr,
                        "Wine GE - LoL: Could not fetch release information from GitHub: {}",
                        err
                    )
                    .unwrap(),
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

    fn do_apply_to_app_config(&self, stdout: &mut impl Write, version: &ManagedVersion) -> anyhow::Result<()> {
        let (modify_msg, success_msg) = match version.kind() {
            TagKind::Proton => (
                format!("Modifying Steam configuration to use {}", version),
                PROTON_APPLY_HINT,
            ),
            TagKind::Wine { .. } => (
                format!("Modifying Lutris configuration to use {}", version),
                "Successfully modified Lutris config: Lutris should be restarted for the new settings to take effect.",
            ),
        };

        writeln!(stdout, "{}", modify_msg).unwrap();

        self.fs_mng
            .apply_to_app_config(version)
            .context("Could not modify app config")?;

        writeln!(stdout, "{}", success_msg).unwrap();

        Ok(())
    }

    pub fn apply_to_app_config(&self, stdout: &mut impl Write, args: ApplyArgs) -> anyhow::Result<()> {
        let managed_versions = self.read_managed_versions()?;

        let version = if args.tag_arg.tag.is_some() {
            let version = args.tag_arg.version();
            match managed_versions.find_version(&version) {
                Some(v) => v,
                None => bail!("Given version is not managed"),
            }
        } else {
            let kind = args.tag_arg.kind;
            if let Some(version) = managed_versions.find_latest_by_kind(&kind) {
                version
            } else {
                bail!("No managed versions exist");
            }
        };

        self.do_apply_to_app_config(stdout, &version)
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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        writer.forget(&mut stdout, args).unwrap();

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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.forget(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Failed to forget version: Version is not managed");
        stdout.assert_empty();
    }

    #[test]
    fn list_newest_output() {
        let args = ListArgs::new(None, true);
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.16-GE-3-LoL", TagKind::lol(), ""),
            ],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let steam_path = PathBuf::from("test_resources/assets/config.vdf");
        path_cfg
            .expect_steam_config()
            .once()
            .returning(move |_| steam_path.clone());

        let lutris_path = PathBuf::from("test_resources/assets/wine.yml");
        path_cfg
            .expect_lutris_wine_runner_config()
            .once()
            .returning(move |_| lutris_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);
        let mut stdout = AssertLines::new();
        writer.list(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* 6.20-GE-1");
        stdout.assert_line(2, "");
        stdout.assert_line(3, "Wine GE:");
        stdout.assert_line(4, "* 6.20-GE-1");
        stdout.assert_line(5, "");
        stdout.assert_line(6, "Wine GE (LoL):");
        stdout.assert_line(7, "* 6.16-GE-3-LoL");
        stdout.assert_line(8, "");
    }

    #[test]
    fn list_newest_output_with_in_use_version() {
        let args = ListArgs::new(None, true);
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, "Proton-6.21-GE-2"),
                ManagedVersion::new("6.21-GE-1", TagKind::wine(), "lutris-ge-6.21-1-x86_64"),
                ManagedVersion::new("6.16-GE-3-LoL", TagKind::lol(), ""),
            ],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let steam_path = PathBuf::from("test_resources/assets/config.vdf");
        path_cfg
            .expect_steam_config()
            .once()
            .returning(move |_| steam_path.clone());

        let lutris_path = PathBuf::from("test_resources/assets/wine.yml");
        path_cfg
            .expect_lutris_wine_runner_config()
            .once()
            .returning(move |_| lutris_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);
        let mut stdout = AssertLines::new();
        writer.list(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* 6.21-GE-2 - In use by Steam");
        stdout.assert_line(2, "");
        stdout.assert_line(3, "Wine GE:");
        stdout.assert_line(4, "* 6.21-GE-1 - In use by Lutris");
        stdout.assert_line(5, "");
        stdout.assert_line(6, "Wine GE (LoL):");
        stdout.assert_line(7, "* 6.16-GE-3-LoL");
        stdout.assert_line(8, "");
    }

    #[test]
    fn list_all() {
        let args = ListArgs::new(None, false);
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![
                ManagedVersion::new("6.20-GE-2", TagKind::Proton, ""),
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.19-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.20-GE-2", TagKind::wine(), ""),
                ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.19-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.16-GE-3-LoL", TagKind::lol(), ""),
                ManagedVersion::new("6.16-2-GE-LoL", TagKind::lol(), ""),
                ManagedVersion::new("6.16-1-GE-LoL", TagKind::lol(), ""),
            ],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let steam_path = PathBuf::from("test_resources/assets/config.vdf");
        path_cfg
            .expect_steam_config()
            .once()
            .returning(move |_| steam_path.clone());

        let lutris_path = PathBuf::from("test_resources/assets/wine.yml");
        path_cfg
            .expect_lutris_wine_runner_config()
            .once()
            .returning(move |_| lutris_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);
        let mut stdout = AssertLines::new();
        writer.list(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* 6.20-GE-2");
        stdout.assert_line(2, "* 6.20-GE-1");
        stdout.assert_line(3, "* 6.19-GE-1");
        stdout.assert_line(4, "");
        stdout.assert_line(5, "Wine GE:");
        stdout.assert_line(6, "* 6.20-GE-2");
        stdout.assert_line(7, "* 6.20-GE-1");
        stdout.assert_line(8, "* 6.19-GE-1");
        stdout.assert_line(9, "");
        stdout.assert_line(10, "Wine GE (LoL):");
        stdout.assert_line(11, "* 6.16-GE-3-LoL");
        stdout.assert_line(12, "* 6.16-2-GE-LoL");
        stdout.assert_line(13, "* 6.16-1-GE-LoL");
        stdout.assert_line(14, "");
    }

    #[test]
    fn list_all_with_in_use_version() {
        let args = ListArgs::new(None, false);
        let fs_mng = MockFilesystemManager::new();
        let ge_downloader = MockDownloader::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![
                ManagedVersion::new("6.21-GE-2", TagKind::Proton, "Proton-6.21-GE-2"),
                ManagedVersion::new("6.20-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.19-GE-1", TagKind::Proton, ""),
                ManagedVersion::new("6.21-GE-1", TagKind::wine(), "lutris-ge-6.21-1-x86_64"),
                ManagedVersion::new("6.20-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.19-GE-1", TagKind::wine(), ""),
                ManagedVersion::new("6.16-GE-3-LoL", TagKind::lol(), ""),
                ManagedVersion::new("6.16-2-GE-LoL", TagKind::lol(), ""),
                ManagedVersion::new("6.16-1-GE-LoL", TagKind::lol(), ""),
            ],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let steam_path = PathBuf::from("test_resources/assets/config.vdf");
        path_cfg
            .expect_steam_config()
            .once()
            .returning(move |_| steam_path.clone());

        let lutris_path = PathBuf::from("test_resources/assets/wine.yml");
        path_cfg
            .expect_lutris_wine_runner_config()
            .once()
            .returning(move |_| lutris_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);
        let mut stdout = AssertLines::new();
        writer.list(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Proton GE:");
        stdout.assert_line(1, "* 6.21-GE-2 - In use by Steam");
        stdout.assert_line(2, "* 6.20-GE-1");
        stdout.assert_line(3, "* 6.19-GE-1");
        stdout.assert_line(4, "");
        stdout.assert_line(5, "Wine GE:");
        stdout.assert_line(6, "* 6.21-GE-1 - In use by Lutris");
        stdout.assert_line(7, "* 6.20-GE-1");
        stdout.assert_line(8, "* 6.19-GE-1");
        stdout.assert_line(9, "");
        stdout.assert_line(10, "Wine GE (LoL):");
        stdout.assert_line(11, "* 6.16-GE-3-LoL");
        stdout.assert_line(12, "* 6.16-2-GE-LoL");
        stdout.assert_line(13, "* 6.16-1-GE-LoL");
        stdout.assert_line(14, "");
    }

    #[test]
    fn add_successful_output() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = AddArgs::new(tag_arg, true, false);

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

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .times(2)
            .returning(move |_| json_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        writer.add(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Skipping checksum comparison");
        stdout.assert_line(1, "Successfully added version");
    }

    #[test]
    fn add_with_checksum_comparison_successful_output() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = AddArgs::new(tag_arg, false, false);

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

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .times(2)
            .returning(move |_| json_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        writer.add(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Performing checksum comparison: Checksums match");
        stdout.assert_line(1, "Successfully added version");
    }

    #[test]
    fn add_specific_version_which_is_already_managed_again_expect_message_about_already_being_managed() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = AddArgs::new(tag_arg, false, false);

        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_setup_version().never();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "")]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.add(&mut stdout, args);
        assert!(result.is_ok());
        stdout.assert_line(0, "Version 6.20-GE-1 (Proton) is already managed");
    }

    #[test]
    fn add_latest_version_which_is_already_managed_again_expect_message_about_already_being_managed() {
        let tag_arg = TagArg::new(None, TagKind::Proton);
        let args = AddArgs::new(tag_arg, false, false);

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_setup_version().never();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "")]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let mut ge_downloader = MockDownloader::new();
        ge_downloader
            .expect_fetch_release()
            .once()
            .returning(move |_, _| Ok(GeRelease::new(String::from("6.20-GE-1"), Vec::new())));

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.add(&mut stdout, args);
        assert!(result.is_ok());
        stdout.assert_line(0, "Version 6.20-GE-1 (Proton) is already managed");
    }

    #[test]
    fn add_with_apply_should_modify_app_config() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = AddArgs::new(tag_arg, false, true);

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
        fs_mng.expect_apply_to_app_config().once().returning(|_| Ok(()));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .times(2)
            .returning(move |_| json_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        writer.add(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Performing checksum comparison: Checksums match");
        stdout.assert_line(1, "Successfully added version");
        stdout.assert_line(2, "Modifying Steam configuration to use 6.20-GE-1 (Proton)");
        stdout.assert_line(3, PROTON_APPLY_HINT);
    }

    #[test]
    fn remove_not_managed_version() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = RemoveArgs::new(tag_arg);
        let ge_downloader = MockDownloader::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().never();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        path_cfg.expect_steam_config().never();

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);
        let mut stdout = AssertLines::new();

        let result = writer.remove(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Given version is not managed");
        stdout.assert_empty();
    }

    #[test]
    fn remove_existing_version() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = RemoveArgs::new(tag_arg);
        let ge_downloader = MockDownloader::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().once().returning(|_| Ok(()));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![ManagedVersion::new(Tag::from("6.20-GE-1"), TagKind::Proton, "")],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .times(2)
            .returning(move |_| json_path.clone());

        let steam_path = PathBuf::from("test_resources/assets/config.vdf");
        path_cfg
            .expect_steam_config()
            .once()
            .returning(move |_| steam_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);
        let mut stdout = AssertLines::new();
        writer.remove(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Successfully removed version 6.20-GE-1 (Proton).")
    }

    #[test]
    fn remove_version_used_by_app_config() {
        let tag_arg = TagArg::new(Some(Tag::from("6.21-GE-2")), TagKind::Proton);
        let args = RemoveArgs::new(tag_arg);
        let ge_downloader = MockDownloader::new();

        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_remove_version().never();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![ManagedVersion::new(
                Tag::from("6.21-GE-2"),
                TagKind::Proton,
                "Proton-6.21-GE-2",
            )],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let steam_path = PathBuf::from("test_resources/assets/config.vdf");
        path_cfg
            .expect_steam_config()
            .once()
            .returning(move |_| steam_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);
        let mut stdout = AssertLines::new();

        let result = writer.remove(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Proton version is in use by Steam. Select a different version to make removal possible."
        );
        stdout.assert_empty();
    }

    #[test]
    fn check_with_successful_requests() {
        let args = CheckArgs::new(None);

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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        writer.check(&mut stdout, &mut stderr, args);

        stdout.assert_line(0, "These are the latest releases.");
        stdout.assert_line(1, "");
        stdout.assert_line(2, "Proton GE: 6.20-GE-1");
        stdout.assert_line(3, "Wine GE: 6.20-GE-1");
        stdout.assert_line(4, "Wine GE - LoL: 6.16-GE-3-LoL");
    }

    #[test]
    fn check_with_only_errors() {
        let args = CheckArgs::new(None);

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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let mut stderr = AssertLines::new();
        writer.check(&mut stdout, &mut stderr, args);

        stdout.assert_line(0, "These are the latest releases.");
        stderr.assert_line(
            0,
            "Proton GE: Could not fetch release information from GitHub: No tags could be found",
        );
        stderr.assert_line(
            1,
            "Wine GE: Could not fetch release information from GitHub: No tags could be found",
        );
        stderr.assert_line(
            2,
            "Wine GE - LoL: Could not fetch release information from GitHub: No tags could be found",
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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.migrate(&mut stdout, args);
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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        writer.migrate(&mut stdout, args).unwrap();
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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.migrate(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Could not migrate directory");
        stdout.assert_empty();
    }

    #[test]
    fn apply_to_app_config_for_non_existent_version() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = ApplyArgs::new(tag_arg);

        let ge_downloader = MockDownloader::new();
        let fs_mng = MockFilesystemManager::new();

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(&json_path, vec![]);

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.apply_to_app_config(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Given version is not managed");
        stdout.assert_empty();
    }

    #[test]
    fn apply_to_app_config_for_latest_version() {
        let tag_arg = TagArg::new(None, TagKind::Proton);
        let args = ApplyArgs::new(tag_arg);

        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_apply_to_app_config().once().returning(|_| Ok(()));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "Proton-6.20-GE-1")],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        writer.apply_to_app_config(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Modifying Steam configuration to use 6.20-GE-1 (Proton)");
        stdout.assert_line(1, PROTON_APPLY_HINT);
    }

    #[test]
    fn apply_to_app_config_for_existent_version() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = ApplyArgs::new(tag_arg);

        let ge_downloader = MockDownloader::new();
        let mut fs_mng = MockFilesystemManager::new();
        fs_mng.expect_apply_to_app_config().once().returning(|_| Ok(()));

        let tmp_dir = TempDir::new().unwrap();
        let json_path = tmp_dir.join("ge_man/managed_versions.json");
        setup_managed_versions(
            &json_path,
            vec![ManagedVersion::new("6.20-GE-1", TagKind::Proton, "Proton-6.20-GE-1")],
        );

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        writer.apply_to_app_config(&mut stdout, args).unwrap();

        stdout.assert_line(0, "Modifying Steam configuration to use 6.20-GE-1 (Proton)");
        stdout.assert_line(1, PROTON_APPLY_HINT);
    }

    #[test]
    fn apply_to_app_config_fails_with_an_error() {
        let tag_arg = TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton);
        let args = ApplyArgs::new(tag_arg);

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

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_managed_versions_config()
            .once()
            .returning(move |_| json_path.clone());

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.apply_to_app_config(&mut stdout, args);
        assert!(result.is_err());

        stdout.assert_line(0, "Modifying Steam configuration to use 6.20-GE-1 (Proton)");
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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.copy_user_settings(&mut stdout, args);
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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.copy_user_settings(&mut stdout, args);
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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        writer.copy_user_settings(&mut stdout, args).unwrap();

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

        let writer = TerminalWriter::new(&ge_downloader, &fs_mng, &path_cfg);

        let mut stdout = AssertLines::new();
        let result = writer.copy_user_settings(&mut stdout, args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Mocked error");
        stdout.assert_empty();
    }
}
