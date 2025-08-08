pub mod command;
pub mod common;
pub mod create;
pub mod create_interactive;
mod error;
pub mod list;
pub mod list_interactive;
pub mod mirror;
pub mod mirror_interactive;
pub(crate) mod download;
pub(crate) mod profile;

use crate::config::{Config, ConfigError};
use crate::mirror::Mirrors;
use inquire::{InquireError, Select};
use std::fs;

pub async fn load_config() -> Result<Config, ConfigError> {
    let config_path = Config::default_config_path();

    if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)?;

        // Try loading with the new format
        match Config::try_parse_config(&config_content) {
            Ok(config) => {
                config.ensure_cache_dir()?;
                Ok(config)
            }
            Err(_) => {
                // New format failed, try migrating from the old format
                println!("🔄 Old configuration migration detected...");
                let migrated_config = Config::migrate_old_config(&config_content)?;

                // Save the new configuration
                migrated_config.save()?;
                println!("✅ Configuration migrated successfully!");

                migrated_config.ensure_cache_dir()?;

                Ok(migrated_config)
            }
        }
    } else {
        // First use — offer mirror selection
        println!("🎉 Welcome to ChrootManager!");
        println!("This is your first use.");
        println!("You need to set up at least one mirror to download stage3 archives.\n");

        let mut config = Config::default();
        config.ensure_cache_dir()?;
        configure_mirrors(&mut config).await?;

        println!("✅ Initial configuration created!\n");

        println!(
            "📂 The chroots will be created in: {}\n",
            config.chroot_base_dir.display()
        );

        Ok(config)
    }
}

/// Interactive function to choose which mirror to save in the configuration
async fn configure_mirrors(config: &mut Config) -> Result<(), ConfigError> {
    let mirrors = Mirrors::fetch().await?;

    loop {
        let selected_option: Result<&str, InquireError> = Select::new(
            "Select one or multiple mirrors",
            vec!["Add mirror", "Save configuration"],
        )
        .without_help_message()
        .prompt();

        match selected_option {
            Ok("Add mirror") => {
                let regions = mirrors.get_regions();
                let selected_region: Result<&str, InquireError> =
                    Select::new("Select your region", regions).prompt();
                let selected_region = selected_region?;

                let countries = mirrors.get_countries(selected_region);
                let selected_country: Result<&str, InquireError> =
                    Select::new("Select your country", countries).prompt();
                let selected_country = selected_country?;

                let locations = mirrors.get_locations(selected_region, selected_country);
                let selected_locations: Result<&str, InquireError> =
                    Select::new("Select your location", locations).prompt();
                let selected_locations = selected_locations?;

                let protocols = mirrors.get_protocols(selected_locations);
                let selected_protocols: Result<&str, InquireError> =
                    Select::new("Select your protocols", protocols).prompt();
                let selected_protocols = selected_protocols?;

                let new_mirror = mirrors.get_url(selected_locations, selected_protocols);
                config.add_mirror(&new_mirror).await?;
            }
            Ok("Save configuration") => {
                // Ensure default mirror
                config.add_mirror("https://distfiles.gentoo.org").await?;
                break;
            }
            _ => panic!("Unknown option!"),
        }
    }

    println!("\n✅ Updated mirror configuration:");
    for (index, mirror_url) in config.mirrors_url.iter().enumerate() {
        println!("  {}. {}", index + 1, mirror_url);
    }

    Ok(())
}