use crate::{AxumState, auth::CustomAuth};

pub async fn run_api(state: AxumState) {
    let app = axum::Router::new()
        .route("/", axum::routing::get(landing_page))
        .route("/simple/", axum::routing::get(package_index))
        .route("/simple/{package}/", axum::routing::get(package_files))
        .route(
            "/simple/{package}/{file}",
            axum::routing::get(download_file),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn landing_page(
    auth: Result<CustomAuth, axum::response::Response>,
) -> axum::response::Response<String> {
    tracing::debug!("Landing Page");

    let account = match auth {
        Ok(account) => account,
        Err(_) => {
            return axum::response::Response::builder()
                .status(401)
                .header("WWW-Authenticate", "Basic realm = \"Testing\"")
                .body(String::new())
                .unwrap();
        }
    };

    tracing::debug!(?account, "Already logged in");

    match account {
        CustomAuth::Customer { name } => {
            axum::response::Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(format!("<html><body><h1>Customer Portal</h1><p>{}</p><a href=\"/simple/\">Simple Index</a></body></html>", name))
                .unwrap()
        }
        CustomAuth::Developer => {
            axum::response::Response::builder().status(200).header("Content-Type", "text/html").body("<html><body><h1>Developer Portal</h1><p>Developer</p><a href=\"/simple/\">Simple Index</a></body></html>".into()).unwrap()
        }
    }
}

async fn package_index(
    authed: CustomAuth,
    axum::extract::State(state): axum::extract::State<AxumState>,
) -> axum::response::Html<String> {
    tracing::debug!(?authed, "Simple Index");

    let state = state.0.read().await;

    let all_packages = state.packages.keys();

    let packages: Vec<String> = match authed {
        CustomAuth::Customer { name } => match state.customer_packages.get(&name) {
            Some(customer_packages) => all_packages
                .filter(|p| customer_packages.contains(*p))
                .cloned()
                .collect(),
            None => Vec::new(),
        },
        CustomAuth::Developer => all_packages.cloned().collect(),
    };

    let response = format!(
        "<html><body>{}</body></html>",
        packages
            .iter()
            .map(|p| format!("<a href=\"{}/\">{}</a>", p, p))
            .collect::<String>()
    );

    axum::response::Html(response)
}

async fn package_files(
    axum::extract::Path(package): axum::extract::Path<String>,
    authed: CustomAuth,
    axum::extract::State(state): axum::extract::State<AxumState>,
) -> axum::response::Response<String> {
    tracing::debug!(?package, "Files for package");

    let state = state.0.read().await;

    match authed {
        CustomAuth::Customer { name } => {
            let customer_state = match state.customer_packages.get(&name) {
                Some(s) => s,
                None => {
                    return axum::response::Response::builder()
                        .status(404)
                        .body("".into())
                        .unwrap();
                }
            };

            if !customer_state.contains(&package) {
                return axum::response::Response::builder()
                    .status(404)
                    .body("".into())
                    .unwrap();
            }
        }
        CustomAuth::Developer => {}
    };

    axum::response::Response::new("Files for a packages".into())
}

async fn download_file(
    axum::extract::Path((package, filename)): axum::extract::Path<(String, String)>,
) -> String {
    tracing::debug!(?package, ?filename, "Download file for package");

    "TODO".into()
}
