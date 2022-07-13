use crate::data::ManagedVersion;
use ge_man_lib::config::{LutrisConfig, SteamConfig};
use ge_man_lib::tag::TagKind;
use std::path::Path;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct ApplicationConfig {
    kind: TagKind,
    version_dir_name: String,
}

impl ApplicationConfig {
    pub fn new(kind: TagKind, version_dir_name: String) -> Self {
        Self { kind, version_dir_name }
    }

    pub fn kind(&self) -> TagKind {
        self.kind
    }

    pub fn version_dir_name(&self) -> &String {
        &self.version_dir_name
    }

    pub fn create_copy(kind: TagKind, path: &Path) -> anyhow::Result<ApplicationConfig> {
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

    pub fn check_if_version_is_in_use(&self, version: &ManagedVersion) -> bool {
        self.version_dir_name().eq(version.directory_name())
    }
}

impl From<SteamConfig> for ApplicationConfig {
    fn from(config: SteamConfig) -> Self {
        ApplicationConfig::new(config.kind(), config.proton_version())
    }
}

impl From<LutrisConfig> for ApplicationConfig {
    fn from(config: LutrisConfig) -> Self {
        ApplicationConfig::new(config.kind(), config.wine_version())
    }
}

pub trait AppConfig {
    fn version_dir_name(&self) -> String;
    fn kind(&self) -> TagKind;
}

impl AppConfig for SteamConfig {
    fn version_dir_name(&self) -> String {
        self.proton_version()
    }

    fn kind(&self) -> TagKind {
        TagKind::Proton
    }
}

impl AppConfig for LutrisConfig {
    fn version_dir_name(&self) -> String {
        self.wine_version()
    }

    fn kind(&self) -> TagKind {
        if self.wine_version().to_lowercase().contains("lol") {
            TagKind::lol()
        } else {
            TagKind::wine()
        }
    }
}
