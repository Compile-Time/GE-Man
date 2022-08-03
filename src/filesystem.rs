use std::ffi::OsStr;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{anyhow, bail, Context};
use ge_man_lib::archive;
use ge_man_lib::config::{LutrisConfig, SteamConfig};
use ge_man_lib::tag::TagKind;
use log::debug;
#[cfg(test)]
use mockall::{automock, predicate::*};

use crate::compat_tool_app::ApplicationConfig;
use crate::data::ManagedVersion;
use crate::path::{overrule, PathConfiguration, LUTRIS_WINE_RUNNERS_DIR, STEAM_COMP_DIR};
use crate::version::Version;

const USER_SETTINGS_PY: &str = "user_settings.py";
const LUTRIS_INITIAL_WINE_RUNNER_CONFIG: &str = r#"
wine:
  version: VERSION
"#;

pub fn in_use_compat_tool_dir_name(config_file_path: &Path, kind: TagKind) -> anyhow::Result<String> {
    debug!(
        "Reading currently used compatibility tool version from the following config file: {}",
        config_file_path.display()
    );
    let config = ApplicationConfig::create_copy(kind, config_file_path)?;
    Ok(config.version_dir_name().clone())
}

#[cfg_attr(test, automock)]
pub trait FilesystemManager {
    fn setup_version(&self, version: Version, compressed_tar: Box<dyn Read>) -> anyhow::Result<ManagedVersion>;
    fn remove_version(&self, version: &ManagedVersion) -> anyhow::Result<()>;
    fn paths_for_directory_items(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn migrate_folder(&self, version: Version, source_path: &Path) -> anyhow::Result<ManagedVersion>;
    fn apply_to_app_config(&self, version: &ManagedVersion) -> anyhow::Result<()>;
    fn copy_user_settings(&self, src_version: &ManagedVersion, dst_version: &ManagedVersion) -> anyhow::Result<()>;
}

/// Inside this struct it is assumed that all data passed to the methods of this struct contain valid data which
/// passed clap's or the ui module's validations.
pub struct FsMng<'a> {
    path_config: &'a dyn PathConfiguration,
}

impl<'a> FsMng<'a> {
    pub fn new(path_config: &'a dyn PathConfiguration) -> Self {
        FsMng { path_config }
    }

    fn copy_directory(&self, src: &Path, dst: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(dst).unwrap();
        for entry in src.read_dir()? {
            let dir_entry = entry?;
            let dst = dst.join(dir_entry.file_name());

            if dir_entry.path().is_dir() {
                self.copy_directory(&dir_entry.path(), &dst)?;
            } else {
                fs::copy(&dir_entry.path(), &dst)?;
            }
        }

        Ok(())
    }

    fn move_or_copy_directory(&self, version: &ManagedVersion, src_path: &Path) -> anyhow::Result<()> {
        let dst_path = match version.kind() {
            TagKind::Proton => self.path_config.steam_compatibility_tools_dir(overrule::steam_root()),
            TagKind::Wine { .. } => self.path_config.lutris_runners_dir(overrule::xdg_data_home()),
        };
        let dst_path = dst_path.join(version.directory_name());

        // A rename is used here to move the directory into the destination folder. We could just copy the files but
        // Proton GE releases tend to be 400 MB in size and Wine GE releases about 100 MB.
        if let Err(err) = fs::rename(src_path, &dst_path) {
            match err.raw_os_error() {
                // Rename only works when the source and destination are on the same device. In the case that the
                // destination is a different device the source must be copied to the destination.
                Some(18) => {
                    self.copy_directory(src_path, &dst_path).context(format!(
                        "Failed to copy source to destination.\n\
                                         Source: {}\n\
                                         Destination: {}\n",
                        src_path.display(),
                        dst_path.display(),
                    ))?;
                }
                _ => bail!(err),
            }
        }

        Ok(())
    }
}

impl<'a> FilesystemManager for FsMng<'a> {
    fn setup_version(&self, version: Version, compressed_tar: Box<dyn Read>) -> anyhow::Result<ManagedVersion> {
        let mut version = ManagedVersion::try_from(version)?;

        let local_state_path = self.path_config.xdg_state_dir(overrule::xdg_state_home());
        let extracted_location = archive::extract_compressed(version.kind(), compressed_tar, &local_state_path)
            .context("Failed to extract compressed archive")?;

        let app_tool_path = match version.kind() {
            TagKind::Proton => self.path_config.steam_compatibility_tools_dir(overrule::steam_root()),
            TagKind::Wine { .. } => self.path_config.lutris_runners_dir(overrule::xdg_data_home()),
        };

        let mut directory_name = extracted_location
            .file_name()
            .map(OsStr::to_string_lossy)
            .map(|string| string.to_string())
            .unwrap();
        directory_name.push_str(&format!("_{}", version.label()));
        version.set_directory_name(directory_name.clone());

        // Move extracted archive to correct location.
        fs::rename(extracted_location, app_tool_path.join(directory_name))?;
        Ok(version)
    }

    fn remove_version(&self, version: &ManagedVersion) -> anyhow::Result<()> {
        let path = match version.kind() {
            TagKind::Proton => self.path_config.steam_compatibility_tools_dir(overrule::steam_root()),
            TagKind::Wine { .. } => self.path_config.lutris_runners_dir(overrule::xdg_data_home()),
        };
        let path = path.join(version.directory_name());

        fs::remove_dir_all(&path).context(format!("Could not remove directory '{}'", path.display()))
    }

    fn paths_for_directory_items(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        fs::read_dir(path)?
            .into_iter()
            .map(|result| result.map(|dir_entry| dir_entry.path()))
            .collect::<io::Result<Vec<PathBuf>>>()
    }

    fn migrate_folder(&self, version: Version, source_path: &Path) -> anyhow::Result<ManagedVersion> {
        let mut managed_version = ManagedVersion::try_from(version)?;
        let dir_name = format!(
            "GE-MAN_{}_{}_L{}",
            managed_version.kind(),
            managed_version.tag(),
            managed_version.label(),
        );
        managed_version.set_directory_name(dir_name);

        match source_path.parent() {
            Some(parent) => {
                if parent.ends_with(STEAM_COMP_DIR) || parent.ends_with(LUTRIS_WINE_RUNNERS_DIR) {
                    managed_version
                        .set_directory_name(String::from_utf8_lossy(source_path.file_name().unwrap().as_bytes()));
                    return Ok(managed_version);
                } else {
                    self.move_or_copy_directory(&managed_version, source_path)?;
                }
            }
            None => self.move_or_copy_directory(&managed_version, source_path)?,
        }

        Ok(managed_version)
    }

    fn apply_to_app_config(&self, version: &ManagedVersion) -> anyhow::Result<()> {
        // TODO: Improve this with ApplicationConfig struct abstraction.
        match version.kind() {
            TagKind::Proton => {
                let steam_cfg_path = self.path_config.steam_config(overrule::steam_root());
                let backup_path = self
                    .path_config
                    .app_config_backup_file(overrule::xdg_config_home(), version.kind());

                fs::copy(&steam_cfg_path, &backup_path).context(format!(
                    r#"Could not create backup of Steam config from "{}" to "{}" "#,
                    steam_cfg_path.display(),
                    backup_path.display()
                ))?;

                let mut config = SteamConfig::create_copy(&steam_cfg_path)?;
                config.set_proton_version(version.directory_name());

                let new_config: Vec<u8> = config.into();
                fs::write(steam_cfg_path, new_config)?;
            }
            TagKind::Wine { .. } => {
                let runner_cfg_path = self.path_config.lutris_wine_runner_config(overrule::xdg_config_home());
                let backup_path = self
                    .path_config
                    .app_config_backup_file(overrule::xdg_config_home(), version.kind());

                let copy_result = fs::copy(&runner_cfg_path, &backup_path);

                if let Err(io_err) = copy_result {
                    if let io::ErrorKind::NotFound = io_err.kind() {
                        fs::write(
                            runner_cfg_path,
                            LUTRIS_INITIAL_WINE_RUNNER_CONFIG.replace("VERSION", version.directory_name()),
                        )
                        .context("Failed to create initial Wine runner configuration for Lutris")?;
                    } else {
                        return Err(anyhow!(io_err)).context(format!(
                            r#"Could not create backup of Wine runner config from "{}" to "{}""#,
                            runner_cfg_path.display(),
                            backup_path.display()
                        ));
                    }
                } else {
                    let mut config = LutrisConfig::create_copy(&runner_cfg_path)?;
                    config.set_wine_version(version.directory_name());

                    let new_config: Vec<u8> = config.into();
                    fs::write(runner_cfg_path, new_config)?;
                };
            }
        }

        Ok(())
    }

    fn copy_user_settings(&self, src_version: &ManagedVersion, dst_version: &ManagedVersion) -> anyhow::Result<()> {
        let src_path = self
            .path_config
            .steam_compatibility_tools_dir(overrule::steam_root())
            .join(src_version.directory_name())
            .join(USER_SETTINGS_PY);
        let dst_path = self
            .path_config
            .steam_compatibility_tools_dir(overrule::steam_root())
            .join(dst_version.directory_name())
            .join(USER_SETTINGS_PY);
        debug!(
            "Copying user-settings.py from {} to {}",
            src_path.display(),
            dst_path.display()
        );

        fs::copy(src_path, dst_path).context(format!(
            "Could not copy user_settings.py from {} to {}",
            src_version, dst_version
        ))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use std::path::PathBuf;

    use crate::fixture;
    use crate::version::Versioned;
    use assert_fs::prelude::{PathAssert, PathChild};
    use assert_fs::TempDir;
    use ge_man_lib::tag::Tag;

    use super::*;

    struct MockPathConfig {
        pub tmp_dir: PathBuf,
    }

    impl MockPathConfig {
        pub fn new(tmp_dir: PathBuf) -> Self {
            MockPathConfig { tmp_dir }
        }
    }

    impl PathConfiguration for MockPathConfig {
        fn xdg_data_dir(&self, _xdg_data_path: Option<PathBuf>) -> PathBuf {
            self.tmp_dir.join(".local/share")
        }

        fn xdg_config_dir(&self, _xdg_config_path: Option<PathBuf>) -> PathBuf {
            self.tmp_dir.join(".config")
        }

        fn steam(&self, _steam_root_path_override: Option<PathBuf>) -> PathBuf {
            self.tmp_dir.join(".steam/root")
        }
    }

    #[test]
    fn setup_proton_version() {
        let dir_name = "Proton-6.20-GE-1";

        let tmp_dir = TempDir::new().unwrap();
        let path_config = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(path_config.steam_compatibility_tools_dir(None)).unwrap();

        let fs_manager = FsMng::new(&path_config);

        let compressed_tar = BufReader::new(File::open("test_resources/assets/Proton-6.20-GE-1.tar.gz").unwrap());
        let version = fixture::version::v6_20_1_proton();
        let managed_version = fs_manager
            .setup_version(version.clone(), Box::new(compressed_tar))
            .unwrap();

        assert_eq!(managed_version.tag(), version.tag());
        assert_eq!(managed_version.kind(), version.kind());
        assert_eq!(managed_version.directory_name(), dir_name);
        tmp_dir
            .child(".steam/root/compatibilitytools.d")
            .child(&dir_name)
            .assert(path::exists());

        drop(fs_manager);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn setup_wine_version() {
        let dir_name = "Wine-6.20-GE-1";

        let tmp_dir = TempDir::new().unwrap();
        let path_config = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(path_config.lutris_runners_dir(None)).unwrap();

        let fs_manager = FsMng::new(&path_config);

        let compressed_tar = BufReader::new(File::open("test_resources/assets/Wine-6.20-GE-1.tar.xz").unwrap());
        let version = fixture::version::v6_20_1_wine();
        let managed_version = fs_manager
            .setup_version(version.clone(), Box::new(compressed_tar))
            .unwrap();

        assert_eq!(managed_version.tag(), version.tag());
        assert_eq!(managed_version.kind(), version.kind());
        assert_eq!(managed_version.directory_name(), dir_name);
        tmp_dir
            .child(".local/share/lutris/runners/wine")
            .child(&dir_name)
            .assert(path::exists());

        drop(fs_manager);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn setup_wine_lol_version() {
        let dir_name = "Wine-6.20-GE-1-LoL";

        let tmp_dir = TempDir::new().unwrap();
        let path_config = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(path_config.lutris_runners_dir(None)).unwrap();

        let fs_manager = FsMng::new(&path_config);

        let compressed_tar = BufReader::new(File::open("test_resources/assets/Wine-6.20-GE-1-LoL.tar.xz").unwrap());
        let version = fixture::version::v6_20_1_lol();
        let managed_version = fs_manager
            .setup_version(version.clone(), Box::new(compressed_tar))
            .unwrap();

        assert_eq!(managed_version.tag(), version.tag());
        assert_eq!(managed_version.kind(), version.kind());
        assert_eq!(managed_version.directory_name(), dir_name);
        tmp_dir
            .child(".local/share/lutris/runners/wine")
            .child(&dir_name)
            .assert(path::exists());

        drop(fs_manager);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn remove_proton_version() {
        let version = fixture::managed_version::v6_20_1_proton();

        let tmp_dir = TempDir::new().unwrap();
        let path_config = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(
            path_config
                .steam_compatibility_tools_dir(None)
                .join(version.directory_name()),
        )
        .unwrap();

        let fs_manager = FsMng::new(&path_config);
        fs_manager.remove_version(&version).unwrap();

        tmp_dir
            .child(".local/share/game-compatibility-manager/versions/proton-ge")
            .child(version.directory_name())
            .assert(path::missing());

        drop(fs_manager);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn remove_wine_version() {
        let version = fixture::managed_version::v6_20_1_wine();

        let tmp_dir = TempDir::new().unwrap();
        let path_config = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(path_config.lutris_runners_dir(None).join(version.directory_name())).unwrap();

        let fs_manager = FsMng::new(&path_config);
        fs_manager.remove_version(&version).unwrap();

        tmp_dir
            .child(".local/share/game-compatibility-manager/versions/wine-ge")
            .child(version.directory_name())
            .assert(path::missing());

        drop(fs_manager);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn remove_wine_lol_version() {
        let version = fixture::managed_version::v6_20_1_wine();

        let tmp_dir = TempDir::new().unwrap();
        let path_config = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(path_config.lutris_runners_dir(None).join(version.directory_name())).unwrap();

        let fs_manager = FsMng::new(&path_config);
        fs_manager.remove_version(&version).unwrap();

        tmp_dir
            .child(".local/share/game-compatibility-manager/versions/wine-ge")
            .child(version.directory_name())
            .assert(path::missing());

        drop(fs_manager);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn migrate_proton_version_in_steam_dir() {
        let tmp_dir = TempDir::new().unwrap();
        let version = fixture::version::v6_20_1_proton();
        let source_path = tmp_dir.join(".local/share/Steam/compatibilitytools.d/Proton-6.20-GE-1");
        fs::create_dir_all(&source_path).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        let fs_mng = FsMng::new(&path_cfg);

        let version = fs_mng.migrate_folder(version, &source_path).unwrap();
        assert_eq!(version.tag(), &Tag::from("6.20-GE-1"));
        assert_eq!(version.kind(), &TagKind::Proton);
        assert_eq!(version.directory_name(), &String::from("Proton-6.20-GE-1"));

        tmp_dir
            .child(".local/share/Steam/compatibilitytools.d/GE-MAN_PROTON_6.20-GE-1_L6.20-GE-1")
            .assert(path::missing());
        tmp_dir
            .child(".local/share/Steam/compatibilitytools.d/Proton-6.20-GE-1")
            .assert(path::exists());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn migrate_proton_version_present_in_random_directory() {
        let tmp_dir = TempDir::new().unwrap();
        let source_path = tmp_dir.join("some/dir/Proton-6.20-GE-1");
        let version = fixture::version::v6_20_1_proton();
        fs::create_dir_all(&source_path).unwrap();
        fs::create_dir_all(tmp_dir.join(".steam/root/compatibilitytools.d")).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        let fs_mng = FsMng::new(&path_cfg);

        let managed_version = fs_mng.migrate_folder(version.clone(), &source_path).unwrap();
        assert_eq!(managed_version.tag(), version.tag());
        assert_eq!(managed_version.kind(), version.kind());
        assert_eq!(managed_version.directory_name(), "GE-MAN_PROTON_6.20-GE-1_L6.20-GE-1");

        tmp_dir
            .child(".steam/root/compatibilitytools.d/Proton-6.20-GE-1")
            .assert(path::missing());
        tmp_dir
            .child(".steam/root/compatibilitytools.d/GE-MAN_PROTON_6.20-GE-1_L6.20-GE-1")
            .assert(path::exists());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn migrate_wine_version_in_lutris_directory() {
        let tmp_dir = TempDir::new().unwrap();
        let source_path = tmp_dir.join(".local/share/lutris/runners/wine/Wine-6.20-GE-1");
        let version = fixture::version::v6_20_1_wine();
        fs::create_dir_all(&source_path).unwrap();
        fs::create_dir_all(tmp_dir.join(".local/share/lutris/runners/wine")).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        let fs_mng = FsMng::new(&path_cfg);

        let version = fs_mng.migrate_folder(version, &source_path).unwrap();
        assert_eq!(version.tag(), &Tag::from("6.20-GE-1"));
        assert_eq!(version.kind(), &TagKind::wine());
        assert_eq!(version.directory_name(), &String::from("Wine-6.20-GE-1"));

        tmp_dir
            .child(".local/share/lutris/runners/wine/GE-MAN_Wine_6.20-GE-1_L6.20-GE-1")
            .assert(path::missing());
        tmp_dir
            .child(".local/share/lutris/runners/wine/Wine-6.20-GE-1")
            .assert(path::exists());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn migrate_wine_version_in_random_directory() {
        let tmp_dir = TempDir::new().unwrap();
        let source_path = tmp_dir.join("some/dir/Wine-6.20-GE-1");
        let version = fixture::version::v6_20_1_wine();
        fs::create_dir_all(&source_path).unwrap();
        fs::create_dir_all(tmp_dir.join(".local/share/lutris/runners/wine")).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        let fs_mng = FsMng::new(&path_cfg);

        let managed_version = fs_mng.migrate_folder(version.clone(), &source_path).unwrap();
        assert_eq!(managed_version.tag(), version.tag());
        assert_eq!(managed_version.kind(), version.kind());
        assert_eq!(managed_version.directory_name(), "GE-MAN_WINE_6.20-GE-1_L6.20-GE-1");

        tmp_dir
            .child(".local/share/lutris/runners/wine/Wine-6.20-GE-1")
            .assert(path::missing());
        tmp_dir
            .child(".local/share/lutris/runners/wine/GE-MAN_WINE_6.20-GE-1_L6.20-GE-1")
            .assert(path::exists());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn migrate_lol_version_in_lutris_directory() {
        let tmp_dir = TempDir::new().unwrap();
        let source_path = PathBuf::from(tmp_dir.join(".local/share/lutris/runners/wine/Wine-LoL-6.20-GE-1"));
        let version = fixture::version::v6_20_1_lol();
        fs::create_dir_all(&source_path).unwrap();
        fs::create_dir_all(tmp_dir.join(".local/share/lutris/runners/wine")).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        let fs_mng = FsMng::new(&path_cfg);

        let version = fs_mng.migrate_folder(version, &source_path).unwrap();
        assert_eq!(version.tag(), &Tag::from("6.20-GE-1"));
        assert_eq!(version.kind(), &TagKind::lol());
        assert_eq!(version.directory_name(), &String::from("Wine-LoL-6.20-GE-1"));

        tmp_dir
            .child(".local/share/lutris/runners/wine/GEH_LoL_Wine_6.20-GE-1")
            .assert(path::missing());
        tmp_dir
            .child(".local/share/lutris/runners/wine/Wine-LoL-6.20-GE-1")
            .assert(path::exists());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn migrate_lol_version_in_random_directory() {
        let tmp_dir = TempDir::new().unwrap();
        let source_path = PathBuf::from(tmp_dir.join("some/dir/Wine-LoL-6.20-GE-1"));
        let version = fixture::version::v6_20_1_lol();
        fs::create_dir_all(&source_path).unwrap();
        fs::create_dir_all(tmp_dir.join(".local/share/lutris/runners/wine")).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        let fs_mng = FsMng::new(&path_cfg);

        let managed_version = fs_mng.migrate_folder(version.clone(), &source_path).unwrap();
        assert_eq!(managed_version.tag(), version.tag());
        assert_eq!(managed_version.kind(), version.kind());
        assert_eq!(managed_version.directory_name(), "GE-MAN_LOL_WINE_6.20-GE-1_L6.20-GE-1");

        tmp_dir
            .child(".local/share/lutris/runners/wine/Wine-LoL-6.20-GE-1")
            .assert(path::missing());
        tmp_dir
            .child(".local/share/lutris/runners/wine/GE-MAN_LOL_WINE_6.20-GE-1_L6.20-GE-1")
            .assert(path::exists());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn apply_proton_ge_version_to_steam_config() {
        let tmp_dir = TempDir::new().unwrap();
        let steam_cfg_dir = tmp_dir.join(".steam/root/config");
        let steam_cfg_file = steam_cfg_dir.join("config.vdf");
        fs::create_dir_all(&steam_cfg_dir).unwrap();
        fs::copy("test_resources/assets/config.vdf", &steam_cfg_file).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(
            path_cfg
                .app_config_backup_file(None, &TagKind::Proton)
                .parent()
                .unwrap(),
        )
        .unwrap();
        let fs_mng = FsMng::new(&path_cfg);

        let managed_version = fixture::managed_version::v6_20_1_proton();
        fs_mng.apply_to_app_config(&managed_version).unwrap();

        let modified_config = SteamConfig::create_copy(&steam_cfg_file).unwrap();
        assert_eq!(modified_config.proton_version(), managed_version.directory_name());

        tmp_dir
            .child(path_cfg.app_config_backup_file(None, &TagKind::Proton))
            .assert(path::exists());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn apply_wine_ge_version_to_lutris_config_when_runner_config_already_exists() {
        let tmp_dir = TempDir::new().unwrap();
        let cfg_dir = tmp_dir.join(".config/lutris/runners");
        let cfg_file = cfg_dir.join("wine.yml");
        fs::create_dir_all(&cfg_dir).unwrap();
        fs::copy("test_resources/assets/wine.yml", &cfg_file).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(
            path_cfg
                .app_config_backup_file(None, &TagKind::wine())
                .parent()
                .unwrap(),
        )
        .unwrap();
        let fs_mng = FsMng::new(&path_cfg);

        let managed_version = fixture::managed_version::v6_20_1_wine();
        fs_mng.apply_to_app_config(&managed_version).unwrap();

        let modified_config = LutrisConfig::create_copy(&cfg_file).unwrap();
        assert_eq!(modified_config.wine_version(), managed_version.directory_name());

        tmp_dir
            .child(path_cfg.app_config_backup_file(None, &TagKind::wine()))
            .assert(path::exists());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn apply_wine_ge_version_to_lutris_config_when_no_runner_config_exists() {
        let tmp_dir = TempDir::new().unwrap();
        let cfg_dir = tmp_dir.join(".config/lutris/runners");
        let cfg_file = cfg_dir.join("wine.yml");
        fs::create_dir_all(&cfg_dir).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        fs::create_dir_all(
            path_cfg
                .app_config_backup_file(None, &TagKind::wine())
                .parent()
                .unwrap(),
        )
        .unwrap();
        let fs_mng = FsMng::new(&path_cfg);

        let managed_version = fixture::managed_version::v6_21_1_wine();
        fs_mng.apply_to_app_config(&managed_version).unwrap();

        let modified_config = LutrisConfig::create_copy(&cfg_file).unwrap();
        assert_eq!(modified_config.wine_version(), managed_version.directory_name());

        tmp_dir
            .child(path_cfg.app_config_backup_file(None, &TagKind::wine()))
            .assert(path::missing());

        drop(fs_mng);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn copy_proton_settings() {
        let tmp_dir = TempDir::new().unwrap();
        fs::create_dir_all(tmp_dir.join(".steam/root/compatibilitytools.d")).unwrap();
        fs::create_dir_all(tmp_dir.join(".steam/root/config")).unwrap();

        let path_cfg = MockPathConfig::new(PathBuf::from(tmp_dir.path()));
        let fs_mng = FsMng::new(&path_cfg);

        let src_version = fixture::version::v6_19_1_proton();
        let src_tar = File::open("test_resources/assets/Proton-6.19-GE-1.tar.gz").unwrap();
        let dst_version = fixture::version::v6_20_2_proton();
        let dst_tar = File::open("test_resources/assets/Proton-6.20-GE-2.tar.gz").unwrap();

        let src_managed_version = fs_mng.setup_version(src_version, Box::new(src_tar)).unwrap();
        let dst_managed_version = fs_mng.setup_version(dst_version, Box::new(dst_tar)).unwrap();

        tmp_dir
            .child(".steam/root/compatibilitytools.d/Proton-6.19-GE-1")
            .assert(path::exists());
        tmp_dir
            .child(".steam/root/compatibilitytools.d/Proton-6.20-GE-2")
            .assert(path::exists());

        fs::copy(
            tmp_dir.join(".steam/root/compatibilitytools.d/Proton-6.19-GE-1/hello-world.txt"),
            tmp_dir.join(".steam/root/compatibilitytools.d/Proton-6.19-GE-1/user_settings.py"),
        )
        .unwrap();

        tmp_dir
            .child(".steam/root/compatibilitytools.d/Proton-6.19-GE-1/user_settings.py")
            .assert(path::exists());

        fs_mng
            .copy_user_settings(&src_managed_version, &dst_managed_version)
            .unwrap();

        tmp_dir
            .child(".steam/root/compatibilitytools.d/Proton-6.20-GE-2/user_settings.py")
            .assert(path::exists());

        tmp_dir.close().unwrap();
    }
}
