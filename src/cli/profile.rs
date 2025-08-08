use crate::cli::error::ChrootManagerError;
use crate::cli::load_config;
use crate::error::ProfileError;
use crate::error::ProfileError::ArchitectureNotFound;
use crate::profile::{manager::ProfileManager, selected::SelectedProfile};
use colored::Colorize;
use inquire::{InquireError, Select};

/// Display profile information
pub(crate) fn display_profile_info(profile: &SelectedProfile) {
    println!("📋 Selected Profile:");
    println!("   Architecture: {}", profile.arch().cyan().bold());
    println!("   Profile: {}", profile.profile().cyan().bold());
    println!(
        "   Stage3 pattern: {}",
        profile.get_stage3_pattern().dimmed()
    );
}

/// Display architecture selection menu and return selected profile
pub(crate) async fn architecture_profile_selection() -> Result<SelectedProfile, ChrootManagerError>
{
    println!("🔍 Discovering available architectures and profiles...");

    // Load config to use configured mirrors
    let config = load_config().await?;

    // Use configured mirrors
    let profile_manager = ProfileManager::discover(&config).await?;
    let arch_names = profile_manager.get_architecture_names();

    if arch_names.is_empty() {
        return Err(ChrootManagerError::Profile(
            ProfileError::NoArchitecturesAvailable,
        ));
    }

    // Display available architectures
    let arch_strings: Vec<String> = arch_names.iter().map(|s| s.to_string()).collect();
    let arch_selection: Result<String, InquireError> =
        Select::new("Select your architecture:", arch_strings).prompt();
    let selected_arch = arch_selection?;

    // Get profiles for the selected architecture
    let architecture = profile_manager
        .get_architecture(&selected_arch)
        .ok_or_else(|| {
            ChrootManagerError::Profile(ArchitectureNotFound(selected_arch.to_owned()))
        })?;

    let profiles = architecture.get_profiles();
    if profiles.is_empty() {
        return Err(ChrootManagerError::Profile(
            ProfileError::NoProfilesAvailableForArchitecture(selected_arch),
        ));
    }

    // Display available profiles
    let profile_selection: Result<String, InquireError> =
        Select::new("Select your profile:", profiles.to_vec()).prompt();
    let selected_profile = profile_selection?;

    Ok(SelectedProfile::new(selected_arch, selected_profile))
}
