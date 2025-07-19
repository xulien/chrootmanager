use std::io;
use thiserror::Error;

/// Erreurs principales de ChrootManager
#[derive(Error, Debug)]
pub enum ChrootManagerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("User interaction error: {0}")]
    Inquire(#[from] inquire::InquireError),

    #[error("Configuration validation error: {0}")]
    Validation(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("File verification failed: {0}")]
    Verification(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("No valid filename found")]
    NoFilename,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("HTTP error: {0}")]
    HttpError(u16),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("No mirrors available")]
    NoMirrorsAvailable,

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Mirror fetch failed: {0}")]
    FetchFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Profile error: {0}")]
    Profile(String),

    #[error("Chroot operation error: {0}")]
    ChrootOperation(String),

    #[error("Permission error: {0}")]
    Permission(String),

    #[error("System error: {0}")]
    System(String),

    #[error("Stage3 extraction failed: {0}")]
    Stage3ExtractionFailed(String),

    #[error("No stage3 file found")]
    NoStage3Found,

    #[error("All mirror failed: {0}")]
    AllMirrorFail(String),

    #[error("Downloaded file is corrupted (SHA256 verification failed).")]
    DownloadedFileCorrupted,

    #[error("Error deleting corrupted file : {0}")]
    DeletingCorrupted(io::Error),

    #[error("SHA256 hash not found in file")]
    SHA256HashNotFoundInFile,
}

impl ChrootManagerError {
    pub fn profile<S: Into<String>>(msg: S) -> Self {
        ChrootManagerError::Profile(msg.into())
    }

    pub fn chroot_operation<S: Into<String>>(msg: S) -> Self {
        ChrootManagerError::ChrootOperation(msg.into())
    }

    pub fn permission<S: Into<String>>(msg: S) -> Self {
        ChrootManagerError::Permission(msg.into())
    }

    pub fn system<S: Into<String>>(msg: S) -> Self {
        ChrootManagerError::System(msg.into())
    }
}

/// Type alias pour les résultats courants
pub type Result<T> = std::result::Result<T, ChrootManagerError>;
