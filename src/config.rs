use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
pub struct PackageConfiguration {
    pub index: HashMap<String, IndexConfigEntry>,
    pub package: HashMap<String, PackageConfigEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct IndexConfigEntry {
    pub url: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct PackageConfigEntry {
    pub index: Option<String>,
    pub folder: Option<String>,
}

impl PackageConfiguration {
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, ()> {
        let content = std::fs::read_to_string(path).map_err(|e| ())?;
        toml::from_str(&content).map_err(|e| ())
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct CustomerConfig {
    #[serde(flatten)]
    pub customers: HashMap<String, ConfigCustomer>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConfigCustomer {
    pub packages: Vec<String>,
}
