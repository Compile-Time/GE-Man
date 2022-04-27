use std::path::PathBuf;
use std::{env, fs};

use anyhow::Context;
use ge_man_lib::tag::TagKind;
#[cfg(test)]
use mockall::automock;

pub const STEAM_COMP_DIR: &str = "Steam/compatibilitytools.d";
pub const LUTRIS_WINE_RUNNERS_DIR: &str = "lutris/runners/wine";

const HOME: &str = "HOME";
const XDG_DATA_HOME: &str = "XDG_DATA_HOME";
const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
const STEAM_PATH_ENV: &str = "STEAM_PATH";

const APP_NAME: &str = "ge_man";

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
    fn xdg_data_dir(&self, xdg_data_home: Option<String>) -> PathBuf {
        let data_dir = xdg_data_home
            .or_else(|| env::var(HOME).ok().map(|home| format!("{}/.local/share", home)))
            .unwrap();

        PathBuf::from(data_dir)
    }

    fn xdg_config_dir(&self, xdg_config_home: Option<String>) -> PathBuf {
        let config_dir = xdg_config_home
            .or_else(|| env::var(HOME).ok().map(|home| format!("{}/.config", home)))
            .unwrap();

        PathBuf::from(config_dir)
    }

    fn steam(&self, xdg_data_home: Option<String>, steam_path: Option<String>) -> PathBuf {
        steam_path
            .map(PathBuf::from)
            .unwrap_or_else(|| self.xdg_data_dir(xdg_data_home).join("Steam"))
    }

    fn lutris_local(&self, xdg_data_home: Option<String>) -> PathBuf {
        self.xdg_data_dir(xdg_data_home).join("lutris")
    }

    fn lutris_config(&self, xdg_config_home: Option<String>) -> PathBuf {
        self.xdg_config_dir(xdg_config_home).join("lutris")
    }

    fn steam_config(&self, xdg_data_home: Option<String>, steam_path: Option<String>) -> PathBuf {
        self.steam(xdg_data_home, steam_path).join("config/config.vdf")
    }

    fn steam_compatibility_tools_dir(&self, xdg_data_home: Option<String>, steam_path: Option<String>) -> PathBuf {
        self.steam(xdg_data_home, steam_path).join("compatibilitytools.d")
    }

    fn lutris_runners_config_dir(&self, xdg_config_home: Option<String>) -> PathBuf {
        self.lutris_config(xdg_config_home).join("runners")
    }

    fn lutris_wine_runner_config(&self, xdg_config_home: Option<String>) -> PathBuf {
        self.lutris_runners_config_dir(xdg_config_home).join("wine.yml")
    }

    fn lutris_runners_dir(&self, xdg_data_home: Option<String>) -> PathBuf {
        self.lutris_local(xdg_data_home).join("runners/wine")
    }

    fn ge_man_data_dir(&self, xdg_data_home: Option<String>) -> PathBuf {
        self.xdg_data_dir(xdg_data_home).join(APP_NAME)
    }

    fn ge_man_config_dir(&self, xdg_config_home: Option<String>) -> PathBuf {
        self.xdg_config_dir(xdg_config_home).join(APP_NAME)
    }

    fn managed_versions_config(&self, xdg_data_home: Option<String>) -> PathBuf {
        self.ge_man_data_dir(xdg_data_home).join("managed_versions.json")
    }

    fn app_config_backup_file(&self, xdg_config_home: Option<String>, kind: &TagKind) -> PathBuf {
        let config_file = match kind {
            TagKind::Proton => "steam-config-backup.vdf",
            TagKind::Wine { .. } => "lutris-wine-runner-config-backup.yml",
        };
        self.ge_man_config_dir(xdg_config_home).join(config_file)
    }

    fn create_ge_man_dirs(&self, xdg_config_home: Option<String>, xdg_data_home: Option<String>) -> anyhow::Result<()> {
        let ge_config_dir = self.ge_man_config_dir(xdg_config_home);
        let ge_data_dir = self.ge_man_data_dir(xdg_data_home);

        fs::create_dir_all(&ge_config_dir).context(format!(
            r#"Failed to create directory "ge_man" in {}"#,
            ge_config_dir.display()
        ))?;
        fs::create_dir_all(&ge_data_dir).context(format!(
            r#"Failed to create directory "ge_man" in {}"#,
            ge_data_dir.display()
        ))?;

        Ok(())
    }

    fn create_app_dirs(
        &self,
        xdg_config_home: Option<String>,
        xdg_data_home: Option<String>,
        steam_path: Option<String>,
    ) -> anyhow::Result<()> {
        let steam_compat_dir = self.steam_compatibility_tools_dir(xdg_data_home.clone(), steam_path);
        let lutris_runners_cfg_dir = self.lutris_runners_config_dir(xdg_config_home);
        let lutris_runners_dir = self.lutris_runners_dir(xdg_data_home);

        fs::create_dir_all(&steam_compat_dir).context(format!(
            r#"Failed to create directory "compatibilitytools.d" in {}"#,
            steam_compat_dir.display()
        ))?;
        fs::create_dir_all(&lutris_runners_cfg_dir).context(format!(
            r#"Failed to create directory "runners" in {}"#,
            lutris_runners_cfg_dir.display()
        ))?;
        fs::create_dir_all(&lutris_runners_dir).context(format!(
            r#"Failed to create directory "wine" in {}"#,
            lutris_runners_dir.display()
        ))?;

        Ok(())
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
            path_cfg.lutris_wine_runner_config(xdg_config_home()),
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
    use ge_man_lib::tag::TagKind;

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
        let path = path_cfg.lutris_wine_runner_config(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".config/lutris/runners/wine.yml"));
    }

    #[test]
    fn lutris_wine_config_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.lutris_wine_runner_config(Some(String::from("/tmp/xdg-config")));

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
    fn ge_man_data_dir_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.ge_man_data_dir(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".local/share/ge_man"));
    }

    #[test]
    fn ge_man_data_dir_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.ge_man_data_dir(Some(String::from("/tmp/xdg-data")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-data/ge_man"));
    }

    #[test]
    fn ge_man_config_dir_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.ge_man_config_dir(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".config/ge_man"));
    }

    #[test]
    fn ge_man_config_dir_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.ge_man_config_dir(Some(String::from("/tmp/xdg-config")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/ge_man"));
    }

    #[test]
    fn ge_man_managed_versions_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.managed_versions_config(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path
            .to_string_lossy()
            .contains(".local/share/ge_man/managed_versions.json"));
    }

    #[test]
    fn ge_man_managed_versions_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.managed_versions_config(Some(String::from("/tmp/xdg-config")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/ge_man/managed_versions.json"));
    }

    #[test]
    fn ge_man_backup_file_for_steam_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.app_config_backup_file(None, &TagKind::Proton);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path
            .to_string_lossy()
            .contains(".config/ge_man/steam-config-backup.vdf"));
    }

    #[test]
    fn ge_man_backup_file_for_steam_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.app_config_backup_file(Some(String::from("/tmp/xdg-config")), &TagKind::Proton);

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/ge_man/steam-config-backup.vdf"));
    }

    #[test]
    fn ge_man_backup_file_for_lutris_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.app_config_backup_file(None, &TagKind::wine());

        assert!(path.to_string_lossy().contains("home"));
        assert!(path
            .to_string_lossy()
            .contains(".config/ge_man/lutris-wine-runner-config-backup.yml"));
    }

    #[test]
    fn ge_man_backup_file_for_lutris_with_xdg_config_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.app_config_backup_file(Some(String::from("/tmp/xdg-config")), &TagKind::wine());

        assert_eq!(
            path,
            PathBuf::from("/tmp/xdg-config/ge_man/lutris-wine-runner-config-backup.yml")
        );
    }

    #[test]
    fn create_ge_man_dirs_should_create_ge_man_directories() {
        let tmp_dir = TempDir::new().unwrap();
        let config_dir = tmp_dir.join("config");
        let data_dir = tmp_dir.join("local");

        let path_cfg = PathConfig::default();
        path_cfg
            .create_ge_man_dirs(
                Some(config_dir.display().to_string()),
                Some(data_dir.display().to_string()),
            )
            .unwrap();

        tmp_dir.child("config/ge_man").assert(predicates::path::exists());
        tmp_dir.child("local/ge_man").assert(predicates::path::exists());

        tmp_dir.close().unwrap();
    }

    #[test]
    fn create_app_dirs_should_create_steam_and_lutris_directories() {
        let tmp_dir = TempDir::new().unwrap();
        let steam_dir = tmp_dir.join("local/Steam");
        let config_dir = tmp_dir.join("config");
        let data_dir = tmp_dir.join("local");

        let path_cfg = PathConfig::default();
        path_cfg
            .create_app_dirs(
                Some(config_dir.display().to_string()),
                Some(data_dir.display().to_string()),
                Some(steam_dir.display().to_string()),
            )
            .unwrap();

        tmp_dir
            .child("local/Steam/compatibilitytools.d")
            .assert(predicates::path::exists());
        tmp_dir
            .child("config/lutris/runners")
            .assert(predicates::path::exists());
        tmp_dir
            .child("local/lutris/runners/wine")
            .assert(predicates::path::exists());

        tmp_dir.close().unwrap();
    }
}
