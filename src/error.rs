use inquire::InquireError;
use std::io;
use std::io::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProfileError {
    #[error("No architectures available")]
    NoArchitecturesAvailable,
    #[error("Architecture isn't found: {0}")]
    ArchitectureNotFound(String),
    #[error("No profiles available for architecture: {0}")]
    NoProfilesAvailableForArchitecture(String),
}

#[derive(Error, Debug)]
pub enum DownloaderError {
    // These variants are kept for comprehensive error handling in future scenarios
    #[allow(dead_code)]
    #[error("IO Error: {0}")]
    Io(Error),
    #[allow(dead_code)]
    #[error("Error parsing empty profile")]
    ReadProfileEmpty,
    #[allow(dead_code)]
    #[error("Error reading profile: {0}")]
    CantReadProfile(String),
    #[error("Error retrieving mirror: {0}")]
    RetrievingMirror(String),
    #[error("Reqwest Error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Inquire Error: {0}")]
    Inquire(#[from] InquireError),
    #[error("Toml Deserialisation Error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("Toml Serialization Error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("Downloader Error: {0}")]
    Downloader(#[from] DownloaderError),
}

#[derive(Error, Debug)]
pub enum ElevationError {
    // These variants are kept for comprehensive error handling in privilege elevation scenarios
    #[error("Access denied by a user")]
    AccessDenied,
    #[error("Permission Denied")]
    PermissionDenied,
    #[error("IO Error: {0}")]
    IoError(#[from] io::Error),
    #[error("Authentication required. Please call pre_authenticate_operations() first")]
    AuthenticationRequired,
    #[error("Failed to acquire elevation lock")]
    FailedToAcquireElevationLock,
    #[error("Sudo not available")]
    SudoNotAvailable,
}

#[derive(Error, Debug)]
pub enum ChrootError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Config Error: {0}")]
    Config(#[from] ConfigError),
    #[error("Downloader Error: {0}")]
    Downloader(#[from] DownloaderError),
    #[error("Command Error: {0}")]
    Command(String),
    #[error("Elevation Error: {0}")]
    Elevation(#[from] ElevationError),
    #[error("Elevation Error: {0}")] // TODO: workaround
    ElevationError(String),
    #[error("No profile")]
    NoProfile,
}

#[derive(Error, Debug)]
pub enum MirrorError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Reqwest Error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("XML Error: {0}")]
    Xml(#[from] xml::reader::Error),
    #[error("Config Error: {0}")]
    Config(#[from] ConfigError),
    #[error("Downloader Error: {0}")]
    Downloader(#[from] DownloaderError),
    #[error("EmptyDataReceived")]
    EmptyDataReceived,
    #[error("InvalidFormat: {0}")]
    InvalidFormat(String),
    #[error("XML document does not contain a root element 'mirrors'")]
    NoRootElementIntoMirrors,
}
