use crate::chroot::ChrootUnit;
use crate::cli::error::ChrootManagerError;
use crate::error::ChrootError;
use colored::Colorize;
use std::fs;
use std::io::Write;

/// Loads and validates chroot units from the base directory
pub async fn load_chroot_units() -> Result<Vec<ChrootUnit>, ChrootManagerError> {
    let config = crate::cli::load_config().await?;
    let base_dir_display = config.chroot_base_dir.display();
    println!("   📂 Chroot Directory: {base_dir_display}");

    if !config.chroot_base_dir.exists() {
        println!("   ❌ Chroot directory not found");
        println!("   The directory will be created when the first chroot is created");
        println!("   Make sure you have permissions to create chroots");
        return Ok(Vec::new());
    }

    let rd = fs::read_dir(&config.chroot_base_dir);

    if let Err(e) = rd {
        println!("   ❌ Directory access error: {e}");
        let base_dir_display = config.chroot_base_dir.display();
        println!("   💡 Check permissions for: {base_dir_display}");
        return Ok(Vec::new());
    }

    let dirs = rd
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|e| e.is_dir())
        .collect::<Vec<_>>();

    match dirs
        .iter()
        .map(|p| ChrootUnit::load(p))
        .collect::<Result<Vec<ChrootUnit>, ChrootError>>()
    {
        Ok(units) => Ok(units),
        Err(e) => Err(ChrootManagerError::Chroot(e)),
    }
}

/// Checks if a chroot already exists and handles the case
pub fn handle_existing_chroot(chroot_unit: &ChrootUnit) -> Result<bool, ChrootManagerError> {
    if chroot_unit.chroot_path.exists() {
        let chroot_name = &chroot_unit.name;
        println!(
            "{}",
            format!("⚠️ The chroot '{chroot_name}' already exists.")
                .yellow()
                .bold()
        );
        print!("Do you want to delete and recreate it? (y/N): ");
        std::io::stdout().flush().map_err(ChrootManagerError::Io)?;

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(ChrootManagerError::Io)?;

        if input.trim().to_lowercase().starts_with('y') {
            println!("{}", "🗑️ Removing the old chroot...".red().bold());
            chroot_unit.cleanup(true).map_err(ChrootManagerError::Chroot)?;
            println!("✅ Old chroot deleted");
            Ok(true)
        } else {
            let chroot_name = &chroot_unit.name;
            Err(ChrootManagerError::Custom(format!(
                "The chroot '{chroot_name}' already exists. Use another name or delete it first."
            )))
        }
    } else {
        Ok(false)
    }
}

/// Finalizes chroot creation with common steps
pub async fn finalize_chroot_creation(
    chroot_unit: &ChrootUnit,
    cached_path: &std::path::Path,
) -> Result<(), ChrootManagerError> {
    chroot_unit.prepare_chroot_directory().await.map_err(ChrootManagerError::Chroot)?;
    chroot_unit.extract_stage3(cached_path).await.map_err(ChrootManagerError::Chroot)?;
    chroot_unit.copy_dns_info().map_err(ChrootManagerError::Chroot)?;
    chroot_unit.write_arch_profile_info().map_err(ChrootManagerError::Chroot)?;

    println!(
        "{}",
        format!("✅ Chroot '{}' created successfully!", chroot_unit.name)
            .green()
            .bold()
    );
    let chroot_path_display = chroot_unit.chroot_path.display();
    println!("📍 Path: {chroot_path_display}");

    Ok(())
}