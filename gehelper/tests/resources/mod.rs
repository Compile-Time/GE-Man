pub mod json {
    use lazy_static::lazy_static;

    lazy_static! {
        static ref RESOURCES: &'static str = "tests/resources";
        static ref MANAGED_VERSIONS_JSON: String = format!("{}/{}", *RESOURCES, "managed_versions_json");

        pub static ref EMPTY: String = format!("{}/{}", *MANAGED_VERSIONS_JSON, "empty.json");
        pub static ref INVALID: String = format!("{}/{}", *MANAGED_VERSIONS_JSON, "invalid.json");
    }

    pub mod in_use {
        use gcm_lib::tag::{TagKind, WineTagKind};

        use super::*;

        lazy_static! {
            static ref IN_USE: String = format!("{}/{}", *super::MANAGED_VERSIONS_JSON, "in_use");

            pub static ref EVERY_VERSION: String = format!("{}/{}", *IN_USE, "every_version.json");
            pub static ref EVERY_VERSION_MULTIPLE: String = format!("{}/{}", *IN_USE, "every_version_multiple.json");
            pub static ref INVALID: String = format!("{}/{}", *IN_USE, "invalid.json");
            pub static ref LATEST_VERSIONS: String = format!("{}/{}", *IN_USE, "latest_versions_test.json");
            pub static ref LOL: String = format!("{}/{}", *IN_USE, "lol.json");
            pub static ref PROTON: String = format!("{}/{}", *IN_USE, "proton.json");
            pub static ref PROTON_AND_WINE: String = format!("{}/{}", *IN_USE, "proton_and_wine.json");
            pub static ref PROTON_MULTIPLE: String = format!("{}/{}", *IN_USE, "proton_multiple.json");
            pub static ref WINE: String = format!("{}/{}", *IN_USE, "wine.json");
        }

        pub fn by_app_kind(kind: &TagKind) -> String {
            let resource = match kind {
                TagKind::Proton => &*PROTON,
                TagKind::Wine { kind } => match kind {
                    WineTagKind::WineGe => &*WINE,
                    WineTagKind::LolWineGe => &*LOL,
                }
            };
            resource.clone()
        }
    }

    pub mod unused {
        use gcm_lib::tag::{TagKind, WineTagKind};

        use super::*;

        lazy_static! {
            static ref UNUSED: String = format!("{}/{}", *super::MANAGED_VERSIONS_JSON, "unused");

            pub static ref EVERY_VERSION: String = format!("{}/{}", *UNUSED, "every_version.json");
            pub static ref EVERY_VERSION_MULTIPLE: String = format!("{}/{}", *UNUSED, "every_version_multiple.json");
            pub static ref INVALID: String = format!("{}/{}", *UNUSED, "invalid.json");
            pub static ref LATEST_VERSIONS: String = format!("{}/{}", *UNUSED, "latest_versions_test.json");
            pub static ref LOL: String = format!("{}/{}", *UNUSED, "lol.json");
            pub static ref PROTON: String = format!("{}/{}", *UNUSED, "proton.json");
            pub static ref PROTON_AND_WINE: String = format!("{}/{}", *UNUSED, "proton_and_wine.json");
            pub static ref PROTON_MULTIPLE: String = format!("{}/{}", *UNUSED, "proton_multiple.json");
            pub static ref WINE: String = format!("{}/{}", *UNUSED, "wine.json");
        }

        pub fn by_app_kind(kind: &TagKind) -> String {
            let resource = match kind {
                TagKind::Proton => &*PROTON,
                TagKind::Wine { kind } => match kind {
                    WineTagKind::WineGe => &*WINE,
                    WineTagKind::LolWineGe => &*LOL,
                }
            };
            resource.clone()
        }
    }

    pub mod unformatted {
        use gcm_lib::tag::{TagKind, WineTagKind};

        use super::*;

        lazy_static! {
            static ref IN_USE: String = format!("{}/{}", *super::MANAGED_VERSIONS_JSON, "unformatted");

            pub static ref LOL: String = format!("{}/{}", *IN_USE, "lol.json");
            pub static ref PROTON: String = format!("{}/{}", *IN_USE, "proton.json");
            pub static ref WINE: String = format!("{}/{}", *IN_USE, "wine.json");
        }

        pub fn by_app_kind(kind: &TagKind) -> String {
            let resource = match kind {
                TagKind::Proton => &*PROTON,
                TagKind::Wine { kind } => match kind {
                    WineTagKind::WineGe => &*WINE,
                    WineTagKind::LolWineGe => &*LOL,
                }
            };
            resource.clone()
        }
    }
}

pub mod migrate {
    use gcm_lib::tag::{TagKind, WineTagKind};
    use lazy_static::lazy_static;

    lazy_static! {
        static ref RESOURCES: &'static str = "tests/resources/extern";

        pub static ref PROTON_6_20_GE_1: String = format!("{}/{}", *RESOURCES, "Proton-6.20-GE-1");
        pub static ref WINE_6_20_GE_1: String = format!("{}/{}", *RESOURCES, "Wine-6.20-GE-1");
        pub static ref WINE_6_16_GE_3_LOL: String = format!("{}/{}", *RESOURCES, "Wine-6.16-GE-3-LoL");
    }

    pub fn by_app_kind(kind: &TagKind) -> String {
        let resource = match kind {
            TagKind::Proton => &*PROTON_6_20_GE_1,
            TagKind::Wine { kind } => match kind {
                WineTagKind::WineGe => &*WINE_6_20_GE_1,
                WineTagKind::LolWineGe => &*WINE_6_16_GE_3_LOL,
            }
        };
        resource.clone()
    }
}

pub mod assets {
    use lazy_static::lazy_static;

    lazy_static! {
        static ref RESOURCES: &'static str = "tests/resources/assets";

        pub static ref TEST_TAR_GZ: String = format!("{}/{}", *RESOURCES, "test.tar.gz");
        pub static ref TEST_SHA512SUM: String = format!("{}/{}", *RESOURCES, "test.sha512sum");
    }

    pub mod proton {
        use super::*;

        lazy_static! {
            pub static ref GE_6_19_1: String = format!("{}/{}", *RESOURCES, "Proton-6.19-GE-1.tar.gz");
            pub static ref GE_6_20_1: String = format!("{}/{}", *RESOURCES, "Proton-6.20-GE-1.tar.gz");
        }
    }

    pub mod wine {
        use super::*;

        lazy_static! {
            pub static ref GE_6_19_1: String = format!("{}/{}", *RESOURCES, "Wine-6.19-GE-1.tar.gz");
            pub static ref GE_6_20_1: String = format!("{}/{}", *RESOURCES, "Wine-6.20-GE-1.tar.gz");
            pub static ref GE_6_16_LOL_3: String = format!("{}/{}", *RESOURCES, "Wine-6.16-GE-3-LoL.tar.gz");
            pub static ref GE_6_16_2_LOL: String = format!("{}/{}", *RESOURCES, "Wine-6.16-2-GE-LoL.tar.gz");
        }
    }
}

pub mod responses {
    use lazy_static::lazy_static;

    pub mod releases {
        use gcm_lib::tag::{TagKind, WineTagKind};

        use super::*;

        lazy_static! {
            static ref RESOURCES: &'static str = "tests/resources/responses/releases";

            pub static ref PROTON_GE: String = format!("{}/proton-ge-release.json", *RESOURCES);
            pub static ref WINE_GE: String = format!("{}/wine-ge-release.json", *RESOURCES);
            pub static ref WINE_GE_LOL: String = format!("{}/wine-ge-lol-release.json", *RESOURCES);
        }

        pub fn download_url(server: Option<&str>, tag: &str, kind: &TagKind, file_name: &str) -> String {
            let url = download_url_without_server(tag, kind, file_name);

            if let Some(host) = server {
                format!("{}{}", host, url)
            } else {
                format!("SERVER{}", url)
            }
        }

        pub fn download_url_without_server(tag: &str, kind: &TagKind, file_name: &str) -> String {
            let repo = match kind {
                TagKind::Proton => "proton",
                TagKind::Wine { .. } => "wine"
            };
            format!("/GloriousEggroll/{}-ge-custom/releases/download/{}/{}", repo, tag, file_name)
        }

        pub fn mock_url(kind: &TagKind, server: &str) -> String {
            let file_path = match kind {
                TagKind::Proton => {
                    &*PROTON_GE
                }
                TagKind::Wine { kind: wine_kind } => match wine_kind {
                    WineTagKind::WineGe => &*WINE_GE,
                    WineTagKind::LolWineGe => &*WINE_GE_LOL,
                }
            };
            std::fs::read_to_string(file_path).unwrap().replace("SERVER", server)
        }
    }

    pub mod tags {
        use super::*;

        lazy_static! {
            static ref RESOURCES: &'static str = "tests/resources/responses/tags";

            pub static ref EMPTY: String = format!("{}/empty.json", *RESOURCES);
            pub static ref WINE_GE: String = format!("{}/wine_ge.json", *RESOURCES);
            pub static ref WINE_GE_LOL: String = format!("{}/wine_ge_lol.json", *RESOURCES);
        }
    }
}