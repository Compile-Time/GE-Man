use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{env, fs};

use anyhow::{anyhow, Context};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

lazy_static! {
    static ref ENV_REGEX: Regex = Regex::new(r"(\$([A-Za-z0-9]+))").unwrap();
    pub static ref GE_MAN_CONFIG: Mutex<GeManConfig> = Mutex::new(GeManConfig::default());
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeManConfig {
    #[serde(rename = "steam_root_path")]
    steam_root_path_str: Option<String>,
    #[serde(skip)]
    steam_root_path: Option<PathBuf>,
}

impl GeManConfig {
    fn new(steam_root_path_str: Option<String>) -> Self {
        let steam_root_path = steam_root_path_str.clone().map(PathBuf::from);

        GeManConfig {
            steam_root_path_str: steam_root_path_str,
            steam_root_path: steam_root_path,
        }
    }

    pub fn read_config(&mut self, path: &Path) -> anyhow::Result<()> {
        let json = match fs::read_to_string(&path) {
            Ok(json) => json,
            Err(err) => match err.kind() {
                // This case here would be a candidate for logging.
                ErrorKind::NotFound => return Ok(()),
                _ => return Err(anyhow!(err).context(format!("Could not read config from {}", path.display()))),
            },
        };

        let fs_config: GeManConfig = serde_yaml::from_str(&json).context("Could not serialize YAML to struct")?;
        self.steam_root_path_str = fs_config.steam_root_path_str;
        self.determine_steam_root_path()
            .context("The configured Steam root path in the GE-Man config is not valid")?;
        Ok(())
    }

    pub fn steam_root_path(&self) -> Option<PathBuf> {
        self.steam_root_path.clone()
    }

    fn path_with_expanded_env_vars(str: &str) -> anyhow::Result<String> {
        let mut expanded_path = String::from(str);
        let captures = ENV_REGEX.captures_iter(str);

        for cap in captures {
            let env_name = &cap[1];
            let env_value = env::var(&cap[2]).context(format!("Environment variable {} does not exist", &cap[2]))?;

            expanded_path = expanded_path.replace(env_name, &env_value);
        }

        Ok(expanded_path)
    }

    fn determine_steam_root_path(&mut self) -> anyhow::Result<()> {
        if self.steam_root_path_str.is_none() {
            self.steam_root_path = None;
            return Ok(());
        }

        let steam_root_path_str = self.steam_root_path_str.as_ref().unwrap();
        let expanded_path = GeManConfig::path_with_expanded_env_vars(steam_root_path_str)?;
        let expanded_path =
            PathBuf::try_from(&expanded_path).context(format!("Path {} is not valid", &expanded_path))?;
        self.steam_root_path = Some(expanded_path);
        Ok(())
    }
}

impl Default for GeManConfig {
    fn default() -> Self {
        GeManConfig::new(None)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::{Path, PathBuf};

    use crate::config::GeManConfig;
    use crate::path::HOME;

    #[test]
    fn read_config_should_create_config_file_from_yml_file() {
        let mut config = GeManConfig::default();
        config
            .read_config(Path::new("test_resources/config/steam_path_set.yml"))
            .unwrap();
        assert_eq!(config.steam_root_path(), Some(PathBuf::from("/tmp/test")));
    }

    #[test]
    fn read_config_should_not_error_on_missing_config() {
        let mut config = GeManConfig::default();
        config
            .read_config(Path::new("test_resources/config/does-not-exist.yml"))
            .unwrap();
        assert_eq!(config.steam_root_path, None);
    }

    #[test]
    fn read_config_should_parse_home_env_in_steam_path() {
        let mut config = GeManConfig::default();
        config
            .read_config(Path::new("test_resources/config/steam_path_set_with_env.yml"))
            .unwrap();
        assert!(config.steam_root_path.is_some());

        let path = config.steam_root_path.unwrap();
        let expected_path = PathBuf::from(format!("{}/.local/share/Steam", env::var(HOME).unwrap()));
        assert_eq!(path, expected_path);
    }
}
