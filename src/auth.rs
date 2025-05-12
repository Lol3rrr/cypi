use std::collections::HashMap;

use tower_sessions::SessionStore;
use axum::{extract::FromRef, response::sse};
use axum_extra::headers::HeaderMapExt;
use tracing::Instrument;

#[derive(Debug)]
pub enum CustomAuth {
    Customer { name: String },
    Developer,
}

#[derive(Debug, Clone)]
pub struct AuthState {
    pub customers: std::sync::Arc<tokio::sync::RwLock<HashMap<String, String>>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            customers: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new()))
        }
    }
}

impl<S> axum::extract::FromRequestParts<S> for CustomAuth
where
    AuthState: axum::extract::FromRef<S>,
    S: Sync + Send,
{
    type Rejection = axum::response::Response;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            tracing::trace!("Extractor");

            let header = &parts.headers;

            if let Some(h) = header.typed_get::<axum_extra::headers::Authorization<axum_extra::headers::authorization::Basic>>() {
                let auth: AuthState = AuthState::from_ref(state);
                let customers = auth.customers.read().await;

                if let Some(customer_password) = customers.get(h.username()) {
                    if customer_password == h.password() {
                        return Ok(Self::Customer { name: h.username().to_string() });
                    }
                }
            }

            if let Ok(session) = tower_sessions::Session::from_request_parts(parts, state).await {
                if session.get::<String>("gitlab-username").await.map(|v| v.is_some()).unwrap_or(false) {
                    return Ok(Self::Developer);
                }
            }

            Err(axum::response::Response::builder()
                .status(401)
                .body(axum::body::Body::empty())
                .unwrap())
        }
        .instrument(tracing::trace_span!("CustomAuth-Extractor"))
    }
}
