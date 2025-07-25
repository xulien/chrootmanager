use crate::{config::Config, profile::arch::Arch};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};
use tokio::{fs::File, io::AsyncWriteExt};
use tokio_stream::StreamExt;

/// Utility function to format the size in bytes in a readable way
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Calculate download speed
fn calculate_speed(bytes: u64, duration: Duration) -> String {
    let bytes_per_sec = bytes as f64 / duration.as_secs_f64();
    format!("{}/s", format_bytes(bytes_per_sec as u64))
}

/// Display a progress bar in the terminal
fn display_progress(downloaded: u64, total: u64, speed: &str) {
    const BAR_WIDTH: usize = 40;

    if total == 0 {
        // If we don't know the total size, display only the downloaded bytes
        print!("\r📥 Downloaded: {} @ {}", format_bytes(downloaded), speed);
        io::stdout().flush().unwrap();
        return;
    }

    let progress = downloaded as f64 / total as f64;
    let filled_width = (progress * BAR_WIDTH as f64) as usize;
    let empty_width = BAR_WIDTH - filled_width;

    let filled_bar = "█".repeat(filled_width);
    let empty_bar = "░".repeat(empty_width);

    let percentage = (progress * 100.0) as u8;

    print!(
        "\r📥 [{}{}] {}% ({} / {}) @ {}",
        filled_bar,
        empty_bar,
        percentage,
        format_bytes(downloaded),
        format_bytes(total),
        speed
    );

    io::stdout().flush().unwrap();
}

/// Generate a stage3 URL based on the mirror's base URL
fn build_stage3_url(base_mirror_url: &str, profile: &Arch) -> String {
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

/// Updated function to use configured mirrors with fallback
fn get_stage3_url(profile: &Arch, config: &Config) -> Vec<String> {
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

/// Function to attempt downloading a file with multiple mirrors
async fn try_download_with_mirrors(
    urls: &[String],
    client: &reqwest::Client,
    show_progress: bool,
) -> Result<(String, reqwest::Response), Box<dyn std::error::Error>> {
    let mut last_error = None;

    for (index, url) in urls.iter().enumerate() {
        if show_progress {
            println!("🔗 Attempting with mirror {} : {}", index + 1, url);
        }
        log::debug!("Downloading {url}");
        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if show_progress {
                        println!("✅ Success with mirror {}", index + 1);
                    }
                    return Ok((url.clone(), response));
                } else {
                    if show_progress {
                        println!(
                            "❌ Mirror {} failed - Status: {}",
                            index + 1,
                            response.status()
                        );
                    }
                    last_error = Some(format!("HTTP Status {}", response.status()));
                }
            }
            Err(e) => {
                if show_progress {
                    println!("❌ Error with mirror {} : {}", index + 1, e);
                }
                last_error = Some(format!("Network error : {e}"));
            }
        }
    }

    Err(format!(
        "All mirrors failed. Last error : {}",
        last_error.unwrap_or_else(|| "No specific error".to_string())
    )
    .into())
}

pub async fn get_current_stage3_filename(
    arch: &Arch,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error>> {
    let base_urls = get_stage3_url(arch, config);

    // Build URLs for the latest file
    let latest_urls: Vec<String> = base_urls
        .iter()
        .map(|base_url| format!("{base_url}latest-stage3-{arch}.txt"))
        .collect();

    let client = reqwest::Client::new();

    // Attempt to download the latest file with different mirrors (without progress display)
    let (_successful_url, response) =
        try_download_with_mirrors(&latest_urls, &client, false).await?;

    // Don't display success message for the txt file
    log::debug!("Latest file downloaded successfully");

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

            // Check that it's actually a stage3 file
            if filename.contains("stage3") && filename.ends_with(".tar.xz") {
                return Ok(filename.to_string());
            }
        }

        // Fallback: if the format is different, look for a .tar.xz file
        if line.contains("stage3") && line.contains(".tar.xz") {
            // Extract the filename from the line
            if let Some(start) = line.find("stage3") {
                let remaining = &line[start..];
                if let Some(end) = remaining.find(".tar.xz") {
                    let filename = &remaining[..end + 7]; // +7 to include ".tar.xz"
                    return Ok(filename.to_string());
                }
            }
        }
    }

    Err("No stage3 file found".into())
}

// Download function with cache support and SHA256 verification
pub async fn download_stage3_with_cache(
    profile: &Arch,
    config: &Config,
    force_download: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("🔍 Retrieving information on stage 3...");
    let filename = get_current_stage3_filename(profile, config).await?;
    println!("📋 Current stage3 file : {filename}");

    // Check if the file already exists in the cache
    if !force_download {
        let cached_path = config.get_cache_path(&filename);

        if cached_path.exists() {
            println!("💾 Stage3 found in cache, integrity check...");

            // Download SHA256 hash for verification
            match download_stage3_sha256(profile, config, &filename).await {
                Ok(expected_hash) => {
                    match verify_stage3_integrity(&cached_path, &expected_hash).await {
                        Ok(true) => {
                            println!(
                                "✅ Cached stage3 successfully verified : {}",
                                cached_path.display()
                            );
                            return Ok(cached_path.to_string_lossy().to_string());
                        }
                        Ok(false) => {
                            println!("❌ Cached stage3 corrupted, deleting and re-downloading...");
                            if let Err(e) = tokio::fs::remove_file(&cached_path).await {
                                log::warn!("Error deleting corrupted file : {e}");
                            }
                        }
                        Err(e) => {
                            log::warn!("Error during SHA256 verification : {e}, re-downloading...")
                        }
                    }
                }
                Err(e) => log::warn!("Unable to download SHA256 hash : {e}, re-downloading..."),
            }
        }
    }

    // Download to cache
    let cache_path = config.get_cache_path(&filename);
    let cache_dir = cache_path.parent().unwrap().to_str().unwrap();

    println!("📦 Downloading stage3 to cache...");
    let downloaded_path = download_stage3_with_progress(profile, cache_dir, config).await?;

    // Verify the downloaded file
    println!("🔍 Verifying downloaded file integrity...");
    match download_stage3_sha256(profile, config, &filename).await {
        Ok(expected_hash) => {
            let file_path = std::path::Path::new(&downloaded_path);
            match verify_stage3_integrity(file_path, &expected_hash).await {
                Ok(true) => {
                    println!("✅ Stage3 downloaded and verified successfully");
                }
                Ok(false) => {
                    // Delete the corrupted file
                    if let Err(e) = tokio::fs::remove_file(file_path).await {
                        log::warn!("Error deleting corrupted file : {e}");
                    }
                    return Err("Downloaded file is corrupted (SHA256 verification failed).".into());
                }
                Err(e) => {
                    log::warn!("Error during SHA256 verification : {e}");
                    return Err(format!("Error during SHA256 verification : {e}").into());
                }
            }
        }
        Err(e) => {
            log::warn!("Unable to download SHA256 hash for verification : {e}");
            println!("⚠️ File downloaded without SHA256 verification (hash not available)");
        }
    }

    Ok(downloaded_path)
}

pub async fn download_stage3_with_progress(
    profile: &Arch,
    destination_path: &str,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error>> {
    let filename = get_current_stage3_filename(profile, config).await?;
    let base_urls = get_stage3_url(profile, config);

    // Build download URLs
    let download_urls: Vec<String> = base_urls
        .iter()
        .map(|base_url| format!("{base_url}{filename}"))
        .collect();

    log::debug!("download_urls: {download_urls:?}");

    let full_path = format!("{destination_path}/{filename}");

    println!("📥 Downloading : {filename}");

    let client = reqwest::Client::new();

    // Attempt to download with different mirrors (with progress display)
    let (successful_url, response) =
        try_download_with_mirrors(&download_urls, &client, true).await?;

    println!("📡 Downloading from : {successful_url}");

    let total_size = response.content_length().unwrap_or(0);

    if total_size > 0 {
        println!("📊 File size : {}", format_bytes(total_size));
    } else {
        println!("📊 File size : unknown");
    }

    let mut file = File::create(&full_path).await?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();

    // Variables for speed calculation and display frequency
    let mut last_update = Instant::now();
    let mut last_downloaded = 0u64;
    let start_time = Instant::now();
    let update_interval = Duration::from_millis(250); // Update every 250ms

    // Initial progress display
    display_progress(downloaded, total_size, "0 B/s");

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        // Update progress periodically
        let now = Instant::now();
        if now.duration_since(last_update) >= update_interval {
            let speed = if last_update != start_time {
                let bytes_since_last = downloaded - last_downloaded;
                let time_since_last = now.duration_since(last_update);
                calculate_speed(bytes_since_last, time_since_last)
            } else {
                let total_time = now.duration_since(start_time);
                calculate_speed(downloaded, total_time)
            };

            display_progress(downloaded, total_size, &speed);

            last_update = now;
            last_downloaded = downloaded;
        }
    }

    // Final display with average speed
    let total_time = Instant::now().duration_since(start_time);
    let avg_speed = calculate_speed(downloaded, total_time);
    display_progress(downloaded, total_size, &avg_speed);
    println!(); // New line after progress bar

    println!("✅ Stage3 downloaded successfully : {full_path}");
    println!("📈 Average speed : {avg_speed}");

    Ok(full_path)
}

/// Download the SHA256 file for a given stage3
pub async fn download_stage3_sha256(
    profile: &Arch,
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

    // Attempt to download the SHA256 file with different mirrors (without display)
    let (_successful_url, response) =
        try_download_with_mirrors(&sha256_urls, &client, false).await?;

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

    Err("SHA256 hash not found in file".into())
}

/// Calculate the SHA256 of a local file
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

/// Verify the integrity of a stage3 file with its SHA256 hash
pub async fn verify_stage3_integrity(
    file_path: &std::path::Path,
    expected_sha256: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("🔍 SHA256 verification in progress...");

    let calculated_hash = calculate_file_sha256(file_path).await?;
    let is_valid = calculated_hash.to_lowercase() == expected_sha256.to_lowercase();

    if is_valid {
        println!("✅ SHA256 verification successful");
    } else {
        println!("❌ SHA256 verification failed");
        println!("   Expected : {expected_sha256}");
        println!("   Calculated : {calculated_hash}");
    }

    Ok(is_valid)
}
