use crate::error::ChrootManagerError::{
    AllMirrorFail, DeletingCorrupted, DownloadedFileCorrupted, NoStage3Found,
    SHA256HashNotFoundInFile,
};
use crate::profile::stage3::{calculate_file_sha256, get_stage3_url};
use crate::{
    chroot::ChrootUnit,
    config::Config,
    error::ChrootManagerError,
    profile::{amd64::Amd64Profile, arch::Arch, arm64::Arm64Profile},
};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::{InquireError, Select};
use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};
use std::{
    io::{self, Write},
    path::PathBuf,
};
use strum::IntoEnumIterator;

#[derive(Parser)]
#[command(
    name = "chrootmanager",
    about = "Gentoo chroot manager with CLI interface",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new chroot
    Create {
        /// Chroot name
        name: String,
        /// Force download even if the file exists in the cache
        #[arg(long)]
        force_download: bool,
    },
    /// List all chroots
    List,
    Mirror,
    Tui,
}

pub fn amd64_profile_selection() -> Result<Arch, ChrootManagerError> {
    let profile_list = Amd64Profile::iter().collect::<Vec<_>>();

    let profile_selection: Result<Amd64Profile, InquireError> =
        Select::new("What's your Amd64 profile ?", profile_list).prompt();
    let selected_profile = profile_selection?;

    Ok(Arch::Amd64(selected_profile))
}

pub fn arm64_profile_selection() -> Result<Arch, ChrootManagerError> {
    let profile_list = Arm64Profile::iter().collect::<Vec<_>>();

    let profile_selection: Result<Arm64Profile, InquireError> =
        Select::new("What's your Arm64 profile ?", profile_list).prompt();
    let selected_profile = profile_selection?;

    Ok(Arch::Arm64(selected_profile))
}

pub fn create_chroot(
    name: String,
    force_download: bool,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "📦 Création du chroot...".green().bold());
    println!(
        "   📂 Répertoire de base : {}",
        config.chroot_base_dir.display()
    );

    config.ensure_chroot_base_dir()?;

    let arch_list = Arch::labels();
    let arch_selection: Result<String, InquireError> =
        Select::new("What's your Arch ?", arch_list).prompt();
    let arch_selection = arch_selection?;

    let selected_profile = match arch_selection.as_str() {
        "amd64" => amd64_profile_selection()?,
        "arm64" => arm64_profile_selection()?,
        _ => panic!("Invalid architecture"),
    };

    let chroot_unit = ChrootUnit::new(name.clone(), Some(&selected_profile))?;

    log::debug!("chroot path: {:?}", chroot_unit.chroot_path);

    // Check if chroot already exists
    if chroot_unit.chroot_path.exists() {
        println!(
            "{}",
            format!("⚠️ The chroot '{}' already exists.", chroot_unit.name)
                .yellow()
                .bold()
        );
        print!("Do you want to delete and recreate it? ? (o/N) : ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase().starts_with('o') {
            println!("{}", "🗑️ Removing the old chroot...".red().bold());
            chroot_unit.cleanup(true)?;
            println!("✅ Old chroot deleted");
        } else {
            return Err(format!(
                "The chroot '{}' already exists. Use another name or delete it first.",
                chroot_unit.name
            )
            .into());
        }
    }

    let cached_path =
        download_stage3_with_cache(&selected_profile.clone(), config, force_download)?;
    let cached_path = PathBuf::from(cached_path);

    chroot_unit.prepare_chroot_directory()?;
    chroot_unit.extract_stage3(&cached_path)?;
    chroot_unit.copy_dns_info()?;

    println!(
        "{}",
        format!("✅ Chroot '{name}' created successfully!")
            .green()
            .bold()
    );
    println!("📍 Path : {}", chroot_unit.chroot_path.display());

    list_chroots(config)?;

    Ok(())
}

pub fn list_chroots(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "   📂 Chroot Directory : {}",
        config.chroot_base_dir.display()
    );

    if !config.chroot_base_dir.exists() {
        println!("   ❌ Chroot directory not found");
        println!("   The directory will be created when the chroot is first created");
        println!("   Make sure you have permissions to create chroots");
        return Ok(());
    }

    let units = ChrootUnit::find_units(config)?;

    if units.is_empty() {
        println!(
            "   📭 No chroot found in : {}",
            config.chroot_base_dir.display()
        );
        println!("   💡 Use 'create' to create your first chroot");
    }

    let units_choices = units.iter().map(|u| u.name.as_str()).collect::<Vec<_>>();

    let units_selected: Result<&str, InquireError> =
        Select::new("📋 List of chroots", units_choices).prompt();
    let units_selected = units_selected?;
    let unit: Vec<&ChrootUnit> = units.iter().filter(|u| u.name.eq(units_selected)).collect();
    let unit = unit[0];

    unit.mount_filesystems()?.enter_chroot_interactive()?;

    Ok(())
}

pub fn setup_mirrors(config: &mut Config) -> Result<(), ChrootManagerError> {
    let options = vec![
        "Select mirror from the official list (recommended)",
        "Use Gentoo's default mirror",
    ];

    let mirror_configuration_select: Result<&str, InquireError> =
        Select::new("🔧 Mirror Configuration", options).prompt();

    match mirror_configuration_select {
        Ok(choice) => match choice {
            "Select mirror from the official list (recommended)" => config.configure_mirrors()?,
            "Use Gentoo's default mirror" => {
                config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
                println!("✅ Using Gentoo's Default Mirror");
            }
            _ => {
                println!("❌ Error during choice");
                println!("Using the default mirror...");
                config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
            }
        },
        Err(e) => {
            println!("❌ Error during configuration : {e}");
            println!("Using the default mirror...");
            config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
        }
    }

    Ok(())
}

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

/// Calculate download speed
fn calculate_speed(bytes: u64, duration: Duration) -> String {
    let bytes_per_sec = bytes as f64 / duration.as_secs_f64();
    format!("{}/s", format_bytes(bytes_per_sec as u64))
}

fn verify_stage3_integrity(
    file_path: &std::path::Path,
    expected_sha256: &str,
) -> Result<bool, ChrootManagerError> {
    println!("🔍 SHA256 verification in progress...");

    let calculated_hash = calculate_file_sha256(file_path)?;
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

fn download_stage3_with_cache(
    profile: &Arch,
    config: &Config,
    force_download: bool,
) -> Result<String, ChrootManagerError> {
    println!("🔍 Retrieving information on stage 3...");
    let filename = get_current_stage3_filename(profile, config)?;
    println!("📋 Current stage3 file : {filename}");

    // Check if the file already exists in the cache
    if !force_download {
        let cached_path = config.get_cache_path(&filename);

        if cached_path.exists() {
            println!("💾 Stage3 found in cache, integrity check...");

            // Download SHA256 hash for verification
            match download_stage3_sha256(profile, config, &filename) {
                Ok(expected_hash) => match verify_stage3_integrity(&cached_path, &expected_hash) {
                    Ok(true) => {
                        println!(
                            "✅ Cached stage3 successfully verified : {}",
                            cached_path.display()
                        );
                        return Ok(cached_path.to_string_lossy().to_string());
                    }
                    Ok(false) => {
                        println!("❌ Cached stage3 corrupted, deleting and re-downloading...");
                        if let Err(e) = std::fs::remove_file(&cached_path) {
                            log::warn!("Error deleting corrupted file : {e}");
                        }
                    }
                    Err(e) => {
                        log::warn!("Error during SHA256 verification : {e}, re-downloading...")
                    }
                },
                Err(e) => log::warn!("Unable to download SHA256 hash : {e}, re-downloading..."),
            }
        }
    }

    // Download to cache
    let cache_path = config.get_cache_path(&filename);
    let cache_dir = cache_path.parent().unwrap().to_str().unwrap();

    println!("📦 Downloading stage3 to cache...");
    let downloaded_path = download_stage3_with_progress(profile, cache_dir, config)?;

    // Verify the downloaded file
    println!("🔍 Verifying downloaded file integrity...");
    if let Ok(expected_hash) = download_stage3_sha256(profile, config, &filename) {
        let file_path = std::path::Path::new(&downloaded_path);
        match verify_stage3_integrity(file_path, &expected_hash) {
            Ok(true) => {
                println!("✅ Stage3 downloaded and verified successfully");
            }
            Ok(false) => {
                if let Err(e) = std::fs::remove_file(file_path) {
                    return Err(DeletingCorrupted(e));
                }
                return Err(DownloadedFileCorrupted);
            }
            Err(e) => panic!("{e:?}"),
        }
    }

    Ok(downloaded_path)
}

fn try_download_with_mirrors(
    urls: &[String],
    client: &reqwest::blocking::Client,
    show_progress: bool,
) -> Result<(String, reqwest::blocking::Response), ChrootManagerError> {
    let mut last_error = None;

    for (index, url) in urls.iter().enumerate() {
        if show_progress {
            println!("🔗 Attempting with mirror {} : {}", index + 1, url);
        }
        log::debug!("Downloading {url}");
        match client.get(url).send() {
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

    Err(AllMirrorFail(
        last_error.unwrap_or_else(|| "No specific error".to_string()),
    ))
}

fn get_current_stage3_filename(arch: &Arch, config: &Config) -> Result<String, ChrootManagerError> {
    let base_urls = get_stage3_url(arch, config);

    // Build URLs for the latest file
    let latest_urls: Vec<String> = base_urls
        .iter()
        .map(|base_url| format!("{base_url}latest-stage3-{arch}.txt"))
        .collect();

    let client = reqwest::blocking::Client::new();

    // Attempt to download the latest file with different mirrors (without progress display)
    let (_successful_url, response) = try_download_with_mirrors(&latest_urls, &client, false)?;

    // Don't display success message for the txt file
    log::debug!("Latest file downloaded successfully");

    let content = response.text()?;

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

    Err(NoStage3Found)
}

pub fn download_stage3_with_progress(
    profile: &Arch,
    destination_path: &str,
    config: &Config,
) -> Result<String, ChrootManagerError> {
    let filename = get_current_stage3_filename(profile, config)?;
    let base_urls = get_stage3_url(profile, config);

    // Build download URLs
    let download_urls: Vec<String> = base_urls
        .iter()
        .map(|base_url| format!("{base_url}{filename}"))
        .collect();

    log::debug!("download_urls: {download_urls:?}");

    let full_path = format!("{destination_path}/{filename}");

    println!("📥 Downloading : {filename}");

    let client = reqwest::blocking::Client::new();

    // Attempt to download with different mirrors (with progress display)
    let (successful_url, mut response) = try_download_with_mirrors(&download_urls, &client, true)?;

    println!("📡 Downloading from : {successful_url}");

    let total_size = response.content_length().unwrap_or(0);

    if total_size > 0 {
        println!("📊 File size : {}", format_bytes(total_size));
    } else {
        println!("📊 File size : unknown");
    }

    let mut file = File::create(&full_path)?;
    let mut downloaded = 0u64;
    let mut buffer = vec![0u8; 8192]; // 8KB buffer

    // Variables for speed calculation and display frequency
    let mut last_update = Instant::now();
    let mut last_downloaded = 0u64;
    let start_time = Instant::now();
    let update_interval = Duration::from_millis(250); // Update every 250ms

    // Initial progress display
    display_progress(downloaded, total_size, "0 B/s");

    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

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

pub fn download_stage3_sha256(
    profile: &Arch,
    config: &Config,
    filename: &str,
) -> Result<String, ChrootManagerError> {
    let base_urls = get_stage3_url(profile, config);
    let sha256_filename = format!("{filename}.sha256");

    // Build URLs for the SHA256 file
    let sha256_urls: Vec<String> = base_urls
        .iter()
        .map(|base_url| format!("{base_url}{sha256_filename}"))
        .collect();

    let client = reqwest::blocking::Client::new();

    // Attempt to download the SHA256 file with different mirrors (without display)
    let (successful_url, response) = try_download_with_mirrors(&sha256_urls, &client, false)?;

    let sha256_content = response.text()?;

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

    Err(SHA256HashNotFoundInFile)
}
