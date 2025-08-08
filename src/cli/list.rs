use crate::cli::common::load_chroot_units;
use crate::cli::error::ChrootManagerError;
use colored::Colorize;

/// Lists all available chroots in a formatted table
///
/// This function is used by the non-interactive list command.
pub async fn list_chroots() -> Result<(), ChrootManagerError> {
    // Load chroot units using the common function
    let units = load_chroot_units().await?;

    if units.is_empty() {
        return Ok(());
    }

    // Display available chroots
    println!("\n   📋 Available chroots:");
    println!("   {:<20} {:<15} {}", "NAME", "PROFILE", "PATH");
    println!("   {}", "─".repeat(60));

    for unit in &units {
        let profile_name = match &unit.profile {
            Some(profile) => format!("{profile}"),
            None => "Undefined".to_string(),
        };

        let path_display = unit.chroot_path.display();
        println!("   {:<20} {:<15} {}", unit.name, profile_name, path_display);
    }

    println!("\n   {}", format!("✅ {} chroot(s) found", units.len()).green());

    Ok(())
}
