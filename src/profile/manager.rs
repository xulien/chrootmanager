use crate::error::DownloaderError;
use crate::profile::{architecture::Architecture, parser};
use std::collections::HashMap;

/// Profile manager that discovers available architectures and profiles from mirrors
#[derive(Debug)]
pub struct ProfileManager {
    architectures: HashMap<String, Architecture>,
}

impl ProfileManager {
    /// Create a new profile manager by discovering profiles from configured mirrors
    pub async fn discover(config: &crate::config::Config) -> Result<Self, DownloaderError> {
        let parser = parser::ProfileParser::new();

        // Use configured mirrors
        let architectures = parser.discover_profiles_from_config_mirrors(config).await?;

        Ok(Self { architectures })
    }

    /// Get all available architecture names
    pub fn get_architecture_names(&self) -> Vec<&String> {
        let mut names: Vec<&String> = self.architectures.keys().collect();
        names.sort();
        names
    }

    /// Get architecture by name
    pub fn get_architecture(&self, name: &str) -> Option<&Architecture> {
        self.architectures.get(name)
    }

    /// Get all architectures
    pub fn get_architectures(&self) -> &HashMap<String, Architecture> {
        &self.architectures
    }

    /// Check if an architecture exists
    pub fn has_architecture(&self, name: &str) -> bool {
        self.architectures.contains_key(name)
    }

    /// Get profiles for a specific architecture
    pub fn get_profiles_for_arch(&self, arch_name: &str) -> Option<&[String]> {
        self.architectures
            .get(arch_name)
            .map(|arch| arch.get_profiles())
    }

    /// Validate if a combination of architecture and profile is valid
    #[allow(dead_code)]
    pub fn validate_arch_profile(&self, arch_name: &str, profile: &str) -> bool {
        self.architectures
            .get(arch_name)
            .map(|arch| arch.has_profile(profile))
            .unwrap_or(false)
    }
}
