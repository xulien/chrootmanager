use crate::mirror::MirrorError;
use inquire::InquireError;
use std::error;

#[derive(Debug)]
pub enum ChrootManagerError {
    Io(std::io::Error),
    TomlParsing(toml::de::Error),
    TomlSerialization(toml::ser::Error),
    Mirror(MirrorError),
    Config(String),
    Command(String),
    Download(String),
    Ui(InquireError),
}

impl std::fmt::Display for ChrootManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChrootManagerError::Io(e) => write!(f, "IO error: {e}"),
            ChrootManagerError::TomlParsing(e) => write!(f, "Toml parsing error: {e}"),
            ChrootManagerError::TomlSerialization(e) => write!(f, "Toml serialization error: {e}"),
            ChrootManagerError::Mirror(e) => write!(f, "Mirror error: {e}"),
            ChrootManagerError::Command(e) => write!(f, "Command error: {e}"),
            ChrootManagerError::Download(e) => write!(f, "Download error: {e}"),
            ChrootManagerError::Config(e) => write!(f, "Parsing error: {e}"),
            ChrootManagerError::Ui(e) => write!(f, "Ui error: {e}"),
        }
    }
}

impl error::Error for ChrootManagerError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ChrootManagerError::Io(e) => Some(e),
            ChrootManagerError::TomlParsing(e) => Some(e),
            ChrootManagerError::TomlSerialization(e) => Some(e),
            ChrootManagerError::Mirror(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ChrootManagerError {
    fn from(error: std::io::Error) -> Self {
        ChrootManagerError::Io(error)
    }
}

impl From<toml::de::Error> for ChrootManagerError {
    fn from(error: toml::de::Error) -> Self {
        ChrootManagerError::TomlParsing(error)
    }
}

impl From<toml::ser::Error> for ChrootManagerError {
    fn from(error: toml::ser::Error) -> Self {
        ChrootManagerError::TomlSerialization(error)
    }
}

impl From<MirrorError> for ChrootManagerError {
    fn from(error: MirrorError) -> Self {
        ChrootManagerError::Mirror(error)
    }
}

impl From<InquireError> for ChrootManagerError {
    fn from(error: InquireError) -> Self {
        ChrootManagerError::Ui(error)
    }
}
