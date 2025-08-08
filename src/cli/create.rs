use crate::chroot::ChrootUnit;
use crate::cli::common::{handle_existing_chroot, finalize_chroot_creation};
use crate::cli::download::download_stage3_with_cache;
use crate::cli::error::ChrootManagerError;
use crate::cli::load_config;
use crate::profile::manager::ProfileManager;
use crate::profile::selected::SelectedProfile;
use colored::Colorize;
use std::path::PathBuf;

/// Creates a new chroot with the specified name, architecture, and profile
///
/// This function is used by the non-interactive create command.
pub async fn create_chroot(
    name: String,
    arch: String,
    profile: String,
) -> Result<(), ChrootManagerError> {
    let config = load_config().await?;
    println!("{}", "📦 Creating chroot...".green().bold());
    let base_dir_display = config.chroot_base_dir.display();
    println!("   📂 Base directory: {base_dir_display}");

    config.ensure_chroot_base_dir()?;

    let profile_manager = ProfileManager::discover(&config).await?;

    // Validate architecture
    if !profile_manager.has_architecture(arch.as_str()) {
        println!("{}", format!("⚠️ The arch '{arch}' is not supported.").yellow().bold());
        println!("   Available architectures:");
        let arch_choices = profile_manager
            .get_architectures()
            .keys()
            .map(|k| k.to_string())
            .collect::<Vec<String>>();
        for arch_name in arch_choices {
            println!("   • {arch_name}");
        }
        return Err(ChrootManagerError::Custom("The architecture is not supported.".to_string()));
    }

    // Validate profile for the selected architecture
    if !profile_manager.validate_arch_profile(arch.as_str(), profile.as_str()) {
        println!("{}", format!("⚠️ The profile '{profile}' is not supported for arch '{arch}'.").yellow().bold());
        println!("   Available profiles for '{arch}':");
        if let Some(profiles) = profile_manager.get_profiles_for_arch(arch.as_str()) {
            for profile_name in profiles {
                println!("   • {profile_name}");
            }
        }
        return Err(ChrootManagerError::Custom(
            "The profile is not supported for this architecture.".to_string()
        ));
    }

    let selected_profile = SelectedProfile::new(arch, profile);

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

    Ok(())
}
