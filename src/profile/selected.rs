/// Represents a selected architecture and profile combination
#[derive(Debug, Clone)]
pub struct SelectedProfile {
    pub architecture: String,
    pub profile: String,
}

impl SelectedProfile {
    /// Create a newly selected profile
    pub fn new(architecture: String, profile: String) -> Self {
        Self {
            architecture,
            profile,
        }
    }

    /// Get the architecture name
    pub fn arch(&self) -> &str {
        &self.architecture
    }

    /// Get the profile name
    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// Generate the stage3 filename pattern for this profile
    pub fn get_stage3_pattern(&self) -> String {
        format!("stage3-{}-{}", self.architecture, self.profile)
    }
}

impl Default for SelectedProfile {
    fn default() -> Self {
        Self {
            architecture: "amd64".to_string(),
            profile: "openrc".to_string(),
        }
    }
}

impl std::fmt::Display for SelectedProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.architecture, self.profile)
    }
}
