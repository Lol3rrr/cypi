use async_session::SessionStore;
use oauth2::{CsrfToken, TokenResponse};

use super::{AxumState, COOKIE_NAME, CSRF_TOKEN, Oauth2Client};

pub fn auth_router() -> axum::Router<AxumState> {
    axum::Router::new()
        .route("/auth/discord", axum::routing::get(auth_discord))
        .route("/auth/authorized", axum::routing::get(login_authorized))
}

async fn auth_discord(
    axum::extract::State(client): axum::extract::State<Oauth2Client>,
    axum::extract::State(store): axum::extract::State<async_session::MemoryStore>,
) -> impl axum::response::IntoResponse {
    let (auth_url, csrf_token) = client
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("identify".to_string()))
        .url();

    let mut session = async_session::Session::new();
    session.insert(CSRF_TOKEN, &csrf_token).unwrap();

    let cookie = store.store_session(session).await.unwrap().unwrap();

    let cookie = format!("{COOKIE_NAME}={cookie}; SameSite=Lax; HttpOnly; Secure; Path=/");
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::SET_COOKIE, cookie.parse().unwrap());

    (headers, axum::response::Redirect::to(auth_url.as_ref()))
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct AuthRequest {
    code: String,
    state: String,
}

async fn csrf_token_validation_workflow(
    auth_request: &AuthRequest,
    cookies: &axum_extra::headers::Cookie,
    store: &async_session::MemoryStore,
) -> Result<(), ()> {
    // Extract the cookie from the request
    let cookie = cookies.get(COOKIE_NAME).ok_or(())?.to_string();

    // Load the session
    let session = match store.load_session(cookie).await.map_err(|e| ())? {
        Some(session) => session,
        None => return Err(()),
    };

    // Extract the CSRF token from the session
    let stored_csrf_token = session.get::<CsrfToken>(CSRF_TOKEN).ok_or(())?.to_owned();

    // Cleanup the CSRF token session
    store.destroy_session(session).await.map_err(|e| ())?;

    // Validate CSRF token is the same as the one in the auth request
    if *stored_csrf_token.secret() != auth_request.state {
        return Err(());
    }

    Ok(())
}

// The user data we'll get back from Discord.
// https://discord.com/developers/docs/resources/user#user-object-user-structure
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct User {
    id: String,
    avatar: Option<String>,
    username: String,
    discriminator: String,
}

async fn login_authorized(
    axum::extract::Query(query): axum::extract::Query<AuthRequest>,
    axum::extract::State(oauth_client): axum::extract::State<Oauth2Client>,
    axum::extract::State(store): axum::extract::State<async_session::MemoryStore>,
    axum_extra::TypedHeader(cookies): axum_extra::TypedHeader<axum_extra::headers::Cookie>,
) -> impl axum::response::IntoResponse {
    csrf_token_validation_workflow(&query, &cookies, &store)
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
    let user_data: User = client
        // https://discord.com/developers/docs/resources/user#get-current-user
        .get("https://discordapp.com/api/users/@me")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .unwrap()
        .json::<User>()
        .await
        .unwrap();

    // Create a new session filled with user data
    let mut session = async_session::Session::new();
    session.insert("user", &user_data).unwrap();

    // Store session and get corresponding cookie
    let cookie = store.store_session(session).await.unwrap().unwrap();

    // Build the cookie
    let cookie = format!("{COOKIE_NAME}={cookie}; SameSite=Lax; HttpOnly; Secure; Path=/");

    // Set cookie
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::SET_COOKIE, cookie.parse().unwrap());

    (headers, axum::response::Redirect::to("/"))
}
