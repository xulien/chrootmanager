use crate::config::Config;
use crate::downloader::{
    DownloadProgress, check_stage3_integrity, download_stage3_sha256,
    download_stage3_with_progress, get_current_stage3_filename,
};
use std::io;
use std::io::Write;
use std::path::Path;
use crate::profile::selected::SelectedProfile;

/// Utility function to format the size in bytes readably
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Verify the integrity of a stage3 file with its SHA256 hash
async fn verify_stage3_integrity_with_display(
    file_path: &Path,
    expected_sha256: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("🔍 SHA256 verification in progress...");

    let (is_valid, expected, calculated) =
        check_stage3_integrity(file_path, expected_sha256).await?;

    if is_valid {
        println!("✅ SHA256 verification successful");
    } else {
        println!("❌ SHA256 verification failed");
        println!("   Expected: {expected}");
        println!("   Calculated: {calculated}");
    }

    Ok(is_valid)
}

/// Display a progress bar in the terminal
fn display_progress(progress: &DownloadProgress) {
    const BAR_WIDTH: usize = 40;

    if progress.total == 0 {
        // If we don't know the total size, display only the downloaded bytes
        let speed_formatted = format_bytes(progress.speed_bytes_per_sec as u64);
        print!(
            "\r📥 Downloaded: {} @ {}/s       ",
            format_bytes(progress.downloaded),
            speed_formatted
        );
        io::stdout().flush().unwrap();
        return;
    }

    let progress_ratio = progress.downloaded as f64 / progress.total as f64;
    let filled_width = (progress_ratio * BAR_WIDTH as f64) as usize;
    let empty_width = BAR_WIDTH - filled_width;

    let filled_bar = "█".repeat(filled_width);
    let empty_bar = "░".repeat(empty_width);

    let percentage = (progress_ratio * 100.0) as u8;
    let speed_formatted = format_bytes(progress.speed_bytes_per_sec as u64);

    print!(
        "\r📥 [{}{}] {}% ({} / {}) @ {}/s     ",
        filled_bar,
        empty_bar,
        percentage,
        format_bytes(progress.downloaded),
        format_bytes(progress.total),
        speed_formatted
    );

    io::stdout().flush().unwrap();
}

/// Download stage3 with a visual progress display
async fn download_stage3_with_visual_progress(
    profile: &SelectedProfile,
    destination_path: &str,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("🔍 Retrieving information on stage 3...");
    let filename = get_current_stage3_filename(profile, config).await?;
    println!("📋 Current stage3 file : {filename}");

    println!("📥 Downloading : {filename}");

    let result = download_stage3_with_progress(profile, destination_path, config, |progress| {
        if progress.downloaded == 0 {
            if progress.total > 0 {
                println!("📡 Downloading from mirror");
                println!("📊 File size: {}", format_bytes(progress.total));
            } else {
                println!("📊 File size: unknown");
            }
        }
        display_progress(&progress);
    })
    .await?;

    println!(); // New line after the progress bar
    println!("✅ Stage3 downloaded successfully: {}", result.file_path);
    println!(
        "📈 Average speed : {}/s     ",
        format_bytes(result.average_speed_bytes_per_sec as u64)
    );

    Ok(result.file_path)
}

// Download function with cache support and SHA256 verification
pub(crate) async fn download_stage3_with_cache(
    profile: &SelectedProfile,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("🔍 Retrieving information on stage 3...");
    let filename = get_current_stage3_filename(profile, config).await?;
    println!("📋 Current stage3 file: {filename}");

    // Check if the file already exists in the cache
    let cached_path = config.get_cache_path(&filename);

    if cached_path.exists() {
        println!("💾 Stage3 found in cache, integrity check...");

        // Download SHA256 hash for verification
        match download_stage3_sha256(profile, config, &filename).await {
            Ok(expected_hash) => {
                match verify_stage3_integrity_with_display(&cached_path, &expected_hash).await {
                    Ok(true) => {
                        let cached_path_display = cached_path.display();
                        println!("✅ Cached stage3 successfully verified: {cached_path_display}");
                        return Ok(cached_path.to_string_lossy().to_string());
                    }
                    Ok(false) => {
                        println!("❌ Cached stage3 corrupted, deleting and re-downloading...");
                        if let Err(e) = tokio::fs::remove_file(&cached_path).await {
                            log::warn!("Error deleting corrupted file: {e}");
                        }
                    }
                    Err(e) => {
                        log::warn!("Error during SHA256 verification: {e}, re-downloading...")
                    }
                }
            }
            Err(e) => log::warn!("Unable to download SHA256 hash: {e}, re-downloading..."),
        }
    }

    // Download to cache
    let cache_path = config.get_cache_path(&filename);
    let cache_dir = cache_path.parent().unwrap().to_str().unwrap();

    println!("📦 Downloading stage3 to cache...");
    let downloaded_path = download_stage3_with_visual_progress(profile, cache_dir, config).await?;

    // Verify the downloaded file
    println!("🔍 Verifying downloaded file integrity...");
    match download_stage3_sha256(profile, config, &filename).await {
        Ok(expected_hash) => {
            let file_path = Path::new(&downloaded_path);
            match verify_stage3_integrity_with_display(file_path, &expected_hash).await {
                Ok(true) => {
                    println!("✅ Stage3 downloaded and verified successfully");
                }
                Ok(false) => {
                    // Delete the corrupted file
                    if let Err(e) = tokio::fs::remove_file(file_path).await {
                        log::warn!("Error deleting corrupted file: {e}");
                    }
                    return Err(
                        "The downloaded file is corrupted (SHA256 verification failed).".into(),
                    );
                }
                Err(e) => {
                    log::warn!("Error during SHA256 verification: {e}");
                    return Err(format!("Error during SHA256 verification: {e}").into());
                }
            }
        }
        Err(e) => {
            log::warn!("Unable to download SHA256 hash for verification: {e}");
            println!("⚠️ File downloaded without SHA256 verification (hash not available)");
        }
    }

    Ok(downloaded_path)
}
