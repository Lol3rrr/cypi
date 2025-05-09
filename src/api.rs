use crate::{AxumState, auth::CustomAuth};

pub async fn run_api(state: AxumState) {
    let index_router = axum::Router::<AxumState>::new()
        .route("/simple/", axum::routing::get(package_index))
        .route("/simple/{package}/", axum::routing::get(package_files))
        .route(
            "/simple/{package}/{file}",
            axum::routing::get(download_file),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            load_user_packages,
        ));

    let app = axum::Router::new()
        .route("/", axum::routing::get(landing_page))
        .merge(index_router)
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
                .body(format!("<html><body><h1>Customer Portal</h1><p>Logged in as '{}'</p><a href=\"/simple/\">Simple Index</a></body></html>", name))
                .unwrap()
        }
        CustomAuth::Developer => {
            axum::response::Response::builder().status(200).header("Content-Type", "text/html").body("<html><body><h1>Developer Portal</h1><p>Developer</p><a href=\"/simple/\">Simple Index</a></body></html>".into()).unwrap()
        }
    }
}

#[derive(Debug, Clone)]
struct UserPackages(pub Vec<String>);

async fn load_user_packages(
    authed: CustomAuth,
    axum::extract::State(state): axum::extract::State<AxumState>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    tracing::trace!(?authed, "Loading Packages for User");

    let packages: Vec<String> = {
        let state = state.state.read().await;

        let all_packages = state.packages.keys();

        match authed {
            CustomAuth::Customer { name } => match state.customer_packages.get(&name) {
                Some(customer_packages) => all_packages
                    .filter(|p| customer_packages.contains(*p))
                    .cloned()
                    .collect(),
                None => Vec::new(),
            },
            CustomAuth::Developer => all_packages.cloned().collect(),
        }
    };

    request.extensions_mut().insert(UserPackages(packages));

    next.run(request).await
}

#[tracing::instrument(skip(packages))]
async fn package_index(
    axum::extract::Extension(packages): axum::extract::Extension<UserPackages>,
) -> axum::response::Html<String> {
    tracing::debug!("Simple Index");

    let response = format!(
        "<html><body>{}</body></html>",
        packages
            .0
            .iter()
            .map(|p| format!("<a href=\"{}/\">{}</a><br/>", p, p))
            .collect::<String>()
    );

    axum::response::Html(response)
}

#[tracing::instrument(skip(packages, state))]
async fn package_files(
    axum::extract::Path(package): axum::extract::Path<String>,
    axum::extract::Extension(packages): axum::extract::Extension<UserPackages>,
    axum::extract::State(state): axum::extract::State<AxumState>,
) -> axum::response::Response<String> {
    tracing::debug!("Files for package");

    // Check if the user has the package configured
    if !packages.0.iter().any(|p| p == &package) {
        tracing::error!("Unknown file request for user");

        return axum::response::Response::builder()
            .status(404)
            .body("".into())
            .unwrap();
    }

    let state = state.state.read().await;

    let files = match state.packages.get(&package) {
        Some(package) => {
            tracing::debug!("Package found");
            package.files.clone()
        }
        None => {
            tracing::error!("Unknown Package");
            Vec::new()
        }
    };

    let response_content = format!(
        "<html><body>{}</body></html>",
        files
            .into_iter()
            .map(|f| match f {
                crate::PackageFile::RemotePackage { name, .. } =>
                    format!("<a href=\"{}\">{}</a><br/>", name, name),
            })
            .collect::<String>()
    );

    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(response_content)
        .unwrap()
}

#[tracing::instrument(skip(packages, state))]
async fn download_file(
    axum::extract::Path((package, filename)): axum::extract::Path<(String, String)>,
    axum::extract::Extension(packages): axum::extract::Extension<UserPackages>,
    axum::extract::State(state): axum::extract::State<AxumState>,
) -> axum::http::Response<axum::body::Body> {
    tracing::debug!(?package, ?filename, "Download file for package");

    if !packages.0.iter().any(|p| p == &package) {
        tracing::error!("Unknown file request for user");

        return axum::response::Response::builder()
            .status(404)
            .body(axum::body::Body::empty())
            .unwrap();
    }

    let state = state.state.read().await;

    let test = match state.packages.get(&package) {
        Some(v) => v,
        None => {
            tracing::error!("Unknown file request for user");

            return axum::response::Response::builder()
                .status(404)
                .body(axum::body::Body::empty())
                .unwrap();
        }
    };

    let file = test.files.iter().find(|f| match f {
        crate::PackageFile::RemotePackage { name, .. } => name == &filename,
    });

    match file {
        Some(crate::PackageFile::RemotePackage { url, auth, .. }) => {
            tracing::trace!("Found Remote Package");

            let client = reqwest::Client::new();

            let req = client.get(url.clone());
            let req = match auth {
                crate::RemotePackageAuth::Unauthorized => req,
            };

            let response = req.send().await.unwrap();
            return axum::response::Response::builder()
                .status(200)
                .body(axum::body::Body::from_stream(response.bytes_stream()))
                .unwrap();
        }
        None => axum::response::Response::builder()
            .status(404)
            .body(axum::body::Body::empty())
            .unwrap(),
    }
}
