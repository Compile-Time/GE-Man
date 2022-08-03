use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};

use ge_man_lib::tag::{Tag, TagKind};

use crate::label::Label;

// TODO: With Version and ManagedVersion diverging due to the label attribute this trait loses its benefit. It should
//  be removed.
pub trait Versioned {
    fn tag(&self) -> &Tag;
    fn kind(&self) -> &TagKind;
}

impl<'a> PartialEq for dyn Versioned + 'a {
    fn eq(&self, other: &Self) -> bool {
        self.tag().eq(other.tag()) && self.kind().eq(other.kind())
    }
}

impl<T: Versioned> PartialEq<T> for dyn Versioned {
    fn eq(&self, other: &T) -> bool {
        self.tag().eq(other.tag()) && self.kind().eq(other.kind())
    }
}

impl<'a> PartialOrd for dyn Versioned + 'a {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Versioned> PartialOrd<T> for dyn Versioned {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        Some(self.tag().cmp(other.tag()).then(self.kind().cmp(other.kind())))
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
    label: Option<Label>,
}

impl Version {
    pub fn new<T>(label: Option<Label>, tag: T, kind: TagKind) -> Version
    where
        T: Into<Tag>,
    {
        let tag = tag.into();
        Version { label, tag, kind }
    }

    pub fn proton(label: Option<Label>, tag: &str) -> Self {
        Version::new(label, tag, TagKind::Proton)
    }

    pub fn wine(label: Option<Label>, tag: &str) -> Self {
        Version::new(label, tag, TagKind::wine())
    }

    pub fn lol(label: Option<Label>, tag: &str) -> Self {
        Version::new(label, tag, TagKind::lol())
    }

    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    pub fn set_label(&mut self, label: Option<Label>) {
        self.label = label;
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

#[cfg(test)]
mod version_tests {
    use test_case::test_case;

    use crate::fixture;

    use super::*;

    #[test]
    fn new() {
        let tag = "6.20-GE-1";
        let version = Version::new(Some(Label::new(tag).unwrap()), tag, TagKind::Proton);
        assert_eq!(version.tag, Tag::new(tag));
        assert_eq!(version.kind, TagKind::Proton);
    }

    #[test]
    fn proton() {
        let tag = "6.20-GE-1";
        let label = Label::new(tag).unwrap();
        let version = Version::proton(Some(label), tag);
        assert_eq!(version.tag, Tag::new(tag));
        assert_eq!(version.kind, TagKind::Proton);
    }

    #[test]
    fn wine() {
        let tag = "6.20-GE-1";
        let label = Label::new(tag).unwrap();
        let version = Version::wine(Some(label), tag);
        assert_eq!(version.tag, Tag::new(tag));
        assert_eq!(version.kind, TagKind::wine());
    }

    #[test]
    fn lol() {
        let tag = "6.16-GE-3-LoL";
        let label = Label::new(tag).unwrap();
        let version = Version::lol(Some(label), tag);
        assert_eq!(version.tag, Tag::new(tag));
        assert_eq!(version.kind, TagKind::lol());
    }

    #[test_case(fixture::version::v6_20_1_proton(), fixture::version::v6_20_1_proton() => true; "Proton versions should be equal")]
    #[test_case(fixture::version::v6_20_1_proton(), fixture::version::v6_19_1_proton() => false; "Proton versions should not be equal")]
    #[test_case(fixture::version::v6_20_1_wine(), fixture::version::v6_20_1_wine() => true; "Wine versions should be equal")]
    #[test_case(fixture::version::v6_20_1_wine(), fixture::version::v6_19_1_wine() => false; "Wine versions should not be equal")]
    #[test_case(fixture::version::v6_16_3_lol(), fixture::version::v6_16_3_lol() => true; "LoL versions should be equal")]
    #[test_case(fixture::version::v6_16_3_lol(), fixture::version::v6_16_2_lol() => false; "LoL versions should not be equal")]
    #[test_case(fixture::version::v6_20_1_proton(), fixture::version::v6_20_1_wine() => false; "Proton version should not be equal to Wine version")]
    #[test_case(fixture::version::v6_20_1_proton(), fixture::version::v6_16_3_lol() => false; "Proton version should not be equal to LoL version")]
    #[test_case(fixture::version::v6_20_1_wine(), fixture::version::v6_20_1_lol() => false; "Wine version should not be equal to LoL version")]
    #[test_case(fixture::version::v6_20_1_proton_custom_label(), fixture::version::v6_20_1_proton_custom_label() => true; "Same custom label should be equal")]
    #[test_case(fixture::version::v6_20_1_proton_custom_label(), fixture::version::v6_20_1_proton_custom_label_2() => false; "Different custom label should not be equal")]
    fn eq(version1: Version, version2: Version) -> bool {
        version1.eq(&version2)
    }

    #[test_case(fixture::version::v6_20_1_proton(), fixture::version::v6_20_1_proton() => true; "Order should be equal for Proton tags")]
    #[test_case(fixture::version::v6_20_1_proton_custom_label(), fixture::version::v6_20_1_proton_custom_label() => true; "Order should be equal for Proton tags with same label")]
    #[test_case(fixture::version::v6_20_1_proton_custom_label(), fixture::version::v6_20_1_proton_custom_label_2() => true; "Order should not be equal for Proton tags with different labels")]
    #[test_case(fixture::version::v6_20_1_proton(), fixture::version::v6_20_1_wine() => false; "Order should not be equal for Proton and Wine tag")]
    #[test_case(fixture::version::v6_20_1_proton(), fixture::version::v6_16_3_lol() => false; "Order should not be equal for Proton and LoL tag")]
    fn cmp(version1: Version, version2: Version) -> bool {
        version1.cmp(&version2).eq(&Ordering::Equal)
    }
}
