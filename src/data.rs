use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::slice::Iter;
use std::vec::IntoIter;

use anyhow::{bail, Context};
use ge_man_lib::tag::{Tag, TagKind};
use serde::{Deserialize, Serialize};

use crate::version::{Version, Versioned};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub struct ManagedVersion {
    tag: Tag,
    kind: TagKind,
    directory_name: String,
}

impl ManagedVersion {
    pub fn new<T, S>(tag: T, kind: TagKind, directory_name: S) -> Self
    where
        T: Into<Tag>,
        S: Into<String>,
    {
        let tag = tag.into();
        let directory_name = directory_name.into();
        ManagedVersion {
            tag,
            kind,
            directory_name,
        }
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    pub fn set_tag<T: Into<Tag>>(&mut self, tag: T) {
        self.tag = tag.into();
    }

    pub fn kind(&self) -> &TagKind {
        &self.kind
    }

    pub fn set_kind(&mut self, kind: TagKind) {
        self.kind = kind;
    }

    pub fn directory_name(&self) -> &str {
        &self.directory_name
    }

    pub fn set_directory_name<S: Into<String>>(&mut self, name: S) {
        self.directory_name = name.into();
    }
}

impl From<Version> for ManagedVersion {
    fn from(v: Version) -> Self {
        let tag = v.tag().clone();
        let kind = *v.kind();
        ManagedVersion::new(tag, kind, String::new())
    }
}

impl From<&Version> for ManagedVersion {
    fn from(v: &Version) -> Self {
        let tag = v.tag().clone();
        let kind = *v.kind();
        ManagedVersion::new(tag, kind, String::new())
    }
}

impl Versioned for ManagedVersion {
    fn tag(&self) -> &Tag {
        &self.tag
    }

    fn kind(&self) -> &TagKind {
        &self.kind
    }
}

impl<'a> Versioned for &'a ManagedVersion {
    fn tag(&self) -> &Tag {
        &self.tag
    }

    fn kind(&self) -> &TagKind {
        &self.kind
    }
}

impl PartialEq for ManagedVersion {
    fn eq(&self, other: &Self) -> bool {
        self.tag().eq(other.tag()) && self.kind().eq(other.kind())
    }
}

impl<'a> PartialEq<dyn Versioned + 'a> for ManagedVersion {
    fn eq(&self, other: &(dyn Versioned + 'a)) -> bool {
        self.tag().eq(other.tag()) && self.kind().eq(other.kind())
    }
}

impl PartialEq<Box<dyn Versioned>> for ManagedVersion {
    fn eq(&self, other: &Box<dyn Versioned>) -> bool {
        self.tag.eq(other.tag()) && self.kind().eq(other.kind())
    }
}

impl PartialOrd for ManagedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialOrd<dyn Versioned + 'a> for ManagedVersion {
    fn partial_cmp(&self, other: &(dyn Versioned + 'a)) -> Option<Ordering> {
        Some(self.tag().cmp(other.tag()).then(self.kind().cmp(other.kind())))
    }
}

impl PartialOrd<Box<dyn Versioned>> for ManagedVersion {
    fn partial_cmp(&self, other: &Box<dyn Versioned>) -> Option<Ordering> {
        Some(self.tag.cmp(other.tag()).then(self.kind.cmp(other.kind())))
    }
}

impl Eq for ManagedVersion {}

impl Ord for ManagedVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tag().cmp(other.tag()).then(self.kind().cmp(other.kind()))
    }
}

impl Display for ManagedVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.tag, self.kind.compatibility_tool_kind())
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ManagedVersions {
    versions: Vec<ManagedVersion>,
}

impl ManagedVersions {
    pub fn new(items: Vec<ManagedVersion>) -> Self {
        ManagedVersions { versions: items }
    }

    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let managed_versions = match fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json)?,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    ManagedVersions::default()
                } else {
                    bail!("Could not convert managed_versions.json to struct");
                }
            }
        };

        Ok(managed_versions)
    }

    pub fn write_to_file(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string(&self).context("Could not convert managed version struct to json")?;

        fs::write(path, json).context("Could not write changes to managed_versions.json")?;

        Ok(())
    }

    fn get_version_index(&self, version: &dyn Versioned) -> Option<usize> {
        self.versions.iter().position(|i| i.eq(version))
    }

    pub fn find_latest_by_kind(&self, kind: &TagKind) -> Option<ManagedVersion> {
        self.versions
            .iter()
            .filter(|v| v.kind().eq(kind))
            .max_by(|a, b| a.tag().semver().cmp(&b.tag().semver()))
            .cloned()
    }

    pub fn add(&mut self, version: ManagedVersion) -> ManagedVersion {
        self.versions.push(version.clone());
        version
    }

    pub fn remove(&mut self, version: &dyn Versioned) -> Option<ManagedVersion> {
        match self.get_version_index(version) {
            Some(index) => Some(self.versions.swap_remove(index)),
            None => None,
        }
    }

    pub fn find_version(&self, version: &dyn Versioned) -> Option<ManagedVersion> {
        self.get_version_index(version)
            .and_then(|index| self.versions.get(index).cloned())
    }

    pub fn latest_versions(&self) -> Vec<ManagedVersion> {
        let kinds = TagKind::values();
        let mut versions = Vec::with_capacity(kinds.len());
        for kind in &kinds {
            if let Some(v) = self.find_latest_by_kind(kind) {
                versions.push(v);
            }
        }
        versions
    }

    pub fn vec_mut(&mut self) -> &mut Vec<ManagedVersion> {
        &mut self.versions
    }

    pub fn vec_ref(&self) -> &Vec<ManagedVersion> {
        &self.versions
    }
}

impl Clone for ManagedVersions {
    fn clone(&self) -> Self {
        ManagedVersions::new(self.versions.clone())
    }
}

impl Default for ManagedVersions {
    fn default() -> Self {
        ManagedVersions::new(Vec::new())
    }
}

impl IntoIterator for ManagedVersions {
    type Item = ManagedVersion;
    type IntoIter = IntoIter<ManagedVersion>;

    fn into_iter(self) -> Self::IntoIter {
        self.versions.into_iter()
    }
}

impl<'a> IntoIterator for &'a ManagedVersions {
    type Item = &'a ManagedVersion;
    type IntoIter = Iter<'a, ManagedVersion>;

    fn into_iter(self) -> Self::IntoIter {
        self.versions.iter()
    }
}

impl Deref for ManagedVersions {
    type Target = Vec<ManagedVersion>;

    fn deref(&self) -> &Self::Target {
        &self.versions
    }
}

impl DerefMut for ManagedVersions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.versions
    }
}

#[cfg(test)]
mod managed_version_tests {
    use super::*;

    fn setup_version() -> ManagedVersion {
        ManagedVersion::new(Tag::from("6.20-GE-1"), TagKind::Proton, "Proton-6.20-GE-1")
    }

    #[test]
    fn get_tag() {
        let tag = "6.20-GE-1";
        let managed_version = setup_version();
        assert_eq!(managed_version.tag(), &Tag::from(tag));
    }

    #[test]
    fn set_tag() {
        let tag = "6.20-GE-1";
        let mut managed_version = setup_version();
        managed_version.set_tag("6.20-GE-1");
        assert_eq!(managed_version.tag(), &Tag::from(tag));
    }

    #[test]
    fn get_kind() {
        let managed_version = setup_version();
        assert_eq!(managed_version.kind(), &TagKind::Proton);
    }

    #[test]
    fn set_kind() {
        let mut managed_version = setup_version();
        managed_version.set_kind(TagKind::wine());
        assert_eq!(managed_version.kind(), &TagKind::wine());
    }

    #[test]
    fn get_directory_name() {
        let managed_version = setup_version();
        assert_eq!(managed_version.directory_name(), "Proton-6.20-GE-1");
    }

    #[test]
    fn set_directory_name() {
        let mut managed_version = setup_version();
        managed_version.set_directory_name("Test");
        assert_eq!(managed_version.directory_name(), "Test");
    }
}

#[cfg(test)]
mod managed_versions_tests {
    use std::fs::File;
    use std::io::Write;

    use assert_fs::TempDir;
    use ge_man_lib::tag::TagKind;
    use lazy_static::lazy_static;

    use super::*;

    lazy_static! {
        static ref VERSIONS: Vec<ManagedVersion> = vec![
            ManagedVersion::from(Version::proton("6.20-GE-1")),
            ManagedVersion::from(Version::proton("6.19-GE-2")),
            ManagedVersion::from(Version::wine("6.20-GE-1")),
            ManagedVersion::from(Version::wine("6.19-GE-2")),
            ManagedVersion::from(Version::lol("6.16-GE-3-LoL")),
            ManagedVersion::from(Version::lol("6.16-2-GE-Lol")),
        ];
    }

    #[test]
    fn latest_by_kind() {
        let managed_versions = ManagedVersions::new(VERSIONS.clone());

        let latest_proton = managed_versions.find_latest_by_kind(&TagKind::Proton).unwrap();
        let latest_wine = managed_versions.find_latest_by_kind(&TagKind::wine()).unwrap();
        let latest_lol = managed_versions.find_latest_by_kind(&TagKind::lol()).unwrap();
        assert_eq!(
            latest_proton,
            ManagedVersion::new(Tag::from("6.20-GE-1"), TagKind::Proton, String::new())
        );
        assert_eq!(
            latest_wine,
            ManagedVersion::new(Tag::from("6.20-GE-1"), TagKind::wine(), String::new())
        );
        assert_eq!(
            latest_lol,
            ManagedVersion::new(Tag::from("6.16-GE-3-LoL"), TagKind::lol(), String::new())
        );
    }

    #[test]
    fn latest_versions() {
        let managed_versions = ManagedVersions::new(VERSIONS.clone());
        let result = managed_versions.latest_versions();

        assert_eq!(
            result,
            vec![
                ManagedVersion::new(Tag::from("6.20-GE-1"), TagKind::Proton, String::new()),
                ManagedVersion::new(Tag::from("6.20-GE-1"), TagKind::wine(), String::new()),
                ManagedVersion::new(Tag::from("6.16-GE-3-LoL"), TagKind::lol(), String::new()),
            ]
        );
    }

    #[test]
    fn add() {
        let mut managed_versions = ManagedVersions::default();
        let version = ManagedVersion::from(Version::proton("6.20-GE-1"));
        managed_versions.add(version);
        assert!(managed_versions.find_version(&Version::proton("6.20-GE-1")).is_some());
    }

    #[test]
    fn remove_existing_version() {
        let version = ManagedVersion::from(Version::proton("6.20-GE-1"));
        let mut managed_versions = ManagedVersions::new(vec![version]);
        managed_versions.remove(&Version::proton("6.20-GE-1")).unwrap();
        assert!(!managed_versions.find_version(&Version::proton("6.20-GE-1")).is_some());
        assert!(managed_versions.vec_mut().is_empty());
    }

    #[test]
    fn remove_version_that_does_not_exist() {
        let mut managed_versions = ManagedVersions::default();
        let option = managed_versions.remove(&Version::proton("6.20-GE-1"));
        assert_eq!(option, None);
    }

    #[test]
    fn has_version() {
        let version = ManagedVersion::from(Version::proton("6.19-GE-1"));
        let managed_versions = ManagedVersions::new(vec![version]);
        assert!(!managed_versions.find_version(&Version::proton("6.20-GE-1")).is_some());
        assert!(managed_versions.find_version(&Version::proton("6.19-GE-1")).is_some());
    }

    #[test]
    fn read_invalid_json() {
        let tmp_dir = TempDir::new().unwrap();
        let path = tmp_dir.join("version_mng.json");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"{").unwrap();

        let result = ManagedVersions::from_file(&path);
        assert!(result.is_err());

        drop(file);
        tmp_dir.close().unwrap();
    }
}
