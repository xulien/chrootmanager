use crate::chroot::ChrootUnit;
use crate::cli::common::{handle_existing_chroot, finalize_chroot_creation};
use crate::cli::download::download_stage3_with_cache;
use crate::cli::error::ChrootManagerError;
use crate::cli::list_interactive::list_chroots_interactive;
use crate::cli::load_config;
use crate::cli::profile::{architecture_profile_selection, display_profile_info};
use colored::Colorize;
use std::path::PathBuf;

/// Creates a new chroot interactively with the specified name
pub async fn create_chroot_interactive(name: String) -> Result<(), ChrootManagerError> {
    let config = load_config().await?;
    println!("{}", "📦 Creating chroot...".green().bold());
    let base_dir_display = config.chroot_base_dir.display();
    println!("   📂 Base directory: {base_dir_display}");

    config.ensure_chroot_base_dir()?;

    // Use the interactive profile selection system
    let selected_profile = architecture_profile_selection().await?;
    display_profile_info(&selected_profile);

    let chroot_unit = ChrootUnit::new(name.clone(), Some(&selected_profile), &config).await
        .map_err(ChrootManagerError::Chroot)?;

    log::debug!("chroot path: {:?}", chroot_unit.chroot_path);

    // Check if chroot already exists using the common function
    handle_existing_chroot(&chroot_unit)?;

    // Download stage3 archive
    let cached_path = download_stage3_with_cache(&selected_profile, &config).await?;
    let cached_path = PathBuf::from(cached_path);

    // Finalize chroot creation using the common function
    finalize_chroot_creation(&chroot_unit, &cached_path).await?;

    // Show the list of chroots interactively
    list_chroots_interactive().await?;

    Ok(())
}
