use crate::error::MirrorError;
use log::{debug, info, warn};
use std::io::Cursor;
use xml::reader::{EventReader, XmlEvent};

const MIRRORS_URL: &str = "https://api.gentoo.org/mirrors/distfiles.xml";

#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Http,
    Https,
    Ftp,
    Rsync,
    Unknown,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Http => "http",
            Protocol::Https => "https",
            Protocol::Ftp => "ftp",
            Protocol::Rsync => "rsync",
            Protocol::Unknown => "unknown",
        }
    }
}

impl From<&str> for Protocol {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "http" => Protocol::Http,
            "https" => Protocol::Https,
            "ftp" => Protocol::Ftp,
            "rsync" => Protocol::Rsync,
            _ => Protocol::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UriInfo {
    pub protocol: Protocol,
    pub ipv4: bool,
    pub ipv6: bool,
    pub partial: bool,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct MirrorGroup {
    pub name: String,
    pub region: String,
    pub country_code: String,
    pub country_name: String,
    pub mirrors: Vec<UriInfo>,
}

#[derive(Debug, Clone)]
pub struct Mirror {
    pub name: String,
    pub group: MirrorGroup,
}

impl Mirror {
    fn new() -> Self {
        Mirror {
            name: String::new(),
            group: MirrorGroup::new(),
        }
    }
}

impl MirrorGroup {
    fn new() -> Self {
        MirrorGroup {
            name: String::new(),
            region: String::new(),
            country_code: String::new(),
            country_name: String::new(),
            mirrors: Vec::new(),
        }
    }
}

impl UriInfo {
    fn new() -> Self {
        UriInfo {
            protocol: Protocol::Unknown,
            ipv4: false,
            ipv6: false,
            partial: false,
            uri: String::new(),
        }
    }
}

/// Parse the XML data of mirrors and return a list of mirrors
fn parse_mirrors_xml(data: &[u8]) -> Result<Vec<Mirror>, MirrorError> {
    // Validation du format de base
    if data.is_empty() {
        return Err(MirrorError::EmptyDataReceived);
    }

    let cursor = Cursor::new(data);
    let parser = EventReader::new(cursor);

    let mut mirrors = Vec::new();
    let mut current_group = MirrorGroup::new();
    let mut current_mirror = Mirror::new();
    let mut current_uri = UriInfo::new();
    let mut current_text = String::new();
    let mut in_mirrorgroup = false;
    let mut in_mirror = false;
    let mut in_uri = false;
    let mut in_name = false;
    let mut found_mirrors_root = false;

    for event in parser {
        match event? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                current_text.clear();

                debug!("{name} {attributes:?}");

                match name.local_name.as_str() {
                    "mirrors" => {
                        found_mirrors_root = true;
                        debug!("Élément racine 'mirrors' trouvé");
                    }
                    "mirrorgroup" => {
                        if !found_mirrors_root {
                            return Err(MirrorError::InvalidFormat(
                                "Element 'mirrorgroup' found without a root element 'mirrors'"
                                    .to_string(),
                            ));
                        }
                        debug!("Start of a mirrorgroup");
                        in_mirrorgroup = true;
                        current_group = MirrorGroup::new();

                        // Extract mirrorgroup attributes
                        for attr in attributes {
                            debug!(
                                "mirrorgroup attributes: {} = {}",
                                attr.name.local_name, attr.value
                            );
                            match attr.name.local_name.as_str() {
                                "region" => current_group.region = attr.value,
                                "country" => current_group.country_code = attr.value,
                                "countryname" => current_group.country_name = attr.value,
                                _ => {
                                    debug!(
                                        "Mirrorgroup attribute ignored : {}",
                                        attr.name.local_name
                                    );
                                }
                            }
                        }

                        debug!(
                            "Group created with region='{}', country_code='{}', country_name='{}'",
                            current_group.region,
                            current_group.country_code,
                            current_group.country_name
                        );
                    }
                    "mirror" => {
                        if !in_mirrorgroup {
                            return Err(MirrorError::InvalidFormat(
                                "'Mirror' element found outside a mirrorgroup".to_string(),
                            ));
                        }
                        debug!("Beginning of a mirror");
                        in_mirror = true;
                        current_mirror = Mirror::new();
                        // Copy current group information
                        current_mirror.group = current_group.clone();
                    }
                    "name" => {
                        if in_mirror {
                            debug!("Start of a name element for the mirror");
                            in_name = true;
                        }
                    }
                    "uri" => {
                        if !in_mirror {
                            return Err(MirrorError::InvalidFormat(
                                "'uri' element found outside a mirror".to_string(),
                            ));
                        }
                        debug!("Start of a URI");
                        in_uri = true;
                        current_uri = UriInfo::new();

                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "protocol" => {
                                    current_uri.protocol = Protocol::from(attr.value.as_str())
                                }
                                "ipv4" => current_uri.ipv4 = attr.value == "y",
                                "ipv6" => current_uri.ipv6 = attr.value == "y",
                                "partial" => current_uri.partial = attr.value == "y",
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }

            XmlEvent::Characters(text) => {
                current_text.push_str(&text);
            }

            XmlEvent::EndElement { name } => {
                let element_name = name.local_name;

                match element_name.as_str() {
                    "mirrorgroup" => {
                        if in_mirrorgroup {
                            debug!(
                                "End of the mirrorgroup: region='{}', country_code='{}', country_name='{}'",
                                current_group.region,
                                current_group.country_code,
                                current_group.country_name
                            );
                            in_mirrorgroup = false;
                        }
                    }
                    "mirror" => {
                        if in_mirror {
                            debug!(
                                "End of mirror: '{}' with {} URIs",
                                current_mirror.name,
                                current_mirror.group.mirrors.len()
                            );

                            // Set the group name if necessary
                            current_mirror.group.name = current_mirror.name.clone();

                            // Accept mirrors that have at least URIs
                            if !current_mirror.group.mirrors.is_empty() {
                                // Si le nom est vide, utiliser la première URI comme nom
                                if current_mirror.name.is_empty() {
                                    current_mirror.name =
                                        current_mirror.group.mirrors[0].uri.clone();
                                    current_mirror.group.name = current_mirror.name.clone();
                                    warn!(
                                        "Missing mirror name, using URI : {}",
                                        current_mirror.name
                                    );
                                }
                                mirrors.push(current_mirror.clone());
                                debug!("Mirror added : {}", current_mirror.name);
                            } else {
                                warn!("Mirror ignored because no URI : {}", current_mirror.name);
                            }
                            current_mirror = Mirror::new();
                            in_mirror = false;
                        }
                    }
                    "name" => {
                        if in_name {
                            let name_text = current_text.trim();
                            debug!("Mirror name found: '{name_text}'");
                            if !name_text.is_empty() {
                                current_mirror.name = name_text.to_string();
                            }
                            in_name = false;
                        }
                    }
                    "uri" => {
                        if in_uri {
                            let uri_text = current_text.trim();
                            debug!("End of URI : '{uri_text}'");

                            if !uri_text.is_empty() {
                                current_uri.uri = uri_text.to_string();
                                // Directly add the URI to the current mirror group
                                current_mirror.group.mirrors.push(current_uri.clone());
                                debug!("URI added : {uri_text}");
                            } else {
                                warn!("Empty URI ignored");
                            }
                            current_uri = UriInfo::new();
                            in_uri = false;
                        }
                    }
                    _ => {}
                }

                current_text.clear();
            }

            XmlEvent::EndDocument => break,
            _ => {}
        }
    }

    if !found_mirrors_root {
        return Err(MirrorError::NoRootElementIntoMirrors);
    }

    info!("Parsing completed. {} mirrors found", mirrors.len());
    Ok(mirrors)
}

pub async fn get_mirrors() -> Result<Vec<Mirror>, MirrorError> {
    info!("Data recovery from {MIRRORS_URL}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let response = client.get(MIRRORS_URL).send().await?;
    let data = response.bytes().await?;

    info!("Data received: {} bytes", data.len());

    parse_mirrors_xml(&data)
}
