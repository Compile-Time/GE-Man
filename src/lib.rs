mod compat_tool_app;
mod label;
mod message;

pub mod clap;
pub mod command_execution;
pub mod command_input;
pub mod config;
pub mod data;
pub mod filesystem;
pub mod path;
pub mod progress;
pub mod version;

#[cfg(test)]
mod fixture {
    use ge_man_lib::tag::TagKind;

    use crate::label::Label;

    pub mod managed_version {
        use crate::data::ManagedVersion;

        use super::*;

        pub fn v7_22_proton() -> ManagedVersion {
            ManagedVersion::new(
                Label::new("GE-Proton7-22").unwrap(),
                "GE-Proton7-22",
                TagKind::Proton,
                "GE-Proton7-22",
            )
        }

        pub fn v6_21_1_proton() -> ManagedVersion {
            ManagedVersion::new(
                Label::new("6.21-GE-1").unwrap(),
                "6.21-GE-1",
                TagKind::Proton,
                "Proton-6.21-GE-1_L6.21-GE-1",
            )
        }

        pub fn v6_21_1_wine() -> ManagedVersion {
            ManagedVersion::new(
                Label::new("6.21-GE-1").unwrap(),
                "6.21-GE-1",
                TagKind::wine(),
                "Wine-6.21-GE-1_L6.21-GE-1",
            )
        }

        pub fn v6_21_1_lol() -> ManagedVersion {
            ManagedVersion::new(
                Label::new("6.21-GE-1").unwrap(),
                "6.21-GE-1",
                TagKind::lol(),
                "Wine-6.21-GE-1-LoL_L6.21-GE-1",
            )
        }

        pub fn v6_21_2_proton() -> ManagedVersion {
            ManagedVersion::new(
                Label::new("6.21-GE-2").unwrap(),
                "6.21-GE-2",
                TagKind::Proton,
                "Proton-6.21-GE-2_L6.21-GE-2",
            )
        }

        pub fn v6_21_2_with_kind(kind: TagKind) -> ManagedVersion {
            ManagedVersion::new(
                Label::new("6.21-GE-2").unwrap(),
                "6.21-GE-2",
                kind,
                "Proton-6.21-GE-2_L6.21-GE-2",
            )
        }

        pub fn v6_22_1_proton() -> ManagedVersion {
            ManagedVersion::new(
                Label::new("6.22-GE-1").unwrap(),
                "6.22-GE-1",
                TagKind::Proton,
                "Proton-6.22-GE-1_L6.22-GE-1",
            )
        }

        pub fn v6_19_1_proton() -> ManagedVersion {
            let tag = "6.19-GE-1";
            ManagedVersion::new(
                Label::new(tag).unwrap(),
                tag,
                TagKind::Proton,
                "Proton-6.19-GE-1_L6.20-GE-1",
            )
        }

        pub fn v6_19_1_wine() -> ManagedVersion {
            let tag = "6.19-GE-1";
            ManagedVersion::new(
                Label::new(tag).unwrap(),
                tag,
                TagKind::wine(),
                "Wine-6.19-GE-1_L6.20-GE-1",
            )
        }

        pub fn v6_20_1_with_kind(kind: TagKind) -> ManagedVersion {
            ManagedVersion::new(
                Label::new("6.20-GE-1").unwrap(),
                "6.20-GE-1",
                kind,
                "Wine-6.20-GE-1-LoL_L6.20-GE-1",
            )
        }

        pub fn v6_20_1_proton() -> ManagedVersion {
            let tag = "6.20-GE-1";
            ManagedVersion::new(
                Label::new(tag).unwrap(),
                tag,
                TagKind::Proton,
                "Proton-6.20-GE-1_L6.20-GE-1",
            )
        }

        pub fn v6_20_1_proton_custom_label() -> ManagedVersion {
            let tag = "6.20-GE-1";
            ManagedVersion::new(
                Label::new("custom-label").unwrap(),
                tag,
                TagKind::Proton,
                "Proton-6.20-GE-1_L6.20-GE-1",
            )
        }

        pub fn v6_20_1_wine() -> ManagedVersion {
            let tag = "6.20-GE-1";
            ManagedVersion::new(
                Label::new(tag).unwrap(),
                tag,
                TagKind::wine(),
                "Wine-6.20-GE-1_L6.20-GE-1",
            )
        }

        pub fn v6_20_1_lol() -> ManagedVersion {
            ManagedVersion::new(
                Label::new("6.20-GE-1").unwrap(),
                "6.20-GE-1",
                TagKind::lol(),
                "Wine-6.20-GE-1-LoL_L6.20-GE-1",
            )
        }

        pub fn v6_16_3_lol() -> ManagedVersion {
            let tag = "6.16-GE-3-LoL";
            ManagedVersion::new(
                Label::new(tag).unwrap(),
                tag,
                TagKind::lol(),
                "Wine-6.16-GE-3-LoL_L6.16-GE-3-LoL",
            )
        }

        pub fn v6_16_2_lol() -> ManagedVersion {
            let tag = "6.16-2-GE-LoL";
            ManagedVersion::new(
                Label::new(tag).unwrap(),
                tag,
                TagKind::lol(),
                "Wine-6.16-2-GE-LoL_L6.16-2-GE-LoL",
            )
        }
    }

    pub mod version {
        use crate::version::Version;

        use super::*;

        pub fn v6_19_1_proton() -> Version {
            let tag = "6.19-GE-1";
            Version::proton(Some(Label::new(tag).unwrap()), tag)
        }

        pub fn v6_19_1_wine() -> Version {
            let tag = "6.19-GE-1";
            Version::wine(Some(Label::new(tag).unwrap()), tag)
        }

        pub fn v6_20_1_proton() -> Version {
            let tag = "6.20-GE-1";
            Version::proton(Some(Label::new(tag).unwrap()), tag)
        }

        pub fn v6_20_1_proton_custom_label() -> Version {
            let tag = "6.20-GE-1";
            Version::proton(Some(Label::new("custom-label").unwrap()), tag)
        }

        pub fn v6_20_1_proton_custom_label_2() -> Version {
            let tag = "6.20-GE-1";
            Version::proton(Some(Label::new("custom-label-2").unwrap()), tag)
        }

        pub fn v6_20_2_proton() -> Version {
            let tag = "6.20-GE-2";
            Version::proton(Some(Label::new(tag).unwrap()), tag)
        }

        pub fn v6_20_1_wine() -> Version {
            let tag = "6.20-GE-1";
            Version::wine(Some(Label::new(tag).unwrap()), tag)
        }

        pub fn v6_20_1_lol() -> Version {
            let tag = "6.20-GE-1";
            Version::lol(Some(Label::new(tag).unwrap()), tag)
        }

        pub fn v6_16_3_lol() -> Version {
            let tag = "6.16-GE-3-LoL";
            Version::lol(Some(Label::new(tag).unwrap()), tag)
        }

        pub fn v6_16_2_lol() -> Version {
            let tag = "6.16-2-GE-LoL";
            Version::lol(Some(Label::new(tag).unwrap()), tag)
        }
    }
}
