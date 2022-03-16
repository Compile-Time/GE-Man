use std::path::PathBuf;
use std::{env, fs};

use anyhow::Context;
use gehelper_lib::tag::TagKind;
#[cfg(test)]
use mockall::automock;

pub const STEAM_COMP_DIR: &str = "Steam/compatibilitytools.d";
pub const LUTRIS_WINE_RUNNERS_DIR: &str = "lutris/runners/wine";

const HOME: &str = "HOME";
const XDG_DATA_HOME: &str = "XDG_DATA_HOME";
const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
const STEAM_PATH_ENV: &str = "STEAM_PATH";

const APP_NAME: &str = "gehelper";

pub fn create_xdg_directories(path_cfg: &impl PathConfiguration) -> anyhow::Result<()> {
    fs::create_dir_all(path_cfg.gehelper_config_dir(env::var(XDG_CONFIG_HOME).ok()))
        .context("Failed to setup config directory in XDG_CONFIG_HOME")?;
    fs::create_dir_all(path_cfg.gehelper_data_dir(env::var(XDG_DATA_HOME).ok()))
        .context("Failed to setup data directory in XDG_DATA_HOME")?;

    Ok(())
}

pub fn xdg_data_home() -> Option<String> {
    env::var(XDG_DATA_HOME).ok()
}

pub fn xdg_config_home() -> Option<String> {
    env::var(XDG_CONFIG_HOME).ok()
}

pub fn steam_path() -> Option<String> {
    env::var(STEAM_PATH_ENV).ok()
}

#[cfg_attr(test, automock)]
pub trait PathConfiguration {
    fn xdg_data_dir(&self, xdg_data_path: Option<String>) -> PathBuf {
        let data_dir = xdg_data_path
            .or_else(|| env::var(HOME).ok().map(|home| format!("{}/.local/share", home)))
            .unwrap();

        PathBuf::from(data_dir)
    }

    fn xdg_config_dir(&self, xdg_config_path: Option<String>) -> PathBuf {
        let config_dir = xdg_config_path
            .or_else(|| env::var(HOME).ok().map(|home| format!("{}/.config", home)))
            .unwrap();

        PathBuf::from(config_dir)
    }

    fn steam(&self, xdg_data_path: Option<String>, steam_path: Option<String>) -> PathBuf {
        steam_path
            .map(PathBuf::from)
            .unwrap_or_else(|| self.xdg_data_dir(xdg_data_path).join("Steam"))
    }

    fn lutris_local(&self, xdg_data_path: Option<String>) -> PathBuf {
        self.xdg_data_dir(xdg_data_path).join("lutris")
    }

    fn lutris_config(&self, xdg_config_path: Option<String>) -> PathBuf {
        self.xdg_config_dir(xdg_config_path).join("lutris")
    }

    fn steam_config(&self, xdg_data_path: Option<String>, steam_path: Option<String>) -> PathBuf {
        self.steam(xdg_data_path, steam_path).join("config/config.vdf")
    }

    fn steam_compatibility_tools_dir(&self, xdg_data_path: Option<String>, steam_path: Option<String>) -> PathBuf {
        self.steam(xdg_data_path, steam_path).join("compatibilitytools.d")
    }

    fn lutris_wine_config(&self, xdg_config_path: Option<String>) -> PathBuf {
        self.lutris_config(xdg_config_path).join("runners/wine.yml")
    }

    fn lutris_runners_dir(&self, xdg_data_path: Option<String>) -> PathBuf {
        self.lutris_local(xdg_data_path).join("runners/wine")
    }

    fn gehelper_data_dir(&self, xdg_data_path: Option<String>) -> PathBuf {
        self.xdg_data_dir(xdg_data_path).join(APP_NAME)
    }

    fn gehelper_config_dir(&self, xdg_config_path: Option<String>) -> PathBuf {
        self.xdg_config_dir(xdg_config_path).join(APP_NAME)
    }

    fn managed_versions_config(&self, xdg_data_path: Option<String>) -> PathBuf {
        self.gehelper_data_dir(xdg_data_path).join("managed_versions.json")
    }

    fn app_config_backup_file(&self, xdg_config_path: Option<String>, kind: &TagKind) -> PathBuf {
        let config_file = match kind {
            TagKind::Proton => "steam-config-backup.vdf",
            TagKind::Wine { .. } => "lutris-wine-runner-config-backup.yml",
        };
        self.gehelper_config_dir(xdg_config_path).join(config_file)
    }
}

pub struct AppConfigPaths {
    pub steam: PathBuf,
    pub lutris: PathBuf,
}

impl AppConfigPaths {
    pub fn new<P: Into<PathBuf>>(steam: P, lutris: P) -> Self {
        AppConfigPaths {
            steam: steam.into(),
            lutris: lutris.into(),
        }
    }
}

impl<T: PathConfiguration> From<&T> for AppConfigPaths {
    fn from(path_cfg: &T) -> Self {
        AppConfigPaths::new(
            path_cfg.steam_config(xdg_data_home(), steam_path()),
            path_cfg.lutris_wine_config(xdg_config_home()),
        )
    }
}

pub struct PathConfig {}

impl PathConfig {
    pub fn new() -> Self {
        PathConfig {}
    }
}

impl Default for PathConfig {
    fn default() -> Self {
        PathConfig::new()
    }
}

impl PathConfiguration for PathConfig {}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use assert_fs::prelude::{PathAssert, PathChild};
    use assert_fs::TempDir;
    use gehelper_lib::tag::TagKind;

    use super::*;

    #[test]
    fn get_xdg_data_path_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.xdg_data_dir(None);
        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".local/share"));
    }

    #[test]
    fn get_xdg_data_path_with_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.xdg_data_dir(Some(String::from("/tmp/xdg-data")));
        assert_eq!(path, PathBuf::from("/tmp/xdg-data/"));
    }

    #[test]
    fn steam_path_with_no_overrides() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam(None, None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".local/share/Steam"));
    }

    #[test]
    fn steam_path_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam(Some(String::from("/tmp/xdg-data")), None);

        assert_eq!(path, PathBuf::from("/tmp/xdg-data/Steam"));
    }

    #[test]
    fn steam_path_with_steam_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam(None, Some(String::from("/tmp/steam")));

        assert_eq!(path, PathBuf::from("/tmp/steam"));
    }

    #[test]
    fn lutris_local_path_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_local(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".local/share/lutris"));
    }

    #[test]
    fn lutris_local_path_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_local(Some(String::from("/tmp/xdg-data")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-data/lutris"));
    }

    #[test]
    fn lutris_config_path_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_config(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".config/lutris"));
    }

    #[test]
    fn lutris_config_path_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_config(Some(String::from("/tmp/xdg-config")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/lutris"));
    }

    #[test]
    fn steam_config_with_no_overrides() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_config(None, None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".local/share/Steam/config/config.vdf"));
    }

    #[test]
    fn steam_config_with_steam_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_config(None, Some(String::from("/tmp/steam")));

        assert_eq!(path, PathBuf::from("/tmp/steam/config/config.vdf"));
    }

    #[test]
    fn steam_config_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_config(Some(String::from("/tmp/xdg-data")), None);

        assert_eq!(path, PathBuf::from("/tmp/xdg-data/Steam/config/config.vdf"));
    }

    #[test]
    fn steam_compatibilitytools_with_no_overrides() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_compatibility_tools_dir(None, None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path
            .to_string_lossy()
            .contains(".local/share/Steam/compatibilitytools.d"));
    }

    #[test]
    fn steam_compatibilitytools_with_steam_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_compatibility_tools_dir(None, Some(String::from("/tmp/steam")));

        assert_eq!(path, PathBuf::from("/tmp/steam/compatibilitytools.d"));
    }

    #[test]
    fn steam_compatibilitytools_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_compatibility_tools_dir(Some(String::from("/tmp/xdg-data")), None);

        assert_eq!(path, PathBuf::from("/tmp/xdg-data/Steam/compatibilitytools.d"));
    }

    #[test]
    fn lutris_wine_config_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_wine_config(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".config/lutris/runners/wine.yml"));
    }

    #[test]
    fn lutris_wine_config_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_wine_config(Some(String::from("/tmp/xdg-config")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/lutris/runners/wine.yml"));
    }

    #[test]
    fn lutris_runners_dir_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_runners_dir(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".local/share/lutris/runners/wine"));
    }

    #[test]
    fn lutris_runners_dir_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_runners_dir(Some(String::from("/tmp/xdg-data")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-data/lutris/runners/wine"));
    }

    #[test]
    fn gehelper_data_dir_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.gehelper_data_dir(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".local/share/gehelper"));
    }

    #[test]
    fn gehelper_data_dir_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.gehelper_data_dir(Some(String::from("/tmp/xdg-data")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-data/gehelper"));
    }

    #[test]
    fn gehelper_config_dir_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.gehelper_config_dir(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".config/gehelper"));
    }

    #[test]
    fn gehelper_config_dir_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.gehelper_config_dir(Some(String::from("/tmp/xdg-config")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/gehelper"));
    }

    #[test]
    fn gehelper_managed_versions_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.managed_versions_config(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path
            .to_string_lossy()
            .contains(".local/share/gehelper/managed_versions.json"));
    }

    #[test]
    fn gehelper_managed_versions_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.managed_versions_config(Some(String::from("/tmp/xdg-config")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/gehelper/managed_versions.json"));
    }

    #[test]
    fn gehelper_backup_file_for_steam_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.app_config_backup_file(None, &TagKind::Proton);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path
            .to_string_lossy()
            .contains(".config/gehelper/steam-config-backup.vdf"));
    }

    #[test]
    fn gehelper_backup_file_for_steam_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.app_config_backup_file(Some(String::from("/tmp/xdg-config")), &TagKind::Proton);

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/gehelper/steam-config-backup.vdf"));
    }

    #[test]
    fn gehelper_backup_file_for_lutris_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.app_config_backup_file(None, &TagKind::wine());

        assert!(path.to_string_lossy().contains("home"));
        assert!(path
            .to_string_lossy()
            .contains(".config/gehelper/lutris-wine-runner-config-backup.yml"));
    }

    #[test]
    fn gehelper_backup_file_for_lutris_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.app_config_backup_file(Some(String::from("/tmp/xdg-config")), &TagKind::wine());

        assert_eq!(
            path,
            PathBuf::from("/tmp/xdg-config/gehelper/lutris-wine-runner-config-backup.yml")
        );
    }

    #[test]
    fn create_paths_should_create_application_directories() {
        let tmp_dir = TempDir::new().unwrap();
        let config_dir = tmp_dir.join("config");
        let data_dir = tmp_dir.join("data");

        let mut path_cfg = MockPathConfiguration::new();
        path_cfg
            .expect_gehelper_config_dir()
            .once()
            .returning(move |_| config_dir.clone());
        path_cfg
            .expect_gehelper_data_dir()
            .once()
            .returning(move |_| data_dir.clone());

        create_xdg_directories(&path_cfg).unwrap();

        tmp_dir.child("config").assert(predicates::path::exists());
        tmp_dir.child("data").assert(predicates::path::exists());

        tmp_dir.close().unwrap();
    }
}
