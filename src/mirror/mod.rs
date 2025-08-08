use self::parser::{Mirror, Protocol, UriInfo, get_mirrors};
use crate::error::{DownloaderError, MirrorError};
use std::collections::HashSet;

pub mod parser;

/// Verifies if a URL is a valid Gentoo mirror by checking if it responds and has the expected structure
pub async fn verify_mirror_url(url: &str) -> Result<(), MirrorError> {
    println!("🔄 Verifying mirror URL: {url}");

    // Ensure the URL ends with a slash
    let url = if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    };

    // Create a client with a timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // Check if the URL responds
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(MirrorError::InvalidFormat(format!(
            "Mirror URL returned status code: {}",
            response.status()
        )));
    }

    // Check for common Gentoo mirror directories
    // Try to access the releases directory which should exist in a valid Gentoo mirror
    let releases_url = format!("{url}releases/");
    let releases_response = client.get(&releases_url).send().await?;

    if !releases_response.status().is_success() {
        return Err(MirrorError::InvalidFormat(format!(
            "URL does not appear to be a valid Gentoo mirror. Could not access the releases directory: {releases_url}"
        )));
    }

    println!("✅ Mirror URL verified successfully");
    Ok(())
}

#[derive(Debug)]
pub struct Mirrors {
    mirrors: Vec<Mirror>,
}

impl Mirrors {
    pub async fn fetch() -> Result<Self, DownloaderError> {
        println!("\n🔄 Retrieving the list of mirror...");

        let mirrors = match get_mirrors().await {
            Ok(mirrors) => mirrors,
            Err(e) => {
                return Err(DownloaderError::RetrievingMirror(e.to_string()));
            }
        };

        println!("✅ {} mirror found\n", mirrors.len());

        Ok(Self { mirrors })
    }

    pub fn get_regions(&self) -> Vec<&str> {
        let mut regions: HashSet<&str> = HashSet::new();

        for mirror in &self.mirrors {
            regions.insert(mirror.group.region.as_str());
        }

        let mut regions: Vec<&str> = regions.into_iter().collect();
        regions.sort();
        regions
    }

    pub fn get_countries(&self, region: &str) -> Vec<&str> {
        let mut countries: HashSet<&str> = HashSet::new();
        for mirror in &self.mirrors {
            if mirror.group.region.eq(region) {
                countries.insert(mirror.group.country_name.as_str());
            }
        }
        let mut countries: Vec<&str> = countries.into_iter().collect();
        countries.sort();
        countries
    }

    pub fn get_locations(&self, region: &str, countries: &str) -> Vec<&str> {
        let mut locations: Vec<&str> = self
            .mirrors
            .iter()
            .filter(|m| m.group.region.eq(region) && m.group.country_name.eq(countries))
            .map(|m| m.name.as_str())
            .collect();
        locations.sort();
        locations
    }

    pub fn get_uris_info(&self, location: &str) -> Vec<UriInfo> {
        let uri_infos: Vec<Vec<UriInfo>> = self
            .mirrors
            .iter()
            .filter(|m| m.name.eq(location))
            .map(|m| m.group.mirrors.clone())
            .collect();
        uri_infos[0].clone()
    }

    pub fn get_protocols(&self, location: &str) -> Vec<&str> {
        let uri_infos = self.get_uris_info(location);
        let mut protocols: Vec<&str> = uri_infos
            .iter()
            .map(|info| info.protocol.as_str())
            .collect();
        protocols.sort();
        protocols
    }

    pub fn get_url(&self, location: &str, protocol: &str) -> String {
        let uri_infos = self.get_uris_info(location);
        let uri_infos_filtered: Vec<&UriInfo> = uri_infos
            .iter()
            .filter(|info| info.protocol.eq(&Protocol::from(protocol)))
            .collect();
        uri_infos_filtered[0].uri.clone()
    }
}
