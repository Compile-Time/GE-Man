use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};

use ge_man_lib::tag::{Tag, TagKind};

use crate::data::ManagedVersion;

pub trait Versioned {
    fn tag(&self) -> &Tag;
    fn kind(&self) -> &TagKind;
}

impl<'a> PartialEq for dyn Versioned + 'a {
    fn eq(&self, other: &Self) -> bool {
        self.tag().eq(other.tag()) && self.kind().eq(other.kind())
    }
}

impl<'a> PartialOrd for dyn Versioned + 'a {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for dyn Versioned + 'a {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tag().cmp(other.tag()).then(self.kind().cmp(other.kind()))
    }
}

impl<'a> Eq for dyn Versioned + 'a {}

impl<'a> fmt::Debug for dyn Versioned + 'a {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Box<dyn Versioned>")
            .field("tag", self.tag())
            .field("kind", self.kind())
            .finish()
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Version {
    tag: Tag,
    kind: TagKind,
}

impl Version {
    pub fn new<T>(tag: T, kind: TagKind) -> Version
    where
        T: Into<Tag>,
    {
        let tag = tag.into();
        Version { tag, kind }
    }

    pub fn proton(tag: &str) -> Self {
        Version::new(tag, TagKind::Proton)
    }

    pub fn wine(tag: &str) -> Self {
        Version::new(tag, TagKind::wine())
    }

    pub fn lol(tag: &str) -> Self {
        Version::new(tag, TagKind::lol())
    }

    pub fn into_managed(self, file_name: String) -> ManagedVersion {
        let mut v = ManagedVersion::from(self);
        v.set_directory_name(file_name);
        v
    }
}

impl Versioned for Version {
    fn tag(&self) -> &Tag {
        &self.tag
    }

    fn kind(&self) -> &TagKind {
        &self.kind
    }
}

impl<'a> PartialEq<dyn Versioned + 'a> for Version {
    fn eq(&self, other: &(dyn Versioned + 'a)) -> bool {
        self.tag().eq(other.tag()) && self.kind().eq(other.kind())
    }
}

impl<'a> PartialOrd<dyn Versioned + 'a> for Version {
    fn partial_cmp(&self, other: &(dyn Versioned + 'a)) -> Option<Ordering> {
        Some(self.tag().cmp(other.tag()).then(self.kind().cmp(other.kind())))
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.tag, self.kind.compatibility_tool_kind())
    }
}

impl From<Box<dyn Versioned>> for Version {
    fn from(versioned: Box<dyn Versioned>) -> Self {
        Version::new(versioned.tag().clone(), versioned.kind().clone())
    }
}

#[cfg(test)]
mod version_tests {
    use test_case::test_case;

    use super::*;

    #[test]
    fn new() {
        let tag = "6.20-GE-1";
        let version = Version::new(tag, TagKind::Proton);
        assert_eq!(version.tag, Tag::new(tag));
        assert_eq!(version.kind, TagKind::Proton);
    }

    #[test]
    fn proton() {
        let tag = "6.20-GE-1";
        let version = Version::proton(tag);
        assert_eq!(version.tag, Tag::new(tag));
        assert_eq!(version.kind, TagKind::Proton);
    }

    #[test]
    fn wine() {
        let tag = "6.20-GE-1";
        let version = Version::wine(tag);
        assert_eq!(version.tag, Tag::new(tag));
        assert_eq!(version.kind, TagKind::wine());
    }

    #[test]
    fn lol() {
        let tag = "6.16-GE-3-LoL";
        let version = Version::lol(tag);
        assert_eq!(version.tag, Tag::new(tag));
        assert_eq!(version.kind, TagKind::lol());
    }

    #[test_case(Version::proton("6.20-GE-1"), Version::proton("6.20-GE-1") => true; "Proton versions should be equal")]
    #[test_case(Version::proton("6.20-GE-1"), Version::proton("6.19-GE-1") => false; "Proton versions should not be equal")]
    #[test_case(Version::proton("6.20-GE-1"), Version::proton("6.20-GE-1") => true; "Wine versions should be equal")]
    #[test_case(Version::proton("6.20-GE-1"), Version::proton("6.19-GE-1") => false; "Wine versions should not be equal")]
    #[test_case(Version::proton("6.16-GE-3-LoL"), Version::proton("6.16-GE-3-LoL") => true; "LoL versions should be equal")]
    #[test_case(Version::proton("6.16-GE-3-LoL"), Version::proton("6.16-2-GE-LoL") => false; "LoL versions should not be equal")]
    #[test_case(Version::proton("6.20-GE-1"), Version::wine("6.20-GE-1") => false; "Proton version should not be equal to Wine version")]
    #[test_case(Version::proton("6.20-GE-1"), Version::lol("6.20-GE-1") => false; "Proton versions should not be equal to LoL version")]
    fn eq(version1: Version, version2: Version) -> bool {
        version1.eq(&version2)
    }

    #[test_case(Version::proton("6.20-GE-1"), Version::proton("6.20-GE-1") => true; "Order should be equal Proton tags")]
    #[test_case(Version::proton("6.20-GE-1"), Version::wine("6.20-GE-1") => false; "Order should not be equal Proton and Wine tag")]
    #[test_case(Version::proton("6.20-GE-1"), Version::lol("6.20-GE-1") => false; "Order should not be equal Proton and LoL tag")]
    fn cmp(version1: Version, version2: Version) -> bool {
        version1.cmp(&version2).eq(&Ordering::Equal)
    }
}
