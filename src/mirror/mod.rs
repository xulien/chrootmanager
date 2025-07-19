use thiserror::Error;

pub mod mirrors;
pub mod parser;
pub mod stage3;

#[derive(Debug, Error)]
pub enum MirrorError {
    Network(reqwest::Error),
    XmlParsing(xml::reader::Error),
    InvalidFormat(String),
}

impl std::fmt::Display for MirrorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MirrorError::Network(e) => write!(f, "Network error : {e}"),
            MirrorError::XmlParsing(e) => write!(f, "XML parsing error : {e}"),
            MirrorError::InvalidFormat(msg) => write!(f, "Invalid format : {msg}"),
        }
    }
}
