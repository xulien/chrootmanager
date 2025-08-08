pub use crate::error::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{fs, io, path::PathBuf};
use toml::de::Error;
use toml::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub chroot_base_dir: PathBuf,
    pub stage3_cache_dir: PathBuf,
    pub mirrors_url: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = home::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));

        // Chroot in user space
        let chroot_base_dir = home_dir
            .join(".local")
            .join("share")
            .join("chrootmanager")
            .join("chroots");

        // Cache in user space
        let stage3_cache_dir = home_dir.join(".cache").join("chrootmanager").join("stage3");

        let config = Config {
            chroot_base_dir,
            stage3_cache_dir,
            mirrors_url: Vec::new(),
        };

        // Ensure all default directories exist
        if let Err(e) = config.ensure_default_directories() {
            log::warn!("Failed to create default directories: {e}");
        }

        config
    }
}

impl Config {
    /// Ensure all default directories exist
    fn ensure_default_directories(&self) -> Result<(), ConfigError> {
        // Create chroot base directory
        if !self.chroot_base_dir.exists() {
            fs::create_dir_all(&self.chroot_base_dir)?;
            log::info!(
                "Chroot base directory created: {}",
                self.chroot_base_dir.display()
            );
        }

        // Create a cache directory
        if !self.stage3_cache_dir.exists() {
            fs::create_dir_all(&self.stage3_cache_dir)?;
            log::info!(
                "Cache directory created: {}",
                self.stage3_cache_dir.display()
            );
        }

        // Create config directory
        let config_path = Self::default_config_path();
        if let Some(config_dir) = config_path.parent() {
            if !config_dir.exists() {
                fs::create_dir_all(config_dir)?;
                log::info!("Configuration directory created: {}", config_dir.display());
            }
        }

        Ok(())
    }

    pub fn ensure_chroot_base_dir(&self) -> Result<(), ConfigError> {
        if !self.chroot_base_dir.exists() {
            fs::create_dir_all(&self.chroot_base_dir)?;
            log::info!(
                "Chroot directory created: {}",
                self.chroot_base_dir.display()
            );
        }
        Ok(())
    }

    pub async fn add_mirror(&mut self, mirror_url: &str) -> Result<(), ConfigError> {
        let mut new_mirrors: HashSet<&str> = HashSet::new();
        new_mirrors.extend(self.mirrors_url.iter().map(|m| m.as_str()));
        new_mirrors.insert(mirror_url);
        self.mirrors_url = new_mirrors.iter().map(|m| m.to_string()).collect();
        self.save()?;
        Ok(())
    }

    pub fn try_parse_config(config_content: &str) -> Result<Config, Error> {
        toml::from_str::<Config>(config_content)
    }

    pub fn migrate_old_config(old_content: &str) -> Result<Self, ConfigError> {
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
            "   Chroot Directory: {}",
            new_config.chroot_base_dir.display()
        );
        println!(
            "   Configured mirror: {}",
            new_config.mirrors_url.to_vec().join(", ")
        );

        Ok(new_config)
    }

    /// Check if any mirrors are configured
    pub fn has_mirrors(&self) -> bool {
        !self.mirrors_url.is_empty()
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::default_config_path();

        // Create the configuration directory if it does not exist
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
                log::info!("Configuration directory created: {}", parent.display());
            }
        }

        let config_content = toml::to_string_pretty(self)?;
        fs::write(&config_path, config_content)?;

        Ok(())
    }

    pub fn ensure_cache_dir(&self) -> Result<(), io::Error> {
        if !self.stage3_cache_dir.exists() {
            fs::create_dir_all(&self.stage3_cache_dir)?;
            log::info!(
                "Cache directory created: {}",
                self.stage3_cache_dir.display()
            );
        }
        Ok(())
    }

    pub fn get_cache_path(&self, filename: &str) -> PathBuf {
        self.stage3_cache_dir.join(filename)
    }

    pub fn default_config_path() -> PathBuf {
        let home_dir = home::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home_dir
            .join(".config")
            .join("chrootmanager")
            .join("config.toml")
    }
}
