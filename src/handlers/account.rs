use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Form, Json};
use axum_extra::extract::SignedCookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{self, AuthUser, MaybeUser};
use crate::error::AppResult;
use crate::handlers::PageQuery;
use crate::models;
use crate::state::AppState;
use crate::util::{self, Page};
use crate::views::render;

/// `?error=<code>` on redirects back to a form, rendered as an alert banner.
#[derive(Deserialize, Default)]
pub struct FlashQuery {
    pub error: Option<String>,
}

fn flash_message(code: &Option<String>) -> Option<&'static str> {
    Some(match code.as_deref()? {
        "username_taken" => "That username is already taken.",
        "invalid_username" => {
            "Usernames may contain letters, numbers, '.', '-' and '_' (max 39 characters)."
        }
        "invalid_credentials" => "Invalid username or password.",
        "missing_fields" => "Please fill in both username and password.",
        "weak_password" => "Password must be at least 8 characters.",
        "too_many_attempts" => "Too many sign-in attempts. Please try again later.",
        "signup_closed" => "Sign-up is closed. Ask an administrator for an account.",
        "invalid_signup_code" => "Invalid invitation code.",
        "oauth_failed" => "Google sign-in failed. Please try again.",
        "domain_not_allowed" => "This email domain is not allowed.",
        "wrong_password" => "Current password is incorrect.",
        "password_mismatch" => "New passwords do not match.",
        "invalid_icon_url" => "Icon URL must be an http(s) URL or an uploaded image.",
        _ => return None,
    })
}

pub async fn get_signup(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Query(q): Query<FlashQuery>,
) -> AppResult<Response> {
    if user.is_some() {
        return Ok(Redirect::to("/").into_response());
    }
    let data = json!({
        "google_enabled": state.config.google_configured(),
        "signup_enabled": state.config.team_signup_enabled,
        "signup_code_required": !state.config.team_signup_code.is_empty(),
        "error_message": flash_message(&q.error),
    });
    Ok(render(&state, "account/signup", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct SignupForm {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub signup_code: String,
}

pub async fn post_signup(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<SignupForm>,
) -> AppResult<Response> {
    if !state.config.team_signup_enabled {
        return Ok(Redirect::to("/signup?error=signup_closed").into_response());
    }
    let expected_code = state.config.team_signup_code.as_str();
    if !expected_code.is_empty() && !constant_time_eq(expected_code, form.signup_code.trim()) {
        return Ok(Redirect::to("/signup?error=invalid_signup_code").into_response());
    }

    let username = form.username.trim();
    if username.is_empty() || form.password.is_empty() {
        return Ok(Redirect::to("/signup?error=missing_fields").into_response());
    }
    if !util::username_valid(username) {
        return Ok(Redirect::to("/signup?error=invalid_username").into_response());
    }
    if !util::password_acceptable(&form.password) {
        return Ok(Redirect::to("/signup?error=weak_password").into_response());
    }

    let password = util::hash_password(form.password).await?;
    let user_id = match models::user::create(&state.pool, username, &password).await {
        Ok(id) => id,
        Err(e) if crate::error::is_unique_violation(&e) => {
            return Ok(Redirect::to("/signup?error=username_taken").into_response());
        }
        Err(e) => return Err(e.into()),
    };
    let jar = auth::sign_in(jar, user_id, state.config.secure_cookies());
    Ok((jar, Redirect::to("/")).into_response())
}

/// Compare two secrets without leaking their common prefix length.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= (x ^ y) as usize;
    }
    diff == 0
}

pub async fn get_signin(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Query(q): Query<FlashQuery>,
) -> AppResult<Response> {
    if user.is_some() {
        return Ok(Redirect::to("/").into_response());
    }
    let data = json!({
        "google_enabled": state.config.google_configured(),
        "signup_enabled": state.config.team_signup_enabled,
        "error_message": flash_message(&q.error),
    });
    Ok(render(&state, "account/signin", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct CredentialsForm {
    pub username: String,
    pub password: String,
}

pub async fn post_signin(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Form(form): Form<CredentialsForm>,
) -> AppResult<Response> {
    // Throttle guessing both per client address and per target account.
    let ip = util::client_ip(&headers, Some(peer), state.config.team_trust_proxy);
    let ip_key = format!("ip:{}", ip);
    let user_key = format!("user:{}", form.username.trim().to_ascii_lowercase());
    let limiter = &state.login_limiter;
    if limiter.is_blocked(&ip_key) || limiter.is_blocked(&user_key) {
        tracing::warn!(target: "team::auth", "sign-in throttled for {}", ip);
        return Ok(Redirect::to("/signin?error=too_many_attempts").into_response());
    }

    let user =
        models::user::get_with_password_by_username(&state.pool, form.username.trim()).await?;
    // Verify against a dummy hash when the user is unknown so the response
    // time does not reveal whether the username exists.
    let stored = if user.username.is_empty() || user.password.is_empty() {
        UNKNOWN_USER_HASH.to_string()
    } else {
        user.password.clone()
    };
    let verified = util::verify_password(form.password.clone(), stored).await;
    if user.username.is_empty() || user.password.is_empty() || !verified {
        limiter.record_failure(&ip_key);
        limiter.record_failure(&user_key);
        return Ok(Redirect::to("/signin?error=invalid_credentials").into_response());
    }
    limiter.clear(&ip_key);
    limiter.clear(&user_key);

    // Transparently upgrade accounts still on the legacy hash scheme.
    if util::password_needs_rehash(&user.password) {
        match util::hash_password(form.password).await {
            Ok(new_hash) => {
                if let Err(e) = models::user::update_password(&state.pool, user.id, &new_hash).await
                {
                    tracing::error!("password rehash failed: {:?}", e);
                }
            }
            Err(e) => tracing::error!("password rehash failed: {:?}", e),
        }
    }

    let jar = auth::sign_in(jar, user.id, state.config.secure_cookies());
    let (jar, ref_url) = auth::take_ref_url(jar);
    Ok((jar, Redirect::to(&ref_url)).into_response())
}

/// A valid Argon2id hash of a random string, used only to equalize timing
/// for sign-in attempts against nonexistent accounts.
const UNKNOWN_USER_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$Zd5nR3Oq2fQ4F2v9Zb2m4V5y4nN7u1oVqk0Y1xq1d1M";

pub async fn signout(jar: SignedCookieJar) -> Response {
    let jar = auth::sign_out(jar);
    (jar, Redirect::to("/signin")).into_response()
}

pub async fn get_settings(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<FlashQuery>,
) -> AppResult<Response> {
    let user = models::user::get_by_id(&state.pool, login_user.id).await?;

    #[derive(Serialize, Debug, Default)]
    struct Menu {
        name: String,
        checked: String,
    }

    let mut menu: Vec<Menu> = Vec::new();
    for name in state.config.team_menu.split(',') {
        menu.push(Menu {
            name: name.to_string(),
            checked: if login_user.menu.contains(&name.to_string()) {
                "checked".to_string()
            } else {
                String::new()
            },
        });
    }

    let theme_light = if login_user.theme == "light" {
        "checked"
    } else {
        ""
    };
    let theme_black = if login_user.theme == "black" {
        "checked"
    } else {
        ""
    };

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        user: models::user::User,
        login_user: models::user::UserWithPreference,
        menu: Vec<Menu>,
        theme_light: String,
        theme_black: String,
        error_message: Option<&'static str>,
    }

    let data = Data {
        logged_in: true,
        user,
        menu,
        theme_light: theme_light.to_string(),
        theme_black: theme_black.to_string(),
        login_user,
        error_message: flash_message(&q.error),
    };
    Ok(render(&state, "account/settings", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct SettingsForm {
    pub icon_url: String,
}

pub async fn post_settings(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<SettingsForm>,
) -> AppResult<Response> {
    let icon_url = form.icon_url.trim();
    if !util::icon_url_acceptable(icon_url) {
        return Ok(Redirect::to("/account/settings?error=invalid_icon_url").into_response());
    }
    models::user::update_icon_url(&state.pool, login_user.id, icon_url).await?;
    Ok(Redirect::to("/account/settings").into_response())
}

#[derive(Deserialize)]
pub struct PasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

pub async fn update_password(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<PasswordForm>,
) -> AppResult<Response> {
    if form.new_password != form.confirm_password {
        return Ok(Redirect::to("/account/settings?error=password_mismatch").into_response());
    }
    if !util::password_acceptable(&form.new_password) {
        return Ok(Redirect::to("/account/settings?error=weak_password").into_response());
    }
    let user = models::user::get_with_password_by_id(&state.pool, login_user.id).await?;
    // Google-only accounts have no password and cannot set one here.
    if user.password.is_empty()
        || !util::verify_password(form.current_password, user.password).await
    {
        return Ok(Redirect::to("/account/settings?error=wrong_password").into_response());
    }
    let new_hash = util::hash_password(form.new_password).await?;
    models::user::update_password(&state.pool, login_user.id, &new_hash).await?;
    Ok(Redirect::to("/account/settings").into_response())
}

#[derive(Deserialize)]
pub struct UsernameForm {
    pub username: String,
}

pub async fn update_username(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<UsernameForm>,
) -> AppResult<Response> {
    let username = form.username.trim();
    if !util::username_valid(username) {
        return Ok(Redirect::to("/account/settings?error=invalid_username").into_response());
    }
    match models::user::update_username(&state.pool, login_user.id, username).await {
        Ok(()) => Ok(Redirect::to("/account/settings").into_response()),
        Err(e) if crate::error::is_unique_violation(&e) => {
            Ok(Redirect::to("/account/settings?error=username_taken").into_response())
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(Deserialize)]
pub struct MenuForm {
    pub menu_param: String,
}

pub async fn preference_menu(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<MenuForm>,
) -> AppResult<Response> {
    models::user::set_preference_menu(&state.pool, login_user.id, &form.menu_param).await?;
    Ok(Redirect::to("/account/settings").into_response())
}

#[derive(Deserialize)]
pub struct ThemeForm {
    pub theme: String,
}

pub async fn preference_theme(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<ThemeForm>,
) -> AppResult<Response> {
    models::user::set_preference_theme(&state.pool, login_user.id, &form.theme).await?;
    Ok(Redirect::to("/account/settings").into_response())
}

async fn profile(
    state: AppState,
    login_user: models::user::UserWithPreference,
    username: String,
    kind: &str,
    page: Option<i32>,
) -> AppResult<Response> {
    let page = Page::new(page);

    let user = models::user::get_by_username(&state.pool, &username).await?;
    if user.id == 0 {
        return Err(crate::error::AppError::NotFound);
    }
    let mut posts: Vec<models::post::Post> = Vec::new();
    let mut gists: Vec<models::gist::Gist> = Vec::new();
    let mut tweets: Vec<models::tweet::Tweet> = Vec::new();
    let count = match kind {
        "gist" => {
            gists =
                models::gist::user_gists(&state.pool, &username, page.offset, page.limit).await?;
            models::gist::user_gists_count(&state.pool, &username).await?
        }
        "tweet" => {
            tweets = models::tweet::user_tweets(
                &state.pool,
                login_user.id,
                &username,
                page.offset,
                page.limit,
            )
            .await?;
            models::tweet::user_tweets_count(&state.pool, &username).await?
        }
        _ => {
            posts = models::post::user_posts(&state.pool, &username, page.offset, page.limit, kind)
                .await?;
            models::post::user_posts_count(&state.pool, &username, kind).await?
        }
    };

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        user: models::user::User,
        posts: Vec<models::post::Post>,
        gists: Vec<models::gist::Gist>,
        tweets: Vec<models::tweet::Tweet>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
        kind: String,
        kind_post_active: String,
        kind_nippo_active: String,
        kind_gist_active: String,
        kind_tweet_active: String,
    }

    let data = Data {
        logged_in: true,
        login_user,
        user,
        posts,
        gists,
        tweets,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
        kind: kind.to_string(),
        kind_post_active: if kind == "post" { "is-active" } else { "" }.to_string(),
        kind_nippo_active: if kind == "nippo" { "is-active" } else { "" }.to_string(),
        kind_gist_active: if kind == "gist" { "is-active" } else { "" }.to_string(),
        kind_tweet_active: if kind == "tweet" { "is-active" } else { "" }.to_string(),
    };
    Ok(render(&state, "account/profile", &data)?.into_response())
}

pub async fn profile_post(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(username): Path<String>,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    profile(state, login_user, username, "post", q.page).await
}

pub async fn profile_nippo(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(username): Path<String>,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    profile(state, login_user, username, "nippo", q.page).await
}

pub async fn profile_gist(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(username): Path<String>,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    profile(state, login_user, username, "gist", q.page).await
}

pub async fn profile_tweet(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(username): Path<String>,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    profile(state, login_user, username, "tweet", q.page).await
}

#[derive(Deserialize)]
pub struct GoogleAuthQuery {
    pub code: Option<String>,
    pub state: Option<String>,
}

pub async fn auth_google(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Query(q): Query<GoogleAuthQuery>,
) -> AppResult<Response> {
    if !state.config.google_configured() {
        return Ok(Redirect::to("/signin").into_response());
    }
    let secure = state.config.secure_cookies();
    let code = match q.code {
        Some(code) if !code.is_empty() => code,
        _ => {
            // Start the flow: issue a per-request CSRF state token.
            let (jar, token) = auth::set_oauth_state(jar, secure);
            let url = crate::services::google_authorize_url(&state, &token);
            return Ok((jar, Redirect::to(&url)).into_response());
        }
    };

    // Callback: the state must match the token we issued.
    let (jar, expected_state) = auth::take_oauth_state(jar);
    match (&expected_state, &q.state) {
        (Some(expected), Some(got)) if expected == got => {}
        _ => {
            tracing::warn!("google oauth: state mismatch");
            return Ok((jar, Redirect::to("/signin?error=oauth_failed")).into_response());
        }
    }

    let email = match crate::services::google_email_for_code(&state, &code).await {
        Ok(email) => email,
        Err(e) => {
            tracing::error!("google oauth failed: {:?}", e);
            return Ok(Redirect::to("/signin?error=oauth_failed").into_response());
        }
    };

    let allow_domain = state.config.team_google_allow_domain.as_str();
    let mut parts = email.splitn(2, '@');
    let local_part = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if !allow_domain.is_empty() && !allow_domain.eq_ignore_ascii_case(domain) {
        return Ok(Redirect::to("/signin?error=domain_not_allowed").into_response());
    }

    let user = models::user::get_by_email(&state.pool, &email).await?;
    let user_id = if user.username.is_empty() {
        // New account: a Google user can sign in even if sign-up is closed,
        // because the domain restriction is the gate for that path. Derive
        // a valid, unique username from the email local part.
        let base = util::sanitize_username(local_part);
        create_google_user(&state, &base, &email).await?
    } else {
        user.id
    };

    let jar = auth::sign_in(jar, user_id, secure);
    let (jar, ref_url) = auth::take_ref_url(jar);
    Ok((jar, Redirect::to(&ref_url)).into_response())
}

/// Create a Google-backed account, appending a numeric suffix when the
/// preferred username is already taken.
async fn create_google_user(state: &AppState, base: &str, email: &str) -> anyhow::Result<i32> {
    for attempt in 0..20u32 {
        let candidate = if attempt == 0 {
            base.to_string()
        } else {
            let suffix = format!("_{}", attempt + 1);
            let mut c = base.to_string();
            c.truncate(util::MAX_USERNAME_LEN.saturating_sub(suffix.len()));
            c + &suffix
        };
        match models::user::create_with_email(&state.pool, &candidate, email).await {
            Ok(id) => return Ok(id),
            Err(e) if crate::error::is_unique_violation(&e) => continue,
            Err(e) => return Err(e),
        }
    }
    anyhow::bail!("could not allocate a unique username for {}", email)
}

pub async fn icon_upload(
    State(state): State<AppState>,
    AuthUser(_login_user): AuthUser,
    multipart: Multipart,
) -> AppResult<Response> {
    Ok(upload_and_respond(&state, multipart, "public/img/profile/icons").await)
}

/// Maximum accepted upload size. The router's body limit is set slightly
/// above this so oversized requests fail here with a friendly message
/// instead of a raw stream error.
pub const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024; // 20MB

enum UploadError {
    TooLarge,
    NotAnImage,
    Missing,
    Other(anyhow::Error),
}

impl From<anyhow::Error> for UploadError {
    fn from(e: anyhow::Error) -> Self {
        UploadError::Other(e)
    }
}

/// Save the uploaded `file` field and answer with `{fileurl}` JSON, or an
/// error status with `{error}` JSON.
pub async fn upload_and_respond(state: &AppState, multipart: Multipart, dir: &str) -> Response {
    match save_uploaded_image(multipart, dir).await {
        Ok(fileurl) => Json(json!({
            "fileurl": format!("{}/{}", state.config.team_domain, fileurl),
        }))
        .into_response(),
        Err(e) => {
            let (status, message) = match e {
                UploadError::TooLarge => (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "アップロードに失敗しました。ファイルサイズの上限は20MBです。",
                ),
                UploadError::NotAnImage => (
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "画像ファイル（PNG / JPEG / GIF / WebP）のみアップロードできます。",
                ),
                UploadError::Missing => (StatusCode::BAD_REQUEST, "ファイルが選択されていません。"),
                UploadError::Other(err) => {
                    tracing::warn!("upload failed: {:?}", err);
                    (StatusCode::BAD_REQUEST, "アップロードに失敗しました。")
                }
            };
            (status, Json(json!({ "error": message }))).into_response()
        }
    }
}

/// Save the `file` field of a multipart form under `dir`, returning the
/// public URL path (without the leading `public/`). The format is sniffed
/// from the bytes and the name is random, so clients cannot choose either.
async fn save_uploaded_image(mut multipart: Multipart, dir: &str) -> Result<String, UploadError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| UploadError::Other(e.into()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let data = match field.bytes().await {
            Ok(data) => data,
            // The body limit layer surfaces as a stream error here.
            Err(_) => return Err(UploadError::TooLarge),
        };
        if data.len() > MAX_UPLOAD_BYTES {
            return Err(UploadError::TooLarge);
        }
        let ext = util::sniff_image_extension(&data).ok_or(UploadError::NotAnImage)?;
        let name = &util::random_token()[..32];
        let filepath = format!("{}/{}.{}", dir, name, ext);
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(anyhow::Error::from)?;
        tokio::fs::write(&filepath, &data)
            .await
            .map_err(anyhow::Error::from)?;
        return Ok(filepath.trim_start_matches("public/").to_string());
    }
    Err(UploadError::Missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
        assert!(!constant_time_eq("", "a"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn unknown_user_hash_parses() {
        assert!(argon2::password_hash::PasswordHash::new(UNKNOWN_USER_HASH).is_ok());
    }
}
