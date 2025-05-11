use crate::auth::CustomAuth;

use super::AxumState;

pub fn index_router(state: AxumState) -> axum::Router<AxumState> {
    axum::Router::<AxumState>::new()
        .route("/simple/", axum::routing::get(package_index))
        .route("/simple/{package}/", axum::routing::get(package_files))
        .route(
            "/simple/{package}/{file}",
            axum::routing::get(download_file),
        )
        .layer(axum::middleware::from_fn_with_state(
            state,
            load_user_packages,
        ))
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
            .map(|f| {
                let name = match f {
                    crate::PackageFile::RemotePackage { name, .. } => name,
                    crate::PackageFile::FilePackage { name, .. } => name,
                };
                format!("<a href=\"{}\">{}</a><br/>", name, name)
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
        crate::PackageFile::FilePackage { name, .. } => name == &filename,
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
            axum::response::Response::builder()
                .status(200)
                .body(axum::body::Body::from_stream(response.bytes_stream()))
                .unwrap()
        }
        Some(crate::PackageFile::FilePackage { path, .. }) => {
            tracing::trace!("Found FIle Package");

            let file = tokio::fs::File::open(path).await.unwrap();

            axum::response::Response::builder()
                .status(200)
                .body(axum::body::Body::from_stream(
                    tokio_util::io::ReaderStream::new(file),
                ))
                .unwrap()
        }
        None => axum::response::Response::builder()
            .status(404)
            .body(axum::body::Body::empty())
            .unwrap(),
    }
}
