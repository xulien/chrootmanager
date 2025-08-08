//! Stage3 downloader module for Gentoo chroot management
//!
//! This module provides functionality to download stage3 tarballs and verify their integrity
//! using the new profile management system.

use crate::config::Config;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use crate::profile::selected::SelectedProfile;

/// Represents the progress information during download
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub speed_bytes_per_sec: f64,
    /// Filename being downloaded (kept for debugging and future UI enhancements)
    #[allow(dead_code)]
    pub filename: String,
}

/// Represents the result of a download attempt
#[derive(Debug)]
pub struct DownloadResult {
    /// URL that was successfully used for download (kept for logging and retry logic)
    #[allow(dead_code)]
    pub successful_url: String,
    pub file_path: String,
    /// Total bytes downloaded (kept for statistics and reporting)
    #[allow(dead_code)]
    pub total_bytes: u64,
    pub average_speed_bytes_per_sec: f64,
}

/// Generate a stage3 URL based on the mirror's base URL and selected profile
fn build_stage3_url(base_mirror_url: &str, profile: &SelectedProfile) -> String {
    let mut mirror_url = base_mirror_url.to_string();

    // Ensure the URL ends with a slash
    if !mirror_url.ends_with('/') {
        mirror_url.push('/');
    }

    format!(
        "{mirror_url}releases/{}/autobuilds/current-stage3-{}-{}/",
        profile.arch(),
        profile.arch(),
        profile.profile()
    )
}

/// Build stage3 URLs using configured mirrors with fallback
fn get_stage3_url(profile: &SelectedProfile, config: &Config) -> Vec<String> {
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

/// Calculate download speed in bytes per second
fn calculate_speed_bytes_per_sec(bytes: u64, duration: Duration) -> f64 {
    bytes as f64 / duration.as_secs_f64()
}

/// Download stage3 with a progress callback using the new profile system
pub async fn download_stage3_with_progress<F>(
    profile: &SelectedProfile,
    destination_path: &str,
    config: &Config,
    mut progress_callback: F,
) -> Result<DownloadResult, Box<dyn std::error::Error>>
where
    F: FnMut(DownloadProgress),
{
    let filename = get_current_stage3_filename(profile, config).await?;
    let base_urls = get_stage3_url(profile, config);

    // Build download URLs
    let download_urls: Vec<String> = base_urls
        .iter()
        .map(|base_url| format!("{base_url}{filename}"))
        .collect();

    log::debug!("download_urls: {download_urls:?}");

    let full_path = format!("{destination_path}/{filename}");

    let client = reqwest::Client::new();

    // Attempt to download with different mirrors
    let (successful_url, response) = try_download_with_mirrors(&download_urls, &client).await?;

    let total_size = response.content_length().unwrap_or(0);

    // Initial progress callback
    progress_callback(DownloadProgress {
        downloaded: 0,
        total: total_size,
        speed_bytes_per_sec: 0.0,
        filename: filename.clone(),
    });

    let mut file = File::create(&full_path).await?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();

    // Variables for speed calculation and display frequency
    let mut last_update = std::time::Instant::now();
    let mut last_downloaded = 0u64;
    let start_time = std::time::Instant::now();
    let update_interval = Duration::from_millis(250); // Update every 250 ms

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        // Update progress periodically
        let now = std::time::Instant::now();
        if now.duration_since(last_update) >= update_interval {
            let speed = if last_update != start_time {
                let bytes_since_last = downloaded - last_downloaded;
                let time_since_last = now.duration_since(last_update);
                calculate_speed_bytes_per_sec(bytes_since_last, time_since_last)
            } else {
                let total_time = now.duration_since(start_time);
                calculate_speed_bytes_per_sec(downloaded, total_time)
            };

            progress_callback(DownloadProgress {
                downloaded,
                total: total_size,
                speed_bytes_per_sec: speed,
                filename: filename.clone(),
            });

            last_update = now;
            last_downloaded = downloaded;
        }
    }

    // Final callback with average speed
    let total_time = std::time::Instant::now().duration_since(start_time);
    let avg_speed = calculate_speed_bytes_per_sec(downloaded, total_time);

    progress_callback(DownloadProgress {
        downloaded,
        total: total_size,
        speed_bytes_per_sec: avg_speed,
        filename: filename.clone(),
    });

    Ok(DownloadResult {
        successful_url,
        file_path: full_path,
        total_bytes: downloaded,
        average_speed_bytes_per_sec: avg_speed,
    })
}

/// Function to attempt downloading a file with multiple mirrors
async fn try_download_with_mirrors(
    urls: &[String],
    client: &reqwest::Client,
) -> Result<(String, reqwest::Response), Box<dyn std::error::Error>> {
    let mut last_error = None;

    for (index, url) in urls.iter().enumerate() {
        log::debug!("Attempting mirror {} : {}", index + 1, url);
        log::debug!("Downloading {url}");
        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    log::debug!("Success with mirror {}", index + 1);
                    return Ok((url.clone(), response));
                } else {
                    log::debug!(
                        "Mirror {} failed - Status: {}",
                        index + 1,
                        response.status()
                    );
                    last_error = Some(format!("HTTP Status {}", response.status()));
                }
            }
            Err(e) => {
                log::debug!("Error with mirror {} : {}", index + 1, e);
                last_error = Some(format!("Network error : {e}"));
            }
        }
    }

    Err(format!(
        "All mirrors failed. Last error: {}",
        last_error.unwrap_or_else(|| "No specific error".to_string())
    )
    .into())
}

/// Get the current stage3 filename for the specified profile
pub async fn get_current_stage3_filename(
    profile: &SelectedProfile,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error>> {
    let base_urls = get_stage3_url(profile, config);

    // Build URLs for the latest file using the new stage3 pattern
    let latest_urls: Vec<String> = base_urls
        .iter()
        .map(|base_url| format!("{base_url}latest-{}.txt", profile.get_stage3_pattern()))
        .collect();

    let client = reqwest::Client::new();

    // Attempt to download the latest file with different mirrors
    let (_successful_url, response) = try_download_with_mirrors(&latest_urls, &client).await?;

    log::debug!("The latest file downloaded successfully");

    let content = response.text().await?;

    // Parse the content to extract the filename
    // The typical format is: timestamp filename size
    // Example: 20231201T170504Z stage3-amd64-openrc-20231201T170504Z.tar.xz 123456789
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split by spaces/tabs
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            // The filename is usually the second element
            let filename = parts[1];

            // Check that it's actually a stage3 file with the correct pattern
            if filename.contains(&profile.get_stage3_pattern()) && filename.ends_with(".tar.xz") {
                return Ok(filename.to_string());
            }
        }

        // Fallback: if the format is different, look for a .tar.xz file with our pattern
        if line.contains(&profile.get_stage3_pattern()) && line.contains(".tar.xz") {
            // Extract the filename from the line
            if let Some(start) = line.find(&profile.get_stage3_pattern()) {
                let remaining = &line[start..];
                if let Some(end) = remaining.find(".tar.xz") {
                    let filename = &remaining[..end + 7]; // +7 to include ".tar.xz"
                    return Ok(filename.to_string());
                }
            }
        }
    }

    Err(format!("No stage3 file found for profile {profile}").into())
}

/// Download the SHA256 file for a given stage3 archive
pub async fn download_stage3_sha256(
    profile: &SelectedProfile,
    config: &Config,
    filename: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let base_urls = get_stage3_url(profile, config);
    let sha256_filename = format!("{filename}.sha256");

    // Build URLs for the SHA256 file
    let sha256_urls: Vec<String> = base_urls
        .iter()
        .map(|base_url| format!("{base_url}{sha256_filename}"))
        .collect();

    let client = reqwest::Client::new();

    // Attempt to download the SHA256 file with different mirrors
    let (_successful_url, response) = try_download_with_mirrors(&sha256_urls, &client).await?;

    let sha256_content = response.text().await?;

    // Parse the SHA256 content (format: "hash filename")
    for line in sha256_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0];
            let file_in_hash = parts[1];

            // Check that it's the right file
            if file_in_hash.ends_with(&filename) || file_in_hash == filename {
                return Ok(hash.to_string());
            }
        }
    }

    Err("SHA256 hash isn't found in the file".into())
}

/// Calculate the SHA256 hash of a local file
pub async fn calculate_file_sha256(
    file_path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    let mut file = File::open(file_path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192]; // 8KB buffer

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Check the integrity of a stage3 file with its SHA256 hash
/// Returns (is_valid, expected_hash, calculated_hash)
pub async fn check_stage3_integrity(
    file_path: &std::path::Path,
    expected_sha256: &str,
) -> Result<(bool, String, String), Box<dyn std::error::Error>> {
    let calculated_hash = calculate_file_sha256(file_path).await?;
    let is_valid = calculated_hash.to_lowercase() == expected_sha256.to_lowercase();

    Ok((is_valid, expected_sha256.to_string(), calculated_hash))
}
