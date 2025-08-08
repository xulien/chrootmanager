use crate::error::{ConfigError, DownloaderError, MirrorError, ProfileError, ChrootError};
use inquire::InquireError;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChrootManagerError {
    #[error("Mirror Error: {0}")]
    Mirror(#[from] MirrorError),
    
    #[error("Download Error: {0}")]
    Download(#[from] DownloaderError),
    
    #[error("Inquire Error: {0}")]
    Inquire(#[from] InquireError),
    
    #[error("Config Error: {0:?}")]
    Config(#[from] ConfigError),
    
    #[error("Profile Error: {0}")]
    Profile(#[from] ProfileError),
    
    #[error("Chroot Error: {0}")]
    Chroot(#[from] ChrootError),
    
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Generic Error: {0}")]
    Generic(Box<dyn std::error::Error>),
    
    #[error("{0}")]
    Custom(String),
}

impl From<Box<dyn std::error::Error>> for ChrootManagerError {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        ChrootManagerError::Generic(error)
    }
}
