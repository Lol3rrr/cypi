use std::collections::{HashMap, HashSet};

pub mod api;
pub mod auth;
pub mod background;
pub mod config;

#[derive(Debug, serde::Deserialize)]
pub struct CustomerConfig {
    #[serde(flatten)]
    pub customers: HashMap<String, ConfigCustomer>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConfigCustomer {
    pub packages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub src: PackageSrc,
    pub files: Vec<PackageFile>,
}

#[derive(Debug, Clone)]
pub enum PackageFile {
    FilePackage {
        name: String,
        path: std::path::PathBuf,
    },
    RemotePackage {
        name: String,
        url: reqwest::Url,
        auth: RemotePackageAuth,
    },
}

#[derive(Debug, Clone)]
pub enum RemotePackageAuth {
    Unauthorized,
}

#[derive(Debug, Clone)]
pub enum PackageSrc {
    Folder,
    Index { url: reqwest::Url },
}

pub struct State {
    pub packages: HashMap<String, Package>,
    pub customer_packages: HashMap<String, HashSet<String>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            customer_packages: HashMap::new(),
        }
    }
}
