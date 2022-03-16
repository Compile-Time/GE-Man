use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::path::Path;

use lazy_static::lazy_static;
use regex::{Captures, Match, Regex};
use serde::{Deserialize, Serialize};

use crate::error::TagKindError;

pub const PROTON: &str = "Proton";
pub const WINE: &str = "Wine";
pub const LOL_WINE: &str = "LoL_Wine";

const RELEASE_CANDIDATE_MARKER: &str = "rc";
const FIRST_GROUP: usize = 1;

lazy_static! {
    static ref NUMBERS: Regex = Regex::new(r"(\d+)").unwrap();
    static ref TAG_MARKERS: Vec<String> = vec![String::from("rc"), String::from("LoL"), String::from("MF")];
}

mod semver {
    use super::*;

    pub fn from_git_tag(git_tag: &String) -> Option<String> {
        let captures: Vec<Captures> = NUMBERS.captures_iter(&git_tag).collect();

        let semver = if git_tag.contains(RELEASE_CANDIDATE_MARKER) {
            if let Some(rc_match) = release_candidate_match(&git_tag, &captures) {
                let captures_without_rc: Vec<Captures> = captures
                    .into_iter()
                    .filter(|cap| cap.get(FIRST_GROUP).unwrap().ne(&rc_match))
                    .collect();
                let mut semver = construct_string(&captures_without_rc);
                let rc_marker = format!("-rc{}", rc_match.as_str());

                semver.push_str(&rc_marker);
                semver
            } else {
                panic!("Git tag is not parsable!");
            }
        } else {
            let mut semver = construct_string(&captures);

            for marker in &*TAG_MARKERS {
                if git_tag.contains(marker) {
                    let marker = format!("-{}", marker);
                    semver.push_str(&marker);
                }
            }

            semver
        };

        Some(semver)
    }

    fn construct_string(captures: &[Captures]) -> String {
        let mut semver = String::new();
        for cap in captures {
            semver.push_str(&cap[1]);
            semver.push('.');
        }

        // In the case that we do not have enough matches to fill the semver string we fill it with empty zeros.
        let captures_len = captures.len();
        if captures_len < 3 {
            for _ in captures_len..3 {
                semver.push_str("0.");
            }
        }

        semver.pop();
        semver
    }

    fn release_candidate_match<'a>(git_tag: &String, captures: &Vec<Captures<'a>>) -> Option<Match<'a>> {
        for cap in captures.iter().skip(1) {
            let rc_query = format!("{}{}", RELEASE_CANDIDATE_MARKER, &cap[FIRST_GROUP]);
            if git_tag.contains(&rc_query) {
                // Since the regex match is equal to the group the first group is always present.
                return Some(cap.get(FIRST_GROUP).unwrap().clone());
            }
        }
        None
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tag {
    value: String,
    semver: Option<String>,
}

impl Tag {
    pub fn new<S: Into<String>>(git_tag: S) -> Self {
        let value = git_tag.into();
        let semver = semver::from_git_tag(&value);

        Tag { value, semver }
    }

    pub fn compare_semver(a: &Tag, b: &Tag) -> Ordering {
        a.semver().cmp(&b.semver())
    }

    pub fn semver(&self) -> Option<&String> {
        self.semver.as_ref()
    }

    pub fn set_semver<S: Into<String>>(&mut self, semver: Option<S>) {
        self.semver = semver.map(|val| val.into());
    }

    pub fn value(&self) -> &String {
        &self.value
    }
}

impl Default for Tag {
    fn default() -> Self {
        Tag::new("")
    }
}

impl From<String> for Tag {
    fn from(s: String) -> Self {
        Tag::new(&s)
    }
}

impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Tag::new(s)
    }
}

impl From<Option<String>> for Tag {
    fn from(opt: Option<String>) -> Self {
        match opt {
            Some(str) => Tag::new(str),
            None => Tag::default(),
        }
    }
}

impl From<Option<&str>> for Tag {
    fn from(opt: Option<&str>) -> Self {
        match opt {
            Some(str) => Tag::new(str),
            None => Tag::default(),
        }
    }
}

impl AsRef<Path> for Tag {
    fn as_ref(&self) -> &Path {
        self.value.as_ref()
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        self.value.as_ref()
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl PartialEq<Tag> for Tag {
    fn eq(&self, other: &Tag) -> bool {
        self.value().eq(other.value())
    }
}

impl PartialOrd<Tag> for Tag {
    fn partial_cmp(&self, other: &Tag) -> Option<Ordering> {
        self.value().partial_cmp(other.value())
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value().cmp(other.value())
    }
}

impl Eq for Tag {}

impl PartialEq<String> for Tag {
    fn eq(&self, other: &String) -> bool {
        self.value.eq(other)
    }
}

impl PartialEq<str> for Tag {
    fn eq(&self, other: &str) -> bool {
        self.value.eq(other)
    }
}

impl PartialOrd<String> for Tag {
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        self.value().partial_cmp(other)
    }
}

impl PartialOrd<str> for Tag {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.value().as_str().partial_cmp(other)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[serde(tag = "type")]
pub enum TagKind {
    Proton,
    Wine { kind: WineTagKind },
}

impl TagKind {
    pub fn wine() -> TagKind {
        TagKind::Wine {
            kind: WineTagKind::WineGe,
        }
    }

    pub fn lol() -> TagKind {
        TagKind::Wine {
            kind: WineTagKind::LolWineGe,
        }
    }

    pub fn values() -> Vec<TagKind> {
        vec![TagKind::Proton, TagKind::wine(), TagKind::lol()]
    }

    pub fn compatibility_tool_name(&self) -> String {
        let name = match self {
            TagKind::Proton => "Proton GE",
            TagKind::Wine { kind } => match kind {
                WineTagKind::WineGe => "Wine GE",
                WineTagKind::LolWineGe => "Wine GE (LoL)",
            },
        };
        String::from(name)
    }

    pub fn str(&self) -> String {
        let name = match self {
            TagKind::Proton => PROTON,
            TagKind::Wine { kind } => match kind {
                WineTagKind::WineGe => WINE,
                WineTagKind::LolWineGe => LOL_WINE,
            },
        };
        String::from(name)
    }

    fn from_str(str: &str) -> Result<Self, TagKindError> {
        let kind = match str {
            PROTON => TagKind::Proton,
            WINE => TagKind::wine(),
            LOL_WINE => TagKind::lol(),
            _ => return Err(TagKindError::UnknownString),
        };
        Ok(kind)
    }
}

impl From<&WineTagKind> for TagKind {
    fn from(kind: &WineTagKind) -> Self {
        TagKind::Wine { kind: *kind }
    }
}

impl From<WineTagKind> for TagKind {
    fn from(kind: WineTagKind) -> Self {
        TagKind::Wine { kind }
    }
}

impl TryFrom<&str> for TagKind {
    type Error = TagKindError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        TagKind::from_str(value)
    }
}

impl TryFrom<String> for TagKind {
    type Error = TagKindError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        TagKind::from_str(&value)
    }
}

impl Display for TagKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.str())
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
#[serde(tag = "type")]
pub enum WineTagKind {
    WineGe,
    LolWineGe,
}

impl From<&str> for WineTagKind {
    fn from(string: &str) -> Self {
        match string {
            s if s.eq(WINE) => WineTagKind::WineGe,
            s if s.eq(LOL_WINE) => WineTagKind::LolWineGe,
            _ => panic!("Cannot map string to LutrisVersionKind"),
        }
    }
}

#[cfg(test)]
mod tag_tests {
    use test_case::test_case;

    use super::*;

    #[test_case("6.20-GE-1" => Some(String::from("6.20.1")))]
    #[test_case("6.20-GE-0" => Some(String::from("6.20.0")))]
    #[test_case("6.20-GE" => Some(String::from("6.20.0")))]
    #[test_case("6.16-GE-3-LoL" => Some(String::from("6.16.3-LoL")))]
    #[test_case("6.16-2-GE-LoL" => Some(String::from("6.16.2-LoL")))]
    #[test_case("6.16-GE-LoL" => Some(String::from("6.16.0-LoL")))]
    #[test_case("6.16-GE-0-LoL" => Some(String::from("6.16.0-LoL")))]
    #[test_case("6.16-0-GE-LoL" => Some(String::from("6.16.0-LoL")))]
    #[test_case("7.0rc3-GE-1" => Some(String::from("7.0.1-rc3")))]
    #[test_case("7.0rc3-GE-0" => Some(String::from("7.0.0-rc3")))]
    #[test_case("7.0rc3-GE" => Some(String::from("7.0.0-rc3")))]
    #[test_case("7.0-GE" => Some(String::from("7.0.0")))]
    #[test_case("7.0-GE-1" => Some(String::from("7.0.1")))]
    #[test_case("GE-Proton7-8" => Some(String::from("7.8.0")))]
    #[test_case("GE-Proton7-4" => Some(String::from("7.4.0")))]
    #[test_case("5.11-GE-1-MF" => Some(String::from("5.11.1-MF")))]
    #[test_case("proton-3.16-5" => Some(String::from("3.16.5")))]
    #[test_case("5.0-rc5-GE-1" => Some(String::from("5.0.1-rc5")))]
    fn get_semver_format(tag_str: &str) -> Option<String> {
        let tag = Tag::new(tag_str);
        tag.semver().cloned()
    }

    #[test]
    fn create_from_json() {
        let tag: Tag = serde_json::from_str(r###"{"value": "6.20-GE-1", "major_minor": "6.20.1"}"###).unwrap();
        assert_eq!(tag.value(), "6.20-GE-1");
    }
}

#[cfg(test)]
mod tag_kind_tests {
    use test_case::test_case;

    use super::*;

    #[test]
    fn wine() {
        let kind = TagKind::wine();
        assert_eq!(
            kind,
            TagKind::Wine {
                kind: WineTagKind::WineGe
            }
        )
    }

    #[test]
    fn lol() {
        let kind = TagKind::lol();
        assert_eq!(
            kind,
            TagKind::Wine {
                kind: WineTagKind::LolWineGe
            }
        );
    }

    #[test]
    fn values() {
        let values = TagKind::values();
        assert_eq!(
            values,
            vec![
                TagKind::Proton,
                TagKind::Wine {
                    kind: WineTagKind::WineGe
                },
                TagKind::Wine {
                    kind: WineTagKind::LolWineGe
                },
            ]
        );
    }

    #[test_case(TagKind::Proton => "Proton GE"; "Correct app name should be returned for Proton")]
    #[test_case(TagKind::wine() => "Wine GE"; "Correct app name should be returned for Wine")]
    #[test_case(TagKind::lol() => "Wine GE (LoL)"; "Correct app name should be returned for Wine (LoL)")]
    fn get_compatibility_tool_name(kind: TagKind) -> String {
        kind.compatibility_tool_name()
    }

    #[test_case(TagKind::Proton => "PROTON"; "Correct type name should be returned for Proton")]
    #[test_case(TagKind::wine() => "WINE"; "Correct type name should be returned for Wine")]
    #[test_case(TagKind::lol() => "LOL_WINE"; "Correct type name should be returned for Wine (LoL)")]
    fn get_type_name(kind: TagKind) -> String {
        kind.str()
    }
}
