use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChrootManagerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("User interaction error: {0}")]
    Inquire(#[from] inquire::InquireError),

    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("XmlEvent error: {0}")]
    XmlEvent(#[from] xml::reader::Error),

    #[error("Chroot operation error: {0}")]
    ChrootOperation(String),

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

    #[error("Empty data received")]
    EmptyDataReceived,

    #[error("Element 'mirrorgroup' found without root element 'mirrors'")]
    MirrorGroupNoRootElementMirrors,

    #[error("'Mirror' element found outside a mirrorgroup")]
    ElementOutsideMirrorGroup,

    #[error("'uri' element found outside a mirror")]
    UriOutsideMirror,

    #[error("XML document does not contain a root element 'mirrors'")]
    DocumentNoRootElementMirrors,
}
