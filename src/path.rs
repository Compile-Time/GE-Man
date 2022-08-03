use std::path::PathBuf;
use std::{env, fs};

use anyhow::Context;
use ge_man_lib::tag::TagKind;
#[cfg(test)]
use mockall::automock;

use crate::config;

pub const STEAM_COMP_DIR: &str = "Steam/compatibilitytools.d";
pub const LUTRIS_WINE_RUNNERS_DIR: &str = "lutris/runners/wine";

pub const HOME: &str = "HOME";
pub const XDG_DATA_HOME: &str = "XDG_DATA_HOME";
pub const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
pub const XDG_STATE_HOME: &str = "XDG_STATE_HOME";
pub const STEAM_PATH_ENV: &str = "GE_MAN_STEAM_PATH";
const APP_NAME: &str = "ge_man";

pub mod overrule {
    use super::*;

    pub fn xdg_data_home() -> Option<PathBuf> {
        env::var(XDG_DATA_HOME).map(PathBuf::from).ok()
    }

    pub fn xdg_config_home() -> Option<PathBuf> {
        env::var(XDG_CONFIG_HOME).map(PathBuf::from).ok()
    }

    pub fn xdg_state_home() -> Option<PathBuf> {
        env::var(XDG_STATE_HOME).map(PathBuf::from).ok()
    }

    pub fn steam_root() -> Option<PathBuf> {
        env::var(STEAM_PATH_ENV)
            .map(PathBuf::from)
            .ok()
            .or_else(|| config::GE_MAN_CONFIG.lock().unwrap().steam_root_path())
    }
}

#[cfg_attr(test, automock)]
pub trait PathConfiguration {
    /// Path contained in "XDG_DATA_HOME" or "$HOME/.local/share".
    fn xdg_data_dir(&self, xdg_data_home: Option<PathBuf>) -> PathBuf {
        let data_dir = xdg_data_home
            .or_else(|| {
                env::var(HOME)
                    .ok()
                    .map(|home| PathBuf::from(format!("{}/.local/share", home)))
            })
            .unwrap();

        PathBuf::from(data_dir)
    }

    /// Path contained in "XDG_CONFIG_HOME" or "$HOME/.config".
    fn xdg_config_dir(&self, xdg_config_home: Option<PathBuf>) -> PathBuf {
        xdg_config_home
            .or_else(|| {
                env::var(HOME)
                    .ok()
                    .map(|home| PathBuf::from(format!("{}/.config", home)))
            })
            .unwrap()
    }

    /// Path contained in "XDG_STATE_HOME" or "$HOME/.local/state"
    fn xdg_state_dir(&self, xdg_state_home: Option<PathBuf>) -> PathBuf {
        xdg_state_home
            .or_else(|| {
                env::var(HOME)
                    .ok()
                    .map(|home| PathBuf::from(format!("{}/.local/state", home)))
            })
            .unwrap()
    }

    /// Path to the Steam installation folder.
    fn steam(&self, steam_root_path_override: Option<PathBuf>) -> PathBuf {
        let steam_root_symlink = env::var(HOME)
            .ok()
            .map(|home| format!("{}/.steam/root", home))
            .map(PathBuf::from)
            .unwrap();

        steam_root_path_override.unwrap_or_else(|| steam_root_symlink)
    }

    /// Path to the XDG compliant Lutris data folder.
    ///
    /// The `xdg_data_home` variable can be used to access a non XDG compliant Lutris installation.
    fn lutris_local(&self, xdg_data_home_override: Option<PathBuf>) -> PathBuf {
        self.xdg_data_dir(xdg_data_home_override).join("lutris")
    }

    /// Path to the XDG compliant Lutris config folder.
    ///
    /// The `xdg_config_home` variable can be used to access a non XDG compliant Lutris installation.
    fn lutris_config(&self, xdg_config_home_override: Option<PathBuf>) -> PathBuf {
        self.xdg_config_dir(xdg_config_home_override).join("lutris")
    }

    /// Path to the Steam config file which contains the default Proton version to use.
    fn steam_config(&self, steam_root_path_override: Option<PathBuf>) -> PathBuf {
        self.steam(steam_root_path_override).join("config/config.vdf")
    }

    /// Path to the Steam compatiblity tools directory.
    fn steam_compatibility_tools_dir(&self, steam_root_path_override: Option<PathBuf>) -> PathBuf {
        self.steam(steam_root_path_override).join("compatibilitytools.d")
    }

    /// Path to the global default Wine runner config directory.
    fn lutris_runners_config_dir(&self, xdg_config_home_override: Option<PathBuf>) -> PathBuf {
        self.lutris_config(xdg_config_home_override).join("runners")
    }

    /// Path to the global default Wine runner config file.
    fn lutris_wine_runner_config(&self, xdg_config_home_override: Option<PathBuf>) -> PathBuf {
        self.lutris_runners_config_dir(xdg_config_home_override)
            .join("wine.yml")
    }

    /// Path to the Lutris runners directory which contains all Wine versions.
    fn lutris_runners_dir(&self, xdg_data_home_override: Option<PathBuf>) -> PathBuf {
        self.lutris_local(xdg_data_home_override).join("runners/wine")
    }

    /// Path to the XDG compliant GE-Man data directory.
    fn ge_man_data_dir(&self, xdg_data_home_override: Option<PathBuf>) -> PathBuf {
        self.xdg_data_dir(xdg_data_home_override).join(APP_NAME)
    }

    /// Path to the XDG compliant GE-Man config directory.
    fn ge_man_config_dir(&self, xdg_config_home_override: Option<PathBuf>) -> PathBuf {
        self.xdg_config_dir(xdg_config_home_override).join(APP_NAME)
    }

    fn managed_versions_config(&self, xdg_data_home: Option<PathBuf>) -> PathBuf {
        self.ge_man_data_dir(xdg_data_home).join("managed_versions.json")
    }

    fn ge_man_config(&self, xdg_config_home_override: Option<PathBuf>) -> PathBuf {
        self.ge_man_config_dir(xdg_config_home_override).join("config.yml")
    }

    /// Returns a path to the backup file of the Steam or Lutris config.
    fn app_config_backup_file(&self, xdg_config_home_override: Option<PathBuf>, kind: &TagKind) -> PathBuf {
        let config_file = match kind {
            TagKind::Proton => "steam-config-backup.vdf",
            TagKind::Wine { .. } => "lutris-wine-runner-config-backup.yml",
        };
        self.ge_man_config_dir(xdg_config_home_override).join(config_file)
    }

    fn create_ge_man_dirs(
        &self,
        xdg_config_home_override: Option<PathBuf>,
        xdg_data_home_override: Option<PathBuf>,
    ) -> anyhow::Result<()> {
        let ge_config_dir = self.ge_man_config_dir(xdg_config_home_override);
        let ge_data_dir = self.ge_man_data_dir(xdg_data_home_override);

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
        xdg_config_home_override: Option<PathBuf>,
        xdg_data_home_override: Option<PathBuf>,
        steam_root_override: Option<PathBuf>,
    ) -> anyhow::Result<()> {
        let steam_compat_dir = self.steam_compatibility_tools_dir(steam_root_override);
        let lutris_runners_cfg_dir = self.lutris_runners_config_dir(xdg_config_home_override);
        let lutris_runners_dir = self.lutris_runners_dir(xdg_data_home_override);

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

    /// Returns the Steam compatibility tools folder or the Lutris runners folder.
    fn application_compatibility_tools_dir(&self, kind: TagKind) -> PathBuf {
        match kind {
            TagKind::Proton => self.steam_compatibility_tools_dir(overrule::steam_root()),
            TagKind::Wine { .. } => self.lutris_runners_dir(overrule::xdg_data_home()),
        }
    }

    /// Returns the Steam configuration file path or the Lutris Wine runner configuration file path.
    fn application_config_file(&self, kind: TagKind) -> PathBuf {
        match kind {
            TagKind::Proton => self.steam_config(overrule::steam_root()),
            TagKind::Wine { .. } => self.lutris_wine_runner_config(overrule::xdg_config_home()),
        }
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
        let path = path_cfg.xdg_data_dir(Some(PathBuf::from("/tmp/xdg-data")));
        assert_eq!(path, PathBuf::from("/tmp/xdg-data/"));
    }

    #[test]
    fn steam_path_with_no_overrides() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".steam/root"));
    }

    #[test]
    fn steam_path_with_steam_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam(Some(PathBuf::from("/tmp/steam")));

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
        let path = path_cfg.lutris_local(Some(PathBuf::from("/tmp/xdg-data")));

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
        let path = path_cfg.lutris_config(Some(PathBuf::from("/tmp/xdg-config")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/lutris"));
    }

    #[test]
    fn steam_config_with_no_overrides() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_config(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".steam/root/config/config.vdf"));
    }

    #[test]
    fn steam_config_with_steam_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_config(Some(PathBuf::from("/tmp/steam")));

        assert_eq!(path, PathBuf::from("/tmp/steam/config/config.vdf"));
    }

    #[test]
    fn steam_compatibilitytools_with_no_overrides() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_compatibility_tools_dir(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".steam/root/compatibilitytools.d"));
    }

    #[test]
    fn steam_compatibilitytools_with_steam_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.steam_compatibility_tools_dir(Some(PathBuf::from("/tmp/steam")));

        assert_eq!(path, PathBuf::from("/tmp/steam/compatibilitytools.d"));
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
        let path = path_cfg.lutris_wine_runner_config(Some(PathBuf::from("/tmp/xdg-config")));

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
        let path = path_cfg.lutris_runners_dir(Some(PathBuf::from("/tmp/xdg-data")));

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
        let path = path_cfg.ge_man_data_dir(Some(PathBuf::from("/tmp/xdg-data")));

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
    fn ge_man_config_file_with_no_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.ge_man_config(None);

        assert!(path.to_string_lossy().contains("home"));
        assert!(path.to_string_lossy().contains(".config/ge_man/config.yml"));
    }

    #[test]
    fn ge_man_config_file_with_xdg_home_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.ge_man_config(Some(PathBuf::from("/tmp/xdg-config")));

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/ge_man/config.yml"));
    }

    #[test]
    fn ge_man_config_dir_with_xdg_data_override() {
        let path_cfg = PathConfig::default();
        let path = path_cfg.ge_man_config_dir(Some(PathBuf::from("/tmp/xdg-config")));

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
        let path = path_cfg.managed_versions_config(Some(PathBuf::from("/tmp/xdg-config")));

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
        let path = path_cfg.app_config_backup_file(Some(PathBuf::from("/tmp/xdg-config")), &TagKind::Proton);

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
        let path = path_cfg.app_config_backup_file(Some(PathBuf::from("/tmp/xdg-config")), &TagKind::wine());

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
        path_cfg.create_ge_man_dirs(Some(config_dir), Some(data_dir)).unwrap();

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
            .create_app_dirs(Some(config_dir), Some(data_dir), Some(steam_dir))
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
