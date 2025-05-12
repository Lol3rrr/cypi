use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;

use cypi::{State, api::AxumState, CliArgs};

fn main() {
    let args = CliArgs::parse();

    let registry = tracing_subscriber::Registry::default().with(tracing_subscriber::fmt::layer());
    tracing::subscriber::set_global_default(registry).unwrap();

    tracing::info!("Starting...");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let state = std::sync::Arc::new(tokio::sync::RwLock::new(State::new()));
    let auth_state = cypi::auth::AuthState::new();

    let axum_state = AxumState {
        state: state.clone(),
        auth_state: auth_state.clone(),
        client: cypi::api::gitlab_oauth_client().unwrap(),
    };

    // Spawn the API in its own task
    let handle = rt.spawn(async move {
        let sqlite_pool = tower_sessions_sqlx_store::sqlx::SqlitePool::connect(&args.sqlite_url).await.unwrap();
        let store = tower_sessions_sqlx_store::SqliteStore::new(sqlite_pool);
        store.migrate().await.unwrap();

        let router = cypi::api::api_router(axum_state, store);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();
        axum::serve(listener, router).await.unwrap();
    });

    // Customer auth config related stuff
    let (customer_auth_notifier, customer_auth_recv) = cypi::background::notifier();
    rt.spawn_blocking({
        move || cypi::background::customer_auth::customer_auth_updates(auth_state, customer_auth_recv, reqwest::Url::parse("http://127.0.0.1:8200").unwrap())
    });
    rt.spawn(async move {
        loop {
            if let Err(e) = customer_auth_notifier.notify() {
                tracing::error!(?e, "Could not notify customer auth reload");
                return;
            }

            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });

    // All the customer config related stuff
    let (customer_notifier, customer_recv) = cypi::background::notifier();
    let customer_handle = rt.spawn_blocking({
        let state = state.clone();
        let config_path = args.customer_config;
        move || cypi::background::customers::customer_updates(state, customer_recv, config_path)
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

    // All the package config related stuff
    let (package_notifier, package_recv) = cypi::background::notifier();
    let packages_handle = rt.spawn_blocking({
        let state = state.clone();
        let config_path = args.package_config;
        move || cypi::background::packages::package_updates(state, package_recv, config_path)
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
