use tracing_subscriber::layer::SubscriberExt;

use cypi::{AxumState, State};

fn main() {
    let registry = tracing_subscriber::Registry::default().with(tracing_subscriber::fmt::layer());
    tracing::subscriber::set_global_default(registry).unwrap();

    tracing::info!("Starting...");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let state = std::sync::Arc::new(tokio::sync::RwLock::new(State::new()));

    let handle = rt.spawn(cypi::api::run_api(AxumState {
        state: state.clone(),
        auth_state: std::sync::Arc::new(cypi::auth::AuthState {
            customers: [("TODO".into(), "password".into()), ("second".into(), "password2".into()), ("third".into(), "password3".into())].into_iter().collect()
        }),
    }));

    let customer_handle = rt.spawn_blocking({
        let state = state.clone();
        move || cypi::background::customers::customer_updates(state)
    });

    let packages_handle = rt.spawn_blocking({
        let state = state.clone();
        move || cypi::background::packages::package_updates(state)
    });

    let _ = rt.block_on(handle);
    let _ = rt.block_on(customer_handle);
    let _ = rt.block_on(packages_handle);
}
