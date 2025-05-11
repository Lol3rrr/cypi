use tracing_subscriber::layer::SubscriberExt;

use cypi::{State, api::AxumState};

fn main() {
    let registry = tracing_subscriber::Registry::default().with(tracing_subscriber::fmt::layer());
    tracing::subscriber::set_global_default(registry).unwrap();

    tracing::info!("Starting...");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let state = std::sync::Arc::new(tokio::sync::RwLock::new(State::new()));

    let axum_state = AxumState {
        state: state.clone(),
        auth_state: std::sync::Arc::new(cypi::auth::AuthState {
            customers: [
                ("TODO".into(), "password".into()),
                ("second".into(), "password2".into()),
                ("third".into(), "password3".into()),
            ]
            .into_iter()
            .collect(),
        }),
        client: cypi::api::oauth_client().unwrap(),
        session_store: async_session::MemoryStore::new(),
    };
    let handle = rt.spawn(async move {
        let router = cypi::api::api_router(axum_state);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();
        axum::serve(listener, router).await.unwrap();
    });

    let (customer_notifier, customer_recv) = cypi::background::notifier();
    let customer_handle = rt.spawn_blocking({
        let state = state.clone();
        move || cypi::background::customers::customer_updates(state, customer_recv)
    });
    rt.spawn(async move {
        loop {
            if let Err(e) = customer_notifier.notify() {
                tracing::error!(?e, "Could not notify customer reload");
                return;
            }

            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });

    let (package_notifier, package_recv) = cypi::background::notifier();
    let packages_handle = rt.spawn_blocking({
        let state = state.clone();
        move || cypi::background::packages::package_updates(state, package_recv)
    });
    rt.spawn(async move {
        loop {
            if let Err(e) = package_notifier.notify() {
                tracing::error!(?e, "Could not notify package reload");
                return;
            }

            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });

    let _ = rt.block_on(handle);
    let _ = rt.block_on(customer_handle);
    let _ = rt.block_on(packages_handle);
}
