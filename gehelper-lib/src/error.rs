use std::io;

use reqwest::blocking::Response;
use thiserror::Error;

use crate::tag::TagKind;

#[derive(Error, Debug)]
pub enum SteamConfigError {
    #[error("Config to copy has no CompatToolMapping group")]
    NoDefaultCompatToolAttribute,
    #[error("IO error occurred - Inspect the source for more information")]
    IoError {
        #[from]
        source: io::Error,
    },
}

#[derive(Error, Debug)]
pub enum LutrisConfigError {
    #[error("Config to copy has no version attribute")]
    NoVersionAttribute,
    #[error("IO error occurred - Inspect the source for more information")]
    IoError {
        #[from]
        source: io::Error,
    },
}

#[derive(Error, Debug)]
pub enum DeserializeError {
    #[error("Could not convert Github JSON response into a struct")]
    FailedToConvertToStruct {
        #[from]
        source: Box<dyn std::error::Error>,
    },
}

#[derive(Debug, Error)]
pub enum GithubError {
    #[error("Failed to convert GitHub resource with serde")]
    SerdeDeserializeError {
        #[from]
        source: serde_json::Error,
    },
    #[error("Failed to fetch resource from GitHub API")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error("No tags could be found")]
    NoTags,
    #[error("For {tag} {kind} the release has no assets")]
    ReleaseHasNoAssets { tag: String, kind: TagKind },
    #[error("HTTP response status was not OK (200)")]
    StatusNotOk(Response),
}

#[derive(Debug, Error)]
pub enum TagKindError {
    #[error("Could not create TagKind from provided string.")]
    UnknownString,
}
