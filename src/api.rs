//! The API specifics

use crate::auth::CustomAuth;

type Oauth2Client = oauth2::basic::BasicClient<
    oauth2::EndpointSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;

mod auth;
mod index;

static COOKIE_NAME: &str = "SESSION";
static CSRF_TOKEN: &str = "csrf_token";

/// Combines the different states needed for the API to work
#[derive(Clone)]
pub struct AxumState {
    pub state: std::sync::Arc<tokio::sync::RwLock<crate::State>>,
    pub auth_state: std::sync::Arc<crate::auth::AuthState>,
    pub client: Oauth2Client,
    pub session_store: async_session::MemoryStore,
}

impl AsRef<crate::auth::AuthState> for AxumState {
    fn as_ref(&self) -> &crate::auth::AuthState {
        &self.auth_state
    }
}
impl axum::extract::FromRef<AxumState> for Oauth2Client {
    fn from_ref(input: &AxumState) -> Self {
        input.client.clone()
    }
}
impl axum::extract::FromRef<AxumState> for async_session::MemoryStore {
    fn from_ref(input: &AxumState) -> Self {
        input.session_store.clone()
    }
}

/// Setup the oauth client for Gitlab
pub fn oauth_client() -> Result<Oauth2Client, ()> {
    let client_id = std::env::var("CLIENT_ID").map_err(|e| ())?;
    let client_secret = std::env::var("CLIENT_SECRET").map_err(|e| ())?;
    let redirect_url = std::env::var("REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:3030/auth/authorized".to_string());

    let auth_url = std::env::var("AUTH_URL")
        .unwrap_or_else(|_| "https://gitlab.com/oauth/authorize?response_type=code".to_string());

    let token_url =
        std::env::var("TOKEN_URL").unwrap_or_else(|_| "https://gitlab.com/oauth/token".to_string());

    Ok(
        oauth2::basic::BasicClient::new(oauth2::ClientId::new(client_id))
            .set_client_secret(oauth2::ClientSecret::new(client_secret))
            .set_auth_uri(oauth2::AuthUrl::new(auth_url).map_err(|e| ())?)
            .set_token_uri(oauth2::TokenUrl::new(token_url).map_err(|e| ())?)
            .set_redirect_uri(oauth2::RedirectUrl::new(redirect_url).map_err(|e| ())?),
    )
}

/// Setup the entire Axum Router to handle the api
pub fn api_router(state: AxumState) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(landing_page))
        .merge(auth::auth_router())
        .merge(index::index_router(state.clone()))
        .with_state(state)
}

async fn landing_page(
    auth: Result<CustomAuth, axum::response::Response>,
) -> axum::response::Response<String> {
    let account = match auth {
        Ok(account) => account,
        Err(_) => {
            return axum::response::Response::builder()
                .status(401)
                .header("WWW-Authenticate", "Basic realm = \"Testing\"")
                .body(
                    "<html><body><a href=\"/auth/gitlab\">Login with Gitlab</a></body></html>"
                        .into(),
                )
                .unwrap();
        }
    };

    tracing::debug!(?account, "Logged in");

    match account {
        CustomAuth::Customer { name } => {
            axum::response::Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(format!("<html><body><h1>Customer Portal</h1><p>Logged in as '{}'</p><a href=\"/simple/\">Simple Index</a></body></html>", name))
                .unwrap()
        }
        CustomAuth::Developer => {
            axum::response::Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body("<html><body><h1>Developer Portal</h1><p>Developer</p><a href=\"/simple/\">Simple Index</a></body></html>".into())
                .unwrap()
        }
    }
}
