use crate::error::ChrootManagerError;
use crate::{config::Config, profile::arch::Arch};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
};

/// Generate a stage3 URL based on the mirror's base URL
pub fn build_stage3_url(base_mirror_url: &str, profile: &Arch) -> String {
    let mut mirror_url = base_mirror_url.to_string();

    // Ensure the URL ends with a slash
    if !mirror_url.ends_with('/') {
        mirror_url.push('/');
    }

    format!(
        "{mirror_url}releases/{}/autobuilds/current-stage3-{profile}/",
        profile.arch()
    )
}

pub fn get_stage3_url(profile: &Arch, config: &Config) -> Vec<String> {
    let mut urls = Vec::new();

    // Use configured mirrors
    if config.has_mirrors() {
        for mirror_url in &config.mirrors_url {
            let url = build_stage3_url(mirror_url, profile);
            urls.push(url);
        }
    } else {
        // Default URL if no mirror is configured
        log::warn!("No mirrors configured, using default mirror");
        let default_url = build_stage3_url("https://distfiles.gentoo.org/", profile);
        urls.push(default_url);
    }

    urls
}

pub fn calculate_file_sha256(file_path: &std::path::Path) -> Result<String, ChrootManagerError> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192]; // 8KB buffer
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
