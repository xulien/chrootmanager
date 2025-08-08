/// Represents an architecture with its available profiles
#[derive(Debug, Clone)]
pub struct Architecture {
    /// Architecture name (e.g., "amd64", "arm64")
    pub name: String,
    pub profiles: Vec<String>,
    /// Default profile for this architecture (e.g., "openrc")
    pub default_profile: String,
}

impl Architecture {
    /// Create a new architecture with profiles
    pub fn new(name: String, profiles: Vec<String>) -> Self {
        // Use the first profile as default, or "openrc" if available
        let default_profile = profiles
            .iter()
            .find(|p| p.contains("openrc") && !p.contains("desktop") && !p.contains("hardened"))
            .or_else(|| profiles.first())
            .cloned()
            .unwrap_or_else(|| "openrc".to_string());

        Self {
            name,
            profiles,
            default_profile,
        }
    }

    /// Get all available profiles for this architecture
    pub fn get_profiles(&self) -> &[String] {
        &self.profiles
    }

    /// Get the default profile for this architecture
    /// Kept for automatic profile selection and future UI enhancements
    #[allow(dead_code)]
    pub fn get_default_profile(&self) -> &str {
        &self.default_profile
    }

    /// Check if a specific profile exists for this architecture
    /// Kept for validation and error handling in profile selection
    #[allow(dead_code)]
    pub fn has_profile(&self, profile: &str) -> bool {
        self.profiles.iter().any(|p| p == profile)
    }
}