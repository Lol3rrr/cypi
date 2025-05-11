use std::collections::{HashMap, HashSet};

pub mod api;
pub mod auth;
pub mod background;
pub mod config;

/// A specific package
#[derive(Debug, Clone)]
pub struct Package {
    pub src: PackageSrc,
    pub files: Vec<PackageFile>,
}

/// A specific file for a package
#[derive(Debug, Clone)]
pub enum PackageFile {
    /// A locally stored package file
    FilePackage {
        name: String,
        path: std::path::PathBuf,
    },
    /// A remotely stored package file (potentially requiring auth)
    RemotePackage {
        name: String,
        url: reqwest::Url,
        auth: RemotePackageAuth,
    },
}

/// Auth for remotely stored packages
#[derive(Debug, Clone)]
pub enum RemotePackageAuth {
    Unauthorized,
}

/// The basic source for a package
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
