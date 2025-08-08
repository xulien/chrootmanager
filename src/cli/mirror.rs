use crate::cli::error::ChrootManagerError;
use crate::cli::load_config;
use crate::mirror::verify_mirror_url;
use colored::Colorize;

/// Adds a new mirror to the configuration after verifying it
pub async fn setup_mirrors(new_mirror: String) -> Result<(), ChrootManagerError> {
    // Verify that the URL is a valid Gentoo mirror before adding it
    println!("🔄 Verifying mirror URL...");
    verify_mirror_url(&new_mirror).await?;
    
    // If verification succeeds, proceed with adding the mirror
    let mut config = load_config().await?;
    
    config.add_mirror(&new_mirror).await?;
    
    println!("{}", format!("✅ Mirror '{new_mirror}' added successfully").green().bold());
    
    // Save the configuration
    config.save()?;
    
    Ok(())
}
