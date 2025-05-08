use tracing::Instrument;

#[derive(Debug)]
pub enum CustomAuth {
    Customer { name: String },
    Developer,
}

impl<S> axum::extract::FromRequestParts<S> for CustomAuth {
    type Rejection = axum::response::Response;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            tracing::trace!("Extractor");

            let header = &parts.headers;

            let auth_header = match header.get("Authorization") {
                Some(h) => h,
                None => {
                    return Err(axum::response::Response::builder()
                        .status(401)
                        .body(axum::body::Body::empty())
                        .unwrap());
                }
            };

            tracing::trace!(?auth_header, "Has Authorization header");

            let auth_str = match auth_header.to_str() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(?e, "Converting auth header to string");
                    return Err(axum::response::Response::builder()
                        .status(401)
                        .body(axum::body::Body::empty())
                        .unwrap());
                }
            };

            tracing::trace!(?auth_str, "Has Auth String");

            let (kind, content) = match auth_str.split_once(' ') {
                Some(v) => v,
                None => {
                    tracing::error!(?auth_str, "Malformed Authorization Header");
                    return Err(axum::response::Response::builder()
                        .status(401)
                        .body(axum::body::Body::empty())
                        .unwrap());
                }
            };

            match kind {
                "Basic" => {
                    tracing::trace!(?content, "Basic Auth");

                    // How do authenticate a Basic Auth user

                    Ok(CustomAuth::Customer {
                        name: "TODO".into(),
                    })
                }
                other => {
                    tracing::error!(?other, "Unsupported Authorization Type");
                    return Err(axum::response::Response::builder()
                        .status(401)
                        .body(axum::body::Body::empty())
                        .unwrap());
                }
            }
        }
        .instrument(tracing::trace_span!("CustomAuth-Extractor"))
    }
}
