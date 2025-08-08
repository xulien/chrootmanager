//! Profile parser module for discovering available profiles from Gentoo mirrors

use crate::error::DownloaderError;
use crate::profile::Architecture;
use log::{debug, info, warn};
use std::collections::HashMap;

/// Parser for discovering profiles from Gentoo mirrors
pub struct ProfileParser {
    client: reqwest::Client,
}

impl ProfileParser {
    /// Create a new profile parser
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Discover profiles using only the configured mirrors
    pub async fn discover_profiles_from_config_mirrors(
        &self,
        config: &crate::config::Config,
    ) -> Result<HashMap<String, Architecture>, DownloaderError> {
        info!("🔍 Discovering available profiles from configured mirrors...");
        debug!("Config has_mirrors: {}", config.has_mirrors());
        debug!("Number of configured mirrors: {}", config.mirrors_url.len());

        // Check if mirrors are configured
        if !config.has_mirrors() {
            warn!("⚠️ No mirrors configured using fallback architectures");
            return Ok(self.get_fallback_architectures());
        }

        // Try each configured mirror
        for (index, mirror_url) in config.mirrors_url.iter().enumerate() {
            debug!("Trying to configure mirror {index}: {mirror_url}", index = index + 1);

            match self.discover_from_mirror(mirror_url).await {
                Ok(architectures) => {
                    debug!("Successfully discovered {count} architectures from mirror: {mirror_url}", 
                           count = architectures.len());
                    for (arch_name, arch) in &architectures {
                        debug!("  - Architecture '{arch_name}' with {count} profiles: {profiles:?}", 
                               count = arch.profiles.len(), profiles = arch.profiles);
                    }
                    info!("✅ Successfully discovered profiles from configured mirror: {mirror_url}");
                    return Ok(architectures);
                }
                Err(e) => {
                    warn!("Failed to discover from configured mirror {mirror_url}: {e}");
                    debug!("Mirror failure details: {e:?}");
                    continue;
                }
            }
        }

        // If all configured mirrors fail, return hardcoded fallback
        warn!("⚠️ Could not discover profiles from any configured mirror using fallback");
        debug!("Falling back to hardcoded architectures");
        Ok(self.get_fallback_architectures())
    }

    /// Discover profiles from a specific mirror
    async fn discover_from_mirror(
        &self,
        base_url: &str,
    ) -> Result<HashMap<String, Architecture>, DownloaderError> {
        let releases_url = format!("{}/releases/", base_url.trim_end_matches('/'));

        debug!("Fetching releases from: {releases_url}");

        // Make HTTP request to get release page
        let response = self.client.get(&releases_url).send().await?;
        debug!("HTTP response status: {}", response.status());
        debug!("HTTP response headers: {headers:?}", headers = response.headers());
        
        let content = response.text().await?;
        debug!("HTML content length: {len} bytes", len = content.len());
        debug!("First 500 chars of HTML: {preview}", preview = &content[..content.len().min(500)]);

        // Parse HTML to find architecture directories
        let architectures = self.parse_architecture_directories(&content)?;
        debug!("Parsed architectures from HTML: {architectures:?}");

        let mut result = HashMap::new();

        for arch_name in &architectures {
            debug!("Discovering profiles for architecture: {arch_name}");

            match self
                .discover_profiles_for_arch(&releases_url, arch_name.as_str())
                .await
            {
                Ok(profiles) => {
                    debug!("Found {count} profiles for {arch_name}: {profiles:?}", 
                           count = profiles.len());
                    if !profiles.is_empty() {
                        result.insert((*arch_name).clone(), Architecture::new((*arch_name).clone(), profiles));
                        debug!("Added architecture '{arch_name}' to results");
                    } else {
                        debug!("Skipping architecture '{arch_name}' - no profiles found");
                    }
                }
                Err(e) => {
                    warn!("Failed to discover profiles for {arch_name}: {e}");
                    debug!("Profile discovery error details for {arch_name}: {e:?}");
                }
            }
        }

        debug!("The final result contains {count} architectures: {archs:?}",
               count = result.len(), archs = result.keys().collect::<Vec<_>>());

        if result.is_empty() {
            debug!("No valid architectures found, returning error");
            return Err(DownloaderError::RetrievingMirror(
                "No valid architecture found".to_string(),
            ));
        }

        Ok(result)
    }

    /// Parse HTML content to find architecture directories
    fn parse_architecture_directories(&self, html: &str) -> Result<Vec<String>, DownloaderError> {
        debug!("Starting HTML parsing for architecture directories");
        let mut architectures = Vec::new();
        let mut line_count = 0;

        // Simple HTML parsing to find href links that look like architectures
        for line in html.lines() {
            line_count += 1;
            
            if line.contains("href=") && line.contains("/") {
                debug!("Line {line_count}: Found href link: {line}", line = line.trim());
                
                // Extract directory names that look like architectures
                if let Some(arch) = self.extract_architecture_from_line(line) {
                    debug!("Line {line_count}: Extracted potential architecture: '{arch}'");
                    
                    if self.is_valid_architecture(&arch) {
                        debug!("Line {line_count}: '{arch}' is a valid architecture");
                        architectures.push(arch);
                    } else {
                        debug!("Line {line_count}: '{arch}' is not a valid architecture");
                    }
                } else {
                    debug!("Line {line_count}: Could not extract architecture from line");
                }
            }
        }

        debug!("Processed {line_count} lines total");
        architectures.sort();
        architectures.dedup();

        debug!("Found architectures: {architectures:?}");
        info!("Architecture discovery completed: found {count} architectures", 
              count = architectures.len());
        Ok(architectures)
    }

    /// Extract architecture name from HTML line
    fn extract_architecture_from_line(&self, line: &str) -> Option<String> {
        debug!("Extracting architecture from line: {line}", line = line.trim());
        
        // Look for patterns like href="amd64/" or href="arm64/"
        if let Some(start) = line.find("href=\"") {
            let start = start + 6; // Skip 'href="'
            debug!("Found href start at position: {start}");
            
            if let Some(end) = line[start..].find('"') {
                let href = &line[start..start + end];
                debug!("Extracted href: '{href}'");
                
                if href.ends_with('/') {
                    let arch = href.trim_end_matches('/');
                    debug!("Extracted architecture candidate: '{arch}'");
                    return Some(arch.to_string());
                } else {
                    debug!("Href does not end with '/', skipping: '{href}'");
                }
            } else {
                debug!("Could not find the closing quote for href");
            }
        } else {
            debug!("No href found in the line");
        }
        
        None
    }

    /// Check if a string looks like a valid architecture name
    fn is_valid_architecture(&self, name: &str) -> bool {
        // Known architecture patterns
        let valid = matches!(
            name,
            "amd64"
                | "arm64"
                | "arm"
                | "x86"
                | "ppc64"
                | "ppc"
                | "sparc"
                | "alpha"
                | "hppa"
                | "ia64"
                | "mips"
                | "riscv"
                | "s390"
        );
        
        debug!("Architecture validation for '{name}': {valid}");
        valid
    }

    /// Discover profiles for a specific architecture
    async fn discover_profiles_for_arch(
        &self,
        releases_url: &str,
        arch: &str,
    ) -> Result<Vec<String>, DownloaderError> {
        let autobuilds_url = format!("{releases_url}{arch}/autobuilds/");

        debug!("Fetching autobuilds directory from: {autobuilds_url}");

        // Make an HTTP request to get autobuilds page
        let response = self.client.get(&autobuilds_url).send().await?;
        debug!("HTTP response status for {arch} autobuilds: {status}", status = response.status());

        if !response.status().is_success() {
            debug!("Non-success status for {arch} autobuilds: {status}", status = response.status());
            return Ok(Vec::new()); // Return empty instead of error for non-critical failures
        }

        let content = response.text().await?;
        debug!("HTML content length for {arch} autobuilds: {len} bytes", len = content.len());
        debug!("First 300 chars of {arch} autobuilds HTML: {preview}",
               preview = &content[..content.len().min(300)]);

        let profiles = self.parse_autobuilds_directories(&content, arch)?;

        debug!("Found profiles for {arch}: {profiles:?}");
        Ok(profiles)
    }

    /// Parse HTML content from the autobuilds directory to find profile directories
    fn parse_autobuilds_directories(&self, html: &str, arch: &str) -> Result<Vec<String>, DownloaderError> {
        debug!("Parsing autobuilds directories for architecture: {arch}");

        let mut profiles = Vec::new();
        let current_stage3_prefix = format!("current-stage3-{arch}-");
        let mut line_count = 0;
        let mut directories_found = 0;

        debug!("Looking for directories with the prefix: '{current_stage3_prefix}'");

        for line in html.lines() {
            line_count += 1;

            if line.contains("href=") && line.contains(&current_stage3_prefix) {
                directories_found += 1;
                debug!("Line {line_count}: Found current-stage3 directory line: {line}", line = line.trim());

                if let Some(profile) = self.extract_profile_from_autobuilds_line(line, arch) {
                    debug!("Line {line_count}: Extracted profile: '{profile}'");
                    profiles.push(profile);
                } else {
                    debug!("Line {line_count}: Could not extract profile from autobuilds line");
                }
            }
        }

        debug!("Processed {line_count} lines, found {directories_found} current-stage3 directories");

        profiles.sort();
        profiles.dedup();

        debug!("Final profiles for {arch}: {profiles:?}");

        // If no profiles found, add a default profile
        if profiles.is_empty() {
            debug!("No profiles extracted, adding the default 'openrc' profile");
            profiles.push("openrc".to_string());
        }

        Ok(profiles)
    }

    /// Extract profile name from current-stage3 directory name in HTML
    fn extract_profile_from_autobuilds_line(&self, line: &str, arch: &str) -> Option<String> {
        debug!("Extracting profile from autobuilds line for {arch}: {line}", line = line.trim());

        let current_stage3_prefix = format!("current-stage3-{arch}-");

        // Clean the HTML line first to handle malformed HTML
        let clean_line = line.replace("\"", "").replace(">", " ");
        debug!("Cleaned line: {clean_line}");

        // Find the directory name
        if let Some(start) = clean_line.find(&current_stage3_prefix) {
            debug!("Found current-stage3 prefix at position: {start}");

            let remaining = &clean_line[start + current_stage3_prefix.len()..];
            debug!("Remaining part after prefix: '{remaining}'");

            // Find the end of the directory name (before / or space)
            if let Some(end) = remaining.find('/') {
                let profile = &remaining[..end];
                debug!("Found profile before slash: '{profile}'");
                return Some(profile.to_string());
            } else if let Some(end) = remaining.find(' ') {
                let profile = &remaining[..end];
                debug!("Found profile before space: '{profile}'");
                return Some(profile.to_string());
            }

            debug!("Could not parse the profile from the remaining part: '{remaining}'");
        } else {
            debug!("Current-stage3 prefix '{current_stage3_prefix}' is not found in the cleaned line");
        }

        None
    }

    /// Get fallback architectures when mirror discovery fails
    fn get_fallback_architectures(&self) -> HashMap<String, Architecture> {
        debug!("Creating fallback architectures");
        let mut architectures = HashMap::new();

        // AMD64 profiles
        let amd64_profiles = vec![
            "desktop-openrc".to_string(),
            "desktop-systemd".to_string(),
            "hardened-selinux-openrc".to_string(),
            "hardened-openrc".to_string(),
            "hardened-systemd".to_string(),
            "llvm-openrc".to_string(),
            "llvm-systemd".to_string(),
            "musl-hardened".to_string(),
            "musl-llvm".to_string(),
            "musl".to_string(),
            "no-multilib-openrc".to_string(),
            "no-multilib-systemd".to_string(),
            "openrc-splitusr".to_string(),
            "openrc".to_string(),
            "systemd".to_string(),
            "x32-openrc".to_string(),
            "x32-systemd".to_string(),
        ];
        debug!("Adding amd64 architecture with {count} profiles", count = amd64_profiles.len());
        architectures.insert(
            "amd64".to_string(),
            Architecture::new("amd64".to_string(), amd64_profiles),
        );

        // ARM64 profiles
        let arm64_profiles = vec![
            "aarch64be-openrc".to_string(),
            "aarch64be-systemd".to_string(),
            "desktop-openrc".to_string(),
            "desktop-systemd".to_string(),
            "llvm-openrc".to_string(),
            "llvm-systemd".to_string(),
            "musl-hardened".to_string(),
            "musl-llvm".to_string(),
            "musl".to_string(),
            "openrc-splitusr".to_string(),
            "openrc".to_string(),
            "systemd".to_string(),
        ];
        debug!("Adding arm64 architecture with {count} profiles", count = arm64_profiles.len());
        architectures.insert(
            "arm64".to_string(),
            Architecture::new("arm64".to_string(), arm64_profiles),
        );

        // SPARC profiles (adding this since it's being detected)
        let sparc_profiles = vec![
            "openrc".to_string(),
        ];
        debug!("Adding sparc architecture with {count} profiles", count = sparc_profiles.len());
        architectures.insert(
            "sparc".to_string(),
            Architecture::new("sparc".to_string(), sparc_profiles),
        );

        debug!("Created {count} fallback architectures: {archs:?}", 
               count = architectures.len(), archs = architectures.keys().collect::<Vec<_>>());
        
        architectures
    }
}

impl Default for ProfileParser {
    fn default() -> Self {
        Self::new()
    }
}