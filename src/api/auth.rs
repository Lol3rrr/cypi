use oauth2::{CsrfToken, TokenResponse};

use super::{AxumState,CSRF_TOKEN, Oauth2Client};

pub fn auth_router() -> axum::Router<AxumState> {
    axum::Router::new()
        .route("/auth/gitlab", axum::routing::get(auth_discord))
        .route("/auth/authorized", axum::routing::get(login_authorized))
}

async fn auth_discord(
    axum::extract::State(client): axum::extract::State<Oauth2Client>,
    session: tower_sessions::Session,
) -> impl axum::response::IntoResponse {
    let (auth_url, csrf_token) = client
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("read_user".to_string()))
        .url();

    session.insert(CSRF_TOKEN, &csrf_token).await.unwrap();
    session.save().await.unwrap();

    axum::response::Redirect::to(auth_url.as_ref())
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct AuthRequest {
    code: String,
    state: String,
}

async fn csrf_token_validation_workflow(
    auth_request: &AuthRequest,
    session: &mut tower_sessions::Session,
) -> Result<(), ()> {
    // Extract the CSRF token from the session
    let stored_csrf_token = session.get::<CsrfToken>(CSRF_TOKEN).await.map_err(|e| ())?.ok_or(())?.to_owned(); 

    // Cleanup the CSRF token session
    session.remove::<CsrfToken>(CSRF_TOKEN).await.map_err(|e| ())?;
    session.save().await.map_err(|e| ())?;

    // Validate CSRF token is the same as the one in the auth request
    if *stored_csrf_token.secret() != auth_request.state {
        return Err(());
    }

    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GitlabUser {
    username: String,
    email: String,
    name: String,
}

async fn login_authorized(
    axum::extract::Query(query): axum::extract::Query<AuthRequest>,
    axum::extract::State(oauth_client): axum::extract::State<Oauth2Client>,
    mut session: tower_sessions::Session,
) -> impl axum::response::IntoResponse {
    csrf_token_validation_workflow(&query, &mut session)
        .await
        .unwrap();

    // Get an auth token
    let token = oauth_client
        .exchange_code(oauth2::AuthorizationCode::new(query.code.clone()))
        .request_async(&oauth2::reqwest::Client::new())
        .await
        .unwrap();

    // Fetch user data from discord
    let client = reqwest::Client::new();
    let user_data: GitlabUser = client
        .get("https://gitlab.com/api/v4/user")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .unwrap()
        .json::<GitlabUser>()
        .await
        .unwrap();

    // Create a new session filled with user data
    session.insert("gitlab-username", user_data.username).await.unwrap();
    session.save().await.unwrap();

    // Store session and get corresponding cookie
    axum::response::Redirect::to("/")
}
