use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::Method;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::SignedCookieJar;

use crate::models;
use crate::models::user::UserWithPreference;
use crate::state::AppState;

pub const LOGIN_COOKIE: &str = "team_login";
pub const REF_URL_COOKIE: &str = "team_ref_url";
pub const OAUTH_STATE_COOKIE: &str = "team_oauth_state";

/// Extractor for pages that require a signed-in user.
/// Rejects with a redirect to /signin, remembering the requested URL.
pub struct AuthUser(pub UserWithPreference);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Response> {
        let jar = SignedCookieJar::from_headers(&parts.headers, state.cookie_key.clone());
        if let Some(id) = jar
            .get(LOGIN_COOKIE)
            .and_then(|c| c.value().parse::<i32>().ok())
        {
            match models::user::get_current_user(&state.pool, id, &state.config).await {
                Ok(user) if user.id != 0 => return Ok(AuthUser(user)),
                Ok(_) => {}
                Err(e) => tracing::error!("failed to load current user: {:?}", e),
            }
        }

        // Remember where the user was heading (GET only), then send to /signin.
        let mut jar = jar;
        if parts.method == Method::GET {
            if let Some(pq) = parts.uri.path_and_query() {
                jar = jar.add(session_cookie(
                    REF_URL_COOKIE,
                    pq.as_str().to_string(),
                    state.config.secure_cookies(),
                ));
            }
        }
        Err((jar, Redirect::to("/signin")).into_response())
    }
}

/// Optional variant: extracts the user if signed in, `None` otherwise.
pub struct MaybeUser(pub Option<UserWithPreference>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = SignedCookieJar::from_headers(&parts.headers, state.cookie_key.clone());
        if let Some(id) = jar
            .get(LOGIN_COOKIE)
            .and_then(|c| c.value().parse::<i32>().ok())
        {
            if let Ok(user) = models::user::get_current_user(&state.pool, id, &state.config).await {
                if user.id != 0 {
                    return Ok(MaybeUser(Some(user)));
                }
            }
        }
        Ok(MaybeUser(None))
    }
}

fn session_cookie(name: &str, value: String, secure: bool) -> Cookie<'static> {
    Cookie::build((name.to_string(), value))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .build()
}

/// Sign the user in by storing their id in the signed cookie jar.
pub fn sign_in(jar: SignedCookieJar, user_id: i32, secure: bool) -> SignedCookieJar {
    jar.add(session_cookie(LOGIN_COOKIE, user_id.to_string(), secure))
}

/// Sign the user out.
pub fn sign_out(jar: SignedCookieJar) -> SignedCookieJar {
    jar.remove(Cookie::build((LOGIN_COOKIE, "")).path("/").build())
}

/// Store a freshly generated OAuth CSRF state token in the signed jar.
pub fn set_oauth_state(jar: SignedCookieJar, secure: bool) -> (SignedCookieJar, String) {
    let token = crate::util::random_token();
    let jar = jar.add(session_cookie(OAUTH_STATE_COOKIE, token.clone(), secure));
    (jar, token)
}

/// Pop the stored OAuth CSRF state token.
pub fn take_oauth_state(jar: SignedCookieJar) -> (SignedCookieJar, Option<String>) {
    let token = jar.get(OAUTH_STATE_COOKIE).map(|c| c.value().to_string());
    let jar = jar.remove(Cookie::build((OAUTH_STATE_COOKIE, "")).path("/").build());
    (jar, token)
}

/// True for an in-app path such as `/post/show/1?x=y`. Protocol-relative
/// forms (`//evil.com`, `/\evil.com`) and anything with a scheme are
/// rejected, which rules out open redirects.
pub fn is_safe_redirect(url: &str) -> bool {
    url.starts_with('/')
        && !url.starts_with("//")
        && !url.starts_with("/\\")
        && !url.contains('\\')
        && !url.contains("\r")
        && !url.contains("\n")
}

/// Pop the remembered return-to URL. Only relative paths are honored.
pub fn take_ref_url(jar: SignedCookieJar) -> (SignedCookieJar, String) {
    let url = jar
        .get(REF_URL_COOKIE)
        .map(|c| c.value().to_string())
        .filter(|u| is_safe_redirect(u))
        .unwrap_or_else(|| "/".to_string());
    let jar = jar.remove(Cookie::build((REF_URL_COOKIE, "")).path("/").build());
    (jar, url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_redirect_rejects_external_forms() {
        assert!(is_safe_redirect("/"));
        assert!(is_safe_redirect("/post/show/1?page=2"));
        assert!(!is_safe_redirect("//evil.com"));
        assert!(!is_safe_redirect("/\\evil.com"));
        assert!(!is_safe_redirect("https://evil.com"));
        assert!(!is_safe_redirect("post/list"));
        assert!(!is_safe_redirect(""));
    }
}
