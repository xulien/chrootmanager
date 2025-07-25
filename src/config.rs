use crate::{error::ChrootManagerError, mirror::Mirrors};
use inquire::{InquireError, Select};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub chroot_base_dir: PathBuf,
    pub stage3_cache_dir: PathBuf,
    pub mirrors_url: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let chroot_base_dir = PathBuf::from("/var/lib/chrootmanager");

        let stage3_cache_dir = PathBuf::from("/root")
            .join(".cache")
            .join("chrootmanager")
            .join("stage3");

        Config {
            chroot_base_dir,
            stage3_cache_dir,
            mirrors_url: Vec::new(),
        }
    }
}

impl Config {
    pub fn ensure_chroot_base_dir(&self) -> Result<(), io::Error> {
        if !self.chroot_base_dir.exists() {
            fs::create_dir_all(&self.chroot_base_dir)?;
            log::info!(
                "Chroot directory created : {}",
                self.chroot_base_dir.display()
            );
        }
        Ok(())
    }

    /// Loading configuration with automatic migration
    pub async fn load() -> Result<Self, ChrootManagerError> {
        let config_path = Self::default_config_path();

        if config_path.exists() {
            let config_content = fs::read_to_string(&config_path)?;

            // Try loading with the new format
            match toml::from_str::<Config>(&config_content) {
                Ok(config) => {
                    config.ensure_cache_dir()?;
                    Ok(config)
                }
                Err(_) => {
                    // New format failed, try migrating from old format
                    println!("🔄 Migration of old configuration detected...");
                    let migrated_config = Self::migrate_old_config(&config_content)?;

                    // Save the new configuration
                    migrated_config.save()?;
                    println!("✅ Configuration migrated successfully !");

                    migrated_config.ensure_cache_dir()?;

                    Ok(migrated_config)
                }
            }
        } else {
            // First use — offer mirror selection
            println!("🎉 Welcome to ChrootManager !");
            println!("This is your first use.");
            println!("You need to set up at least one mirror to download stage3 archives.\n");

            let mut config = Self::default();
            config.ensure_cache_dir()?;
            config.configure_mirrors().await?;

            println!("✅ Initial configuration created !\n");

            println!(
                "📂 The chroots will be created in : {}\n",
                config.chroot_base_dir.display()
            );

            Ok(config)
        }
    }

    fn migrate_old_config(old_content: &str) -> Result<Self, ChrootManagerError> {
        use toml::Value;

        // Parse the old format as a generic TOML value
        let old_config: Value = toml::from_str(old_content)?;

        let mut new_config = Self::default();

        // Migrate known fields
        if let Some(old_chroot_dir) = old_config.get("default_chroot_dir") {
            if let Some(dir_str) = old_chroot_dir.as_str() {
                new_config.chroot_base_dir = PathBuf::from(dir_str);
            }
        }

        if let Some(old_mirror) = old_config.get("default_mirror") {
            if let Some(mirror_str) = old_mirror.as_str() {
                new_config.mirrors_url = vec![mirror_str.to_string()];
            }
        }

        println!(
            "   Chroot Directory : {}",
            new_config.chroot_base_dir.display()
        );
        println!(
            "   Configured mirror : {}",
            new_config.mirrors_url.to_vec().join(", ")
        );

        Ok(new_config)
    }

    /// Interactive function to choose which mirror to save in the configuration
    pub async fn configure_mirrors(&mut self) -> Result<(), ChrootManagerError> {
        let mirrors = Mirrors::fetch().await?;

        let mut new_mirrors: HashSet<String> = HashSet::new();

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
                    new_mirrors.insert(new_mirror);
                }
                Ok("Save configuration") => {
                    new_mirrors.insert("https://distfiles.gentoo.org".to_string());
                    self.mirrors_url = new_mirrors.iter().map(|m| m.to_string()).collect();
                    self.save()?;
                    break;
                }
                _ => panic!("option unknown!"),
            }
        }

        println!("\n✅ Updated mirror configuration :");
        for (index, mirror_url) in self.mirrors_url.iter().enumerate() {
            println!("  {}. {}", index + 1, mirror_url);
        }

        Ok(())
    }

    /// Check if any mirrors are configured
    pub fn has_mirrors(&self) -> bool {
        !self.mirrors_url.is_empty()
    }

    pub fn save(&self) -> Result<(), ChrootManagerError> {
        let config_path = Self::default_config_path();

        // Creates the configuration directory if it does not exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let config_content = toml::to_string_pretty(self)?;
        fs::write(&config_path, config_content)?;

        Ok(())
    }

    pub fn ensure_cache_dir(&self) -> Result<(), io::Error> {
        if !self.stage3_cache_dir.exists() {
            fs::create_dir_all(&self.stage3_cache_dir)?;
            log::info!(
                "Répertoire de cache créé : {}",
                self.stage3_cache_dir.display()
            );
        }
        Ok(())
    }

    pub fn get_cache_path(&self, filename: &str) -> PathBuf {
        self.stage3_cache_dir.join(filename)
    }

    fn default_config_path() -> PathBuf {
        let home_dir = home::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home_dir
            .join(".config")
            .join("chrootmanager")
            .join("config.toml")
    }
}
