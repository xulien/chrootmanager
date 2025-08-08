use crate::chroot::ChrootUnit;
use crate::cli::common::load_chroot_units;
use crate::cli::error::ChrootManagerError;
use colored::Colorize;
use inquire::{InquireError, Select};

/// Enters a chroot environment interactively using a ChrootUnit
///
/// This function handles mounting, entering, and cleaning up the chroot.
fn enter_chroot_with_unit(chroot_unit: &ChrootUnit) -> Result<(), ChrootManagerError> {
    // Show chroot info
    println!("✅ Found chroot: {}", chroot_unit.chroot_path.display());

    if let Ok(profile) = chroot_unit.read_arch_profile_info() {
        println!("📋 Profile: {}", profile.cyan());
    }

    // Pre-authenticate
    println!("🔐 Authenticating for privileged operations...");
    chroot_unit.pre_authenticate_operations().map_err(ChrootManagerError::Chroot)?;

    // Mount filesystems
    println!("🗄️ Mounting filesystems...");
    chroot_unit.mount_filesystems().map_err(ChrootManagerError::Chroot)?;

    let result = chroot_unit.enter_chroot_interactive();

    // Always try to unmount, even if chroot failed
    println!("🧹 Cleaning up filesystems...");
    if let Err(e) = chroot_unit.unmount_filesystems() {
        println!("{}", format!("⚠️ Warning: Failed to unmount filesystems: {e}").yellow());
    } else {
        println!("{}", "✅ Filesystems unmounted successfully".green());
    }

    // Handle chroot result
    match result {
        Ok(()) => {
            println!("{}", format!("✅ Successfully exited chroot '{}'", chroot_unit.name).green());
            Ok(())
        }
        Err(e) => Err(ChrootManagerError::Generic(Box::new(e))),
    }
}

/// Lists all available chroots interactively and allows entering a selected chroot
///
/// This function is used by the interactive list command.
pub async fn list_chroots_interactive() -> Result<(), ChrootManagerError> {
    // Load chroot units using the common function
    let units = load_chroot_units().await?;

    if units.is_empty() {
        return Ok(());
    }

    // Create a list of chroot names for selection
    let units_choices = units.iter().map(|u| u.name.as_str()).collect::<Vec<_>>();

    // Prompt user to select a chroot
    let units_selected: Result<&str, InquireError> =
        Select::new("📋 List of chroots", units_choices)
            .without_help_message()
            .prompt();
    
    let units_selected = units_selected?;
    let unit: Vec<&ChrootUnit> = units.iter().filter(|u| u.name.eq(units_selected)).collect();
    let unit = unit[0];

    // Pre-authenticate for all upcoming privileged operations
    println!(
        "{}",
        "🔐 Requesting authentication for chroot operations..."
            .yellow()
            .bold()
    );
    unit.pre_authenticate_operations().map_err(ChrootManagerError::Chroot)?;

    // Mount filesystems and enter chroot
    unit.mount_filesystems().map_err(ChrootManagerError::Chroot)?;

    // Enter chroot interactively
    let chroot_result = enter_chroot_with_unit(unit);

    // Always try to unmount filesystems after exiting chroot
    println!("{}", "🔧 Cleaning up mount points...".yellow().bold());

    // Clean up mount points directly
    match unit.unmount_filesystems() {
        Ok(_) => println!("{}", "✅ Mount points cleaned up successfully".green()),
        Err(e) => {
            println!("{}", format!("⚠️ Warning during unmounting: {e}").yellow());
            log::warn!("Failed to unmount some filesystems: {e}");
        }
    }

    // Only return the chroot error if it was a real execution error
    match chroot_result {
        Ok(()) => Ok(()),
        Err(e) => {
            // Log the error but don't fail the entire operation
            log::warn!("Chroot session ended with: {e}");
            Ok(())
        }
    }
}
