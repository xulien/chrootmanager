use crate::{
    error::ChrootManagerError,
    mirror::parser::{get_mirrors, Mirror, Protocol, UriInfo}
};
use std::collections::HashSet;

pub(crate) mod parser;

#[derive(Debug)]
pub enum MirrorError {
    Network(reqwest::Error),
    XmlParsing(xml::reader::Error),
    InvalidFormat(String),
}

impl std::fmt::Display for MirrorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MirrorError::Network(e) => write!(f, "Network error : {e}"),
            MirrorError::XmlParsing(e) => write!(f, "XML parsing error : {e}"),
            MirrorError::InvalidFormat(msg) => write!(f, "Invalid format : {msg}"),
        }
    }
}

impl std::error::Error for MirrorError {}

#[derive(Debug)]
pub struct Mirrors {
    mirrors: Vec<Mirror>,
}

impl Mirrors {
    pub async fn fetch() -> Result<Self, ChrootManagerError> {
        println!("\n🔄 Retrieving the list of mirror...");

        let mirrors = match get_mirrors().await {
            Ok(mirrors) => mirrors,
            Err(e) => {
                eprintln!("❌ Error retrieving mirror : {e}");
                return Err(ChrootManagerError::Config(e.to_string()));
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
