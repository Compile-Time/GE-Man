use ge_man_lib::config::{LutrisConfig, SteamConfig};
use ge_man_lib::error::{LutrisConfigError, SteamConfigError};
use ge_man_lib::tag::TagKind;
use std::path::Path;

pub struct ApplicationConfig {
    kind: String,
    version: String,
    version_dir_name: String,
}

impl ApplicationConfig {
    pub fn new(kind: String, version: String, version_dir_name: String) -> Self {
        Self {
            kind,
            version,
            version_dir_name,
        }
    }

    pub fn kind(&self) -> &String {
        &self.kind
    }

    pub fn version(&self) -> &String {
        &self.version
    }

    pub fn version_dir_name(&self) -> &String {
        &self.version_dir_name
    }

    pub fn create_copy(kind: &TagKind, path: &Path) -> anyhow::Result<ApplicationConfig> {
        let config = match kind {
            TagKind::Proton => {
                let config = SteamConfig::create_copy(path)?;
                ApplicationConfig::from(config)
            }
            TagKind::Wine { .. } => {
                let config = LutrisConfig::create_copy(path)?;
                ApplicationConfig::from(config)
            }
        };
        Ok(config)
    }
}

impl From<SteamConfig> for ApplicationConfig {
    fn from(config: SteamConfig) -> Self {
        ApplicationConfig::new(config.kind(), config.proton_version(), config.version_dir_name())
    }
}

impl From<LutrisConfig> for ApplicationConfig {
    fn from(config: LutrisConfig) -> Self {
        ApplicationConfig::new(config.kind(), config.wine_version(), config.version_dir_name())
    }
}

pub trait AppConfig {
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

pub trait AppConfigError {
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
