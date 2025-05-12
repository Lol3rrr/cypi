use crate::{State, config};

use super::NotificationReceiver;

#[tracing::instrument(skip(state, recv, config_path))]
pub fn customer_updates(
    state: std::sync::Arc<tokio::sync::RwLock<State>>,
    mut recv: NotificationReceiver,
    config_path: impl Into<std::path::PathBuf>
) {
    let config_path = config_path.into();

    loop {
        if let Err(e) = recv.listen() {
            tracing::error!("NotificationReceiver is broken");
            return;
        }

        tracing::trace!("Reloading Customer configuration");

        let customer_config = match load_customers(&config_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(?e, "Loading Customer Config");
                continue;
            }
        };
        {
            let mut state = state.blocking_write();
            for (cname, customer_entry) in customer_config.customers {
                let new_value = customer_entry.packages.into_iter().collect();
                state.customer_packages.insert(cname, new_value);
            }
        }
    }
}

fn load_customers(path: impl AsRef<std::path::Path>) -> Result<config::CustomerConfig, ()> {
    let content = std::fs::read_to_string(path).map_err(|e| ())?;
    toml::from_str(content.as_str()).map_err(|e| ())
}
