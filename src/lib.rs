use std::collections::{HashMap, HashSet};

pub mod api;
pub mod auth;

#[derive(Debug, serde::Deserialize)]
pub struct CustomerConfig {
    #[serde(flatten)]
    pub customers: HashMap<String, ConfigCustomer>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConfigCustomer {
    pub packages: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PackageConfig {
    #[serde(flatten)]
    pub packages: HashMap<String, toml::Value>,
}

pub struct Package {
    pub files: Vec<String>,
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

#[derive(Clone)]
pub struct AxumState(pub std::sync::Arc<tokio::sync::RwLock<State>>);

pub fn customer_updates(state: std::sync::Arc<tokio::sync::RwLock<State>>) {
    loop {
        let customer_config = load_customers("./customers.toml");
        {
            let mut state = state.blocking_write();
            for (cname, customer_entry) in customer_config.customers {
                let new_value = customer_entry.packages.into_iter().collect();
                state.customer_packages.insert(cname, new_value);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(15));
    }
}

pub fn package_updates(state: std::sync::Arc<tokio::sync::RwLock<State>>) {
    loop {
        let package_config = load_packages("./packages.toml");
        {
            let mut state = state.blocking_write();
            for (pname, _package) in package_config.packages {
                let new_value = Package { files: Vec::new() };
                state.packages.insert(pname, new_value);

                // TODO
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(15));
    }
}

fn load_customers(path: impl AsRef<std::path::Path>) -> CustomerConfig {
    let content = std::fs::read_to_string(path).unwrap();
    toml::from_str(content.as_str()).unwrap()
}
fn load_packages(path: impl AsRef<std::path::Path>) -> PackageConfig {
    let content = std::fs::read_to_string(path).unwrap();
    toml::from_str(content.as_str()).unwrap()
}
