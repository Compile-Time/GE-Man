use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    NoVersionProvided,
    VersionDoesNotExist,

    CouldNotSerializeManagedVersions(serde_json::Error),
    CouldNotDeserializeManagedVersions(serde_json::Error),
    CouldNotSaveManagedVersions(std::io::Error),
    CouldNotReadManagedVersions(std::io::Error),

    StdLibError(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let err = match self {
            Error::VersionDoesNotExist => "Version does not exist.",
            Error::NoVersionProvided => "No version was provided.",
            _ => "",
        };

        let err = match self {
            Error::CouldNotDeserializeManagedVersions(err) => {
                format!("Could not deserialize JSON into ManagedVersions struct: {}", err)
            }
            Error::CouldNotReadManagedVersions(err) => {
                format!("Could not read managed_versions.json: {}", err)
            }
            Error::CouldNotSerializeManagedVersions(err) => {
                format!("Could not convert ManagedVersions struct to JSON: {}", err)
            }
            Error::CouldNotSaveManagedVersions(err) => {
                format!("Could not write JSON to managed_versions.json: {}", err)
            }
            _ => String::from(err),
        };

        writeln!(f, "{}", err)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::StdLibError(error)
    }
}
