//! Get a copy of a Steam or Lutris config file to get or modify the used compatibility tool version.
//!
//! This module provides structs that allow a crate to modify the globally used Proton version for Steam or the
//! globally Wine version in Lutris.
//!
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::{LutrisConfigError, SteamConfigError};

const DEFAULT_PROTON_VERSION_INDEX: &str = r###""0""###;
const WINE_VERSION_ATTRIBUTE: &str = "version";

/// Represents a copy of a Steam configuration file.
///
/// This struct only allow modifications to the globally used Proton version by Steam.
///
/// # Examples
///
/// The struct can be written to a file by using its `Into` trait.
///
/// ```ignore
/// let path = Path::from("/some/path");
/// let steam_config = SteamConfig::create_copy(path).unwrap();
///
/// steam_config.set_proton_version("Proton-6.20-GE-1");
/// let steam_config: Vec<u8> = steam_config.into();
/// std::fs::write(path, steam_config).unwrap();
/// ```
pub struct SteamConfig {
    lines: Vec<String>,
    // This is the line data for the default compatibility tool to use.
    compat_tool_line: String,
    compat_tool_line_idx: usize,
    compat_tool_value_start_idx: usize,
    compat_tool_value_end_idx: usize,
}

impl SteamConfig {
    /// Create a copy of a Steam config provided by path.
    ///
    /// This method reads a global Steam configuration file and creates a copy of it by reading it line by line. While
    /// reading each line the following information is determined:
    ///
    /// * The line that contains the default compatibility tool directory name (dir_name_line)
    /// * The index of the dir_name_line
    /// * The index where the value of dir_name_line begins
    /// * The index where the value of dir_name_line ends
    ///
    /// This above information is stored to make future modifications easier.
    ///
    /// # Errors
    ///
    /// This method will return an error in the following cases:
    ///
    /// * When the default compatibility tool attribute could not be found
    /// * When any filesystem operations returns an IO error
    pub fn create_copy(config_file_path: &Path) -> Result<Self, SteamConfigError> {
        let steam_config = File::open(config_file_path)?;
        let steam_config = BufReader::new(steam_config);

        let mut passed_compat_tool_attribute = false;
        let mut default_compat_tool_dir_name_line_idx = None;
        let mut lines: Vec<String> = Vec::with_capacity(steam_config.capacity());

        // Create in memory copy and find default value for CompatTool attribute.
        for (idx, line) in steam_config.lines().enumerate() {
            if let Ok(line) = line {
                lines.push(line.clone());

                if line.contains("CompatToolMapping") {
                    passed_compat_tool_attribute = true;
                }

                if default_compat_tool_dir_name_line_idx.is_none()
                    && passed_compat_tool_attribute
                    && line.contains(DEFAULT_PROTON_VERSION_INDEX)
                {
                    default_compat_tool_dir_name_line_idx = Some(idx + 2);
                }
            }
        }

        if let Some(dir_name_line_idx) = default_compat_tool_dir_name_line_idx {
            let dir_name_line = lines.get(dir_name_line_idx).cloned().unwrap();
            let mut value_indices = dir_name_line.rmatch_indices('\"');

            let value_end_idx = value_indices.next().unwrap().0;
            let value_start_idx = value_indices.next().unwrap().0 + 1;

            Ok(SteamConfig {
                lines,
                compat_tool_line: dir_name_line,
                compat_tool_line_idx: dir_name_line_idx,
                compat_tool_value_start_idx: value_start_idx,
                compat_tool_value_end_idx: value_end_idx,
            })
        } else {
            Err(SteamConfigError::NoDefaultCompatToolAttribute)
        }
    }

    /// Get the global Proton version stored in the Steam config file.
    ///
    /// The "version" is actually the name of the directory that contains all the version data.
    pub fn proton_version(&self) -> String {
        self.compat_tool_line[self.compat_tool_value_start_idx..self.compat_tool_value_end_idx].to_owned()
    }

    /// Set the global Proton version for this file copy.
    ///
    /// The "version" is actually the name of the directory that contains all the version data.
    pub fn set_proton_version(&mut self, proton_dir_name: &str) {
        self.compat_tool_line.replace_range(
            self.compat_tool_value_start_idx..self.compat_tool_value_end_idx,
            proton_dir_name,
        );
        self.lines.splice(
            self.compat_tool_line_idx..=self.compat_tool_line_idx,
            [self.compat_tool_line.clone()],
        );
    }
}

impl Into<Vec<u8>> for SteamConfig {
    fn into(self) -> Vec<u8> {
        self.lines.join("\n").into_bytes()
    }
}

/// # Examples
///
/// The struct can be written to a file by using its `Into` trait.
///
/// ```ignore
/// let path = Path::from("/some/path");
/// let lutris_config = LutrisConfig::create_copy(path).unwrap();
///
/// lutris_config.set_proton_version("Proton-6.20-GE-1");
/// let lutris_config: Vec<u8> = lutris_config.into();
/// std::fs::write(path, lutris_config).unwrap();
/// ```
pub struct LutrisConfig {
    lines: Vec<String>,
    wine_dir_line: String,
    wine_dir_line_idx: usize,
    wine_dir_line_value_start_idx: usize,
    wine_dir_line_value_end_idx: usize,
}

impl LutrisConfig {
    /// Create a copy of a Lutris config provided by path.
    ///
    /// This method reads a global Wine configuration file of Lutris and creates a copy of it by reading it line by
    /// line. While reading each line the following information is determined:
    ///
    /// * The line that contains the default compatibility tool directory name (dir_name_line)
    /// * The index of the dir_name_line
    /// * The index where the value of dir_name_line begins
    /// * The index where the value of dir_name_line ends
    ///
    /// This above information is stored make future modifications easier.
    ///
    /// # Errors
    ///
    /// * When the default compatibility tool attribute could not be found
    /// * When any filesystem operations returns an IO error
    pub fn create_copy(config_file_path: &Path) -> Result<Self, LutrisConfigError> {
        let runner_config = File::open(config_file_path)?;
        let runner_config = BufReader::new(runner_config);

        let mut lines: Vec<String> = Vec::with_capacity(runner_config.capacity());

        let mut dir_name_line_idx = None;
        let mut wine_dir_line = String::new();
        let mut wine_dir_name_line_value_start_idx = usize::default();
        let mut wine_dir_name_line_value_end_idx = usize::default();

        for (idx, line) in runner_config.lines().enumerate() {
            let line = line?;

            if line.contains(WINE_VERSION_ATTRIBUTE) {
                dir_name_line_idx = Some(idx);
                wine_dir_name_line_value_start_idx = line.find(": ").unwrap() + 2;
                wine_dir_name_line_value_end_idx = line.len();

                wine_dir_line = line.clone();
            }
            lines.push(line.clone());
        }

        if let Some(dir_name_line_idx) = dir_name_line_idx {
            Ok(LutrisConfig {
                lines,
                wine_dir_line,
                wine_dir_line_idx: dir_name_line_idx,
                wine_dir_line_value_start_idx: wine_dir_name_line_value_start_idx,
                wine_dir_line_value_end_idx: wine_dir_name_line_value_end_idx,
            })
        } else {
            Err(LutrisConfigError::NoVersionAttribute)
        }
    }

    /// Set the global Wine version for this file copy.
    ///
    /// The "version" is actually the name of the directory that contains all the version data.
    pub fn set_wine_version(&mut self, wine_directory_name: &str) {
        self.wine_dir_line.replace_range(
            self.wine_dir_line_value_start_idx..self.wine_dir_line_value_end_idx,
            wine_directory_name,
        );
        self.lines.splice(
            self.wine_dir_line_idx..=self.wine_dir_line_idx,
            [self.wine_dir_line.clone()],
        );
    }

    /// Get the global Wine version stored in global Wine config for Lutris.
    ///
    /// The "version" is actually the name of the directory that contains all the version data.
    pub fn wine_version(&self) -> String {
        self.wine_dir_line[self.wine_dir_line_value_start_idx..self.wine_dir_line_value_end_idx].to_owned()
    }
}

impl Into<Vec<u8>> for LutrisConfig {
    fn into(self) -> Vec<u8> {
        self.lines.join("\n").into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn create_lutris_config_from_non_existent_file() {
        let config_path = PathBuf::from("/tmp/none");
        let result = LutrisConfig::create_copy(&config_path);
        assert!(result.is_err());

        let err = result.err().unwrap();
        assert!(matches!(err, LutrisConfigError::IoError { .. }));
    }

    #[test]
    fn create_lutris_config_from_file_with_no_version_property() {
        let config_path = Path::new("tests/resources/assets/wine-no-version.yml");
        let result = LutrisConfig::create_copy(&config_path);
        assert!(result.is_err());

        let err = result.err().unwrap();
        assert!(matches!(err, LutrisConfigError::NoVersionAttribute));
    }

    #[test]
    fn create_lutris_config_copy_with_modified_default_wine_runner_version() {
        let lutris_runner_dir = "lutris-ge-6.20-1-x86_64";
        let config_file_path = Path::new("tests/resources/assets/wine.yml");
        let mut lutris_config = LutrisConfig::create_copy(config_file_path).unwrap();
        lutris_config.set_wine_version(lutris_runner_dir);

        let bytes_copy: Vec<u8> = lutris_config.into();

        let mut lines_copy = BufReader::new(std::io::Cursor::new(bytes_copy)).lines();
        let config_file = BufReader::new(File::open(config_file_path).unwrap());

        for (idx, line) in config_file.lines().enumerate() {
            let line = line.unwrap();
            let line_copy = lines_copy.next().unwrap().unwrap();

            if idx == 8 {
                assert_eq!(line_copy, format!(r###"  version: {}"###, lutris_runner_dir));
                assert_eq!(line, r###"  version: lutris-ge-6.21-1-x86_64"###);
            } else {
                assert_eq!(line, line_copy);
            }
        }
    }

    #[test]
    fn read_wine_version_from_lutris_config() {
        let config_file = Path::new("tests/resources/assets/wine.yml");
        let lutris_config = LutrisConfig::create_copy(config_file).unwrap();

        let version = lutris_config.wine_version();
        assert_eq!(version, "lutris-ge-6.21-1-x86_64");
    }

    #[test]
    fn create_steam_config_copy_with_modified_default_proton_version() {
        let proton_dir_name = "Proton-6.20-GE-1";
        let config_file_path = Path::new("tests/resources/assets/config.vdf");

        let mut steam_config = SteamConfig::create_copy(config_file_path).unwrap();
        steam_config.set_proton_version(proton_dir_name);

        let bytes_copy: Vec<u8> = steam_config.into();

        let mut lines_copy = bytes_copy.lines();
        let conf_file = BufReader::new(File::open(config_file_path).unwrap());

        for (idx, line) in conf_file.lines().enumerate() {
            let line = line.unwrap();
            let line_copy = lines_copy.next().unwrap().unwrap();

            if idx == 12 {
                assert_eq!(line_copy, format!(r###"						"name"		"{}""###, proton_dir_name));
                assert_eq!(line, r###"						"name"		"Proton-6.21-GE-2""###)
            } else {
                assert_eq!(line, line_copy);
            }
        }
    }

    #[test]
    fn read_proton_version_from_steam_config() {
        let config_file = Path::new("tests/resources/assets/config.vdf");

        let steam_config = SteamConfig::create_copy(config_file).unwrap();
        let version = steam_config.proton_version();

        assert_eq!(version, "Proton-6.21-GE-2");
    }

    #[test]
    fn create_steam_config_copy_from_file_with_no_compat_tool_attribute() {
        let config_file = Path::new("tests/resources/assets/config-no-compat-tool-attr.vdf");
        let result = SteamConfig::create_copy(&config_file);
        assert!(result.is_err());

        let err = result.err().unwrap();
        assert!(matches!(err, SteamConfigError::NoDefaultCompatToolAttribute));
    }

    #[test]
    fn create_steam_config_copy_from_file_with_no_default_proton_version() {
        let config_file = Path::new("tests/resources/assets/config-no-default-version.vdf");
        let result = SteamConfig::create_copy(&config_file);
        assert!(result.is_err());

        let err = result.err().unwrap();
        assert!(matches!(err, SteamConfigError::NoDefaultCompatToolAttribute));
    }

    #[test]
    fn create_steam_config_copy_from_invalid_path() {
        let config_file = PathBuf::from("/tmp/none");
        let result = SteamConfig::create_copy(&config_file);
        assert!(result.is_err());

        let err = result.err().unwrap();
        assert!(matches!(err, SteamConfigError::IoError { .. }));
    }
}
