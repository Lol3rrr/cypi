use std::collections::HashMap;

use crate::auth::AuthState;

use super::NotificationReceiver;

#[derive(Debug, serde::Deserialize)]
struct Response<T> {
    data: T,
}

#[derive(Debug, serde::Deserialize)]
struct ListResponse {
    keys: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct DataResponse<T> {
    data: T,
}

#[tracing::instrument(skip(recv))]
pub fn customer_auth_updates(
    auth_state: AuthState,
    mut recv: NotificationReceiver,
    vault_url: reqwest::Url,
) {
    let http_client = reqwest::blocking::Client::new();

    let vault_secret_mount = "secret";
    let vault_secret_path = "customers";
    let vault_token = std::env::var("VAULT_TOKEN").unwrap();

    loop {
        if let Err(e) = recv.listen() {
            tracing::error!(?e, "NotificationReceiver is broken");
            return;
        }

        tracing::trace!("Reloading Customer Authentication configuration");
        
        let new_customers = match load_customers(&http_client, &vault_url, &vault_token, &vault_secret_mount, &vault_secret_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(?e, "Loading Customers from vault");
                continue;
            }
        };

        {
            let mut state = auth_state.customers.blocking_write();
            *state = new_customers;
        }
    }
}

fn load_customers(
    http_client: &reqwest::blocking::Client,
    vault_url: &reqwest::Url,
    vault_token: &str,
    secret_mount: &str,
    secret_path: &str,
) -> Result<HashMap<String, String>, ()> {
    let target_url = vault_url.join(&format!("/v1/{secret_mount}/metadata/{secret_path}")).map_err(|e| ())?;
    tracing::debug!(?target_url, "");

    let list_method = reqwest::Method::from_bytes(b"LIST").map_err(|e| ())?;
    let response = http_client.request(list_method, target_url).bearer_auth(&vault_token).send().map_err(|e| ())?;

    let content = response.json::<Response<ListResponse>>().map_err(|e| ())?;

    let mut result = HashMap::new();
    for entry in content.data.keys {
        match load_customer(http_client, vault_url, vault_token, &format!("{secret_path}/{entry}")) {
            Ok(cdata) => {
                result.insert(cdata.username, cdata.password);
            }
            Err(e) => {
                tracing::error!(?e, "Loading Customer Data from Vault");
            }
        };
    }

    Ok(result)
}

#[derive(Debug, serde::Deserialize)]
struct CustomerData {
    username: String,
    password: String,
}

fn load_customer(http_client: &reqwest::blocking::Client, vault_url: &reqwest::Url, vault_token: &str, path: &str) -> Result<CustomerData, ()> {
    let secret_mount = "secret";
        
    let target_url = vault_url.join(&format!("/v1/{secret_mount}/data/{path}")).map_err(|e| ())?;
    tracing::debug!(?target_url, "");

    let response = http_client.get(target_url).bearer_auth(&vault_token).send().map_err(|e| ())?;

    let content = response.json::<Response<DataResponse<CustomerData>>>().map_err(|e| ())?;
    
    Ok(content.data.data)
}
