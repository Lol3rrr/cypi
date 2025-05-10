use std::collections::HashMap;

use async_session::SessionStore;
use axum::extract::FromRef;
use axum_extra::headers::HeaderMapExt;
use tracing::Instrument;

#[derive(Debug)]
pub enum CustomAuth {
    Customer { name: String },
    Developer,
}

#[derive(Debug)]
pub struct AuthState {
    pub customers: HashMap<String, String>,
}

impl<S> axum::extract::FromRequestParts<S> for CustomAuth
where
    S: AsRef<AuthState> + Sync,
    async_session::MemoryStore: axum::extract::FromRef<S>,
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
                let auth: &AuthState = state.as_ref();

                if let Some(customer_password) = auth.customers.get(h.username()) {
                    if customer_password == h.password() {
                        return Ok(Self::Customer { name: h.username().to_string() });
                    }
                }
            }

            if let Some(h) = header.typed_get::<axum_extra::headers::Cookie>() {
                if let Some(session) = h.get("SESSION") {
                    let store = async_session::MemoryStore::from_ref(state);

                    let session = store.load_session(session.to_string()).await.unwrap();

                    if let Some(_) = session {
                        return Ok(Self::Developer);
                    }
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
