use crate::{CustomerConfig, State};

use super::NotificationReceiver;

#[tracing::instrument(skip(state, recv))]
pub fn customer_updates(
    state: std::sync::Arc<tokio::sync::RwLock<State>>,
    mut recv: NotificationReceiver,
) {
    loop {
        if let Err(e) = recv.listen() {
            tracing::error!("NotificationReceiver is broken");
            return;
        }

        tracing::trace!("Reloading Customer configuration");

        let customer_config = load_customers("./customers.toml");
        {
            let mut state = state.blocking_write();
            for (cname, customer_entry) in customer_config.customers {
                let new_value = customer_entry.packages.into_iter().collect();
                state.customer_packages.insert(cname, new_value);
            }
        }
    }
}

fn load_customers(path: impl AsRef<std::path::Path>) -> CustomerConfig {
    let content = std::fs::read_to_string(path).unwrap();
    toml::from_str(content.as_str()).unwrap()
}
