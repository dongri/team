use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
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
        "invalid_credentials" => "Invalid username or password.",
        "missing_fields" => "Please fill in both username and password.",
        "oauth_failed" => "Google sign-in failed. Please try again.",
        "domain_not_allowed" => "This email domain is not allowed.",
        "wrong_password" => "Current password is incorrect.",
        "password_mismatch" => "New passwords do not match.",
        _ => return None,
    })
}

pub async fn get_signup(
    State(state): State<AppState>,
    Query(q): Query<FlashQuery>,
) -> AppResult<Response> {
    let data = json!({
        "google_enabled": state.config.google_configured(),
        "error_message": flash_message(&q.error),
    });
    Ok(render(&state, "account/signup", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct CredentialsForm {
    pub username: String,
    pub password: String,
}

pub async fn post_signup(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<CredentialsForm>,
) -> AppResult<Response> {
    if form.username.trim().is_empty() || form.password.is_empty() {
        return Ok(Redirect::to("/signup?error=missing_fields").into_response());
    }
    let password = util::hash_password(form.password).await?;
    let user_id = match models::user::create(&state.pool, form.username.trim(), &password).await {
        Ok(id) => id,
        Err(e) if crate::error::is_unique_violation(&e) => {
            return Ok(Redirect::to("/signup?error=username_taken").into_response());
        }
        Err(e) => return Err(e.into()),
    };
    let jar = auth::sign_in(jar, user_id);
    Ok((jar, Redirect::to("/")).into_response())
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
        "error_message": flash_message(&q.error),
    });
    Ok(render(&state, "account/signin", &data)?.into_response())
}

pub async fn post_signin(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<CredentialsForm>,
) -> AppResult<Response> {
    let user = models::user::get_with_password_by_username(&state.pool, &form.username).await?;
    if user.username.is_empty()
        || !util::verify_password(form.password.clone(), user.password.clone()).await
    {
        return Ok(Redirect::to("/signin?error=invalid_credentials").into_response());
    }

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

    let jar = auth::sign_in(jar, user.id);
    let (jar, ref_url) = auth::take_ref_url(jar);
    Ok((jar, Redirect::to(&ref_url)).into_response())
}

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
    models::user::update_icon_url(&state.pool, login_user.id, &form.icon_url).await?;
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
    let user = models::user::get_with_password_by_id(&state.pool, login_user.id).await?;
    if !util::verify_password(form.current_password, user.password).await {
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
    match models::user::update_username(&state.pool, login_user.id, &form.username).await {
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
            posts =
                models::post::user_posts(&state.pool, &username, page.offset, page.limit, kind)
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
    let code = match q.code {
        Some(code) if !code.is_empty() => code,
        _ => {
            // Start the flow: issue a per-request CSRF state token.
            let (jar, token) = auth::set_oauth_state(jar);
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
    let username = parts.next().unwrap_or("").to_string();
    let domain = parts.next().unwrap_or("");
    if !allow_domain.is_empty() && allow_domain != domain {
        return Ok(Redirect::to("/signin?error=domain_not_allowed").into_response());
    }

    let user = models::user::get_by_email(&state.pool, &email).await?;
    let user_id = if user.username.is_empty() {
        models::user::create_with_email(&state.pool, &username, &email).await?
    } else {
        user.id
    };

    let jar = auth::sign_in(jar, user_id);
    let (jar, ref_url) = auth::take_ref_url(jar);
    Ok((jar, Redirect::to(&ref_url)).into_response())
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

/// Save the uploaded `file` field and answer with `{fileurl}` JSON, or a
/// 413 with `{error}` JSON when the upload is too large / malformed.
pub async fn upload_and_respond(state: &AppState, multipart: Multipart, dir: &str) -> Response {
    match save_uploaded_image(multipart, dir).await {
        Ok(fileurl) => Json(json!({
            "fileurl": format!("{}/{}", state.config.team_domain, fileurl),
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("upload failed: {:?}", e);
            (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(json!({
                    "error": "アップロードに失敗しました。ファイルサイズの上限は20MBです。",
                })),
            )
                .into_response()
        }
    }
}

/// Save the `file` field of a multipart form under `dir`, returning the
/// public URL path (without the leading `public/`).
async fn save_uploaded_image(mut multipart: Multipart, dir: &str) -> anyhow::Result<String> {
    let timestamp = chrono::Utc::now().timestamp();
    let filepath = format!("{}/{}.png", dir, timestamp);
    let fileurl = filepath.trim_start_matches("public/").to_string();

    while let Some(field) = multipart.next_field().await? {
        if field.name() == Some("file") {
            let data = field.bytes().await?;
            anyhow::ensure!(data.len() <= MAX_UPLOAD_BYTES, "file too large");
            tokio::fs::create_dir_all(dir).await?;
            tokio::fs::write(&filepath, &data).await?;
        }
    }
    Ok(fileurl)
}
